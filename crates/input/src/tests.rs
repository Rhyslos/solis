//! Input: bindings and their file, sampling, recordings, and damaged
//! files never crashing.

use super::*;
use glam::Vec2;

const TICK: f32 = 1.0 / 60.0;

#[test]
fn the_ready_made_actions_are_the_chosen_controls() {
    let b = Bindings::default();
    let names: Vec<&str> = b.actions.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "Move", "Look", "Jump", "Sprint", "Interact", "Pan", "Turn", "Zoom", "Drag"
        ]
    );
    // The cameras' controls (decision 0041).
    assert_eq!(b.get("Pan").unwrap().left, ["Left"]);
    assert_eq!(b.get("Turn").unwrap().left, ["Alt+Left"]);
    assert_eq!(b.get("Zoom").unwrap().up, ["PageUp", "Pad RT"]);
    assert!(b.get("Zoom").unwrap().is_motion());
    assert_eq!(b.get("Drag").unwrap().inputs, ["Mouse Middle"]);
    assert_eq!(b.get("Interact").unwrap().inputs, ["F", "Pad X"]);
    assert_eq!(b.get("Move").unwrap().up, ["W"]);
    assert!(b.get("Look").unwrap().is_motion());
    assert!(!b.get("Move").unwrap().is_motion());
    b.check().unwrap();
}

#[test]
fn bindings_round_trip_through_their_file() {
    let mut b = Bindings::default();
    b.actions
        .push(Action::button("Fire_2", &["Mouse Left", "Pad RT"]));
    let text = b.write();
    assert!(text.contains("name = \"Jump\""));
    assert_eq!(Bindings::read(&text).unwrap(), b);
    // An empty file is no actions, not a mistake.
    assert!(Bindings::read("").unwrap().actions.is_empty());
}

#[test]
fn wrong_bindings_are_refused_with_why() {
    let bad = [
        "[[action]]\nname = \"has space\"\n",
        "[[action]]\nname = \"Jump\"\n[[action]]\nname = \"Jump\"\n",
        "[[action]]\nname = \"Jump\"\ninputs = [\"Pad Z\"]\n",
        "[[action]]\nname = \"Jump\"\ninputs = [\"Mouse Sideways\"]\n",
        "[[action]]\nname = \"Look\"\nkind = \"axis\"\nmouse_speed = -1.0\n",
        "[[action]]\nname = \"Look\"\nkind = \"sideways\"\n",
        "[[action]]\nname = \"Jump\"\nsurprise = 1\n",
        "not toml at all [[",
    ];
    for text in bad {
        assert!(Bindings::read(text).is_err(), "{text}");
    }
    let many: String = (0..=MAX_ACTIONS)
        .map(|n| format!("[[action]]\nname = \"A{n}\"\n"))
        .collect();
    assert!(matches!(
        Bindings::read(&many),
        Err(BindingsError::Wrong(_))
    ));
    assert_eq!(
        Bindings::read(&"#".repeat(2 << 20)),
        Err(BindingsError::TooBig)
    );
}

#[test]
fn a_tap_between_ticks_is_a_press_and_a_release() {
    let b = Bindings::default();
    let mut d = Devices::new();
    d.set("Space", true);
    let f = d.sample(&b, TICK);
    let jump = f.get("Jump");
    assert!(jump.held && jump.pressed && !jump.released);
    // Still down: held, not pressed again.
    let jump = d.sample(&b, TICK).get("Jump");
    assert!(jump.held && !jump.pressed);
    d.set("Space", false);
    let jump = d.sample(&b, TICK).get("Jump");
    assert!(!jump.held && jump.released);
    assert!(d.sample(&b, TICK).get("Jump").is_idle());
    // Down and up again before a tick: still seen.
    d.set("Space", true);
    d.set("Space", false);
    let jump = d.sample(&b, TICK).get("Jump");
    assert!(jump.pressed && jump.released && !jump.held);
    // Either input sets it off.
    d.set_pad(Some(Pad {
        buttons: ["A".to_string()].into(),
        ..Pad::default()
    }));
    assert!(d.sample(&b, TICK).get("Jump").pressed);
}

