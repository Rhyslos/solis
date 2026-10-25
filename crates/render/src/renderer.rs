//! Drawing a viewport into an offscreen image on the GPU: the backdrop,
//! models, the grid and the axes.
//!
//! Each viewport renders into its own [`ViewportTarget`] (colour and depth
//! images, with multisampling when the GPU supports it). The caller then
//! shows the finished colour image however it likes; the editor hands it to
//! egui as a texture.

use crate::frame::{FrameLight, FrameParams, FrameUniforms, GRID_HALF_EXTENT};
use crate::gpu_model::{GpuModel, ModelPipelines, Pass};
use crate::instances::Placement;
use crate::lighting::{
    Light, LightsUniform, MARKER_VERTICES, SHADOW_FORMAT, SHADOW_MAP_SIZE, light_markers,
    sun_view_projection,
};
use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use wgpu::util::DeviceExt;

/// The finished image's format: plain 8-bit RGBA, which is what egui
/// expects for native textures. The last pass encodes to sRGB itself
/// (`encode` in `viewport.wgsl`): an sRGB format with a plain "view" of it
/// would let the GPU do it, but older and software GPUs (OpenGL) cannot
/// make such views, and the editor must open on them.
pub const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
/// The format the world is drawn in before the last pass: linear HDR
/// colour, brighter than white allowed, 16-bit floats per channel. Alpha
/// marks the pixels the last pass tone maps (see `viewport.wgsl`).
pub const HDR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
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
    if supports(HDR_FORMAT) && supports(DEPTH_FORMAT) {
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
    /// Multisampled HDR image (only when multisampling).
    msaa_view: Option<wgpu::TextureView>,
    /// The single-sample HDR image the last pass reads.
    hdr_view: wgpu::TextureView,
    /// The last pass's view of `hdr_view`.
    post_bind_group: wgpu::BindGroup,
    /// The finished, tone-mapped sRGB image.
    resolved: wgpu::Texture,
    resolved_view: wgpu::TextureView,
    depth_view: wgpu::TextureView,
    /// This viewport's own frame data, lamps, shadow map and lamp
    /// markers, so several viewports never overwrite each other's.
    frame: FrameResources,
}

/// The most lamps shown with a marker; more are drawn without one.
const MAX_MARKED_LAMPS: usize = 1024;

/// The GPU data one viewport's frames use.
#[derive(Debug)]
struct FrameResources {
    uniform_buffer: wgpu::Buffer,
    lights_buffer: wgpu::Buffer,
    shadow_view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
    shadow_bind_group: wgpu::BindGroup,
    /// Room for every lamp's marker lines, filled each frame.
    markers: wgpu::Buffer,
    /// Every mesh copy's placement, filled each frame; replaced by a
    /// bigger one when a frame has more (decision 0021).
    instances: Option<wgpu::Buffer>,
}

impl FrameResources {
    /// Puts `raw` into the instance buffer, growing it if needed.
    fn write_instances(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        raw: &[crate::instances::InstanceRaw],
    ) {
        let bytes: &[u8] = bytemuck::cast_slice(raw);
        if bytes.is_empty() {
            return;
        }
        let fits = self
            .instances
            .as_ref()
            .is_some_and(|b| b.size() >= bytes.len() as u64);
        if !fits {
            // Room to grow, so a slowly growing scene isn't re-made every
            // frame.
            let size = (bytes.len() as u64).next_power_of_two().max(4096);
            self.instances = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("instances"),
                size,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
        }
        if let Some(buffer) = &self.instances {
            queue.write_buffer(buffer, 0, bytes);
        }
    }
}

