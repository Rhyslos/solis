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
/// How often a frame is drawn while background work is on its way.
const WAITING_FRAME: std::time::Duration = std::time::Duration::from_millis(100);
/// How often a frame is drawn while nothing happens, to look for changes
/// on disk and messages from background work.
const IDLE_FRAME: std::time::Duration = std::time::Duration::from_secs(1);
const LAYOUT_KEY: &str = "panel_layout";
/// Key under which the accent colour is remembered.
const ACCENT_KEY: &str = "accent";
/// Key under which the viewport's corner controls are remembered.
const OVERLAYS_KEY: &str = "viewport_overlays";
/// Key under which snapping and the axis locks are remembered.
const SNAPPING_KEY: &str = "viewport_snapping";
/// Key under which the viewport's look is remembered.
const LOOK_KEY: &str = "viewport_look";
/// Key under which the viewport camera is remembered.
const CAMERA_KEY: &str = "viewport_camera";
/// Key under which the project open last is remembered, to open it again.
const LAST_PROJECT_KEY: &str = "last_project";
/// Key under which the recent projects are remembered.
const RECENT_PROJECTS_KEY: &str = "recent_projects";
/// Key under which the viewport's views (layout, cameras, looks) are
/// remembered.
const VIEWS_KEY: &str = "viewport_views";
/// Key under which the console's earlier commands are remembered.
const CONSOLE_HISTORY_KEY: &str = "console_history";
/// Whether updates are checked when Solis starts (decisions 0043, 0045).
const AUTO_UPDATE_KEY: &str = "auto_update";
/// The build skipped with Skip this version (decision 0045).
const SKIPPED_BUILD_KEY: &str = "skipped_build";
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
    /// The Library's thumbnails.
    thumbnails: crate::thumbnails::Thumbnails,
    /// The game's code, checked on a worker.
    game_code: crate::game_code::GameCode,
    /// The last note on the project's `armour.rs` (changes to it don't
    /// run until Phase 10), to say it again only when it changes.
    rust_note: Option<String>,
    /// The program and version, for the files describing the engine.
    made_by: String,
    /// What's in the assets folder, as the Library last showed it.
    listing: asset_db::Listing,
    /// Assemblies that couldn't be read ahead for the Library's listing:
    /// not tried again until the listing changes.
    unreadable: HashSet<AssetId>,
    /// The open area's file: saving, opening, and changes on disk.
    /// The Open window for a recording of play.
    recordings: crate::play_files::RecordingDialog,
    /// Reads the gamepad.
    gamepads: crate::gamepad::Gamepads,
    area_files: AreaFiles,
    /// Actions waiting for a Save window to close (Save, then quit).
    queued: VecDeque<WorkbenchAction>,
    /// The window title last set, to change it only when it changes.
    title: String,
    /// Messages the libraries wrote, for the Console.
    messages: Option<crate::logger::Messages>,
    /// Checking for, downloading and installing newer builds.
    updates: crate::updates::Updates,
    /// Starting up behind the splash screen, until the editor opens.
    startup: Option<crate::startup::Startup>,
    /// When the splash appeared.
    began: std::time::Instant,
    /// The logo, for the splash.
    logo: Option<egui::TextureHandle>,
}

