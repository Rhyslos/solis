//! Moving a project's older files to their new names when it opens:
//! each file Solis made in an older layout or under an older name is
//! read, written again beside it under its new name, and checked; its
//! sidecar moves with it, so its ID and everything pointing at it stay;
//! and the older file goes into the backups (`.solis/backups/…`), never
//! simply deleted.
//!
//! - **Lust** ([decision 0081](../../../docs/decisions/0081-what-solis-makes-is-lust.md)):
//!   `farm.soltilemap` becomes the Lust data file `farm.tilemap.lst`.
//! - **Levels** ([decision 0118](../../../docs/decisions/0118-the-editor-rethink.md)):
//!   areas are called levels, so `farm.solarea` becomes `farm.sollevel`,
//!   its header saying `level` (the layout is the same: format 4).
//! - **Prefabs** (decision 0118): assemblies are called prefabs, so
//!   `crate.solassembly` becomes the Lust data file `crate.prefab.lst`
//!   (decision 0081). Levels name their copies' prefab by its ID, which
//!   the sidecar keeps, and game code by its path without its ending
//!   (`Prefab.Crate`), which doesn't change: nothing else needs moving.
//!
//! A file that can't be moved (it doesn't read, or a file already has the
//! new name) stays as it is, still read as before, and the Console says
//! why.

use std::path::{Path, PathBuf};

/// One kind of older file and how to move it: its old extension, the end
/// of its new name, how to turn its text into the new text (`Err`: why it
/// can't be), and why it moves.
struct Older {
    extension: &'static str,
    ending: &'static str,
    convert: fn(&str) -> Result<String, String>,
    why: Why,
}

/// Why a kind of file moves, which is what the Console says about it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Why {
    /// What Solis makes is Lust (decision 0081).
    Lust,
    /// Areas are called levels (decision 0118).
    Levels,
    /// Assemblies are called prefabs (decision 0118).
    Prefabs,
}

impl Why {
    /// The note for `moved` files of this kind that moved.
    fn moved(self, moved: usize) -> String {
        let one = moved == 1;
        match self {
            Self::Lust => format!(
                "{moved} file{} made by an older Solis {} moved to Lust (a tile map's \
                 .soltilemap is a .tilemap.lst now, and so on), so you can read {} in Ledger.",
                if one { "" } else { "s" },
                if one { "was" } else { "were" },
                if one { "it" } else { "them" },
            ),
            Self::Levels => format!(
                "{moved} area{} made by an older Solis {} renamed: areas are called levels \
                 now, so an area's .solarea file is a level's .sollevel file.",
                if one { "" } else { "s" },
                if one { "was" } else { "were" },
            ),
            Self::Prefabs => format!(
                "{moved} assembl{} made by an older Solis {} renamed: assemblies are called \
                 prefabs now, so an assembly's .solassembly file is a prefab's .prefab.lst file \
                 (game code names it Prefab, where it said Assembly).",
                if one { "y" } else { "ies" },
                if one { "was" } else { "were" },
            ),
        }
    }

    /// The note for a file that couldn't move.
    fn stayed(self, shown: &str, why: &str) -> String {
        match self {
            Self::Lust => {
                format!("{shown} wasn't moved to Lust ({why}); it's read as it is for now.")
            }
            Self::Levels => {
                format!("{shown} wasn't renamed a level ({why}); it's read as an area for now.")
            }
            Self::Prefabs => format!(
                "{shown} wasn't renamed a prefab ({why}); it's read as an assembly for now."
            ),
        }
    }
}

/// Every kind of file that moves when its project opens, so far.
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
        why: Why::Lust,
    },
    Older {
        extension: tiles::tileset::OLD_EXTENSION,
        ending: tiles::tileset::ENDING,
        convert: |text| {
            let read = tiles::tileset::read(text).map_err(|e| e.to_string())?;
            Ok(tiles::tileset::write(&read.tileset))
        },
        why: Why::Lust,
    },
    Older {
        extension: asset_db::OLD_LEVEL_EXTENSION,
        ending: ".sollevel",
        convert: level_text,
        why: Why::Levels,
    },
    Older {
        extension: asset_db::OLD_PREFAB_EXTENSION,
        ending: asset_db::PREFAB_ENDING,
        convert: crate::prefab_files::from_old,
        why: Why::Prefabs,
    },
];