impl FrameResources {
    fn new(
        device: &wgpu::Device,
        frame_layout: &wgpu::BindGroupLayout,
        shadow_layout: &wgpu::BindGroupLayout,
        shadow_sampler: &wgpu::Sampler,
    ) -> Self {
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("frame uniforms"),
            size: std::mem::size_of::<FrameUniforms>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let lights_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("lamps"),
            size: std::mem::size_of::<LightsUniform>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        // The sun's shadow map: a depth image the models are drawn into
        // from the sun's side, then looked up while shading.
        let shadow_view = device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("sun shadow map"),
                size: wgpu::Extent3d {
                    width: SHADOW_MAP_SIZE,
                    height: SHADOW_MAP_SIZE,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: SHADOW_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("frame bind group"),
            layout: frame_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: lights_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&shadow_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(shadow_sampler),
                },
            ],
        });
        let shadow_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shadow frame bind group"),
            layout: shadow_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });
        let markers = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("lamp markers"),
            size: (MAX_MARKED_LAMPS * MARKER_VERTICES * std::mem::size_of::<LineVertex>())
                as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            uniform_buffer,
            lights_buffer,
            shadow_view,
            bind_group,
            shadow_bind_group,
            markers,
            instances: None,
        }
    }
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
    // See-through lines blend their colour, and leave the alpha below them
    // alone: it marks which pixels the last pass tone maps.
    let blend = (kind != Kind::Sky).then_some(wgpu::BlendState {
        color: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::SrcAlpha,
            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
            operation: wgpu::BlendOperation::Add,
        },
        alpha: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::Zero,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Add,
        },
    });
    let depth_compare = if kind == Kind::Sky {
        wgpu::CompareFunction::Always
    } else {
        // Depth is reversed (1 near, 0 far: see the camera), so nearer
        // means greater.
        wgpu::CompareFunction::GreaterEqual
    };
    let targets = [Some(wgpu::ColorTargetState {
        format: HDR_FORMAT,
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

/// The last pass: reads the HDR image and writes the finished one.
fn create_post_pipeline(
    device: &wgpu::Device,
    frame_layout: &wgpu::BindGroupLayout,
    post_layout: &wgpu::BindGroupLayout,
    shader: &wgpu::ShaderModule,
) -> wgpu::RenderPipeline {
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("last pass layout"),
        bind_group_layouts: &[Some(frame_layout), None, Some(post_layout)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("last pass pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("post_vs"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("post_fs"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: COLOR_FORMAT,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

/// Draws viewports: the pipelines and buffers they share.
#[derive(Debug)]
pub struct ViewportRenderer {
    sample_count: u32,
    frame_layout: wgpu::BindGroupLayout,
    shadow_layout: wgpu::BindGroupLayout,
    shadow_sampler: wgpu::Sampler,
    shadow_pipeline: wgpu::RenderPipeline,
    sky_pipeline: wgpu::RenderPipeline,
    post_layout: wgpu::BindGroupLayout,
    post_pipeline: wgpu::RenderPipeline,
    grid_pipeline: wgpu::RenderPipeline,
    line_pipeline: wgpu::RenderPipeline,
    models: ModelPipelines,
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
        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("sun shadow sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });
        let uniform_entry = wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("frame layout"),
            entries: &[
                uniform_entry,
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
            ],
        });
        // Drawing the shadow map can't read the shadow map, so it gets a
        // layout with the frame data alone.
        let shadow_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("shadow frame layout"),
            entries: &[uniform_entry],
        });
        let shadow_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("shadow layout"),
                bind_group_layouts: &[Some(&shadow_layout)],
                immediate_size: 0,
            });
        let shadow_pipeline =
            crate::gpu_model::create_shadow_pipeline(device, &shadow_pipeline_layout, &shader);
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("viewport layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let pipeline = |kind| create_pipeline(device, &layout, &shader, sample_count, kind);
        let post_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("last pass layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }],
        });
        let post_pipeline = create_post_pipeline(device, &bind_group_layout, &post_layout, &shader);
        let axis_vertices = ground_axes();
        let axes = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ground axes"),
            contents: bytemuck::cast_slice(&axis_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        Self {
            sample_count,
            sky_pipeline: pipeline(Kind::Sky),
            post_layout,
            post_pipeline,
            grid_pipeline: pipeline(Kind::Grid),
            line_pipeline: pipeline(Kind::Lines),
            models: ModelPipelines::new(device, &bind_group_layout, &shader, sample_count),
            frame_layout: bind_group_layout,
            shadow_layout,
            shadow_sampler,
            shadow_pipeline,
            axes,
            axis_count: axis_vertices.len() as u32,
        }
    }

    /// Uploads `model` to the GPU, ready to pass to [`Self::render`]. Done
    /// once per model, not every frame.
    pub fn upload_model(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        model: &assets::Model,
    ) -> GpuModel {
        self.models.upload(device, queue, model)
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
        let hdr_view = texture(
            "viewport HDR",
            HDR_FORMAT,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            1,
        )
        .create_view(&wgpu::TextureViewDescriptor::default());
        let msaa_view = (self.sample_count > 1).then(|| {
            texture(
                "viewport HDR (multisampled)",
                HDR_FORMAT,
                wgpu::TextureUsages::RENDER_ATTACHMENT,
                self.sample_count,
            )
            .create_view(&wgpu::TextureViewDescriptor::default())
        });
        let post_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("last pass bind group"),
            layout: &self.post_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&hdr_view),
            }],
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
            frame: FrameResources::new(
                device,
                &self.frame_layout,
                &self.shadow_layout,
                &self.shadow_sampler,
            ),
            size,
            msaa_view,
            hdr_view,
            post_bind_group,
            resolved,
            resolved_view,
            depth_view,
        }
    }

    /// The pipelines for copies wound as made, or mirrored.
    fn facing(&self, mirrored: bool) -> &crate::gpu_model::Facing {
        if mirrored {
            &self.models.mirrored
        } else {
            &self.models.normal
        }
    }

    /// Draws every batch whose mesh belongs in `kind`.
    fn draw_batches<'m>(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        batches: &[crate::instances::Batch],
        model_of: &impl Fn(&crate::instances::Batch) -> &'m GpuModel,
        kind: Pass,
    ) {
        for batch in batches {
            let model = model_of(batch);
            if model.pass_of(batch.mesh) != Some(kind) {
                continue;
            }
            pass.set_pipeline(self.facing(batch.mirrored).for_pass(kind));
            model.draw(pass, batch.mesh, batch.instances.clone());
        }
    }

    /// Draws one frame into `target`, with the models `placed` lit by the
    /// sun in `params` and by `lights`, and submits it. Never waits for the
    /// GPU to finish.
    pub fn render(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target: &mut ViewportTarget,
        params: &FrameParams,
        placed: &[Placement<'_>],
        lights: &[Light],
    ) {
        let (lamps, lamp_count) = LightsUniform::new(lights, params.camera_position);
        let shadow_box = crate::instances::world_bounds(placed);
        let (raw, batches, batch_models) = crate::instances::gather(placed);
        target.frame.write_instances(device, queue, &raw);
        let target = &*target;
        let model_of = |b: &crate::instances::Batch| placed[batch_models[b.model]].model;
        let shadows_drawn = params.sun.shadows
            && params.shading.look != crate::frame::Look::Unlit
            && shadow_box.is_some();
        let mut light = FrameLight {
            lamps: lamp_count,
            shadows_drawn,
            ..FrameLight::default()
        };
        if let Some((min, max)) = shadow_box.filter(|_| shadows_drawn) {
            // Down to the ground, so shadows reach the grid below models.
            let min = min.min(Vec3::new(min.x, min.y, 0.0));
            let max = max.max(Vec3::new(max.x, max.y, 0.0));
            (light.sun_view_projection, light.texel) = sun_view_projection(&params.sun, min, max);
        }
        let frame = &target.frame;
        queue.write_buffer(
            &frame.uniform_buffer,
            0,
            bytemuck::bytes_of(&FrameUniforms::new(params, &light)),
        );
        queue.write_buffer(&frame.lights_buffer, 0, bytemuck::bytes_of(&lamps));
        let marked = &lights[..lights.len().min(MAX_MARKED_LAMPS)];
        let markers = light_markers(marked);
        if !markers.is_empty() {
            queue.write_buffer(&frame.markers, 0, bytemuck::cast_slice(&markers));
        }
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("viewport frame"),
        });
        if shadows_drawn {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("sun shadow map"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &frame.shadow_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &frame.shadow_bind_group, &[]);
            pass.set_pipeline(&self.shadow_pipeline);
            if let Some(instances) = &frame.instances {
                pass.set_vertex_buffer(1, instances.slice(..));
                for batch in &batches {
                    model_of(batch).draw_depth(&mut pass, batch.mesh, batch.instances.clone());
                }
            }
        }
        // With multisampling, drawing goes into the multisampled image,
        // which is smoothed into the single-sample HDR one at the end.
        let (view, resolve_target) = match &target.msaa_view {
            Some(msaa) => (msaa, Some(&target.hdr_view)),
            None => (&target.hdr_view, None),
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
                        load: wgpu::LoadOp::Clear(0.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &frame.bind_group, &[]);
            pass.set_pipeline(&self.sky_pipeline);
            pass.draw(0..3, 0..1);
            if let Some(instances) = &frame.instances {
                pass.set_vertex_buffer(1, instances.slice(..));
            }
            // Solid shapes first, so the see-through grid and axes are
            // hidden behind them; blended shapes last, over everything.
            self.draw_batches(&mut pass, &batches, &model_of, Pass::Solid);
            self.draw_batches(&mut pass, &batches, &model_of, Pass::SolidDoubleSided);
            if params.shading.outlines() {
                for batch in &batches {
                    let facing = self.facing(batch.mirrored);
                    pass.set_pipeline(&facing.outline);
                    model_of(batch).draw_outline(&mut pass, batch.mesh, batch.instances.clone());
                }
            }
            pass.set_pipeline(&self.grid_pipeline);
            pass.draw(0..6, 0..1);
            pass.set_pipeline(&self.line_pipeline);
            pass.set_vertex_buffer(0, self.axes.slice(..));
            pass.draw(0..self.axis_count, 0..1);
            if !markers.is_empty() {
                pass.set_vertex_buffer(0, frame.markers.slice(..));
                pass.draw(0..markers.len() as u32, 0..1);
            }
            if let Some(instances) = &frame.instances {
                pass.set_vertex_buffer(1, instances.slice(..));
            }
            self.draw_batches(&mut pass, &batches, &model_of, Pass::Blended);
        }
        // The last pass: exposure, tone mapping and sRGB, into the
        // finished image.
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("viewport last pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target.resolved_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.post_pipeline);
            pass.set_bind_group(0, &frame.bind_group, &[]);
            pass.set_bind_group(2, &target.post_bind_group, &[]);
            pass.draw(0..3, 0..1);
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
