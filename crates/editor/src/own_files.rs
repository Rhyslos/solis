//! Files of their own that the editor changes and saves at once (a clips
//! file, an animator, a tileset, an atlas, a tile map), and the one way
//! each change reaches its file and each file reaches its editor (the
//! review, BUG-09).
//!
//! The app's store of the project's files is the one owner of what each
//! file holds: what it last read from disk, or what it last saved. The
//! editor keeps no file beside that. A change is shown at once and handed
//! to the app as a save ([`WorkbenchAction::SaveFile`]), and until the app
//! has answered ([`Workbench::file_saved`], [`Workbench::file_not_saved`])
//! the editor's value is the newer one, laid over whatever the app hands
//! it meanwhile ([`Unanswered`]): an unrelated read can't bring an older
//! copy back, and the editor never takes its own save for a change made on
//! disk. Once every save of a file is answered the file is what the app's
//! store says. That is the value saved, or, when the save failed, the
//! file as it still is: an editor that says every change is saved at once
//! must never show one that isn't. The change that wasn't saved is kept as
//! one undo step, so Ctrl+Z brings it back and saves it again, and the
//! Console says so.
//!
//! An open file's editor follows its file the same way whichever kind it
//! is ([`OpenFile`], [`follow`]): a file that differs from what the editor
//! has is taken in as one undo step, and one that's gone closes its tabs.

use crate::console::LogLevel;
use crate::sprite_sheets::{
    AnimatorState, AtlasState, FramesState, MapFileState, SpriteSheets, TilesetState,
};
use crate::workbench::{Workbench, WorkbenchAction};
use std::collections::HashMap;
use std::sync::Arc;

/// A file of one of the kinds the editor changes itself and saves at
/// once, as it is: what a save hands the app, and what the app keeps as
/// the file once it's saved.
#[derive(Debug, Clone, PartialEq)]
pub enum OwnFile {
    /// A sprite's clips file (`.solframes`).
    Frames(Arc<animation::FramesFile>),
    /// An animator file (`.solanim`).
    Animator(Arc<animation::AnimatorFile>),
    /// A tileset (`.tileset.lst`).
    Tileset(Arc<tiles::Tileset>),
    /// An atlas file (`.atlas.lst`).
    Atlas(Arc<animation::atlas::Atlas>),
    /// A tile map file (`.tilemap.lst`).
    TileMap(Arc<tiles::TileMap>),
}

/// What the project's files hold at a path ([`SpriteSheets::held`]).
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Held {
    /// A file of the editor's own kinds, read.
    Read(OwnFile),
    /// A file that couldn't be read.
    Unreadable,
    /// No file of those kinds.
    Gone,
}

impl SpriteSheets {
    /// Makes `file` the file at `path` (inside the assets), as if just
    /// read with nothing to put right. This is how a file the editor
    /// changed gets here, in the editor's copy and in the app's. What's
    /// worked out from the files ([`Self::work_out_clips`]) isn't worked
    /// out again here.
    pub fn put(&mut self, path: &str, file: OwnFile) {
        let path = path.to_owned();
        let notes = Vec::new();
        match file {
            OwnFile::Frames(file) => {
                self.frames.insert(path, FramesState::Read(file, notes));
            }
            OwnFile::Animator(file) => {
                self.animators
                    .insert(path, AnimatorState::Read(file, notes));
            }
            OwnFile::Tileset(file) => {
                self.tilesets.insert(path, TilesetState::Read(file, notes));
            }
            OwnFile::Atlas(file) => {
                self.atlases.insert(path, AtlasState::Read(file, notes));
            }
            OwnFile::TileMap(file) => {
                self.tile_maps.insert(path, MapFileState::Read(file, notes));
            }
        }
    }

