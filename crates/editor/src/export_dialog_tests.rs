//! Build > Export and the playtest file (decision 0093).

use super::*;
use crate::test_support::{harness, info, ribbon_tab};
use crate::workbench::WorkbenchAction;
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
fn only_levels_inside_the_assets_can_start_a_game() {
    assert!(usable_level("levels/farm.sollevel"));
    assert!(usable_level("farm.sollevel"));
    for bad in [
        "",
        "farm.lst",
        "/levels/farm.sollevel",
        "../farm.sollevel",
        "levels/../../farm.sollevel",
        "levels\\farm.sollevel",
        "C:/farm.sollevel",
        "levels//farm.sollevel",
        "levels/far\nm.sollevel",
    ] {
        assert!(!usable_level(bad), "{bad}");
    }
}

#[test]
fn a_playtest_file_reads_back_as_written() {
    let p = Playtest {
        name: "Farm".into(),
        level: "levels/farm.sollevel".into(),
        fullscreen: true,
        size: [1920, 1080],
    };
    assert_eq!(Playtest::parse(&p.text()), Ok(p.clone()));
    assert!(
        p.text()
            .starts_with("playtest 2\nname Farm\nlevel levels/farm.sollevel\n")
    );
    // Odd values put right; a missing or unsafe level refused.
    let odd =
        Playtest::parse("playtest 2\nname a/b\nlevel x.sollevel\nwindow huge 5 99999\nmore\n")
            .unwrap();
    assert_eq!(odd.name, "ab");
    assert!(!odd.fullscreen);
    assert_eq!(odd.size, [320, 16384], "kept within what Solis opens");
    assert!(Playtest::parse("playtest 2\nname Farm\n").is_err());
    assert!(Playtest::parse("playtest 2\nlevel ../../x.sollevel\n").is_err());
    assert!(Playtest::parse("not one\nlevel x.sollevel\n").is_err());
    assert!(
        Playtest::parse("playtest 3\nlevel x.sollevel\n").is_err(),
        "newer"
    );
}

#[test]
fn a_version_1_playtest_file_names_its_level_by_the_new_ending() {
    // Decision 0118: version 1 said `area`, and its level's file ended
    // `.solarea`, which became `.sollevel` when the project opened.
    let one = "playtest 1\nname Farm\narea areas/farm.solarea\nwindow fullscreen 1920 1080\n";
    let read = Playtest::parse(one).unwrap();
    assert_eq!(read.level, "areas/farm.sollevel");
    assert_eq!(
        read.text(),
        "playtest 2\nname Farm\nlevel areas/farm.sollevel\nwindow fullscreen 1920 1080\n"
    );
    assert_eq!(Playtest::parse(&read.text()).unwrap(), read);
    // Each version reads only its own word for the level.
    assert!(Playtest::parse("playtest 1\nlevel x.sollevel\n").is_err());
    assert!(Playtest::parse("playtest 2\narea x.sollevel\n").is_err());
    // The examples kept for good: version 1 is written as version 2.
    let fixtures =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/playtest");
    let read_fixture = |v: &str| std::fs::read_to_string(fixtures.join(v)).unwrap();
    let one = Playtest::parse(&read_fixture("v1/playtest.txt")).unwrap();
    assert_eq!(one.text(), read_fixture("v2/playtest.txt"));
}

