//! Modules' files on disk (`chicken.module.lst`; decisions 0081 and
//! 0118): reading one (or an assembly's older `.solassembly`), writing
//! one, and saving one through the backups, so an older file is kept
//! before it changes (rule 23).

use level::text::LevelFile;
use std::path::{Path, PathBuf};

/// Reads the module at `path`, within the reader's limits (the modules it
/// places not yet expanded), with a note for the Console when it was
/// saved by an older version.
pub(crate) fn read(
    path: &Path,
    registry: &reflect::Registry,
) -> Result<(LevelFile, Option<String>), String> {
    let bytes =
        asset_db::read_limited(path, level::module_file::MAX_BYTES as u64).map_err(|e| {
            if e.kind() == std::io::ErrorKind::FileTooLarge {
                format!(
                    "it's bigger than {} MB",
                    level::module_file::MAX_BYTES >> 20
                )
            } else {
                e.to_string()
            }
        })?;
    let text = String::from_utf8(bytes).map_err(|_| "it isn't text".to_string())?;
    let file = level::module_file::read(&text, registry).map_err(|e| e.to_string())?;
    let name = path
        .file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    let note = crate::upgrades::older_note(&name, &text, &level::module_file::FORMAT);
    Ok((file, note))
}

/// `file` as a module's text.
pub(crate) fn text(file: &LevelFile, registry: &reflect::Registry) -> Result<String, String> {
    level::module_file::write(file, registry).map_err(|e| e.to_string())
}

/// Saves `file` as the module at `path`: inside the project at `root`
/// through the backups (an older file there, a `.solassembly` too, is
/// copied first), and only when what's written reads back as itself.
/// Returns where a copy went, if one was made.
pub(crate) fn save(
    root: Option<&Path>,
    path: &Path,
    file: &LevelFile,
    registry: &reflect::Registry,
) -> Result<Option<PathBuf>, String> {
    let written = text(file, registry)?;
    let read_back = |new: &str| {
        let again = level::module_file::read(new, registry).map_err(|e| e.to_string())?;
        text(&again, registry)
    };
    match root {
        Some(root) => {
            crate::upgrades::save(root, path, &written, &level::module_file::FORMAT, read_back)
        }
        None => asset_db::write_atomically(path, written.as_bytes())
            .map(|()| None)
            .map_err(|e| e.to_string()),
    }
}

/// A free name for a new module called `name` in `folder`:
/// `chicken.module.lst`, then `chicken 2.module.lst` and on. `name` comes
/// from an entity's name, which a level can set to anything, so it's made
/// a name that works on every system first.
pub(crate) fn free_path(folder: &Path, name: &str) -> PathBuf {
    let safe = asset_db::file_name_from(name, "Module");
    let at = |stem: &str| folder.join(format!("{stem}{}", asset_db::MODULE_ENDING));
    let mut path = at(&safe);
    let mut n = 2;
    while std::fs::symlink_metadata(&path).is_ok() {
        path = at(&format!("{safe} {n}"));
        n += 1;
    }
    path
}

/// An assembly's older file as a module's (for `older_files`): read as it
/// always was, written in Lust, and checked to read as the same module.
pub(crate) fn from_old(text: &str) -> Result<String, String> {
    let registry = level::builtin::registry().map_err(|e| e.to_string())?;
    if !level::module_file::is_old_layout(text) {
        // Already a module's: given back as it is, so it reads the same.
        level::module_file::read(text, &registry).map_err(|e| e.to_string())?;
        return Ok(text.to_string());
    }
    let old = level::module_file::read(text, &registry).map_err(|e| e.to_string())?;
    let new = level::module_file::write(&old, &registry).map_err(|e| e.to_string())?;
    let again = level::module_file::read(&new, &registry).map_err(|e| e.to_string())?;
    if again != old {
        return Err("as a module it would read differently".into());
    }
    Ok(new)
}

#[cfg(test)]
#[path = "module_files_tests.rs"]
mod tests;
