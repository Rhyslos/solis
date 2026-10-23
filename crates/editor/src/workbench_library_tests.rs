//! The Library in the whole editor: folders, search, the Code panel,
//! placing by dragging, and the file actions it asks the app for.

use crate::library::{LibraryItem, LibraryKind, ProjectInfo};
use crate::panel::Panel;
use crate::test_support::*;
use crate::viewport_tools::PickAnswer;
use crate::workbench::{Workbench, WorkbenchAction};
use area::builtin::Transform;
use egui_kittest::kittest::Queryable;
use glam::Vec3;
use reflect::AssetId;

fn item(path: &str, kind: LibraryKind, id: Option<AssetId>) -> LibraryItem {
    let name = path
        .rsplit('/')
        .next()
        .and_then(|n| n.split('.').next())
        .unwrap_or_default()
        .to_string();
    LibraryItem {
        id,
        kind,
        name,
        path: path.into(),
        thumbnail: None,
    }
}

/// A workbench with a project holding a crate, a rock in a folder, an
/// area, a picture and some code; the Library in front.
fn with_a_project() -> (Workbench, AssetId, AssetId) {
    let mut w = Workbench::new(info(), None);
    w.set_project(Some(ProjectInfo {
        name: "Harbour".into(),
        root: "/projects/harbour".into(),
        assets: "/projects/harbour/assets".into(),
    }));
    let crate_id = AssetId::random();
    let rock_id = AssetId::random();
    w.set_library(vec![
        item("crate.glb", LibraryKind::Model, Some(crate_id)),
        item("props/rock.glb", LibraryKind::Model, Some(rock_id)),
        item("docks.solarea", LibraryKind::Area, None),
        item("props/bark.png", LibraryKind::Texture, None),
        item("turret.lst", LibraryKind::Code, None),
    ]);
    w.set_folders(vec!["props".into()]);
    (w, crate_id, rock_id)
}

fn shown(h: &egui_kittest::Harness<'_, State>, name: &str) -> bool {
    h.query_all_by_label(name).next().is_some()
}

#[test]
fn the_library_shows_one_folder_and_the_tree_opens_others() {
    let (w, _, _) = with_a_project();
    let mut h = harness(w);
    h.run();
    assert!(shown(&h, "crate") && shown(&h, "docks"));
    assert!(!shown(&h, "rock"), "inside a folder");
    assert!(!shown(&h, "turret"), "code is in the Code panel");
    h.get_by_label("Folder props").click();
    h.run();
    assert!(shown(&h, "rock") && shown(&h, "bark"));
    assert!(!shown(&h, "crate"));
    // Searching looks in every folder.
    h.state_mut().0.library_state.search = "r".into();
    h.run();
    assert!(shown(&h, "rock") && shown(&h, "crate"));
}

#[test]
fn the_code_panel_lists_code() {
    let (mut w, _, _) = with_a_project();
    let path = w.dock.find_tab(&Panel::Code).unwrap();
    let _ = w.dock.set_active_tab(path);
    let mut h = harness(w);
    h.run();
    assert!(shown(&h, "turret"));
}

#[test]
fn placing_models_and_opening_areas() {
    let (w, crate_id, _) = with_a_project();
    let mut h = harness(w);
    h.run();
    h.get_by_label("Place crate").click();
    h.run();
    open(&mut h, "docks");
    h.run();
    let actions = &h.state().1.actions;
    assert!(actions.contains(&WorkbenchAction::PlaceAsset(crate_id)));
    assert!(actions.contains(&WorkbenchAction::OpenAreaFile(
        "/projects/harbour/assets/docks.solarea".into()
    )));
}

#[test]
fn opening_an_area_with_unsaved_changes_asks_first() {
    let (mut w, _, _) = with_a_project();
    w.add_point_light();
    let mut h = harness(w);
    h.run();
    open(&mut h, "docks");
    h.run();
    assert!(
        !h.state()
            .1
            .actions
            .iter()
            .any(|a| matches!(a, WorkbenchAction::OpenAreaFile(_))),
        "not before the answer"
    );
    let _ = h.get_by_label("Don't save");
}

#[test]
fn the_library_asks_the_app_to_move_make_folders_and_delete() {
    let (mut w, _, _) = with_a_project();
    let mut actions = Vec::new();
    use crate::library::LibraryRequest as R;
    w.library_request(
        R::Move("crate.glb".into(), "props/crate.glb".into()),
        &mut actions,
    );
    w.library_request(R::NewFolder("props".into()), &mut actions);
    w.library_request(R::Delete("props/bark.png".into()), &mut actions);
    w.library_request(R::Show("props".into()), &mut actions);
    w.library_request(
        R::Open(LibraryKind::Texture, "props/bark.png".into()),
        &mut actions,
    );
    assert_eq!(
        actions,
        [
            WorkbenchAction::MoveFile {
                from: "crate.glb".into(),
                to: "props/crate.glb".into()
            },
            WorkbenchAction::NewFolder("props".into()),
            WorkbenchAction::DeleteFile("props/bark.png".into()),
            WorkbenchAction::OpenExternally("/projects/harbour/assets/props".into()),
            WorkbenchAction::OpenExternally("/projects/harbour/assets/props/bark.png".into()),
        ]
    );
}

