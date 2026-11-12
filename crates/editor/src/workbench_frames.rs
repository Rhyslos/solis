//! The workbench's side of the Clips and Frames tabs (Phase 9, D3):
//! opening a clips file into them and closing it, playing its clip,
//! saving each change and showing it at once on every sprite
//! (`Workbench::save_file`), and undo. The open file follows its file on
//! disk as every editor's does (`own_files.rs`).

use crate::console::LogLevel;
use crate::frames_editor::FramesEditor;
use crate::frames_view::FramesUi;
use crate::own_files::OwnFile;
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
        // The file that's open already keeps its editor, with its undo
        // steps and what's chosen: only its tabs come back.
        if self
            .open
            .frames_editor
            .as_ref()
            .is_none_or(|e| e.path != path)
        {
            let clips = file
                .sheet
                .and_then(|id| sheets.get(id))
                .map(|sheet| animation::ClipSet::of_sheet(sheet, Some(&file)))
                .unwrap_or_default();
            self.open.frames_editor = Some(FramesEditor::new(path.to_owned(), file, &clips));
        }
        self.show_panel(Panel::Frames);
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
        let text = animation::frames_file::write(&file);
        self.save_file(&path, OwnFile::Frames(Arc::new(file)), text);
        self.open_frames(&path);
    }

    /// Closes the clips file open, and its tabs.
    pub(crate) fn close_frames(&mut self) {
        self.open.frames_editor = None;
    }

    /// Moves the open clip on by `seconds` while it's playing in the
    /// Frames tab.
    pub(crate) fn play_frames(&mut self, seconds: f64) {
        let Some(editor) = &mut self.open.frames_editor else {
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

    /// After the tabs are drawn: saves a change, and closes the file if a
    /// tab was closed.
    pub(crate) fn frames_tabs_done(
        &mut self,
        out: crate::panels::FramesOut,
        actions: &mut Vec<WorkbenchAction>,
    ) {
        if out.changed {
            self.save_frames();
        }
        if self.open.frames_editor.is_some() && out.closed {
            self.close_frames();
        }
        if let Some(said) = &out.screen_said {
            self.log(crate::console::LogLevel::Info, said.clone());
        }
        // Saved before it's closed: a change made on the frame its tab
        // closes isn't lost.
        if out.screen_changed {
            self.save_screen();
        }
        if out.screen_code
            && let Some(doc) = &self.open.screen_doc
        {
            let path = doc.path.clone();
            self.library_request(
                crate::library::LibraryRequest::Open(crate::library::LibraryKind::Code, path),
                actions,
            );
        }
        // The canvas's × closes the screen.
        if out.screen_closed {
            self.close_screen();
        }
        if out.atlas_changed {
            self.save_open_atlas();
        }
        if let Some(said) = &out.atlas_said {
            self.log(crate::console::LogLevel::Info, said.clone());
        }
        if self.open.atlas_editor.is_some() && out.atlas_closed {
            self.close_atlas();
        }
        self.open.tilesets.in_properties = out.properties_tileset;
        if out.tileset_changed {
            self.save_open_tileset();
        }
        if let Some(kind) = out.tileset_template {
            self.save_template(kind);
        }
        if self.open.tilesets.tab().is_some() && out.tileset_closed {
            self.close_tileset();
        }
        if out.animator_changed {
            self.save_animator();
        }
        if self.open.animator_editor.is_some() && out.animator_closed {
            self.close_animator();
        }
    }

    /// Undoes (or redoes) the clips file's last change, from Ctrl+Z (or
    /// Ctrl+Y) over its tabs.
    pub(crate) fn frames_undo(&mut self, redo: bool, undo: bool) {
        let Some(editor) = &mut self.open.frames_editor else {
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
            self.save_frames();
        }
    }

    /// Saves the open clips file, and shows the change at once on every
    /// sprite playing it.
    pub(crate) fn save_frames(&mut self) {
        let Some(editor) = &self.open.frames_editor else {
            return;
        };
        let (path, text, file) = (editor.path.clone(), editor.text(), editor.file.clone());
        self.save_file(&path, OwnFile::Frames(Arc::new(file)), text);
    }
}

#[cfg(test)]
#[path = "workbench_frames_tests.rs"]
pub(crate) mod tests;
