//! Models on the GPU: an [`assets::Model`] uploaded once into vertex and
//! index buffers, textures and per-material settings, then drawn every
//! frame in the chosen look (standard, stylized or unlit; see
//! `viewport.wgsl`), lit by a fixed sun and the sky until real lights
//! arrive in milestone 4.

use crate::renderer::{DEPTH_FORMAT, HDR_FORMAT};
use assets::{AlphaMode, Material, Model, Texture, Wrap};
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

/// A material's settings exactly as the shader's `Material` struct
/// expects them.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub(crate) struct MaterialUniform {
    /// Linear RGBA, multiplied with the colour texture.
    base_color: [f32; 4],
    /// Glow, linear RGB; w unused.
    emissive: [f32; 4],
    /// Metallic, roughness, normal strength, occlusion strength.
    surface: [f32; 4],
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
        let [r, g, b] = material.emissive;
        Self {
            base_color: material.base_color,
            emissive: [r, g, b, 0.0],
            surface: [
                material.metallic,
                material.roughness,
                material.normal_scale,
                material.occlusion_strength,
            ],
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
    /// Whether the stylized look draws an outline around it: solid,
    /// single-sided surfaces without cut-out parts.
    outlined: bool,
}

#[derive(Debug)]
struct GpuMesh {
    vertices: wgpu::Buffer,
    /// The same corners with smoothed normals, for outlines (only for
    /// outlined materials).
    outline_vertices: Option<wgpu::Buffer>,
    indices: wgpu::Buffer,
    index_count: u32,
    material: usize,
}

/// A model uploaded to the GPU, ready to draw with
/// [`ViewportRenderer::render`](crate::ViewportRenderer::render): each mesh
/// once, and where the model places each (decision 0021).
#[derive(Debug)]
pub struct GpuModel {
    /// The meshes, by their index in the model; `None` for one that can't
    /// be drawn (empty, or too big for the card).
    meshes: Vec<Option<GpuMesh>>,
    /// Every mesh as drawn: which, and where in the model.
    placements: Vec<(usize, glam::Mat4)>,
    /// Each node's own meshes, for drawing one part on its own.
    nodes: Vec<Vec<usize>>,
    materials: Vec<GpuMaterial>,
    triangles: usize,
    bounds: (glam::Vec3, glam::Vec3),
}

impl GpuModel {
    /// How many triangles it draws.
    pub fn triangles(&self) -> usize {
        self.triangles
    }

    /// The box the model fits in, lowest and highest corners, in metres.
    pub fn bounds(&self) -> (glam::Vec3, glam::Vec3) {
        self.bounds
    }

    /// Every mesh as drawn: which, and where in the model.
    pub(crate) fn placements(&self) -> &[(usize, glam::Mat4)] {
        &self.placements
    }

    /// Node `node`'s own meshes (none for a node that isn't there).
    pub(crate) fn node_meshes(&self, node: usize) -> &[usize] {
        self.nodes.get(node).map_or(&[], Vec::as_slice)
    }

    /// The mesh and its material, if it can be drawn.
    fn mesh(&self, mesh: usize) -> Option<(&GpuMesh, &GpuMaterial)> {
        let mesh = self.meshes.get(mesh)?.as_ref()?;
        let material = self.materials.get(mesh.material)?;
        (mesh.index_count > 0).then_some((mesh, material))
    }

    /// Which pass a mesh is drawn in.
    pub(crate) fn pass_of(&self, mesh: usize) -> Option<Pass> {
        self.mesh(mesh).map(|(_, material)| material.pass)
    }

