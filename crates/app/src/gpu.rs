//! Connects the viewport renderer (the `render` crate) to the window's wgpu
//! device and to egui, so the viewport panel can show the rendered image.

use area::builtin::{Environment, LightKind, Sky, ToneMapping};
use editor::{LookKind, PickAnswer, ViewLight, ViewportRenderer, ViewportView};
use eframe::egui;
use eframe::egui_wgpu::RenderState;
use eframe::wgpu;
use reflect::AssetId;
use render::{FrameParams, GpuModel, ViewportTarget};
use std::collections::HashMap;

/// The viewport's image and the egui texture showing it.
struct Image {
    target: ViewportTarget,
    /// A pick the renderer refused, to answer with "nothing".
    refused: Option<[u32; 4]>,
    /// The size last asked for (the target may be clamped smaller).
    requested_size: [u32; 2],
    texture: egui::TextureId,
}

/// A 2D camera's images (Phase 9, D1): the small picture, drawn at the
/// base resolution plus a pixel each side, and the screen-sized one the
/// pixel stage scales it up into, which egui shows.
struct PixelImages {
    small: ViewportTarget,
    small_size: [u32; 2],
    out: render::PixelImage,
    out_size: [u32; 2],
    texture: egui::TextureId,
}

/// Draws the viewport with the same GPU device the window uses.
pub struct GpuViewport {
    state: RenderState,
    renderer: render::ViewportRenderer,
    /// Each view's image (the viewport's, the Game panel's), by number.
    images: Vec<Option<Image>>,
    /// Each view's images when it looks through a 2D camera, by number.
    pixel_images: Vec<Option<PixelImages>>,
    /// Imported models, uploaded.
    models: HashMap<AssetId, GpuModel>,
    /// Sprite sheets' atlases, uploaded (Phase 9, D2).
    sheets: HashMap<AssetId, render::GpuSheet>,
    /// The image thumbnails are drawn into, made the first time.
    thumbnail_target: Option<ViewportTarget>,
    /// What was drawn since the Profiler last asked, and which images
    /// (by number, and whether through a 2D camera).
    report: editor::DrawReport,
    drawn: Vec<(usize, bool)>,
}

impl GpuViewport {
    /// Sets up viewport drawing on the window's GPU device.
    pub fn new(state: &RenderState) -> Self {
        let samples = render::best_sample_count(&state.adapter);
        // wgpu's own handler stops the program on any GPU mistake; this one
        // tells the Console instead, so a driver hiccup never loses work.
        state
            .device
            .on_uncaptured_error(std::sync::Arc::new(|error: eframe::wgpu::Error| {
                log::error!("the graphics card reported a problem: {error}");
            }));
        Self {
            renderer: render::ViewportRenderer::new(&state.device, samples),
            state: state.clone(),
            images: Vec::new(),
            pixel_images: Vec::new(),
            models: HashMap::new(),
            sheets: HashMap::new(),
            thumbnail_target: None,
            report: editor::DrawReport::default(),
            drawn: Vec::new(),
        }
    }

    /// Draws `model` from a three-quarter view into a small image of its
    /// own for the Library, and returns the egui texture showing it. The
    /// model is uploaded only for this and let go straight after.
    pub fn thumbnail(&mut self, model: &assets::Model) -> Option<egui::TextureId> {
        let side = crate::thumbnails::SIDE;
        let device = &self.state.device;
        let queue = &self.state.queue;
        let target = self
            .thumbnail_target
            .get_or_insert_with(|| self.renderer.create_target(device, [side, side]));
        let (min, max) = (model.bounds.min, model.bounds.max);
        if !(min.is_finite() && max.is_finite()) {
            return None;
        }
        let centre = (min + max) * 0.5;
        let radius = ((max - min).length() * 0.5).max(0.05);
        let mut camera = editor::Camera {
            position: centre + glam::Vec3::new(1.0, -1.5, 0.9).normalize() * radius * 2.4,
            ..editor::Camera::default()
        };
        camera.look_at(centre);
        let uploaded = self.renderer.upload_model(device, queue, model);
        let params = FrameParams {
            view_projection: camera.view_projection(1.0),
            camera_position: camera.position,
            // The grid under the model, as in the viewport.
            focus: glam::Vec3::new(centre.x, centre.y, 0.0),
            sky_top: editor::theme::backdrop::rgb(editor::theme::backdrop::SKY_TOP),
            sky_horizon: editor::theme::backdrop::rgb(editor::theme::backdrop::HORIZON),
            ground: editor::theme::backdrop::rgb(editor::theme::backdrop::GROUND),
            shading: render::Shading {
                look: render::Look::Standard,
                bands: 3,
                outline_px: 0.0,
            },
            size_px: [side, side],
            sun: render::Sun::default(),
            environment: to_render_environment(&Environment::default()),
            editor_lines: true,
            grid_plane: render::GridPlane::Ground,
            grid_step: 1.0,
        };
        self.renderer.render(
            device,
            queue,
            target,
            &params,
            &[render::Placement {
                model: &uploaded,
                node: None,
                transform: glam::Mat4::IDENTITY,
                pick: 0,
            }],
            &[],
        );
        // Copied into a texture of its own, so the next thumbnail can be
        // drawn in the same image.
        let size = wgpu::Extent3d {
            width: side,
            height: side,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("thumbnail"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: render::COLOR_FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("thumbnail copy"),
        });
        encoder.copy_texture_to_texture(
            target.color_texture().as_image_copy(),
            texture.as_image_copy(),
            size,
        );
        queue.submit([encoder.finish()]);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Some(self.state.renderer.write().register_native_texture(
            device,
            &view,
            wgpu::FilterMode::Linear,
        ))
    }

