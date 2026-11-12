//! The editor's basics (E1): New in the Library, a new Lust file that
//! compiles, a new area, and what isn't built kept out of sight.

use crate::library::{LibraryItem, LibraryKind, LibraryRequest, ProjectInfo};
use crate::test_support::*;
use crate::workbench::{Workbench, WorkbenchAction};
use egui_kittest::kittest::Queryable;

fn farm() -> ProjectInfo {
    ProjectInfo {
        name: "Farm".into(),
        root: "/projects/farm".into(),
        assets: "/projects/farm/assets".into(),
    }
}

fn item(path: &str, kind: LibraryKind) -> LibraryItem {
    LibraryItem {
        id: Some(reflect::AssetId::random()),
        kind,
        name: crate::sprite_sheets::name_of(path),
        path: path.into(),
        thumbnail: None,
    }
}

/// The text of each new file asked for, by path.
fn made(actions: &[WorkbenchAction]) -> Vec<(String, String)> {
    actions
        .iter()
        .filter_map(|a| match a {
            WorkbenchAction::NewTextFile { path, text } => {
                Some((path.display().to_string(), text.clone()))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn the_first_code_file_is_main_lst_and_later_ones_components() {
    let mut w = Workbench::new(info());
    w.set_project(Some(farm()));
    let mut actions = Vec::new();
    w.library_request(LibraryRequest::NewCodeFile("crops".into()), &mut actions);
    let files = made(&actions);
    assert_eq!(files[0].0, "main.lst", "game code starts there");
    assert!(files[0].1.contains("component Game"));
    // It opens in the computer's editor for Lust (Ledger), once written.
    let opened = actions
        .iter()
        .position(|a| matches!(a, WorkbenchAction::OpenExternally(p) if p.ends_with("main.lst")))
        .expect("opened");
    assert!(opened > 0, "after it's made");
    w.set_library(vec![item("main.lst", LibraryKind::Code)]);
    let mut actions = Vec::new();
    w.library_request(LibraryRequest::NewCodeFile("crops".into()), &mut actions);
    let files = made(&actions);
    assert_eq!(files[0].0, "crops/component.lst");
    assert!(files[0].1.contains("fetch \"crops/component\":Component;"));
    assert!(files[0].1.contains("component Component"));
}

#[test]
fn new_code_files_compile_as_they_are() {
    // The game's first file on its own, and a component fetched from it.
    let check = |files: Vec<(String, String)>| {
        let mut all = script::describe::descriptions("Test 1.0", "solis");
        all.extend(files);
        let code = script::Code {
            files: all,
            lua: Vec::new(),
            notes: Vec::new(),
        };
        let problems = script::check(&code);
        assert!(problems.is_empty(), "{problems:?}");
    };
    let main = crate::workbench_library::starter_code("Game", "main.lst");
    check(vec![("main.lst".into(), main)]);
    let wheat = crate::workbench_library::starter_code("Wheat", "crops/wheat.lst");
    check(vec![
        (
            "main.lst".into(),
            "fetch \"crops/wheat\":Wheat;\nend\n".into(),
        ),
        ("crops/wheat.lst".into(), wheat),
    ]);
}

#[test]
fn a_new_area_is_saved_where_it_was_asked_for_and_opened() {
    let mut w = Workbench::new(info());
    w.set_project(Some(farm()));
    w.set_library(vec![item("levels/area.solarea", LibraryKind::Area)]);
    let mut actions = Vec::new();
    w.library_request(LibraryRequest::NewArea("levels".into()), &mut actions);
    let files = made(&actions);
    assert_eq!(files[0].0, "levels/area 2.solarea", "a free name");
    assert!(files[0].1.contains("area version"));
    let read = area::text::read(&files[0].1, &w.open.document.registry).expect("reads");
    assert!(!read.is_empty(), "an environment, as a new area has");
    assert!(actions.iter().any(
        |a| matches!(a, WorkbenchAction::OpenAreaFile(p) if p.ends_with("levels/area 2.solarea"))
    ));
}

#[test]
fn right_clicking_the_library_offers_new_everywhere() {
    let mut w = Workbench::new(info());
    w.set_project(Some(farm()));
    w.set_library(vec![item("tiles.png", LibraryKind::Texture)]);
    let mut h = harness(w);
    h.run();
    crate::test_support::ribbon_tab(&mut h, crate::shell::RibbonTab::Library);
    h.get_by_label("Create").click();
    h.run();
    for label in [
        "Lust code file",
        "Folder",
        "Tile map",
        "Tileset",
        "Animator",
        "Sound event",
    ] {
        assert!(h.query_by_label(label).is_some(), "{label}");
    }
    assert!(
        h.query_all_by_label("Area").count() >= 1,
        "the New list's item"
    );
    h.get_by_label("Tile map").click();
    h.run();
    assert!(h.state().1.actions.iter().any(|a| matches!(
        a,
        WorkbenchAction::SaveFile {
            file: crate::OwnFile::TileMap(_),
            ..
        }
    )));
}

#[test]
fn what_isn_t_built_isn_t_shown() {
    let mut h = harness(Workbench::new(info()));
    h.run();
    for label in [
        "Build",
        "Build & Play",
        "Play with two players",
        "Layers",
        "Wireframe",
    ] {
        assert!(h.query_by_label(label).is_none(), "{label}");
    }
}
