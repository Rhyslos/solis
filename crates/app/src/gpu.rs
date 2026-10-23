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
    /// The size last asked for (the target may be clamped smaller).
    requested_size: [u32; 2],
    texture: egui::TextureId,
}

/// Draws the viewport with the same GPU device the window uses.
pub struct GpuViewport {
    state: RenderState,
    renderer: render::ViewportRenderer,
    /// Each view's image (the viewport's, the Game panel's), by number.
    images: Vec<Option<Image>>,
    /// Imported models, uploaded.
    models: HashMap<AssetId, GpuModel>,
    /// The image thumbnails are drawn into, made the first time.
    thumbnail_target: Option<ViewportTarget>,
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
            models: HashMap::new(),
            thumbnail_target: None,
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
        self.state.renderer.write().free_texture(&id);
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
        });
    }
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
                editor::ViewKind::Perspective | editor::ViewKind::Top => render::GridPlane::Ground,
            },
        };
        let lights: Vec<render::Light> = view.lamps.iter().map(to_render_light).collect();
        if view.image >= editor::MAX_IMAGES {
            return Err("too many views".into());
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
        self.renderer.render(
            &self.state.device,
            &self.state.queue,
            &mut image.target,
            &params,
            &placed,
            &lights,
        );
        if let Some(rect) = view.pick {
            // Drawn after the frame, never waited for: the answer is
            // collected by `picked` on a later frame.
            self.renderer.request_pick(
                &self.state.device,
                &self.state.queue,
                &mut image.target,
                view.view_projection,
                &placed,
                rect,
            );
        }
        Ok(image.texture)
    }

    fn picked(&mut self, image: usize) -> Option<PickAnswer> {
        let image = self.images.get_mut(image)?.as_mut()?;
        let result = image.target.poll_pick(&self.state.device)?;
        Some(to_answer(&result))
    }
}
