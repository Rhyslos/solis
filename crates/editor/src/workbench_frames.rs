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
use crate::workbench::Workbench;
use std::collections::HashMap;
use std::sync::Arc;

/// What the tabs draw from, for the file open in `editor`. The lists to
/// choose a sheet and a sound event from are made only when one is
/// opened (the second review pass: every frame sorted every picture's
/// name, on any tab).
pub(crate) fn frames_ui<'a>(
    editor: &'a mut FramesEditor,
    sheets: &'a SpriteSheets,
    pictures: &'a HashMap<String, egui::TextureHandle>,
    cue_sounds: &'a HashMap<String, String>,
) -> FramesUi<'a> {
    // A picture cut into cells shows its cells; an Aseprite file its
    // frames.
    let cut = (sheets.cut.get(&editor.path)).and_then(|c| c.sheet.as_ref().ok());
    let (sheet, atlas) = match cut {
        Some(cut) => (Some(cut), pictures.get(&editor.path)),
        None => (
            editor.file.sheet().and_then(|id| sheets.get(id)),
            (editor.file.sheet())
                .and_then(|id| sheets.paths.get(&id))
                .and_then(|path| pictures.get(path)),
        ),
    };
    let mut notes = match sheets.frames.get(&editor.path) {
        Some(FramesState::Read(_, notes)) => notes.clone(),
        _ => Vec::new(),
    };
    // What cutting its picture into cells left out, or why it couldn't.
    if let Some(cut) = sheets.cut.get(&editor.path) {
        notes.extend(
            cut.notes
                .iter()
                .map(|n| format!("Cutting the picture: {n}.")),
        );
        if let Err(why) = &cut.sheet {
            notes.push(format!("The picture couldn't be cut: {why}."));
        }
    }
    FramesUi {
        sheets,
        cue_sounds,
        notes,
        cut: sheets.cut.contains_key(&editor.path),
        sheet,
        atlas,
        editor,
    }
}

/// The project's Aseprite sheets, by name.
pub(crate) fn aseprite_sheets(sheets: &SpriteSheets) -> Vec<(String, reflect::AssetId)> {
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
pub(crate) fn sound_names(cue_sounds: &HashMap<String, String>) -> Vec<String> {
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
                .sheet()
                .and_then(|id| sheets.get(id))
                .map(|sheet| animation::ClipSet::of_sheet(sheet, Some(&file)))
                .unwrap_or_default();
            let picture = file.sheet().and_then(|id| sheets.paths.get(&id)).cloned();
            let mut editor = FramesEditor::new(path.to_owned(), file, &clips);
            if let Some(picture) = picture {
                editor.picture = picture.rsplit('/').next().unwrap_or_default().to_owned();
            }
            self.open.frames_editor = Some(editor);
        }
        self.show_panel(Panel::Frames);
    }

    /// Makes a clips file beside the Aseprite file at `sheet_path` (or
    /// opens the one already there) for the sheet `sheet`.
    pub(crate) fn make_clips_file(&mut self, sheet_path: &str, sheet: reflect::AssetId) {
        let path = animation::clips_file::file_for(sheet_path);
        if self.viewport.sheets.frames.contains_key(&path) {
            self.open_frames(&path);
            return;
        }
        let file = animation::ClipsFile::for_sheet(sheet);
        let picture = sheet_path.rsplit('/').next().unwrap_or_default();
        let text = animation::clips_file::write(&file, picture);
        self.save_file(&path, OwnFile::Frames(Arc::new(file)), text);
        self.open_frames(&path);
    }

    /// Opens Cut picture… on the clips file open, if its picture is a
    /// plain one (an Aseprite file's frames are its cells already).
    pub(crate) fn open_cut_picture(&mut self) {
        let Some(editor) = &self.open.frames_editor else {
            return;
        };
        let sheets = &self.viewport.sheets;
        let Some(picture) = editor.file.sheet().and_then(|id| sheets.paths.get(&id)) else {
            return;
        };
        if crate::sprite_sheets::is_plain_picture(picture) {
            self.open.cut_picture = Some(crate::cut_picture::CutPicture::new(
                editor.path.clone(),
                picture.clone(),
                editor.file.grid,
            ));
        }
    }

    /// Shows Cut picture… while it's open: Use these cells keeps its grid
    /// in the clips file, as one undo step, and saves it.
    pub(crate) fn cut_picture_dialog(&mut self, ctx: &egui::Context) {
        let Some(cut) = &mut self.open.cut_picture else {
            return;
        };
        let sheets = &self.viewport.sheets;
        let picture = sheets.at(&cut.picture).map(Arc::as_ref);
        if picture.is_none() {
            sheets.pictures_wanted.ask(&cut.picture);
        }
        let texture = self.open.sheet_pictures.get(&cut.picture);
        let shown = texture.zip(picture);
        let answer = crate::cut_picture::cut_dialog(ctx, cut, shown, self.accent.colors().fill);
        if let Some(answer) = answer {
            self.cut_picture_answered(answer);
        }
    }

    /// Closes Cut picture…: Use these cells keeps its grid in the clips
    /// file as one undo step, and saves it; Cancel changes nothing.
    pub(crate) fn cut_picture_answered(&mut self, answer: crate::cut_picture::CutAnswer) {
        let cut = self.open.cut_picture.take();
        if answer == crate::cut_picture::CutAnswer::Use
            && let Some(cut) = cut
            && let Some(editor) = &mut self.open.frames_editor
            && editor.path == cut.path
            && editor.change(|f| f.grid = cut.grid)
        {
            self.save_frames();
        }
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
            .sheet()
            .and_then(|id| sheets.get(id))
            .and_then(|sheet| editor.clips(sheet).get(&editor.clip).cloned());
        match clip {
            Some(clip) => editor.play_on(&clip, seconds),
            None => editor.playing = false,
        }
    }

    /// After the tabs are drawn: saves a change, and closes the file if a
    /// tab was closed.
    pub(crate) fn frames_tabs_done(&mut self, out: crate::panels::FramesOut) {
        if let Some(editor) = &mut self.open.frames_editor
            && std::mem::take(&mut editor.cut_asked)
        {
            self.open_cut_picture();
        }
        if let Some(frame) = out.frame_picked {
            self.profiler.picked = Some(frame);
        }
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
        if out.module_page_closed {
            self.close_module_page();
        }
        let table = out.table.clone();
        if let Some(said) = table.said {
            self.log(crate::console::LogLevel::Warning, said);
        }
        if table.changed {
            self.table_changed();
        }
        if table.import {
            self.ask_table_csv();
        }
        if table.export {
            self.export_table_csv();
        }
        if out.table_closed {
            self.close_table();
        }
        self.translations_asked(out.translations.clone());
        if out.translations_closed {
            self.open.translations_tab = None;
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
