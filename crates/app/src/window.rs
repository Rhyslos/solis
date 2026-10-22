//! The editor window: opens it with eframe (winit + wgpu), restores the
//! settings saved last time, and hands each frame to the workbench.

use crate::branding;
use crate::gpu::GpuViewport;
use crate::import::{Finished, Importer};
use editor::{
    AppInfo, Camera, DockState, EnvironmentSettings, LampFromFile, LogLevel, Look, ModelId,
    ModelInfo, NoRenderer, Overlays, Panel, SunSettings, Workbench, WorkbenchAction, theme::Accent,
};
use eframe::egui;
use std::path::{Path, PathBuf};

/// Key under which the panel layout is remembered between sessions.
const LAYOUT_KEY: &str = "panel_layout";
/// Key under which the accent colour is remembered.
const ACCENT_KEY: &str = "accent";
/// Key under which the viewport's corner controls are remembered.
const OVERLAYS_KEY: &str = "viewport_overlays";
/// Key under which the viewport's look is remembered.
const LOOK_KEY: &str = "viewport_look";
/// Key under which the sun's settings are remembered.
const SUN_KEY: &str = "viewport_sun";
/// Key under which the sky, fog and exposure are remembered.
const ENVIRONMENT_KEY: &str = "viewport_environment";
/// Key under which the viewport camera is remembered.
const CAMERA_KEY: &str = "viewport_camera";
/// Window size on first launch, in logical pixels.
const FIRST_SIZE: [f32; 2] = [1440.0, 900.0];
/// The smallest the window may get while staying usable.
const MIN_SIZE: [f32; 2] = [960.0, 600.0];

/// The running editor.
struct App {
    workbench: Workbench,
    /// Draws the viewport on the window's GPU, if it has one.
    gpu: Option<GpuViewport>,
    /// Explains the empty viewport when there is no GPU.
    no_gpu: NoRenderer,
    /// Reads model files in the background.
    importer: Importer,
    /// The window title last set, to change it only when it changes.
    title: String,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>, info: AppInfo, imports: Vec<PathBuf>) -> Self {
        let storage = cc.storage;
        let layout: Option<DockState<Panel>> =
            storage.and_then(|s| eframe::get_value(s, LAYOUT_KEY));
        let accent: Accent = storage
            .and_then(|s| eframe::get_value(s, ACCENT_KEY))
            .unwrap_or_default();
        let camera: Camera = storage
            .and_then(|s| eframe::get_value(s, CAMERA_KEY))
            .unwrap_or_default();
        let look: Look = storage
            .and_then(|s| eframe::get_value(s, LOOK_KEY))
            .unwrap_or_default();
        let sun: SunSettings = storage
            .and_then(|s| eframe::get_value(s, SUN_KEY))
            .unwrap_or_default();
        let environment: EnvironmentSettings = storage
            .and_then(|s| eframe::get_value(s, ENVIRONMENT_KEY))
            .unwrap_or_default();
        let overlays: Overlays = storage
            .and_then(|s| eframe::get_value(s, OVERLAYS_KEY))
            .unwrap_or_default();
        editor::theme::apply(&cc.egui_ctx, accent);
        let mut workbench = Workbench::new(info.clone(), layout)
            .with_accent(accent)
            .with_overlays(overlays)
            .with_camera(camera)
            .with_look(look)
            .with_sun(sun)
            .with_environment(environment);
        workbench.log(
            LogLevel::Info,
            format!("{} {} started.", info.name, info.version),
        );
        let gpu = cc.wgpu_render_state.as_ref().map(GpuViewport::new);
        let no_gpu = NoRenderer {
            reason: "the window has no GPU device".to_string(),
        };
        match &gpu {
            Some(gpu) => workbench.log(LogLevel::Info, gpu.describe()),
            None => workbench.log(LogLevel::Error, format!("No 3D view: {}.", no_gpu.reason)),
        }
        let importer = Importer::new(gpu.as_ref().map_or(4096, GpuViewport::max_texture_side));
        let mut app = Self {
            workbench,
            gpu,
            no_gpu,
            importer,
            title: String::new(),
        };
        for path in imports {
            app.import(&path);
        }
        app
    }

    /// Starts importing the model at `path`, or explains why it won't.
    fn import(&mut self, path: &Path) {
        let file = file_name(path);
        if assets::is_model_file(path) {
            self.importer.start(path);
            self.workbench
                .log(LogLevel::Info, format!("Importing {file}…"));
        } else {
            self.workbench.log(
                LogLevel::Warning,
                format!("{file} isn't a glTF model. Solis imports .gltf and .glb files."),
            );
        }
    }

    /// Hands the lights a model file carries to the editor: lamps join the
    /// Scene panel; a sun in the file sets the scene's sun.
    fn add_file_lights(&mut self, id: ModelId, lamps: &[assets::Lamp]) {
        let mut from_file = Vec::new();
        for lamp in lamps {
            let cone = match lamp.kind {
                assets::LampKind::Sun => {
                    self.workbench.use_sun_from_file(
                        &lamp.name,
                        lamp.direction,
                        lamp.color,
                        lamp.strength,
                    );
                    continue;
                }
                assets::LampKind::Point => None,
                assets::LampKind::Spot { inner, outer } => Some((inner, outer)),
            };
            from_file.push(LampFromFile {
                name: lamp.name.clone(),
                cone,
                position: lamp.position,
                direction: lamp.direction,
                color: lamp.color,
                strength: lamp.strength,
                range: lamp.range,
            });
        }
        self.workbench.add_model_lamps(id, &from_file);
    }

    /// Puts finished imports into the viewport, or reports why they failed.
    fn collect_imports(&mut self) {
        for Finished {
            id,
            path,
            result,
            seconds,
        } in self.importer.finished()
        {
            match (result, self.gpu.as_mut()) {
                (Ok(model), Some(gpu)) => {
                    gpu.add_model(id, &model);
                    let info = ModelInfo {
                        id,
                        name: model.name.clone(),
                        triangles: model.triangles(),
                        min: model.bounds.min,
                        max: model.bounds.max,
                    };
                    self.workbench.add_model(info, &model.warnings, seconds);
                    self.add_file_lights(id, &model.lamps);
                }
                (Ok(_), None) => self.workbench.log(
                    LogLevel::Error,
                    format!("Couldn't show {}: there is no 3D view.", file_name(&path)),
                ),
                (Err(why), _) => self.workbench.log(
                    LogLevel::Error,
                    format!("Couldn't import {}: {why}.", file_name(&path)),
                ),
            }
        }
    }
}

