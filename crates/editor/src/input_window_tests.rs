//! File > Input in the whole window: when a change is saved.

use crate::test_support::*;
use crate::workbench::{Workbench, WorkbenchAction};
use egui_kittest::kittest::Queryable;

/// The window open on a project's ready-made actions.
fn opened() -> egui_kittest::Harness<'static, State> {
    let mut w = Workbench::new(info(), None);
    w.open.project = Some(crate::library::ProjectInfo {
        name: "Test".into(),
        root: std::env::temp_dir(),
        assets: std::env::temp_dir(),
    });
    w.open_input_window();
    let mut h = harness(w);
    h.run();
    h.run();
    h
}

/// How many times the bindings were sent to be saved.
fn saves(h: &egui_kittest::Harness<'static, State>) -> usize {
    let saved = |a: &&WorkbenchAction| matches!(a, WorkbenchAction::SaveBindings(_));
    h.state().1.actions.iter().filter(saved).count()
}

/// The first action that goes with the mouse (Look), as the project has it.
fn look(h: &egui_kittest::Harness<'static, State>) -> input::Action {
    let bindings = h.state().0.bindings();
    bindings
        .actions
        .iter()
        .find(|a| a.is_motion())
        .expect("a ready-made action goes with the mouse")
        .clone()
}

/// BUG-25: Mouse speed dragged for ten frames is saved once, when the
/// mouse is let go, and so is Stick and key speed.
#[test]
fn a_dragged_speed_is_saved_once_when_it_s_let_go() {
    let mut h = opened();
    let before = look(&h);
    for (label, by) in [("Mouse speed", 8.0), ("Stick and key speed", 4.0)] {
        h.state_mut().1.actions.clear();
        let as_it_was = look(&h);
        let field = number_field_beside(&h, label);
        h.hover_at(field.center());
        h.run();
        h.drag_at(field.center());
        h.run();
        for frame in 1..=10 {
            h.hover_at(field.center() + egui::vec2(by, 0.0) * frame as f32);
            h.run();
            assert_eq!(saves(&h), 0, "{label}: nothing saved mid-drag");
            assert_eq!(look(&h), as_it_was, "{label}: nor changed");
        }
        h.drop_at(field.center() + egui::vec2(by, 0.0) * 10.0);
        h.run();
        assert_eq!(saves(&h), 1, "{label}: saved once, when let go");
    }
    let after = look(&h);
    assert!(after.mouse_speed > before.mouse_speed + 0.3, "{after:?}");
    assert!(after.pad_speed > before.pad_speed + 20.0, "{after:?}");
    // What was saved is what the project has now.
    let Some(WorkbenchAction::SaveBindings(text)) = h.state().1.actions.last() else {
        panic!("the last thing asked for was the save");
    };
    assert_eq!(*text, h.state().0.bindings().write());
}

/// A typed speed is saved when Enter is pressed, not a digit at a time.
#[test]
fn a_typed_speed_is_saved_on_enter() {
    let mut h = opened();
    let field = number_field_beside(&h, "Mouse speed");
    h.state_mut().1.actions.clear();
    h.hover_at(field.center());
    h.drag_at(field.center());
    h.drop_at(field.center());
    h.run();
    for digit in ["0", ".", "4"] {
        h.event(egui::Event::Text(digit.into()));
        h.run();
        assert_eq!(saves(&h), 0, "nothing saved while typing");
    }
    h.key_press(egui::Key::Enter);
    h.run();
    assert_eq!(saves(&h), 1);
    assert_eq!(look(&h).mouse_speed, 0.4);
}

/// The window's other controls each save once, on their click.
#[test]
fn a_click_in_the_window_saves_once() {
    let mut h = opened();
    h.state_mut().1.actions.clear();
    h.get_all_by_label(input::MOUSE_WHEEL)
        .next()
        .unwrap()
        .click();
    h.run();
    h.run();
    h.run();
    assert_eq!(saves(&h), 1, "a whole-axis tick");
}
