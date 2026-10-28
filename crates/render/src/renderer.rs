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
    Light, LightsUniform, MARKER_VERTICES, SHADOW_FORMAT, SHADOW_MAP_SIZE, SHADOW_REACH,
    light_markers, sun_view_fitted,
};
use crate::timing::FrameStats;
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
    /// The picking images, made the first time something is picked.
    pick: Option<crate::pick::PickImages>,
    /// This viewport's own frame data, lamps, shadow map and lamp
    /// markers, so several viewports never overwrite each other's.
    frame: FrameResources,
    /// Times the GPU's passes, once the first frame has asked (`None`
    /// inside: the card can't).
    timer: Option<Option<crate::timing::GpuTimer>>,
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
    /// What the instance buffer holds now, so a frame sends only what
    /// changed.
    sent: Vec<crate::instances::InstanceRaw>,
    /// Each placement's level of detail last frame.
    levels: Vec<u8>,
    /// Every projectile, filled each frame; replaced by a bigger one
    /// when a frame has more.
    bolts: Option<wgpu::Buffer>,
    /// Every sprite, filled each frame; replaced by a bigger one when a
    /// frame has more.
    sprites: Option<wgpu::Buffer>,
}

/// How many copies are compared and sent together: a run that changed
/// anywhere is sent whole.
const SENT_TOGETHER: usize = 512;

impl FrameResources {
    /// Puts every run's sprites into the sprite buffer, one after another,
    /// growing it if needed. Returns where each run's start in it (none if
    /// too many for one buffer).
    fn write_sprites(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        runs: &[crate::sprites::SpriteRun<'_>],
    ) -> Option<Vec<u32>> {
        let size = std::mem::size_of::<crate::sprites::SpriteInstance>() as u64;
        let total: u64 = runs.iter().map(|r| r.instances.len() as u64).sum();
        let bytes = total * size;
        let most = device.limits().max_buffer_size;
        if total == 0 || bytes > most || total > u64::from(u32::MAX) {
            return (total == 0).then(Vec::new);
        }
        if !self.sprites.as_ref().is_some_and(|b| b.size() >= bytes) {
            let size = bytes.next_power_of_two().max(4096).min(most);
            self.sprites = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("sprites"),
                size,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
        }
        let buffer = self.sprites.as_ref()?;
        let mut starts = Vec::with_capacity(runs.len());
        let mut at = 0u64;
        for run in runs {
            starts.push(at as u32);
            if !run.instances.is_empty() {
                queue.write_buffer(buffer, at * size, bytemuck::cast_slice(run.instances));
            }
            at += run.instances.len() as u64;
        }
        Some(starts)
    }

    /// Puts `bolts` into the projectile buffer, growing it if needed.
    /// Returns how many will be drawn (none if too many for one buffer).
    fn write_bolts(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bolts: &[crate::projectiles::Bolt],
    ) -> u32 {
        let bytes: &[u8] = bytemuck::cast_slice(bolts);
        let most = device.limits().max_buffer_size;
        if bytes.is_empty() || bytes.len() as u64 > most {
            return 0;
        }
        if !self
            .bolts
            .as_ref()
            .is_some_and(|b| b.size() >= bytes.len() as u64)
        {
            let size = (bytes.len() as u64).next_power_of_two().max(4096).min(most);
            self.bolts = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("projectiles"),
                size,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
        }
        match &self.bolts {
            Some(buffer) => {
                queue.write_buffer(buffer, 0, bytes);
                bolts.len() as u32
            }
            None => 0,
        }
    }

    /// Puts `raw` into the instance buffer, growing it if needed, sending
    /// only the runs that differ from what it holds. `None`: too many for
    /// one buffer; otherwise the bytes sent.
    fn write_instances(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        raw: Vec<crate::instances::InstanceRaw>,
    ) -> Option<u64> {
        use rayon::prelude::*;
        let bytes: &[u8] = bytemuck::cast_slice(&raw);
        if bytes.is_empty() {
            return Some(0);
        }
        // More copies than one buffer can hold would be a GPU error, which
        // stops the program: they aren't drawn instead.
        let most = device.limits().max_buffer_size;
        if bytes.len() as u64 > most {
            self.sent.clear();
            return None;
        }
        let fits = self
            .instances
            .as_ref()
            .is_some_and(|b| b.size() >= bytes.len() as u64);
        if !fits {
            // Room to grow, so a slowly growing scene isn't re-made every
            // frame.
            let size = (bytes.len() as u64).next_power_of_two().max(4096).min(most);
            self.instances = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("instances"),
                size,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            self.sent.clear();
        }
        let Some(buffer) = &self.instances else {
            return Some(0);
        };
        let size = std::mem::size_of::<crate::instances::InstanceRaw>();
        let held: &[u8] = bytemuck::cast_slice(&self.sent);
        let run = SENT_TOGETHER * size;
        let changed: Vec<bool> = bytes
            .par_chunks(run)
            .enumerate()
            .map(|(i, chunk)| held.get(i * run..i * run + chunk.len()) != Some(chunk))
            .collect();
        let mut total = 0;
        let mut i = 0;
        while i < changed.len() {
            if !changed[i] {
                i += 1;
                continue;
            }
            let first = i;
            while i < changed.len() && changed[i] {
                i += 1;
            }
            let span = first * run..(i * run).min(bytes.len());
            queue.write_buffer(buffer, span.start as u64, &bytes[span.clone()]);
            total += span.len() as u64;
        }
        self.sent = raw;
        Some(total)
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
            sent: Vec::new(),
            levels: Vec::new(),
            bolts: None,
            sprites: None,
        }
    }
}

