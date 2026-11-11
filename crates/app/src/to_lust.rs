//! Moving a project's older files to Lust when it opens
//! ([decision 0081](../../../docs/decisions/0081-what-solis-makes-is-lust.md)):
//! each file Solis made in an older layout (`farm.soltilemap`) is read,
//! written again as a Lust data file beside it (`farm.tilemap.lst`), and
//! checked; its sidecar moves with it, so its ID and everything pointing
//! at it stay; and the older file goes into the backups
//! (`.solis/backups/…`), never simply deleted.
//!
//! A file that can't be moved (it doesn't read, or a file already has the
//! new name) stays as it is, still read as before, and the Console says
//! why.

use std::path::{Path, PathBuf};

/// One kind of older file and how to move it: its old extension, the end
/// of its new name, and how to turn its text into the new text (`Err`:
/// why it can't be).
struct Older {
    extension: &'static str,
    ending: &'static str,
    convert: fn(&str) -> Result<String, String>,
}

/// Every kind of file that moves to Lust, so far.
const OLDER: &[Older] = &[
    Older {
        extension: tiles::map_file::OLD_EXTENSION,
        ending: tiles::map_file::ENDING,
        convert: |text| {
            let read = tiles::map_file::read(text).map_err(|e| e.to_string())?;
            // The older layout holds more than a Lust file that's read
            // back can: such a map stays as it is.
            tiles::map_file::write(&read.map).map_err(|e| e.to_string())
        },
    },
    Older {
        extension: tiles::tileset::OLD_EXTENSION,
        ending: tiles::tileset::ENDING,
        convert: |text| {
            let read = tiles::tileset::read(text).map_err(|e| e.to_string())?;
            Ok(tiles::tileset::write(&read.tileset))
        },
    },
];

/// The most files looked at (a project far bigger than any real one stops
/// there; the rest move the next time it opens).
const MAX_FILES: usize = 200_000;

/// Moves every older file in `assets` (inside the project at `root`) to
/// Lust. Gives back notes for the Console: one saying how many moved, and
/// one for each that couldn't.
pub(crate) fn move_older_files(root: &Path, assets: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut folders = vec![assets.to_path_buf()];
    let mut looked = 0;
    while let Some(folder) = folders.pop() {
        let Ok(entries) = std::fs::read_dir(&folder) else {
            continue;
        };
        for entry in entries.flatten() {
            looked += 1;
            if looked > MAX_FILES {
                break;
            }
            let name = entry.file_name();
            if name.to_string_lossy().starts_with('.') {
                continue;
            }
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            let path = entry.path();
            if kind.is_dir() {
                folders.push(path);
            } else if kind.is_file()
                && let Some(older) = older_kind(&path)
            {
                found.push((path, older));
            }
        }
    }
    found.sort_by(|a, b| a.0.cmp(&b.0));
    let mut notes = Vec::new();
    let mut moved = 0;
    for (path, older) in found {
        let shown = path
            .strip_prefix(assets)
            .unwrap_or(&path)
            .display()
            .to_string();
        match move_one(root, &path, older) {
            Ok(_) => moved += 1,
            Err(why) => notes.push(format!(
                "{shown} wasn't moved to Lust ({why}); it's read as it is for now."
            )),
        }
    }
    if moved > 0 {
        notes.insert(
            0,
            format!(
                "{moved} file{} made by an older Solis {} moved to Lust (a tile map's \
                 .soltilemap is a .tilemap.lst now, and so on), so you can read {} in Ledger. \
                 Each older file was kept in {}/{} first.",
                if moved == 1 { "" } else { "s" },
                if moved == 1 { "was" } else { "were" },
                if moved == 1 { "it" } else { "them" },
                asset_db::CACHE_FOLDER,
                asset_db::backup::BACKUPS_FOLDER,
            ),
        );
    }
    notes
}

/// The kind of older file `path` is, if it's one.
fn older_kind(path: &Path) -> Option<&'static Older> {
    let ext = path.extension()?.to_str()?;
    OLDER.iter().find(|o| o.extension.eq_ignore_ascii_case(ext))
}

/// Moves one older file: where its copy went, or why it couldn't.
fn move_one(root: &Path, path: &Path, older: &Older) -> Result<PathBuf, String> {
    let bytes = asset_db::read_limited(path, asset_db::backup::MAX_BACKUP_BYTES)
        .map_err(|e| e.to_string())?;
    let text = std::str::from_utf8(&bytes).map_err(|_| "it isn't text".to_string())?;
    let new_text = (older.convert)(text)?;
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("its name isn't text")?;
    let new_path = path.with_file_name(format!("{stem}{}", older.ending));
    asset_db::backup::move_to_new_format(root, path, &new_path, new_text.as_bytes(), |written| {
        // What's written reads back, and writes the same again.
        match (older.convert)(written) {
            Ok(again) if again == written => Ok(()),
            Ok(_) => Err("it read back differently".into()),
            Err(why) => Err(why),
        }
    })
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(name: &str) -> (PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "solis-to-lust-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let assets = root.join("assets");
        std::fs::create_dir_all(assets.join("maps")).unwrap();
        (root, assets)
    }

    fn fixture(path: &str) -> String {
        std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/fixtures")
                .join(path),
        )
        .unwrap()
    }

    #[test]
    fn older_tile_maps_and_tilesets_move_to_lust_keeping_their_ids() {
        let (root, assets) = project("moves");
        let map = assets.join("maps/farm.soltilemap");
        std::fs::write(&map, fixture("tilemap/v2/farm.soltilemap")).unwrap();
        std::fs::write(asset_db::sidecar::path_for(&map), "the map's ID").unwrap();
        std::fs::write(
            assets.join("ground.soltileset"),
            fixture("tileset/v3/farm.soltileset"),
        )
        .unwrap();
        let notes = move_older_files(&root, &assets);
        assert_eq!(notes.len(), 1, "{notes:?}");
        assert!(
            notes[0].starts_with("2 files made by an older Solis were moved"),
            "{notes:?}"
        );
        let new_map = assets.join("maps/farm.tilemap.lst");
        assert_eq!(
            std::fs::read_to_string(&new_map).unwrap(),
            fixture("tilemap/v3/farm.tilemap.lst")
        );
        assert_eq!(
            std::fs::read_to_string(asset_db::sidecar::path_for(&new_map)).unwrap(),
            "the map's ID"
        );
        assert!(!map.exists());
        assert_eq!(
            std::fs::read_to_string(assets.join("ground.tileset.lst")).unwrap(),
            fixture("tileset/v4/farm.tileset.lst")
        );
        // Opened again: nothing left to move.
        assert!(move_older_files(&root, &assets).is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_file_that_can_t_move_stays_and_is_said() {
        let (root, assets) = project("stays");
        std::fs::write(assets.join("maps/broken.soltilemap"), "not a tile map").unwrap();
        std::fs::write(
            assets.join("maps/both.soltilemap"),
            fixture("tilemap/v2/farm.soltilemap"),
        )
        .unwrap();
        std::fs::write(assets.join("maps/both.tilemap.lst"), "mine").unwrap();
        let notes = move_older_files(&root, &assets);
        assert_eq!(notes.len(), 2, "{notes:?}");
        assert!(notes.iter().all(|n| n.contains("wasn't moved to Lust")));
        assert!(assets.join("maps/broken.soltilemap").exists());
        assert_eq!(
            std::fs::read_to_string(assets.join("maps/both.tilemap.lst")).unwrap(),
            "mine",
            "never written over"
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}
