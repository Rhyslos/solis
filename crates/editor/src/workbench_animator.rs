//! The workbench's side of the Animator tab (Phase 9, D3b): opening an
//! animator file into it and closing it, making a new one, the preview,
//! saving each change and running it at once in the game, undo, the
//! selected entity's state while playing, and a file changed on disk.

use crate::animator_editor::AnimatorEditor;
use crate::animator_view::AnimatorUi;
use crate::console::LogLevel;
use crate::layout;
use crate::panel::Panel;
use crate::sprite_sheets::{AnimatorState, SpriteSheets};
use crate::workbench::{Workbench, WorkbenchAction};
use animation::ParamValue;
use std::collections::HashMap;
use std::sync::Arc;

/// What the tab and its Properties draw from, for the file in `editor`.
pub(crate) fn animator_ui<'a>(
    editor: &'a mut AnimatorEditor,
    sheets: &'a SpriteSheets,
    pictures: &'a HashMap<String, egui::TextureHandle>,
    live: Option<(String, Vec<(String, ParamValue)>)>,
    accent: egui::Color32,
) -> AnimatorUi<'a> {
    let mut notes = match sheets.animators.get(&editor.path) {
        Some(AnimatorState::Read(_, notes)) => notes.clone(),
        _ => Vec::new(),
    };
    notes.extend(editor.running.notes.iter().cloned());
    AnimatorUi {
        editor,
        sheets,
        pictures,
        live,
        notes,
        accent,
    }
}

impl Workbench {
    /// Opens the animator file at `path` (inside the assets) in the
    /// Animator tab, beside the viewport, in front.
    pub(crate) fn open_animator(&mut self, path: &str) {
        let file = match self.viewport.sheets.animators.get(path) {
            Some(AnimatorState::Read(file, _)) => (**file).clone(),
            Some(AnimatorState::Failed(why)) => {
                self.log(
                    LogLevel::Error,
                    format!("Couldn't open {path}: {why}. Fix it in a text editor, or delete it."),
                );
                return;
            }
            None => {
                self.log(
                    LogLevel::Warning,
                    format!("{path} is still being read; open it again in a moment."),
                );
                return;
            }
        };
        if self
            .animator_editor
            .as_ref()
            .is_some_and(|e| e.path == path)
        {
            layout::open_first_beside(&mut self.dock, Panel::Animator, Panel::Viewport);
            return;
        }
        self.animator_editor = Some(AnimatorEditor::new(path.to_owned(), file));
        layout::open_first_beside(&mut self.dock, Panel::Animator, Panel::Viewport);
    }

    /// Makes a new animator file in `folder` (inside the assets), with one
    /// state to start in, and opens it.
    pub(crate) fn new_animator(&mut self, folder: &str) {
        let taken = |name: &str| {
            self.viewport.sheets.animators.contains_key(name)
                || self.library.iter().any(|i| i.path == name)
        };
        let base = if folder.is_empty() {
            String::new()
        } else {
            format!("{folder}/")
        };
        let ext = animation::animator_file::EXTENSION;
        let path = std::iter::once(format!("{base}animator.{ext}"))
            .chain((2..).map(|n| format!("{base}animator {n}.{ext}")))
            .find(|p| !taken(p))
            .unwrap_or_default();
        let id = reflect::Id::random();
        let file = animation::AnimatorFile {
            entry: Some(id),
            states: vec![animation::State {
                id,
                name: "Idle".into(),
                place: [0.0, 0.0],
                plays: animation::Plays::Clip(String::new()),
                speed: 1.0,
                speed_by: String::new(),
            }],
            ..animation::AnimatorFile::default()
        };
        self.outbox.push(WorkbenchAction::SaveAnimator {
            path: path.clone().into(),
            text: animation::animator_file::write(&file),
        });
        let sheets = Arc::make_mut(&mut self.viewport.sheets);
        sheets.animators.insert(
            path.clone(),
            AnimatorState::Read(Arc::new(file), Vec::new()),
        );
        sheets.work_out_animators();
        self.open_animator(&path);
        if let Some(editor) = &mut self.animator_editor {
            editor.saving();
        }
    }

