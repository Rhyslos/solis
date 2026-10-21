//! Models on the GPU: an [`assets::Model`] uploaded once into vertex and
//! index buffers, textures and per-material settings, then drawn every
//! frame with the simple material (a colour, a colour texture, and light
//! from a fixed sun and the sky until real lights arrive in milestone 4).

use crate::renderer::{COLOR_FORMAT, DEPTH_FORMAT};
use assets::{AlphaMode, Material, Model, Texture, Wrap};
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

/// A material's settings exactly as the shader's `Material` struct
/// expects them.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub(crate) struct MaterialUniform {
    /// Linear RGBA, multiplied with the texture.
    base_color: [f32; 4],
    /// x: alpha cutoff; y: 0 solid, 1 cut out, 2 blended; zw unused.
    alpha: [f32; 4],
}

impl MaterialUniform {
    pub(crate) fn new(material: &Material) -> Self {
        let (cutoff, mode) = match material.alpha {
            AlphaMode::Opaque => (0.0, 0.0),
            AlphaMode::Mask(cutoff) => (cutoff, 1.0),
            AlphaMode::Blend => (0.0, 2.0),
        };
        Self {
            base_color: material.base_color,
            alpha: [cutoff, mode, 0.0, 0.0],
        }
    }
}

/// How a mesh is drawn, which picks its pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Pass {
    /// Solid, back faces hidden.
    Solid,
    /// Solid, both sides drawn.
    SolidDoubleSided,
    /// Blended with what's behind, drawn last, both sides.
    Blended,
}

#[derive(Debug)]
struct GpuMaterial {
    bind_group: wgpu::BindGroup,
    pass: Pass,
}

#[derive(Debug)]
struct GpuMesh {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    index_count: u32,
    material: usize,
}

/// A model uploaded to the GPU, ready to draw with
/// [`ViewportRenderer::render`](crate::ViewportRenderer::render).
#[derive(Debug)]
pub struct GpuModel {
    meshes: Vec<GpuMesh>,
    materials: Vec<GpuMaterial>,
    triangles: usize,
}

impl GpuModel {
    /// How many triangles it draws.
    pub fn triangles(&self) -> usize {
        self.triangles
    }

    /// Draws the meshes that belong in `pass`, with `pipeline` already set.
    pub(crate) fn draw(&self, render_pass: &mut wgpu::RenderPass<'_>, pass: Pass) {
        for mesh in &self.meshes {
            let Some(material) = self.materials.get(mesh.material) else {
                continue;
            };
            if material.pass != pass || mesh.index_count == 0 {
                continue;
            }
            render_pass.set_bind_group(1, &material.bind_group, &[]);
            render_pass.set_vertex_buffer(0, mesh.vertices.slice(..));
            render_pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..mesh.index_count, 0, 0..1);
        }
    }
}

/// The parts every model's materials share.
#[derive(Debug)]
pub(crate) struct ModelPipelines {
    pub(crate) material_layout: wgpu::BindGroupLayout,
    pub(crate) solid: wgpu::RenderPipeline,
    pub(crate) solid_double_sided: wgpu::RenderPipeline,
    pub(crate) blended: wgpu::RenderPipeline,
}

