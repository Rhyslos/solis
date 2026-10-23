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
#[derive(Debug, Clone, PartialEq, Eq)]
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
            self.document = Document::default();
            self.models.clear();
            self.library.clear();
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

    /// The project's assets, for the Library.
    pub fn set_library(&mut self, items: Vec<LibraryItem>) {
        self.library = items;
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
        self.document = Document::default();
        self.pending_reload = None;
    }

    /// Shows `area`, read from `file`, in place of the open one.
    pub fn open_area(&mut self, area: area::Area, file: std::path::PathBuf) {
        self.document = Document::opened(area, file);
        self.pending_reload = None;
        self.log(LogLevel::Info, format!("Opened {}.", self.document.name()));
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

    /// The viewport camera, for saving.
    pub fn camera(&self) -> Camera {
        self.viewport.camera
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
        let mut actions = Vec::new();
        if ui.input(|i| i.pointer.any_pressed()) {
            self.gesture += 1;
        }
        self.tool_keys(ui);
        self.undo_keys(ui);
        self.editing_keys(ui);
        self.file_keys(ui, &mut actions);
        egui::Panel::top("top_bar")
            .frame(theme::top_bar_frame())
            .exact_size(34.0)
            .show(ui, |ui| self.top_bar(ui, &mut actions));
        let style = theme::dock_style(ui.style().as_ref(), self.accent);
        let mut shown = HashSet::new();
        let mut area_requests = Vec::new();
        let mut place = None;
        let mut requests = Vec::new();
        self.viewport.area = AreaView::of(self.document.area(), &self.models);
        self.viewport.entities = self.document.area().len();
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
                        viewport: &mut self.viewport,
                        renderer,
                        console: &mut self.console,
                        document: &self.document,
                        models: &self.models,
                        area_requests: &mut area_requests,
                        project: self.project.as_ref(),
                        library: &self.library,
                        place: &mut place,
                        requests: &mut requests,
                        accent: self.accent,
                        in_front: &self.in_front,
                        shown: &mut shown,
                    },
                );
        });
        self.in_front = shown;
        for request in requests {
            self.apply_property(request, self.gesture);
        }
        if let Some(id) = place {
            actions.push(WorkbenchAction::PlaceAsset(id));
        }
        for request in area_requests {
            self.apply_area_request(request);
        }
        if self.files_hovering {
            drop_hint(ui, self.accent);
        }
        self.about(ui);
        self.prompts(ui, &mut actions);
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
        let loses_changes = matches!(
            action,
            WorkbenchAction::NewArea
                | WorkbenchAction::OpenArea
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

    /// Whether the window may close now. If the area has unsaved changes,
    /// asks first (and the app should cancel this close).
    pub fn close_requested(&mut self) -> bool {
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
        if self.pending_reload.is_some() {
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