    /// Draws `instances` of a mesh's shape only, for the sun's shadow map
    /// (blended ones, like glass, cast no shadow).
    pub(crate) fn draw_depth(
        &self,
        render_pass: &mut wgpu::RenderPass<'_>,
        mesh: usize,
        instances: std::ops::Range<u32>,
    ) {
        let Some((mesh, material)) = self.mesh(mesh) else {
            return;
        };
        if material.pass == Pass::Blended {
            return;
        }
        render_pass.set_vertex_buffer(0, mesh.vertices.slice(..));
        render_pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..mesh.index_count, 0, instances);
    }

    /// Draws `instances` of a mesh's shape only, whatever its material
    /// (glass too), for picking.
    pub(crate) fn draw_shape(
        &self,
        render_pass: &mut wgpu::RenderPass<'_>,
        mesh: usize,
        instances: std::ops::Range<u32>,
    ) {
        let Some((mesh, _)) = self.mesh(mesh) else {
            return;
        };
        render_pass.set_vertex_buffer(0, mesh.vertices.slice(..));
        render_pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..mesh.index_count, 0, instances);
    }

    /// Draws `instances` of a mesh with its material, with the pipeline
    /// for its pass already set.
    pub(crate) fn draw(
        &self,
        render_pass: &mut wgpu::RenderPass<'_>,
        mesh: usize,
        instances: std::ops::Range<u32>,
    ) {
        let Some((mesh, material)) = self.mesh(mesh) else {
            return;
        };
        render_pass.set_bind_group(1, &material.bind_group, &[]);
        render_pass.set_vertex_buffer(0, mesh.vertices.slice(..));
        render_pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..mesh.index_count, 0, instances);
    }

    /// Draws the outline of `instances` of a mesh, if it gets one, with
    /// the outline pipeline already set.
    pub(crate) fn draw_outline(
        &self,
        render_pass: &mut wgpu::RenderPass<'_>,
        mesh: usize,
        instances: std::ops::Range<u32>,
    ) {
        let Some((mesh, _)) = self.mesh(mesh) else {
            return;
        };
        let Some(outline) = &mesh.outline_vertices else {
            return;
        };
        render_pass.set_vertex_buffer(0, outline.slice(..));
        render_pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..mesh.index_count, 0, instances);
    }
}

/// The textures a material reads, in binding order: colour, metal and
/// roughness, normals, occlusion, glow.
const TEXTURE_SLOTS: usize = 5;

/// The pipelines for one way of winding triangles.
#[derive(Debug)]
pub(crate) struct Facing {
    pub(crate) solid: wgpu::RenderPipeline,
    pub(crate) solid_double_sided: wgpu::RenderPipeline,
    pub(crate) blended: wgpu::RenderPipeline,
    pub(crate) outline: wgpu::RenderPipeline,
}

impl Facing {
    /// The pipeline for meshes in `pass`.
    pub(crate) fn for_pass(&self, pass: Pass) -> &wgpu::RenderPipeline {
        match pass {
            Pass::Solid => &self.solid,
            Pass::SolidDoubleSided => &self.solid_double_sided,
            Pass::Blended => &self.blended,
        }
    }
}

/// The parts every model's materials share.
#[derive(Debug)]
pub(crate) struct ModelPipelines {
    pub(crate) material_layout: wgpu::BindGroupLayout,
    /// For copies as made.
    pub(crate) normal: Facing,
    /// For mirrored copies, whose triangles wind the other way.
    pub(crate) mirrored: Facing,
}