    /// Closes the animator file open, and its tab.
    pub(crate) fn close_animator(&mut self) {
        self.animator_editor = None;
        self.animator_hovered = false;
        layout::close(&mut self.dock, Panel::Animator);
    }

    /// Moves the preview on by `seconds` (it always plays, decision 0057).
    pub(crate) fn play_animator(&mut self, seconds: f64) {
        let Some(editor) = &mut self.animator_editor else {
            return;
        };
        let clips = editor
            .file
            .sheet
            .and_then(|id| self.viewport.sheets.clips(id))
            .cloned();
        editor.play_preview(seconds, clips.as_deref());
    }

    /// Undoes (or redoes) the animator's last change, from Ctrl+Z (or
    /// Ctrl+Y) over its tab.
    pub(crate) fn animator_undo(&mut self, redo: bool, undo: bool) {
        let Some(editor) = &mut self.animator_editor else {
            return;
        };
        let changed = if redo {
            editor.redo()
        } else if undo {
            editor.undo()
        } else {
            false
        };
        if changed {
            let mut actions = Vec::new();
            self.save_animator(&mut actions);
            self.outbox.extend(actions);
        }
    }

    /// Saves the open animator file, and runs it at once for every entity
    /// using it (in the running game too: each carries on by name).
    pub(crate) fn save_animator(&mut self, actions: &mut Vec<WorkbenchAction>) {
        let Some(editor) = &mut self.animator_editor else {
            return;
        };
        editor.saving();
        actions.push(WorkbenchAction::SaveAnimator {
            path: editor.path.clone().into(),
            text: editor.text(),
        });
        let (path, file) = (editor.path.clone(), editor.file.clone());
        let sheets = Arc::make_mut(&mut self.viewport.sheets);
        sheets
            .animators
            .insert(path, AnimatorState::Read(Arc::new(file), Vec::new()));
        sheets.work_out_animators();
        self.sources += 1;
    }

    /// While playing, the selected entity's animator state and values, if
    /// it runs the file open.
    pub(crate) fn live_animator(&self) -> Option<(String, Vec<(String, ParamValue)>)> {
        let editor = self.animator_editor.as_ref()?;
        let playing = self.playing.as_ref()?;
        let id = self.document.selection.primary()?;
        let animator = playing.world.get::<area::builtin::Animator>(id)?;
        let file = animator.animator?.asset;
        (self.viewport.sheets.paths.get(&file) == Some(&editor.path))
            .then(|| playing.world.animator_of(id))
            .flatten()
    }

    /// After the app reads the project's animators again: an open file
    /// changed on disk (by hand, or by Git) is taken in, as one undo step.
    pub(crate) fn animator_read_again(&mut self) {
        let Some(editor) = &mut self.animator_editor else {
            return;
        };
        match self.viewport.sheets.animators.get(&editor.path) {
            Some(AnimatorState::Read(file, _)) if editor.is_own(file) => editor.on_disk = true,
            Some(AnimatorState::Read(file, _)) => {
                editor.on_disk = true;
                editor.reload((**file).clone());
                let path = editor.path.clone();
                self.log(
                    LogLevel::Info,
                    format!("{path} changed on disk, so the Animator tab shows it as it is now (Ctrl+Z over the tab undoes that)."),
                );
            }
            None if editor.on_disk => {
                let path = editor.path.clone();
                self.close_animator();
                self.log(
                    LogLevel::Warning,
                    format!("{path} is gone, so the Animator tab closed."),
                );
            }
            _ => {}
        }
    }
}

#[cfg(test)]
#[path = "workbench_animator_tests.rs"]
mod tests;
