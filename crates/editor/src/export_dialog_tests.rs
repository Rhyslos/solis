//! File > Export for playtesting… and the playtest file (decision 0093).

use super::*;
use crate::test_support::{harness, info};
use egui_kittest::kittest::Queryable;

#[test]
fn game_names_are_usable_as_file_names() {
    assert_eq!(game_name("Farm"), "Farm");
    assert_eq!(game_name("  My: Farm?/2 . "), "My Farm2");
    assert_eq!(game_name("..."), "Game");
    assert_eq!(game_name(""), "Game");
    assert_eq!(game_name("con"), "Game", "a name Windows keeps for itself");
    assert_eq!(game_name("COM1"), "Game");
    assert_eq!(game_name("lpt9"), "Game");
    assert_eq!(game_name("COM²"), "Game");
    assert_eq!(game_name("CONIN$"), "Game");
    assert_eq!(game_name("con.x"), "Game", "a device name before a dot too");
    assert_eq!(game_name("COM0"), "COM0", "Windows allows these");
    assert_eq!(game_name("LPT10"), "LPT10");
    assert_eq!(game_name("Compost"), "Compost");
    assert_eq!(game_name("Console"), "Console");
    assert_eq!(game_name(&"a".repeat(200)).len(), MAX_NAME);
    assert_eq!(game_name("tab\there"), "tabhere");
}

#[test]
fn only_areas_inside_the_assets_can_start_a_game() {
    assert!(usable_area("areas/farm.solarea"));
    assert!(usable_area("farm.solarea"));
    for bad in [
        "",
        "farm.lst",
        "/areas/farm.solarea",
        "../farm.solarea",
        "areas/../../farm.solarea",
        "areas\\farm.solarea",
        "C:/farm.solarea",
        "areas//farm.solarea",
        "areas/far\nm.solarea",
    ] {
        assert!(!usable_area(bad), "{bad}");
    }
}

#[test]
fn a_playtest_file_reads_back_as_written() {
    let p = Playtest {
        name: "Farm".into(),
        area: "areas/farm.solarea".into(),
        fullscreen: true,
        size: [1920, 1080],
        folder: Some("/home/me/Documents/Farm exports/Farm playtest 2026-11-06".into()),
    };
    assert_eq!(Playtest::parse(&p.text()), Ok(p.clone()));
    assert!(
        p.text()
            .starts_with("playtest 1\nname Farm\narea areas/farm.solarea\n")
    );
    // Odd values put right; a missing or unsafe area refused.
    let odd = Playtest::parse("playtest 1\nname a/b\narea x.solarea\nwindow huge 5 99999\nmore\n")
        .unwrap();
    assert_eq!(odd.name, "ab");
    assert!(!odd.fullscreen);
    assert_eq!(odd.size, [320, 16384], "kept within what Solis opens");
    assert!(Playtest::parse("playtest 1\nname Farm\n").is_err());
    assert!(Playtest::parse("playtest 1\narea ../../x.solarea\n").is_err());
    assert!(Playtest::parse("not one\narea x.solarea\n").is_err());
}

#[test]
fn playtest_files_survive_being_damaged() {
    // A small fuzz: a real file cut short and changed in many ways never
    // panics, and what reads is always safe to use.
    let text = Playtest {
        name: "Farm".into(),
        area: "areas/farm.solarea".into(),
        fullscreen: false,
        size: [1280, 720],
        folder: Some("/x/y".into()),
    }
    .text();
    let bytes = text.as_bytes();
    let mut seed = 0x5EED_1234u64;
    for _ in 0..3_000 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let mut broken = bytes.to_vec();
        broken.truncate(1 + (seed as usize) % bytes.len());
        let at = (seed >> 20) as usize % broken.len();
        broken[at] = b" \n/.:\\abc019-areasolfullscreen"[(seed >> 40) as usize % 30];
        let text = String::from_utf8_lossy(&broken);
        if let Ok(p) = Playtest::parse(&text) {
            assert!(usable_area(&p.area), "{text}");
            assert_eq!(game_name(&p.name), p.name);
            assert!(p.size[0] >= 320 && p.size[1] >= 320, "{:?}", p.size);
        }
    }
}

#[test]
fn exports_go_in_a_dated_folder_in_documents() {
    assert_eq!(
        default_folder(Path::new("/home/me/Documents"), "Farm", "2026-11-06"),
        Path::new("/home/me/Documents/Farm exports/Farm playtest 2026-11-06")
    );
}

/// A workbench with a project holding two areas, the farm open.
fn farm() -> Workbench {
    let mut w = Workbench::new(info(), None);
    w.set_project(Some(crate::library::ProjectInfo {
        name: "Farm: the game".into(),
        root: "/projects/farm".into(),
        assets: "/projects/farm/assets".into(),
    }));
    let item = |path: &str| crate::library::LibraryItem {
        id: None,
        kind: crate::library::LibraryKind::Area,
        name: path.into(),
        path: path.into(),
        thumbnail: None,
    };
    w.library = vec![item("areas/town.solarea"), item("areas/farm.solarea")];
    w.open_area(
        area::Area::new(),
        std::path::PathBuf::from("/projects/farm/assets/areas/farm.solarea"),
    );
    w.set_documents_folder(Some("/home/me/Documents".into()));
    w
}

