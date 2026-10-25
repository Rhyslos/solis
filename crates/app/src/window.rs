//! The editor window: opens it with eframe (winit + wgpu), restores the
//! settings saved last time, and hands each frame to the workbench.

use crate::area_file::{AreaFiles, Purpose};
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
use reflect::{AssetId, EntityId};
use std::collections::{HashSet, VecDeque};
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
    /// The open area's file: saving, opening, and changes on disk.
    area_files: AreaFiles,
    /// Actions waiting for a Save window to close (Save, then quit).
    queued: VecDeque<WorkbenchAction>,
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
            area_files: AreaFiles::default(),
            queued: VecDeque::new(),
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
                self.area_files.forget();
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
                    // Stable part numbers from the sidecar (nodes come
                    // first), so unpacked parts survive a re-export; without
                    // a project, their places in the file.
                    let parts = self
                        .projects
                        .db()
                        .and_then(|db| db.update_parts(id, &model).ok())
                        .filter(|p| p.len() >= model.nodes.len())
                        .unwrap_or_else(|| (1..=model.nodes.len() as u32).collect());
                    let info = ModelInfo {
                        asset: id,
                        name: model.name.clone(),
                        triangles: model.triangles(),
                        min: model.bounds.min,
                        max: model.bounds.max,
                        nodes: model
                            .nodes
                            .iter()
                            .zip(parts)
                            .map(|(node, part)| editor::models::NodeInfo {
                                part,
                                name: node.name.clone(),
                                parent: node.parent,
                                local: node.local,
                                has_meshes: !node.meshes.is_empty(),
                            })
                            .collect(),
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
                Change::Changed(id) if self.workbench.assemblies().contains_key(id) => {
                    let id = *id;
                    if let Err(why) = self.load_assembly(id, true) {
                        self.workbench.log(
                            LogLevel::Warning,
                            format!("Couldn't read the changed assembly: {why}; copies stay as they were."),
                        );
                    }
                    self.load_area_models();
                    continue;
                }
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

    /// Starts reading every model the open area uses that isn't loaded
    /// yet (after opening or reloading an area).
    fn load_area_models(&mut self) {
        let Some(db) = self.projects.db() else { return };
        let wanted: Vec<AssetId> = self
            .workbench
            .document()
            .area()
            .with::<area::builtin::Model>()
            .filter_map(|(_, m)| m.asset.map(|r| r.asset))
            .filter(|id| !self.workbench.has_model(*id))
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        let mut jobs = Vec::new();
        for id in wanted {
            if let Some(file) = db.file(id) {
                jobs.push((id, file));
            }
        }
        for (id, file) in jobs {
            self.importer.start(id, &file, self.projects.cache());
        }
    }

    /// `file` as text, with the asset paths for comments.
    fn file_text(&self, file: &area::text::AreaFile) -> String {
        let db = self.projects.db();
        area::text::write_file(file, &self.workbench.document().registry, &|id| {
            db.and_then(|db| db.get(id))
                .map(|r| r.path.display().to_string().replace('\\', "/"))
        })
    }

    /// Reads assembly `id` (and the assemblies it places) from the project
    /// and hands them to the editor. Assemblies already known are read
    /// again only when `fresh`.
    fn load_assembly(&mut self, id: AssetId, fresh: bool) -> Result<(), String> {
        let mut todo = vec![id];
        let mut seen = HashSet::new();
        while let Some(next) = todo.pop() {
            if !seen.insert(next)
                || (next != id || !fresh) && self.workbench.assemblies().contains_key(&next)
            {
                continue;
            }
            let path = self
                .projects
                .db()
                .and_then(|db| db.file(next))
                .ok_or_else(|| format!("the assembly {next} isn't in the project"))?;
            let file = AreaFiles::read(&path, &self.workbench.document().registry)
                .map_err(|why| format!("{}: {why}", file_name(&path)))?;
            todo.extend(
                file.area
                    .iter()
                    .filter_map(|(_, e)| e.origin.as_ref().map(|o| o.assembly)),
            );
            self.workbench.set_assembly(next, std::sync::Arc::new(file));
        }
        Ok(())
    }

    /// Reads every assembly an area file places.
    fn load_assemblies_of(&mut self, file: &area::text::AreaFile) {
        let wanted: HashSet<AssetId> = file
            .area
            .iter()
            .filter_map(|(_, e)| e.origin.as_ref().map(|o| o.assembly))
            .collect();
        for id in wanted {
            if let Err(why) = self.load_assembly(id, false) {
                self.workbench.log(
                    LogLevel::Warning,
                    format!("Couldn't read an assembly: {why}."),
                );
            }
        }
    }

    /// Saves the area to `path`; whether it worked.
    fn save_to(&mut self, path: &Path) -> bool {
        let text = match self.workbench.area_file() {
            Ok(file) => self.file_text(&file),
            Err(why) => {
                self.workbench
                    .log(LogLevel::Error, format!("Couldn't save: {why}."));
                return false;
            }
        };
        match self.area_files.save(path, &text) {
            Ok(()) => {
                self.workbench.mark_saved(path.to_path_buf());
                true
            }
            Err(why) => {
                self.workbench.log(
                    LogLevel::Error,
                    format!("Couldn't save {}: {why}.", file_name(path)),
                );
                false
            }
        }
    }

    /// Opens the area file at `path`.
    fn open_area(&mut self, path: &Path) {
        let read = AreaFiles::read(path, &self.workbench.document().registry).and_then(|file| {
            self.load_assemblies_of(&file);
            self.workbench.expand_file(&file).map_err(|e| e.to_string())
        });
        match read {
            Ok(area) => {
                self.workbench.open_area(area, path.to_path_buf());
                self.area_files.watch(path);
                self.load_area_models();
            }
            Err(why) => self.workbench.log(
                LogLevel::Error,
                format!("Couldn't open {}: {why}.", file_name(path)),
            ),
        }
    }

    /// The folder Open and Save windows start in: the project's areas, or
    /// its assets.
    fn areas_folder(&self) -> Option<PathBuf> {
        let assets = self.projects.db()?.project().assets();
        let areas = assets.join("areas");
        Some(if areas.is_dir() { areas } else { assets })
    }

    /// Carries out the queued actions in order. A save that needs the Save
    /// window pauses the rest until it closes.
    fn run_queue(&mut self, ctx: &egui::Context) {
        while let Some(action) = self.queued.pop_front() {
            match action {
                WorkbenchAction::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                WorkbenchAction::ImportModel => self.importer.ask(),
                WorkbenchAction::NewProject => self.importer.ask_folder(FolderFor::NewProject),
                WorkbenchAction::OpenProject => self.importer.ask_folder(FolderFor::OpenProject),
                WorkbenchAction::OpenRecent(root) => self.open_project(&root, false),
                WorkbenchAction::PlaceAsset(id) => self.place(id),
                WorkbenchAction::CreateAssembly(root) => self.create_assembly(root),
                WorkbenchAction::ApplyToAssembly {
                    assembly,
                    path,
                    component,
                    field,
                    value,
                } => self.apply_to_assembly(assembly, &path, &component, &field, value),
                WorkbenchAction::AddSampleArea => {
                    let id = self.importer.start_sample("Sample area");
                    self.to_place.insert(id);
                    self.workbench
                        .log(LogLevel::Info, "Adding the sample area…".to_string());
                }
                WorkbenchAction::NewArea => {
                    self.workbench.new_area();
                    self.area_files.forget();
                }
                WorkbenchAction::OpenArea => {
                    let folder = self.areas_folder();
                    self.area_files.ask(Purpose::Open, folder, "");
                }
                WorkbenchAction::SaveArea => {
                    match self.workbench.document().file().map(Path::to_path_buf) {
                        Some(path) => {
                            if !self.save_to(&path) {
                                // A failed save never lets a quit through.
                                self.queued.clear();
                            }
                        }
                        None => {
                            let (folder, name) =
                                (self.areas_folder(), self.workbench.document().name());
                            self.area_files.ask(Purpose::Save, folder, &name);
                            return;
                        }
                    }
                }
                WorkbenchAction::SaveAreaAs => {
                    let (folder, name) = (self.areas_folder(), self.workbench.document().name());
                    self.area_files.ask(Purpose::Save, folder, &name);
                    return;
                }
            }
        }
    }

    /// Saves `root` and everything under it as a new assembly in the
    /// project's `assemblies` folder, and turns it into a copy of it.
    fn create_assembly(&mut self, root: EntityId) {
        let Some(db) = self.projects.db() else {
            self.workbench.log(
                LogLevel::Warning,
                "Assemblies live in a project: make or open one first (File > New project…).",
            );
            return;
        };
        let Some(file) = self.workbench.assembly_file(root) else {
            return;
        };
        let name = self
            .workbench
            .document()
            .area()
            .get(root)
            .map_or_else(|| "Assembly".to_string(), |e| e.name.clone());
        let safe: String = name
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || " -_()".contains(c) {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let folder = db.project().assets().join("assemblies");
        let mut path = folder.join(format!("{safe}.{}", asset_db::ASSEMBLY_EXTENSION));
        let mut n = 2;
        while path.exists() {
            path = folder.join(format!("{safe} {n}.{}", asset_db::ASSEMBLY_EXTENSION));
            n += 1;
        }
        let text = self.file_text(&file);
        let written = std::fs::create_dir_all(&folder)
            .and_then(|()| asset_db::write_atomically(&path, text.as_bytes()));
        if let Err(why) = written {
            self.workbench.log(
                LogLevel::Error,
                format!("Couldn't save the assembly: {why}."),
            );
            return;
        }
        let (changes, notes) = self.projects.rescan();
        let Some(id) = self.projects.db().and_then(|db| db.id_at(&path)) else {
            self.follow_changes(changes, notes);
            return;
        };
        self.workbench.set_assembly(id, std::sync::Arc::new(file));
        self.follow_changes(changes, notes);
        self.workbench.make_copy(root, id);
        self.workbench.log(
            LogLevel::Info,
            format!("Saved the assembly {}.", file_name(&path)),
        );
    }

    /// Sets a field in an assembly's file (Apply to assembly), saves it,
    /// and updates every copy.
    fn apply_to_assembly(
        &mut self,
        assembly: AssetId,
        path: &[EntityId],
        component: &str,
        field: &str,
        value: reflect::Value,
    ) {
        let (Some(file), Some(on_disk)) = (
            self.workbench.assemblies().get(&assembly).cloned(),
            self.projects.db().and_then(|db| db.file(assembly)),
        ) else {
            return;
        };
        let mut file = (*file).clone();
        if !area::assembly::set_in_file(&mut file, path, component, field, value) {
            return;
        }
        let text = self.file_text(&file);
        match asset_db::write_atomically(&on_disk, text.as_bytes()) {
            Ok(()) => self
                .workbench
                .set_assembly(assembly, std::sync::Arc::new(file)),
            Err(why) => self.workbench.log(
                LogLevel::Error,
                format!("Couldn't save {}: {why}.", file_name(&on_disk)),
            ),
        }
    }

    /// Places a project asset where the camera looks: a model is read
    /// (from the cache, so quickly) and placed when it arrives; an
    /// assembly is placed at once.
    fn place(&mut self, id: AssetId) {
        let kind = self.projects.db().and_then(|db| db.get(id)).map(|r| r.kind);
        if kind == Some(asset_db::AssetKind::Assembly) {
            match self.load_assembly(id, false) {
                Ok(()) => {
                    self.workbench.place_assembly(id);
                    self.load_area_models();
                }
                Err(why) => self
                    .workbench
                    .log(LogLevel::Error, format!("Couldn't place it: {why}.")),
            }
            return;
        }
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
        match self.area_files.chosen() {
            Some((Purpose::Open, Some(path))) => self.open_area(&path),
            Some((Purpose::Save, Some(path))) => {
                if self.save_to(&path) {
                    self.run_queue(&ctx);
                } else {
                    self.queued.clear();
                }
            }
            // Cancelled: whatever waited for the save (a quit) doesn't
            // happen.
            Some((_, None)) => self.queued.clear(),
            None => {}
        }
        if let Some(path) = self.area_files.changed_on_disk() {
            let read =
                AreaFiles::read(&path, &self.workbench.document().registry).and_then(|file| {
                    self.load_assemblies_of(&file);
                    self.workbench.expand_file(&file).map_err(|e| e.to_string())
                });
            match read {
                Ok(area) => {
                    self.workbench.offer_reload(area);
                    self.load_area_models();
                }
                Err(why) => self.workbench.log(
                    LogLevel::Warning,
                    format!(
                        "{} changed on disk but can't be read ({why}); keeping what's open.",
                        file_name(&path)
                    ),
                ),
            }
        }
        if ctx.input(|i| i.viewport().close_requested()) && !self.workbench.close_requested() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }
        self.collect_imports();
        self.workbench.set_files_hovering(hovering);
        let renderer: &mut dyn editor::ViewportRenderer = match self.gpu.as_mut() {
            Some(gpu) => gpu,
            None => &mut self.no_gpu,
        };
        let actions = self.workbench.show(ui, renderer);
        self.queued.extend(actions);
        self.run_queue(&ctx);
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
