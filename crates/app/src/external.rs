//! Handing a file or folder to the computer's own programs.
//! [`open_externally`] is the one place the editor's files and folders
//! reach them, so the rule that protects users can't be skipped by a new
//! way of asking.
//!
//! A project can come from anyone. Only kinds of file whose program just
//! shows or edits them are ever opened in it
//! (`editor::LibraryKind::opens_in_its_program`, asked about the kind the
//! file's own name gives); any other file is shown in its folder, never
//! opened, so a `.lua` file can't be run outside the sandbox by an
//! interpreter the computer happens to have. And a path the editor asks
//! for, which a project's file may have named, is only looked at if it is
//! inside the open project's assets folder: never a network share, another
//! drive or a folder above the project.

use std::io;
use std::path::{Path, PathBuf};

/// What a caller wants from the computer's own programs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum External<'a> {
    /// A file or folder the editor asked for by its full path. It must be
    /// inside `assets`, the open project's assets folder (`None` with no
    /// project open). A folder is shown in the file browser; a file is
    /// opened in its program if its kind is safe to open, and shown in
    /// its folder if not.
    InProject {
        /// The file or folder.
        path: &'a Path,
        /// The open project's assets folder.
        assets: Option<&'a Path>,
    },
    /// A folder the user chose on this computer (a project in the
    /// launcher's list, an export's folder), shown in the file browser. A
    /// file is never opened.
    Folder(&'a Path),
}

/// What was handed to the computer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Handed {
    /// This file, to the program the computer has for it.
    Opened(PathBuf),
    /// This folder, to the file browser.
    Shown(PathBuf),
}

/// Does what `want` asks as far as the rule allows, and says what was
/// handed over; an error says in plain words why nothing was.
pub(crate) fn open_externally(want: External<'_>) -> io::Result<Handed> {
    let handed = settle(want)?;
    // A folder goes straight to Explorer on Windows, so it can never be
    // taken for a program to run (if it vanished and a file of a similar
    // name took its place, say).
    #[cfg(windows)]
    if let Handed::Shown(folder) = &handed {
        std::process::Command::new("explorer.exe")
            .arg(folder)
            .spawn()?;
        return Ok(handed);
    }
    let (Handed::Opened(path) | Handed::Shown(path)) = &handed;
    opener::open(path).map_err(io::Error::other)?;
    Ok(handed)
}

/// What `want` comes to: the file to open or the folder to show. Nothing
/// is started here.
fn settle(want: External<'_>) -> io::Result<Handed> {
    match want {
        External::Folder(folder) => {
            if std::fs::metadata(folder)?.is_dir() {
                Ok(Handed::Shown(folder.to_path_buf()))
            } else {
                Err(io::Error::other("it isn't a folder"))
            }
        }
        External::InProject { path, assets } => {
            // Checked before the path is looked at: even asking whether a
            // network share exists sends it the user's sign-in.
            let inside = inside(path, assets)
                .ok_or_else(|| io::Error::other("it isn't in the project's assets folder"))?;
            let meta = std::fs::metadata(path)?;
            if meta.is_dir() {
                Ok(Handed::Shown(path.to_path_buf()))
            } else if !meta.is_file() {
                Err(io::Error::other("it isn't a file or a folder"))
            } else if opens_in_its_program(inside) {
                Ok(Handed::Opened(path.to_path_buf()))
            } else {
                path.parent()
                    .map(|folder| Handed::Shown(folder.to_path_buf()))
                    .ok_or_else(|| io::Error::other("it has no folder to show"))
            }
        }
    }
}

/// `path` as a path inside `assets`, if that is where it is: the assets
/// folder itself, or plain names under it (`project::plain_inside`, the
/// rule the savers keep to as well).
fn inside<'a>(path: &'a Path, assets: Option<&Path>) -> Option<&'a Path> {
    let rest = path.strip_prefix(assets?).ok()?;
    (rest.as_os_str().is_empty() || crate::project::plain_inside(rest)).then_some(rest)
}

/// Whether the file at `inside` (a path inside the assets) may be opened
/// in the program the computer has for it. Its kind is read from its own
/// name here, never taken from whoever asked.
fn opens_in_its_program(inside: &Path) -> bool {
    let kind = crate::project::library_kind(asset_db::FileKind::of(inside));
    kind.opens_in_its_program(&crate::project::slashed(inside))
}

#[cfg(test)]
#[path = "external_tests.rs"]
mod tests;