#[test]
fn the_window_starts_from_the_project_and_the_open_area() {
    let mut w = farm();
    w.open_export();
    let window = w.export_window.as_ref().unwrap();
    assert_eq!(window.playtest.name, "Farm the game");
    assert_eq!(window.playtest.area, "areas/farm.solarea");
    let folder = PathBuf::from(&window.folder);
    assert_eq!(
        folder.parent(),
        Some(Path::new("/home/me/Documents/Farm the game exports"))
    );
    let name = folder.file_name().unwrap().to_string_lossy().into_owned();
    assert!(name.starts_with("Farm the game playtest "), "{name}");
    // Remembered from last time: a new dated folder beside the last.
    w.set_last_export(Some(Playtest {
        name: "Farm".into(),
        area: "areas/town.solarea".into(),
        fullscreen: true,
        size: [1600, 900],
        folder: Some("/elsewhere/Farm playtest 2026-01-01".into()),
    }));
    w.open_export();
    let window = w.export_window.as_ref().unwrap();
    assert_eq!(window.playtest.area, "areas/town.solarea");
    assert!(window.playtest.fullscreen);
    let folder = PathBuf::from(&window.folder);
    assert_eq!(folder.parent(), Some(Path::new("/elsewhere")));
    let name = folder.file_name().unwrap().to_string_lossy().into_owned();
    assert!(name.starts_with("Farm playtest "), "{name}");
    assert!(!name.ends_with("2026-01-01"));
    // An area that's gone: the open one instead.
    w.set_last_export(Some(Playtest {
        area: "areas/gone.solarea".into(),
        ..Playtest::default()
    }));
    w.open_export();
    assert_eq!(
        w.export_window.as_ref().unwrap().playtest.area,
        "areas/farm.solarea"
    );
}

#[test]
fn without_a_project_there_s_nothing_to_export() {
    let mut w = Workbench::new(info(), None);
    w.open_export();
    assert!(w.export_window.is_none());
    assert!(
        w.log_lines()
            .last()
            .unwrap()
            .text
            .contains("needs a project")
    );
}

#[test]
fn export_asks_the_app_and_says_where_it_went() {
    let mut w = farm();
    w.open_export();
    let mut h = harness(w);
    h.run();
    h.get_by_label("Export").click();
    h.run();
    let asked: Vec<_> = h
        .state()
        .1
        .actions
        .iter()
        .filter_map(|a| match a {
            WorkbenchAction::ExportPlaytest(r) => Some(r.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(asked.len(), 1, "once");
    assert_eq!(asked[0].playtest.area, "areas/farm.solarea");
    assert_eq!(asked[0].playtest.folder.as_ref(), Some(&asked[0].folder));
    assert!(h.query_by_label("Exporting…").is_some(), "busy until done");
    h.state_mut()
        .0
        .export_finished(Ok("/home/me/Documents/Farm exports/x".into()));
    h.run();
    assert!(h.query_by_label("Open folder").is_some());
    h.get_by_label("Open folder").click();
    h.run();
    assert!(h.state().1.actions.iter().any(|a| matches!(
        a,
        WorkbenchAction::ShowFolder(p) if p.ends_with("x")
    )));
    assert!(
        h.state()
            .0
            .log_lines()
            .iter()
            .any(|l| l.text.starts_with("Exported for playtesting to"))
    );
    h.state_mut()
        .0
        .export_finished(Err("the disk is full".into()));
    h.run();
    assert!(h.query_by_label_contains("the disk is full").is_some());
}

#[test]
fn file_has_export_for_playtesting_above_quit() {
    let mut h = harness(farm());
    h.run();
    h.get_by_label("File").click();
    h.run();
    h.get_by_label("Export for playtesting…").click();
    h.run();
    assert!(h.state().0.export_window.is_some());
}

#[test]
fn a_second_export_the_same_day_gets_a_folder_of_its_own() {
    let dir = std::env::temp_dir().join(format!("solis-export-unused-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let first = dir.join("Farm playtest 2026-11-06");
    assert_eq!(unused(first.clone()), first, "nothing there yet");
    std::fs::create_dir_all(&first).unwrap();
    assert_eq!(unused(first.clone()), first, "an empty folder is fine");
    std::fs::write(first.join("Farm.exe"), "x").unwrap();
    let second = dir.join("Farm playtest 2026-11-06 (2)");
    assert_eq!(unused(first.clone()), second);
    std::fs::create_dir_all(&second).unwrap();
    std::fs::write(second.join("Farm.exe"), "x").unwrap();
    assert_eq!(unused(first), dir.join("Farm playtest 2026-11-06 (3)"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn unsaved_changes_are_pointed_out_before_exporting() {
    let mut w = farm();
    w.add_entity();
    assert!(w.document.is_dirty());
    w.open_export();
    let mut h = harness(w);
    h.run();
    assert!(
        h.query_by_label_contains("changes that aren't saved")
            .is_some()
    );
}
