//! The Area panel's search, folding, hiding and locking, its menus, and
//! Properties with several entities selected.

use crate::area_panel::AreaRequest;
use crate::area_view::AreaView;
use crate::menu_items::MenuCommand;
use crate::properties::PropertyRequest;
use crate::test_support::*;
use crate::viewport_tools::PickAnswer;
use crate::workbench::Workbench;
use area::EntityData;
use area::builtin::{Light, Transform};
use egui_kittest::kittest::Queryable;
use glam::{Quat, Vec3};
use reflect::{EntityId, Value};

/// A workbench with a room holding a lamp, and a crate beside it.
fn a_room() -> (Workbench, EntityId, EntityId, EntityId) {
    let (mut w, crate_id) = with_a_crate();
    let room = w.add_entity();
    w.document.selection.select_only(room);
    let lamp = w.add_point_light();
    w.apply_area_request(AreaRequest::Reparent {
        entity: lamp,
        onto: Some(room),
    });
    w.document.selection.clear();
    (w, room, lamp, crate_id)
}

/// Whether the Area panel lists a row called `name`.
fn listed(h: &egui_kittest::Harness<'_, State>, name: &str) -> bool {
    h.query_all_by_label(name).next().is_some()
}

#[test]
fn the_search_box_lists_matches_with_their_parents() {
    let (mut w, room, _, _) = a_room();
    let room_name = w.document.area().get(room).unwrap().name.clone();
    w.area_panel.search = "point".into();
    let mut h = harness(w);
    h.run();
    assert!(listed(&h, "Point light 1"), "the match");
    assert!(listed(&h, &room_name), "its parent");
    assert!(!listed(&h, "Crate"), "not the rest");
    h.state_mut().0.area_panel.search = "nothing like it".into();
    h.run();
    let _ = h.get_by_label("Nothing has that name.");
}

#[test]
fn folding_hides_children_and_the_view_menu_unfolds_everything() {
    let (w, room, _, _) = a_room();
    let mut h = harness(w);
    h.run();
    assert!(listed(&h, "Point light 1"));
    h.get_by_label("Fold").click();
    h.run();
    assert!(!listed(&h, "Point light 1"), "folded away");
    assert!(h.state().0.area_panel.collapsed.contains(&room));
    h.state_mut()
        .0
        .run(MenuCommand::ExpandAll, &egui::Context::default());
    h.run();
    assert!(listed(&h, "Point light 1"));
    h.state_mut()
        .0
        .run(MenuCommand::CollapseAll, &egui::Context::default());
    h.run();
    assert!(!listed(&h, "Point light 1"));
}

#[test]
fn picking_something_folded_away_unfolds_the_way_to_it() {
    let (mut w, room, lamp, _) = a_room();
    w.area_panel.collapsed.insert(room);
    let at = w.document.area().world_matrix(lamp).w_axis.truncate();
    w.viewport.camera.look_at(at);
    let mut h = harness(w);
    h.run();
    let n = h
        .state()
        .0
        .viewport
        .area
        .lamps
        .iter()
        .position(|l| l.entity == lamp);
    assert!(n.is_some());
    // A box around the lamp's marker (it has no surface to pick).
    h.state_mut().1.answer = Some(PickAnswer::default());
    let marker = {
        let w = &h.state().0;
        let rect = w.viewport.rect;
        let size = glam::Vec2::new(rect.width(), rect.height());
        let p = w
            .viewport
            .camera
            .project(w.viewport.area.lamps[n.unwrap()].position, size)
            .unwrap();
        rect.min + egui::vec2(p.x, p.y)
    };
    click(&mut h, marker, egui::Modifiers::NONE);
    h.run();
    assert_eq!(h.state().0.selection(), Some(lamp));
    assert!(
        !h.state().0.area_panel.collapsed.contains(&room),
        "unfolded"
    );
    assert!(listed(&h, "Point light 1"));
}