impl ModelPipelines {
    pub(crate) fn new(
        device: &wgpu::Device,
        frame_layout: &wgpu::BindGroupLayout,
        shader: &wgpu::ShaderModule,
        sample_count: u32,
    ) -> Self {
        let material_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("material layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("model layout"),
            bind_group_layouts: &[Some(frame_layout), Some(&material_layout)],
            immediate_size: 0,
        });
        let pipeline = |pass| create_pipeline(device, &layout, shader, sample_count, pass);
        Self {
            solid: pipeline(Pass::Solid),
            solid_double_sided: pipeline(Pass::SolidDoubleSided),
            blended: pipeline(Pass::Blended),
            material_layout,
        }
    }

    /// Uploads `model`. Textures larger than the GPU allows are replaced by
    /// plain white rather than failing (the loader is normally told the
    /// GPU's limit, so this doesn't happen).
    pub(crate) fn upload(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        model: &Model,
    ) -> GpuModel {
        let max_side = device.limits().max_texture_dimension_2d;
        let white = assets::texture::solid([255, 255, 255, 255]);
        let textures: Vec<(wgpu::TextureView, wgpu::Sampler)> = model
            .textures
            .iter()
            .map(|t| {
                let usable =
                    t.width <= max_side && t.height <= max_side && t.width > 0 && t.height > 0;
                upload_texture(device, queue, if usable { t } else { &white })
            })
            .collect();
        let fallback = upload_texture(device, queue, &white);
        // Each material, then the plain default last for meshes without one.
        let mut materials: Vec<&Material> = model.materials.iter().collect();
        let default = Material::default();
        materials.push(&default);
        let materials: Vec<GpuMaterial> = materials
            .into_iter()
            .map(|material| {
                let (view, sampler) = material
                    .base_color_texture
                    .and_then(|i| textures.get(i))
                    .unwrap_or(&fallback);
                let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("material"),
                    contents: bytemuck::bytes_of(&MaterialUniform::new(material)),
                    usage: wgpu::BufferUsages::UNIFORM,
                });
                let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("material bind group"),
                    layout: &self.material_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: uniform.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::Sampler(sampler),
                        },
                    ],
                });
                let pass = match (material.alpha, material.double_sided) {
                    (AlphaMode::Blend, _) => Pass::Blended,
                    (_, true) => Pass::SolidDoubleSided,
                    (_, false) => Pass::Solid,
                };
                GpuMaterial { bind_group, pass }
            })
            .collect();
        let default_material = materials.len() - 1;
        let meshes = model
            .meshes
            .iter()
            .filter(|m| !m.indices.is_empty() && !m.vertices.is_empty())
            .map(|mesh| GpuMesh {
                vertices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("mesh vertices"),
                    contents: bytemuck::cast_slice(&mesh.vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                }),
                indices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("mesh indices"),
                    contents: bytemuck::cast_slice(&mesh.indices),
                    usage: wgpu::BufferUsages::INDEX,
                }),
                index_count: mesh.indices.len() as u32,
                material: mesh
                    .material
                    .filter(|&m| m < default_material)
                    .unwrap_or(default_material),
            })
            .collect();
        GpuModel {
            meshes,
            materials,
            triangles: model.triangles(),
        }
    }
}

fn upload_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &Texture,
) -> (wgpu::TextureView, wgpu::Sampler) {
    let size = wgpu::Extent3d {
        width: texture.width,
        height: texture.height,
        depth_or_array_layers: 1,
    };
    let gpu = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("model texture"),
        size,
        mip_level_count: texture.levels.len().max(1) as u32,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        // The pixels are sRGB; sampling turns them into linear colour.
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let (mut width, mut height) = (texture.width, texture.height);
    for (level, pixels) in texture.levels.iter().enumerate() {
        if pixels.len() as u64 == u64::from(width) * u64::from(height) * 4 {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &gpu,
                    mip_level: level as u32,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                pixels,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * width),
                    rows_per_image: Some(height),
                },
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            );
        }
        width = (width / 2).max(1);
        height = (height / 2).max(1);
    }
    let address = |wrap| match wrap {
        Wrap::Repeat => wgpu::AddressMode::Repeat,
        Wrap::Mirror => wgpu::AddressMode::MirrorRepeat,
        Wrap::Clamp => wgpu::AddressMode::ClampToEdge,
    };
    let filter = if texture.sampler.nearest {
        wgpu::FilterMode::Nearest
    } else {
        wgpu::FilterMode::Linear
    };
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("model sampler"),
        address_mode_u: address(texture.sampler.wrap_u),
        address_mode_v: address(texture.sampler.wrap_v),
        mag_filter: filter,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::MipmapFilterMode::Linear,
        ..Default::default()
    });
    (
        gpu.create_view(&wgpu::TextureViewDescriptor::default()),
        sampler,
    )
}

fn create_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    sample_count: u32,
    pass: Pass,
) -> wgpu::RenderPipeline {
    const ATTRIBUTES: [wgpu::VertexAttribute; 3] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2];
    let vertex_layout = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<assets::Vertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &ATTRIBUTES,
    };
    let blended = pass == Pass::Blended;
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(match pass {
            Pass::Solid => "solid mesh pipeline",
            Pass::SolidDoubleSided => "double-sided mesh pipeline",
            Pass::Blended => "blended mesh pipeline",
        }),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("mesh_vs"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[Some(vertex_layout)],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: (pass == Pass::Solid).then_some(wgpu::Face::Back),
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(!blended),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: sample_count,
            ..Default::default()
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("mesh_fs"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: COLOR_FORMAT,
                blend: blended.then_some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn material_settings_match_the_shader_layout() {
        assert_eq!(std::mem::size_of::<MaterialUniform>(), 32);
        let m = MaterialUniform::new(&Material {
            alpha: AlphaMode::Mask(0.3),
            ..Material::default()
        });
        assert_eq!(m.alpha[..2], [0.3, 1.0]);
        assert_eq!(
            MaterialUniform::new(&Material::default()).base_color,
            [1.0; 4]
        );
    }
}
