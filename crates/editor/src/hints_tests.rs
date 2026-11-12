use super::*;
use crate::test_support::{harness, info};
use egui_kittest::kittest::Queryable;

fn fresh() -> Workbench {
    Workbench::new(info())
}

#[test]
fn with_nothing_selected_the_hint_says_how_to_start() {
    let w = fresh();
    let hints = w.hints();
    assert_eq!(hints.len(), 1);
    assert!(hints[0].starts_with("Select something"));
}

#[test]
fn a_selected_entity_s_components_are_explained_transform_first() {
    let mut w = fresh();
    let lamp = w.add_point_light();
    w.select(lamp);
    let hints = w.hints();
    assert!(hints[0].starts_with("Transform"), "{hints:?}");
    assert!(hints.iter().any(|h| h.starts_with("Light makes it shine")));
}

#[test]
fn an_empty_entity_and_many_entities_have_their_own_hint() {
    let mut w = fresh();
    let empty = w.add_entity();
    w.apply_property(
        crate::properties::PropertyRequest::RemoveComponent {
            entity: empty,
            component: "Transform".into(),
        },
        1,
    );
    w.select(empty);
    assert!(w.hints()[0].contains("is empty"), "{:?}", w.hints());
    let lamp = w.add_point_light();
    w.open
        .document
        .selection
        .select_all(vec![empty, lamp], lamp);
    assert!(w.hints()[0].starts_with("2 entities are selected"));
}

#[test]
fn playing_and_paused_add_a_hint() {
    let mut w = fresh();
    w.play();
    assert!(w.hints().iter().any(|h| h.starts_with("You're playing")));
    w.pause();
    assert!(
        w.hints()
            .iter()
            .any(|h| h.starts_with("The game is paused"))
    );
}

#[test]
fn every_engine_component_has_a_hint() {
    let registry = area::builtin::registry().unwrap();
    for schema in registry.iter() {
        assert!(about(&schema.name).is_some(), "{}", schema.name);
    }
}

#[test]
fn the_card_shows_the_hints_and_goes_when_they_are_off() {
    let mut h = harness(fresh());
    // Worked out once Properties is known to be in front.
    h.run();
    h.run();
    assert!(h.query_by_label_contains("Select something").is_some());
    h.state_mut().0.set_hints_on(false);
    h.run();
    assert!(h.query_by_label_contains("Select something").is_none());
    assert!(h.query_by_label("Turn hints off").is_none());
}

#[test]
fn turn_hints_off_on_the_card_turns_them_off_and_says_where_they_come_back() {
    let mut h = harness(fresh());
    h.run();
    h.run();
    h.get_by_label("Turn hints off").click();
    h.run();
    h.run();
    assert!(!h.state().0.hints_on());
    assert!(
        h.query_by_label_contains("File \u{203a} Preferences turns them back on")
            .is_some()
    );
}

#[test]
fn hints_are_the_last_card_in_properties_on_home_s_entity_tab() {
    let mut h = harness(fresh());
    h.run();
    h.run();
    let card = h.get_by_label("Turn hints off").rect();
    let properties = h
        .state()
        .0
        .shell
        .drawn_at(crate::panel::Panel::Properties)
        .expect("in view");
    assert!(properties.contains_rect(card), "{card:?} in {properties:?}");
}

#[test]
fn settings_turns_hints_off_and_reset_turns_them_on() {
    let mut w = fresh();
    w.open_settings();
    let mut h = harness(w);
    // Reset restyles the editor, whose headings need its fonts.
    h.ctx.set_fonts(crate::theme::fonts());
    h.run();
    h.get_by_label("Show hints").click();
    h.run();
    assert!(!h.state().0.hints_on());
    h.get_by_label("Reset this page").click();
    h.run();
    assert!(h.state().0.hints_on());
}
