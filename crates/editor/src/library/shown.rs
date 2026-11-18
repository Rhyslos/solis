//! What the browser shows of the project: the open folder's folders and
//! files, or every match of the search, through the kind filter. The
//! engine's own folder of game code descriptions (`solis/`, written by
//! Solis and written again whenever the project changes) is left out:
//! nothing in it is the person's to change.

use super::{LibraryFiles, LibraryItem, LibraryState, last_part, parent_of};

/// The folder the engine writes its game code descriptions in, at the top
/// of the project's files ([`level::lust_data::ENGINE_FOLDER`]).
const ENGINE_FOLDER: &str = level::lust_data::ENGINE_FOLDER;

/// One thing the browser shows: a folder, or a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Entry<'a> {
    /// A folder, by its path inside the project.
    Folder(&'a str),
    /// A file.
    File(&'a LibraryItem),
}

impl<'a> Entry<'a> {
    /// Its path inside the project.
    pub(crate) fn path(self) -> &'a str {
        match self {
            Self::Folder(path) => path,
            Self::File(item) => &item.path,
        }
    }

    /// The name shown under it: a folder's own, a file's without its
    /// ending (as Solis names files).
    pub(crate) fn name(self) -> &'a str {
        match self {
            Self::Folder(path) => last_part(path),
            Self::File(item) => &item.name,
        }
    }

    /// The file, if it is one.
    pub(crate) fn file(self) -> Option<&'a LibraryItem> {
        match self {
            Self::File(item) => Some(item),
            Self::Folder(_) => None,
        }
    }
}

/// Whether `path` is in the engine's own folder (or is it), which the
/// Library leaves out.
pub(crate) fn is_engine(path: &str) -> bool {
    path == ENGINE_FOLDER || path.starts_with(&format!("{ENGINE_FOLDER}/"))
}

/// The folders directly inside `folder` ("" for the project's own), in
/// the order `folders` has them (sorted), the engine's left out.
pub(crate) fn folders_in<'a>(folders: &'a [String], folder: &str) -> Vec<&'a str> {
    folders
        .iter()
        .map(String::as_str)
        .filter(|f| parent_of(f) == folder && !is_engine(f))
        .collect()
}

/// What the browser shows: while searching, every folder and file whose
/// name has the words (folders only with no kind chosen); otherwise the
/// open folder's folders, then its files. Files pass the kind filter.
pub(crate) fn shown<'a>(
    items: &'a LibraryFiles,
    folders: &'a [String],
    state: &LibraryState,
) -> Vec<Entry<'a>> {
    let search = state.search.trim().to_lowercase();
    let mut entries: Vec<Entry<'a>> = if search.is_empty() {
        folders_in(folders, &state.folder)
            .into_iter()
            .map(Entry::Folder)
            .collect()
    } else if state.filter == super::Filter::All {
        folders
            .iter()
            .filter(|f| !is_engine(f) && last_part(f).to_lowercase().contains(&search))
            .map(|f| Entry::Folder(f.as_str()))
            .collect()
    } else {
        Vec::new()
    };
    entries.extend(
        items
            .with_lowered()
            .filter(|(item, _)| state.filter.passes(item.kind) && !is_engine(&item.path))
            .filter(|(item, lowered)| {
                if search.is_empty() {
                    item.folder() == state.folder
                } else {
                    lowered.contains(&search)
                }
            })
            .map(|(item, _)| Entry::File(item)),
    );
    entries
}

/// A folder's name as the Library shows it: the project's name for its
/// own folder.
pub(crate) fn folder_name<'a>(folder: &'a str, project: &'a str) -> &'a str {
    if folder.is_empty() {
        project
    } else {
        last_part(folder)
    }
}