#[test]
fn playtest_files_survive_being_damaged() {
    // A small fuzz: a real file cut short and changed in many ways never
    // panics, and what reads is always safe to use.
    let text = Playtest {
        name: "Farm".into(),
        level: "levels/farm.sollevel".into(),
        fullscreen: false,
        size: [1280, 720],
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
        broken[at] = b" \n/.:\\abc019-levelsolfullscreen"[(seed >> 40) as usize % 30];
        let text = String::from_utf8_lossy(&broken);
        if let Ok(p) = Playtest::parse(&text) {
            assert!(usable_level(&p.level), "{text}");
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

/// A workbench with a project holding two levels, the farm open.
fn farm() -> Workbench {
    let mut w = Workbench::new(info());
    w.set_project(Some(crate::library::ProjectInfo {
        name: "Farm: the game".into(),
        root: "/projects/farm".into(),
        assets: "/projects/farm/assets".into(),
    }));
    let item = |path: &str| crate::library::LibraryItem {
        id: None,
        kind: crate::library::LibraryKind::Level,
        name: path.into(),
        path: path.into(),
        thumbnail: None,
    };
    w.open.library = vec![item("levels/town.sollevel"), item("levels/farm.sollevel")].into();
    w.open_level(
        level::Level::new(),
        std::path::PathBuf::from("/projects/farm/assets/levels/farm.sollevel"),
    );
    w.set_documents_folder(Some("/home/me/Documents".into()));
    w
}

#[test]
fn the_window_starts_from_the_project_and_the_open_level() {
    let mut w = farm();
    w.open_export();
    let window = w.open.export.as_ref().unwrap();
    assert_eq!(window.playtest.name, "Farm the game");
    assert_eq!(window.playtest.level, "levels/farm.sollevel");
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
        level: "levels/town.sollevel".into(),
        fullscreen: true,
        size: [1600, 900],
    }));
    w.set_last_export_folder(Some(local("/elsewhere/Farm playtest 2026-01-01")));
    // A form filled in afresh (opening the page again keeps the one there).
    w.prepare_export();
    let window = w.open.export.as_ref().unwrap();
    assert_eq!(window.playtest.level, "levels/town.sollevel");
    assert!(window.playtest.fullscreen);
    let folder = PathBuf::from(&window.folder);
    assert_eq!(folder.parent(), Some(local("/elsewhere").as_path()));
    let name = folder.file_name().unwrap().to_string_lossy().into_owned();
    assert!(name.starts_with("Farm playtest "), "{name}");
    assert!(!name.ends_with("2026-01-01"));
    // A level that's gone: the open one instead.
    w.set_last_export(Some(Playtest {
        level: "levels/gone.sollevel".into(),
        ..Playtest::default()
    }));
    w.prepare_export();
    assert_eq!(
        w.open.export.as_ref().unwrap().playtest.level,
        "levels/farm.sollevel"
    );
}

#[test]
fn without_a_project_there_s_nothing_to_export() {
    let mut w = Workbench::new(info());
    w.open_export();
    assert!(w.open.export.is_none());
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
    h.get_by_role_and_label(egui::accesskit::Role::Button, "Export")
        .click();
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
    assert_eq!(asked[0].playtest.level, "levels/farm.sollevel");
    assert!(asked[0].folder.starts_with("/home/me/Documents"));
    assert!(asked[0].suggested, "the window's own folder, unchanged");
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
    // On the page, and in the Console in front on Build's shelf.
    assert!(
        h.query_all_by_label_contains("the disk is full")
            .next()
            .is_some()
    );
}

#[test]
fn the_build_tab_opens_export_for_playtesting() {
    let mut h = harness(farm());
    h.run();
    // Build left Home's ribbon (decision 0118, board 03).
    assert!(h.query_by_label("Export for playtesting…").is_none());
    // The Build tab's own button.
    ribbon_tab(&mut h, crate::shell::RibbonTab::Build);
    h.get_by_label("Export for playtesting…").click();
    h.run();
    assert!(h.state().0.open.export.is_some());
}

/// N-09: the Build tab's big Export exports what the page's form says
/// (it filled the form in afresh instead, losing the name typed), and
/// Home's Build button keeps what was typed.
#[test]
fn build_s_ribbon_export_exports_the_form_as_typed() {
    let mut w = farm();
    w.open_export();
    w.open.export.as_mut().unwrap().playtest.name = "Harvest".into();
    let mut h = harness(w);
    h.run();
    ribbon_tab(&mut h, crate::shell::RibbonTab::Build);
    let form = h.state().0.open.export.as_ref().unwrap();
    assert_eq!(form.playtest.name, "Harvest", "kept, not filled in afresh");
    assert!(!form.busy);
    h.get_by_label("Export for playtesting…").click();
    h.run();
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
    assert_eq!(asked.len(), 1, "exported once");
    assert_eq!(asked[0].playtest.name, "Harvest");
    let form = h.state().0.open.export.as_ref().unwrap();
    assert!(form.busy, "busy until done");
    assert_eq!(form.playtest.name, "Harvest");
}