    /// Lets go of a thumbnail made by [`Self::thumbnail`].
    pub fn free_thumbnail(&self, id: egui::TextureId) {
        // Only thumbnails drawn here; pictures' textures belong to egui,
        // which lets them go when their handle is dropped.
        if matches!(id, egui::TextureId::User(_)) {
            self.state.renderer.write().free_texture(&id);
        }
    }

    /// A line for the console about the GPU in use.
    pub fn describe(&self) -> String {
        let info = self.state.adapter.get_info();
        format!(
            "Graphics: {} ({:?}), {}x multisampling.",
            info.name,
            info.backend,
            self.renderer.sample_count()
        )
    }

    /// The widest texture the graphics card holds, in pixels.
    pub fn max_texture_side(&self) -> u32 {
        self.state.device.limits().max_texture_dimension_2d
    }

    /// The biggest single GPU buffer the graphics card allows, in bytes.
    pub fn max_buffer_bytes(&self) -> u64 {
        self.state.device.limits().max_buffer_size
    }

    /// Uploads an imported model, to draw whenever the viewport lists `id`.
    pub fn add_model(&mut self, id: AssetId, model: &assets::Model) {
        let uploaded = self
            .renderer
            .upload_model(&self.state.device, &self.state.queue, model);
        self.models.insert(id, uploaded);
    }

    /// Frees every model (another project was opened).
    pub fn clear_models(&mut self) {
        self.models.clear();
    }

    /// Uploads a sprite sheet's atlas, to draw sprites of `id` from (again
    /// when it changes).
    pub fn add_sheet(&mut self, id: AssetId, sheet: &assets::sprite_sheet::SpriteSheet) {
        let uploaded = self.renderer.upload_sheet(
            &self.state.device,
            &self.state.queue,
            sheet.atlas_size,
            &sheet.atlas,
        );
        self.sheets.insert(id, uploaded);
    }

    /// Frees every sprite sheet (another project was opened).
    pub fn clear_sheets(&mut self) {
        self.sheets.clear();
    }

    /// Makes image number `slot` `size` pixels, unless it already is.
    fn prepare_image(&mut self, slot: usize, size: [u32; 2]) {
        if self.images.len() <= slot {
            self.images.resize_with(slot + 1, || None);
        }
        if self.images[slot]
            .as_ref()
            .is_some_and(|image| image.requested_size == size)
        {
            return;
        }
        let device = &self.state.device;
        let target = self.renderer.create_target(device, size);
        let mut egui_renderer = self.state.renderer.write();
        let texture = match self.images[slot].as_ref() {
            Some(image) => {
                egui_renderer.update_egui_texture_from_wgpu_texture(
                    device,
                    target.display_view(),
                    wgpu::FilterMode::Nearest,
                    image.texture,
                );
                image.texture
            }
            None => egui_renderer.register_native_texture(
                device,
                target.display_view(),
                wgpu::FilterMode::Nearest,
            ),
        };
        drop(egui_renderer);
        self.images[slot] = Some(Image {
            target,
            requested_size: size,
            texture,
            refused: None,
        });
    }
}

