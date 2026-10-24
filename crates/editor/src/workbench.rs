//! The whole editor window: the top bar and the docked panels
//! ([decision 0004](../../../docs/decisions/0004-editor-layout.md)).
//!
//! The workbench never touches the window, the GPU or files itself: it
//! returns [`WorkbenchAction`]s for the app to carry out, and draws the
//! viewport through a [`ViewportRenderer`] it is handed each frame.

use crate::area_view::AreaView;
use crate::camera::Camera;
#[cfg(test)]
use crate::console::CONSOLE_LIMIT;
use crate::console::{Console, LogLevel, LogLine};
use crate::document::Document;
use crate::layout::{self, Preset};
use crate::library::{LibraryItem, ProjectInfo};
use crate::models::ModelInfo;
use crate::panel::Panel;
use crate::panels::{PanelViewer, drop_hint};
use crate::theme::{self, Accent};
use crate::viewport::{Overlays, Tool, Viewport, ViewportRenderer};
use egui::Ui;
use egui_dock::{DockArea, DockState};
use reflect::{AssetId, EntityId};
use std::collections::{BTreeMap, HashSet};

/// Facts about the program, shown in the window title and the About box.
/// The app passes these in, so this crate never names the program itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppInfo {
    /// The program's name, such as "Solis".
    pub name: String,
    /// The version, such as "0.0.1".
    pub version: String,
}

/// Something the app must do, asked for by the editor.
#[derive(Debug, Clone, PartialEq)]
pub enum WorkbenchAction {
    /// Close the window.
    Quit,
    /// Ask which model file to import (File > Import model…).
    ImportModel,
    /// Bring in the built-in sample area (Help > Add the sample area).
    AddSampleArea,
    /// Make a new project in a folder the user picks (File > New project…).
    NewProject,
    /// Open a project from a folder the user picks (File > Open project…).
    OpenProject,
    /// Open the project in this folder (File > Recent projects).
    OpenRecent(std::path::PathBuf),
    /// Place the project's asset where the camera looks (a double-click in
    /// the Library).
    PlaceAsset(AssetId),
    /// Start a new area (File > New area, Ctrl+N): the app calls
    /// [`Workbench::new_area`].
    NewArea,
    /// Ask which area file to open (File > Open area…, Ctrl+O).
    OpenArea,
    /// Save the area to its file, or ask where if it has none (Ctrl+S).
    SaveArea,
    /// Ask where to save the area (Ctrl+Shift+S).
    SaveAreaAs,
    /// Save this entity and everything under it as an assembly in the
    /// project, then turn it into a copy of it with
    /// [`Workbench::make_copy`].
    CreateAssembly(EntityId),
    /// Open this area file (a double-click in the Library); the app reads
    /// it and calls [`Workbench::open_area`].
    OpenAreaFile(std::path::PathBuf),
    /// Open this file or folder in the system's own program or file
    /// browser.
    OpenExternally(std::path::PathBuf),
    /// Rename or move a file or folder inside the project's assets, with
    /// its sidecar.
    MoveFile {
        /// Where it is, inside the assets.
        from: std::path::PathBuf,
        /// Where it goes, inside the assets.
        to: std::path::PathBuf,
    },
    /// Make a new folder inside this one (inside the assets).
    NewFolder(std::path::PathBuf),
    /// Move a file or folder (inside the assets) to the Recycle Bin.
    DeleteFile(std::path::PathBuf),
    /// Change one field of an entity inside an assembly's file (Apply to
    /// assembly); the app saves it and hands it back with
    /// [`Workbench::set_assembly`], which updates every copy.
    ApplyToAssembly {
        /// The assembly.
        assembly: AssetId,
        /// The entity inside it.
        path: Vec<EntityId>,
        /// The component's name.
        component: String,
        /// The field's name.
        field: String,
        /// Its new value.
        value: reflect::Value,
    },
}