#[test]
fn hidden_entities_and_their_children_are_not_drawn_and_locked_ones_not_picked() {
    let (mut w, room, lamp, crate_id) = a_room();
    let view = |w: &Workbench| {
        let (hidden, locked) = w.area_panel.in_effect(&w.document);
        AreaView::of_shown(w.document.area(), &w.models, &hidden, &locked)
    };
    assert_eq!(view(&w).lamps.len(), 1);
    w.area_panel.hidden.insert(room);
    let v = view(&w);
    assert!(v.lamps.is_empty(), "the lamp is under the hidden room");
    assert_eq!(v.models.len(), 1, "the crate is still drawn");
    w.area_panel.hidden.clear();
    w.area_panel.locked.insert(crate_id);
    let v = view(&w);
    assert_eq!(v.models.len(), 1, "a locked crate is still drawn…");
    assert_eq!(v.models[0].pick, 0, "…but can't be picked");
    assert!(v.pickable.is_empty());
    w.area_panel.locked.insert(lamp);
    assert!(view(&w).locked.contains(&lamp));
}

#[test]
fn the_hide_and_lock_switches_appear_on_the_row_and_switch() {
    let (w, _, _, crate_id) = a_room();
    let mut h = harness(w);
    h.hover_at(egui::pos2(1200.0, 160.0));
    h.run();
    h.get_by_label("Hide Crate").click();
    h.run();
    assert!(h.state().0.area_panel.hidden.contains(&crate_id));
    let steps = h.state().0.document.history().undo_len();
    h.get_by_label("Lock Crate").click();
    h.run();
    assert!(h.state().0.area_panel.locked.contains(&crate_id));
    // Neither is an undo step.
    assert_eq!(h.state().0.document.history().undo_len(), steps);
}

#[test]
fn the_area_panels_menus_are_the_top_bars() {
    let (mut w, _, _, crate_id) = a_room();
    w.document.selection.select_only(crate_id);
    let mut h = harness(w);
    h.run();
    let before = h.state().0.document.area().len();
    h.state_mut()
        .0
        .apply_area_request(AreaRequest::Command(MenuCommand::Duplicate));
    // Commands are run by the workbench where it has the window.
    h.state_mut()
        .0
        .run(MenuCommand::Duplicate, &egui::Context::default());
    h.run();
    assert_eq!(h.state().0.document.area().len(), before + 1);
    h.state_mut()
        .0
        .run(MenuCommand::SelectAll, &egui::Context::default());
    assert_eq!(
        h.state().0.document.selection.iter().count(),
        h.state().0.document.area().len()
    );
    h.state_mut()
        .0
        .run(MenuCommand::SelectNone, &egui::Context::default());
    assert_eq!(h.state().0.selection(), None);
}

/// Two crates, at different places, both selected.
fn two_crates() -> (Workbench, EntityId, EntityId) {
    let (mut w, a) = with_a_crate();
    let b = w.add_model(crate_model(), &[], 0.1, &[]);
    for (id, x) in [(a, 1.0), (b, 5.0)] {
        w.apply_property(
            PropertyRequest::SetField {
                entity: id,
                component: "Transform".into(),
                field: "position".into(),
                value: Value::vec3(Vec3::new(x, x, 0.0)),
            },
            id.0.raw(),
        );
    }
    w.document.selection.select_only(a);
    w.document.selection.toggle(b);
    w.document.selection.toggle(a);
    w.document.selection.toggle(a);
    (w, a, b)
}

fn transform(w: &Workbench, id: EntityId) -> Transform {
    w.document
        .area()
        .get(id)
        .unwrap()
        .get::<Transform>()
        .unwrap()
}

