//! Home's level editor as the design's first screen draws it (milestone
//! M5): the toolbox, the pills, the snapping steps on the ribbon, the
//! cards, the Assets column and the Console's rows.

use crate::console::LogLevel;
use crate::library::{LibraryItem, LibraryKind, ProjectInfo};
use crate::panel::Panel;
use crate::properties_cards::across;
use crate::shell::status::snap_words;
use crate::test_support::*;
use crate::tokens::size;
use crate::transforming::{PixelSnap, Snapping};
use crate::workbench::{Workbench, WorkbenchAction};
use egui_kittest::kittest::{By, NodeT as _, Queryable};

#[test]
fn as_many_cards_go_across_as_fit_at_their_least_width() {
    let least = size::CARD_MIN_W;
    let gap = size::CARD_GAP;
    assert_eq!(across(920.0, 4), 4, "the design's four Light card row");
    assert_eq!(across(920.0, 3), 3, "never more than there are");
    assert_eq!(across(2.0 * least + gap, 5), 2);
    assert_eq!(across(2.0 * least + gap - 1.0, 5), 1, "a point short");
    assert_eq!(across(50.0, 3), 1, "one, however narrow");
    assert_eq!(across(920.0, 0), 1);
}

#[test]
fn the_status_bar_says_what_moves_snap_to() {
    let mut snapping = Snapping {
        grid: true,
        grid_step: 0.25,
        ..Snapping::default()
    };
    assert_eq!(snap_words(&snapping, false), "Snap 0.25 \u{b7} World");
    snapping.grid = false;
    snapping.object_axes = true;
    assert_eq!(snap_words(&snapping, false), "Snap off \u{b7} Object");
    snapping.pixel_snap = PixelSnap::Tiles;
    assert_eq!(snap_words(&snapping, true), "Snap tiles");
}

#[test]
fn the_toolbox_runs_down_the_viewport_s_left_edge_at_the_design_s_size() {
    let (w, _) = with_a_crate();
    let mut h = harness(w);
    h.run();
    h.run();
    let viewport = h.state().0.shell.drawn_at(Panel::Viewport).unwrap();
    let mut top = 0.0;
    for name in [
        "Select (Q)",
        "Move (W)",
        "Rotate (R)",
        "Scale (S)",
        "Transform (T)",
    ] {
        let tool = h.get_by_label(name).rect();
        assert_eq!(
            [tool.width(), tool.height()],
            [size::TOOL_BUTTON; 2],
            "{name}"
        );
        assert_eq!(
            tool.center().x,
            viewport.left() + size::TOOLBOX_W / 2.0,
            "{name}: in the column's middle"
        );
        assert!(tool.top() > top, "{name}: under the one before");
        top = tool.top();
    }
    // The views start right of the column.
    assert_eq!(
        h.state().0.viewport.full.left(),
        viewport.left() + size::TOOLBOX_W
    );
    // A click on a tool picks it.
    h.get_by_label("Rotate (R)").click();
    h.run();
    assert_eq!(h.state().0.tool(), crate::viewport::Tool::Rotate);
}

#[test]
fn the_ribbon_chooses_snapping_steps_from_the_value() {
    let (w, _) = with_a_crate();
    let mut h = harness(w);
    h.run();
    h.run();
    let row = h.get_by_label("Grid · 1").rect();
    // The value, after the "·": its list, and no switching.
    let on_value = egui::pos2(row.right() - size::RIBBON_ROW_PAD - 2.0, row.center().y);
    click(&mut h, on_value, egui::Modifiers::NONE);
    h.run();
    assert!(!h.state().0.snapping().grid, "the box wasn't switched");
    h.get_by_label("0.25 m").click();
    h.run();
    h.run();
    assert_eq!(h.state().0.snapping().grid_step, 0.25);
    // A right-click opens the angle's.
    h.get_by_label("Angle · 15°").click_secondary();
    h.run();
    h.get_by_label("45°").click();
    h.run();
    assert_eq!(h.state().0.snapping().angle_step, 45.0);
    assert!(
        h.query_by_label_contains("Snap off").is_some(),
        "the status bar"
    );
}

