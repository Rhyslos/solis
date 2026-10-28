//! The pixel stage (Phase 9, D1; [the 2D plan](../../../docs/plans/2d.md)):
//! a 2D game is drawn small, at its base resolution (say 480 × 270) with
//! a one-pixel margin all round, then scaled up to the screen here.
//!
//! - **Whole steps:** every pixel becomes the same whole number of screen
//!   pixels (×4 on a 1080p screen), with black bars when the screen
//!   doesn't divide evenly. Every screen pixel shows exactly one of the
//!   game's pixels: never a blend.
//! - **Fill:** scaled to fill the screen; within each of the game's
//!   pixels only the last screen pixel blends into the next ("sharp
//!   bilinear"), so edges stay sharp at sizes that don't divide evenly.
//!
//! **Smooth scrolling without shimmer:** the game's camera stays on whole
//! pixels, so what's drawn never shimmers as it moves; the bit of a pixel
//! it's really moved is given here as the *slide*, and the finished
//! picture moves by that much on the screen (in whole screen pixels for
//! whole steps), using the margin. A camera moving a tenth of a pixel a
//! frame moves the picture smoothly at ×4 and up, and never blurs it.
//!
//! [`PixelLayout`] works the numbers out on the CPU, where tests and the
//! editor (which needs to know where a click lands) can use them; the
//! shader (`pixel.wgsl`) does the same sums per screen pixel.

use bytemuck::{Pod, Zeroable};

/// The pixel stage's shader, in WGSL.
pub const PIXEL_SHADER_SOURCE: &str = include_str!("pixel.wgsl");

/// How the small picture fills the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PixelScaling {
    /// Every pixel scaled the same whole number of times, with bars at the
    /// edges if need be.
    #[default]
    WholeSteps,
    /// Scaled to fill the screen, the last bit between whole steps
    /// smoothed at each pixel's edge.
    Fill,
}

/// Where the small picture goes on the screen, and how it's scaled: what
/// the pixel stage draws, worked out by [`pixel_layout`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PixelLayout {
    /// The base resolution, in pixels (the source is this plus one pixel
    /// each side).
    pub base: [u32; 2],
    /// The screen's size, in pixels.
    pub screen: [u32; 2],
    /// Screen pixels per source pixel.
    pub scale: f32,
    /// Whether each source pixel is a whole number of screen pixels
    /// (`scale` is whole).
    pub whole: bool,
    /// The picture's top-left corner on the screen, in whole pixels.
    pub origin: [u32; 2],
    /// The picture's size on the screen, in whole pixels.
    pub size: [u32; 2],
    /// How far the picture slides across the source: whole screen pixels
    /// when `whole`, otherwise source pixels.
    pub slide: [f32; 2],
}

/// The most a picture may slide, in source pixels, each way: the margin.
pub const MAX_SLIDE: f32 = 1.0;

/// Works out where a picture of `base` pixels goes on a `screen`, scaled
/// by `scaling`, slid by `slide` source pixels (x right, y down; the bit
/// of a pixel the camera has moved beyond the whole pixel it's drawn at,
/// at most [`MAX_SLIDE`] each way).
///
/// A screen smaller than the picture shows it shrunk to fit, smoothed.
pub fn pixel_layout(
    base: [u32; 2],
    screen: [u32; 2],
    scaling: PixelScaling,
    slide: [f32; 2],
) -> PixelLayout {
    let base = base.map(|v| v.max(1));
    let screen = screen.map(|v| v.max(1));
    let slide = slide.map(|s| {
        if s.is_finite() {
            s.clamp(-MAX_SLIDE, MAX_SLIDE)
        } else {
            0.0
        }
    });
    let fits = (screen[0] as f32 / base[0] as f32).min(screen[1] as f32 / base[1] as f32);
    let steps = (screen[0] / base[0]).min(screen[1] / base[1]);
    let whole = steps >= 1 && (scaling == PixelScaling::WholeSteps || fits == steps as f32);
    let scale = if whole { steps as f32 } else { fits };
    let size = if whole {
        [base[0] * steps, base[1] * steps]
    } else {
        [
            ((base[0] as f32 * scale).round() as u32).clamp(1, screen[0]),
            ((base[1] as f32 * scale).round() as u32).clamp(1, screen[1]),
        ]
    };
    let origin = [(screen[0] - size[0]) / 2, (screen[1] - size[1]) / 2];
    let slide = if whole {
        slide.map(|s| (s * scale).round())
    } else {
        slide
    };
    PixelLayout {
        base,
        screen,
        scale,
        whole,
        origin,
        size,
        slide,
    }
}