/// The editor's state, kept between frames.
pub struct Workbench {
    pub(crate) info: AppInfo,
    pub(crate) dock: DockState<Panel>,
    pub(crate) accent: Accent,
    pub(crate) viewport: Viewport,
    pub(crate) console: Console,
    /// The open area and what's selected in it.
    pub(crate) document: Document,
    /// What the editor knows about the imported models, by asset.
    pub(crate) models: BTreeMap<AssetId, ModelInfo>,
    /// Whether files are being dragged over the window.
    pub(crate) files_hovering: bool,
    pub(crate) about_open: bool,
    /// The tabs that were in front last frame, to underline them in the
    /// accent colour.
    pub(crate) in_front: HashSet<Panel>,
    /// The open project, if any.
    pub(crate) project: Option<ProjectInfo>,
    /// Projects opened lately, newest first, for File > Recent projects.
    pub(crate) recent: Vec<std::path::PathBuf>,
    /// The project's assets, for the Library.
    pub(crate) library: Vec<LibraryItem>,
    /// The project's assemblies the area uses, as read from their files.
    pub(crate) assemblies: BTreeMap<AssetId, std::sync::Arc<area::text::AreaFile>>,
    /// Actions asked for outside [`Self::show`]'s own flow (menus,
    /// Properties), handed to the app with the next frame's.
    pub(crate) outbox: Vec<WorkbenchAction>,
    /// What waits for an answer about unsaved changes: Save, Don't save
    /// or Cancel.
    pub(crate) unsaved_prompt: Option<WorkbenchAction>,
    /// The area as the file now on disk holds it, waiting for Keep mine
    /// or Load the file.
    pub(crate) pending_reload: Option<area::Area>,
    /// Whether the window may close despite unsaved changes (the user
    /// said Don't save).
    pub(crate) may_close: bool,
    /// Counts presses of the mouse: changes to one field during one press
    /// (a drag) are one undo step.
    pub(crate) gesture: u64,
    /// While a gizmo is dragged: what moves, as it was when the drag
    /// began, and the drag's gesture.
    pub(crate) moving: Option<(crate::transforming::Targets, u64)>,
    /// What the Area panel searches for, folds, hides and locks.
    pub(crate) area_panel: crate::area_panel::AreaPanelState,
    /// Components' values copied in Properties, to paste.
    pub(crate) copied: Vec<(String, reflect::ComponentData)>,
    /// The project's folders, for the Library's tree.
    pub(crate) folders: Vec<String>,
    /// How the Library is shown.
    pub(crate) library_state: crate::library::LibraryState,
    /// Where assets being placed go, when they were dropped somewhere:
    /// the point, and the entity they go under.
    pub(crate) place_targets: BTreeMap<AssetId, (glam::Vec3, Option<EntityId>)>,
    /// Mistakes found in the area, for the Problems panel.
    pub(crate) problems: crate::problems::Problems,
    /// The game, while it runs.
    pub(crate) playing: Option<crate::play::Playing>,
}

impl Workbench {
    /// A workbench with a saved panel layout, if it is usable (otherwise the
    /// default one), and a new area.
    pub fn new(info: AppInfo, saved_layout: Option<DockState<Panel>>) -> Self {
        Self {
            info,
            dock: layout::restore(saved_layout),
            accent: Accent::default(),
            viewport: Viewport::default(),
            console: Console::default(),
            document: Document::default(),
            models: BTreeMap::new(),
            files_hovering: false,
            about_open: false,
            in_front: HashSet::new(),
            gesture: 0,
            moving: None,
            area_panel: Default::default(),
            copied: Vec::new(),
            folders: Vec::new(),
            library_state: Default::default(),
            place_targets: BTreeMap::new(),
            problems: Default::default(),
            playing: None,
            assemblies: BTreeMap::new(),
            outbox: Vec::new(),
            unsaved_prompt: None,
            pending_reload: None,
            may_close: false,
            project: None,
            recent: Vec::new(),
            library: Vec::new(),
        }
    }

    /// Uses `accent` (as saved last time).
    #[must_use]
    pub fn with_accent(mut self, accent: Accent) -> Self {
        self.accent = accent;
        self
    }

