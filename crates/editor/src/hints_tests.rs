use super::*;
use crate::test_support::{harness, info};
use egui_kittest::kittest::Queryable;

fn fresh() -> Workbench {
    Workbench::new(info(), None)
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
    w.document.selection.select_all(vec![empty, lamp], lamp);
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
fn the_panel_shows_the_hints_or_says_they_are_off() {
    let mut h = harness(fresh());
    // Worked out once the panel is known to be in front.
    h.run();
    h.run();
    assert!(h.query_by_label_contains("Select something").is_some());
    h.state_mut().0.set_hints_on(false);
    h.run();
    assert!(h.query_by_label_contains("Hints are off").is_some());
}

#[test]
fn a_layout_saved_before_hints_gets_them_under_properties() {
    let mut old = crate::layout::standard();
    crate::layout::close(&mut old, crate::panel::Panel::Hints);
    let back = crate::layout::restore(Some(old));
    let hints = back.find_tab(&crate::panel::Panel::Hints).unwrap();
    let properties = back.find_tab(&crate::panel::Panel::Properties).unwrap();
    assert_ne!(hints.node, properties.node, "a group of its own");
    assert!(crate::layout::is_complete(&back));
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