impl PixelLayout {
    /// For whole steps: the source pixel (counting the margin, so the
    /// picture's first is 1) that screen pixel `x`, `y` shows, or `None`
    /// in the bars. What the shader does, for tests to check it against.
    pub fn source_pixel(&self, x: u32, y: u32) -> Option<[u32; 2]> {
        let at = [
            x as f32 - self.origin[0] as f32,
            y as f32 - self.origin[1] as f32,
        ];
        if at[0] < 0.0
            || at[1] < 0.0
            || at[0] >= self.size[0] as f32
            || at[1] >= self.size[1] as f32
        {
            return None;
        }
        let texel = |i: usize| {
            let t = ((at[i] + self.slide[i]) / self.scale).floor() + 1.0;
            t.clamp(0.0, self.base[i] as f32 + 1.0) as u32
        };
        Some([texel(0), texel(1)])
    }

    /// Where on the screen (in pixels) a point of the picture is, given in
    /// base pixels from its top-left corner: for showing where something
    /// in the game is, or the other way, with [`Self::to_picture`].
    pub fn to_screen(&self, picture: [f32; 2]) -> [f32; 2] {
        let slide = |i: usize| {
            if self.whole {
                self.slide[i]
            } else {
                self.slide[i] * self.scale
            }
        };
        [
            self.origin[0] as f32 + picture[0] * self.scale - slide(0),
            self.origin[1] as f32 + picture[1] * self.scale - slide(1),
        ]
    }

    /// The point of the picture (base pixels from its top-left corner)
    /// under screen point `screen`: where a click lands in the game.
    pub fn to_picture(&self, screen: [f32; 2]) -> [f32; 2] {
        let slide = |i: usize| {
            if self.whole {
                self.slide[i]
            } else {
                self.slide[i] * self.scale
            }
        };
        [
            (screen[0] - self.origin[0] as f32 + slide(0)) / self.scale,
            (screen[1] - self.origin[1] as f32 + slide(1)) / self.scale,
        ]
    }
}

/// The layout exactly as the shader's `Stage` struct expects it.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub(crate) struct StageUniform {
    origin: [f32; 2],
    size: [f32; 2],
    base: [f32; 2],
    scale: f32,
    whole: f32,
    slide: [f32; 2],
    _pad: [f32; 2],
}

impl StageUniform {
    pub(crate) fn new(layout: &PixelLayout) -> Self {
        Self {
            origin: layout.origin.map(|v| v as f32),
            size: layout.size.map(|v| v as f32),
            base: layout.base.map(|v| v as f32),
            scale: layout.scale,
            whole: if layout.whole { 1.0 } else { 0.0 },
            slide: layout.slide,
            _pad: [0.0; 2],
        }
    }
}

/// The pixel stage's pipeline, shared by every [`PixelImage`].
#[derive(Debug)]
pub(crate) struct PixelPipeline {
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
    sampler: wgpu::Sampler,
}

impl PixelPipeline {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("pixel stage shader"),
            source: wgpu::ShaderSource::Wgsl(PIXEL_SHADER_SOURCE.into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("pixel stage layout"),
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
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pixel stage pipeline layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("pixel stage pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("pixel_vs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("pixel_fs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: crate::renderer::COLOR_FORMAT,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("pixel stage sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });
        Self {
            layout,
            pipeline,
            sampler,
        }
    }
}

/// The screen-sized image a 2D game's picture is scaled up into. Create
/// with [`crate::ViewportRenderer::create_pixel_image`].
#[derive(Debug)]
pub struct PixelImage {
    size: [u32; 2],
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    uniform: wgpu::Buffer,
}