impl GpuViewport {
    /// Makes view `slot`'s 2D images: a small picture of `small` pixels
    /// and a screen image of `out` pixels, unless they already are.
    fn prepare_pixel_images(&mut self, slot: usize, small: [u32; 2], out: [u32; 2]) {
        if self.pixel_images.len() <= slot {
            self.pixel_images.resize_with(slot + 1, || None);
        }
        let device = &self.state.device;
        match &mut self.pixel_images[slot] {
            Some(images) => {
                if images.small_size != small {
                    images.small = self.renderer.create_target(device, small);
                    images.small_size = small;
                }
                if images.out_size != out {
                    images.out = self.renderer.create_pixel_image(device, out);
                    images.out_size = out;
                    self.state
                        .renderer
                        .write()
                        .update_egui_texture_from_wgpu_texture(
                            device,
                            images.out.display_view(),
                            wgpu::FilterMode::Nearest,
                            images.texture,
                        );
                }
            }
            None => {
                let out_image = self.renderer.create_pixel_image(device, out);
                let texture = self.state.renderer.write().register_native_texture(
                    device,
                    out_image.display_view(),
                    wgpu::FilterMode::Nearest,
                );
                self.pixel_images[slot] = Some(PixelImages {
                    small: self.renderer.create_target(device, small),
                    small_size: small,
                    out: out_image,
                    out_size: out,
                    texture,
                });
            }
        }
    }

    /// Draws a view through a 2D camera: the world at the base resolution
    /// plus a pixel each side, then scaled up to the view by the pixel
    /// stage (Phase 9, D1).
    fn render_pixels(
        &mut self,
        view: &ViewportView,
        pixel: editor::PixelView,
        mut params: FrameParams,
        lights: &[render::Light],
    ) -> Result<egui::TextureId, String> {
        let small = [pixel.base[0] + 2, pixel.base[1] + 2];
        params.size_px = small;
        self.prepare_pixel_images(view.image, small, view.size_px);
        let images = self.pixel_images[view.image]
            .as_mut()
            .ok_or("the 2D images could not be made")?;
        let placed: Vec<render::Placement<'_>> = view
            .models
            .iter()
            .filter_map(|m| {
                self.models.get(&m.asset).map(|model| render::Placement {
                    model,
                    node: m.node,
                    transform: m.transform,
                    pick: m.pick,
                })
            })
            .collect();
        let bolts: &[render::Bolt] = bytemuck::cast_slice(view.bolts.as_slice());
        let picks = sprite_runs(&self.sheets, &view.sprites);
        let runs: Vec<render::SpriteRun> = picks.iter().map(|p| p.run).collect();
        let stats = self.renderer.render_scene(
            &self.state.device,
            &self.state.queue,
            &mut images.small,
            &params,
            &render::Scene {
                placed: &placed,
                lights,
                bolts,
                sprites: &runs,
            },
        );
        let layout = render::pixel_layout(
            pixel.base,
            images.out_size,
            match pixel.scaling {
                area::pixels::Scaling::WholeSteps => render::PixelScaling::WholeSteps,
                area::pixels::Scaling::Fill => render::PixelScaling::Fill,
            },
            pixel.slide,
        );
        self.renderer.draw_pixels(
            &self.state.device,
            &self.state.queue,
            &images.small,
            &images.out,
            &layout,
        );
        let texture = images.texture;
        count(&mut self.report, &stats);
        if !self.drawn.contains(&(view.image, true)) {
            self.drawn.push((view.image, true));
        }
        Ok(texture)
    }
}

/// A view's sprites as the renderer takes them: a run per batch whose
/// sheet is uploaded, with each sprite's pick number. The same layout
/// (checked by a test), so no copy.
fn sprite_runs<'a>(
    sheets: &'a HashMap<AssetId, render::GpuSheet>,
    batches: &'a [editor::sprite_sheets::SpriteBatch],
) -> Vec<render::SpritePicks<'a>> {
    batches
        .iter()
        .filter_map(|b| {
            Some(render::SpritePicks {
                run: render::SpriteRun {
                    sheet: sheets.get(&b.sheet)?,
                    instances: bytemuck::cast_slice(b.sprites.as_slice()),
                },
                picks: &b.picks,
            })
        })
        .collect()
}

/// Adds what a frame drew to the Profiler's report.
fn count(report: &mut editor::DrawReport, stats: &render::FrameStats) {
    report.bolts += u64::from(stats.bolts);
    report.instances += u64::from(stats.instances);
    report.casting += u64::from(stats.casting);
    report.sent_bytes += stats.sent_bytes;
    for (total, n) in report.lods.iter_mut().zip(stats.lods) {
        *total += u64::from(n);
    }
    report.batches += u64::from(stats.batches);
    report.triangles += stats.triangles;
}

