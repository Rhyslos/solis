//! The commands of the Level column's menus (its header's ⋯ and a row's
//! right-click menu, drawn in `level_panel.rs`): each returns the
//! [`MenuCommand`] picked, and [`Workbench::run`] carries it out. The
//! editor's commands (`shell/commands.rs`) run the same ones, so a thing
//! is done one way wherever it is asked for.

use crate::workbench::Workbench;

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
    MakeModule,
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
                    && let Some(entity) = self.open.document.level().get(id)
                {
                    crate::level_panel::begin_rename(ctx, id, &entity.name);
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
            MenuCommand::MakeModule => self.ask_make_module(),
            MenuCommand::ExpandAll => self.open.level_panel.collapsed.clear(),
            MenuCommand::CollapseAll => {
                let level = self.open.document.level();
                let parents: Vec<_> = level.iter().filter_map(|(_, e)| e.parent).collect();
                self.open.level_panel.collapsed.extend(parents);
            }
        }
    }

    /// Selects every entity in the level (Ctrl+A).
    pub(crate) fn select_all(&mut self) {
        // While a brush's insides are edited: every part of the mode.
        if let Some(edit) = self.viewport.brushing.edit.as_mut() {
            edit.pick_all();
            return;
        }
        let ids: Vec<_> = self
            .open
            .document
            .level()
            .iter()
            .map(|(id, _)| id)
            .collect();
        if let Some(&first) = ids.first() {
            let primary = self.open.document.selection.primary().unwrap_or(first);
            self.open.document.selection.select_all(ids, primary);
        }
    }
}