#[test]
fn the_model_card_shows_its_rows_to_come_unless_they_are_hidden() {
    let (mut w, id) = with_a_crate();
    w.open.document.selection.select_only(id);
    roomy_shelf(&mut w);
    let mut h = harness(w);
    h.run();
    h.run();
    let unbuilt: Vec<String> = h
        .query_all(
            By::new().predicate(|node| {
                node.description().as_deref() == Some(crate::unbuilt::DESCRIPTION)
            }),
        )
        .map(|node| node.accesskit_node().label().unwrap_or_default())
        .collect();
    for name in ["Material", "Shadows"] {
        assert!(unbuilt.iter().any(|n| n == name), "{name} in {unbuilt:?}");
    }
    run(&mut h, "Hide unfinished parts");
    h.run();
    assert!(h.query_by_label("Material").is_none());
    assert!(h.query_by_label("Shadows").is_none());
}

/// Home as the design's first screen draws it: the Assets column on the
/// shelf.
fn with_files() -> Workbench {
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
        item("deep", "props/deep.glb", LibraryKind::Model),
    ]);
    w
}

#[test]
fn the_assets_column_shows_the_open_folder_and_imports() {
    let mut h = harness(with_files());
    h.run();
    h.run();
    assert!(in_view(&h.state().0, Panel::Assets));
    let column = h.state().0.shell.drawn_at(Panel::Assets).unwrap();
    for name in ["crate", "creak", "Import a model"] {
        let tile = h.get_by_label(name).rect();
        assert!(column.contains(tile.center()), "{name} in {column:?}");
    }
    assert!(h.query_by_label("deep").is_none(), "another folder's");
    let _ = h.get_by_label("Assets");
    // Four across, square.
    let crate_tile = h.get_by_label("crate").rect();
    let gap = size::TILE_GAP;
    let width = ((column.width() - 4.0 * gap - 3.0 * gap) / 4.0).floor();
    assert_eq!(crate_tile.width(), width);
    h.get_by_label("Import a model").click();
    h.run();
    assert!(h.state().1.actions.contains(&WorkbenchAction::ImportModel));
    // A click picks it.
    h.get_by_label("crate").click();
    h.run();
    assert_eq!(
        h.state().0.open.library_state.selected.as_deref(),
        Some("crate.glb")
    );
}

#[test]
fn the_console_s_lines_say_their_level_as_a_word() {
    let mut w = Workbench::new(info());
    w.log(LogLevel::Error, "Target was destroyed");
    w.log(LogLevel::Warning, "Lighting is out of date");
    w.log(LogLevel::Info, "Saved");
    let mut h = harness(w);
    h.run();
    h.get_by_label("Console").click();
    h.run();
    for word in ["Target was destroyed", "Lighting is out of date", "Saved"] {
        assert!(h.query_by_label(word).is_some(), "{word}");
    }
}

/// The selected point light's strength.
fn strength(w: &Workbench, id: reflect::EntityId) -> f64 {
    w.open.document.area().get(id).unwrap().components["Light"]
        .get("strength")
        .and_then(reflect::Value::as_f64)
        .unwrap()
}

#[test]
fn dragging_a_number_s_name_changes_it_as_one_undo_step() {
    let mut w = Workbench::new(info());
    roomy_shelf(&mut w);
    let id = w.add_point_light();
    let before = strength(&w, id);
    let steps = w.open.document.history().undo_len();
    let mut h = harness(w);
    h.run();
    h.run();
    let name = h.get_all_by_label("Strength").next().unwrap().rect();
    drag_for_frames(&mut h, name.center(), egui::vec2(10.0, 0.0), 3);
    let after = strength(&h.state().0, id);
    assert!(after > before, "{before} became {after}");
    assert_eq!(
        h.state().0.open.document.history().undo_len(),
        steps + 1,
        "the whole drag is one step"
    );
    // Leftwards lowers it, never under its least.
    let name = h.get_all_by_label("Strength").next().unwrap().rect();
    drag_for_frames(&mut h, name.center(), egui::vec2(-4000.0, 0.0), 3);
    assert_eq!(strength(&h.state().0, id), 0.0);
    h.state_mut().0.undo();
    h.state_mut().0.undo();
    assert_eq!(strength(&h.state().0, id), before, "undone");
}