    /// Uses `overlays` (as saved last time).
    #[must_use]
    pub fn with_overlays(mut self, overlays: Overlays) -> Self {
        self.viewport.overlays = overlays;
        self
    }

    /// Uses `snapping` (as saved last time), with anything out of range
    /// put back.
    #[must_use]
    pub fn with_snapping(mut self, snapping: crate::Snapping) -> Self {
        self.viewport.snapping = snapping.sanitized();
        self
    }

    /// Snapping, axis locks and world or object axes, for saving.
    pub fn snapping(&self) -> crate::Snapping {
        self.viewport.snapping
    }

    /// Tells the workbench whether files are being dragged over the window,
    /// so it can show where to drop them.
    pub fn set_files_hovering(&mut self, hovering: bool) {
        self.files_hovering = hovering;
    }

    /// What the editor knows about the imported models, by asset.
    pub fn models(&self) -> &BTreeMap<AssetId, ModelInfo> {
        &self.models
    }

    /// The open area and its selection.
    pub fn document(&self) -> &Document {
        &self.document
    }

    /// The entity shown in Properties, if any.
    pub fn selection(&self) -> Option<EntityId> {
        self.document.selection.primary()
    }

    /// Shows `project` as the open one (or none). A different project
    /// starts a new area, as the old one's models belong to the old
    /// project.
    pub fn set_project(&mut self, project: Option<ProjectInfo>) {
        if project != self.project {
            self.replace_document(Document::default());
            self.models.clear();
            self.library.clear();
            // Nothing of the old project's is used with the new one: its
            // assemblies (a copy of a project shares IDs with it), where
            // things were dropped, a question about its file, the
            // Library's place in its folders.
            self.assemblies.clear();
            self.place_targets.clear();
            self.pending_reload = None;
            self.unsaved_prompt = None;
            self.library_state = Default::default();
            if let Some(p) = &project {
                self.log(LogLevel::Info, format!("Opened the project {}.", p.name));
            }
        }
        self.project = project;
    }

    /// The open project, if any.
    pub fn project(&self) -> Option<&ProjectInfo> {
        self.project.as_ref()
    }

    /// The projects for File > Recent projects, newest first.
    pub fn set_recent(&mut self, recent: Vec<std::path::PathBuf>) {
        self.recent = recent;
    }

    /// The project's files, for the Library.
    pub fn set_library(&mut self, items: Vec<LibraryItem>) {
        self.library = items;
    }

    /// The project's folders inside its assets, `/` between names, for
    /// the Library's tree.
    pub fn set_folders(&mut self, folders: Vec<String>) {
        if !folders.contains(&self.library_state.folder) {
            self.library_state.folder.clear();
        }
        self.folders = folders;
    }

    /// Whether the model `id` is loaded (placed or reloaded before).
    pub fn has_model(&self, id: AssetId) -> bool {
        self.models.contains_key(&id)
    }

    /// Swaps in a model that changed on disk (or loads one an opened area
    /// uses): everything using it now draws it. Not an undo step: the area
    /// didn't change.
    pub fn reload_model(&mut self, info: ModelInfo, warnings: &[String]) {
        let (name, triangles) = (info.name.clone(), info.triangles);
        if self.models.insert(info.asset, info).is_none() {
            // First loaded for an opened area: nothing to announce.
            return;
        }
        self.log(
            LogLevel::Info,
            format!(
                "Reloaded {name}: {} triangles.",
                crate::models::grouped(triangles)
            ),
        );
        for warning in warnings {
            self.log(LogLevel::Warning, format!("{name}: {warning}"));
        }
    }

    /// Starts a new, unsaved area (File > New area). Doesn't ask about
    /// unsaved changes: asking comes before the action.
    pub fn new_area(&mut self) {
        self.replace_document(Document::default());
        self.pending_reload = None;
    }

    /// Shows `area`, read from `file`, in place of the open one.
    pub fn open_area(&mut self, area: area::Area, file: std::path::PathBuf) {
        self.replace_document(Document::opened(area, file));
        self.pending_reload = None;
        self.log(LogLevel::Info, format!("Opened {}.", self.document.name()));
    }