/// An area's text as a level's (decision 0118): the same blocks, under a
/// header that says `level` at the current version, and the first line's
/// comment naming a level. Everything else is kept byte for byte (a
/// hand-written comment too), and the new text must read as exactly what
/// the old one did. A level's own text is given back as it is.
fn level_text(text: &str) -> Result<String, String> {
    use level::text::{FORMAT, HEADER_WORD, OLD_HEADER_WORD, VERSION};
    let registry = level::builtin::registry().map_err(|e| e.to_string())?;
    let read = |text: &str| level::text::read_file(text, &registry).map_err(|e| e.to_string());
    let old = read(text)?;
    let mut new = String::with_capacity(text.len());
    let mut header_done = false;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_start_matches('\u{feff}').trim();
        if header_done || trimmed.is_empty() || trimmed.starts_with("--") {
            match line.strip_prefix(OLD_COMMENT) {
                Some(rest) if !header_done => {
                    new.push_str(NEW_COMMENT);
                    new.push_str(rest);
                }
                _ => new.push_str(line),
            }
            continue;
        }
        header_done = true;
        let words: Vec<&str> = trimmed
            .split(|c: char| c.is_whitespace() || c == ';')
            .collect();
        match words.as_slice() {
            [word, "version", _, ..] if *word == OLD_HEADER_WORD || *word == HEADER_WORD => {
                let end = if line.ends_with("\r\n") { "\r\n" } else { "\n" };
                new.push_str(&format!("{HEADER_WORD} version {VERSION};{end}"));
            }
            // A prefab, or nothing a level starts with.
            _ => return Err("it isn't an area".into()),
        }
    }
    if (FORMAT.version_of)(&new) != Some(VERSION) {
        return Err("its first line couldn't be made a level's".into());
    }
    if read(&new)? != old {
        return Err("as a level it would read differently".into());
    }
    Ok(new)
}

/// The comment an area's first line had, and a level's.
const OLD_COMMENT: &str = "-- An area. Safe to edit by hand";
const NEW_COMMENT: &str = "-- A level. Safe to edit by hand";

/// The most files looked at (a project far bigger than any real one stops
/// there; the rest move the next time it opens).
const MAX_FILES: usize = 200_000;