/// A file's name, for messages.
fn file_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        // Files dropped onto the window, and whether some are being
        // dragged over it right now.
        let (dropped, hovering) = ctx.input(|i| {
            (
                i.raw
                    .dropped_files
                    .iter()
                    .map(|f| f.path().to_path_buf())
                    .filter(|p| !p.as_os_str().is_empty())
                    .collect::<Vec<_>>(),
                !i.raw.hovered_files.is_empty(),
            )
        });
        for path in dropped {
            self.import(&path);
        }
        self.collect_imports();
        self.workbench.set_files_hovering(hovering);
        let renderer: &mut dyn editor::ViewportRenderer = match self.gpu.as_mut() {
            Some(gpu) => gpu,
            None => &mut self.no_gpu,
        };
        for action in self.workbench.show(ui, renderer) {
            match action {
                WorkbenchAction::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                WorkbenchAction::ImportModel => {
                    if let Some((_, path)) = self.importer.ask_and_start() {
                        self.workbench
                            .log(LogLevel::Info, format!("Importing {}…", file_name(&path)));
                    }
                }
                WorkbenchAction::AddSampleScene => {
                    self.importer.start_sample("Sample scene");
                    self.workbench
                        .log(LogLevel::Info, "Adding the sample scene…".to_string());
                }
                WorkbenchAction::RemoveModel(id) => {
                    if let Some(gpu) = self.gpu.as_mut() {
                        gpu.remove_model(id);
                    }
                }
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
        eframe::set_value(storage, CAMERA_KEY, &self.workbench.camera());
        eframe::set_value(storage, LOOK_KEY, &self.workbench.look());
        eframe::set_value(storage, SUN_KEY, &self.workbench.sun());
        eframe::set_value(storage, ENVIRONMENT_KEY, &self.workbench.environment());
    }
}

/// Opens the editor window and runs until it closes.
pub fn run(info: AppInfo, imports: Vec<PathBuf>) -> Result<(), String> {
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
            Box::new(move |cc| Ok(Box::new(App::new(cc, info, imports)))),
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
