//! Limits at their edge, the stick's dead zone, pad presses and error
//! messages: found untested by the hardening milestone's mutation testing.

use super::TICK;
use crate::*;
use glam::Vec2;

/// `count` button actions named A0, A1, …, each with `inputs` keys.
fn many(count: usize, inputs: usize) -> Bindings {
    let keys: Vec<String> = (0..inputs)
        .map(|i| ((b'A' + i as u8) as char).to_string())
        .collect();
    let keys: Vec<&str> = keys.iter().map(String::as_str).collect();
    Bindings {
        actions: (0..count)
            .map(|n| Action::button(&format!("A{n}"), &keys))
            .collect(),
    }
}

#[test]
fn the_most_actions_and_inputs_are_allowed_and_one_more_isn_t() {
    assert!(many(MAX_ACTIONS, 1).check().is_ok());
    assert!(many(MAX_ACTIONS + 1, 1).check().is_err());
    assert!(many(1, MAX_INPUTS).check().is_ok());
    assert!(many(1, MAX_INPUTS + 1).check().is_err());
}

#[test]
fn a_bindings_file_of_exactly_the_most_bytes_is_read() {
    let text = many(1, 1).write();
    let limit = crate::bindings::MAX_FILE_BYTES;
    // Padded with a comment line up to the limit: still read.
    let mut padded = text.clone();
    padded.push('#');
    padded.push_str(&" ".repeat(limit - padded.len() - 1));
    padded.push('\n');
    assert_eq!(padded.len(), limit);
    assert!(Bindings::read(&padded).is_ok());
    padded.push(' ');
    assert_eq!(Bindings::read(&padded), Err(BindingsError::TooBig));
}

#[test]
fn adding_the_ready_made_actions_stops_at_the_most_actions() {
    let mut nearly_full = many(MAX_ACTIONS - 1, 1);
    let added = nearly_full.add_missing();
    assert_eq!(added.len(), 1, "{added:?}");
    assert_eq!(nearly_full.actions.len(), MAX_ACTIONS);
    assert!(nearly_full.check().is_ok());
    assert!(nearly_full.add_missing().is_empty(), "full");
}

#[test]
fn a_pad_button_held_is_pressed_once() {
    let mut b = Bindings::default();
    b.actions.push(Action::button("Fire", &["Pad A"]));
    let mut d = Devices::new();
    let a_down = || {
        Some(Pad {
            buttons: ["A".to_string()].into(),
            ..Pad::default()
        })
    };
    d.set_pad(a_down());
    let first = d.sample(&b, TICK).get("Fire");
    assert!(first.pressed && first.held);
    // Read again, still down: held, not pressed again.
    d.set_pad(a_down());
    let second = d.sample(&b, TICK).get("Fire");
    assert!(second.held && !second.pressed, "{second:?}");
}

#[test]
fn a_stick_counts_from_the_edge_of_its_dead_zone() {
    let b = Bindings::default();
    let mut d = Devices::new();
    let mut move_at = |x: f32| {
        d.set_pad(Some(Pad {
            left: Vec2::new(x, 0.0),
            ..Pad::default()
        }));
        d.sample(&b, TICK).get("Move").axis.x
    };
    assert_eq!(move_at(DEAD_ZONE * 0.99), 0.0, "inside: nothing");
    assert!(move_at(DEAD_ZONE).abs() < 1e-6, "at its edge: starts at 0");
    let half = (DEAD_ZONE + 1.0) / 2.0;
    assert!((move_at(half) - 0.5).abs() < 1e-5, "half way out: half");
    assert!((move_at(1.0) - 1.0).abs() < 1e-6);
    assert!((move_at(1.5) - 1.0).abs() < 1e-6, "never past 1");
    assert!((move_at(-1.0) + 1.0).abs() < 1e-6, "the other way too");
}

#[test]
fn a_tick_held_pressed_and_released_at_once_round_trips() {
    let mut frame = Frame::default();
    frame.actions.insert(
        "Tap".to_string(),
        ActionState {
            held: true,
            pressed: true,
            released: true,
            axis: Vec2::ZERO,
        },
    );
    let recording = Recording {
        area: None,
        seed: 1,
        start: 2,
        ticks: vec![Tick {
            frame,
            world: 3,
            game: None,
        }],
    };
    assert_eq!(Recording::read(&recording.write()), Ok(recording));
}

#[test]
fn errors_say_what_went_wrong() {
    assert!(BindingsError::TooBig.to_string().contains("MB"));
    assert!(
        BindingsError::Wrong("two actions are called Jump".into())
            .to_string()
            .contains("Jump")
    );
    assert!(
        BindingsError::Damaged("expected =".into())
            .to_string()
            .contains("expected =")
    );
    assert!(RecordingError::TooBig.to_string().contains("hour"));
    assert!(RecordingError::Damaged(7).to_string().contains('7'));
    assert!(!RecordingError::NotARecording.to_string().is_empty());
}

#[test]
fn a_recording_holds_exactly_an_hour() {
    // The limit's own value: the test of the limit uses the constant, so
    // it wouldn't notice the constant changing.
    assert_eq!(
        crate::recording::MAX_TICKS,
        216_000,
        "an hour at 60 a second"
    );
    assert_eq!(MAX_RECORDING_BYTES, 64 * 1024 * 1024);
}