#[test]
fn changing_one_number_on_several_changes_only_that_number_on_each() {
    let (mut w, a, b) = two_crates();
    let steps = w.document.history().undo_len();
    w.apply_property(
        PropertyRequest::SetFieldMany {
            entities: vec![a, b],
            component: "Transform".into(),
            field: "position".into(),
            value: Value::vec3(Vec3::new(1.0, 1.0, 3.0)),
            part: Some((2, 3.0)),
        },
        99,
    );
    assert_eq!(transform(&w, a).position, Vec3::new(1.0, 1.0, 3.0));
    assert_eq!(
        transform(&w, b).position,
        Vec3::new(5.0, 5.0, 3.0),
        "only Z"
    );
    assert_eq!(w.document.history().undo_len(), steps + 1, "one step");
    w.undo();
    assert_eq!(transform(&w, b).position, Vec3::new(5.0, 5.0, 0.0));
    assert_eq!(transform(&w, a).position, Vec3::new(1.0, 1.0, 0.0));
}

#[test]
fn turning_several_around_one_axis_keeps_their_other_angles() {
    let (mut w, a, b) = two_crates();
    w.apply_property(
        PropertyRequest::SetField {
            entity: b,
            component: "Transform".into(),
            field: "rotation".into(),
            value: crate::inspector::rotation_of([10.0, 0.0, 0.0]),
        },
        7,
    );
    w.apply_property(
        PropertyRequest::SetFieldMany {
            entities: vec![a, b],
            component: "Transform".into(),
            field: "rotation".into(),
            value: crate::inspector::rotation_of([0.0, 0.0, 90.0]),
            part: Some((2, 90.0)),
        },
        8,
    );
    let degrees = |id| crate::inspector::degrees_of(&Value::quat(transform(&w, id).rotation));
    let [ax, _, az] = degrees(a);
    let [bx, _, bz] = degrees(b);
    assert!(ax.abs() < 1e-3 && (az - 90.0).abs() < 1e-3);
    assert!((bx - 10.0).abs() < 1e-3, "kept its tilt: {bx}");
    assert!((bz - 90.0).abs() < 1e-3);
    let _ = Quat::IDENTITY;
}

#[test]
fn properties_shows_what_several_share_with_differences_marked() {
    let (mut w, a, b) = two_crates();
    w.document.selection.select_only(a);
    w.document.selection.toggle(b);
    w.document.selection.toggle(a);
    w.document.selection.toggle(a);
    let mut h = harness(w);
    h.run();
    let _ = h.get_by_label_contains("2 entities selected");
    let _ = h.get_by_label("Transform");
    // The Name box is only there for one entity.
    assert_eq!(h.query_all_by_label("Name").count(), 0);
    // The positions differ in X and Y but not Z.
    let schema = area::builtin::registry().unwrap();
    let field = schema.get("Transform").unwrap().fields[0].clone();
    let data_b = h.state().0.document.area().get(b).unwrap().components["Transform"].clone();
    let data_a = h.state().0.document.area().get(a).unwrap().components["Transform"].clone();
    let mixed = crate::inspector::Mixed::of(&field, data_a.get("position").unwrap(), &[&data_b]);
    assert!(mixed.whole);
    assert_eq!(mixed.parts, [true, true, false]);
}

#[test]
fn copying_and_pasting_values_adds_what_is_missing_and_reset_puts_defaults_back() {
    let (w, _, lamp, crate_id) = a_room();
    let mut w = w;
    w.apply_property(
        PropertyRequest::CopyComponents {
            entity: lamp,
            only: Some("Light".into()),
        },
        1,
    );
    w.apply_property(
        PropertyRequest::PasteComponents {
            entities: vec![crate_id],
            only: None,
        },
        2,
    );
    let pasted: Light = w.document.area().get(crate_id).unwrap().get().unwrap();
    let original: Light = w.document.area().get(lamp).unwrap().get().unwrap();
    assert_eq!(pasted, original, "the crate glows like the lamp");
    assert_eq!(w.document.history().undo_label(), Some("Paste components"));
    w.apply_property(
        PropertyRequest::SetField {
            entity: crate_id,
            component: "Light".into(),
            field: "strength".into(),
            value: Value::Float(1234.0),
        },
        3,
    );
    w.apply_property(
        PropertyRequest::ResetComponent {
            entities: vec![crate_id],
            component: "Light".into(),
        },
        4,
    );
    let reset: Light = w.document.area().get(crate_id).unwrap().get().unwrap();
    assert_eq!(reset, Light::default());
    w.undo();
    let back: Light = w.document.area().get(crate_id).unwrap().get().unwrap();
    assert_eq!(back.strength, 1234.0);
}