#[test]
fn axes_come_from_keys_sticks_and_the_mouse() {
    let b = Bindings::default();
    let mut d = Devices::new();
    d.set("W", true);
    d.set("D", true);
    let walk = d.sample(&b, TICK).get("Move");
    assert!(walk.held);
    assert!(
        (walk.axis.length() - 1.0).abs() < 1e-5,
        "never past 1: {walk:?}"
    );
    assert!(walk.axis.x > 0.0 && walk.axis.y > 0.0);
    d.set("W", false);
    d.set("D", false);
    // A stick resting near the middle doesn't count.
    d.set_pad(Some(Pad {
        left: Vec2::new(0.05, -0.1),
        ..Pad::default()
    }));
    assert!(!d.sample(&b, TICK).get("Move").held);
    d.set_pad(Some(Pad {
        left: Vec2::new(0.0, -1.0),
        ..Pad::default()
    }));
    assert_eq!(d.sample(&b, TICK).get("Move").axis, Vec2::new(0.0, -1.0));
    // Look: the mouse's pixels times its speed, y up; a stick at its
    // speed for the tick.
    d.set_pad(None);
    d.mouse_moved(Vec2::new(10.0, -20.0));
    let look = d.sample(&b, TICK).get("Look");
    assert!(
        (look.axis - Vec2::new(1.5, 3.0)).length() < 1e-5,
        "{look:?}"
    );
    let after = d.sample(&b, TICK).get("Look");
    assert!(
        after.axis == Vec2::ZERO && !after.held && after.released,
        "movement is used once"
    );
    d.set_pad(Some(Pad {
        right: Vec2::new(1.0, 0.0),
        ..Pad::default()
    }));
    let look = d.sample(&b, TICK).get("Look");
    assert!((look.axis.x - 180.0 * TICK).abs() < 1e-4, "{look:?}");
    // Nonsense movement is ignored.
    d.mouse_moved(Vec2::NAN);
    assert!(d.sample(&b, TICK).get("Look").axis.is_finite());
}

#[test]
fn letting_go_of_everything_releases_what_was_held() {
    let b = Bindings::default();
    let mut d = Devices::new();
    d.set("Shift", true);
    assert!(d.sample(&b, TICK).get("Sprint").held);
    d.release_all();
    let sprint = d.sample(&b, TICK).get("Sprint");
    assert!(sprint.released && !sprint.held);
}

#[test]
fn rebinding_replaces_inputs_of_the_same_kind() {
    let mut b = Bindings::default();
    assert!(b.rebind("Interact", "E"));
    assert_eq!(b.get("Interact").unwrap().inputs, ["Pad X", "E"]);
    assert!(b.rebind("Interact", "Pad Y"));
    assert_eq!(b.get("Interact").unwrap().inputs, ["E", "Pad Y"]);
    assert!(!b.rebind("Interact", "Pad Q"));
    assert!(!b.rebind("Nothing", "E"));
    assert_eq!(b.get("Interact").unwrap().first_input(""), Some("E"));
    assert_eq!(
        b.get("Interact").unwrap().first_input("Pad "),
        Some("Pad Y")
    );
}

fn sample_recording() -> Recording {
    let b = Bindings::default();
    let mut d = Devices::new();
    let mut ticks = Vec::new();
    for n in 0..10_u64 {
        d.set("W", n % 3 == 0);
        d.mouse_moved(Vec2::new(n as f32 * 1.7, -0.3));
        ticks.push(Tick {
            frame: d.sample(&b, TICK),
            world: n.wrapping_mul(0x9e37_79b9_7f4a_7c15),
            game: (n % 4 == 0).then_some(n * 7),
        });
    }
    Recording {
        seed: 42,
        start: 0xdead_beef,
        ticks,
        area: Some("areas/harbour.solarea".into()),
    }
}

