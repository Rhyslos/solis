//! Connects the viewport renderer (the `render` crate) to the window's wgpu
//! device and to egui, so the viewport panel can show the rendered image.

use editor::{ViewportRenderer, ViewportView};
use eframe::egui;
use eframe::egui_wgpu::RenderState;
use eframe::wgpu;
use render::{FrameParams, ViewportTarget};

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
}

impl GpuViewport {
    /// Sets up viewport drawing on the window's GPU device.
    pub fn new(state: &RenderState) -> Self {
        let samples = render::best_sample_count(&state.adapter);
        Self {
            renderer: render::ViewportRenderer::new(&state.device, samples),
            state: state.clone(),
            image: None,
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

impl ViewportRenderer for GpuViewport {
    fn render(&mut self, view: &ViewportView) -> Result<egui::TextureId, String> {
        let params = FrameParams {
            view_projection: view.view_projection,
            camera_position: view.camera_position,
            focus: view.focus,
            sky_top: view.sky_top,
            sky_horizon: view.sky_horizon,
            ground: view.ground,
        };
        self.prepare_image(view.size_px);
        let image = self
            .image
            .as_ref()
            .ok_or("the viewport image could not be made")?;
        self.renderer.render(
            &self.state.device,
            &self.state.queue,
            &image.target,
            &params,
        );
        Ok(image.texture)
    }
}
