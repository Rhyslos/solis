//! Drawing a viewport into an offscreen image on the GPU.
//!
//! Each viewport renders into its own [`ViewportTarget`] (colour and depth
//! images, with multisampling when the GPU supports it). The caller then
//! shows the finished colour image however it likes; the editor hands it to
//! egui as a texture.

use crate::frame::{FrameParams, FrameUniforms, GRID_HALF_EXTENT};
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

/// Colour format the viewport renders in: plain 8-bit RGBA, which is what
/// egui expects for native textures. The shaders work in linear colour and
/// encode to sRGB themselves as they write (`encode` in `viewport.wgsl`).
///
/// An sRGB format with a plain "view" of it would let the GPU do the
/// encoding, but older and software GPUs (OpenGL) cannot make such views,
/// and the editor must open on them. Blending therefore happens on encoded
/// colours, as egui's own drawing does; the lit 3D renderer (Phase 1) will
/// draw into a linear HDR image and encode in one final pass.
pub const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
/// Depth buffer format.
pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
/// The viewport shaders, in WGSL.
pub const SHADER_SOURCE: &str = include_str!("viewport.wgsl");

/// The X axis on the ground, linear colour (red).
pub const AXIS_X_COLOR: [f32; 4] = [0.80, 0.09, 0.10, 0.85];
/// The Y axis on the ground, linear colour (green).
pub const AXIS_Y_COLOR: [f32; 4] = [0.10, 0.55, 0.12, 0.85];

/// Picks the multisample count for smooth edges: 4 when the GPU supports it
/// for both colour and depth, otherwise 1 (no multisampling).
pub fn best_sample_count(adapter: &wgpu::Adapter) -> u32 {
    let supports = |format| {
        adapter
            .get_texture_format_features(format)
            .flags
            .sample_count_supported(4)
    };
    if supports(COLOR_FORMAT) && supports(DEPTH_FORMAT) {
        4
    } else {
        1
    }
}

/// GPU images one viewport draws into. Create with
/// [`ViewportRenderer::create_target`].
#[derive(Debug)]
pub struct ViewportTarget {
    size: [u32; 2],
    /// Multisampled colour image (only when multisampling).
    msaa_view: Option<wgpu::TextureView>,
    /// The finished, single-sample colour image.
    resolved: wgpu::Texture,
    resolved_view: wgpu::TextureView,
    depth_view: wgpu::TextureView,
}

impl ViewportTarget {
    /// Size in pixels.
    pub fn size(&self) -> [u32; 2] {
        self.size
    }

    /// The finished colour image, ready for showing in egui.
    pub fn display_view(&self) -> &wgpu::TextureView {
        &self.resolved_view
    }

    /// The finished colour image as a texture (for copying).
    pub fn color_texture(&self) -> &wgpu::Texture {
        &self.resolved
    }
}

/// A point of a coloured line.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct LineVertex {
    /// Position in the world, metres.
    pub position: [f32; 3],
    /// Linear colour with opacity.
    pub color: [f32; 4],
}

impl LineVertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4];
    const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<LineVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &Self::ATTRIBUTES,
    };
}

/// The X and Y axes along the ground, as line pairs, as far as the grid
/// reaches.
pub fn ground_axes() -> [LineVertex; 4] {
    let h = GRID_HALF_EXTENT;
    [
        LineVertex {
            position: [-h, 0.0, 0.0],
            color: AXIS_X_COLOR,
        },
        LineVertex {
            position: [h, 0.0, 0.0],
            color: AXIS_X_COLOR,
        },
        LineVertex {
            position: [0.0, -h, 0.0],
            color: AXIS_Y_COLOR,
        },
        LineVertex {
            position: [0.0, h, 0.0],
            color: AXIS_Y_COLOR,
        },
    ]
}

/// The kinds of thing the viewport draws, in drawing order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// The backdrop, behind everything.
    Sky,
    /// The ground grid: see-through, hidden behind solids.
    Grid,
    /// See-through lines hidden behind solids: the axes.
    Lines,
}

fn create_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    sample_count: u32,
    kind: Kind,
) -> wgpu::RenderPipeline {
    let lines: &[wgpu::VertexBufferLayout<'static>] = &[LineVertex::LAYOUT];
    let (label, vs, fs, topology, buffers) = match kind {
        Kind::Sky => (
            "sky pipeline",
            "sky_vs",
            "sky_fs",
            wgpu::PrimitiveTopology::TriangleList,
            &[][..],
        ),
        Kind::Grid => (
            "grid pipeline",
            "grid_vs",
            "grid_fs",
            wgpu::PrimitiveTopology::TriangleList,
            &[][..],
        ),
        Kind::Lines => (
            "line pipeline",
            "line_vs",
            "line_fs",
            wgpu::PrimitiveTopology::LineList,
            lines,
        ),
    };
    let blend = (kind != Kind::Sky).then_some(wgpu::BlendState::ALPHA_BLENDING);
    let depth_compare = if kind == Kind::Sky {
        wgpu::CompareFunction::Always
    } else {
        wgpu::CompareFunction::LessEqual
    };
    let targets = [Some(wgpu::ColorTargetState {
        format: COLOR_FORMAT,
        blend,
        write_mask: wgpu::ColorWrites::ALL,
    })];
    let vertex_buffers: Vec<Option<wgpu::VertexBufferLayout<'static>>> =
        buffers.iter().cloned().map(Some).collect();
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some(vs),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &vertex_buffers,
        },
        primitive: wgpu::PrimitiveState {
            topology,
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(false),
            depth_compare: Some(depth_compare),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: sample_count,
            ..Default::default()
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(fs),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &targets,
        }),
        multiview_mask: None,
        cache: None,
    })
}