#[test]
fn recordings_round_trip_exactly() {
    let r = sample_recording();
    let text = r.write();
    assert!(text.starts_with("solis recording 2\nseed 42\narea areas/harbour.solarea\n"));
    assert_eq!(Recording::read(&text).unwrap(), r);
    // Playing back the same: no difference; a changed checksum is found.
    assert_eq!(r.first_difference(&r.ticks), None);
    let mut other = r.ticks.clone();
    other[6].world ^= 1;
    assert_eq!(r.first_difference(&other), Some(6));
    other[6].world ^= 1;
    other[4].game = Some(1);
    assert_eq!(r.first_difference(&other), Some(4));
}

#[test]
fn damaged_recordings_are_refused_never_crashing() {
    let good = sample_recording().write();
    assert_eq!(Recording::read("hello"), Err(RecordingError::NotARecording));
    // A newer one is refused before it's read on, with why.
    let newer = Recording::read("solis recording 3\nseed 1\nstart 0\n").unwrap_err();
    assert!(newer.to_string().contains("newer version"), "{newer}");
    assert!(matches!(
        Recording::read("solis recording 0\nseed 1\nstart 0\n"),
        Err(RecordingError::Upgrade(_))
    ));
    for bad in [
        "solis recording 1\nseed x\nstart 0\n",
        "solis recording 1\nseed 1\nstart zz\n",
        "solis recording 1\nseed 1\nstart 0\n1 w 0\n",
        "solis recording 1\nseed 1\nstart 0\n0 w 0 Jump=q\n",
        "solis recording 1\nseed 1\nstart 0\n0 w 0 Jump=h,1\n",
        "solis recording 1\nseed 1\nstart 0\n0 w 0 Jump=h,7fc00000,0\n",
        "solis recording 1\nseed 1\nstart 0\n0 w 0 has-dash=h\n",
        "solis recording 1\nseed 1\nstart 0\n0 x 0\n",
    ] {
        assert!(
            matches!(Recording::read(bad), Err(RecordingError::Damaged(_))),
            "{bad}"
        );
    }
    // Fuzz: every prefix and many single-byte changes of a good file are
    // read or refused, never a crash.
    for cut in 0..good.len() {
        if good.is_char_boundary(cut) {
            let _ = Recording::read(&good[..cut]);
        }
    }
    let bytes = good.as_bytes();
    let mut seed = 0x1234_5678_u64;
    for _ in 0..5_000 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let mut changed = bytes.to_vec();
        let at = (seed as usize) % changed.len();
        changed[at] = (seed >> 32) as u8;
        if let Ok(text) = String::from_utf8(changed) {
            let _ = Recording::read(&text);
        }
    }
}

#[test]
fn damaged_bindings_never_crash() {
    let good = Bindings::default().write();
    let bytes = good.as_bytes();
    let mut seed = 0x8765_4321_u64;
    for _ in 0..5_000 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let mut changed = bytes.to_vec();
        let at = (seed as usize) % changed.len();
        changed[at] = (seed >> 32) as u8;
        if let Ok(text) = String::from_utf8(changed)
            && let Ok(b) = Bindings::read(&text)
        {
            // Whatever's read is allowed.
            b.check().unwrap();
        }
    }
}

#[test]
fn names_are_checked() {
    for good in ["Jump", "Fire_2", "a"] {
        assert!(valid_action_name(good));
    }
    for bad in ["", "2fast", "has space", "dash-ed", &"x".repeat(65)] {
        assert!(!valid_action_name(bad), "{bad}");
    }
    for good in [
        "W",
        "Space",
        "Up",
        "Shift",
        "Mouse Left",
        "Mouse Motion",
        "Pad A",
        "Pad LeftStick",
        "[",
    ] {
        assert!(valid_input_name(good), "{good}");
    }
    for bad in ["", "Pad Q", "Mouse Up", "has space", &"K".repeat(33)] {
        assert!(!valid_input_name(bad), "{bad}");
    }
}