impl ViewportTarget {
    /// How long the GPU took over this viewport's newest frame whose times
    /// have come back, a frame or two after it was drawn; `None` before
    /// any, or on a card without timestamps. Never waits.
    pub fn gpu_times(&self, device: &wgpu::Device) -> Option<crate::timing::GpuTimes> {
        self.timer.as_ref()?.as_ref()?.poll(device)
    }

    /// Whether this card can time its passes (known after the first
    /// frame).
    pub fn can_time(&self) -> Option<bool> {
        self.timer.as_ref().map(Option::is_some)
    }

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

    /// The answer to [`ViewportRenderer::request_pick`], once the GPU has
    /// it (usually the frame after). Never waits.
    pub fn poll_pick(&mut self, device: &wgpu::Device) -> Option<crate::PickResult> {
        self.pick.as_mut()?.poll(device)
    }

    /// Whether a pick asked for is still waiting for its answer.
    pub fn is_picking(&self) -> bool {
        self.pick.as_ref().is_some_and(|p| p.is_waiting())
    }
}

/// The box around what `view_projection` sees (depth reversed: 1 near, 0
/// far), up to `reach` from its near side for a perspective view; `None`
/// for a view that can't be undone.
pub(crate) fn view_box(view_projection: glam::Mat4, reach: f32) -> Option<(Vec3, Vec3)> {
    let inverse = view_projection.inverse();
    if !inverse.is_finite() {
        return None;
    }
    let perspective = view_projection.row(3).truncate().length_squared() > 1e-12;
    let mut lo = Vec3::splat(f32::MAX);
    let mut hi = Vec3::splat(f32::MIN);
    for (x, y) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        let near = inverse.project_point3(Vec3::new(x, y, 1.0));
        let far = inverse.project_point3(Vec3::new(x, y, 0.0));
        let far = if perspective {
            let deeper = inverse.project_point3(Vec3::new(x, y, 0.5));
            near + (deeper - near).try_normalize()? * reach
        } else {
            far
        };
        if !(near.is_finite() && far.is_finite()) {
            return None;
        }
        lo = lo.min(near).min(far);
        hi = hi.max(near).max(far);
    }
    Some((lo, hi))
}