impl App {
    fn new(
        cc: &eframe::CreationContext<'_>,
        info: AppInfo,
        imports: Vec<PathBuf>,
        pending: crate::updates::PendingSlot,
    ) -> Self {
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
        let snapping: editor::Snapping = storage
            .and_then(|s| eframe::get_value(s, SNAPPING_KEY))
            .unwrap_or_default();
        let views: Option<editor::Views> = storage.and_then(|s| eframe::get_value(s, VIEWS_KEY));
        let console_history: Vec<String> = storage
            .and_then(|s| eframe::get_value(s, CONSOLE_HISTORY_KEY))
            .unwrap_or_default();
        let last_project = storage.and_then(saved_last_project);
        let recent: Vec<PathBuf> = storage
            .and_then(|s| eframe::get_value(s, RECENT_PROJECTS_KEY))
            .unwrap_or_default();
        let auto_update: bool = storage
            .and_then(|s| eframe::get_value(s, AUTO_UPDATE_KEY))
            .unwrap_or(true);
        editor::theme::apply(&cc.egui_ctx, accent);
        let mut workbench = Workbench::new(info.clone(), layout)
            .with_accent(accent)
            .with_overlays(overlays)
            .with_snapping(snapping)
            .with_camera(camera)
            .with_look(look)
            .with_console_history(console_history);
        workbench.set_auto_update(auto_update);
        // Saved before views existed: the one camera and look above.
        if let Some(views) = &views {
            workbench = workbench.with_views(views);
        }
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
        let skipped: Option<u64> = storage
            .and_then(|s| eframe::get_value::<Option<u64>>(s, SKIPPED_BUILD_KEY))
            .flatten();
        // The splash asks GitHub while it shows, unless checking is off
        // (decision 0045).
        let copy = crate::updates::this_copy();
        let check = auto_update.then(|| crate::updates::start_check(&cc.egui_ctx));
        let startup = crate::startup::Startup::new(check, skipped, copy.installed, copy.build);
        let mut app = Self {
            workbench,
            gpu,
            no_gpu,
            importer,
            projects: Projects::with_recent(recent),
            to_place: HashSet::new(),
            thumbnails: crate::thumbnails::Thumbnails::default(),
            game_code: crate::game_code::GameCode::default(),
            rust_note: None,
            // Without the build number, so the engine's files in a project
            // don't change with every update.
            made_by: format!("{} {}", info.name, env!("CARGO_PKG_VERSION")),
            listing: asset_db::Listing::default(),
            unreadable: HashSet::new(),
            area_files: AreaFiles::default(),
            recordings: crate::play_files::RecordingDialog::default(),
            gamepads: crate::gamepad::Gamepads::new(),
            queued: VecDeque::new(),
            title: String::new(),
            messages: crate::logger::install(),
            updates: crate::updates::Updates::new(pending, skipped),
            startup: Some(startup),
            began: std::time::Instant::now(),
            logo: load_logo(&cc.egui_ctx),
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
        // Opening the open project again changes nothing: going through
        // with it would empty the viewport and stop watching the area.
        if self
            .projects
            .root()
            .is_some_and(|open| same_folder(open, root))
        {
            self.workbench.log(
                LogLevel::Info,
                format!("{} is already open.", file_name(root)),
            );
            return;
        }
        match self.projects.open(root, create, self.importer.limits()) {
            Ok(opened) => {
                if let Some(gpu) = self.gpu.as_mut() {
                    gpu.clear_models();
                }
                self.thumbnails.clear(self.gpu.as_ref());
                self.game_code.forget();
                self.to_place.clear();
                self.area_files.forget();
                let assets = opened.info.assets.clone();
                self.workbench.set_project(Some(opened.info));
                // Its input actions (decision 0037).
                let (bindings, note) = crate::play_files::load_bindings(root);
                self.workbench.set_bindings(bindings);
                if let Some(note) = note {
                    self.workbench.log(LogLevel::Warning, note);
                }
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
                // The engine described to game code, then the code checked.
                for note in crate::game_code::GameCode::describe(&assets, &self.made_by) {
                    self.workbench.log(LogLevel::Warning, note);
                }
                // Rust components built into Solis that this project uses
                // (the turret sample's), until Phase 10 builds a game's own.
                let (rust, note) = crate::sample_game::rust_components(&assets);
                self.workbench.set_rust_components(rust);
                if let Some(note) = &note {
                    self.workbench.log(LogLevel::Warning, note.clone());
                }
                self.rust_note = note;
                self.game_code.check(assets);
                self.refresh_library();
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
            self.refresh_library();
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
    /// Shows in the Console what the libraries wrote since last frame.
    fn show_library_messages(&mut self) {
        let Some(messages) = &self.messages else {
            return;
        };
        for (level, text) in messages.take() {
            let level = match level {
                crate::logger::Level::Info => LogLevel::Info,
                crate::logger::Level::Warning => LogLevel::Warning,
                crate::logger::Level::Error => LogLevel::Error,
            };
            self.workbench.log(level, text);
        }
    }

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
                        lods: model.lod_triangles().into_iter().skip(1).collect(),
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
                (Err(why), _) if placing => {
                    self.workbench.placing_failed(id);
                    self.workbench.log(
                        LogLevel::Error,
                        format!("Couldn't import {}: {why}.", file_name(&path)),
                    );
                }
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
                Change::Changed(id) => {
                    // Its thumbnail is made again (or tried again, if it
                    // failed before), and it's read again if it's in use.
                    if let Some(record) = db.get(*id) {
                        self.thumbnails.forget(&record.path, self.gpu.as_ref());
                    }
                    if self.workbench.has_model(*id)
                        && let Some(file) = db.file(*id)
                    {
                        self.importer.start(*id, &file, self.projects.cache());
                    }
                }
                Change::Moved { id, from } => {
                    self.thumbnails.forget(from, self.gpu.as_ref());
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
        // A model the area uses that was missing may be back (from the
        // Recycle Bin, a Git pull): it's read now, not on the next opening.
        if changes.iter().any(|c| matches!(c, Change::Added(_))) {
            self.load_area_models();
        }
        self.refresh_library();
    }

    /// Starts reading every model the open area uses that isn't loaded
    /// yet (after opening or reloading an area).
    fn load_area_models(&mut self) {
        let Some(db) = self.projects.db() else { return };
        let wanted: Vec<AssetId> = models_wanted(
            self.workbench.document().area(),
            self.workbench.assemblies().values().map(|file| &file.area),
        )
        .into_iter()
        .filter(|id| !self.workbench.has_model(*id))
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
        let mut read = Vec::new();
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
            read.push((next, file));
        }
        // Every file is read before any is used, and the ones placed
        // inside others go first: an assembly that now places a new one
        // (a Git pull) finds it already known when its copies are rebuilt.
        for (next, file) in read.into_iter().rev() {
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
        // Nothing goes on while an Open or Save window is open: a quit or a
        // new area waits for the save before it.
        while !self.area_files.is_asking()
            && let Some(action) = self.queued.pop_front()
        {
            match action {
                WorkbenchAction::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                WorkbenchAction::DownloadUpdate => {
                    self.updates.download(&mut self.workbench, ctx);
                }
                WorkbenchAction::OpenUpdatePage => {
                    if let Err(why) = opener::open(crate::updates::releases_page()) {
                        self.workbench.log(
                            LogLevel::Warning,
                            format!("Couldn't open the download page: {why}."),
                        );
                    }
                }
                WorkbenchAction::SkipUpdate(skip) => {
                    if let Some(build) = skip {
                        self.updates.skip(build);
                    }
                }
                WorkbenchAction::CheckForUpdates => {
                    self.updates.check_now(&mut self.workbench, ctx);
                }
                // Kept in the editor; saved with the rest.
                WorkbenchAction::SetAutoUpdate(_) => {}
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
                WorkbenchAction::AddSampleArea => self.add_sample(),
                WorkbenchAction::MakeSampleGame => self.importer.ask_folder(FolderFor::SampleGame),
                WorkbenchAction::MakeBenchmark => self.importer.ask_folder(FolderFor::Benchmark),
                WorkbenchAction::MakePlayground => self.importer.ask_folder(FolderFor::Playground),
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
                                self.drop_queue();
                            }
                        }
                        None => {
                            let (folder, name) =
                                (self.areas_folder(), self.workbench.document().name());
                            self.area_files.ask(Purpose::Save, folder, &name);
                        }
                    }
                }
                WorkbenchAction::SaveAreaAs => {
                    let (folder, name) = (self.areas_folder(), self.workbench.document().name());
                    self.area_files.ask(Purpose::Save, folder, &name);
                }
                WorkbenchAction::OpenAreaFile(path) => self.open_area(&path),
                WorkbenchAction::SaveBindings(text) => {
                    if let Some(root) = self.projects.root().map(Path::to_path_buf) {
                        match crate::play_files::save_bindings(&root, &text) {
                            Ok(Some(backup)) => self.workbench.log(
                                LogLevel::Warning,
                                format!(
                                    "The input actions file had a mistake, so it was kept as \
                                     {backup} before being written again: your own actions \
                                     are in it."
                                ),
                            ),
                            Ok(None) => {}
                            Err(why) => self.workbench.log(
                                LogLevel::Error,
                                format!("Couldn't save the input actions: {why}."),
                            ),
                        }
                    }
                }
                WorkbenchAction::SaveRecording(text) => {
                    match self.projects.root().map(Path::to_path_buf) {
                        Some(root) => match crate::play_files::save_recording(&root, &text) {
                            Ok(path) => self.workbench.log(
                                LogLevel::Info,
                                format!(
                                    "Saved the recording as {} (Play > Replay a recording… plays it back).",
                                    file_name(&path)
                                ),
                            ),
                            Err(why) => self.workbench.log(
                                LogLevel::Error,
                                format!("Couldn't save the recording: {why}."),
                            ),
                        },
                        None => self.workbench.log(
                            LogLevel::Warning,
                            "The recording wasn't kept: recordings are kept in a project.",
                        ),
                    }
                }
                WorkbenchAction::OpenRecording => {
                    let folder = self
                        .projects
                        .root()
                        .map(|r| r.join(crate::play_files::RECORDINGS));
                    self.recordings
                        .ask(crate::play_files::PlayFile::Recording, folder);
                }
                WorkbenchAction::OpenSave => {
                    let folder = self
                        .projects
                        .root()
                        .map(|r| r.join(crate::play_files::SAVES));
                    self.recordings
                        .ask(crate::play_files::PlayFile::Save, folder);
                }
                WorkbenchAction::SaveGame { slot, text } => {
                    match self.projects.root().map(Path::to_path_buf) {
                        Some(root) => match crate::play_files::save_game(&root, &slot, &text) {
                            Ok(_) => self
                                .workbench
                                .log(LogLevel::Info, format!("Saved the game as {slot}.")),
                            Err(why) => self.workbench.log(
                                LogLevel::Error,
                                format!("Couldn't save the game as {slot}: {why}."),
                            ),
                        },
                        None => self.workbench.log(
                            LogLevel::Warning,
                            "The game wasn't saved: save games are kept in a project.",
                        ),
                    }
                }
                WorkbenchAction::LoadGame(slot) => {
                    let path = self
                        .projects
                        .root()
                        .and_then(|root| crate::play_files::save_path(root, &slot));
                    let read = path.as_ref().map(|p| {
                        crate::play_files::read_limited(p, crate::play_files::MAX_SAVE_BYTES)
                    });
                    match read {
                        Some(Ok(Some(text))) => self.workbench.load_game_from_code(&text, &slot),
                        Some(Ok(None)) => self.workbench.log(
                            LogLevel::Warning,
                            format!("There's no save called {slot} to load."),
                        ),
                        Some(Err(why)) => self
                            .workbench
                            .log(LogLevel::Error, format!("Couldn't load {slot}: {why}.")),
                        None => self.workbench.log(
                            LogLevel::Warning,
                            "Save games are kept in a project: open one first.",
                        ),
                    }
                }
                WorkbenchAction::OpenExternally(path) => {
                    if let Err(why) = open_externally(&path) {
                        self.workbench.log(
                            LogLevel::Error,
                            format!("Couldn't open {}: {why}.", file_name(&path)),
                        );
                    }
                }
                WorkbenchAction::MoveFile { from, to } => {
                    match self.projects.move_path(&from, &to) {
                        Ok(()) => {
                            self.workbench.log(
                                LogLevel::Info,
                                format!("Moved {} to {}.", from.display(), to.display()),
                            );
                            self.follow_area_file(&from, Some(&to));
                        }
                        Err(why) => self.workbench.log(
                            LogLevel::Error,
                            format!("Couldn't move {}: {why}.", from.display()),
                        ),
                    }
                    self.rescan_now();
                }
                WorkbenchAction::NewFolder(parent) => {
                    if let Err(why) = self.projects.new_folder(&parent) {
                        self.workbench
                            .log(LogLevel::Error, format!("Couldn't make a folder: {why}."));
                    }
                    self.rescan_now();
                }
                WorkbenchAction::DeleteFile(path) => {
                    match self.projects.delete(&path) {
                        Ok(()) => {
                            self.workbench.log(
                                LogLevel::Info,
                                format!(
                                    "Moved {} to the Recycle Bin; anything using it keeps the \
                                     last version until Solis restarts.",
                                    path.display()
                                ),
                            );
                            self.follow_area_file(&path, None);
                        }
                        Err(why) => self.workbench.log(
                            LogLevel::Error,
                            format!("Couldn't delete {}: {why}.", path.display()),
                        ),
                    }
                    self.rescan_now();
                }
            }
        }
    }