#[test]
fn combinations_are_named_with_their_modifiers_first() {
    assert_eq!(split_combination("Alt+Left"), Some((vec!["Alt"], "Left")));
    assert_eq!(
        split_combination("Ctrl+Shift+S"),
        Some((vec!["Ctrl", "Shift"], "S"))
    );
    assert_eq!(split_combination("W"), Some((vec![], "W")));
    assert_eq!(split_combination("Alt++"), Some((vec!["Alt"], "+")));
    for good in [
        "Alt+Left",
        "Ctrl+Alt+Shift+K",
        "Shift+F6",
        "Mouse Wheel",
        "Mouse WheelUp",
    ] {
        assert!(valid_input_name(good), "{good}");
    }
    for bad in [
        "Alt+",
        "Alt+Ctrl+K",
        "Ctrl+Ctrl+K",
        "Alt+Shift",
        "Shift+Alt+K",
        "Alt+Mouse Left",
        "Mouse Wheelie",
    ] {
        assert!(!valid_input_name(bad), "{bad}");
    }
}

#[test]
fn a_combination_shadows_its_key_on_its_own() {
    let b = Bindings::default();
    let mut d = Devices::new();
    // The left arrow alone pans.
    d.set("Left", true);
    let f = d.sample(&b, TICK);
    assert_eq!(f.get("Pan").axis, Vec2::new(-1.0, 0.0));
    assert_eq!(f.get("Turn").axis, Vec2::ZERO);
    // With Alt held it turns, and doesn't pan.
    d.set("Alt", true);
    let f = d.sample(&b, TICK);
    assert_eq!(f.get("Turn").axis, Vec2::new(-1.0, 0.0));
    assert_eq!(f.get("Pan").axis, Vec2::ZERO);
    // Alt held, the up arrow (with no Alt+Up bound) still pans.
    d.set("Up", true);
    let f = d.sample(&b, TICK);
    assert_eq!(f.get("Pan").axis, Vec2::new(0.0, 1.0));
    // A combination pressed counts as a press once.
    let mut b2 = Bindings::default();
    b2.actions.push(Action::button("Quick", &["Ctrl+S"]));
    let mut d = Devices::new();
    d.set("Ctrl", true);
    let _ = d.sample(&b2, TICK);
    d.set("S", true);
    let f = d.sample(&b2, TICK);
    assert!(f.get("Quick").pressed && f.get("Quick").held);
    let f = d.sample(&b2, TICK);
    assert!(!f.get("Quick").pressed && f.get("Quick").held);
    d.set("Ctrl", false);
    let f = d.sample(&b2, TICK);
    assert!(f.get("Quick").released);
}

#[test]
fn the_wheel_zooms_by_notches_and_keys_by_their_speed() {
    let b = Bindings::default();
    let mut d = Devices::new();
    d.wheel_turned(2.0);
    d.wheel_turned(0.5);
    let f = d.sample(&b, TICK);
    assert_eq!(f.get("Zoom").axis, Vec2::new(0.0, 2.5));
    // Gone the next tick.
    assert_eq!(d.sample(&b, TICK).get("Zoom").axis, Vec2::ZERO);
    // Page Down held: 8 steps a second, a tick's worth.
    d.set("PageDown", true);
    let f = d.sample(&b, TICK);
    assert!((f.get("Zoom").axis.y + 8.0 * TICK).abs() < 1e-6);
    // Wheel directions as buttons: a notch is a press for that tick.
    let mut b2 = Bindings::default();
    b2.actions.push(Action::button("Next", &["Mouse WheelUp"]));
    let mut d = Devices::new();
    d.wheel_turned(1.0);
    let f = d.sample(&b2, TICK);
    assert!(f.get("Next").pressed && f.get("Next").held);
    let f = d.sample(&b2, TICK);
    assert!(!f.get("Next").held && f.get("Next").released);
    // Nonsense is ignored.
    d.wheel_turned(f32::NAN);
    d.wheel_turned(f32::INFINITY);
    assert_eq!(d.sample(&b, TICK).get("Zoom").axis, Vec2::ZERO);
    d.wheel_turned(1.0);
    d.release_all();
    assert_eq!(d.sample(&b, TICK).get("Zoom").axis, Vec2::ZERO);
}