    /// Uses `document` in place of the open one, keeping the components
    /// registered so far (game code registers its own once, not once per
    /// area).
    fn replace_document(&mut self, mut document: Document) {
        document.registry = std::mem::take(&mut self.document.registry);
        self.document = document;
    }

    /// A file or folder was moved from `from` to `to` (full paths), or
    /// deleted (`to` is `None`). If it was the open area's file, or a
    /// folder holding it, the area follows it; returns its new file then
    /// (`None` inside: it was deleted).
    pub fn file_moved(
        &mut self,
        from: &std::path::Path,
        to: Option<&std::path::Path>,
    ) -> Option<Option<std::path::PathBuf>> {
        let file = self.document.file()?.to_path_buf();
        let rest = file.strip_prefix(from).ok()?;
        let moved = to.map(|to| {
            if rest.as_os_str().is_empty() {
                to.to_path_buf()
            } else {
                to.join(rest)
            }
        });
        self.document.file_moved(moved.clone());
        match &moved {
            Some(path) => self.log(
                LogLevel::Info,
                format!("The open area is now {}.", path.display()),
            ),
            None => self.log(
                LogLevel::Warning,
                "The open area's file was deleted; saving asks where to put it.",
            ),
        }
        Some(moved)
    }

    /// Notes that the area was saved to `file`.
    pub fn mark_saved(&mut self, file: std::path::PathBuf) {
        self.document.mark_saved(file);
        self.log(LogLevel::Info, format!("Saved {}.", self.document.name()));
    }

    /// The open area's file changed on disk and now holds `area`: it's
    /// loaded as one undoable step, or, with unsaved changes, the user is
    /// asked which to keep.
    pub fn offer_reload(&mut self, area: area::Area) {
        if let Some(playing) = &mut self.playing {
            // The game's copy isn't the file's; offered again after Stop.
            playing.reload = Some(area);
            return;
        }
        if self.document.is_dirty() {
            self.pending_reload = Some(area);
        } else {
            self.reload_area(area);
        }
    }

    fn reload_area(&mut self, area: area::Area) {
        let file = self
            .document
            .file()
            .and_then(|f| f.file_name())
            .map_or_else(
                || self.document.name(),
                |n| n.to_string_lossy().into_owned(),
            );
        let label = format!("Reloaded {file} from disk");
        match self.document.reload(area, &label) {
            Ok(()) => self.log(LogLevel::Info, format!("{label}.")),
            Err(why) => self.log(LogLevel::Error, format!("Couldn't reload {file}: {why}.")),
        }
    }

    /// The panel layout, for saving.
    pub fn layout(&self) -> &DockState<Panel> {
        &self.dock
    }

    /// The accent colour, for saving.
    pub fn accent(&self) -> Accent {
        self.accent
    }

    /// Uses `camera` (as saved last time), with anything unusable in it
    /// replaced (see [`Camera::sanitized`]).
    #[must_use]
    pub fn with_camera(mut self, camera: Camera) -> Self {
        self.viewport.camera = camera.sanitized();
        self
    }

    /// Uses `look` (as saved last time), with anything out of range put
    /// back in range.
    #[must_use]
    pub fn with_look(mut self, look: crate::look::Look) -> Self {
        self.viewport.look = look.sanitized();
        self
    }

    /// How models are shaded, for saving.
    pub fn look(&self) -> crate::look::Look {
        self.viewport.look
    }

    /// The viewport camera of the view in use.
    pub fn camera(&self) -> Camera {
        self.viewport.camera
    }

    /// Uses `views` (the layout and every view's camera and look, as
    /// saved last time), with anything unusable put back.
    #[must_use]
    pub fn with_views(mut self, views: &crate::viewport::Views) -> Self {
        self.viewport.set_views(views);
        self
    }

    /// The layout and every view's camera and look, for saving.
    pub fn views(&self) -> crate::viewport::Views {
        self.viewport.views()
    }

    /// Which viewport overlays are shown, for saving.
    pub fn overlays(&self) -> Overlays {
        self.viewport.overlays
    }

