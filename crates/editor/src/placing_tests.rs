//! Placing (milestone M6; spec 236-287, round 2 `06-placing-a.png`): the
//! Place shelf tab is the mode, the ribbon takes its build form, the
//! toolbox goes, and a file dragged in shows where it lands.

use crate::library::{LibraryItem, LibraryKind, ProjectInfo};
use crate::panel::Panel;
use crate::test_support::*;
use crate::tokens::size;
use crate::viewport::Tool;
use crate::workbench::Workbench;
use egui_kittest::kittest::Queryable;

/// Placing as the design's Screen 2 draws it, on the shelf (the other
/// layouts' placing is in `shell/layout_tests.rs`).
fn with_models() -> Workbench {
    let mut w = crate::test_support::in_layout(crate::shell::PanelLayout::Shelf);
    w.set_project(Some(ProjectInfo {
        name: "Harbour".into(),
        root: "/projects/harbour".into(),
        assets: "/projects/harbour/assets".into(),
    }));
    let item = |name: &str, path: &str, kind| LibraryItem {
        id: Some(reflect::AssetId(reflect::Id::random())),
        kind,
        name: name.into(),
        path: path.into(),
        thumbnail: None,
    };
    w.set_library(vec![
        item("crate", "crate.glb", LibraryKind::Model),
        item("creak", "creak.wav", LibraryKind::Sound),
        item("lamp", "lamp.solasm", LibraryKind::Assembly),
    ]);
    w
}

/// The editor with the Place shelf tab brought to the front.
fn placing() -> egui_kittest::Harness<'static, State> {
    let mut h = harness(with_models());
    h.run();
    h.get_by_label("Place").click();
    h.run();
    h.run();
    h
}

#[test]
fn home_opens_on_entity_and_place_is_the_first_shelf_tab() {
    let mut h = harness(with_models());
    h.run();
    h.run();
    assert!(!h.state().0.placing());
    let place = h.get_by_label("Place").rect();
    let entity = h.get_by_label("Entity").rect();
    assert!(place.left() < entity.left(), "Place comes first");
    assert!(
        h.query_by_label("Select (Q)").is_some(),
        "the level editor's toolbox"
    );
}

#[test]
fn place_in_front_is_the_mode_the_ribbon_and_toolbox_follow() {
    let mut h = placing();
    assert!(h.state().0.placing());
    // The build form: Place for Create, Transform with the tools, no
    // toolbox and no Lighting.
    assert!(h.query_by_label("Select (Q)").is_none(), "no toolbox");
    for name in ["Select tool", "Move tool", "All three tool"] {
        assert!(h.query_by_label(name).is_some(), "{name} on the ribbon");
    }
    assert!(h.query_by_label("Bake").is_none());
    h.get_by_label("Rotate tool").click();
    h.run();
    assert_eq!(h.state().0.tool(), Tool::Rotate);
    // The Placement column sits between the Area and the Assets.
    let area = h.state().0.shell.drawn_at(Panel::Area).unwrap();
    let placement = h.state().0.shell.drawn_at(Panel::Placement).unwrap();
    let assets = h.state().0.shell.drawn_at(Panel::Assets).unwrap();
    assert!(area.right() <= placement.left() && placement.right() <= assets.left());
    assert!((placement.width() - size::PLACEMENT_COLUMN_W).abs() <= 1.0);
    // Back to Entity: the level editor again.
    h.get_by_label("Entity").click();
    h.run();
    h.run();
    assert!(!h.state().0.placing());
    assert!(h.query_by_label("Select (Q)").is_some());
}

#[test]
fn the_place_tab_shows_what_can_be_placed() {
    let h = placing();
    assert!(h.query_by_label("crate").is_some());
    assert!(h.query_by_label("lamp").is_some());
    assert!(
        h.query_by_label("creak").is_none(),
        "a sound can't be placed"
    );
    assert!(h.query_by_label("Import a model").is_none());
}

#[test]
fn align_to_free_is_kept_and_open_library_goes_there() {
    let mut h = placing();
    assert!(!h.state().0.viewport.snapping.place_free);
    h.get_by_label("Free").click();
    h.run();
    assert!(h.state().0.viewport.snapping.place_free);
    h.get_by_label("Open Library").click();
    h.run();
    assert_eq!(h.state().0.shell.tab, crate::shell::RibbonTab::Library);
}

#[test]
fn with_unfinished_parts_hidden_placement_is_only_align() {
    let mut h = placing();
    for name in ["Rotation", "Random yaw", "Physics", "Brush", "Scatter"] {
        assert!(h.query_by_label(name).is_some(), "{name} shown");
    }
    run(&mut h, "Hide unfinished parts");
    h.run();
    for name in ["Rotation", "Random yaw", "Physics", "Brush", "Scatter"] {
        assert!(h.query_by_label(name).is_none(), "{name} hidden");
    }
    assert!(h.query_by_label("Grid").is_some());
    assert!(h.query_by_label("Free").is_some());
}

#[test]
fn the_status_bar_says_what_is_being_placed() {
    let mut snapping = crate::transforming::Snapping {
        grid: true,
        grid_step: 0.25,
        ..Default::default()
    };
    let words = |s: &crate::transforming::Snapping, ctrl| {
        crate::shell::status::placing_words("crate", s, ctrl)
    };
    assert_eq!(
        words(&snapping, false),
        "Placing crate \u{b7} Snap 0.25 \u{b7} Grid"
    );
    assert_eq!(words(&snapping, true), "Placing crate \u{b7} Free");
    snapping.place_free = true;
    assert_eq!(words(&snapping, false), "Placing crate \u{b7} Free");
}

/// The Place shelf tab with a model being dragged over the viewport, as
/// `06-placing-a.png` draws it: an approved picture of Solis's own.
#[test]
fn placing_matches_its_approved_picture() {
    use crate::picture_support::{drawn, editor_window, matches_approved, sample_project};
    let mut w = sample_project();
    w.open.library_state.folder = "props".into();
    w.viewport.snapping.grid = true;
    w.viewport.snapping.grid_step = 0.25;
    // Screen 2 as the design draws it: on the shelf.
    let look = crate::theme::InterfaceLook {
        layout: crate::shell::PanelLayout::Shelf,
        ..Default::default()
    };
    let Some(mut h) = editor_window(w, look, [1440.0, 900.0]) else {
        return;
    };
    h.run();
    h.run();
    h.get_by_label("Place").click();
    h.run();
    h.run();
    let from = h.get_by_label("crate").rect().center();
    h.drag_at(from);
    h.run();
    for step in 1..=6 {
        h.hover_at(from + egui::vec2(60.0, -50.0) * step as f32);
        h.run();
    }
    let picture = drawn(&mut h);
    matches_approved("placing", &picture);
}