#[test]
fn an_older_project_gets_the_ready_made_actions_it_is_missing() {
    let mut b = Bindings {
        actions: Bindings::default().actions[..5].to_vec(),
    };
    b.get_mut("Jump").unwrap().inputs = vec!["E".into()];
    assert_eq!(b.add_missing(), ["Pan", "Turn", "Zoom", "Drag"]);
    // Its own changes stay.
    assert_eq!(b.get("Jump").unwrap().inputs, ["E"]);
    assert!(b.add_missing().is_empty());
}

#[test]
fn rebinding_never_grows_an_action_past_the_limit() {
    let mut b = Bindings::default();
    let keys: Vec<String> = (0..MAX_INPUTS).map(|n| format!("F{}", n + 1)).collect();
    b.get_mut("Jump").unwrap().inputs = keys;
    assert!(
        !b.rebind("Jump", "Pad A"),
        "full of keys: no room for a pad button"
    );
    assert_eq!(b.get("Jump").unwrap().inputs.len(), MAX_INPUTS);
    // A key replaces the keys, so there's room.
    assert!(b.rebind("Jump", "Space"));
    b.check().unwrap();
    assert!(Bindings::read(&b.write()).is_ok());
}

#[test]
fn a_tick_s_written_length_is_never_less_than_its_line() {
    let mut recording = Recording::default();
    for n in 0..50u64 {
        let mut frame = Frame::default();
        for k in 0..(n % 7) {
            frame.actions.insert(
                format!("Action_{k}_{}", "x".repeat(n as usize % 40)),
                ActionState {
                    held: true,
                    pressed: k % 2 == 0,
                    released: true,
                    axis: Vec2::new(-1.234_567, f32::MIN_POSITIVE),
                },
            );
        }
        recording.ticks.push(Tick {
            frame,
            world: u64::MAX,
            game: Some(u64::MAX - n),
        });
    }
    let header = Recording {
        ticks: Vec::new(),
        ..recording.clone()
    }
    .write()
    .len();
    let lines = recording.write().len() - header;
    let estimated: usize = recording.ticks.iter().map(Tick::written_len).sum();
    assert!(estimated >= lines, "{estimated} < {lines}");
}

#[test]
fn forgetting_changes_keeps_what_s_held() {
    let b = Bindings::default();
    let mut d = Devices::new();
    d.set("W", true);
    d.set("F", true);
    d.set("F", false);
    d.mouse_moved(Vec2::new(300.0, 0.0));
    d.wheel_turned(3.0);
    d.forget_changes();
    let f = d.sample(&b, TICK);
    assert!(f.get("Move").held, "W is still down");
    assert!(!f.get("Interact").pressed, "the tap is forgotten");
    assert_eq!(f.get("Look").axis, Vec2::ZERO);
    assert_eq!(f.get("Zoom").axis, Vec2::ZERO);
}

#[test]
fn older_files_naming_the_arrows_arrowup_read_as_the_keys_pressed() {
    let text = "[[action]]\nname = \"Pan\"\nkind = \"axis\"\nup = [\"ArrowUp\"]\nleft = [\"Alt+ArrowLeft\"]\n";
    let b = Bindings::read(text).unwrap();
    assert_eq!(b.get("Pan").unwrap().up, ["Up"]);
    assert_eq!(b.get("Pan").unwrap().left, ["Alt+Left"]);
}

