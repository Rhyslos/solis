//! Connects the viewport renderer (the `render` crate) to the window's wgpu
//! device and to egui, so the viewport panel can show the rendered image.

use area::builtin::{Environment, LightKind, Sky, ToneMapping};
use editor::{LookKind, ViewLight, ViewportRenderer, ViewportView};
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
    image: Option<Image>,
    /// Imported models, uploaded.
    models: HashMap<AssetId, GpuModel>,
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

    /// Frees an imported model.
    pub fn remove_model(&mut self, id: AssetId) {
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
        };
        let lights: Vec<render::Light> = view.lamps.iter().map(to_render_light).collect();
        self.prepare_image(view.size_px);
        let image = self
            .image
            .as_mut()
            .ok_or("the viewport image could not be made")?;
        let placed: Vec<render::Placement<'_>> = view
            .models
            .iter()
            .filter_map(|(id, transform)| {
                self.models.get(id).map(|model| render::Placement {
                    model,
                    transform: *transform,
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
        Ok(image.texture)
    }
}