/// Draws viewports: the pipelines and buffers they share.
#[derive(Debug)]
pub struct ViewportRenderer {
    sample_count: u32,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    sky_pipeline: wgpu::RenderPipeline,
    grid_pipeline: wgpu::RenderPipeline,
    line_pipeline: wgpu::RenderPipeline,
    axes: wgpu::Buffer,
    axis_count: u32,
}

impl ViewportRenderer {
    /// Sets up drawing on `device`, with `sample_count` samples per pixel
    /// (4 for smooth edges, otherwise 1; see [`best_sample_count`]).
    pub fn new(device: &wgpu::Device, sample_count: u32) -> Self {
        let sample_count = if sample_count == 4 { 4 } else { 1 };
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("viewport shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER_SOURCE.into()),
        });
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("frame uniforms"),
            size: std::mem::size_of::<FrameUniforms>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("frame layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("frame bind group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("viewport layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let pipeline = |kind| create_pipeline(device, &layout, &shader, sample_count, kind);
        let axis_vertices = ground_axes();
        let axes = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ground axes"),
            contents: bytemuck::cast_slice(&axis_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        Self {
            sample_count,
            sky_pipeline: pipeline(Kind::Sky),
            grid_pipeline: pipeline(Kind::Grid),
            line_pipeline: pipeline(Kind::Lines),
            uniform_buffer,
            bind_group,
            axes,
            axis_count: axis_vertices.len() as u32,
        }
    }

    /// Samples per pixel in use (1 or 4).
    pub fn sample_count(&self) -> u32 {
        self.sample_count
    }

    /// Makes the images for one viewport of `size` pixels. Sizes are
    /// clamped to what the GPU allows, and never below 1×1.
    pub fn create_target(&self, device: &wgpu::Device, size: [u32; 2]) -> ViewportTarget {
        let max = device.limits().max_texture_dimension_2d.max(1);
        let size = [size[0].clamp(1, max), size[1].clamp(1, max)];
        let extent = wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        };
        let texture = |label, format, usage, samples| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: extent,
                mip_level_count: 1,
                sample_count: samples,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            })
        };
        let resolved = texture(
            "viewport colour",
            COLOR_FORMAT,
            wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            1,
        );
        let msaa_view = (self.sample_count > 1).then(|| {
            texture(
                "viewport colour (multisampled)",
                COLOR_FORMAT,
                wgpu::TextureUsages::RENDER_ATTACHMENT,
                self.sample_count,
            )
            .create_view(&wgpu::TextureViewDescriptor::default())
        });
        let depth_view = texture(
            "viewport depth",
            DEPTH_FORMAT,
            wgpu::TextureUsages::RENDER_ATTACHMENT,
            self.sample_count,
        )
        .create_view(&wgpu::TextureViewDescriptor::default());
        let resolved_view = resolved.create_view(&wgpu::TextureViewDescriptor::default());
        ViewportTarget {
            size,
            msaa_view,
            resolved,
            resolved_view,
            depth_view,
        }
    }

    /// Draws one frame into `target` and submits it. Never waits for the
    /// GPU to finish.
    pub fn render(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target: &ViewportTarget,
        params: &FrameParams,
    ) {
        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::bytes_of(&FrameUniforms::new(params)),
        );
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("viewport frame"),
        });
        // With multisampling, drawing goes into the multisampled image,
        // which is smoothed into the finished one at the end.
        let (view, resolve_target) = match &target.msaa_view {
            Some(msaa) => (msaa, Some(&target.resolved_view)),
            None => (&target.resolved_view, None),
        };
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("viewport"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: if resolve_target.is_some() {
                            wgpu::StoreOp::Discard
                        } else {
                            wgpu::StoreOp::Store
                        },
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &target.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.set_pipeline(&self.sky_pipeline);
            pass.draw(0..3, 0..1);
            pass.set_pipeline(&self.grid_pipeline);
            pass.draw(0..6, 0..1);
            pass.set_pipeline(&self.line_pipeline);
            pass.set_vertex_buffer(0, self.axes.slice(..));
            pass.draw(0..self.axis_count, 0..1);
        }
        queue.submit([encoder.finish()]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_vertices_match_the_shader_layout() {
        assert_eq!(std::mem::size_of::<LineVertex>(), 7 * 4);
        assert_eq!(LineVertex::LAYOUT.attributes.len(), 2);
    }

    #[test]
    fn the_axes_lie_on_the_ground_through_the_origin() {
        let axes = ground_axes();
        assert!(axes.iter().all(|v| v.position[2] == 0.0));
        assert_eq!(axes[0].color, AXIS_X_COLOR);
        assert_eq!(axes[2].color, AXIS_Y_COLOR);
        assert_eq!(axes[0].position[1], 0.0, "X runs along y = 0");
        assert_eq!(axes[2].position[0], 0.0, "Y runs along x = 0");
    }
}