#[test]
fn unsaved_changes_are_pointed_out_before_exporting() {
    let mut w = farm();
    w.add_entity();
    assert!(w.open.document.is_dirty());
    w.open_export();
    let mut h = harness(w);
    h.run();
    assert!(
        h.query_by_label_contains("changes that aren't saved")
            .is_some()
    );
}

/// A whole path on this computer, as each system writes one: `/x/y`, or
/// `C:\x\y` on Windows.
fn local(path: &str) -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(format!("C:{}", path.replace('/', "\\")))
    } else {
        PathBuf::from(path)
    }
}

/// SEC-04: a project's own file said where the export goes, so a project
/// from someone else could name a folder on their server.
#[test]
fn a_project_s_file_never_chooses_the_folder() {
    for folder in [
        "/mnt/elsewhere/share/x",
        r"C:\elsewhere\x",
        r"\\server.example\share\x",
        "//server.example/share/x",
        "../../x",
    ] {
        let text = format!("playtest 2\nname Farm\nlevel levels/farm.sollevel\nfolder {folder}\n");
        let read = Playtest::parse(&text).unwrap();
        assert!(!read.text().contains("folder"), "never written back");
        let mut w = farm();
        w.set_last_export(Some(read));
        w.open_export();
        let window = w.open.export.as_ref().unwrap();
        assert_eq!(
            Path::new(&window.folder).parent(),
            Some(Path::new("/home/me/Documents/Farm exports")),
            "{folder}"
        );
    }
}

#[test]
fn only_a_whole_path_on_this_computer_is_remembered() {
    assert!(local_folder(&local("/home/me/Documents/Farm exports/x")));
    assert!(local_folder(&local("/x")));
    let long = local(&format!("/{}", "a".repeat(MAX_FOLDER)));
    for bad in [
        PathBuf::new(),
        r"\\server.example\share\x".into(),
        "//server.example/share/x".into(),
        r"\/server.example/share/x".into(),
        r"/\server.example\share\x".into(),
        r"\\?\C:\x".into(),
        r"\\?\UNC\server.example\share".into(),
        r"\\.\pipe\x".into(),
        "x/y".into(),
        "../x".into(),
        r"C:x".into(),
        local("/home/me/../../x"),
        local("/home/me/.."),
        local("/home/me/a\nb"),
        local("/home/me/a\u{7}b"),
        long,
    ] {
        assert!(!local_folder(&bad), "{}", bad.display());
        // Told to the window anyway, it's dropped: Documents instead.
        let mut w = farm();
        w.set_last_export_folder(Some(bad.clone()));
        assert_eq!(w.open.last_export_folder, None, "{}", bad.display());
        w.open_export();
        let window = w.open.export.as_ref().unwrap();
        assert!(
            window.folder.starts_with("/home/me/Documents"),
            "{}",
            window.folder
        );
    }
}

/// Any text at all is judged without a crash, and what passes is a whole
/// path that never goes up or starts as another computer's would.
#[test]
fn remembered_folders_survive_being_damaged() {
    let good = local("/home/me/Documents/Farm exports/Farm playtest 2026-11-06");
    let bytes = good.to_str().unwrap().as_bytes();
    let mut seed = 0x5EED_0404u64;
    for _ in 0..3_000 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let mut broken = bytes.to_vec();
        broken.truncate(1 + (seed as usize) % bytes.len());
        for turn in 0..3 {
            let at = (seed >> (8 * turn)) as usize % broken.len();
            broken[at] = b"\\/.:?\n\0 C$a"[(seed >> (40 + 4 * turn)) as usize % 11];
        }
        let text = String::from_utf8_lossy(&broken).into_owned();
        let folder = PathBuf::from(&text);
        if local_folder(&folder) {
            assert!(folder.is_absolute(), "{text}");
            assert!(
                !text.starts_with("\\\\") && !text.starts_with("//"),
                "{text}"
            );
            assert!(!text.split(['/', '\\']).any(|part| part == ".."), "{text}");
            assert!(!text.chars().any(char::is_control), "{text}");
        }
    }
}

