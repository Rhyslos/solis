//! The workbench's side of the Animator tab (Phase 9, D3b): opening an
//! animator file into it and closing it, making a new one, the preview,
//! saving each change and running it at once in the game
//! (`Workbench::save_file`), undo, and the selected entity's state while
//! playing. The open file follows its file on disk as every editor's does
//! (`own_files.rs`).

use crate::animator_editor::AnimatorEditor;
use crate::animator_view::AnimatorUi;
use crate::console::LogLevel;
use crate::layout;
use crate::own_files::OwnFile;
use crate::panel::Panel;
use crate::sprite_sheets::{AnimatorState, SpriteSheets};
use crate::workbench::Workbench;
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
            .open
            .animator_editor
            .as_ref()
            .is_some_and(|e| e.path == path)
        {
            layout::open_first_beside(&mut self.dock, Panel::Animator, Panel::Viewport);
            return;
        }
        self.open.animator_editor = Some(AnimatorEditor::new(path.to_owned(), file));
        layout::open_first_beside(&mut self.dock, Panel::Animator, Panel::Viewport);
    }

    /// Makes a new animator file in `folder` (inside the assets), with one
    /// state to start in, and opens it.
    pub(crate) fn new_animator(&mut self, folder: &str) {
        let taken = |name: &str| {
            self.viewport.sheets.animators.contains_key(name)
                || self.open.library.iter().any(|i| i.path == name)
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
        let text = animation::animator_file::write(&file);
        self.save_file(&path, OwnFile::Animator(Arc::new(file)), text);
        self.open_animator(&path);
    }

    /// Closes the animator file open, and its tab.
    pub(crate) fn close_animator(&mut self) {
        self.open.animator_editor = None;
        layout::close(&mut self.dock, Panel::Animator);
    }

    /// Moves the preview on by `seconds` (it always plays, decision 0057).
    pub(crate) fn play_animator(&mut self, seconds: f64) {
        let Some(editor) = &mut self.open.animator_editor else {
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
        let Some(editor) = &mut self.open.animator_editor else {
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
            self.save_animator();
        }
    }

    /// Saves the open animator file, and runs it at once for every entity
    /// using it (in the running game too: each carries on by name).
    pub(crate) fn save_animator(&mut self) {
        let Some(editor) = &self.open.animator_editor else {
            return;
        };
        let (path, text, file) = (editor.path.clone(), editor.text(), editor.file.clone());
        self.save_file(&path, OwnFile::Animator(Arc::new(file)), text);
    }

    /// While playing, the selected entity's animator state and values, if
    /// it runs the file open.
    pub(crate) fn live_animator(&self) -> Option<(String, Vec<(String, ParamValue)>)> {
        let editor = self.open.animator_editor.as_ref()?;
        let playing = self.open.playing.as_ref()?;
        let id = self.open.document.selection.primary()?;
        let animator = playing.world.get::<area::builtin::Animator>(id)?;
        let file = animator.animator?.asset;
        (self.viewport.sheets.paths.get(&file) == Some(&editor.path))
            .then(|| playing.world.animator_of(id))
            .flatten()
    }
}

#[cfg(test)]
#[path = "workbench_animator_tests.rs"]
mod tests;