    /// After the Library moved (or deleted) `from` to `to`, both inside the
    /// assets: if the open area's file went with it, the area and the
    /// watch on its file follow, so the next save goes to the new place
    /// instead of making the old file again.
    fn follow_area_file(&mut self, from: &Path, to: Option<&Path>) {
        let Some(assets) = self.projects.db().map(|db| db.project().assets()) else {
            return;
        };
        let to = to.map(|to| assets.join(to));
        match self.workbench.file_moved(&assets.join(from), to.as_deref()) {
            Some(Some(path)) => self.area_files.watch(&path),
            Some(None) => self.area_files.forget(),
            None => {}
        }
    }

    /// Drops what waited for a save that failed or was cancelled; a quit
    /// among it is called off, so a later close asks again.
    fn drop_queue(&mut self) {
        self.queued.clear();
        self.workbench.quit_cancelled();
    }

    /// Looks at the assets folder now (after Solis changed it), rather
    /// than waiting for the watcher.
    fn rescan_now(&mut self) {
        let (changes, notes) = self.projects.rescan();
        self.follow_changes(changes, notes);
        self.refresh_library();
    }

    /// Lists the assets folder again for the Library, and asks for any
    /// thumbnails missing.
    fn refresh_library(&mut self) {
        self.listing = self.projects.listing();
        self.unreadable.clear();
        if let Some(db) = self.projects.db() {
            self.thumbnails
                .want(&self.listing, &db.project().assets(), self.gpu.as_ref());
        }
        self.show_library();
    }