/// SEC-04: opening the window looked inside the remembered folder at
/// once (to number it past an earlier export), which on another
/// computer's folder signs in to that computer before anything is
/// pressed. Here the folder it would suggest is there with a file in: a
/// window that looked would say "(2)".
#[test]
fn opening_the_window_never_looks_at_the_disk() {
    let dir = reflect::test_support::TempFolder::new("export-open").unwrap();
    let mut w = farm();
    w.set_last_export(Some(Playtest {
        name: "Farm".into(),
        level: "levels/farm.sollevel".into(),
        ..Playtest::default()
    }));
    w.set_last_export_folder(Some(dir.join("Farm playtest 2020-01-01")));
    assert!(
        w.open.last_export_folder.is_some(),
        "a folder of this computer's"
    );
    w.open_export();
    let first = PathBuf::from(&w.open.export.as_ref().unwrap().folder);
    assert_eq!(first.parent(), Some(dir.path()));
    std::fs::create_dir_all(&first).unwrap();
    std::fs::write(first.join("Farm.exe"), "x").unwrap();
    w.open_export();
    let second = w.open.export.as_ref().unwrap().folder.clone();
    // And when an export ends: the next goes beside it, found out later.
    w.export_finished(Ok(first.clone()));
    let after = w.open.export.as_ref().unwrap().folder.clone();
    assert!(!second.ends_with(" (2)"), "{second}");
    assert_eq!(PathBuf::from(after), first);
}

/// The window's own file never names anything that looks at the disk, so
/// no later change can bring the look back unnoticed: the app does it,
/// once Export is pressed.
#[test]
fn the_window_s_code_has_no_way_to_look_at_the_disk() {
    let code = include_str!("export_dialog.rs");
    for looks in [
        "std::fs",
        "fs::",
        "read_dir",
        "exists(",
        "metadata(",
        "is_dir(",
        "is_file(",
        "is_symlink(",
        "canonicalize(",
        "read_link(",
        "File::",
    ] {
        assert!(!code.contains(looks), "export_dialog.rs has `{looks}`");
    }
}

#[test]
fn only_the_window_s_own_folder_may_be_numbered() {
    // Typed over: used exactly as given.
    let mut w = farm();
    w.open_export();
    w.set_export_folder("/home/me/Documents".into());
    let mut h = harness(w);
    h.run();
    h.get_by_role_and_label(egui::accesskit::Role::Button, "Export")
        .click();
    h.run();
    let asked = |h: &egui_kittest::Harness<'_, crate::test_support::State>| {
        let actions = h.state().1.actions.iter();
        let asked: Vec<ExportRequest> = actions
            .filter_map(|a| match a {
                WorkbenchAction::ExportPlaytest(r) => Some(r.clone()),
                _ => None,
            })
            .collect();
        asked
    };
    let first = asked(&h);
    assert_eq!(first[0].folder, Path::new("/home/me/Documents"));
    assert!(!first[0].suggested, "chosen: never numbered");
    // Refused (it has files in): pressing Export again is no different.
    h.state_mut()
        .0
        .export_finished(Err("it already has files in it".into()));
    h.run();
    h.get_by_role_and_label(egui::accesskit::Role::Button, "Export")
        .click();
    h.run();
    assert!(!asked(&h).last().unwrap().suggested);
    // Made: exporting again goes beside where that one went.
    h.state_mut()
        .0
        .export_finished(Ok("/home/me/Documents/out".into()));
    h.run();
    h.get_by_role_and_label(egui::accesskit::Role::Button, "Export")
        .click();
    h.run();
    let again = asked(&h).last().unwrap().clone();
    assert_eq!(again.folder, Path::new("/home/me/Documents/out"));
    assert!(again.suggested);
}