/// Opens the item called `name` from its right-click menu (what a
/// double-click does too; kittest can't double-click reliably).
fn open(h: &mut egui_kittest::Harness<'_, State>, name: &str) {
    h.get_by_label(name).click_secondary();
    h.run();
    h.get_by_label("Open").click();
    h.run();
    h.run();
}

/// Drags the Library item `name` to `to` and lets go.
fn drag(h: &mut egui_kittest::Harness<'_, State>, name: &str, to: egui::Pos2) {
    let from = h.get_by_label(name).rect().center();
    h.hover_at(from);
    h.run();
    press(h, true, from, egui::Modifiers::NONE);
    h.run();
    for t in [0.1, 0.4, 0.8, 1.0] {
        h.hover_at(from + (to - from) * t);
        h.run();
    }
    press(h, false, to, egui::Modifiers::NONE);
    h.run();
    h.run();
}

#[test]
fn a_model_dropped_on_the_viewport_goes_on_the_surface_under_the_mouse() {
    let (w, crate_id, _) = with_a_project();
    let mut h = harness(w);
    h.run();
    let point = Vec3::new(3.0, 4.0, 1.5);
    h.state_mut().1.answer = Some(PickAnswer {
        middle: Some(1),
        point: Some(point),
        ..PickAnswer::default()
    });
    drag(&mut h, "crate", IN_VIEW);
    h.run();
    assert!(
        h.state()
            .1
            .actions
            .contains(&WorkbenchAction::PlaceAsset(crate_id)),
        "{:?}",
        h.state().1.actions
    );
    // When the app has loaded it, it goes where it was dropped.
    let info = crate::models::ModelInfo {
        asset: crate_id,
        ..crate_model()
    };
    let id = h.state_mut().0.add_model(info, &[], 0.1, &[]);
    let w = &h.state().0;
    let t: Transform = w.document.area().get(id).unwrap().get().unwrap();
    assert_eq!(t.position, point);
    assert_eq!(w.selection(), Some(id), "and is selected");
}

#[test]
fn a_model_dropped_where_nothing_is_lands_on_the_ground() {
    let (w, crate_id, _) = with_a_project();
    let mut h = harness(w);
    h.run();
    h.state_mut().1.answer = Some(PickAnswer::default());
    drag(&mut h, "crate", IN_VIEW);
    h.run();
    let info = crate::models::ModelInfo {
        asset: crate_id,
        ..crate_model()
    };
    let id = h.state_mut().0.add_model(info, &[], 0.1, &[]);
    let t: Transform = h.state().0.document.area().get(id).unwrap().get().unwrap();
    assert!(t.position.z.abs() < 1e-3, "on the ground: {}", t.position);
}

#[test]
fn a_model_dropped_on_an_entity_goes_under_it() {
    let (mut w, crate_id, _) = with_a_project();
    let room = w.add_entity();
    w.apply_area_request(crate::area_panel::AreaRequest::Rename {
        entity: room,
        name: "Boathouse".into(),
    });
    let room_name = "Boathouse";
    w.document.selection.clear();
    let mut h = harness(w);
    h.run();
    let target = h
        .get_all_by_label(room_name)
        .next()
        .unwrap()
        .rect()
        .center();
    drag(&mut h, "crate", target);
    assert!(
        h.state()
            .1
            .actions
            .contains(&WorkbenchAction::PlaceAsset(crate_id))
    );
    let info = crate::models::ModelInfo {
        asset: crate_id,
        ..crate_model()
    };
    let id = h.state_mut().0.add_model(info, &[], 0.1, &[]);
    let w = &h.state().0;
    let placed = w.document.area().get(id).unwrap();
    assert_eq!(placed.parent, Some(room));
    let t: Transform = placed.get().unwrap();
    assert!(t.position.length() < 1e-4, "at the room's own place");
}

#[test]
fn dropping_a_file_on_a_folder_moves_it_there() {
    let (w, _, _) = with_a_project();
    let mut h = harness(w);
    h.run();
    let folder = h.get_by_label("Folder props").rect().center();
    drag(&mut h, "crate", folder);
    assert!(h.state().1.actions.contains(&WorkbenchAction::MoveFile {
        from: "crate.glb".into(),
        to: "props/crate.glb".into(),
    }));
}

#[test]
fn without_a_project_the_library_says_how_to_make_one() {
    let mut h = harness(Workbench::new(info(), None));
    h.run();
    let _ = h.get_by_label_contains("No project is open");
}
