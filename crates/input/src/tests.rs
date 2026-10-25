//! Input: bindings and their file, sampling, recordings, and damaged
//! files never crashing.

use super::*;
use glam::Vec2;

const TICK: f32 = 1.0 / 60.0;

#[test]
fn the_ready_made_actions_are_the_chosen_controls() {
    let b = Bindings::default();
    let names: Vec<&str> = b.actions.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(names, ["Move", "Look", "Jump", "Sprint", "Interact"]);
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
    }
}

#[test]
fn recordings_round_trip_exactly() {
    let r = sample_recording();
    let text = r.write();
    assert!(text.starts_with("solis recording 1\nseed 42\n"));
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
    assert_eq!(
        Recording::read("solis recording 2\nseed 1\nstart 0\n"),
        Err(RecordingError::NotARecording)
    );
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
        "ArrowUp",
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