    /// What these files hold at `path`, of the kinds the editor changes
    /// itself (a path is of one kind: its ending says which).
    pub(crate) fn held(&self, path: &str) -> Held {
        let held = |file: Option<OwnFile>| file.map_or(Held::Unreadable, Held::Read);
        if let Some(state) = self.frames.get(path) {
            return held(match state {
                FramesState::Read(file, _) => Some(OwnFile::Frames(Arc::clone(file))),
                FramesState::Failed(_) => None,
            });
        }
        if let Some(state) = self.animators.get(path) {
            return held(match state {
                AnimatorState::Read(file, _) => Some(OwnFile::Animator(Arc::clone(file))),
                AnimatorState::Failed(_) => None,
            });
        }
        if let Some(state) = self.tilesets.get(path) {
            return held(match state {
                TilesetState::Read(file, _) => Some(OwnFile::Tileset(Arc::clone(file))),
                TilesetState::Failed(_) => None,
            });
        }
        if let Some(state) = self.atlases.get(path) {
            return held(match state {
                AtlasState::Read(file, _) => Some(OwnFile::Atlas(Arc::clone(file))),
                AtlasState::Failed(_) => None,
            });
        }
        match self.tile_maps.get(path) {
            Some(MapFileState::Read(file, _)) => Held::Read(OwnFile::TileMap(Arc::clone(file))),
            Some(MapFileState::Failed(_)) => Held::Unreadable,
            None => Held::Gone,
        }
    }
}

/// The saves the app hasn't answered yet, by path: how many of each file,
/// and the file as last sent. Until a file's saves are all answered, the
/// editor's value of it is newer than the app's.
#[derive(Debug, Default)]
pub(crate) struct Unanswered(HashMap<String, (usize, OwnFile)>);

impl Unanswered {
    /// Notes a save of `file` to `path`, sent to the app.
    fn sent(&mut self, path: &str, file: OwnFile) {
        let entry = self.0.entry(path.to_owned()).or_insert((0, file.clone()));
        entry.0 += 1;
        entry.1 = file;
    }

    /// Takes one answer for `path` (the app answers in the order sent).
    /// Gives the file as last sent once every save of it is answered.
    fn answered(&mut self, path: &str) -> Option<OwnFile> {
        let entry = self.0.get_mut(path)?;
        entry.0 = entry.0.saturating_sub(1);
        if entry.0 > 0 {
            return None;
        }
        self.0.remove(path).map(|(_, file)| file)
    }

    /// Lays each file waiting for an answer over `sheets`, the files as
    /// the app has them.
    fn lay_over(&self, sheets: &mut SpriteSheets) {
        for (path, (_, file)) in &self.0 {
            sheets.put(path, file.clone());
        }
    }
}

/// A file open in an editor of its own, which follows the file.
pub(crate) trait OpenFile {
    /// The file as its kind holds it.
    type File: PartialEq + Clone;
    /// Its path inside the assets.
    fn path(&self) -> &str;
    /// The file as the editor has it.
    fn file(&self) -> &Self::File;
    /// Takes in the file as it is now, as one undo step.
    fn take_in(&mut self, file: Self::File);
    /// This kind's file in `held`, if that's its kind.
    fn of(held: &OwnFile) -> Option<&Arc<Self::File>>;
}

/// What an open file's editor did when the project's files were looked
/// at again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Followed {
    /// Nothing: it has the file as it is (or the file can't be read, and
    /// it keeps what it has).
    Same,
    /// The file differed: taken in as one undo step.
    TookIn,
    /// The file isn't there any more.
    Gone,
}

/// Makes `editor` follow its file as `sheets` hold it. An editor has no
/// way to tell its own save from someone else's change, and needs none:
/// while a save of its own waits for an answer, `sheets` hold what it
/// sent ([`Unanswered`]).
pub(crate) fn follow<E: OpenFile>(editor: &mut E, sheets: &SpriteSheets) -> Followed {
    match sheets.held(editor.path()) {
        Held::Gone => Followed::Gone,
        Held::Unreadable => Followed::Same,
        Held::Read(held) => match E::of(&held) {
            Some(file) if **file != *editor.file() => {
                editor.take_in((**file).clone());
                Followed::TookIn
            }
            _ => Followed::Same,
        },
    }
}

