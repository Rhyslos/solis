//! Properties as values, not buttons (the editor overhaul's O2, decision
//! 0085): cards, the entity's ⋯ menu, a dot and a reset on a changed value,
//! "shared by", and searching for a component to add.

use super::*;
use crate::inspector::{moved_from_default, short, users_of};
use crate::test_support::*;
use crate::workbench::Workbench;
use egui_kittest::kittest::Queryable;
use reflect::{FieldInfo, FieldKind};

#[test]
fn only_values_worth_a_dot_get_one() {
    let float = FieldInfo::new("speed", "Speed", FieldKind::Float, Value::Float(3.0));
    assert!(!moved_from_default(&float, &Value::Float(3.0)));
    assert!(moved_from_default(&float, &Value::Float(3.5)));
    let on = FieldInfo::new("on", "On", FieldKind::Bool, Value::Bool(true));
    assert!(moved_from_default(&on, &Value::Bool(false)));
    // A place or a picked file starts empty and is meant to be set.
    let place = FieldInfo::new("p", "Position", FieldKind::Vec3, Value::List(vec![]));
    assert!(!moved_from_default(
        &place,
        &Value::List(vec![Value::Float(1.0); 3])
    ));
    let file = FieldInfo::new("a", "Sheet", FieldKind::Asset, Value::Asset(None));
    assert!(!moved_from_default(
        &file,
        &Value::Asset(Some(reflect::AssetRef {
            asset: AssetId::random(),
            part: None
        }))
    ));
}

#[test]
fn a_standard_value_is_shown_in_a_few_characters() {
    assert_eq!(short(&Value::Float(3.0)), "3");
    assert_eq!(short(&Value::Float(0.25)), "0.25");
    assert_eq!(short(&Value::Float(-0.0001)), "0");
    assert_eq!(short(&Value::Int(-4)), "-4");
    assert_eq!(short(&Value::Bool(true)), "on");
    assert_eq!(short(&Value::Text(String::new())), "empty");
    assert_eq!(
        short(&Value::Text("Walking slowly home".into())),
        "Walking slow…"
    );
}

#[test]
fn every_engine_component_has_a_manual_page_that_exists() {
    let registry = area::builtin::registry().unwrap();
    for schema in registry.iter() {
        let page = manual_page(&schema.name)
            .unwrap_or_else(|| panic!("{} has no manual page", schema.name));
        let path = format!("{}/../../docs/manual/{page}", env!("CARGO_MANIFEST_DIR"));
        assert!(std::path::Path::new(&path).is_file(), "{path}");
    }
    for kit in ["TopDownController", "FollowCamera2D", "CharacterController"] {
        assert!(manual_page(kit).is_some(), "{kit}");
    }
    assert_eq!(
        manual_page("Mover"),
        None,
        "your own code has its file instead"
    );
}

/// A workbench with a point light selected.
fn with_a_lamp() -> (Workbench, EntityId) {
    let mut w = Workbench::new(info());
    let id = w.add_point_light();
    w.select(id);
    // Room for the whole of the light's card and the component list.
    crate::test_support::roomy_shelf(&mut w);
    (w, id)
}

fn strength(w: &Workbench, id: EntityId) -> Option<Value> {
    w.document().area().get(id).unwrap().components["Light"]
        .get("strength")
        .cloned()
}

#[test]
fn a_changed_value_has_a_reset_to_its_standard_value() {
    let (w, id) = with_a_lamp();
    let standard = w
        .document()
        .registry
        .get("Light")
        .unwrap()
        .field("strength")
        .unwrap()
        .default
        .clone();
    let mut h = harness(w);
    h.run();
    assert!(
        h.query_by_label("Reset Strength").is_none(),
        "standard: no reset"
    );
    h.state_mut().0.apply_property(
        PropertyRequest::SetField {
            entity: id,
            component: "Light".into(),
            field: "strength".into(),
            value: Value::Float(123.0),
        },
        99,
    );
    h.run();
    h.get_by_label("Reset Strength").click();
    h.run();
    assert_eq!(
        strength(&h.state().0, id).unwrap_or_else(|| standard.clone()),
        standard
    );
    assert!(h.query_by_label("Reset Strength").is_none());
    // One undo step brings the change back.
    h.state_mut().0.undo();
    assert_eq!(strength(&h.state().0, id), Some(Value::Float(123.0)));
}