    /// Hands the Library its list, with the thumbnails made so far.
    fn show_library(&mut self) {
        let thumbnails = &self.thumbnails;
        let (items, folders) = Projects::library(&self.listing, |p| thumbnails.get(p));
        // Every assembly in the project is known to the editor, so game
        // code can make copies of any (`spawn`). A broken one shows in
        // Problems when it's used.
        let assemblies: Vec<AssetId> = items
            .iter()
            .filter(|item| item.kind == editor::LibraryKind::Assembly)
            .filter_map(|item| item.id)
            .filter(|id| !self.workbench.assemblies().contains_key(id))
            .filter(|id| !self.unreadable.contains(id))
            .collect();
        self.workbench.set_library(items);
        self.workbench.set_folders(folders);
        let read_some = !assemblies.is_empty();
        for id in assemblies {
            if let Err(why) = self.load_assembly(id, false) {
                log::debug!("couldn't read an assembly ahead: {why}");
                self.unreadable.insert(id);
            }
        }
        // And their models, so what game code makes is drawn.
        if read_some {
            self.load_area_models();
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
        // A name that works on every system (not a device's, say), from
        // the entity's name, which an area file can set to anything.
        let safe = asset_db::file_name_from(&name, "Assembly");
        let folder = db.project().assets().join("assemblies");
        let mut path = folder.join(format!("{safe}.{}", asset_db::ASSEMBLY_EXTENSION));
        let mut n = 2;
        while std::fs::symlink_metadata(&path).is_ok() {
            path = folder.join(format!("{safe} {n}.{}", asset_db::ASSEMBLY_EXTENSION));
            n += 1;
        }
        let text = self.file_text(&file);
        let written = asset_db::own_folder(&db.project().assets(), "assemblies")
            .and_then(|_| asset_db::write_atomically(&path, text.as_bytes()))
            .and_then(|()| self.projects.adopt(&path));
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

    /// Makes the turret sample in `folder` as a new project, opens it and
    /// its area, or says why it couldn't.
    fn make_sample_game(&mut self, folder: &Path) {
        if let Err(why) = crate::sample_game::make(folder) {
            self.workbench.log(
                LogLevel::Error,
                format!(
                    "Couldn't make the sample game in {}: {why}.",
                    folder.display()
                ),
            );
            return;
        }
        self.open_project(folder, false);
        let area = folder
            .join(asset_db::ASSETS_FOLDER)
            .join(crate::sample_game::AREA);
        self.open_area(&area);
        self.workbench.log(
            LogLevel::Info,
            "The sample game is ready. Press Play: the turret shoots at the drone, and the \
             Console tells the story. Its code is in the Library's Code tab.",
        );
    }

    /// Makes the playground in `folder` as a new project, opens it and
    /// its area, or says why it couldn't.
    fn make_playground(&mut self, folder: &Path) {
        if let Err(why) = crate::playground::make(folder) {
            self.workbench.log(
                LogLevel::Error,
                format!(
                    "Couldn't make the playground in {}: {why}.",
                    folder.display()
                ),
            );
            return;
        }
        self.open_project(folder, false);
        let area = folder
            .join(asset_db::ASSETS_FOLDER)
            .join(crate::playground::AREA);
        self.open_area(&area);
        self.workbench.log(
            LogLevel::Info,
            "The playground is ready. Press Play and click the Game panel: W A S D walk, the \
             mouse looks, Space jumps, Shift sprints, F6 saves and F7 loads. Esc gives the \
             editor the mouse back.",
        );
    }

    /// Makes the benchmark in `folder` as a new project, opens it and its
    /// target area, or says why it couldn't.
    fn make_benchmark(&mut self, folder: &Path) {
        if let Err(why) = crate::benchmark::make(folder) {
            self.workbench.log(
                LogLevel::Error,
                format!(
                    "Couldn't make the benchmark in {}: {why}.",
                    folder.display()
                ),
            );
            return;
        }
        self.open_project(folder, false);
        let area = folder
            .join(asset_db::ASSETS_FOLDER)
            .join(crate::benchmark::FIRST_AREA);
        self.open_area(&area);
        self.workbench.log(
            LogLevel::Info,
            "The benchmark is ready. Press Play and open the Profiler: target.solarea is 2,000 \
             ships and 20,000 lasers, stress.solarea 10,000 and 100,000, and ceiling.solarea \
             adds ships until 60 frames a second ends, then says where in the Console.",
        );
    }

    /// Adds the sample area. With a project open it becomes a file in the
    /// project's assets, imported like any other, so a saved area that
    /// uses it finds it again; without one it's read straight from memory.
    fn add_sample(&mut self) {
        const NAME: &str = "Sample area";
        if let Some(cache) = self.projects.cache() {
            // Written beside the cache first, then copied in under a name
            // not yet taken, as a model from elsewhere would be.
            let staged = cache.join(format!("{NAME}.glb"));
            let written = std::fs::create_dir_all(&cache).and_then(|()| {
                asset_db::write_atomically(&staged, &assets::sample::showcase_glb())
            });
            match written {
                Ok(()) => self.import(&staged),
                Err(why) => self.workbench.log(
                    LogLevel::Error,
                    format!("Couldn't add the sample area to the project: {why}."),
                ),
            }
            let _ = std::fs::remove_file(&staged);
            return;
        }
        let id = self.importer.start_sample(NAME);
        self.to_place.insert(id);
        self.workbench
            .log(LogLevel::Info, "Adding the sample area…".to_string());
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
                Err(why) => {
                    self.workbench.placing_failed(id);
                    self.workbench
                        .log(LogLevel::Error, format!("Couldn't place it: {why}."));
                }
            }
            return;
        }
        let Some(file) = self.projects.db().and_then(|db| db.file(id)) else {
            self.workbench.placing_failed(id);
            return;
        };
        self.to_place.insert(id);
        self.importer.start(id, &file, self.projects.cache());
    }
}