/// What an open file's editor did, for the Console.
pub(crate) struct Told {
    /// The file.
    pub(crate) path: String,
    /// What its editor did.
    pub(crate) followed: Followed,
    /// Where it shows and is undone ("the Animator tab"). `None`: in no
    /// tab (a tileset changed from Properties), so only a save that
    /// failed is said.
    pub(crate) tabs: Option<&'static str>,
    /// Whether that's several tabs.
    pub(crate) many: bool,
}

impl Told {
    /// What the Console says of a file that changed on disk or is gone.
    fn changed_on_disk(&self) -> Option<(LogLevel, String)> {
        let (path, tabs) = (&self.path, self.tabs?);
        let (show, them) = if self.many {
            ("show", "them")
        } else {
            ("shows", "the tab")
        };
        match self.followed {
            Followed::Same => None,
            Followed::TookIn => Some((
                LogLevel::Info,
                format!(
                    "{path} changed on disk, so {tabs} {show} it as it is now (Ctrl+Z over \
                     {them} undoes that)."
                ),
            )),
            Followed::Gone => Some((
                LogLevel::Warning,
                format!("{path} is gone, so {tabs} closed."),
            )),
        }
    }

    /// What became of a change whose save failed, to say after why.
    fn not_saved(&self) -> Option<String> {
        let over = self.tabs.unwrap_or("its Properties");
        match self.followed {
            Followed::Same => None,
            Followed::TookIn => Some(took_back(over)),
            Followed::Gone => Some(match self.tabs {
                Some(tabs) => format!("{NOT_THERE}, so {tabs} closed."),
                None => format!("{NOT_THERE}."),
            }),
        }
    }
}

/// What the Console says of a file whose save failed and that isn't in
/// the project (a new file that couldn't be made).
const NOT_THERE: &str = "The file isn't there";

/// What the Console says became of a change that wasn't saved, undone
/// with Ctrl+Z over `over`.
fn took_back(over: &str) -> String {
    format!(
        "It's shown as the file has it, without your change: Ctrl+Z over {over} brings the \
         change back and saves it again."
    )
}

impl Workbench {
    /// Makes `file` (written as `text`) the file at `path` inside the
    /// assets: shown at once, and saved by the app. This is the one way a
    /// file of the editor's own kinds is saved.
    pub(crate) fn save_file(&mut self, path: &str, file: OwnFile, text: String) {
        self.open.unanswered.sent(path, file.clone());
        self.open.outbox.push(WorkbenchAction::SaveFile {
            path: path.into(),
            file: file.clone(),
            text,
        });
        self.show_file(path, file);
        self.sources += 1;
    }

    /// Shows `file` as the file at `path` at once, with what's worked out
    /// from its kind: a change being saved, or a stroke under way.
    pub(crate) fn show_file(&mut self, path: &str, file: OwnFile) {
        let sheets = Arc::make_mut(&mut self.viewport.sheets);
        match &file {
            OwnFile::Frames(_) => {
                sheets.put(path, file);
                sheets.work_out_clips();
            }
            OwnFile::Animator(_) => {
                sheets.put(path, file);
                sheets.work_out_animators();
            }
            OwnFile::TileMap(_) => {
                sheets.put(path, file);
                sheets.work_out_maps();
            }
            OwnFile::Tileset(_) | OwnFile::Atlas(_) => sheets.put(path, file),
        }
    }

    /// The app saved the file at `path` (inside the assets, `/` between
    /// folders) as a [`WorkbenchAction::SaveFile`] asked, and keeps what
    /// it saved as the file.
    pub fn file_saved(&mut self, path: &str) {
        self.open.unanswered.answered(path);
    }