impl ModelPipelines {
    pub(crate) fn new(
        device: &wgpu::Device,
        frame_layout: &wgpu::BindGroupLayout,
        shader: &wgpu::ShaderModule,
        sample_count: u32,
    ) -> Self {
        let mut entries = vec![wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }];
        for slot in 0..TEXTURE_SLOTS as u32 {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding: 1 + slot,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            });
        }
        for slot in 0..TEXTURE_SLOTS as u32 {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding: 1 + TEXTURE_SLOTS as u32 + slot,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            });
        }
        let material_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("material layout"),
            entries: &entries,
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("model layout"),
            bind_group_layouts: &[Some(frame_layout), Some(&material_layout)],
            immediate_size: 0,
        });
        let outline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("outline layout"),
            bind_group_layouts: &[Some(frame_layout)],
            immediate_size: 0,
        });
        let facing = |front| {
            let pipeline =
                |pass| create_pipeline(device, &layout, shader, sample_count, pass, front);
            Facing {
                solid: pipeline(Pass::Solid),
                solid_double_sided: pipeline(Pass::SolidDoubleSided),
                blended: pipeline(Pass::Blended),
                outline: create_outline_pipeline(
                    device,
                    &outline_layout,
                    shader,
                    sample_count,
                    front,
                ),
            }
        };
        Self {
            normal: facing(wgpu::FrontFace::Ccw),
            mirrored: facing(wgpu::FrontFace::Cw),
            material_layout,
        }
    }

    /// Uploads `model`. Textures larger than the GPU allows are replaced by
    /// plain ones rather than failing (the loader is normally told the
    /// GPU's limit, so this doesn't happen).
    pub(crate) fn upload(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        model: &Model,
    ) -> GpuModel {
        let max_side = device.limits().max_texture_dimension_2d;
        let textures: Vec<Option<(wgpu::TextureView, wgpu::Sampler)>> = model
            .textures
            .iter()
            .map(|t| {
                let usable =
                    t.width <= max_side && t.height <= max_side && t.width > 0 && t.height > 0;
                usable.then(|| upload_texture(device, queue, t))
            })
            .collect();
        // What a material without a given texture reads instead: white
        // (factors alone), or a flat normal.
        let white_colour = upload_texture(device, queue, &assets::texture::solid([255; 4], true));
        let white_data = upload_texture(device, queue, &assets::texture::solid([255; 4], false));
        let flat_normal = upload_texture(
            device,
            queue,
            &assets::texture::solid([128, 128, 255, 255], false),
        );
        // Each material, then the plain default last for meshes without one.
        let mut materials: Vec<&Material> = model.materials.iter().collect();
        let default = Material::default();
        materials.push(&default);
        let materials: Vec<GpuMaterial> = materials
            .into_iter()
            .map(|material| {
                let pick = |index: Option<usize>, fallback| {
                    index
                        .and_then(|i| textures.get(i))
                        .and_then(Option::as_ref)
                        .unwrap_or(fallback)
                };
                let slots = [
                    pick(material.base_color_texture, &white_colour),
                    pick(material.metallic_roughness_texture, &white_data),
                    pick(material.normal_texture, &flat_normal),
                    pick(material.occlusion_texture, &white_data),
                    pick(material.emissive_texture, &white_colour),
                ];
                let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("material"),
                    contents: bytemuck::bytes_of(&MaterialUniform::new(material)),
                    usage: wgpu::BufferUsages::UNIFORM,
                });
                let mut entries = vec![wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                }];
                for (slot, (view, _)) in slots.iter().enumerate() {
                    entries.push(wgpu::BindGroupEntry {
                        binding: 1 + slot as u32,
                        resource: wgpu::BindingResource::TextureView(view),
                    });
                }
                for (slot, (_, sampler)) in slots.iter().enumerate() {
                    entries.push(wgpu::BindGroupEntry {
                        binding: 1 + TEXTURE_SLOTS as u32 + slot as u32,
                        resource: wgpu::BindingResource::Sampler(sampler),
                    });
                }
                let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("material bind group"),
                    layout: &self.material_layout,
                    entries: &entries,
                });
                let pass = match (material.alpha, material.double_sided) {
                    (AlphaMode::Blend, _) => Pass::Blended,
                    (_, true) => Pass::SolidDoubleSided,
                    (_, false) => Pass::Solid,
                };
                GpuMaterial {
                    bind_group,
                    pass,
                    outlined: pass == Pass::Solid && material.alpha == AlphaMode::Opaque,
                }
            })
            .collect();
        let default_material = materials.len() - 1;
        let meshes = model
            .meshes
            .iter()
            .map(|mesh| {
                // A mesh too big for one of the card's buffers would make
                // wgpu stop the program; the importer splits meshes to
                // fit, so this only guards against a model read with other
                // limits.
                let max = device.limits().max_buffer_size;
                let drawable = !mesh.indices.is_empty()
                    && !mesh.vertices.is_empty()
                    && (std::mem::size_of_val(mesh.vertices.as_slice()) as u64) <= max
                    && (std::mem::size_of_val(mesh.indices.as_slice()) as u64) <= max;
                drawable.then_some(mesh)
            })
            .map(|mesh| {
                let mesh = mesh?;
                let material = mesh
                    .material
                    .filter(|&m| m < default_material)
                    .unwrap_or(default_material);
                let outlined = materials.get(material).is_some_and(|m| m.outlined);
                Some(GpuMesh {
                    vertices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("mesh vertices"),
                        contents: bytemuck::cast_slice(&mesh.vertices),
                        usage: wgpu::BufferUsages::VERTEX,
                    }),
                    outline_vertices: outlined.then(|| {
                        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("outline vertices"),
                            contents: bytemuck::cast_slice(&outline_vertices(&mesh.vertices)),
                            usage: wgpu::BufferUsages::VERTEX,
                        })
                    }),
                    indices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("mesh indices"),
                        contents: bytemuck::cast_slice(&mesh.indices),
                        usage: wgpu::BufferUsages::INDEX,
                    }),
                    index_count: mesh.indices.len() as u32,
                    material,
                })
            })
            .collect();
        GpuModel {
            meshes,
            placements: model.placements().collect(),
            nodes: model.nodes.iter().map(|n| n.meshes.clone()).collect(),
            materials,
            triangles: model.triangles(),
            bounds: (model.bounds.min, model.bounds.max),
        }
    }
}

