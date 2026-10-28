//! Animators at work: transitions on the right tick, triggers put out,
//! the clip by facing, speeds, game code's settings, and carrying a run
//! over a changed file.

use super::*;
use crate::animator_file::tests::farmer;
use crate::animator_file::{AnimatorFile, Condition, Facing, ParameterKind, Plays, Transition};

fn idle_walk_tool(a: &Animator, run: &Run) -> &'static str {
    match a.state_name(run.state) {
        "Idle" => "Idle",
        "Walk" => "Walk",
        "Use tool" => "Use tool",
        other => panic!("{other}"),
    }
}

#[test]
fn transitions_are_taken_when_their_conditions_hold() {
    let a = Animator::new(&farmer());
    assert!(a.notes.is_empty(), "{:?}", a.notes);
    let mut run = a.start("");
    assert_eq!(idle_walk_tool(&a, &run), "Idle");
    assert!(!a.step(&mut run, false), "nothing set: stays");
    a.set(&mut run, "moving", ParamValue::Switch(true)).unwrap();
    assert!(a.step(&mut run, false));
    assert_eq!(idle_walk_tool(&a, &run), "Walk");
    assert!(
        !a.step(&mut run, false),
        "one transition a tick, and none back"
    );
    // Swing from any state; the trigger is used up.
    a.set(&mut run, "swing", ParamValue::Trigger(false))
        .unwrap();
    assert!(a.step(&mut run, false));
    assert_eq!(idle_walk_tool(&a, &run), "Use tool");
    assert_eq!(run.values[3], ParamValue::Trigger(false), "put out");
    // A trigger fired again while in Use tool doesn't restart it, and
    // doesn't wait to be used later.
    a.set(&mut run, "swing", ParamValue::Trigger(true)).unwrap();
    assert!(!a.step(&mut run, false));
    assert_eq!(run.values[3], ParamValue::Trigger(false));
    // The tool's clip finishes: back to Walk (still moving).
    assert!(a.step(&mut run, true));
    assert_eq!(idle_walk_tool(&a, &run), "Walk");
    a.set(&mut run, "moving", ParamValue::Switch(false))
        .unwrap();
    a.step(&mut run, false);
    assert_eq!(idle_walk_tool(&a, &run), "Idle");
}

#[test]
fn numbers_compare_and_earlier_transitions_win() {
    let mut file = farmer();
    // Walk to Idle when slow, before "not moving" (which comes first).
    file.transitions.insert(
        0,
        Transition {
            id: reflect::Id::from_raw(99).unwrap(),
            from: Some(file.states[1].id),
            to: file.states[2].id,
            when: vec![
                Condition::Above("speed".into(), 3.0),
                Condition::Is("moving".into()),
            ],
        },
    );
    let a = Animator::new(&file);
    let mut run = a.start("Walk");
    a.set(&mut run, "moving", ParamValue::Switch(true)).unwrap();
    a.set(&mut run, "speed", ParamValue::Number(3.0)).unwrap();
    assert!(!a.step(&mut run, false), "3 isn't above 3");
    a.set(&mut run, "speed", ParamValue::Number(3.5)).unwrap();
    assert!(a.step(&mut run, false));
    assert_eq!(a.state_name(run.state), "Use tool");
}

#[test]
fn the_clip_is_the_one_nearest_facing_and_stays_on_a_tie() {
    let a = Animator::new(&farmer());
    let mut run = a.start("Walk");
    assert_eq!(a.clip(&run, ""), "walk_down", "facing starts down");
    a.set(&mut run, "facing", ParamValue::Direction([3.0, 0.5]))
        .unwrap();
    assert_eq!(a.clip(&run, "walk_down"), "walk_right");
    // Exactly between up and right: whichever plays stays.
    a.set(&mut run, "facing", ParamValue::Direction([1.0, 1.0]))
        .unwrap();
    assert_eq!(a.clip(&run, "walk_up"), "walk_up");
    assert_eq!(a.clip(&run, "walk_right"), "walk_right");
    // No length: keeps facing as it was.
    a.set(&mut run, "facing", ParamValue::Direction([0.0, 0.0]))
        .unwrap();
    assert_eq!(
        run.values[0],
        ParamValue::Direction(Facing::UpRight.vector())
    );
    // A state that plays one clip.
    let mut file = farmer();
    file.states[0].plays = Plays::Clip("idle".into());
    let a = Animator::new(&file);
    assert_eq!(a.clip(&a.start(""), "walk_up"), "idle");
}