impl App {
    /// One frame of the splash screen: asks the update question when
    /// there's a newer build, moves starting on, and opens the editor (or
    /// closes, to install a new build) when it's done.
    fn splash(&mut self, ui: &mut egui::Ui) {
        use crate::startup::Next;
        let ctx = ui.ctx().clone();
        let Some(startup) = self.startup.as_mut() else {
            return;
        };
        if let Some(offer) = startup.offer()
            && let Some(answer) = editor::splash::update_question(
                &ctx,
                branding::APP_NAME,
                &offer,
                &mut startup.skip_tick,
                editor::splash::BELOW_THE_SPLASH,
            )
        {
            if answer == editor::UpdateAnswer::OpenDownloadPage
                && let Err(why) = opener::open(crate::updates::releases_page())
            {
                log::warn!("couldn't open the download page: {why}");
            }
            startup.answer(answer, |release| {
                crate::updates::start_download(&ctx, release)
            });
        }
        let doing = startup.doing();
        // Full while the question waits; sweeping while working.
        let progress = startup.offer().map(|_| 1.0);
        match startup.step(self.began.elapsed()) {
            Next::Wait => {}
            Next::Open(note) => {
                if let Some(build) = startup.skipped() {
                    self.updates.skip(build);
                }
                if let Some(check) = startup.take_check() {
                    self.updates.adopt(check, &mut self.workbench);
                }
                if let Some((level, text)) = note {
                    self.workbench.log(level, text);
                }
                self.startup = None;
                ctx.request_repaint();
                return;
            }
            Next::Install { installer } => {
                // The window closes, and `run` starts the installer, which
                // opens the new build.
                self.updates.install_on_close(installer);
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        editor::splash::show(
            ui,
            &editor::SplashView {
                info: &crate::app_info(),
                accent: self.workbench.accent(),
                logo: self.logo.as_ref().map(egui::TextureHandle::id),
                doing: &doing,
                progress,
                time: self.began.elapsed().as_secs_f64(),
            },
        );
        // The bar sweeps, and the minimum time runs out without input.
        ctx.request_repaint_after(std::time::Duration::from_millis(16));
    }
}

/// The logo as a texture, for the splash (none if it can't be read).
fn load_logo(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    let icon = eframe::icon_data::from_png_bytes(branding::LOGO_PNG).ok()?;
    let image = egui::ColorImage::from_rgba_unmultiplied(
        [icon.width as usize, icon.height as usize],
        &icon.rgba,
    );
    Some(ctx.load_texture("logo", image, egui::TextureOptions::LINEAR))
}

/// The models the open project needs loaded: those the area uses, and
/// every model any of the project's assemblies uses, since game code can
/// make copies of any assembly while playing (`spawn`). Without the
/// second, copies made only by game code had no model to draw (the
/// benchmark's stress area showed only its lasers).
fn models_wanted<'a>(
    area: &'a area::Area,
    assemblies: impl Iterator<Item = &'a area::Area>,
) -> HashSet<AssetId> {
    std::iter::once(area)
        .chain(assemblies)
        .flat_map(|a| {
            a.with::<area::builtin::Model>()
                .filter_map(|(_, m)| m.asset.map(|r| r.asset))
        })
        .collect()
}

/// The project open when Solis last closed, as saved (an optional path:
/// none when no project was open).
fn saved_last_project(storage: &dyn eframe::Storage) -> Option<PathBuf> {
    eframe::get_value::<Option<PathBuf>>(storage, LAST_PROJECT_KEY).flatten()
}

/// Whether two paths are the same folder (however they're written).
fn same_folder(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
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
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        // Skip this version is saved at once, not only on closing.
        if let Some(skipped) = self.updates.take_unsaved_skip()
            && let Some(storage) = frame.storage_mut()
        {
            eframe::set_value(storage, SKIPPED_BUILD_KEY, &skipped);
            storage.flush();
        }
        // The splash screen until starting is done (decision 0045).
        if self.startup.is_some() {
            self.splash(ui);
            return;
        }
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
        // A project or area chosen in a window opened before Play waits
        // until the game stops, so it never replaces the game's copy.
        let editing = self.workbench.play_state() == editor::PlayState::Editing;
        if editing && let Some((purpose, folder)) = self.importer.chosen_folder() {
            match purpose {
                FolderFor::SampleGame => self.make_sample_game(&folder),
                FolderFor::Benchmark => self.make_benchmark(&folder),
                FolderFor::Playground => self.make_playground(&folder),
                _ => self.open_project(&folder, purpose == FolderFor::NewProject),
            }
        }
        let (changes, notes) = self.projects.poll();
        self.follow_changes(changes, notes);
        if self.projects.take_changed() {
            self.refresh_library();
            // A code file may have changed: checked again on a worker, and
            // reported only if the code is different.
            if let Some(assets) = self.projects.db().map(|db| db.project().assets()) {
                // armour.rs changed: say that its changes don't run.
                let (_, note) = crate::sample_game::rust_components(&assets);
                if note != self.rust_note
                    && let Some(note) = &note
                {
                    self.workbench.log(LogLevel::Warning, note.clone());
                }
                self.rust_note = note;
                self.game_code.check(assets);
            }
        }
        if let Some(checked) = self.game_code.poll() {
            self.workbench
                .set_code_problems(checked.problems, &checked.notes);
            if let Some(program) = checked.program {
                self.workbench.set_program(program);
            }
        }
        if self.thumbnails.poll(&ctx, self.gpu.as_mut()) {
            self.show_library();
        }
        match if editing {
            self.area_files.chosen()
        } else {
            None
        } {
            Some((Purpose::Open, Some(path))) => self.open_area(&path),
            Some((Purpose::Save, Some(path))) => {
                if self.save_to(&path) {
                    self.run_queue(&ctx);
                } else {
                    self.drop_queue();
                }
            }
            // Cancelled: whatever waited for the save (a quit) doesn't
            // happen.
            Some((_, None)) => {
                if let Some(path) = self.area_files.take_refused() {
                    self.workbench.log(
                        LogLevel::Warning,
                        format!(
                            "Not saved: {} already exists. Choose Save as again and pick it \
                             to replace it.",
                            file_name(&path)
                        ),
                    );
                }
                self.drop_queue();
            }
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
        self.updates.frame(&mut self.workbench);
        self.show_library_messages();
        self.workbench.set_gamepad(self.gamepads.read());
        if let Some((what, Some(path))) = self.recordings.chosen() {
            let limit = match what {
                crate::play_files::PlayFile::Recording => input::MAX_RECORDING_BYTES as u64,
                crate::play_files::PlayFile::Save => crate::play_files::MAX_SAVE_BYTES,
            };
            match crate::play_files::read_limited(&path, limit) {
                Ok(Some(text)) => match what {
                    crate::play_files::PlayFile::Recording => {
                        self.workbench.replay(&text, &file_name(&path));
                    }
                    crate::play_files::PlayFile::Save => {
                        self.workbench.load_game(&text, &file_name(&path));
                    }
                },
                Ok(None) => {}
                Err(why) => self.workbench.log(
                    LogLevel::Error,
                    format!("Couldn't read {}: {why}.", file_name(&path)),
                ),
            }
        }
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
        // Frames back to back only while something moves; ten a second
        // while background work is on its way; otherwise one a second, for
        // changes on disk and messages (decision 0026). Input always draws
        // a frame at once.
        if self.workbench.wants_frames(&ctx) {
            ctx.request_repaint();
        } else if self.importer.waiting()
            || self.thumbnails.waiting()
            || self.projects.waiting()
            || self.game_code.waiting()
            || self.area_files.is_asking()
        {
            ctx.request_repaint_after(WAITING_FRAME);
        } else {
            ctx.request_repaint_after(IDLE_FRAME);
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, LAYOUT_KEY, self.workbench.layout());
        eframe::set_value(storage, ACCENT_KEY, &self.workbench.accent());
        eframe::set_value(storage, OVERLAYS_KEY, &self.workbench.overlays());
        eframe::set_value(storage, SNAPPING_KEY, &self.workbench.snapping());
        eframe::set_value(storage, CAMERA_KEY, &self.workbench.camera());
        eframe::set_value(storage, LOOK_KEY, &self.workbench.look());
        eframe::set_value(storage, LAST_PROJECT_KEY, &self.projects.root());
        eframe::set_value(storage, RECENT_PROJECTS_KEY, &self.projects.recent());
        eframe::set_value(storage, VIEWS_KEY, &self.workbench.views());
        eframe::set_value(storage, AUTO_UPDATE_KEY, &self.workbench.auto_update());
        eframe::set_value(storage, SKIPPED_BUILD_KEY, &self.updates.skipped());
        eframe::set_value(
            storage,
            CONSOLE_HISTORY_KEY,
            &self.workbench.console_history(),
        );
    }
}

/// Opens the editor window and runs until it closes.
pub fn run(info: AppInfo, imports: Vec<PathBuf>) -> Result<(), String> {
    start_jobs();
    let title = format!("{} {}", info.name, info.version);
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(title)
            .with_app_id(branding::APP_ID)
            .with_inner_size(FIRST_SIZE)
            .with_min_inner_size(MIN_SIZE)
            .with_icon(window_icon()),
        renderer: eframe::Renderer::Wgpu,
        wgpu_options: gpu_options(),
        ..Default::default()
    };
    let pending = crate::updates::PendingSlot::default();
    let slot = pending.clone();
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        eframe::run_native(
            branding::APP_ID,
            options,
            Box::new(move |cc| Ok(Box::new(App::new(cc, info, imports, slot)))),
        )
    }));
    // The window is gone: a downloaded update installs now (decision 0043).
    crate::updates::install_pending(&pending);
    match outcome {
        Ok(result) => result.map_err(|err| err.to_string()),
        Err(payload) => Err(explain_panic(payload.as_ref())),
    }
}