impl PixelImage {
    pub(crate) fn new(device: &wgpu::Device, size: [u32; 2]) -> Self {
        let max = device.limits().max_texture_dimension_2d.max(1);
        let size = [size[0].clamp(1, max), size[1].clamp(1, max)];
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("pixel stage image"),
            size: wgpu::Extent3d {
                width: size[0],
                height: size[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: crate::renderer::COLOR_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("pixel stage uniform"),
            size: std::mem::size_of::<StageUniform>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            size,
            texture,
            view,
            uniform,
        }
    }

    /// Its size in pixels.
    pub fn size(&self) -> [u32; 2] {
        self.size
    }

    /// The finished image, ready for showing in egui.
    pub fn display_view(&self) -> &wgpu::TextureView {
        &self.view
    }

    /// The finished image as a texture (for copying and reading back).
    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }

    /// Scales `source` (the small picture: the base resolution plus a
    /// pixel each side) into this image as `layout` says, and submits it.
    /// Never waits for the GPU.
    pub(crate) fn draw(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        pipeline: &PixelPipeline,
        source: &wgpu::TextureView,
        layout: &PixelLayout,
    ) {
        queue.write_buffer(
            &self.uniform,
            0,
            bytemuck::bytes_of(&StageUniform::new(layout)),
        );
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("pixel stage bind group"),
            layout: &pipeline.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(source),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&pipeline.sampler),
                },
            ],
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("pixel stage"),
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("pixel stage"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.view,
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
            pass.set_pipeline(&pipeline.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        queue.submit([encoder.finish()]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREENS: [[u32; 2]; 9] = [
        [1280, 720],
        [1366, 768],
        [1600, 900],
        [1920, 1080],
        [1920, 1200],
        [2560, 1080],
        [2560, 1440],
        [3440, 1440],
        [3840, 2160],
    ];

    #[test]
    fn whole_steps_scale_as_far_as_they_fit_with_bars_round_the_rest() {
        let l = pixel_layout([480, 270], [1920, 1080], PixelScaling::WholeSteps, [0.0; 2]);
        assert!(l.whole);
        assert_eq!((l.scale, l.size, l.origin), (4.0, [1920, 1080], [0, 0]));
        let l = pixel_layout([480, 270], [2560, 1440], PixelScaling::WholeSteps, [0.0; 2]);
        assert_eq!((l.scale, l.size, l.origin), (5.0, [2400, 1350], [80, 45]));
        let l = pixel_layout([480, 270], [3840, 2160], PixelScaling::WholeSteps, [0.0; 2]);
        assert_eq!((l.scale, l.origin), (8.0, [0, 0]));
        for screen in SCREENS {
            let l = pixel_layout([480, 270], screen, PixelScaling::WholeSteps, [0.0; 2]);
            assert!(l.whole && l.scale.fract() == 0.0, "{screen:?}");
            assert!(l.origin[0] + l.size[0] <= screen[0]);
            assert!(l.origin[1] + l.size[1] <= screen[1]);
            // Centred, to the pixel.
            assert!((screen[0] - l.size[0]) / 2 == l.origin[0]);
        }
    }

    #[test]
    fn fill_fills_one_way_and_is_whole_when_it_can_be() {
        let l = pixel_layout([480, 270], [2560, 1440], PixelScaling::Fill, [0.0; 2]);
        assert!(!l.whole);
        assert_eq!(l.size, [2560, 1440]);
        assert!((l.scale - 16.0 / 3.0).abs() < 1e-5);
        // An even fit is drawn as whole steps.
        let l = pixel_layout([480, 270], [1920, 1080], PixelScaling::Fill, [0.0; 2]);
        assert!(l.whole && l.scale == 4.0);
        for screen in SCREENS {
            let l = pixel_layout([480, 270], screen, PixelScaling::Fill, [0.0; 2]);
            assert!(
                l.size[0] == screen[0] || l.size[1] == screen[1],
                "{screen:?}"
            );
        }
    }

    #[test]
    fn every_screen_pixel_shows_one_source_pixel_and_each_is_the_same_size() {
        for screen in SCREENS {
            let l = pixel_layout([480, 270], screen, PixelScaling::WholeSteps, [0.0; 2]);
            let k = l.scale as u32;
            for x in [
                l.origin[0],
                l.origin[0] + k - 1,
                l.origin[0] + k,
                l.origin[0] + l.size[0] - 1,
            ] {
                let [sx, _] = l.source_pixel(x, l.origin[1]).unwrap();
                assert_eq!(sx, (x - l.origin[0]) / k + 1, "{screen:?} x {x}");
            }
            // The bars show nothing.
            if l.origin[0] > 0 {
                assert_eq!(l.source_pixel(l.origin[0] - 1, l.origin[1]), None);
            }
            assert_eq!(l.source_pixel(l.origin[0] + l.size[0], l.origin[1]), None);
        }
    }

    #[test]
    fn a_slide_of_part_of_a_pixel_moves_the_picture_in_whole_screen_pixels() {
        let still = pixel_layout([480, 270], [1920, 1080], PixelScaling::WholeSteps, [0.0; 2]);
        let mut last = 0.0;
        for tenth in 0..=10 {
            let f = tenth as f32 / 10.0;
            let l = pixel_layout([480, 270], [1920, 1080], PixelScaling::WholeSteps, [f, 0.0]);
            // Moves at most one screen pixel more each tenth (×4: 0, 0, 1,
            // 1, 2, …), never back.
            assert!(l.slide[0] >= last && l.slide[0] - last <= 1.0);
            assert_eq!(l.slide[0].fract(), 0.0);
            last = l.slide[0];
            // What screen pixel 40 showed still now shows `slide` screen
            // pixels further left.
            let off = l.slide[0] as u32;
            assert_eq!(l.source_pixel(40 - off, 0), still.source_pixel(40, 0));
        }
        assert_eq!(last, 4.0);
    }

    #[test]
    fn the_margin_covers_a_slide_either_way() {
        for slide in [-1.0, -0.5, 0.5, 1.0] {
            let l = pixel_layout(
                [480, 270],
                [1920, 1080],
                PixelScaling::WholeSteps,
                [slide, slide],
            );
            let first = l.source_pixel(0, 0).unwrap();
            let last = l.source_pixel(1919, 1079).unwrap();
            assert!(
                first[0] <= 2 && last[0] >= 479 && last[0] <= 481,
                "{slide}: {first:?} {last:?}"
            );
        }
    }

    #[test]
    fn small_and_odd_screens_and_silly_numbers_still_give_a_picture() {
        let l = pixel_layout([480, 270], [320, 200], PixelScaling::WholeSteps, [0.0; 2]);
        assert!(!l.whole && l.scale < 1.0 && l.size[0] <= 320 && l.size[1] <= 200);
        let l = pixel_layout(
            [0, 0],
            [0, 0],
            PixelScaling::Fill,
            [f32::NAN, f32::INFINITY],
        );
        assert!(l.scale.is_finite() && l.slide == [0.0, 0.0]);
        let l = pixel_layout(
            [480, 270],
            [1921, 1081],
            PixelScaling::WholeSteps,
            [9.0, -9.0],
        );
        assert_eq!(l.slide, [4.0, -4.0]);
        assert_eq!(l.origin, [0, 0]);
    }

    #[test]
    fn screen_and_picture_points_go_both_ways() {
        for (scaling, slide) in [
            (PixelScaling::WholeSteps, [0.3, -0.6]),
            (PixelScaling::Fill, [0.25, 0.5]),
        ] {
            let l = pixel_layout([480, 270], [2560, 1440], scaling, slide);
            for p in [[0.0, 0.0], [240.0, 135.0], [479.5, 269.5]] {
                let back = l.to_picture(l.to_screen(p));
                assert!((back[0] - p[0]).abs() < 1e-3 && (back[1] - p[1]).abs() < 1e-3);
            }
        }
    }

    #[test]
    fn the_uniform_matches_the_shader_layout() {
        assert_eq!(std::mem::size_of::<StageUniform>(), 48);
    }

    #[test]
    fn the_shader_is_valid() {
        use wgpu::naga;
        let module = naga::front::wgsl::parse_str(PIXEL_SHADER_SOURCE)
            .unwrap_or_else(|e| panic!("{}", e.emit_to_string(PIXEL_SHADER_SOURCE)));
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .unwrap_or_else(|e| panic!("{}", e.emit_to_string(PIXEL_SHADER_SOURCE)));
    }
}
