//! The workbench's side of the Clips and Frames tabs (Phase 9, D3):
//! opening a clips file into them and closing it, playing its clip,
//! saving each change and showing it at once on every sprite, and undo.

use crate::console::LogLevel;
use crate::frames_editor::FramesEditor;
use crate::frames_view::FramesUi;
use crate::layout;
use crate::panel::Panel;
use crate::sprite_sheets::{FramesState, SpriteSheets};
use crate::workbench::{Workbench, WorkbenchAction};
use std::collections::HashMap;
use std::sync::Arc;

/// What the tabs draw from, for the file open in `editor`.
pub(crate) fn frames_ui<'a>(
    editor: &'a mut FramesEditor,
    sheets: &'a SpriteSheets,
    pictures: &'a HashMap<String, egui::TextureHandle>,
    cue_sounds: &HashMap<String, String>,
) -> FramesUi<'a> {
    let sheet = editor.file.sheet.and_then(|id| sheets.get(id));
    let atlas = editor
        .file
        .sheet
        .and_then(|id| sheets.paths.get(&id))
        .and_then(|path| pictures.get(path));
    let notes = match sheets.frames.get(&editor.path) {
        Some(FramesState::Read(_, notes)) => notes.clone(),
        _ => Vec::new(),
    };
    FramesUi {
        sheets: aseprite_sheets(sheets),
        sounds: sound_names(cue_sounds),
        notes,
        sheet,
        atlas,
        editor,
    }
}

/// The project's Aseprite sheets, by name.
fn aseprite_sheets(sheets: &SpriteSheets) -> Vec<(String, reflect::AssetId)> {
    sheets
        .listed()
        .into_iter()
        .filter(|(_, id)| {
            sheets
                .paths
                .get(id)
                .is_some_and(|p| sheets.by_path.contains_key(p))
        })
        .collect()
}

/// The sound events by the shortest name a frame's event can use: the
/// file's name when no other has it, otherwise its path.
fn sound_names(cue_sounds: &HashMap<String, String>) -> Vec<String> {
    let mut names: Vec<String> = cue_sounds
        .iter()
        .filter(|(name, path)| {
            // A path is listed only when its file's name is taken.
            *name != *path || {
                let stem = path.rsplit_once('/').map_or(path.as_str(), |(_, n)| n);
                cue_sounds.get(stem) != Some(path)
            }
        })
        .map(|(name, _)| name.clone())
        .collect();
    names.sort();
    names.dedup();
    names
}

impl Workbench {
    /// Opens the clips file at `path` (inside the assets) in the Clips tab
    /// beside Properties and the Frames tab at the bottom.
    pub(crate) fn open_frames(&mut self, path: &str) {
        let sheets = Arc::clone(&self.viewport.sheets);
        let file = match sheets.frames.get(path) {
            Some(FramesState::Read(file, _)) => (**file).clone(),
            Some(FramesState::Failed(why)) => {
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
        let clips = file
            .sheet
            .and_then(|id| sheets.get(id))
            .map(|sheet| animation::ClipSet::of_sheet(sheet, Some(&file)))
            .unwrap_or_default();
        self.frames_editor = Some(FramesEditor::new(path.to_owned(), file, &clips));
        layout::open_beside(&mut self.dock, Panel::Clips, Panel::Properties);
        layout::open_beside(&mut self.dock, Panel::Frames, Panel::Library);
    }

    /// Makes a clips file beside the Aseprite file at `sheet_path` (or
    /// opens the one already there) for the sheet `sheet`.
    pub(crate) fn make_clips_file(&mut self, sheet_path: &str, sheet: reflect::AssetId) {
        let stem = sheet_path
            .rsplit_once('.')
            .map_or(sheet_path, |(stem, _)| stem);
        let path = format!("{stem}.{}", animation::frames_file::EXTENSION);
        if self.viewport.sheets.frames.contains_key(&path) {
            self.open_frames(&path);
            return;
        }
        let file = animation::FramesFile {
            sheet: Some(sheet),
            ..animation::FramesFile::default()
        };
        self.outbox.push(WorkbenchAction::SaveFrames {
            path: path.clone().into(),
            text: animation::frames_file::write(&file),
        });
        let sheets = Arc::make_mut(&mut self.viewport.sheets);
        sheets
            .frames
            .insert(path.clone(), FramesState::Read(Arc::new(file), Vec::new()));
        self.open_frames(&path);
    }

    /// Closes the clips file open, and its tabs.
    pub(crate) fn close_frames(&mut self) {
        self.frames_editor = None;
        self.frames_hovered = false;
        for panel in Panel::FRAMES_TABS {
            layout::close(&mut self.dock, panel);
        }
    }

    /// Moves the open clip on by `seconds` while it's playing in the
    /// Frames tab.
    pub(crate) fn play_frames(&mut self, seconds: f64) {
        let Some(editor) = &mut self.frames_editor else {
            return;
        };
        if !editor.playing {
            return;
        }
        let sheets = &self.viewport.sheets;
        let clip = editor
            .file
            .sheet
            .and_then(|id| sheets.get(id))
            .and_then(|sheet| editor.clips(sheet).get(&editor.clip).cloned());
        match clip {
            Some(clip) => editor.play_on(&clip, seconds),
            None => editor.playing = false,
        }
    }

    /// After the tabs are drawn: saves a change, closes the file if a tab
    /// was closed, and notes whether the pointer is over them.
    pub(crate) fn frames_tabs_done(
        &mut self,
        out: crate::panels::FramesOut,
        actions: &mut Vec<WorkbenchAction>,
    ) {
        self.frames_hovered = out.hovered;
        if out.changed {
            self.save_frames(actions);
        }
        // A tab closed in the dock: the other goes too.
        let one_gone = Panel::FRAMES_TABS
            .iter()
            .any(|p| self.dock.find_tab(p).is_none());
        if self.frames_editor.is_some() && (out.closed || one_gone) {
            self.close_frames();
        }
        self.animator_hovered = out.animator_hovered;
        if out.animator_changed {
            self.save_animator(actions);
        }
        if self.animator_editor.is_some()
            && (out.animator_closed || self.dock.find_tab(&Panel::Animator).is_none())
        {
            self.close_animator();
        }
    }

    /// Undoes (or redoes) the clips file's last change, from Ctrl+Z (or
    /// Ctrl+Y) over its tabs.
    pub(crate) fn frames_undo(&mut self, redo: bool, undo: bool) {
        let Some(editor) = &mut self.frames_editor else {
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
            self.save_frames(&mut actions);
            self.outbox.extend(actions);
        }
    }

    /// Saves the open clips file, and shows the change at once on every
    /// sprite playing it.
    fn save_frames(&mut self, actions: &mut Vec<WorkbenchAction>) {
        let Some(editor) = &self.frames_editor else {
            return;
        };
        actions.push(WorkbenchAction::SaveFrames {
            path: editor.path.clone().into(),
            text: editor.text(),
        });
        let (path, file) = (editor.path.clone(), editor.file.clone());
        let sheets = Arc::make_mut(&mut self.viewport.sheets);
        sheets
            .frames
            .insert(path, FramesState::Read(Arc::new(file), Vec::new()));
        sheets.work_out_clips();
        self.sources += 1;
    }
}

#[cfg(test)]
#[path = "workbench_frames_tests.rs"]
mod tests;