/// The area's sky, fog and exposure as the renderer takes them.
fn to_render_environment(env: &Environment) -> render::Environment {
    render::Environment {
        sky: match env.sky {
            Sky::Backdrop => render::Sky::Backdrop,
            Sky::Daylight => render::Sky::Daylight {
                strength: env.sky_strength,
            },
        },
        fog: env.fog.then(|| render::Fog {
            half_distance: env.fog_distance,
            color: (!env.fog_matches_sky).then_some(env.fog_color),
        }),
        exposure: env.exposure,
        tonemap: match env.tone_mapping {
            ToneMapping::Agx => render::Tonemap::Agx,
            ToneMapping::Aces => render::Tonemap::Aces,
            ToneMapping::Standard => render::Tonemap::Standard,
        },
    }
}

/// The area's sun as the renderer takes it; with no sun, a dark one.
fn to_render_sun(sun: Option<&ViewLight>) -> render::Sun {
    sun.map_or(
        render::Sun {
            strength: 0.0,
            shadows: false,
            ..render::Sun::default()
        },
        |sun| render::Sun {
            direction: sun.direction,
            color: sun.color,
            strength: sun.strength,
            shadows: sun.shadows,
        },
    )
}

/// An area light as the renderer takes it.
fn to_render_light(light: &ViewLight) -> render::Light {
    render::Light {
        kind: match light.kind {
            LightKind::Spot => render::LightKind::Spot {
                inner: light.inner,
                outer: light.outer,
            },
            _ => render::LightKind::Point,
        },
        position: light.position,
        direction: light.direction,
        color: light.color,
        strength: light.strength,
        range: light.range,
    }
}

/// What the renderer found, as the editor takes it.
fn to_answer(result: &render::PickResult) -> PickAnswer {
    let [x, y, w, h] = result.rect;
    let (mx, my) = (x + w / 2, y + h / 2);
    PickAnswer {
        rect: result.rect,
        found: result.all().into_iter().collect(),
        middle: result.pick_at(mx, my),
        point: result.point_at(mx, my),
    }
}

impl ViewportRenderer for GpuViewport {
    fn render(&mut self, view: &ViewportView) -> Result<egui::TextureId, String> {
        let params = FrameParams {
            view_projection: view.view_projection,
            camera_position: view.camera_position,
            focus: view.focus,
            sky_top: view.sky_top,
            sky_horizon: view.sky_horizon,
            ground: view.ground,
            shading: render::Shading {
                look: match view.look.kind {
                    LookKind::Standard => render::Look::Standard,
                    LookKind::Stylized => render::Look::Stylized,
                    LookKind::Unlit => render::Look::Unlit,
                },
                bands: u32::from(view.look.bands),
                outline_px: view.outline_px,
            },
            size_px: view.size_px,
            sun: to_render_sun(view.sun.as_ref()),
            environment: to_render_environment(&view.environment),
            editor_lines: view.editor_lines,
            grid_plane: match view.kind {
                editor::ViewKind::Front => render::GridPlane::Front,
                editor::ViewKind::Right => render::GridPlane::Side,
                editor::ViewKind::Perspective | editor::ViewKind::Top | editor::ViewKind::TwoD => {
                    render::GridPlane::Ground
                }
            },
            grid_step: view.grid_step,
        };
        let lights: Vec<render::Light> = view.lamps.iter().map(to_render_light).collect();
        if view.image >= editor::MAX_IMAGES {
            return Err("too many views".into());
        }
        if let Some(pixel) = view.pixel {
            return self.render_pixels(view, pixel, params, &lights);
        }
        self.prepare_image(view.image, view.size_px);
        let image = self.images[view.image]
            .as_mut()
            .ok_or("the viewport image could not be made")?;
        let placed: Vec<render::Placement<'_>> = view
            .models
            .iter()
            .filter_map(|m| {
                self.models.get(&m.asset).map(|model| render::Placement {
                    model,
                    node: m.node,
                    transform: m.transform,
                    pick: m.pick,
                })
            })
            .collect();
        // The same layout (checked by a test), so no copy.
        let bolts: &[render::Bolt] = bytemuck::cast_slice(view.bolts.as_slice());
        let picks = sprite_runs(&self.sheets, &view.sprites);
        let runs: Vec<render::SpriteRun> = picks.iter().map(|p| p.run).collect();
        let stats = self.renderer.render_scene(
            &self.state.device,
            &self.state.queue,
            &mut image.target,
            &params,
            &render::Scene {
                placed: &placed,
                lights: &lights,
                bolts,
                sprites: &runs,
            },
        );
        count(&mut self.report, &stats);
        if !self.drawn.contains(&(view.image, false)) {
            self.drawn.push((view.image, false));
        }
        if let Some(rect) = view.pick {
            // Drawn after the frame, never waited for: the answer is
            // collected by `picked` on a later frame.
            let asked = self.renderer.request_pick_with_sprites(
                &self.state.device,
                &self.state.queue,
                &mut image.target,
                view.view_projection,
                &placed,
                &picks,
                rect,
            );
            if !asked {
                // Refused (nothing to look at there): answered "nothing"
                // straight away, rather than leaving the editor waiting.
                image.refused = Some(rect);
            }
        }
        Ok(image.texture)
    }