#[test]
fn components_are_added_to_and_removed_from_several_at_once() {
    let (mut w, a, b) = two_crates();
    let mut lit = EntityData::new("Lit");
    lit.set(&Light::default());
    w.apply_property(
        PropertyRequest::AddComponentMany {
            entities: vec![a, b],
            component: "Light".into(),
        },
        1,
    );
    assert!(w.document.area().get(a).unwrap().has("Light"));
    assert!(w.document.area().get(b).unwrap().has("Light"));
    assert_eq!(w.document.history().undo_label(), Some("Add Light"));
    w.apply_property(
        PropertyRequest::RemoveComponentMany {
            entities: vec![a, b],
            component: "Light".into(),
        },
        2,
    );
    assert!(!w.document.area().get(a).unwrap().has("Light"));
    w.undo();
    assert!(
        w.document.area().get(b).unwrap().has("Light"),
        "one step back"
    );
    assert!(w.document.area().get(a).unwrap().has("Light"));
    let _ = lit;
}

#[test]
fn the_top_bars_edit_menu_has_the_selection_commands() {
    let (w, _, _, _) = a_room();
    let mut h = harness(w);
    h.run();
    // The top bar's and the Area panel's Edit menus have the same items.
    let menus = h.query_all_by_label("Edit").count();
    let mut found = 0;
    for i in 0..menus {
        h.query_all_by_label("Edit").nth(i).unwrap().click();
        h.run();
        if h.query_by_label_contains("Select nothing").is_some() {
            found += 1;
            for item in ["Duplicate", "Delete", "Rename", "Select all"] {
                assert!(h.query_by_label_contains(item).is_some(), "{item}");
            }
        }
        h.key_press(egui::Key::Escape);
        h.run();
    }
    assert!(found >= 2, "in the top bar and the Area panel: {found}");
}

#[test]
fn the_problems_panel_lists_mistakes_and_a_click_selects_the_entity() {
    let mut area = area::builtin::starter_area();
    let odd = EntityId::random();
    let mut data = EntityData::new("Odd one");
    data.components
        .insert("Wobble".into(), reflect::ComponentData::default());
    area.insert(odd, data);
    let mut w = Workbench::new(info(), None);
    w.open_area(area, "odd.solarea".into());
    let mut h = harness(w);
    h.run();
    assert_eq!(h.state().0.problems().len(), 1);
    h.get_by_label("Problems").click();
    h.run();
    let row = "Odd one has a Wobble component that isn't known; it's kept, but does nothing";
    h.get_by_label(row).click();
    h.run();
    assert_eq!(h.state().0.document.selection.primary(), Some(odd));
    // Fixed (the entity deleted): the problem goes.
    h.state_mut().0.delete_selection();
    h.run();
    assert!(h.state().0.problems().is_empty());
    let _ = h.get_by_label("No problems found in the area or the game's code.");
}

