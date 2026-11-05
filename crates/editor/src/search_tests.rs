use super::*;
use crate::test_support::{harness, info};
use egui_kittest::kittest::Queryable;

fn fresh() -> Workbench {
    Workbench::new(info(), None)
}

#[test]
fn a_word_at_the_start_scores_most_then_inside_then_letters_in_order() {
    assert_eq!(score("save", "Save area"), Some(3));
    assert_eq!(score("area", "Save area"), Some(3));
    assert_eq!(score("ave", "Save area"), Some(2));
    assert_eq!(score("svar", "Save area"), Some(1));
    assert_eq!(score("save area", "Save area as…"), Some(6));
    assert_eq!(score("zebra", "Save area"), None);
    assert_eq!(
        score("save zebra", "Save area"),
        None,
        "every word must match"
    );
    assert_eq!(score("", "anything"), Some(0));
    assert_eq!(score("light", "Add point light"), Some(3));
    // Odd text is safe.
    assert_eq!(score("é", "Café"), Some(2));
    assert!(score("x", "").is_none());
}

#[test]
fn the_best_match_comes_first_and_actions_before_entities() {
    let w = fresh();
    let items = w.search_items();
    let found = results(&items, "light");
    assert_eq!(found[0].name, "Add point light");
    assert!(found.iter().any(|i| i.name == "Add spotlight"));
    // The Sun is an entity: found by name.
    let sun = results(&items, "sun");
    assert!(sun.iter().any(|i| i.kind == SearchKind::Entity));
    // Panels too.
    assert!(
        results(&items, "console")
            .iter()
            .any(|i| i.kind == SearchKind::Panel)
    );
    // Nothing typed: the actions, in menu order.
    let all = results(&items, "");
    assert_eq!(all[0].name, "New project…");
    assert!(all.iter().all(|i| i.kind == SearchKind::Action));
    assert!(all.len() <= MAX_SHOWN);
}

#[test]
fn actions_that_cant_be_done_are_left_out() {
    let mut w = fresh();
    let names =
        |w: &Workbench| -> Vec<String> { w.search_items().into_iter().map(|i| i.name).collect() };
    assert!(
        !names(&w).contains(&"Pause".to_string()),
        "not while editing"
    );
    assert!(
        !names(&w).contains(&"Delete".to_string()),
        "nothing selected"
    );
    w.play();
    assert!(names(&w).contains(&"Pause".to_string()));
    assert!(!names(&w).contains(&"Play".to_string()));
    w.stop();
    let id = w.add_point_light();
    w.select(id);
    assert!(names(&w).contains(&"Delete".to_string()));
    // Undo shows what it undoes.
    assert!(names(&w).iter().any(|n| n.starts_with("Undo ")));
}

#[test]
fn space_opens_the_search_and_enter_does_the_first_result() {
    let mut h = harness(fresh());
    h.run();
    assert!(h.query_by_label("Search").is_none());
    h.key_press(egui::Key::Space);
    h.run();
    h.run();
    assert!(h.state().0.search.open);
    let before = h.state().0.document().area().iter().count();
    h.get_by_label("Search").type_text("point light");
    h.run();
    h.key_press(egui::Key::Enter);
    h.run();
    assert!(!h.state().0.search.open);
    assert_eq!(h.state().0.document().area().iter().count(), before + 1);
}

#[test]
fn arrows_choose_esc_closes_and_the_box_opens_it_too() {
    let mut h = harness(fresh());
    h.run();
    h.get_by_label("Search or do anything").click();
    h.run();
    h.run();
    assert!(h.state().0.search.open);
    h.get_by_label("Search").type_text("add");
    h.run();
    h.key_press(egui::Key::ArrowDown);
    h.run();
    assert_eq!(h.state().0.search.chosen, 1);
    h.key_press(egui::Key::ArrowUp);
    h.key_press(egui::Key::ArrowUp);
    h.run();
    assert_eq!(h.state().0.search.chosen, 0, "not past the top");
    h.key_press(egui::Key::Escape);
    h.run();
    assert!(!h.state().0.search.open);
}

#[test]
fn space_while_typing_types_a_space() {
    let mut w = fresh();
    let id = w.add_point_light();
    w.select(id);
    let mut h = harness(w);
    h.run();
    let name = h.get_by_label("Name");
    name.click();
    h.run();
    h.get_by_label("Name").type_text(" lamp");
    h.key_press(egui::Key::Space);
    h.run();
    assert!(!h.state().0.search.open);
}

#[test]
fn picking_an_entity_selects_it_and_a_panel_brings_it_forward() {
    let mut w = fresh();
    let lamp = w.add_point_light();
    w.document.selection.clear();
    let ctx = egui::Context::default();
    let mut actions = Vec::new();
    w.do_search(SearchTarget::Entity(lamp), &ctx, &mut actions);
    assert_eq!(w.selection(), Some(lamp));
    w.do_search(SearchTarget::Panel(Panel::Console), &ctx, &mut actions);
    let path = w.dock.find_tab(&Panel::Console).unwrap();
    assert_eq!(
        w.dock[path.surface][path.node].tabs().unwrap()[path.tab.0],
        Panel::Console
    );
    // An entity since deleted is ignored.
    w.delete_selection();
    w.do_search(SearchTarget::Entity(lamp), &ctx, &mut actions);
    assert_eq!(w.selection(), None);
    // A File action goes to the app.
    w.do_search(
        SearchTarget::App(WorkbenchAction::SaveArea),
        &ctx,
        &mut actions,
    );
    assert!(actions.contains(&WorkbenchAction::SaveArea));
}
