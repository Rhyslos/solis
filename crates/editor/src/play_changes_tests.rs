use super::*;
use crate::test_support::{harness, info};
use egui_kittest::kittest::Queryable;

fn fresh() -> Workbench {
    Workbench::new(info(), None)
}

#[test]
fn the_editor_takes_a_colour_while_the_game_runs() {
    let mut w = fresh();
    assert_eq!(w.play_tint(), None);
    assert_eq!(w.tinted(theme::HEADER), theme::HEADER);
    w.play();
    assert_eq!(w.play_tint(), Some(PLAYING));
    assert_ne!(w.tinted(theme::HEADER), theme::HEADER);
    w.pause();
    assert_eq!(w.play_tint(), Some(theme::WARNING));
    w.stop();
    assert_eq!(w.play_tint(), None);
}

#[test]
fn stopping_with_nothing_changed_asks_nothing() {
    let mut w = fresh();
    w.play();
    w.stop();
    assert!(w.play_changes().is_none());
}

#[test]
fn changes_made_while_playing_wait_for_keep() {
    let mut w = fresh();
    let before = w.document().area().iter().count();
    w.play();
    let lamp = w.add_point_light();
    w.stop();
    // Back as it was, and asked.
    assert!(!w.document().area().contains(lamp));
    assert_eq!(w.play_changes().unwrap(), ["Add Point light 1"]);
    // Play waits for the answer.
    w.play();
    assert_eq!(w.play_state(), PlayState::Editing);
    w.keep_play_changes();
    assert!(w.play_changes().is_none());
    assert!(w.document().area().contains(lamp));
    assert_eq!(w.document().area().iter().count(), before + 1);
    // Still undoable.
    w.undo();
    assert!(!w.document().area().contains(lamp));
}

#[test]
fn undo_all_leaves_the_area_as_it_was_before_play() {
    let mut w = fresh();
    w.play();
    let lamp = w.add_point_light();
    w.add_spotlight();
    w.stop();
    assert_eq!(w.play_changes().unwrap().len(), 2);
    w.undo_play_changes();
    assert!(w.play_changes().is_none());
    assert!(!w.document().area().contains(lamp));
}

#[test]
fn the_question_is_answered_with_its_buttons() {
    let mut w = fresh();
    w.play();
    let lamp = w.add_point_light();
    w.stop();
    let mut h = harness(w);
    h.run();
    let _ = h.get_by_label("Keep the changes made while playing?");
    let _ = h.get_by_label("•  Add Point light 1");
    h.get_by_label("Keep").click();
    h.run();
    assert!(h.state().0.play_changes().is_none());
    assert!(h.state().0.document().area().contains(lamp));
}

#[test]
fn many_changes_are_counted_after_the_first_few() {
    let mut w = fresh();
    w.play();
    for _ in 0..11 {
        w.add_point_light();
    }
    w.stop();
    let mut h = harness(w);
    h.run();
    let _ = h.get_by_label("and 3 more");
    h.get_by_label("Undo all").click();
    h.run();
    assert!(h.state().0.play_changes().is_none());
}

#[test]
fn colours_blend_between_two() {
    let a = Color32::from_rgb(0, 100, 200);
    let b = Color32::from_rgb(200, 100, 0);
    assert_eq!(blend(a, b, 0.0), a);
    assert_eq!(blend(a, b, 1.0), b);
    assert_eq!(blend(a, b, 0.5), Color32::from_rgb(100, 100, 100));
    assert_eq!(count(1, "change"), "1 change");
    assert_eq!(count(0, "change"), "0 changes");
}