#[test]
fn input_actions_carry_their_version_and_older_ones_upgrade() {
    use crate::BINDINGS_FORMAT;
    assert!(BINDINGS_FORMAT.is_complete());
    // Written as version 2; a file with no version line is version 1.
    let written = Bindings::default().write();
    assert_eq!((BINDINGS_FORMAT.version_of)(&written), Some(2));
    assert_eq!(Bindings::read(&written).unwrap(), Bindings::default());
    assert_eq!(
        (BINDINGS_FORMAT.version_of)("[[action]]\nname = \"A\"\n"),
        Some(1)
    );
    // Only a version 1 file is renamed: version 2 is read as written.
    let one = "[[action]]\nname = \"Pan\"\ninputs = [\"ArrowUp\"]\n";
    let two = format!("version = 2\n{one}");
    assert_eq!(
        Bindings::read(one).unwrap().get("Pan").unwrap().inputs,
        ["Up"]
    );
    assert_eq!(
        Bindings::read(&two).unwrap().get("Pan").unwrap().inputs,
        ["ArrowUp"]
    );
    // Newer, or a version that isn't a whole number: refused, saying why.
    let newer = Bindings::read("version = 3\n").unwrap_err();
    assert!(newer.to_string().contains("newer version"), "{newer}");
    for bad in ["version = \"two\"\n", "version = -1\n", "version = 0\n"] {
        assert!(Bindings::read(bad).is_err(), "{bad}");
    }
    // A damaged list in an old file is left for the reader to refuse.
    let odd = "[[action]]\nname = \"A\"\ninputs = [1, \"ArrowLeft\"]\n";
    assert!(matches!(
        Bindings::read(odd),
        Err(BindingsError::Damaged(_))
    ));
}

#[test]
fn the_wheel_down_as_a_button_and_up_never_mix() {
    // The hardening milestone's mutation testing: only a notch up was
    // tried as a button.
    let mut b = Bindings::default();
    b.actions.push(Action::button("Next", &["Mouse WheelUp"]));
    b.actions.push(Action::button("Back", &["Mouse WheelDown"]));
    let mut d = Devices::new();
    d.wheel_turned(-1.0);
    let f = d.sample(&b, TICK);
    assert!(f.get("Back").pressed && f.get("Back").held);
    assert!(!f.get("Next").pressed && !f.get("Next").held);
    d.wheel_turned(1.0);
    let f = d.sample(&b, TICK);
    assert!(f.get("Next").pressed && !f.get("Back").held);
    // No turn: neither.
    let f = d.sample(&b, TICK);
    assert!(!f.get("Next").held && !f.get("Back").held);
}

#[test]
fn a_combination_tapped_between_ticks_counts_only_with_its_modifiers_held() {
    // Mutation testing: a combination's tap (down and up between two
    // ticks) wasn't tried. An action is pressed when it starts being held
    // however that happens (the arrow, then Alt, starts Alt+Left); a tap
    // too quick to be held at a tick is a press only if the key went
    // down with the modifiers held.
    let mut b = Bindings::default();
    b.actions.push(Action::button("Back", &["Alt+Left"]));
    let mut d = Devices::new();
    d.set("Alt", true);
    let _ = d.sample(&b, TICK);
    d.set("Left", true);
    d.set("Left", false);
    let f = d.sample(&b, TICK);
    assert!(
        f.get("Back").pressed && f.get("Back").released,
        "{:?}",
        f.get("Back")
    );
    assert!(!f.get("Back").held);
    // The same tap without Alt isn't the combination.
    let mut d = Devices::new();
    d.set("Left", true);
    d.set("Left", false);
    let f = d.sample(&b, TICK);
    assert!(!f.get("Back").pressed, "{:?}", f.get("Back"));
    // The arrow held, then Alt: the action starts being held, a press.
    let mut d = Devices::new();
    d.set("Left", true);
    let _ = d.sample(&b, TICK);
    d.set("Alt", true);
    let f = d.sample(&b, TICK);
    assert!(f.get("Back").pressed && f.get("Back").held);
}

