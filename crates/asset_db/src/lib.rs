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
mod watch;

pub use db::{
    ANIMATOR_EXTENSION, ASSEMBLY_EXTENSION, AssetDb, AssetKind, AssetRecord, Change, SETTLE,
    SOUND_EVENT_EXTENSION, SOUND_EXTENSIONS, SPRITE_CLIPS_EXTENSION, Stamp, TILE_MAP_EXTENSION,
    TILESET_EXTENSION,
};
pub use files::{
    AREA_EXTENSION, FileEntry, FileError, FileKind, Listing, MAX_NAME_CHARS, file_name_from,
    valid_file_name,
};
pub use hash::{ContentHash, hash_file};
pub use jobs::Jobs;
pub use project::{
    ASSETS_FOLDER, CACHE_FOLDER, FORMAT, GITATTRIBUTES, GITIGNORE, PROJECT_FILE, PROJECT_FORMAT,
    Project, ProjectError, own_folder, read_limited, write_atomically,
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