    /// The app couldn't save the file at `path` as a
    /// [`WorkbenchAction::SaveFile`] asked, because of `why`; `sheets` are
    /// the project's files as the app has them. The Console says so, and
    /// unless a later save of the file is still on its way, the file is
    /// shown as it still is, with the change that wasn't saved kept as
    /// one undo step.
    pub fn file_not_saved(&mut self, path: &str, why: &str, sheets: SpriteSheets) {
        let Some(unsaved) = self.open.unanswered.answered(path) else {
            self.log(LogLevel::Error, format!("Couldn't save {path}: {why}."));
            return;
        };
        self.take_sheets(sheets);
        let mut what = None;
        for told in self.follow_files(Some(path)) {
            if told.path == path {
                what = told.not_saved();
            } else {
                self.tell(&told);
            }
        }
        // A tile map has no editor of its own: its changes are the Map
        // tab's undo steps.
        if let OwnFile::TileMap(unsaved) = unsaved {
            match self.map_file(path) {
                Some(held) if *held != *unsaved => {
                    self.open
                        .map_edits
                        .push_undo(path.to_owned(), (*unsaved).clone());
                    what = Some(took_back("the Map tab"));
                }
                Some(_) => {}
                None => what = Some(format!("{NOT_THERE}.")),
            }
        }
        let what = what.map_or_else(String::new, |what| format!(" {what}"));
        self.log(
            LogLevel::Error,
            format!("Couldn't save {path}: {why}.{what}"),
        );
    }

    /// The project's files as the app has them (Phase 9, D2): what it read
    /// from disk and what it saved for the editor. Open files follow them.
    pub fn set_sprite_sheets(&mut self, sheets: SpriteSheets) {
        self.take_sheets(sheets);
        for told in self.follow_files(None) {
            self.tell(&told);
        }
    }

    /// Says in the Console what an open file's editor did about a change
    /// on disk.
    fn tell(&mut self, told: &Told) {
        if let Some((level, text)) = told.changed_on_disk() {
            self.log(level, text);
        }
    }

    /// Takes `sheets` as the project's files, with what the editor has
    /// that's newer laid over them: saves the app hasn't answered, and a
    /// stroke being painted.
    fn take_sheets(&mut self, mut sheets: SpriteSheets) {
        self.open.unanswered.lay_over(&mut sheets);
        if let Some((path, map)) = self.open.map_edits.stroke.as_ref().map(|s| s.shown()) {
            sheets.put(path, OwnFile::TileMap(Arc::new(map.clone())));
        }
        // The clips each sheet and clips file offers, worked out once here
        // for drawing, Properties and the running game.
        sheets.work_out_clips();
        self.viewport.sheets = Arc::new(sheets);
        self.place_read_pictures();
    }

    /// Makes every open file's editor follow its file as the project's
    /// files now hold it, closing the tabs of one that's gone. Gives what
    /// each did, for the Console. `not_saved` is the file whose save just
    /// failed, if that's why.
    fn follow_files(&mut self, not_saved: Option<&str>) -> Vec<Told> {
        let sheets = Arc::clone(&self.viewport.sheets);
        let mut told = Vec::new();
        if let Some(editor) = &mut self.open.animator_editor {
            let followed = follow(editor, &sheets);
            told.push(Told {
                path: editor.path.clone(),
                followed,
                tabs: Some("the Animator tab"),
                many: false,
            });
            if followed == Followed::Gone {
                self.close_animator();
            }
        }
        if let Some(editor) = &mut self.open.atlas_editor {
            let followed = follow(editor, &sheets);
            told.push(Told {
                path: editor.path.clone(),
                followed,
                tabs: Some("the Atlas Crop tab"),
                many: false,
            });
            if followed == Followed::Gone {
                self.close_atlas();
            }
        }
        // The clips file follows only a save of its own that failed: it
        // doesn't follow its file on disk yet.
        if let Some(editor) = &mut self.open.frames_editor
            && not_saved == Some(editor.path.as_str())
        {
            let followed = follow(editor, &sheets);
            told.push(Told {
                path: editor.path.clone(),
                followed,
                tabs: Some("the Clips and Frames tabs"),
                many: true,
            });
            if followed == Followed::Gone {
                self.close_frames();
            }
        }
        told.extend(self.follow_tilesets(&sheets));
        told.retain(|told| told.followed != Followed::Same);
        told
    }
}

#[cfg(test)]
#[path = "own_files_tests.rs"]
mod tests;