    /// The chosen tool.
    pub fn tool(&self) -> Tool {
        self.viewport.tool
    }

    /// The console's messages, oldest first.
    pub fn log_lines(&self) -> &[LogLine] {
        self.console.lines()
    }

    /// Adds a message to the console.
    pub fn log(&mut self, level: LogLevel, text: impl Into<String>) {
        self.console.push(level, text.into());
    }

    /// Arranges the panels as `preset`.
    pub fn apply_preset(&mut self, preset: Preset) {
        self.dock = preset.layout();
    }

    /// The window title: the program, its version and the open area.
    pub fn title(&self) -> String {
        let mut area = self.document.name();
        if self.document.is_dirty() {
            area.push_str(" •");
        }
        match &self.project {
            Some(project) => format!(
                "{} {} · {} · {area}",
                self.info.name, self.info.version, project.name
            ),
            None => format!("{} {} · {area}", self.info.name, self.info.version),
        }
    }

    /// Draws the editor filling `ui`, and returns what the app must do.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        renderer: &mut dyn ViewportRenderer,
    ) -> Vec<WorkbenchAction> {
        let mut actions = std::mem::take(&mut self.outbox);
        if ui.input(|i| i.pointer.any_pressed()) {
            self.gesture += 1;
        }
        self.tool_keys(ui);
        self.tool_extra_keys(ui);
        self.undo_keys(ui);
        self.editing_keys(ui);
        self.file_keys(ui, &mut actions);
        self.console_key(ui);
        let seconds = ui.input(|i| i.stable_dt);
        self.run_game(seconds);
        egui::Panel::top("top_bar")
            .frame(theme::top_bar_frame())
            .exact_size(34.0)
            .show(ui, |ui| self.top_bar(ui, &mut actions));
        let style = theme::dock_style(ui.style().as_ref(), self.accent);
        let mut shown = HashSet::new();
        let mut area_requests = Vec::new();
        let mut library_requests = Vec::new();
        let mut console_requests = Vec::new();
        let part = self.assembly_part();
        let mut requests = Vec::new();
        let (hidden, locked) = self.area_panel.in_effect(self.document.area());
        self.viewport.area =
            AreaView::of_shown(self.document.area(), &self.models, &hidden, &locked);
        self.viewport.entities = self.document.area().len();
        self.prepare_tools();
        self.find_problems();
        let menu = crate::menu_items::MenuState::of(self);
        let copied: Vec<String> = self.copied.iter().map(|(name, _)| name.clone()).collect();
        // Worked out only while the Game panel is in front.
        let game = match &self.playing {
            Some(playing) if self.in_front.contains(&crate::panel::Panel::Game) => Some(
                crate::game_panel::GameScene::of(&playing.world, &self.models, self.play_state()),
            ),
            Some(_) => None,
            None => None,
        };
        let playing = self.playing.is_some();
        theme::dock_frame().show(ui, |ui| {
            DockArea::new(&mut self.dock)
                .style(style)
                .tab_context_menus(false)
                .show_close_buttons(false)
                .show_add_buttons(false)
                .show_leaf_close_all_buttons(false)
                .show_leaf_collapse_buttons(true)
                .show_inside(
                    ui,
                    &mut PanelViewer {
                        area_state: &mut self.area_panel,
                        copied: &copied,
                        menu: &menu,
                        viewport: &mut self.viewport,
                        renderer,
                        console: &mut self.console,
                        console_requests: &mut console_requests,
                        problems: &self.problems.list,
                        game: game.as_ref(),
                        playing,
                        document: &self.document,
                        models: &self.models,
                        area_requests: &mut area_requests,
                        project: self.project.as_ref(),
                        library: &self.library,
                        folders: &self.folders,
                        library_state: &mut self.library_state,
                        library_requests: &mut library_requests,
                        part: part.as_ref(),
                        requests: &mut requests,
                        accent: self.accent,
                        in_front: &self.in_front,
                        shown: &mut shown,
                    },
                );
        });
        self.in_front = shown;
        self.tool_events();
        for request in requests {
            self.apply_property(request, self.gesture);
        }
        actions.append(&mut self.outbox);
        for request in library_requests {
            self.library_request(request, &mut actions);
        }
        for request in console_requests {
            self.console_request(request, ui.ctx(), &mut actions);
        }
        for request in area_requests {
            match request {
                crate::area_panel::AreaRequest::Command(command) => self.run(command, ui.ctx()),
                other => self.apply_area_request(other),
            }
        }
        if self.files_hovering {
            drop_hint(ui, self.accent);
        }
        self.about(ui);
        self.prompts(ui, &mut actions);
        self.hold_while_playing(&mut actions);
        actions
    }

    /// Q, W, R, S and T pick the tools, unless typing in a field.
    pub(crate) fn tool_keys(&mut self, ui: &Ui) {
        // While flying, W A S D, Q and E move the camera instead.
        if ui.ctx().egui_wants_keyboard_input() || self.viewport.is_flying() {
            return;
        }
        let pressed = ui.input(|i| {
            Tool::ALL
                .into_iter()
                .find(|tool| i.key_pressed(tool.key()) && i.modifiers.is_none())
        });
        if let Some(tool) = pressed {
            self.viewport.tool = tool;
        }
    }

    /// The `'` key (beside Æ on a Norwegian keyboard) jumps to the
    /// console's command line, bringing the Console to the front (decided
    /// with the project lead, 2026-10-23). Not while typing.
    pub(crate) fn console_key(&mut self, ui: &Ui) {
        if ui.ctx().egui_wants_keyboard_input() || self.viewport.is_flying() {
            return;
        }
        let pressed = ui.input_mut(|i| {
            let pressed = i.events.iter().any(|e| {
                matches!(
                    e,
                    egui::Event::Key {
                        key: egui::Key::Quote,
                        pressed: true,
                        modifiers,
                        ..
                    } if !modifiers.shift && !modifiers.command && !modifiers.alt
                )
            });
            // The ' itself isn't typed into the command line: dropped now,
            // or, if it comes a frame later, by the command line.
            let typed = pressed
                && i.events
                    .iter()
                    .any(|e| matches!(e, egui::Event::Text(t) if t == "'"));
            if typed {
                i.events
                    .retain(|e| !matches!(e, egui::Event::Text(t) if t == "'"));
            }
            (pressed, pressed && !typed)
        });
        let (pressed, later) = pressed;
        if pressed {
            self.open_command_line();
            self.console.swallow_quote = later;
        }
    }

    /// Brings the Console to the front and puts the keyboard in its
    /// command line.
    pub(crate) fn open_command_line(&mut self) {
        if let Some(found) = self.dock.find_tab(&crate::panel::Panel::Console) {
            let _ = self.dock.set_active_tab(found);
        }
        self.console.focus = true;
    }

    /// Commands typed in the console, and Tab.
    pub(crate) fn console_request(
        &mut self,
        request: crate::console::ConsoleRequest,
        ctx: &egui::Context,
        actions: &mut Vec<WorkbenchAction>,
    ) {
        match request {
            crate::console::ConsoleRequest::Run(line) => self.run_command(&line, ctx, actions),
            crate::console::ConsoleRequest::Complete => {
                let done = self.complete(&self.console.input);
                self.console.input = done.line;
                self.console.choices = done.choices;
                self.console.cursor_to_end = true;
                self.console.focus = true;
            }
        }
    }

    /// Uses `history` as the console's earlier commands (as saved last
    /// time).
    #[must_use]
    pub fn with_console_history(mut self, history: Vec<String>) -> Self {
        self.console.set_history(history);
        self
    }

    /// Looks for problems in the area again, if anything they depend on
    /// changed.
    pub(crate) fn find_problems(&mut self) {
        use crate::problems::{Fingerprint, Sources};
        let print = Fingerprint::of(
            &self.document,
            self.models.len(),
            self.library.len(),
            self.assemblies.len(),
        );
        if self.problems.is_from(&print) {
            return;
        }
        let known: HashSet<AssetId> = self.library.iter().filter_map(|i| i.id).collect();
        let assemblies: HashSet<AssetId> = self.assemblies.keys().copied().collect();
        self.problems.refresh(
            print,
            &Sources {
                area: self.document.area(),
                registry: &self.document.registry,
                models: &self.models,
                known: &known,
                assemblies: &assemblies,
            },
        );
    }

    /// The problems found in the area, for the app and tests.
    pub fn problems(&self) -> &[crate::problems::Problem] {
        &self.problems.list
    }

    /// The console's earlier commands, oldest first, for saving.
    pub fn console_history(&self) -> &[String] {
        self.console.history()
    }

    /// Ctrl+Z undoes; Ctrl+Y or Ctrl+Shift+Z redoes. Not while typing in a
    /// field, where they undo the typing instead.
    pub(crate) fn undo_keys(&mut self, ui: &Ui) {
        if ui.ctx().egui_wants_keyboard_input() {
            return;
        }
        let (redo, undo) = ui.input_mut(|i| {
            let shift_z = egui::KeyboardShortcut::new(
                egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                egui::Key::Z,
            );
            let y = egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::Y);
            let z = egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::Z);
            // Shift+Z first: a plain Ctrl+Z shortcut would take it too.
            let redo = i.consume_shortcut(&shift_z) || i.consume_shortcut(&y);
            (redo, i.consume_shortcut(&z))
        });
        if redo {
            self.redo();
        } else if undo {
            self.undo();
        }
    }

    /// Ctrl+N, Ctrl+O, Ctrl+S and Ctrl+Shift+S: new, open, save and save
    /// as (decided with the project lead, 2026-10-23). Not while typing.
    pub(crate) fn file_keys(&mut self, ui: &Ui, actions: &mut Vec<WorkbenchAction>) {
        if ui.ctx().egui_wants_keyboard_input() {
            return;
        }
        let command = egui::Modifiers::COMMAND;
        let pressed = ui.input_mut(|i| {
            // Shift+S first: a plain Ctrl+S shortcut would take it too.
            let save_as =
                egui::KeyboardShortcut::new(command | egui::Modifiers::SHIFT, egui::Key::S);
            [
                (save_as, WorkbenchAction::SaveAreaAs),
                (
                    egui::KeyboardShortcut::new(command, egui::Key::S),
                    WorkbenchAction::SaveArea,
                ),
                (
                    egui::KeyboardShortcut::new(command, egui::Key::N),
                    WorkbenchAction::NewArea,
                ),
                (
                    egui::KeyboardShortcut::new(command, egui::Key::O),
                    WorkbenchAction::OpenArea,
                ),
            ]
            .into_iter()
            .find(|(keys, _)| i.consume_shortcut(keys))
            .map(|(_, action)| action)
        });
        if let Some(action) = pressed {
            self.request(action, actions);
        }
    }

    /// Asks for `action`, first asking about unsaved changes if it would
    /// lose them (a new or another area, another project, quitting).
    pub(crate) fn request(&mut self, action: WorkbenchAction, actions: &mut Vec<WorkbenchAction>) {
        if self.playing.is_some() && crate::play::waits_for_stop(&action) {
            // Refused, with a word in the Console, at the end of the frame.
            actions.push(action);
            return;
        }
        if self.playing.is_some() && action == WorkbenchAction::Quit {
            self.stop();
        }
        let loses_changes = matches!(
            action,
            WorkbenchAction::NewArea
                | WorkbenchAction::OpenArea
                | WorkbenchAction::OpenAreaFile(_)
                | WorkbenchAction::OpenProject
                | WorkbenchAction::NewProject
                | WorkbenchAction::OpenRecent(_)
                | WorkbenchAction::Quit
        );
        if loses_changes && self.document.is_dirty() {
            self.unsaved_prompt = Some(action);
        } else {
            if action == WorkbenchAction::Quit {
                self.may_close = true;
            }
            actions.push(action);
        }
    }

    /// The app called off a queued quit (the save before it was cancelled
    /// or failed): a later close asks about unsaved changes again.
    pub fn quit_cancelled(&mut self) {
        self.may_close = false;
    }

    /// Whether the window may close now. If the area has unsaved changes,
    /// asks first (and the app should cancel this close).
    pub fn close_requested(&mut self) -> bool {
        if self.playing.is_some() {
            self.stop();
        }
        if self.may_close || !self.document.is_dirty() {
            return true;
        }
        self.unsaved_prompt = Some(WorkbenchAction::Quit);
        false
    }

    /// The questions about unsaved changes and files changed on disk.
    fn prompts(&mut self, ui: &mut Ui, actions: &mut Vec<WorkbenchAction>) {
        if let Some(waiting) = self.unsaved_prompt.clone() {
            let name = self.document.name();
            let mut answer = None;
            egui::Window::new("Unsaved changes")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ui.ctx(), |ui| {
                    ui.label(format!("{name} has changes that aren't saved."));
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {
                            answer = Some(true);
                        }
                        if ui.button("Don't save").clicked() {
                            answer = Some(false);
                        }
                        if ui.button("Cancel").clicked() {
                            self.unsaved_prompt = None;
                        }
                    });
                });
            if let Some(save) = answer {
                self.unsaved_prompt = None;
                if save {
                    actions.push(WorkbenchAction::SaveArea);
                }
                if waiting == WorkbenchAction::Quit {
                    self.may_close = true;
                }
                actions.push(waiting);
            }
        }
        // Asked after the game stops: loading it now would only be undone.
        if self.pending_reload.is_some() && self.playing.is_none() {
            let name = self.document.name();
            let mut answer = None;
            egui::Window::new("Changed on disk")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ui.ctx(), |ui| {
                    ui.label(format!(
                        "{name}'s file changed on disk (perhaps a Git pull), and you have \
                         changes that aren't saved."
                    ));
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("Keep mine").clicked() {
                            answer = Some(false);
                        }
                        if ui
                            .button("Load the file")
                            .on_hover_text("Ctrl+Z brings yours back")
                            .clicked()
                        {
                            answer = Some(true);
                        }
                    });
                });
            if let Some(load) = answer
                && let Some(area) = self.pending_reload.take()
                && load
            {
                self.reload_area(area);
            }
        }
    }

    /// Delete deletes the selection, Ctrl+D duplicates it, F2 renames it
    /// and F frames it (decided with the project lead, 2026-10-23). Not
    /// while typing, nor while flying.
    pub(crate) fn editing_keys(&mut self, ui: &Ui) {
        if ui.ctx().egui_wants_keyboard_input() || self.viewport.is_flying() {
            return;
        }
        let (delete, duplicate, rename, frame) = ui.input_mut(|i| {
            let plain = |i: &egui::InputState, key| i.key_pressed(key) && i.modifiers.is_none();
            (
                plain(i, egui::Key::Delete),
                i.consume_shortcut(&egui::KeyboardShortcut::new(
                    egui::Modifiers::COMMAND,
                    egui::Key::D,
                )),
                plain(i, egui::Key::F2),
                plain(i, egui::Key::F),
            )
        });
        if delete {
            self.delete_selection();
        }
        if duplicate {
            self.duplicate_selection();
        }
        if rename
            && let Some(id) = self.document.selection.primary()
            && let Some(entity) = self.document.area().get(id)
        {
            crate::area_panel::begin_rename(ui.ctx(), id, &entity.name);
        }
        if frame {
            self.frame_selection();
        }
    }

    fn about(&mut self, ui: &mut Ui) {
        let mut open = self.about_open;
        egui::Window::new(format!("About {}", self.info.name))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ui.ctx(), |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading(&self.info.name);
                    ui.label(format!("Version {}", self.info.version));
                    ui.add_space(8.0);
                    ui.label("A free, open-source 3D game engine.");
                    ui.label("Free software under the MIT licence.");
                });
            });
        self.about_open = open;
    }
}

#[cfg(test)]
#[path = "workbench_tests.rs"]
mod tests;
