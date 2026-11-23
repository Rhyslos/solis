use super::*;

fn frames(text: &str, ticks: usize) -> Vec<Frame> {
    let mut script = Script::read(text).unwrap();
    (0..ticks).map(|_| script.next_frame()).collect()
}

#[test]
fn a_held_button_is_pressed_then_held_then_released() {
    let f = frames("at 1 hold Jump\nat 3 release Jump\n", 5);
    assert!(f[0].actions.is_empty());
    let jump = |t: usize| f[t].get("Jump");
    assert!(jump(1).pressed && jump(1).held && !jump(1).released);
    assert!(!jump(2).pressed && jump(2).held);
    assert!(jump(3).released && !jump(3).held);
    assert!(f[4].actions.is_empty());
}

#[test]
fn a_press_lasts_one_tick() {
    let f = frames("at 0 press Interact -- a comment\n", 3);
    assert!(f[0].get("Interact").pressed && f[0].get("Interact").held);
    assert!(f[1].get("Interact").released && !f[1].get("Interact").held);
    assert!(f[2].actions.is_empty());
}

#[test]
fn an_axis_stays_until_it_changes_or_rests() {
    let f = frames("at 2 hold Move 0 0\nat 0 hold Move 1 -0.5\n", 3);
    assert_eq!(f[0].get("Move").axis, Vec2::new(1.0, -0.5));
    assert_eq!(f[1].get("Move").axis, Vec2::new(1.0, -0.5));
    assert!(f[2].actions.is_empty(), "at rest");
}

#[test]
fn the_script_names_its_actions_and_last_tick() {
    let script = Script::read("at 5 hold Sprint\nat 90 release Sprint\nat 7 press Jump\n").unwrap();
    assert_eq!(script.actions(), ["Jump", "Sprint"]);
    assert_eq!(script.last_tick(), 90);
}

#[test]
fn mistakes_say_their_line() {
    for (text, says) in [
        ("hold Jump", "line 1: write it as"),
        ("\nat ten press Jump", "line 2: ten isn't a tick"),
        ("at 1 press jump-up", "isn't an action's name"),
        ("at 1 press Jump 1 0", "press is for buttons"),
        ("at 1 hold Move 2 0", "2 isn't a number from -1 to 1"),
        ("at 1 tap Jump", "write it as"),
        ("at 999999999 press Jump", "isn't a tick"),
    ] {
        let why = Script::read(text).unwrap_err();
        assert!(why.contains(says), "{text}: {why}");
    }
    assert!(Script::read("").unwrap().actions().is_empty());
}

#[test]
fn hostile_text_is_refused_or_read_without_trouble() {
    assert!(Script::read(&"x".repeat(MAX_BYTES + 1)).is_err());
    let many = "at 1 press Jump\n".repeat(MAX_LINES + 1);
    assert!(Script::read(&many).unwrap_err().contains("more than"));
    // Any bytes at all: a mistake or a script, never a panic.
    let mut seed: u64 = 7;
    for _ in 0..2_000 {
        let text: String = (0..40)
            .map(|_| {
                seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
                let pool = b" at01hold pressMoveJump-.\n\t9";
                char::from(pool[(seed >> 33) as usize % pool.len()])
            })
            .collect();
        if let Ok(mut s) = Script::read(&text) {
            let _ = s.next_frame();
        }
    }
}
