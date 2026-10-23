//! Menu items shared by the top bar and the panels' own menu rows, so the
//! Area panel's Edit and Entity menus are the same as the top bar's: each
//! draws its items and returns the [`MenuCommand`] picked, and
//! [`Workbench::run`] carries it out.

use crate::workbench::{Workbench, WorkbenchAction};
use egui::Ui;

/// Something picked from a menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MenuCommand {
    Undo,
    Redo,
    Duplicate,
    Delete,
    Rename,
    SelectAll,
    SelectNone,
    AddEntity,
    AddPointLight,
    AddSpotlight,
    CreateAssembly,
    ExpandAll,
    CollapseAll,
}

/// What the menus need to know to show their items enabled or not.
#[derive(Debug, Clone, Default)]
pub(crate) struct MenuState {
    /// "Undo Move", if there is something to undo.
    pub(crate) undo: Option<String>,
    /// "Redo Move", if there is something to redo.
    pub(crate) redo: Option<String>,
    /// Whether anything is selected.
    pub(crate) selection: bool,
}

impl MenuState {
    /// The workbench's state for its menus.
    pub(crate) fn of(workbench: &Workbench) -> Self {
        let history = workbench.document.history();
        Self {
            undo: history.undo_label().map(|l| format!("Undo {l}")),
            redo: history.redo_label().map(|l| format!("Redo {l}")),
            selection: workbench.document.selection.primary().is_some(),
        }
    }
}

/// One item: a button with its shortcut, closing the menu when clicked.
fn item(
    ui: &mut Ui,
    enabled: bool,
    text: &str,
    shortcut: &str,
    hint: &str,
    command: MenuCommand,
    picked: &mut Option<MenuCommand>,
) {
    let mut button = egui::Button::new(text);
    if !shortcut.is_empty() {
        button = button.shortcut_text(shortcut);
    }
    let mut response = ui.add_enabled(enabled, button);
    if !hint.is_empty() {
        response = response.on_hover_text(hint);
    }
    if response.clicked() {
        *picked = Some(command);
        ui.close();
    }
}

/// The Edit menu's items.
pub(crate) fn edit_items(ui: &mut Ui, state: &MenuState) -> Option<MenuCommand> {
    let mut picked = None;
    let p = &mut picked;
    item(
        ui,
        state.undo.is_some(),
        state.undo.as_deref().unwrap_or("Undo"),
        "Ctrl+Z",
        "",
        MenuCommand::Undo,
        p,
    );
    item(
        ui,
        state.redo.is_some(),
        state.redo.as_deref().unwrap_or("Redo"),
        "Ctrl+Y",
        "",
        MenuCommand::Redo,
        p,
    );
    ui.separator();
    let s = state.selection;
    item(ui, s, "Duplicate", "Ctrl+D", "", MenuCommand::Duplicate, p);
    item(ui, s, "Delete", "Delete", "", MenuCommand::Delete, p);
    item(ui, s, "Rename", "F2", "", MenuCommand::Rename, p);
    ui.separator();
    item(
        ui,
        true,
        "Select all",
        "Ctrl+A",
        "",
        MenuCommand::SelectAll,
        p,
    );
    item(
        ui,
        s,
        "Select nothing",
        "Esc",
        "",
        MenuCommand::SelectNone,
        p,
    );
    picked
}

/// The Entity menu's items.
pub(crate) fn entity_items(ui: &mut Ui, state: &MenuState) -> Option<MenuCommand> {
    let mut picked = None;
    let p = &mut picked;
    item(
        ui,
        true,
        "Add entity",
        "",
        "An empty entity, at the point the camera looks at",
        MenuCommand::AddEntity,
        p,
    );
    ui.separator();
    item(
        ui,
        true,
        "Add point light",
        "",
        "A bulb shining every way, placed above where the camera looks",
        MenuCommand::AddPointLight,
        p,
    );
    item(
        ui,
        true,
        "Add spotlight",
        "",
        "A cone of light at the camera, shining where it looks",
        MenuCommand::AddSpotlight,
        p,
    );
    ui.separator();
    item(
        ui,
        state.selection,
        "Create assembly from selection",
        "",
        "Saves the selected entity, and everything under it, as an assembly in the project, \
         and turns it into a copy of it",
        MenuCommand::CreateAssembly,
        p,
    );
    ui.separator();
    ui.add_enabled(false, egui::Button::new("Add camera"))
        .on_disabled_hover_text("Cameras arrive with play mode in Phase 3");
    picked
}

impl Workbench {
    /// Carries out a menu command.
    pub(crate) fn run(&mut self, command: MenuCommand, ctx: &egui::Context) {
        match command {
            MenuCommand::Undo => self.undo(),
            MenuCommand::Redo => self.redo(),
            MenuCommand::Duplicate => self.duplicate_selection(),
            MenuCommand::Delete => self.delete_selection(),
            MenuCommand::Rename => {
                if let Some(id) = self.document.selection.primary()
                    && let Some(entity) = self.document.area().get(id)
                {
                    crate::area_panel::begin_rename(ctx, id, &entity.name);
                }
            }
            MenuCommand::SelectAll => self.select_all(),
            MenuCommand::SelectNone => self.document.selection.clear(),
            MenuCommand::AddEntity => {
                self.add_entity();
            }
            MenuCommand::AddPointLight => {
                self.add_point_light();
            }
            MenuCommand::AddSpotlight => {
                self.add_spotlight();
            }
            MenuCommand::CreateAssembly => {
                if let Some(id) = self.document.selection.primary() {
                    self.outbox.push(WorkbenchAction::CreateAssembly(id));
                }
            }
            MenuCommand::ExpandAll => self.area_panel.collapsed.clear(),
            MenuCommand::CollapseAll => {
                let area = self.document.area();
                let parents: Vec<_> = area.iter().filter_map(|(_, e)| e.parent).collect();
                self.area_panel.collapsed.extend(parents);
            }
        }
    }

    /// Selects every entity in the area (Ctrl+A).
    pub(crate) fn select_all(&mut self) {
        let ids: Vec<_> = self.document.area().iter().map(|(id, _)| id).collect();
        if let Some(&first) = ids.first() {
            let primary = self.document.selection.primary().unwrap_or(first);
            self.document.selection.select_all(ids, primary);
        }
    }
}
