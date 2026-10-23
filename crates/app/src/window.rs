//! The editor window: opens it with eframe (winit + wgpu), restores the
//! settings saved last time, and hands each frame to the workbench.

use crate::branding;
use crate::gpu::GpuViewport;
use crate::import::{Finished, FolderFor, Importer};
use crate::project::Projects;
use asset_db::Change;
use editor::{
    AppInfo, Camera, DockState, FileLight, FileLightKind, LogLevel, Look, ModelInfo, NoRenderer,
    Overlays, Panel, Workbench, WorkbenchAction, theme::Accent,
};
use eframe::egui;
use reflect::AssetId;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Key under which the panel layout is remembered between sessions.
const LAYOUT_KEY: &str = "panel_layout";
/// Key under which the accent colour is remembered.
const ACCENT_KEY: &str = "accent";
/// Key under which the viewport's corner controls are remembered.
const OVERLAYS_KEY: &str = "viewport_overlays";
/// Key under which the viewport's look is remembered.
const LOOK_KEY: &str = "viewport_look";
/// Key under which the viewport camera is remembered.
const CAMERA_KEY: &str = "viewport_camera";
/// Key under which the project open last is remembered, to open it again.
const LAST_PROJECT_KEY: &str = "last_project";
/// Key under which the recent projects are remembered.
const RECENT_PROJECTS_KEY: &str = "recent_projects";
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
    /// The open project and its assets.
    projects: Projects,
    /// Models being read to be placed in the area when they arrive (the
    /// rest being read are reloads).
    to_place: HashSet<AssetId>,
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
        let overlays: Overlays = storage
            .and_then(|s| eframe::get_value(s, OVERLAYS_KEY))
            .unwrap_or_default();
        let last_project: Option<PathBuf> =
            storage.and_then(|s| eframe::get_value(s, LAST_PROJECT_KEY));
        let recent: Vec<PathBuf> = storage
            .and_then(|s| eframe::get_value(s, RECENT_PROJECTS_KEY))
            .unwrap_or_default();
        editor::theme::apply(&cc.egui_ctx, accent);
        let mut workbench = Workbench::new(info.clone(), layout)
            .with_accent(accent)
            .with_overlays(overlays)
            .with_camera(camera)
            .with_look(look);
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
        let importer = Importer::new(
            gpu.as_ref().map_or(4096, GpuViewport::max_texture_side),
            gpu.as_ref()
                .map_or(256 << 20, GpuViewport::max_buffer_bytes),
        );
        let mut app = Self {
            workbench,
            gpu,
            no_gpu,
            importer,
            projects: Projects::with_recent(recent),
            to_place: HashSet::new(),
            title: String::new(),
        };
        app.workbench.set_recent(app.projects.recent().to_vec());
        if let Some(root) = last_project.filter(|r| r.join(asset_db::PROJECT_FILE).is_file()) {
            app.open_project(&root, false);
        }
        for path in imports {
            app.import(&path);
        }
        app
    }

    /// Opens (or, with `create`, makes) the project in `root`.
    fn open_project(&mut self, root: &Path, create: bool) {
        match self.projects.open(root, create, self.importer.limits()) {
            Ok(opened) => {
                if let Some(gpu) = self.gpu.as_mut() {
                    gpu.clear_models();
                }
                self.to_place.clear();
                self.workbench.set_project(Some(opened.info));
                for note in opened.notes {
                    self.workbench.log(LogLevel::Warning, note);
                }
                if !opened.watching {
                    self.workbench.log(
                        LogLevel::Warning,
                        "The assets folder can't be watched here: changes made outside Solis \
                         show when the project is opened again.",
                    );
                }
                self.workbench.set_library(self.projects.library());
                self.workbench.set_recent(self.projects.recent().to_vec());
            }
            Err(why) => self.workbench.log(
                LogLevel::Error,
                format!("Couldn't open {}: {why}.", root.display()),
            ),
        }
    }

    /// Starts importing the model at `path`, or explains why it won't.
    /// With a project open, it's copied into the project's assets first.
    fn import(&mut self, path: &Path) {
        let file = file_name(path);
        if assets::is_model_file(path) {
            let (id, source) = if self.projects.db().is_some() {
                match self.projects.bring_in(path) {
                    Ok(found) => found,
                    Err(why) => {
                        self.workbench.log(
                            LogLevel::Error,
                            format!("Couldn't copy {file} into the project: {why}."),
                        );
                        return;
                    }
                }
            } else {
                (AssetId::random(), path.to_path_buf())
            };
            self.workbench.set_library(self.projects.library());
            self.to_place.insert(id);
            self.importer.start(id, &source, self.projects.cache());
            self.workbench
                .log(LogLevel::Info, format!("Importing {file}…"));
        } else {
            self.workbench.log(
                LogLevel::Warning,
                format!(
                    "{file} isn't a glTF model. {} imports .gltf and .glb files.",
                    branding::APP_NAME
                ),
            );
        }
    }

    /// The lights a model file carries, as the editor takes them.
    fn file_lights(lamps: &[assets::Lamp]) -> Vec<FileLight> {
        lamps
            .iter()
            .map(|lamp| FileLight {
                name: lamp.name.clone(),
                kind: match lamp.kind {
                    assets::LampKind::Sun => FileLightKind::Sun,
                    assets::LampKind::Point => FileLightKind::Point,
                    assets::LampKind::Spot { inner, outer } => FileLightKind::Spot { inner, outer },
                },
                position: lamp.position,
                direction: lamp.direction,
                color: lamp.color,
                strength: lamp.strength,
                range: lamp.range,
            })
            .collect()
    }

    /// Puts finished imports into the viewport (placing new ones, swapping
    /// in reloaded ones), or reports why they failed.
    fn collect_imports(&mut self) {
        for Finished {
            id,
            path,
            result,
            seconds,
            ..
        } in self.importer.finished()
        {
            let placing = self.to_place.remove(&id);
            match (result, self.gpu.as_mut()) {
                (Ok(model), Some(gpu)) => {
                    gpu.add_model(id, &model);
                    if let Some(db) = self.projects.db() {
                        // Part IDs for assemblies; a sidecar that can't be
                        // written only costs them until next time.
                        let _ = db.update_parts(id, &model);
                    }
                    let info = ModelInfo {
                        asset: id,
                        name: model.name.clone(),
                        triangles: model.triangles(),
                        min: model.bounds.min,
                        max: model.bounds.max,
                    };
                    if placing {
                        let lights = Self::file_lights(&model.lamps);
                        self.workbench
                            .add_model(info, &model.warnings, seconds, &lights);
                    } else {
                        self.workbench.reload_model(info, &model.warnings);
                    }
                }
                (Ok(_), None) => self.workbench.log(
                    LogLevel::Error,
                    format!("Couldn't show {}: there is no 3D view.", file_name(&path)),
                ),
                (Err(why), _) if placing => self.workbench.log(
                    LogLevel::Error,
                    format!("Couldn't import {}: {why}.", file_name(&path)),
                ),
                (Err(why), _) => self.workbench.log(
                    LogLevel::Error,
                    format!(
                        "Couldn't reload {}: {why}. The last version that worked stays.",
                        file_name(&path)
                    ),
                ),
            }
        }
    }

    /// Acts on what changed in the project's assets folder: models in use
    /// that changed are read again and swapped in.
    fn follow_changes(&mut self, changes: Vec<Change>, notes: Vec<String>) {
        for note in notes {
            self.workbench.log(LogLevel::Warning, note);
        }
        if changes.is_empty() {
            return;
        }
        for change in &changes {
            let Some(db) = self.projects.db() else { return };
            match change {
                Change::Changed(id) if self.workbench.has_model(*id) => {
                    if let Some(file) = db.file(*id) {
                        self.importer.start(*id, &file, self.projects.cache());
                    }
                }
                Change::Moved { id, from } => {
                    if let Some(record) = db.get(*id) {
                        self.workbench.log(
                            LogLevel::Info,
                            format!(
                                "{} moved to {}; everything using it still finds it.",
                                from.display(),
                                record.path.display()
                            ),
                        );
                    }
                }
                Change::Removed { path, .. } => self.workbench.log(
                    LogLevel::Warning,
                    format!(
                        "{} is gone from the project's assets. Anything using it keeps the \
                         last version until Solis restarts.",
                        path.display()
                    ),
                ),
                _ => {}
            }
        }
        self.workbench.set_library(self.projects.library());
    }

    /// Places a project asset where the camera looks: it's read (from the
    /// cache, so quickly) and placed when it arrives.
    fn place(&mut self, id: AssetId) {
        let Some(file) = self.projects.db().and_then(|db| db.file(id)) else {
            return;
        };
        self.to_place.insert(id);
        self.importer.start(id, &file, self.projects.cache());
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
        if let Some(path) = self.importer.chosen() {
            self.import(&path);
        }
        if let Some((purpose, folder)) = self.importer.chosen_folder() {
            self.open_project(&folder, purpose == FolderFor::NewProject);
        }
        let (changes, notes) = self.projects.poll();
        self.follow_changes(changes, notes);
        self.collect_imports();
        self.workbench.set_files_hovering(hovering);
        let renderer: &mut dyn editor::ViewportRenderer = match self.gpu.as_mut() {
            Some(gpu) => gpu,
            None => &mut self.no_gpu,
        };
        for action in self.workbench.show(ui, renderer) {
            match action {
                WorkbenchAction::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                WorkbenchAction::ImportModel => self.importer.ask(),
                WorkbenchAction::NewProject => self.importer.ask_folder(FolderFor::NewProject),
                WorkbenchAction::OpenProject => self.importer.ask_folder(FolderFor::OpenProject),
                WorkbenchAction::OpenRecent(root) => self.open_project(&root, false),
                WorkbenchAction::PlaceAsset(id) => self.place(id),
                WorkbenchAction::AddSampleArea => {
                    let id = self.importer.start_sample("Sample area");
                    self.to_place.insert(id);
                    self.workbench
                        .log(LogLevel::Info, "Adding the sample area…".to_string());
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
        eframe::set_value(storage, LAST_PROJECT_KEY, &self.projects.root());
        eframe::set_value(storage, RECENT_PROJECTS_KEY, &self.projects.recent());
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
