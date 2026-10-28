//! Older files, as the editor meets them (Phase 8, C1;
//! [decision 0053](../../../docs/decisions/0053-compatibility-before-2d.md)).
//!
//! Every reader brings an older file up to date in memory
//! (`reflect::upgrade`); this says so in the Console when one is opened,
//! and saves through `asset_db::backup`, which keeps a copy of the older
//! file first.

use reflect::upgrade::Versioned;
use std::path::{Path, PathBuf};

/// A note for the Console when `text`, read from `file`, was written by an
/// older Solis: it's been upgraded as it was read, and saving it keeps a
/// copy of the old one first. `None` when it's current (or not this
/// format at all: the reader says why).
pub(crate) fn older_note(file: &str, text: &str, format: &dyn Versioned) -> Option<String> {
    let found = format.version_of(text)?;
    (found < format.current()).then(|| {
        format!(
            "{file} was saved by an older version (format {found}), so it's been upgraded to \
             format {} as it was read. The file itself is unchanged until it's saved, and then \
             a copy of the old one is kept first, in {}/{}.",
            format.current(),
            asset_db::CACHE_FOLDER,
            asset_db::backup::BACKUPS_FOLDER,
        )
    })
}

/// Saves `text` as `path` in the project at `root`, keeping a copy of the
/// file there first when it's in an older format (or newer, or damaged).
/// `read_back` reads the new text and writes it again: the save goes ahead
/// only when that gives the same text, so an upgrade never loses anything
/// it can't write. Returns where the copy went.
pub(crate) fn save(
    root: &Path,
    path: &Path,
    text: &str,
    format: &dyn Versioned,
    read_back: impl FnOnce(&str) -> Result<String, String>,
) -> Result<Option<PathBuf>, String> {
    asset_db::backup::save(root, path, text.as_bytes(), format, |new| {
        let again = read_back(new)?;
        if again == new {
            Ok(())
        } else {
            Err("it read back differently".into())
        }
    })
    .map_err(|e| e.to_string())
}

/// A note for the Console after a copy of an older file was kept.
pub(crate) fn kept_note(root: &Path, file: &str, copy: &Path) -> String {
    let shown = copy.strip_prefix(root).unwrap_or(copy);
    format!(
        "{file} was saved in the current format. The older file was kept first as {}, in \
         case it's ever needed.",
        shown.display()
    )
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Every file format a project holds, for the notes and the tests that
    /// open the example files of every version (`tests/fixtures/`).
    pub(crate) fn formats() -> [&'static dyn Versioned; 9] {
        [
            &area::text::FORMAT,
            &area::pixels::FORMAT,
            &animation::frames_file::FORMAT,
            &sound::event::FORMAT,
            &sound::mixer::FORMAT,
            &input::BINDINGS_FORMAT,
            &input::RECORDING_FORMAT,
            &asset_db::PROJECT_FORMAT,
            &asset_db::SIDECAR_FORMAT,
        ]
    }

    #[test]
    fn every_format_s_upgraders_are_complete() {
        for format in formats() {
            assert!(format.current() >= 1, "{}", format.name());
        }
        assert!(area::text::FORMAT.is_complete());
        assert!(sound::event::FORMAT.is_complete());
        assert!(sound::mixer::FORMAT.is_complete());
        assert!(area::pixels::FORMAT.is_complete());
        assert!(animation::frames_file::FORMAT.is_complete());
        assert!(input::BINDINGS_FORMAT.is_complete());
        assert!(input::RECORDING_FORMAT.is_complete());
        assert!(asset_db::PROJECT_FORMAT.is_complete());
        assert!(asset_db::SIDECAR_FORMAT.is_complete());
    }

    #[test]
    fn an_older_file_is_noted_once_and_a_current_one_never() {
        let old = "[[action]]\nname = \"Jump\"\ninputs = [\"Space\"]\n";
        let note = older_note("input.solinput", old, &input::BINDINGS_FORMAT).unwrap();
        assert!(
            note.contains("format 1") && note.contains("format 2"),
            "{note}"
        );
        let current = input::Bindings::default().write();
        assert_eq!(
            older_note("input.solinput", &current, &input::BINDINGS_FORMAT),
            None
        );
        assert_eq!(
            older_note("x", "version = \"two\"\n", &input::BINDINGS_FORMAT),
            None
        );
    }
}