/// The corners with each normal replaced by the average of all normals at
/// that same position, so a hard-edged shape (a box: three normals at each
/// corner) is pushed out evenly and its outline has no gaps.
pub(crate) fn outline_vertices(vertices: &[assets::Vertex]) -> Vec<assets::Vertex> {
    use std::collections::HashMap;
    let key = |v: &assets::Vertex| v.position.map(f32::to_bits);
    let mut sums: HashMap<[u32; 3], glam::Vec3> = HashMap::new();
    for v in vertices {
        *sums.entry(key(v)).or_default() += glam::Vec3::from(v.normal);
    }
    vertices
        .iter()
        .map(|v| {
            let smooth = sums
                .get(&key(v))
                .and_then(|n| n.try_normalize())
                .unwrap_or(glam::Vec3::from(v.normal));
            assets::Vertex {
                normal: smooth.to_array(),
                ..*v
            }
        })
        .collect()
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
        // Colours are stored as sRGB, and sampling turns them into linear
        // light; data textures (metal, roughness, normals) are read as is.
        format: if texture.srgb {
            wgpu::TextureFormat::Rgba8UnormSrgb
        } else {
            wgpu::TextureFormat::Rgba8Unorm
        },
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

/// How a mesh corner is laid out in the vertex buffer.
const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 3] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2];

pub(crate) fn vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<assets::Vertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &VERTEX_ATTRIBUTES,
    }
}

fn create_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    sample_count: u32,
    pass: Pass,
    front_face: wgpu::FrontFace,
) -> wgpu::RenderPipeline {
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
            buffers: &[Some(vertex_layout()), Some(crate::instances::layout())],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            front_face,
            cull_mode: (pass == Pass::Solid).then_some(wgpu::Face::Back),
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(!blended),
            depth_compare: Some(wgpu::CompareFunction::Greater),
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
                format: HDR_FORMAT,
                blend: blended.then_some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

/// The outline pass: the back faces only (front faces culled), pushed out
/// by the vertex shader, behind the model itself.
fn create_outline_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    sample_count: u32,
    front_face: wgpu::FrontFace,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("outline pipeline"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("outline_vs"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[Some(vertex_layout()), Some(crate::instances::layout())],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            front_face,
            cull_mode: Some(wgpu::Face::Front),
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Greater),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: sample_count,
            ..Default::default()
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("outline_fs"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: HDR_FORMAT,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

/// The pipeline that draws models into the sun's shadow map: depth only,
/// both sides of every triangle, pushed back a little (depth bias) so
/// surfaces don't shadow themselves in stripes.
pub(crate) fn create_shadow_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("shadow pipeline"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("shadow_vs"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[Some(vertex_layout()), Some(crate::instances::layout())],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: crate::lighting::SHADOW_FORMAT,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState {
                constant: 2,
                slope_scale: 2.0,
                clamp: 0.0,
            },
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: None,
        multiview_mask: None,
        cache: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outline_normals_average_the_normals_at_a_corner() {
        let corner = |normal: [f32; 3]| assets::Vertex {
            position: [1.0, 1.0, 1.0],
            normal,
            uv: [0.0, 0.0],
        };
        let smoothed = outline_vertices(&[
            corner([1.0, 0.0, 0.0]),
            corner([0.0, 1.0, 0.0]),
            corner([0.0, 0.0, 1.0]),
        ]);
        let expected = glam::Vec3::ONE.normalize();
        for v in smoothed {
            assert!((glam::Vec3::from(v.normal) - expected).length() < 1e-5);
        }
    }

    #[test]
    fn material_settings_match_the_shader_layout() {
        assert_eq!(std::mem::size_of::<MaterialUniform>(), 64);
        let m = MaterialUniform::new(&Material {
            alpha: AlphaMode::Mask(0.3),
            metallic: 1.0,
            roughness: 0.25,
            emissive: [1.0, 0.5, 0.0],
            ..Material::default()
        });
        assert_eq!(m.alpha[..2], [0.3, 1.0]);
        assert_eq!(m.surface, [1.0, 0.25, 1.0, 1.0]);
        assert_eq!(m.emissive, [1.0, 0.5, 0.0, 0.0]);
        assert_eq!(
            MaterialUniform::new(&Material::default()).base_color,
            [1.0; 4]
        );
    }
}