/// Where two boxes overlap, if they do.
pub(crate) fn overlap(a: (Vec3, Vec3), b: (Vec3, Vec3)) -> Option<(Vec3, Vec3)> {
    let (lo, hi) = (a.0.max(b.0), a.1.min(b.1));
    (lo.cmple(hi).all()).then_some((lo, hi))
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

/// How long each piece of the ground axes is, in metres. One line from end
/// to end of the grid passes behind a nearby camera, and some GPUs (Mesa's
/// software Vulkan driver, for one) then draw it as a filled wedge; short
/// pieces are clipped properly everywhere.
const AXIS_PIECE: f32 = 10.0;

/// The X and Y axes along the ground, as line pairs (X first), as far as
/// the grid reaches, in short pieces.
pub fn ground_axes() -> Vec<LineVertex> {
    let h = GRID_HALF_EXTENT;
    let pieces = (2.0 * h / AXIS_PIECE).ceil() as usize;
    let mut lines = Vec::with_capacity(pieces * 4);
    for (along, color) in [(Vec3::X, AXIS_X_COLOR), (Vec3::Y, AXIS_Y_COLOR)] {
        for i in 0..pieces {
            let a = -h + i as f32 * AXIS_PIECE;
            let b = (a + AXIS_PIECE).min(h);
            for t in [a, b] {
                lines.push(LineVertex {
                    position: (along * t).to_array(),
                    color,
                });
            }
        }
    }
    lines
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
    bolt_pipeline: wgpu::RenderPipeline,
    models: ModelPipelines,
    axes: wgpu::Buffer,
    axis_count: u32,
    pick: crate::pick::PickPipeline,
    pixel: crate::pixel::PixelPipeline,
    sprites: crate::sprites::SpritePipelines,
}

/// Everything one frame draws.
#[derive(Debug, Clone, Copy, Default)]
pub struct Scene<'a> {
    /// The models, where they are.
    pub placed: &'a [Placement<'a>],
    /// The lamps.
    pub lights: &'a [Light],
    /// Projectiles, drawn in one go.
    pub bolts: &'a [crate::projectiles::Bolt],
    /// Sprites, a run per sheet (Phase 9, D2).
    pub sprites: &'a [crate::sprites::SpriteRun<'a>],
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
        let sprites =
            crate::sprites::SpritePipelines::new(device, &bind_group_layout, &shader, sample_count);
        Self {
            sample_count,
            sky_pipeline: pipeline(Kind::Sky),
            post_layout,
            post_pipeline,
            grid_pipeline: pipeline(Kind::Grid),
            line_pipeline: pipeline(Kind::Lines),
            bolt_pipeline: crate::projectiles::pipeline(device, &layout, &shader, sample_count),
            models: ModelPipelines::new(device, &bind_group_layout, &shader, sample_count),
            frame_layout: bind_group_layout,
            shadow_layout,
            shadow_sampler,
            shadow_pipeline,
            axes,
            axis_count: axis_vertices.len() as u32,
            pick: crate::pick::PickPipeline::new(device, &sprites.sheet_layout),
            pixel: crate::pixel::PixelPipeline::new(device),
            sprites,
        }
    }

    /// Uploads a sprite sheet's atlas (`size` texels, straight RGBA rows,
    /// as `assets::sprite_sheet` packs it), ready to draw sprites from.
    /// Done once per sheet (and again when it changes).
    pub fn upload_sheet(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        size: [u32; 2],
        rgba: &[u8],
    ) -> crate::sprites::GpuSheet {
        self.sprites.upload(device, queue, size, rgba)
    }

    /// Makes the screen-sized image a 2D game's small picture is scaled up
    /// into ([`Self::draw_pixels`]). Sizes are clamped as for
    /// [`Self::create_target`].
    pub fn create_pixel_image(
        &self,
        device: &wgpu::Device,
        size: [u32; 2],
    ) -> crate::pixel::PixelImage {
        crate::pixel::PixelImage::new(device, size)
    }

    /// The pixel stage: scales `source` (a 2D game drawn at its base
    /// resolution plus a pixel each side, by [`Self::render`]) up into
    /// `image` as `layout` says, and submits it. Never waits for the GPU.
    pub fn draw_pixels(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        source: &ViewportTarget,
        image: &crate::pixel::PixelImage,
        layout: &crate::pixel::PixelLayout,
    ) {
        image.draw(device, queue, &self.pixel, source.display_view(), layout);
    }

    /// Asks what is drawn inside `rect` of `target` (x, y, width and
    /// height in image pixels), seen through `view_projection`: each
    /// pixel's placement number ([`Placement::pick`]) and depth. Collect
    /// the answer with [`ViewportTarget::poll_pick`]. Returns `false`, and
    /// asks nothing, while the last question is still waiting or when the
    /// rectangle is empty. Never waits for the GPU.
    pub fn request_pick(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target: &mut ViewportTarget,
        view_projection: glam::Mat4,
        placed: &[Placement<'_>],
        rect: [u32; 4],
    ) -> bool {
        self.request_pick_with_sprites(device, queue, target, view_projection, placed, &[], rect)
    }

    /// [`Self::request_pick`], with sprites too: each is found by its own
    /// pixels, so the clear space round a sprite picks what's behind.
    #[allow(clippy::too_many_arguments)]
    pub fn request_pick_with_sprites(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target: &mut ViewportTarget,
        view_projection: glam::Mat4,
        placed: &[Placement<'_>],
        sprites: &[crate::SpritePicks<'_>],
        rect: [u32; 4],
    ) -> bool {
        let size = target.size;
        let images = target
            .pick
            .get_or_insert_with(|| crate::pick::PickImages::new(device, &self.pick, size));
        images.request(
            device,
            queue,
            &self.pick,
            view_projection,
            placed,
            sprites,
            rect,
        )
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
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
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
            pick: None,
            timer: None,
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
        for batch in batches.iter().filter(|b| b.seen) {
            let model = model_of(batch);
            if model.pass_of(batch.mesh) != Some(kind) {
                continue;
            }
            pass.set_pipeline(self.facing(batch.mirrored).for_pass(kind));
            model.draw(pass, batch.mesh, batch.lod, batch.instances.clone());
        }
    }

    /// Draws one frame into `target`, with the models `placed` lit by the
    /// sun in `params` and by `lights`, and submits it, saying what it
    /// drew. Never waits for the GPU to finish.
    pub fn render(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target: &mut ViewportTarget,
        params: &FrameParams,
        placed: &[Placement<'_>],
        lights: &[Light],
    ) -> FrameStats {
        self.render_with_bolts(device, queue, target, params, placed, lights, &[])
    }

    /// [`Self::render`], with projectiles too: `bolts`, drawn in one go.
    #[allow(clippy::too_many_arguments)]
    pub fn render_with_bolts(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target: &mut ViewportTarget,
        params: &FrameParams,
        placed: &[Placement<'_>],
        lights: &[Light],
        bolts: &[crate::projectiles::Bolt],
    ) -> FrameStats {
        self.render_scene(
            device,
            queue,
            target,
            params,
            &Scene {
                placed,
                lights,
                bolts,
                sprites: &[],
            },
        )
    }

    /// Draws one frame of `scene` into `target`, and submits it, saying
    /// what it drew. Never waits for the GPU to finish.
    pub fn render_scene(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target: &mut ViewportTarget,
        params: &FrameParams,
        scene: &Scene<'_>,
    ) -> FrameStats {
        let Scene {
            placed,
            lights,
            bolts,
            sprites,
        } = *scene;
        if target.timer.is_none() {
            target.timer = Some(crate::timing::GpuTimer::new(device, queue));
        }
        let (lamps, lamp_count) = LightsUniform::new(lights, params.camera_position);
        // Shadows fall in the part of the world in view, up to
        // SHADOW_REACH away; what can cast into it is drawn into the sun's
        // shadow map, and only what's in view is drawn.
        let shadows_wanted = params.sun.shadows && params.shading.look != crate::frame::Look::Unlit;
        let light = params.sun.direction.try_normalize().unwrap_or(-Vec3::Z);
        let receivers = shadows_wanted
            .then(|| crate::instances::world_bounds(placed))
            .flatten()
            .and_then(|(min, max)| {
                // Down to the ground, so shadows reach the grid below
                // models.
                let world = (min.min(min.with_z(0.0)), max.max(max.with_z(0.0)));
                overlap(world, view_box(params.view_projection, SHADOW_REACH)?)
            });
        let cull = crate::instances::Cull::new(
            params.view_projection,
            receivers.map(|(min, max)| (min, max, light)),
        )
        .with_detail(params.view_projection, params.size_px[1]);
        let crate::instances::Gathered {
            raw,
            mut batches,
            models: batch_models,
            seen,
            casting,
            casters,
            levels,
        } = crate::instances::gather(placed, Some(&cull), &target.frame.levels);
        target.frame.levels = levels;
        let count = raw.len();
        let sent = target.frame.write_instances(device, queue, raw);
        let bolt_count = target.frame.write_bolts(device, queue, bolts);
        let sprite_starts = target.frame.write_sprites(device, queue, sprites);
        if sprite_starts.is_none() {
            log::warn!(
                "more sprites than the graphics card can draw at once; sprites aren't drawn"
            );
        }
        if sent.is_none() {
            log::warn!(
                "{count} model copies are more than the graphics card can draw at once; models \
                 aren't drawn"
            );
            batches.clear();
        }
        let target = &*target;
        let model_of = |b: &crate::instances::Batch| placed[batch_models[b.model]].model;
        let stats = FrameStats {
            instances: seen,
            casting,
            batches: batches.len() as u32,
            triangles: batches
                .iter()
                .filter(|b| b.seen)
                .map(|b| {
                    u64::from(model_of(b).triangles_of(b.mesh, b.lod))
                        * u64::from(b.instances.end - b.instances.start)
                })
                .sum(),
            lods: batches
                .iter()
                .filter(|b| b.seen)
                .fold([0; 5], |mut lods, b| {
                    lods[b.lod.min(4)] += b.instances.end - b.instances.start;
                    lods
                }),
            sent_bytes: sent.unwrap_or(0)
                + u64::from(bolt_count) * 40
                + sprites
                    .iter()
                    .map(|r| r.instances.len() as u64)
                    .sum::<u64>()
                    * 88,
            bolts: bolt_count,
            sprites: sprites.iter().map(|r| r.instances.len() as u32).sum(),
        };
        let timer = target.timer.as_ref().and_then(Option::as_ref);
        let shadows_drawn = receivers.is_some() && casters.is_some() && sent.is_some();
        let mut light = FrameLight {
            lamps: lamp_count,
            shadows_drawn,
            ..FrameLight::default()
        };
        if let Some(receivers) = receivers.filter(|_| shadows_drawn) {
            (light.sun_view_projection, light.texel) =
                sun_view_fitted(&params.sun, receivers, casters);
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
        if let Some(timer) = timer {
            timer.begin(shadows_drawn);
        }
        let stamps = |pass| timer.and_then(|t| t.writes(pass));
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
                timestamp_writes: stamps(crate::timing::TimedPass::Shadows),
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &frame.shadow_bind_group, &[]);
            pass.set_pipeline(&self.shadow_pipeline);
            if let Some(instances) = &frame.instances {
                pass.set_vertex_buffer(1, instances.slice(..));
                for batch in batches.iter().filter(|b| b.casts) {
                    model_of(batch).draw_depth(
                        &mut pass,
                        batch.mesh,
                        batch.lod,
                        batch.instances.clone(),
                    );
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
                timestamp_writes: stamps(crate::timing::TimedPass::Models),
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &frame.bind_group, &[]);
            pass.set_pipeline(&self.sky_pipeline);
            pass.draw(0..3, 0..1);
            use crate::sprites::SpriteLayer;
            let draw_sprites = |pass: &mut wgpu::RenderPass<'_>, layer: SpriteLayer| {
                let (Some(buffer), Some(starts)) = (frame.sprites.as_ref(), sprite_starts.as_ref())
                else {
                    return;
                };
                if !sprites
                    .iter()
                    .any(|r| r.instances.first().is_some_and(|f| f.layer() == layer))
                {
                    return;
                }
                pass.set_pipeline(match layer {
                    SpriteLayer::CutOut => &self.sprites.cut_out,
                    SpriteLayer::SeeThrough => &self.sprites.blended,
                    SpriteLayer::Under | SpriteLayer::Over => &self.sprites.flat,
                });
                pass.set_vertex_buffer(0, buffer.slice(..));
                for (run, &start) in sprites.iter().zip(starts) {
                    // A run is drawn whole by one pipeline, by its first
                    // sprite (the caller keeps runs apart).
                    let Some(first) = run.instances.first() else {
                        continue;
                    };
                    if first.layer() != layer {
                        continue;
                    }
                    pass.set_bind_group(3, run.sheet.bind_group(), &[]);
                    pass.draw(0..6, start..start + run.instances.len() as u32);
                }
            };
            // A tile map's ground first, under everything (Phase 9, D4).
            draw_sprites(&mut pass, SpriteLayer::Under);
            if let Some(instances) = &frame.instances {
                pass.set_vertex_buffer(1, instances.slice(..));
            }
            // Solid shapes first, so the see-through grid and axes are
            // hidden behind them; blended shapes last, over everything.
            self.draw_batches(&mut pass, &batches, &model_of, Pass::Solid);
            self.draw_batches(&mut pass, &batches, &model_of, Pass::SolidDoubleSided);
            if params.shading.outlines() {
                for batch in batches.iter().filter(|b| b.seen) {
                    let facing = self.facing(batch.mirrored);
                    pass.set_pipeline(&facing.outline);
                    model_of(batch).draw_outline(
                        &mut pass,
                        batch.mesh,
                        batch.lod,
                        batch.instances.clone(),
                    );
                }
            }
            // Cut-out sprites write depth like solids, so the grid hides
            // behind them too.
            draw_sprites(&mut pass, SpriteLayer::CutOut);
            if params.editor_lines {
                pass.set_pipeline(&self.grid_pipeline);
                pass.draw(0..6, 0..1);
                pass.set_pipeline(&self.line_pipeline);
                pass.set_vertex_buffer(0, self.axes.slice(..));
                pass.draw(0..self.axis_count, 0..1);
                if !markers.is_empty() {
                    pass.set_vertex_buffer(0, frame.markers.slice(..));
                    pass.draw(0..markers.len() as u32, 0..1);
                }
            }
            if let Some(instances) = &frame.instances {
                pass.set_vertex_buffer(1, instances.slice(..));
            }
            self.draw_batches(&mut pass, &batches, &model_of, Pass::Blended);
            if let Some(buffer) = frame.bolts.as_ref().filter(|_| bolt_count > 0) {
                pass.set_pipeline(&self.bolt_pipeline);
                pass.set_vertex_buffer(0, buffer.slice(..));
                pass.draw(0..6, 0..bolt_count);
            }
            draw_sprites(&mut pass, SpriteLayer::SeeThrough);
            draw_sprites(&mut pass, SpriteLayer::Over);
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
                timestamp_writes: stamps(crate::timing::TimedPass::LastPass),
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.post_pipeline);
            pass.set_bind_group(0, &frame.bind_group, &[]);
            pass.set_bind_group(2, &target.post_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        if let Some(timer) = timer {
            timer.finish(&mut encoder);
        }
        queue.submit([encoder.finish()]);
        if let Some(timer) = timer {
            timer.submitted();
        }
        stats
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
        let (x, y): (Vec<&LineVertex>, Vec<&LineVertex>) =
            axes.iter().partition(|v| v.color == AXIS_X_COLOR);
        assert!(x.iter().all(|v| v.position[1] == 0.0), "X runs along y = 0");
        assert!(y.iter().all(|v| v.color == AXIS_Y_COLOR));
        assert!(y.iter().all(|v| v.position[0] == 0.0), "Y runs along x = 0");
        // From one end of the grid to the other, in short pieces.
        let ends = |line: &[&LineVertex], i: usize| {
            line.iter()
                .map(|v| v.position[i])
                .fold((f32::MAX, f32::MIN), |(lo, hi), p| (lo.min(p), hi.max(p)))
        };
        assert_eq!(ends(&x, 0), (-GRID_HALF_EXTENT, GRID_HALF_EXTENT));
        assert_eq!(ends(&y, 1), (-GRID_HALF_EXTENT, GRID_HALF_EXTENT));
        for pair in axes.chunks(2) {
            let length = Vec3::from(pair[0].position).distance(Vec3::from(pair[1].position));
            assert!(length <= AXIS_PIECE + 1e-3);
        }
    }
}