/// The logo, for the window's corner and the taskbar (empty if it somehow
/// can't be read: the system's own icon then).
fn window_icon() -> std::sync::Arc<egui::IconData> {
    std::sync::Arc::new(eframe::icon_data::from_png_bytes(branding::LOGO_PNG).unwrap_or_default())
}

/// The job system's workers (decision 0031): one per core, leaving one for
/// the window. Started once; a second call (tests) keeps the first.
pub fn start_jobs() {
    let cores = std::thread::available_parallelism().map_or(2, std::num::NonZeroUsize::get);
    let _ = rayon::ThreadPoolBuilder::new()
        .num_threads(cores.saturating_sub(1).max(1))
        .thread_name(|i| format!("jobs {i}"))
        .build_global();
}

/// The GPU as egui-wgpu sets it up, but asking for timestamps too when the
/// card has them, for the Profiler's GPU times.
fn gpu_options() -> eframe::egui_wgpu::WgpuConfiguration {
    use eframe::egui_wgpu::{WgpuConfiguration, WgpuSetup, WgpuSetupCreateNew};
    let mut setup = WgpuSetupCreateNew::without_display_handle();
    let usual = std::sync::Arc::clone(&setup.device_descriptor);
    setup.device_descriptor = std::sync::Arc::new(move |adapter| {
        let mut descriptor = usual(adapter);
        descriptor.required_features |=
            adapter.features() & eframe::wgpu::Features::TIMESTAMP_QUERY;
        descriptor
    });
    WgpuConfiguration {
        wgpu_setup: WgpuSetup::CreateNew(setup),
        ..WgpuConfiguration::default()
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

/// Opens a file in the program the computer chooses for it, or shows a
/// folder. A folder goes straight to Explorer on Windows, so it can never
/// be taken for a program to run (if it vanished and a file of a similar
/// name took its place, say).
fn open_externally(path: &std::path::Path) -> std::io::Result<()> {
    let meta = std::fs::metadata(path)?;
    if meta.is_dir() {
        #[cfg(windows)]
        {
            return std::process::Command::new("explorer.exe")
                .arg(path)
                .spawn()
                .map(|_| ());
        }
    } else if !meta.is_file() {
        return Err(std::io::Error::other("it isn't a file or a folder"));
    }
    opener::open(path).map_err(std::io::Error::other)
}

#[cfg(test)]
mod tests {
    #[test]
    fn models_only_game_code_places_are_loaded_too() {
        use area::builtin::Model;
        use reflect::AssetRef;
        let model = |id: AssetId| Model {
            asset: Some(AssetRef {
                asset: id,
                part: None,
            }),
        };
        let (in_area, in_assembly, nowhere) =
            (AssetId::random(), AssetId::random(), AssetId::random());
        let mut area = area::Area::new();
        area.spawn(area::EntityData::new("Rock").with(&model(in_area)));
        area.spawn(area::EntityData::new("Empty"));
        let mut fighter = area::Area::new();
        fighter.spawn(area::EntityData::new("Fighter").with(&model(in_assembly)));
        let wanted = super::models_wanted(&area, [&fighter].into_iter());
        assert!(wanted.contains(&in_area));
        assert!(
            wanted.contains(&in_assembly),
            "spawned by game code: needed too"
        );
        assert!(!wanted.contains(&nowhere));
        assert_eq!(wanted.len(), 2);
        // Nothing placed anywhere: nothing wanted.
        assert!(super::models_wanted(&area::Area::new(), std::iter::empty()).is_empty());
    }

    use super::*;

    #[test]
    fn the_same_folder_is_recognised_however_its_written() {
        let folder = std::env::temp_dir().join(format!("solis-same-{}", std::process::id()));
        std::fs::create_dir_all(folder.join("assets")).unwrap();
        assert!(same_folder(&folder, &folder.join("assets").join("..")));
        assert!(!same_folder(&folder, &folder.join("assets")));
        assert!(same_folder(Path::new("/no/such"), Path::new("/no/such")));
        assert!(!same_folder(Path::new("/no/such"), Path::new("/no/other")));
    }

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

    /// Settings kept in memory, as eframe keeps them on disk.
    #[derive(Default)]
    struct Memory(std::collections::HashMap<String, String>);

    impl eframe::Storage for Memory {
        fn get_string(&self, key: &str) -> Option<String> {
            self.0.get(key).cloned()
        }
        fn set_string(&mut self, key: &str, value: String) {
            self.0.insert(key.to_string(), value);
        }
        fn remove_string(&mut self, key: &str) {
            self.0.remove(key);
        }
        fn flush(&mut self) {}
    }

    #[test]
    fn the_last_project_reopens_as_it_was_saved() {
        // Regression: it was saved as an optional path but read back as a
        // plain one, so it never reopened.
        let mut storage = Memory::default();
        let root = Path::new("/projects/harbour");
        eframe::set_value(&mut storage, LAST_PROJECT_KEY, &Some(root));
        assert_eq!(saved_last_project(&storage), Some(root.to_path_buf()));
        eframe::set_value(&mut storage, LAST_PROJECT_KEY, &None::<&Path>);
        assert_eq!(saved_last_project(&storage), None);
        assert_eq!(saved_last_project(&Memory::default()), None);
    }
}
