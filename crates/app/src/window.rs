//! The editor window: opens it with eframe (winit + wgpu), restores the
//! settings saved last time, and hands each frame to the workbench.

use crate::branding;
use crate::gpu::GpuViewport;
use editor::{
    AppInfo, DockState, LogLevel, NoRenderer, Overlays, Panel, ViewportRenderer, Workbench,
    WorkbenchAction, theme::Accent,
};
use eframe::egui;

/// Key under which the panel layout is remembered between sessions.
const LAYOUT_KEY: &str = "panel_layout";
/// Key under which the accent colour is remembered.
const ACCENT_KEY: &str = "accent";
/// Key under which the viewport's corner controls are remembered.
const OVERLAYS_KEY: &str = "viewport_overlays";
/// Window size on first launch, in logical pixels.
const FIRST_SIZE: [f32; 2] = [1440.0, 900.0];
/// The smallest the window may get while staying usable.
const MIN_SIZE: [f32; 2] = [960.0, 600.0];

/// The running editor.
struct App {
    workbench: Workbench,
    /// Draws the viewport on the window's GPU.
    renderer: Box<dyn ViewportRenderer>,
    /// The window title last set, to change it only when it changes.
    title: String,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>, info: AppInfo) -> Self {
        let storage = cc.storage;
        let layout: Option<DockState<Panel>> =
            storage.and_then(|s| eframe::get_value(s, LAYOUT_KEY));
        let accent: Accent = storage
            .and_then(|s| eframe::get_value(s, ACCENT_KEY))
            .unwrap_or_default();
        let overlays: Overlays = storage
            .and_then(|s| eframe::get_value(s, OVERLAYS_KEY))
            .unwrap_or_default();
        editor::theme::apply(&cc.egui_ctx, accent);
        let mut workbench = Workbench::new(info.clone(), layout)
            .with_accent(accent)
            .with_overlays(overlays);
        workbench.log(
            LogLevel::Info,
            format!("{} {} started.", info.name, info.version),
        );
        let renderer: Box<dyn ViewportRenderer> = match cc.wgpu_render_state.as_ref() {
            Some(state) => {
                let gpu = GpuViewport::new(state);
                workbench.log(LogLevel::Info, gpu.describe());
                Box::new(gpu)
            }
            None => {
                let reason = "the window has no GPU device".to_string();
                workbench.log(LogLevel::Error, format!("No 3D view: {reason}."));
                Box::new(NoRenderer { reason })
            }
        };
        Self {
            workbench,
            renderer,
            title: String::new(),
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        for action in self.workbench.show(ui, self.renderer.as_mut()) {
            match action {
                WorkbenchAction::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            }
        }
        let title = self.workbench.title();
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
        // The frame-rate counter and the viewport keep updating.
        ctx.request_repaint();
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, LAYOUT_KEY, self.workbench.layout());
        eframe::set_value(storage, ACCENT_KEY, &self.workbench.accent());
        eframe::set_value(storage, OVERLAYS_KEY, &self.workbench.overlays());
    }
}

/// Opens the editor window and runs until it closes.
pub fn run(info: AppInfo) -> Result<(), String> {
    let title = format!("{} {}", info.name, info.version);
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(title)
            .with_app_id(branding::APP_ID)
            .with_inner_size(FIRST_SIZE)
            .with_min_inner_size(MIN_SIZE),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        eframe::run_native(
            branding::APP_ID,
            options,
            Box::new(move |cc| Ok(Box::new(App::new(cc, info)))),
        )
    }));
    match outcome {
        Ok(result) => result.map_err(|err| err.to_string()),
        Err(payload) => Err(explain_panic(payload.as_ref())),
    }
}

/// Turns a crash while opening the window into a plain explanation.
fn explain_panic(payload: &(dyn std::any::Any + Send)) -> String {
    let message = payload
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| payload.downcast_ref::<&str>().copied())
        .unwrap_or("the windowing system stopped unexpectedly");
    if message.contains(".so") && message.contains("could not be loaded") {
        format!(
            "{message} On Linux, install the missing library with your package manager \
             (for example libxkbcommon-x11-0 and mesa-vulkan-drivers on Ubuntu)."
        )
    } else {
        message.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crashes_are_explained_in_plain_words() {
        let missing: Box<dyn std::any::Any + Send> =
            Box::new("libxkbcommon-x11.so could not be loaded".to_string());
        assert!(explain_panic(missing.as_ref()).contains("package manager"));
        let other: Box<dyn std::any::Any + Send> = Box::new("boom");
        assert_eq!(explain_panic(other.as_ref()), "boom");
        let unknown: Box<dyn std::any::Any + Send> = Box::new(42);
        assert!(explain_panic(unknown.as_ref()).contains("unexpectedly"));
    }
}