#[test]
fn a_component_s_card_has_its_menu_and_manual_page() {
    let (mut w, id) = with_a_lamp();
    w.set_manual_url("https://example.org/manual".into());
    let mut h = harness(w);
    h.run();
    // The ? opens Light's manual page; the ⋯ holds the rare actions.
    let _ = h.get_by_label("About Light");
    assert!(
        h.query_by_label("Remove Light").is_none(),
        "tucked in the menu"
    );
    h.get_by_label("Light menu").click();
    h.run();
    for item in ["Copy values", "Paste values", "Reset to defaults"] {
        let _ = h.get_by_label(item);
    }
    h.get_by_label("Remove Light").click();
    h.run();
    assert!(!h.state().0.document().area().get(id).unwrap().has("Light"));
}

#[test]
fn the_entity_menu_holds_what_the_bottom_buttons_did() {
    let (w, id) = with_a_lamp();
    let mut h = harness(w);
    h.run();
    // Values only, no row of buttons under them.
    assert!(h.query_by_label("Delete").is_none());
    assert!(h.query_by_label("Move here").is_none());
    h.get_by_label("Entity menu").click();
    h.run();
    for item in ["Move here", "Copy components", "Paste components", "Delete"] {
        let _ = h.get_by_label(item);
    }
    h.get_by_label("Copy components").click();
    h.run();
    assert!(!h.state().0.open.copied.is_empty());
    h.get_by_label("Entity menu").click();
    h.run();
    h.get_by_label("Delete").click();
    h.run();
    assert!(!h.state().0.document().area().contains(id));
}

#[test]
fn renaming_still_works_from_the_name_field() {
    let (w, id) = with_a_lamp();
    let mut h = harness(w);
    h.run();
    let name = h.get_by_label("Name");
    name.click();
    name.type_text("!");
    h.run();
    assert!(
        h.state()
            .0
            .document()
            .area()
            .get(id)
            .unwrap()
            .name
            .ends_with('!'),
        "{}",
        h.state().0.document().area().get(id).unwrap().name
    );
}

#[test]
fn a_file_used_by_several_entities_says_so() {
    let mut w = Workbench::new(info());
    let model = crate_model();
    let a = w.add_model(model.clone(), &[], 0.1, &[]);
    let b = w.add_model(model, &[], 0.1, &[]);
    let asset = w
        .document()
        .area()
        .get(a)
        .unwrap()
        .get::<area::builtin::Model>()
        .unwrap()
        .asset
        .unwrap()
        .asset;
    assert_eq!(users_of(w.document().area(), asset), 2);
    w.select(b);
    let mut h = harness(w);
    h.run();
    assert!(h.query_by_label("shared by 2").is_some());
    // One alone isn't marked.
    h.state_mut()
        .0
        .apply_property(PropertyRequest::Delete(a), 1);
    h.run();
    h.state_mut().0.select(b);
    h.run();
    assert!(h.query_by_label("shared by 2").is_none());
}

#[test]
fn a_search_with_no_match_says_so_and_enter_does_nothing() {
    let (w, id) = with_a_lamp();
    let mut h = harness(w);
    h.run();
    h.get_by_label("Add component").click();
    h.run();
    h.get_by_label("Search components").type_text("zzzz");
    h.run();
    assert!(
        h.query_by_label_contains("No component it lacks is called")
            .is_some()
    );
    let before = h
        .state()
        .0
        .document()
        .area()
        .get(id)
        .unwrap()
        .components
        .len();
    h.key_press(egui::Key::Enter);
    h.run();
    assert_eq!(
        h.state()
            .0
            .document()
            .area()
            .get(id)
            .unwrap()
            .components
            .len(),
        before
    );
}
