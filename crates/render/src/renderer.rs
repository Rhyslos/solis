//! Drawing a viewport into an offscreen image on the GPU: the backdrop,
//! models, the grid and the axes.
//!
//! Each viewport renders into its own [`ViewportTarget`] (colour and depth
//! images, with multisampling when the GPU supports it). The caller then
//! shows the finished colour image however it likes; the editor hands it to
//! egui as a texture.

use crate::frame::{FrameParams, GRID_HALF_EXTENT};
use crate::gpu_model::{GpuModel, ModelPipelines};
use crate::instances::Placement;
use crate::lighting::Light;
use crate::timing::FrameStats;
use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use wgpu::util::DeviceExt;

#[path = "frame_resources.rs"]
mod frame_resources;
#[path = "passes.rs"]
mod passes;
#[path = "pipelines.rs"]
mod pipelines;
#[path = "target.rs"]
mod target;

use frame_resources::depth_image;
use pipelines::{Kind, create_pipeline, create_post_pipeline};
pub use target::ViewportTarget;

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

/// Draws viewports: the pipelines and buffers they share.
#[derive(Debug)]
pub struct ViewportRenderer {
    sample_count: u32,
    frame_layout: wgpu::BindGroupLayout,
    shadow_layout: wgpu::BindGroupLayout,
    shadow_sampler: wgpu::Sampler,
    /// A 1 × 1 depth image bound for the shadow map by every viewport
    /// that hasn't drawn shadows yet (the shaders don't read it then).
    no_shadows: wgpu::TextureView,
    shadow_pipeline: wgpu::RenderPipeline,
    sky_pipeline: wgpu::RenderPipeline,
    post_layout: wgpu::BindGroupLayout,
    post_pipeline: wgpu::RenderPipeline,
    grid_pipeline: wgpu::RenderPipeline,
    line_pipeline: wgpu::RenderPipeline,
    bolt_pipeline: wgpu::RenderPipeline,
    /// [`Self::bolt_pipeline`] for a layer's picture (decision 0094).
    bolt_pipeline_layered: wgpu::RenderPipeline,
    models: ModelPipelines,
    axes: wgpu::Buffer,
    axis_count: u32,
    pick: crate::pick::PickPipeline,
    pixel: crate::pixel::PixelPipeline,
    ui: crate::ui::UiPipeline,
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
            bolt_pipeline: crate::projectiles::pipeline(
                device,
                &layout,
                &shader,
                sample_count,
                false,
            ),
            bolt_pipeline_layered: crate::projectiles::pipeline(
                device,
                &layout,
                &shader,
                sample_count,
                true,
            ),
            models: ModelPipelines::new(device, &bind_group_layout, &shader, sample_count),
            frame_layout: bind_group_layout,
            shadow_layout,
            shadow_sampler,
            no_shadows: depth_image(device, "no shadow map", 1),
            shadow_pipeline,
            axes,
            axis_count: axis_vertices.len() as u32,
            pick: crate::pick::PickPipeline::new(device, &sprites.sheet_layout),
            pixel: crate::pixel::PixelPipeline::new(device),
            ui: crate::ui::UiPipeline::new(device, COLOR_FORMAT),
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

    /// Scales several small pictures into `image`, each as its own layout
    /// says (its own slide), laid on each other in order (decision 0094):
    /// the first as it is, the rest (layer pictures, see
    /// [`ViewportTarget::set_layered`]) over it by how covered each pixel
    /// is. Submits it; never waits for the GPU.
    pub fn draw_pixel_layers(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        sources: &[(&ViewportTarget, crate::pixel::PixelLayout)],
        image: &crate::pixel::PixelImage,
    ) {
        let views: Vec<(&wgpu::TextureView, crate::pixel::PixelLayout)> = sources
            .iter()
            .map(|(target, layout)| (target.display_view(), *layout))
            .collect();
        image.draw_layers(device, queue, &self.pixel, &views);
    }

    /// Draws game UI's `shapes` over `target`'s finished picture (decision
    /// 0083), after tone mapping, with letters from `atlas` and pictures
    /// from `pictures` (decision 0089), and submits it. Never waits for the
    /// GPU.
    pub fn draw_ui(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target: &ViewportTarget,
        shapes: &[ui::Shape],
        atlas: Option<&ui::text::Atlas>,
        pictures: Option<&ui::Pictures>,
    ) {
        self.ui.draw(
            device,
            queue,
            target.display_view(),
            target.size(),
            shapes,
            atlas,
            pictures,
        );
    }

    /// Draws game UI's `shapes` over a 2D game's scaled-up picture.
    pub fn draw_ui_on_pixels(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        image: &crate::pixel::PixelImage,
        shapes: &[ui::Shape],
        atlas: Option<&ui::text::Atlas>,
        pictures: Option<&ui::Pictures>,
    ) {
        self.ui.draw(
            device,
            queue,
            image.display_view(),
            image.size(),
            shapes,
            atlas,
            pictures,
        );
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
}

#[cfg(test)]
#[path = "renderer_tests.rs"]
mod tests;
