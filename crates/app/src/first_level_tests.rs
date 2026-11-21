//! Which level a project opens on (Fixes 1, B4): the last used, else the
//! game's start level, else the first by name; never one outside the
//! project, whatever its remembered file says.

use super::*;
use editor::export_dialog::Playtest;

fn listing(paths: &[&str]) -> asset_db::Listing {
    asset_db::Listing {
        folders: Vec::new(),
        files: paths
            .iter()
            .map(|p| asset_db::FileEntry {
                path: std::path::PathBuf::from(p),
                kind: if p.ends_with(".sollevel") {
                    asset_db::FileKind::Level
                } else {
                    asset_db::FileKind::Other
                },
                id: None,
            })
            .collect(),
    }
}

fn folder(name: &str) -> reflect::test_support::TempFolder {
    reflect::test_support::TempFolder::new(&format!("first-level-{name}")).unwrap()
}

#[test]
fn levels_are_found_by_name_with_slashes() {
    let found = levels(&listing(&[
        "zoo.sollevel",
        "maps/cave.sollevel",
        "main.lst",
        "a.sollevel",
    ]));
    assert_eq!(found, ["a.sollevel", "maps/cave.sollevel", "zoo.sollevel"]);
    assert!(levels(&listing(&["main.lst"])).is_empty());
}

#[test]
fn the_only_level_or_the_first_by_name_opens_with_nothing_remembered() {
    let root = folder("fresh");
    assert_eq!(level_to_open(&root, &[]), None, "a project without levels");
    let one = vec!["world/main.sollevel".to_string()];
    assert_eq!(
        level_to_open(&root, &one).as_deref(),
        Some("world/main.sollevel")
    );
    let two = vec!["a.sollevel".to_string(), "b.sollevel".to_string()];
    assert_eq!(level_to_open(&root, &two).as_deref(), Some("a.sollevel"));
    assert_eq!(start_level(&root, &two).as_deref(), Some("a.sollevel"));
}

#[test]
fn the_export_s_start_level_comes_before_the_first_by_name() {
    let root = folder("exported");
    let levels = vec!["a.sollevel".to_string(), "b.sollevel".to_string()];
    let playtest = Playtest {
        level: "b.sollevel".into(),
        ..Playtest::default()
    };
    crate::playtest::remember_export(&root, &playtest).unwrap();
    assert_eq!(start_level(&root, &levels).as_deref(), Some("b.sollevel"));
    assert_eq!(level_to_open(&root, &levels).as_deref(), Some("b.sollevel"));
    // One that's gone since: the first by name.
    let gone = Playtest {
        level: "gone.sollevel".into(),
        ..Playtest::default()
    };
    crate::playtest::remember_export(&root, &gone).unwrap();
    assert_eq!(start_level(&root, &levels).as_deref(), Some("a.sollevel"));
}

#[test]
fn the_level_last_used_comes_first_and_is_kept_inside_the_project() {
    let root = folder("last");
    let files = root.join("assets");
    let levels = vec!["a.sollevel".to_string(), "maps/b.sollevel".to_string()];
    remember(&root, &files, &files.join("maps").join("b.sollevel"));
    assert_eq!(
        level_to_open(&root, &levels).as_deref(),
        Some("maps/b.sollevel")
    );
    // The check plays the game's start level, never this computer's last.
    assert_eq!(start_level(&root, &levels).as_deref(), Some("a.sollevel"));
    // Outside the files, or not a level: not remembered.
    remember(&root, &files, &root.join("elsewhere.sollevel"));
    remember(&root, &files, &files.join("main.lst"));
    assert_eq!(
        level_to_open(&root, &levels).as_deref(),
        Some("maps/b.sollevel")
    );
    // Gone since: the first by name.
    assert_eq!(
        level_to_open(&root, &levels[..1]).as_deref(),
        Some("a.sollevel")
    );
}

#[test]
fn a_damaged_remembered_level_is_passed_by() {
    let root = folder("damaged");
    let levels = vec!["a.sollevel".to_string(), "../b.sollevel".to_string()];
    let own = asset_db::own_folder(&root, asset_db::CACHE_FOLDER).unwrap();
    let mut seed = 0x9e37_79b9_7f4a_7c15_u64;
    for round in 0..2000 {
        let len = (round % 70) as usize;
        let bytes: Vec<u8> = (0..len)
            .map(|_| {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                // Mostly the characters a path is made of.
                b"ab./\\:..sollevel\n\0\xff"[(seed % 19) as usize]
            })
            .collect();
        std::fs::write(own.join(LAST_LEVEL), &bytes).unwrap();
        let opened = level_to_open(&root, &levels).unwrap();
        assert!(levels.contains(&opened), "{bytes:?}");
        assert_ne!(opened, "../b.sollevel", "{bytes:?}");
    }
    // Too long to be a path: passed by.
    std::fs::write(own.join(LAST_LEVEL), vec![b'a'; 10_000]).unwrap();
    assert_eq!(level_to_open(&root, &levels).as_deref(), Some("a.sollevel"));
}
