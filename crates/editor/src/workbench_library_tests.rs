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
    let (mut w, crate_id, rock_id) = with_a_project();
    // An earlier drop that never arrived (its import failed, say) doesn't
    // send a later placing to the old drop point.
    w.place_dropped(rock_id, Vec3::ONE, None);
    w.placing_failed(rock_id);
    assert!(w.take_place_target(rock_id).is_none());
    w.place_dropped(crate_id, Vec3::ONE, None);
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
    // The click placed it where the camera looks, not at the old drop.
    assert!(h.state_mut().0.take_place_target(crate_id).is_none());
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

/// A workbench with a project holding one sprite sheet, `walker.aseprite`
/// (the test sheet: two frames, a "feet" slice), read and shown.
fn with_a_sprite_sheet() -> (Workbench, AssetId) {
    use crate::sprite_sheets::{SheetState, SpriteSheets};
    let (mut w, _, _) = with_a_project();
    let id = AssetId::random();
    w.set_library(vec![item(
        "walker.aseprite",
        LibraryKind::SpriteSheet,
        Some(id),
    )]);
    let mut sheets = SpriteSheets::default();
    sheets.by_path.insert(
        "walker.aseprite".into(),
        SheetState::Read(std::sync::Arc::new(
            crate::sprite_sheets::tests::test_sheet(),
        )),
    );
    sheets.paths.insert(id, "walker.aseprite".into());
    w.set_sprite_sheets(sheets);
    (w, id)
}

#[test]
fn a_sprite_sheet_dropped_on_the_viewport_becomes_a_sprite_on_a_whole_pixel() {
    let (w, id) = with_a_sprite_sheet();
    let mut h = harness(w);
    h.run();
    let point = Vec3::new(3.03, 4.0, 0.0);
    h.state_mut().1.answer = Some(PickAnswer {
        point: Some(point),
        ..PickAnswer::default()
    });
    drag(&mut h, "walker", IN_VIEW);
    h.run();
    // Placed at once: no model to load first.
    assert!(
        !h.state()
            .1
            .actions
            .iter()
            .any(|a| matches!(a, WorkbenchAction::PlaceAsset(_)))
    );
    let w = &h.state().0;
    let placed = w.selection().expect("selected");
    let entity = w.document.area().get(placed).unwrap();
    assert_eq!(entity.name, "walker");
    let t: Transform = entity.get().unwrap();
    let pixels = t.position * 16.0;
    assert!((pixels.x - pixels.x.round()).abs() < 1e-4, "{}", t.position);
    assert!((t.position.x - 3.03).abs() < 1.0 / 16.0);
    assert_eq!(t.position.y, 4.0);
    let sprite: area::builtin::Sprite = entity.get().unwrap();
    assert_eq!(sprite.sheet.map(|r| r.asset), Some(id));
    assert_eq!(sprite.pivot, "feet", "stands on its feet");
    assert!(!sprite.flip_x, "as drawn");
    // One undo step takes it away.
    h.state_mut().0.undo();
    assert!(h.state().0.document.area().get(placed).is_none());
}

#[test]
fn a_sprite_sheet_shows_its_frames_in_the_library_and_properties() {
    let (w, _) = with_a_sprite_sheet();
    let mut h = harness(w);
    h.run();
    assert_eq!(
        crate::library::frames_text(2),
        "2 frames",
        "the tile's badge"
    );
    assert_eq!(crate::library::frames_text(1), "1 frame");
    h.get_by_label("walker").click();
    h.run();
    h.run();
    for label in ["Frame 1", "Frame 2", "16 × 24 pixels"] {
        assert!(h.query_by_label(label).is_some(), "{label}");
    }
    assert!(
        h.query_by_label_contains("No tags: in Aseprite").is_some(),
        "says how to make clips"
    );
    assert!(h.query_by_label_contains("Slices: feet (pivot)").is_some());
    // Clicking a frame in the strip shows it.
    h.get_by_label("Frame 2").click();
    h.run();
    assert!(h.query_by_label_contains("Frame 2 of 2").is_some());
}

#[test]
fn a_sprite_s_sheet_and_pivot_are_picked_from_lists() {
    let (mut w, id) = with_a_sprite_sheet();
    let placed = w.place_sprite(id, None).expect("placed");
    let mut h = harness(w);
    h.run();
    h.run();
    fn open_combo(h: &egui_kittest::Harness<'_, State>, shows: &str) {
        use egui_kittest::kittest::NodeT;
        let combos: Vec<_> = h.get_all_by_role(egui::accesskit::Role::ComboBox).collect();
        let seen: Vec<(Option<String>, Option<String>)> = combos
            .iter()
            .map(|n| (n.accesskit_node().label(), n.value()))
            .collect();
        let combo = combos.iter().find(|n| {
            n.accesskit_node().label().as_deref() == Some(shows)
                || n.value().as_deref() == Some(shows)
        });
        assert!(combo.is_some(), "no list showing {shows}: {seen:?}");
        if let Some(c) = combo {
            c.click();
        }
    }
    // The pivot: the sheet's slices, and the bottom middle.
    open_combo(&h, "feet");
    h.run();
    h.get_by_label("Bottom middle").click();
    h.run();
    let sprite = |h: &egui_kittest::Harness<'_, State>| -> area::builtin::Sprite {
        h.state()
            .0
            .document
            .area()
            .get(placed)
            .unwrap()
            .get()
            .unwrap()
    };
    assert_eq!(sprite(&h).pivot, "");
    // The sheet: every sheet read, by name, and None.
    open_combo(&h, "walker");
    h.run();
    h.get_by_label("None").click();
    h.run();
    assert_eq!(sprite(&h).sheet, None);
}