/// Moves every older file in `assets` (inside the project at `root`) to
/// its new name. Gives back notes for the Console: one saying how many of
/// each kind moved, and one for each that couldn't.
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
    let mut moved = std::collections::BTreeMap::<Why, usize>::new();
    for (path, older) in found {
        let shown = path
            .strip_prefix(assets)
            .unwrap_or(&path)
            .display()
            .to_string();
        match move_one(root, &path, older) {
            Ok(_) => *moved.entry(older.why).or_default() += 1,
            Err(why) => notes.push(older.why.stayed(&shown, &why)),
        }
    }
    let said: Vec<String> = moved.into_iter().map(|(why, n)| why.moved(n)).collect();
    if !said.is_empty() {
        notes.insert(
            0,
            format!(
                "{} Each older file was kept in {}/{} first.",
                said.join(" "),
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

    /// A project's folder (removed when it's dropped) and its assets.
    fn project(name: &str) -> (reflect::test_support::TempFolder, PathBuf) {
        let root = reflect::test_support::TempFolder::new(&format!("to-lust-{name}")).unwrap();
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
    }

    #[test]
    fn the_new_ending_is_the_level_extension() {
        let levels = OLDER.iter().find(|o| o.why == Why::Levels).unwrap();
        assert_eq!(levels.ending, format!(".{}", asset_db::LEVEL_EXTENSION));
        assert_eq!(levels.extension, asset_db::OLD_LEVEL_EXTENSION);
    }

    #[test]
    fn areas_move_to_levels_keeping_their_ids_and_every_line() {
        // Decision 0118: the version 3 example becomes the version 5 one
        // byte for byte; a version 1 area keeps every line but its first
        // two (a hand-written comment too), and both keep their sidecars.
        let (root, assets) = project("levels");
        std::fs::create_dir_all(assets.join("levels/docks")).unwrap();
        let blockout = assets.join("levels/blockout.solarea");
        std::fs::write(&blockout, fixture("area/v3/blockout.solarea")).unwrap();
        std::fs::write(asset_db::sidecar::path_for(&blockout), "the blockout's ID").unwrap();
        let turret = assets.join("levels/docks/turret.SOLAREA");
        let old_turret =
            fixture("area/v1/turret.solarea").replacen("\n\n", "\n-- my own note\n\n", 1);
        std::fs::write(&turret, &old_turret).unwrap();
        let notes = move_older_files(&root, &assets);
        assert_eq!(notes.len(), 1, "{notes:?}");
        assert!(
            notes[0].starts_with(
                "2 areas made by an older Solis were renamed: areas are called levels now"
            ),
            "{notes:?}"
        );
        assert!(notes[0].contains(".solis/backups"), "{notes:?}");
        let new = assets.join("levels/blockout.sollevel");
        assert_eq!(
            std::fs::read_to_string(&new).unwrap(),
            fixture("area/v5/blockout.sollevel")
        );
        assert_eq!(
            std::fs::read_to_string(asset_db::sidecar::path_for(&new)).unwrap(),
            "the blockout's ID"
        );
        assert!(!blockout.exists());
        let moved = std::fs::read_to_string(assets.join("levels/docks/turret.sollevel")).unwrap();
        let registry = level::builtin::registry().unwrap();
        assert_eq!(
            level::text::read_file(&moved, &registry).unwrap(),
            level::text::read_file(&old_turret, &registry).unwrap()
        );
        assert!(moved.starts_with("-- A level. Safe to edit by hand"));
        assert_eq!(moved.lines().nth(1), Some("level version 5;"));
        assert!(moved.contains("-- my own note"));
        assert_eq!(
            moved.lines().skip(2).collect::<Vec<_>>(),
            old_turret.lines().skip(2).collect::<Vec<_>>()
        );
        // The older files are in the backups, as they were.
        let backups = asset_db::backup::folder(&root);
        let copies: Vec<PathBuf> = walk(&backups);
        assert_eq!(copies.len(), 2, "{copies:?}");
        assert!(copies.iter().any(|c| c.ends_with("levels/blockout.solarea")
            && std::fs::read_to_string(c).unwrap() == fixture("area/v3/blockout.solarea")));
        assert!(
            copies
                .iter()
                .any(|c| c.ends_with("levels/docks/turret.SOLAREA")
                    && std::fs::read_to_string(c).unwrap() == old_turret)
        );
        // Opened again: nothing left to move.
        assert!(move_older_files(&root, &assets).is_empty());
    }

    #[test]
    fn an_area_that_can_t_move_stays_an_area_and_is_said() {
        let (root, assets) = project("levels-stay");
        std::fs::write(
            assets.join("broken.solarea"),
            "area version 1;\nentity X {}",
        )
        .unwrap();
        std::fs::write(
            assets.join("crate.solarea"),
            fixture("area/v1/crate.solassembly"),
        )
        .unwrap();
        std::fs::write(
            assets.join("both.solarea"),
            fixture("area/v3/blockout.solarea"),
        )
        .unwrap();
        std::fs::write(assets.join("both.sollevel"), "mine").unwrap();
        std::fs::write(assets.join("newer.solarea"), "area version 9;\n").unwrap();
        let notes = move_older_files(&root, &assets);
        assert_eq!(notes.len(), 4, "{notes:?}");
        assert!(
            notes.iter().all(|n| n.contains("wasn't renamed a level")),
            "{notes:?}"
        );
        for name in ["broken", "crate", "both", "newer"] {
            assert!(assets.join(format!("{name}.solarea")).exists(), "{name}");
        }
        assert_eq!(
            std::fs::read_to_string(assets.join("both.sollevel")).unwrap(),
            "mine"
        );
        assert!(
            !asset_db::backup::folder(&root).exists(),
            "nothing was kept: nothing moved"
        );
    }

    #[test]
    fn a_level_s_text_is_given_back_as_it_is() {
        let level = fixture("area/v5/blockout.sollevel");
        assert_eq!(level_text(&level).unwrap(), level);
        // Damaged input in many ways is refused or moved, never a crash,
        // and what moves reads as the area did.
        let area = fixture("area/v1/turret.solarea");
        let registry = level::builtin::registry().unwrap();
        let mut seed = 0x0118_u64;
        for _ in 0..500 {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let mut bytes = area.clone().into_bytes();
            let at = (seed as usize) % bytes.len();
            bytes[at] = b"area version;{}\n\r 0123456789-\""[(seed >> 40) as usize % 30];
            let Ok(text) = String::from_utf8(bytes) else {
                continue;
            };
            if let Ok(moved) = level_text(&text) {
                assert_eq!(
                    level::text::read_file(&moved, &registry).unwrap(),
                    level::text::read_file(&text, &registry).unwrap()
                );
            }
        }
    }

    /// Every file under `folder`.
    fn walk(folder: &Path) -> Vec<PathBuf> {
        let mut found = Vec::new();
        let mut folders = vec![folder.to_path_buf()];
        while let Some(f) = folders.pop() {
            for entry in std::fs::read_dir(&f).into_iter().flatten().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    folders.push(path);
                } else {
                    found.push(path);
                }
            }
        }
        found
    }
}
