//! Which level a project opens on (Fixes 1, B4), and which one the
//! project check plays (Fixes 1, T2). A project that opened on an empty
//! "Untitled level" looked as if it had nothing in it: pressing Play
//! showed nothing.
//!
//! The editor opens, in order: the level last opened or saved in this
//! project on this computer (kept in its `.solis` folder, beside the last
//! export); else the level the game starts in when exported; else the
//! project's first level by name (its only one, if it has one). The check
//! plays the level the game starts in, the same on every computer, so the
//! level last used isn't asked.

use std::path::Path;

/// The file inside the project's `.solis` folder naming the level last
/// opened or saved.
pub const LAST_LEVEL: &str = "last level.txt";

/// The longest a remembered level's path is read.
const MAX_LAST_LEVEL_BYTES: u64 = 4096;

/// Every level in `listing`, as paths inside the project's files with `/`
/// between folders, sorted by name.
pub fn levels(listing: &asset_db::Listing) -> Vec<String> {
    let mut levels: Vec<String> = listing
        .files
        .iter()
        .filter(|f| f.kind == asset_db::FileKind::Level)
        .map(|f| crate::project::slashed(&f.path))
        .collect();
    levels.sort();
    levels
}

/// The level the game starts in: the one its last export started in, if
/// it's still there, else the first by name. None for a project without
/// levels.
pub fn start_level(root: &Path, levels: &[String]) -> Option<String> {
    crate::playtest::last_export(root)
        .map(|playtest| playtest.level)
        .filter(|level| levels.contains(level))
        .or_else(|| levels.first().cloned())
}

/// The level the editor opens the project at `root` on: the one last
/// used, if it's still there, else [`start_level`].
pub fn level_to_open(root: &Path, levels: &[String]) -> Option<String> {
    last_used(root)
        .filter(|level| levels.contains(level))
        .or_else(|| start_level(root, levels))
}

/// The level last opened or saved in the project at `root`, as written.
fn last_used(root: &Path) -> Option<String> {
    let path = root.join(asset_db::CACHE_FOLDER).join(LAST_LEVEL);
    let bytes = asset_db::read_limited(&path, MAX_LAST_LEVEL_BYTES).ok()?;
    let level = String::from_utf8(bytes).ok()?.trim().to_string();
    editor::export_dialog::usable_level(&level).then_some(level)
}

/// Remembers `level` (a file inside the project's files folder, `files`)
/// as the last used in the project at `root`. A level outside it is
/// passed by; a disk that refuses only means it isn't remembered.
pub fn remember(root: &Path, files: &Path, level: &Path) {
    let Ok(inside) = level.strip_prefix(files) else {
        return;
    };
    let level = crate::project::slashed(inside);
    if !editor::export_dialog::usable_level(&level) {
        return;
    }
    let _ = asset_db::own_folder(root, asset_db::CACHE_FOLDER)
        .and_then(|own| asset_db::write_atomically(&own.join(LAST_LEVEL), level.as_bytes()));
}

#[cfg(test)]
#[path = "first_level_tests.rs"]
mod tests;
