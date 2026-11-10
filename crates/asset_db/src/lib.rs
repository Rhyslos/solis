//! The project folder and its assets
//! ([decision 0021](../../../docs/decisions/0021-asset-database-and-models.md)):
//! `project.solis`, the `assets/` folder with a `.solmeta` sidecar beside
//! each asset holding its stable ID, the rebuildable import cache in
//! `.solis/`, watching the folder for changes, and worker threads for
//! imports.
//!
//! Layer 1 (engine). Knows nothing about the editor or the GPU. Every file
//! it reads is treated as hostile. See this crate's README for its purpose
//! and limits.

pub mod backup;
pub mod cache;
mod db;
mod files;
mod hash;
mod jobs;
mod project;
pub mod sidecar;
pub mod tidy;
mod watch;

pub use db::{
    ANIMATOR_EXTENSION, ASSEMBLY_EXTENSION, AssetDb, AssetKind, AssetRecord, Change, SETTLE,
    SOUND_EVENT_EXTENSION, SOUND_EXTENSIONS, SPRITE_CLIPS_EXTENSION, Stamp, TILE_MAP_EXTENSION,
    TILESET_EXTENSION,
};
pub use files::{
    AREA_EXTENSION, FileEntry, FileError, FileKind, Listing, MAX_NAME_CHARS, TEXT_EXTENSIONS,
    file_name_from, valid_file_name,
};
pub use hash::{ContentHash, hash_file};
pub use jobs::Jobs;
pub use project::{
    ASSETS_FOLDER, CACHE_FOLDER, FORMAT, GITATTRIBUTES, GITIGNORE, PROJECT_FILE, PROJECT_FORMAT,
    Project, ProjectError, Summary, own_folder, read_limited, write_atomically,
};
pub use sidecar::{Part, PartKey, PartKind, SIDECAR_FORMAT, Sidecar};
pub use watch::Watcher;

/// A fresh, empty folder for one test, in the system's temporary folder.
#[cfg(test)]
pub(crate) fn test_folder(name: &str) -> std::path::PathBuf {
    let folder = std::env::temp_dir().join(format!("solis-test-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    folder
}

/// Things that aren't ordinary files, each called `name` in a folder of
/// its own under `folder`: a folder and, on Unix, a link to a device
/// (which never ends) and a pipe nobody writes to (which never opens).
/// Windows has neither of those in a project's folder. A reader that
/// hangs on one has lost its check (`reflect::ordinary_file`).
#[cfg(test)]
pub(crate) fn odd_files(folder: &std::path::Path, name: &str) -> Vec<std::path::PathBuf> {
    let inside = |kind: &str| {
        std::fs::create_dir_all(folder.join(kind)).unwrap();
        folder.join(kind).join(name)
    };
    let as_folder = inside("as-folder");
    std::fs::create_dir_all(&as_folder).unwrap();
    #[cfg_attr(not(unix), allow(unused_mut))]
    let mut odd = vec![as_folder];
    #[cfg(unix)]
    {
        let device = inside("as-device");
        std::os::unix::fs::symlink("/dev/zero", &device).unwrap();
        let pipe = inside("as-pipe");
        let made = std::process::Command::new("mkfifo").arg(&pipe).status();
        assert!(made.unwrap().success(), "mkfifo");
        odd.extend([device, pipe]);
    }
    odd
}