#[test]
fn mistakes_in_the_game_code_are_listed_first_and_a_click_opens_the_file() {
    let mut w = Workbench::new(info(), None);
    w.set_project(Some(crate::library::ProjectInfo {
        name: "Harbour".into(),
        root: "/p".into(),
        assets: "/p/assets".into(),
    }));
    let mistake = script::CodeProblem {
        file: "weapons/gun.lst".into(),
        line: 12,
        column: 5,
        message: "ammo is an int, but this is text.".into(),
    };
    w.set_code_problems(
        vec![mistake.clone()],
        &["huge.lst is left out.".to_string()],
    );
    assert_eq!(w.code_problems(), std::slice::from_ref(&mistake));
    let mut h = harness(w);
    h.run();
    h.get_by_label("Problems").click();
    h.run();
    let _ = h.get_by_label_contains("1 problem in the game's code");
    h.get_by_label("weapons/gun.lst, line 12:5: ammo is an int, but this is text.")
        .click();
    h.run();
    assert!(
        h.state()
            .1
            .actions
            .contains(&crate::workbench::WorkbenchAction::OpenExternally(
                "/p/assets/weapons/gun.lst".into()
            )),
        "{:?}",
        h.state().1.actions
    );
    // Fixed: the Console says so, and the list empties.
    h.state_mut().0.set_code_problems(Vec::new(), &[]);
    h.run();
    assert!(h.state().0.code_problems().is_empty());
    let _ = h.get_by_label("No problems found in the area or the game's code.");
    // Another project starts with none of the last one's.
    h.state_mut().0.set_code_problems(vec![mistake], &[]);
    h.state_mut().0.set_project(None);
    assert!(h.state().0.code_problems().is_empty());
}

#[test]
fn components_registered_by_game_code_survive_new_areas_and_projects() {
    // Found by review: each new or opened area started with only the
    // built-in components.
    let mut w = Workbench::new(info(), None);
    let schema = reflect::ComponentSchema {
        name: "Wobble".into(),
        help: "Wobbles.".into(),
        fields: Vec::new(),
    };
    w.register_component(schema).unwrap();
    w.new_area();
    assert!(w.document.registry.get("Wobble").is_some());
    w.open_area(area::builtin::starter_area(), "a.solarea".into());
    assert!(w.document.registry.get("Wobble").is_some());
    w.set_project(Some(crate::library::ProjectInfo {
        name: "Harbour".into(),
        root: "/p".into(),
        assets: "/p/assets".into(),
    }));
    assert!(w.document.registry.get("Wobble").is_some());
}

#[test]
fn another_project_starts_without_the_last_ones_assemblies_and_questions() {
    let (mut w, asset, _, _) = with_a_lamp_post();
    w.pending_reload = Some(area::builtin::starter_area());
    w.place_targets.insert(asset, (glam::Vec3::ONE, None));
    w.set_project(Some(crate::library::ProjectInfo {
        name: "Copy of Harbour".into(),
        root: "/copy".into(),
        assets: "/copy/assets".into(),
    }));
    assert!(w.assemblies().is_empty());
    assert!(w.place_targets.is_empty());
    assert!(w.pending_reload.is_none());
}

#[test]
fn the_open_area_follows_its_file_when_the_library_moves_or_deletes_it() {
    use std::path::{Path, PathBuf};
    let mut w = Workbench::new(info(), None);
    w.open_area(
        area::builtin::starter_area(),
        "/p/assets/areas/harbour.solarea".into(),
    );
    // Another file: nothing changes.
    assert_eq!(
        w.file_moved(
            Path::new("/p/assets/crate.glb"),
            Some(Path::new("/p/assets/box.glb"))
        ),
        None
    );
    // Renamed.
    let renamed = PathBuf::from("/p/assets/areas/port.solarea");
    assert_eq!(
        w.file_moved(Path::new("/p/assets/areas/harbour.solarea"), Some(&renamed)),
        Some(Some(renamed.clone()))
    );
    assert_eq!(w.document.file(), Some(renamed.as_path()));
    assert!(!w.document.is_dirty(), "the file still holds it");
    // Its folder moved.
    assert_eq!(
        w.file_moved(
            Path::new("/p/assets/areas"),
            Some(Path::new("/p/assets/levels"))
        ),
        Some(Some(PathBuf::from("/p/assets/levels/port.solarea")))
    );
    // Deleted: unsaved, and saving asks where.
    assert_eq!(
        w.file_moved(Path::new("/p/assets/levels/port.solarea"), None),
        Some(None)
    );
    assert_eq!(w.document.file(), None);
    assert!(w.document.is_dirty());
}
