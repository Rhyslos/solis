//! Connects the viewport renderer (the `render` crate) to the window's wgpu
//! device and to egui, so the viewport panel can show the rendered image.

use editor::{
    EnvironmentSettings, Lamp, LampKind, LookKind, ModelId, SkyKind, ToneMapping, ViewportRenderer,
    ViewportView,
};
use eframe::egui;
use eframe::egui_wgpu::RenderState;
use eframe::wgpu;
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
    image: Option<Image>,
    /// Imported models, uploaded.
    models: HashMap<ModelId, GpuModel>,
}

impl GpuViewport {
    /// Sets up viewport drawing on the window's GPU device.
    pub fn new(state: &RenderState) -> Self {
        let samples = render::best_sample_count(&state.adapter);
        Self {
            renderer: render::ViewportRenderer::new(&state.device, samples),
            state: state.clone(),
            image: None,
            models: HashMap::new(),
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

    /// Uploads an imported model, to draw whenever the viewport lists `id`.
    pub fn add_model(&mut self, id: ModelId, model: &assets::Model) {
        let uploaded = self
            .renderer
            .upload_model(&self.state.device, &self.state.queue, model);
        self.models.insert(id, uploaded);
    }

    /// Frees an imported model.
    pub fn remove_model(&mut self, id: ModelId) {
        self.models.remove(&id);
    }

    /// Makes the image `size` pixels, unless it already is.
    fn prepare_image(&mut self, size: [u32; 2]) {
        if self
            .image
            .as_ref()
            .is_some_and(|image| image.requested_size == size)
        {
            return;
        }
        let device = &self.state.device;
        let target = self.renderer.create_target(device, size);
        let mut egui_renderer = self.state.renderer.write();
        let texture = match self.image.as_ref() {
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
        self.image = Some(Image {
            target,
            requested_size: size,
            texture,
        });
    }
}

/// The editor's sky, fog and exposure as the renderer takes them.
fn to_render_environment(env: &EnvironmentSettings) -> render::Environment {
    render::Environment {
        sky: match env.sky {
            SkyKind::Backdrop => render::Sky::Backdrop,
            SkyKind::Daylight => render::Sky::Daylight {
                strength: env.sky_strength,
            },
        },
        fog: env.fog.then(|| render::Fog {
            half_distance: env.fog_distance,
            color: (!env.fog_matches_sky).then(|| env.fog_color.map(|c| f32::from(c) / 255.0)),
        }),
        exposure: env.exposure,
        tonemap: match env.tone_mapping {
            ToneMapping::Agx => render::Tonemap::Agx,
            ToneMapping::Aces => render::Tonemap::Aces,
            ToneMapping::Standard => render::Tonemap::Standard,
        },
    }
}

/// An editor lamp as the renderer takes it.
fn to_render_light(lamp: &Lamp) -> render::Light {
    render::Light {
        kind: match lamp.kind {
            LampKind::Point => render::LightKind::Point,
            LampKind::Spot { inner, outer } => render::LightKind::Spot {
                inner: inner.to_radians(),
                outer: outer.to_radians(),
            },
        },
        position: lamp.position,
        direction: lamp.direction,
        color: lamp.color.map(|c| f32::from(c) / 255.0),
        strength: lamp.strength,
        range: lamp.range,
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
            sun: render::Sun {
                direction: view.sun.direction(),
                color: view.sun.color.map(|c| f32::from(c) / 255.0),
                strength: view.sun.strength,
                shadows: view.sun.shadows,
            },
            environment: to_render_environment(&view.environment),
        };
        let lights: Vec<render::Light> = view.lamps.iter().map(to_render_light).collect();
        self.prepare_image(view.size_px);
        let image = self
            .image
            .as_ref()
            .ok_or("the viewport image could not be made")?;
        let models: Vec<&GpuModel> = view
            .models
            .iter()
            .filter_map(|id| self.models.get(id))
            .collect();
        self.renderer.render(
            &self.state.device,
            &self.state.queue,
            &image.target,
            &params,
            &models,
            &lights,
        );
        Ok(image.texture)
    }
}