#[test]
fn speed_is_the_state_s_times_its_number() {
    let a = Animator::new(&farmer());
    let mut run = a.start("Walk");
    assert_eq!(a.speed(&run), 1.0);
    a.set(&mut run, "speed", ParamValue::Number(2.5)).unwrap();
    assert_eq!(a.speed(&run), 2.5);
    a.set(&mut run, "speed", ParamValue::Number(-4.0)).unwrap();
    assert_eq!(a.speed(&run), 0.0);
    a.set(&mut run, "speed", ParamValue::Number(1.0e6)).unwrap();
    assert_eq!(a.speed(&run), 16.0);
    run.state = a.state_named("Idle").unwrap();
    assert_eq!(a.speed(&run), 1.0, "Idle isn't sped by it");
}

#[test]
fn game_code_s_settings_are_checked() {
    let a = Animator::new(&farmer());
    let mut run = a.start("");
    assert_eq!(
        a.set(&mut run, "jump", ParamValue::Trigger(true)),
        Err(SetError::NoParameter)
    );
    assert_eq!(
        a.set(&mut run, "moving", ParamValue::Number(1.0)),
        Err(SetError::WrongKind("switch"))
    );
    assert_eq!(
        a.set(&mut run, "speed", ParamValue::Number(f32::NAN)),
        Err(SetError::NotUsable)
    );
    assert_eq!(run, a.start(""), "nothing changed");
}

#[test]
fn a_changed_file_carries_the_run_over_by_name() {
    let before = Animator::new(&farmer());
    let mut run = before.start("Walk");
    before
        .set(&mut run, "speed", ParamValue::Number(2.0))
        .unwrap();
    before
        .set(&mut run, "moving", ParamValue::Switch(true))
        .unwrap();
    let mut file = farmer();
    // A new parameter first, moving made a number, Walk renamed.
    file.parameters[1].kind = ParameterKind::Number(0.0);
    file.parameters.insert(
        0,
        crate::animator_file::Parameter {
            id: reflect::Id::from_raw(50).unwrap(),
            name: "tired".into(),
            kind: ParameterKind::Switch(true),
        },
    );
    let after = Animator::new(&file);
    let carried = after.carry(&before, &run);
    assert_eq!(after.state_name(carried.state), "Walk");
    assert_eq!(carried.values[0], ParamValue::Switch(true), "its start");
    assert_eq!(carried.values[2], ParamValue::Number(0.0), "changed kind");
    assert_eq!(carried.values[3], ParamValue::Number(2.0), "kept");
    file.states[1].name = "Stroll".into();
    let renamed = Animator::new(&file);
    assert_eq!(
        renamed.carry(&before, &run).state,
        renamed.entry(),
        "a state that's gone: the entry"
    );
}

#[test]
fn an_empty_animator_or_a_stray_run_is_safe() {
    let a = Animator::new(&AnimatorFile::default());
    let mut run = a.start("Walk");
    assert!(!a.step(&mut run, true));
    assert_eq!(a.clip(&run, "x"), "");
    assert_eq!(a.speed(&run), 1.0);
    let farmer = Animator::new(&farmer());
    let mut stray = Run {
        state: 40,
        values: Vec::new(),
    };
    farmer.step(&mut stray, false);
    assert_eq!(stray, farmer.start(""));
    // A condition on a parameter that isn't there never holds, with a note.
    let mut file = crate::animator_file::tests::farmer();
    file.transitions[0].when = vec![Condition::Is("jump".into())];
    file.transitions[1].when = vec![Condition::Above("moving".into(), 1.0)];
    let a = Animator::new(&file);
    assert_eq!(a.notes.len(), 2);
    let mut run = a.start("");
    a.step(&mut run, false);
    assert_eq!(a.state_name(run.state), "Idle");
}

#[test]
fn the_same_settings_give_the_same_states() {
    // A long run driven by a fixed pattern twice gives the same states.
    let a = Animator::new(&farmer());
    let go = || {
        let mut run = a.start("");
        let mut seen = Vec::new();
        for tick in 0u32..500 {
            a.set(&mut run, "moving", ParamValue::Switch(tick % 37 < 20))
                .unwrap();
            if tick % 53 == 0 {
                a.set(&mut run, "swing", ParamValue::Trigger(true)).unwrap();
            }
            a.step(&mut run, tick % 11 == 0);
            seen.push(run.state);
        }
        seen
    };
    assert_eq!(go(), go());
}