#[test]
fn a_name_that_is_not_a_number_does_not_drag() {
    let mut w = Workbench::new(info());
    roomy_shelf(&mut w);
    let id = w.add_point_light();
    let before = w.open.document.area().get(id).cloned();
    let mut h = harness(w);
    h.run();
    h.run();
    let name = h.get_all_by_label("Shadows").next().unwrap().rect();
    drag_for_frames(&mut h, name.center(), egui::vec2(20.0, 0.0), 3);
    assert_eq!(h.state().0.open.document.area().get(id).cloned(), before);
}

#[test]
fn a_field_changed_while_playing_is_marked_and_can_go_back() {
    let mut w = Workbench::new(info());
    roomy_shelf(&mut w);
    let id = w.add_point_light();
    let before = strength(&w, id);
    w.play();
    w.apply_property(
        crate::properties::PropertyRequest::SetField {
            entity: id,
            component: "Light".into(),
            field: "strength".into(),
            value: reflect::Value::Float(before + 50.0),
        },
        1,
    );
    let mut h = harness(w);
    h.run();
    h.run();
    assert!(h.query_by_label_contains("1 change so far").is_some());
    let back = h.get_by_label("Put back Strength");
    assert!(back.rect().width() > 0.0);
    back.click();
    h.run();
    assert_eq!(strength(&h.state().0, id), before, "back to before Play");
    h.run();
    assert!(
        h.query_by_label("Put back Strength").is_none(),
        "no mark once it's its value from before again"
    );
}

#[test]
fn f2_and_delete_over_the_assets_act_on_the_picked_file() {
    let mut w = with_files();
    let entity = w.add_point_light();
    let mut h = rest_over(harness(w), Panel::Assets);
    h.get_by_label("crate").click();
    h.run();
    let body = body_of(&h, Panel::Assets);
    h.hover_at(body.center());
    h.run();
    h.key_press(egui::Key::F2);
    h.run();
    assert_eq!(
        h.state().0.open.library_state.renaming,
        Some(("crate.glb".to_string(), "crate.glb".to_string()))
    );
    h.state_mut().0.open.library_state.renaming = None;
    h.run();
    h.key_press(egui::Key::Delete);
    h.run();
    assert!(
        h.state()
            .1
            .actions
            .contains(&WorkbenchAction::DeleteFile("crate.glb".into()))
    );
    assert!(
        h.state().0.open.document.area().contains(entity),
        "the entity stays"
    );
    // Elsewhere Delete is the entity's still.
    let viewport = body_of(&h, Panel::Viewport);
    h.hover_at(viewport.center());
    h.run();
    h.run();
    h.key_press(egui::Key::Delete);
    h.run();
    assert!(!h.state().0.open.document.area().contains(entity));
}

#[test]
fn problems_are_rows_with_where_in_grey() {
    let mut w = Workbench::new(info());
    w.set_code_problems(
        vec![script::CodeProblem {
            file: "crane/crane.lst".into(),
            line: 84,
            column: 9,
            message: "Target was destroyed".into(),
        }],
        &[],
    );
    let mut h = harness(w);
    h.run();
    h.get_by_label("Problems").click();
    h.run();
    let row = h.get_by_label("crane/crane.lst, line 84:9: Target was destroyed");
    assert_eq!(row.rect().height(), size::PROBLEM_ROW_H);
}