    fn picked(&mut self, image: usize) -> Option<PickAnswer> {
        let image = self.images.get_mut(image)?.as_mut()?;
        if let Some(rect) = image.refused.take() {
            return Some(PickAnswer {
                rect,
                ..PickAnswer::default()
            });
        }
        let result = image.target.poll_pick(&self.state.device)?;
        Some(to_answer(&result))
    }

    fn take_report(&mut self) -> editor::DrawReport {
        let mut report = std::mem::take(&mut self.report);
        // The GPU's newest times for each image drawn, together.
        let mut gpu: Option<editor::GpuSplit> = None;
        for &(slot, pixel) in &std::mem::take(&mut self.drawn) {
            let target = if pixel {
                self.pixel_images
                    .get(slot)
                    .and_then(Option::as_ref)
                    .map(|p| &p.small)
            } else {
                self.images
                    .get(slot)
                    .and_then(Option::as_ref)
                    .map(|i| &i.target)
            };
            let Some(target) = target else {
                continue;
            };
            report.gpu_timing = target.can_time().or(report.gpu_timing);
            if let Some(t) = target.gpu_times(&self.state.device) {
                let sum = gpu.get_or_insert_default();
                sum.shadows += t.shadows;
                sum.models += t.models;
                sum.last_pass += t.last_pass;
            }
        }
        report.gpu = gpu;
        report
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_editor_s_bolts_are_laid_out_as_the_renderer_s() {
        // Handed over without a copy: the same fields, in the same order.
        let bolt = editor::ViewBolt {
            head: glam::Vec3::new(1.0, 2.0, 3.0),
            trail: glam::Vec3::new(4.0, 5.0, 6.0),
            colour: [7.0, 8.0, 9.0],
            width: 10.0,
        };
        let made: &[render::Bolt] = bytemuck::cast_slice(std::slice::from_ref(&bolt));
        assert_eq!(
            made[0],
            render::Bolt {
                head: [1.0, 2.0, 3.0],
                trail: [4.0, 5.0, 6.0],
                colour: [7.0, 8.0, 9.0],
                width: 10.0,
            }
        );
        assert_eq!(
            std::mem::size_of::<editor::ViewBolt>(),
            std::mem::size_of::<render::Bolt>()
        );
    }

    #[test]
    fn a_view_sprite_is_laid_out_as_the_renderer_s_sprite() {
        // Handed over without a copy: the same fields, in the same order.
        let sprite = editor::sprite_sheets::ViewSprite {
            corner: [1.0, 2.0, 3.0],
            flags: 5,
            axis_x: [4.0, 5.0],
            axis_y: [6.0, 7.0],
            rect: [8.0, 9.0, 10.0, 11.0],
            tint: [0.1, 0.2, 0.3, 0.4],
            outline: [0.5, 0.6, 0.7, 0.0],
            foot: [12.0, 13.0],
        };
        let made: &[render::SpriteInstance] = bytemuck::cast_slice(std::slice::from_ref(&sprite));
        assert_eq!(
            made[0],
            render::SpriteInstance {
                corner: [1.0, 2.0, 3.0],
                flags: 5,
                axis_x: [4.0, 5.0],
                axis_y: [6.0, 7.0],
                rect: [8.0, 9.0, 10.0, 11.0],
                tint: [0.1, 0.2, 0.3, 0.4],
                outline: [0.5, 0.6, 0.7, 0.0],
                foot: [12.0, 13.0],
            }
        );
        assert_eq!(
            std::mem::size_of::<editor::sprite_sheets::ViewSprite>(),
            std::mem::size_of::<render::SpriteInstance>()
        );
    }
}
