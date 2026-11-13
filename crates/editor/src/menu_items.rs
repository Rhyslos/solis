//! The commands of the Area column's menus (its header's ⋯ and a row's
//! right-click menu, drawn in `area_panel.rs`): each returns the
//! [`MenuCommand`] picked, and [`Workbench::run`] carries it out. The
//! editor's commands (`shell/commands.rs`) run the same ones, so a thing
//! is done one way wherever it is asked for.

use crate::workbench::{Workbench, WorkbenchAction};

/// Something picked from a menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MenuCommand {
    ExportPlaytest,
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
    AddCamera,
    CreateAssembly,
    ExpandAll,
    CollapseAll,
}

impl Workbench {
    /// Carries out a menu command.
    pub(crate) fn run(&mut self, command: MenuCommand, ctx: &egui::Context) {
        match command {
            MenuCommand::ExportPlaytest => self.export_playtest(),
            MenuCommand::Undo => self.undo(),
            MenuCommand::Redo => self.redo(),
            MenuCommand::Duplicate => self.duplicate_selection(),
            MenuCommand::Delete => self.delete_selection(),
            MenuCommand::Rename => {
                if let Some(id) = self.open.document.selection.primary()
                    && let Some(entity) = self.open.document.area().get(id)
                {
                    crate::area_panel::begin_rename(ctx, id, &entity.name);
                }
            }
            MenuCommand::SelectAll => self.select_all(),
            MenuCommand::SelectNone => self.open.document.selection.clear(),
            MenuCommand::AddEntity => {
                self.add_entity();
            }
            MenuCommand::AddPointLight => {
                self.add_point_light();
            }
            MenuCommand::AddSpotlight => {
                self.add_spotlight();
            }
            MenuCommand::AddCamera => {
                self.add_camera();
            }
            MenuCommand::CreateAssembly => {
                if let Some(id) = self.open.document.selection.primary() {
                    self.open.outbox.push(WorkbenchAction::CreateAssembly(id));
                }
            }
            MenuCommand::ExpandAll => self.open.area_panel.collapsed.clear(),
            MenuCommand::CollapseAll => {
                let area = self.open.document.area();
                let parents: Vec<_> = area.iter().filter_map(|(_, e)| e.parent).collect();
                self.open.area_panel.collapsed.extend(parents);
            }
        }
    }

    /// Selects every entity in the area (Ctrl+A).
    pub(crate) fn select_all(&mut self) {
        let ids: Vec<_> = self.open.document.area().iter().map(|(id, _)| id).collect();
        if let Some(&first) = ids.first() {
            let primary = self.open.document.selection.primary().unwrap_or(first);
            self.open.document.selection.select_all(ids, primary);
        }
    }
}
