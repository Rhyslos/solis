//! The fields whose change counts once it's let go: what they hand over
//! and when, for a drag, a typed number and a colour.

use super::*;
use crate::test_support::drag_for_frames;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

/// A value as an editor holds it, and each value its field handed over.
#[derive(Default)]
struct State {
    value: f64,
    whole: bool,
    got: Vec<f64>,
}

/// One field over `value`, taken in whenever it's handed over, as the
/// editors do.
fn harness(value: f64, whole: bool) -> Harness<'static, State> {
    Harness::builder()
        .with_size(egui::vec2(300.0, 120.0))
        .build_ui_state(
            |ui, s: &mut State| {
                let got = if s.whole {
                    settled_whole_drag(ui, "n", s.value, 0.0..=1000.0, 0.05, " px")
                } else {
                    settled_drag(ui, "n", s.value, 0.0..=1000.0, 0.5, "")
                };
                if let Some(v) = got {
                    s.value = v;
                    s.got.push(v);
                }
                // Somewhere else to click.
                ui.add_space(40.0);
                let _ = ui.button("Elsewhere");
            },
            State {
                value,
                whole,
                got: Vec::new(),
            },
        )
}

fn field(h: &Harness<'static, State>) -> egui::Rect {
    h.get_all_by_role(egui::accesskit::Role::SpinButton)
        .next()
        .expect("the field")
        .rect()
}

#[test]
fn a_drag_hands_its_value_over_once_when_it_s_let_go() {
    let mut h = harness(10.0, false);
    h.run();
    let at = field(&h).center();
    h.hover_at(at);
    h.run();
    h.drag_at(at);
    h.run();
    for frame in 1..=12 {
        h.hover_at(at + egui::vec2(5.0, 0.0) * frame as f32);
        h.run();
        assert!(h.state().got.is_empty(), "nothing mid-drag (frame {frame})");
        assert_eq!(h.state().value, 10.0);
    }
    h.drop_at(at + egui::vec2(60.0, 0.0));
    h.run();
    h.run();
    let got = h.state().got.clone();
    assert_eq!(got.len(), 1, "once: {got:?}");
    // The whole drag added up: 60 points at half a step a point, less
    // what the mouse moved before it counted as a drag.
    assert!(got[0] > 30.0 && got[0] <= 40.0, "{got:?}");
}

#[test]
fn a_typed_number_counts_on_enter_or_when_the_field_is_left() {
    for leave_with_enter in [true, false] {
        let mut h = harness(10.0, false);
        h.run();
        let at = field(&h).center();
        // A click to type in it.
        h.hover_at(at);
        h.drag_at(at);
        h.drop_at(at);
        h.run();
        h.event(egui::Event::Text("2".into()));
        h.run();
        h.event(egui::Event::Text("5".into()));
        h.run();
        assert!(h.state().got.is_empty(), "nothing a letter at a time");
        assert_eq!(h.state().value, 10.0);
        if leave_with_enter {
            h.key_press(egui::Key::Enter);
        } else {
            h.get_by_label("Elsewhere").click();
        }
        h.run();
        h.run();
        assert_eq!(h.state().got, [25.0], "enter: {leave_with_enter}");
    }
}

#[test]
fn a_whole_number_is_handed_over_whole_and_only_when_it_changed() {
    let mut h = harness(16.0, true);
    h.run();
    let at = field(&h).center();
    drag_for_frames(&mut h, at, egui::vec2(6.0, 0.0), 10);
    let got = h.state().got.clone();
    assert_eq!(got.len(), 1, "{got:?}");
    assert_eq!(got[0], got[0].round(), "whole");
    assert!(got[0] > 16.0);
    // A drag too small to reach the next whole number (8 points, a
    // twentieth of a step each): nothing.
    let mut h = harness(16.0, true);
    h.run();
    let at = field(&h).center();
    drag_for_frames(&mut h, at, egui::vec2(1.0, 0.0), 8);
    assert!(h.state().got.is_empty(), "{:?}", h.state().got);
    assert_eq!(h.state().value, 16.0);
}

#[test]
fn a_colour_is_handed_over_once_when_the_mouse_comes_up() {
    let mut h = Harness::builder()
        .with_size(egui::vec2(400.0, 400.0))
        .build_ui_state(
            |ui, s: &mut (Vec<[f32; 3]>, [f32; 3])| {
                if let Some(c) = settled_colour(ui, "c", s.1) {
                    s.1 = c;
                    s.0.push(c);
                }
            },
            (Vec::new(), [0.2, 0.6, 0.3]),
        );
    h.run();
    let well = h
        .get_all_by_role(egui::accesskit::Role::ColorWell)
        .next()
        .expect("a colour button");
    let button = well.rect();
    well.click();
    h.run();
    assert!(h.state().0.is_empty(), "opening the picker changes nothing");
    let from = button.left_bottom() + egui::vec2(60.0, 120.0);
    h.hover_at(from);
    h.run();
    h.drag_at(from);
    h.run();
    for frame in 1..=8 {
        h.hover_at(from + egui::vec2(8.0, 6.0) * frame as f32);
        h.run();
        assert!(h.state().0.is_empty(), "nothing mid-drag");
    }
    h.drop_at(from + egui::vec2(64.0, 48.0));
    h.run();
    assert_eq!(h.state().0.len(), 1);
    assert_ne!(h.state().0[0], [0.2, 0.6, 0.3]);
}
