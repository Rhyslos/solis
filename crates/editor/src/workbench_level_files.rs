//! The level's document and its file: a new level, opening one, the
//! document replaced, the file moved or saved, and reading it again
//! when it changed on disk.

use super::Workbench;
use crate::console::LogLevel;
use crate::document::Document;

impl Workbench {
    /// Starts a new, unsaved level (New level, after the level's tab). Doesn't ask about
    /// unsaved changes: asking comes before the action.
    pub fn new_level(&mut self) {
        self.leave_for(Document::default());
    }

    /// Shows `level`, read from `file`, in place of the open one.
    pub fn open_level(&mut self, level: level::Level, file: std::path::PathBuf) {
        self.leave_for(Document::opened(level, file));
        self.log(
            LogLevel::Info,
            format!("Opened {}.", self.open.document.name()),
        );
    }

    /// Leaves the open level for `document`. The questions about the old
    /// one go with it: answering them would put it back over the new one.
    /// Asking comes before this ([`Self::request`]).
    fn leave_for(&mut self, document: Document) {
        self.replace_document(document);
        self.open.pending_reload = None;
        self.open.play_changes = None;
    }

    /// Uses `document` in place of the open one, keeping the components
    /// registered so far (game code registers its own once, not once per
    /// level).
    pub(crate) fn replace_document(&mut self, mut document: Document) {
        document.registry = std::mem::take(&mut self.open.document.registry);
        self.open.document = document;
    }

    /// A file or folder was moved from `from` to `to` (full paths), or
    /// deleted (`to` is `None`). If it was the open level's file, or a
    /// folder holding it, the level follows it; returns its new file then
    /// (`None` inside: it was deleted). While playing, and while Keep or
    /// Undo all is asked, the level that Stop or Keep would put back
    /// follows too, so the move isn't undone by them.
    pub fn file_moved(
        &mut self,
        from: &std::path::Path,
        to: Option<&std::path::Path>,
    ) -> Option<Option<std::path::PathBuf>> {
        let file = self.open.document.file()?.to_path_buf();
        let rest = file.strip_prefix(from).ok()?;
        let moved = to.map(|to| {
            if rest.as_os_str().is_empty() {
                to.to_path_buf()
            } else {
                to.join(rest)
            }
        });
        self.open.document.file_moved(moved.clone());
        // Every other copy of the document that can become the open one
        // again is of the same file: the one kept at Play, which Stop
        // puts back, and the one Keep would put in its place. Left where
        // they were, the next save would make the old file again beside
        // the moved one, or a deleted file's level would count as saved.
        if let Some(playing) = &mut self.open.playing {
            playing.before.file_moved(moved.clone());
        }
        if let Some(changes) = &mut self.open.play_changes {
            changes.document.file_moved(moved.clone());
        }
        match &moved {
            Some(path) => self.log(
                LogLevel::Info,
                format!("The open area is now {}.", path.display()),
            ),
            None => self.log(
                LogLevel::Warning,
                "The open area's file was deleted; saving asks where to put it.",
            ),
        }
        Some(moved)
    }

    /// Notes that the level was saved to `file`.
    pub fn mark_saved(&mut self, file: std::path::PathBuf) {
        self.open.document.mark_saved(file);
        self.log(
            LogLevel::Info,
            format!("Saved {}.", self.open.document.name()),
        );
    }

    /// The open level's file changed on disk and now holds `level`: it's
    /// loaded as one undoable step, or, with unsaved changes, the user is
    /// asked which to keep.
    pub fn offer_reload(&mut self, level: level::Level) {
        if let Some(playing) = &mut self.open.playing {
            // The game's copy isn't the file's; offered again after Stop.
            playing.reload = Some(level);
            return;
        }
        if let Some(changes) = &mut self.open.play_changes {
            // Keep would put the level from before this change over it:
            // offered again once Keep or Undo all is answered.
            changes.reload = Some(level);
            return;
        }
        if self.open.document.is_dirty() {
            self.open.pending_reload = Some(level);
        } else {
            self.reload_level(level);
        }
    }

    pub(super) fn reload_level(&mut self, level: level::Level) {
        let file = self
            .open
            .document
            .file()
            .and_then(|f| f.file_name())
            .map_or_else(
                || self.open.document.name(),
                |n| n.to_string_lossy().into_owned(),
            );
        let label = format!("Reloaded {file} from disk");
        match self.open.document.reload(level, &label) {
            Ok(()) => self.log(LogLevel::Info, format!("{label}.")),
            Err(why) => self.log(LogLevel::Error, format!("Couldn't reload {file}: {why}.")),
        }
    }
}
