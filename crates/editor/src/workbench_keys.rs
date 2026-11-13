//! The editor's own keys, each acting only where the keys are its own
//! (`KeyOwner`, worked out once a frame): the tools, the Console, undo and
//! redo, the file actions and editing.

use super::{Workbench, WorkbenchAction};
use crate::key_owner::KeyOwner;
use crate::viewport::Tool;
use egui::Ui;

impl Workbench {
    /// Q, W, R, S and T pick the tools, where the keys are the area's.
    pub(crate) fn tool_keys(&mut self, ui: &Ui) {
        // While flying, W A S D, Q and E move the camera instead.
        if !self.keys.is_area() || self.viewport.is_flying() {
            return;
        }
        let pressed = ui.input(|i| {
            Tool::ALL
                .into_iter()
                .find(|tool| i.key_pressed(tool.key()) && i.modifiers.is_none())
        });
        if let Some(tool) = pressed {
            self.viewport.tool = tool;
            self.viewport.tile_overlay.paint.tool = None;
        }
    }

    /// The `'` key (beside Æ on a Norwegian keyboard) jumps to the
    /// console's command line, bringing the Console to the front (decided
    /// with the project lead, 2026-10-23). Not while typing.
    pub(crate) fn console_key(&mut self, ui: &Ui) {
        if !self.keys.takes_shortcuts() || self.viewport.is_flying() {
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

    /// Ctrl+Z undoes; Ctrl+Y or Ctrl+Shift+Z redoes: the undo steps of
    /// whoever has the keys. Not while typing in a field, where they undo
    /// the typing instead.
    pub(crate) fn undo_keys(&mut self, ui: &Ui) {
        if !self.keys.takes_shortcuts() {
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
        if !redo && !undo {
            return;
        }
        match self.keys {
            // Left before the keys were read.
            KeyOwner::Game
            | KeyOwner::Question
            | KeyOwner::Binding
            | KeyOwner::Field
            | KeyOwner::Page => {}
            KeyOwner::Area if redo => self.redo(),
            KeyOwner::Area => self.undo(),
            // Over the Map tab, or painting: the map's changes.
            KeyOwner::Map => self.map_undo(redo, undo),
            KeyOwner::Clips | KeyOwner::Frames => self.frames_undo(redo, undo),
            KeyOwner::Animator => self.animator_undo(redo, undo),
            KeyOwner::Tileset | KeyOwner::TilesetProperties => {
                if let Some(path) = self.open.tilesets.undone_by(self.keys) {
                    let path = path.to_owned();
                    self.tileset_undo(&path, redo, undo);
                }
            }
            KeyOwner::Atlas => self.atlas_undo(redo),
            KeyOwner::Canvas | KeyOwner::Elements => self.screen_undo(redo),
        }
    }

    /// Ctrl+N, Ctrl+O, Ctrl+S and Ctrl+Shift+S: new, open, save and save
    /// as (decided with the project lead, 2026-10-23). Not while typing.
    pub(crate) fn file_keys(&mut self, ui: &Ui, actions: &mut Vec<WorkbenchAction>) {
        if !self.keys.takes_shortcuts() {
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

    /// Delete deletes the selection, Ctrl+D duplicates it, F2 renames it
    /// and F frames it (decided with the project lead, 2026-10-23), where
    /// the keys are the area's: over a tab with a file of its own they are
    /// that tab's, and never reach the area as well. Not while flying.
    pub(crate) fn editing_keys(&mut self, ui: &Ui, actions: &mut Vec<WorkbenchAction>) {
        if self.viewport.is_flying() {
            return;
        }
        // Over the Library or the Assets column, F2 and Delete are the
        // picked file's: on the Library tab too, where the area's own
        // keys rest.
        let files = (self.keys.is_area() || self.keys == KeyOwner::Page)
            && self.file_keys_over_the_library(ui, actions);
        if !self.keys.is_area() {
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
        if delete && !files {
            self.delete_selection();
        }
        if duplicate {
            self.duplicate_selection();
        }
        if rename
            && !files
            && let Some(id) = self.open.document.selection.primary()
            && let Some(entity) = self.open.document.area().get(id)
        {
            crate::area_panel::begin_rename(ui.ctx(), id, &entity.name);
        }
        if frame {
            self.frame_selection();
        }
    }
}