#[test]
fn a_tick_s_checksum_tells_held_pressed_and_released_apart() {
    // Mutation testing: the flags' bits could be mixed up unnoticed, and a
    // recording would then miss a difference.
    let sum = |state: ActionState| {
        let mut frame = Frame::default();
        frame.actions.insert("Jump".into(), state);
        let mut sum = reflect::Checksum::new();
        frame.checksum(&mut sum);
        sum.finish()
    };
    let flags = [
        (false, false, false),
        (true, false, false),
        (false, true, false),
        (false, false, true),
        (true, true, false),
        (true, false, true),
        (false, true, true),
        (true, true, true),
    ];
    let sums: std::collections::BTreeSet<u64> = flags
        .iter()
        .map(|&(held, pressed, released)| {
            sum(ActionState {
                held,
                pressed,
                released,
                ..ActionState::default()
            })
        })
        .collect();
    assert_eq!(sums.len(), flags.len(), "every combination differs");
    let empty = {
        let mut s = reflect::Checksum::new();
        Frame::default().checksum(&mut s);
        s.finish()
    };
    assert!(!sums.contains(&empty));
}

#[test]
fn a_recording_holds_an_hour_of_ticks_and_no_more_and_says_which_line_is_wrong() {
    // Mutation testing: the limit's edge and the line numbers in its
    // messages weren't checked.
    let mut text = String::from("solis recording 1\nseed 1\nstart 0\n");
    for n in 0..recording::MAX_TICKS {
        text.push_str(&format!("{n} w 0\n"));
    }
    assert_eq!(
        Recording::read(&text).unwrap().ticks.len(),
        recording::MAX_TICKS
    );
    text.push_str(&format!("{} w 0\n", recording::MAX_TICKS));
    assert_eq!(Recording::read(&text), Err(RecordingError::TooBig));
    // A wrong tick on the file's fifth line says line 5.
    let text = "solis recording 1\nseed 1\nstart 0\n0 w 0\n7 w 0\n";
    assert_eq!(Recording::read(text), Err(RecordingError::Damaged(5)));
    let text = "solis recording 1\nseed one\nstart 0\n";
    assert_eq!(Recording::read(text), Err(RecordingError::Damaged(2)));
    let text = "solis recording 1\nseed 1\nbegin 0\n";
    assert_eq!(Recording::read(text), Err(RecordingError::Damaged(3)));
}

/// Each limit at its very edge, and the rest the hardening milestone's
/// mutation testing found untested.
#[path = "edge_tests.rs"]
mod edges;

#[test]
fn a_recording_names_its_area_and_older_ones_don_t() {
    let r = sample_recording();
    let text = r.write();
    assert!(text.contains("\narea areas/harbour.solarea\n"), "{text}");
    assert_eq!(
        Recording::read(&text).unwrap().area.as_deref(),
        Some("areas/harbour.solarea")
    );
    // Not known: an empty line, read back as none.
    let none = Recording {
        area: None,
        ..r.clone()
    };
    assert_eq!(Recording::read(&none.write()).unwrap().area, None);
    let bare = none.write().replacen("\narea \n", "\narea\n", 1);
    assert_eq!(Recording::read(&bare).unwrap().area, None);
    // A line break in a path can't break the file.
    let odd = Recording {
        area: Some("a\nb".into()),
        ..r.clone()
    };
    assert_eq!(
        Recording::read(&odd.write()).unwrap().area.as_deref(),
        Some("a b")
    );
    // Version 1 had no area line: it reads, with none.
    let one = text
        .replacen("solis recording 2", "solis recording 1", 1)
        .replacen("area areas/harbour.solarea\n", "", 1);
    let read = Recording::read(&one).unwrap();
    assert_eq!((read.area, read.ticks.len()), (None, r.ticks.len()));
    // Too long, or a control character: damaged.
    let long = text.replacen("areas/harbour.solarea", &"x".repeat(2000), 1);
    assert!(matches!(
        Recording::read(&long),
        Err(RecordingError::Damaged(3))
    ));
    let missing = text.replacen("area areas/harbour.solarea\n", "", 1);
    assert!(Recording::read(&missing).is_err());
}
