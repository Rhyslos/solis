//! Editing an animator: states made, renamed, moved and taken away;
//! clips by direction; transitions and their conditions; parameters
//! renamed and removed with what uses them; undo and redo; the preview;
//! and changes on disk told apart from Solis's own saves.

use super::*;
use animation::{Clip, ClipFrame, Direction};

fn editor() -> AnimatorEditor {
    AnimatorEditor::new("sprites/farmer.solanim".into(), AnimatorFile::default())
}

/// walk_down and walk_up (4 frames of 100 ms, over and over), and hoe
/// (2 frames, once).
fn clips() -> ClipSet {
    let frames = |n: u32| -> Vec<ClipFrame> {
        (0..n)
            .map(|i| ClipFrame {
                frame: i,
                ms: 100,
                events: Vec::new(),
            })
            .collect()
    };
    ClipSet {
        clips: vec![
            Clip::new("walk_down", frames(4), Direction::Forward, 0, 1.0, false),
            Clip::new("walk_up", frames(4), Direction::Forward, 0, 1.0, false),
            Clip::new("hoe", frames(2), Direction::Forward, 1, 1.0, false),
        ],
        first: Some(0),
        notes: Vec::new(),
    }
}

#[test]
fn states_are_added_named_moved_started_in_and_taken_away() {
    let mut e = editor();
    assert!(e.add_state([100.0, 50.0]));
    let idle = e.file.states[0].id;
    assert_eq!(e.file.entry, Some(idle), "the first is where it starts");
    assert_eq!(e.file.states[0].place, [25.0, 13.0], "centred on the click");
    assert!(e.add_state([300.0, 50.0]));
    let walk = e.file.states[1].id;
    assert_eq!(e.file.states[1].name, "State 2", "a free name");
    assert!(e.rename_state(idle, " Idle "));
    assert!(e.rename_state(walk, "Idle"), "taken: given a free one");
    assert_eq!(e.file.states[1].name, "Idle 2");
    assert!(!e.rename_state(walk, "   "));
    assert!(e.rename_state(walk, "Walk"));
    assert!(e.move_states(&[Some(walk), None], [10.0, -5.0]));
    assert_eq!(e.file.states[1].place, [235.0, 8.0]);
    assert_eq!(e.file.any_place, [10.0, 235.0], "Any state moves too");
    assert!(e.set_entry(walk));
    assert!(e.join(Some(idle), walk));
    assert!(e.join(None, idle));
    // Taking Walk away takes its transitions, and it starts in Idle.
    e.picked = Picked::States([Some(walk)].into());
    assert!(e.delete_picked());
    assert_eq!(e.file.states.len(), 1);
    assert_eq!(e.file.transitions.len(), 1, "Any state's to Idle stays");
    assert_eq!(e.file.entry, Some(idle));
    assert_eq!(e.picked, Picked::Nothing);
    // Any state itself can't be taken away.
    e.picked = Picked::States([None].into());
    assert!(!e.delete_picked());
}

#[test]
fn a_clip_by_direction_makes_a_facing_parameter_and_fills_its_directions() {
    let mut e = editor();
    e.add_state([0.0, 0.0]);
    let id = e.file.states[0].id;
    assert!(e.set_clip(id, "walk_down"));
    assert!(e.set_by_direction(id, true));
    assert_eq!(e.file.parameters[0].name, "facing");
    assert_eq!(
        e.file.states[0].plays,
        Plays::ByDirection {
            by: "facing".into(),
            clips: vec![(Facing::Down, "walk_down".into())],
        }
    );
    // Up, then right (the nearest's clip), then left.
    assert!(e.add_direction(id));
    assert!(e.set_direction_clip(id, Facing::Up, "walk_up"));
    assert!(e.add_direction(id));
    let Plays::ByDirection { clips, .. } = &e.file.states[0].plays else {
        panic!()
    };
    assert_eq!(clips[2].0, Facing::Right);
    for _ in 0..10 {
        e.add_direction(id);
    }
    let Plays::ByDirection { clips, .. } = &e.file.states[0].plays else {
        panic!()
    };
    assert_eq!(clips.len(), 8, "eight at most");
    assert!(e.remove_direction(id, Facing::UpLeft));
    assert!(!e.remove_direction(id, Facing::UpLeft));
    // Back to one clip: the one facing down.
    assert!(e.set_by_direction(id, false));
    assert_eq!(e.file.states[0].plays, Plays::Clip("walk_down".into()));
    assert!(!e.set_by_direction(id, false), "already one clip");
    // A second direction parameter, chosen.
    e.add_parameter(ParameterKind::Direction(Facing::Up));
    e.set_by_direction(id, true);
    assert!(e.set_direction_by(id, "facing2"));
    assert!(!e.set_direction_by(id, "nothing"));
    assert!(e.set_state_speed(id, 40.0));
    assert_eq!(e.file.states[0].speed, 16.0);
    assert!(!e.set_state_speed(id, f32::NAN));
    assert!(!e.set_speed_by(id, "facing"), "not a number");
    e.add_parameter(ParameterKind::Number(0.0));
    assert!(e.set_speed_by(id, "speed"));
}

#[test]
fn transitions_and_their_conditions_follow_the_parameters() {
    let mut e = editor();
    e.add_state([0.0, 0.0]);
    e.add_state([200.0, 0.0]);
    let (a, b) = (e.file.states[0].id, e.file.states[1].id);
    assert!(!e.join(Some(a), a), "not to itself");
    assert!(e.join(Some(a), b));
    let t = e.file.transitions[0].id;
    assert_eq!(e.picked, Picked::Transition(t));
    assert!(e.add_condition(t));
    assert_eq!(
        e.file.transitions[0].when,
        [Condition::Finished],
        "no parameters yet"
    );
    e.add_parameter(ParameterKind::Switch(false));
    e.add_parameter(ParameterKind::Number(0.0));
    assert!(e.add_condition(t));
    assert_eq!(
        e.file.transitions[0].when[1],
        Condition::Is("moving".into())
    );
    assert!(e.set_condition(t, 1, Condition::Not("moving".into())));
    assert!(e.set_condition(t, 0, Condition::Above("speed".into(), 2.0)));
    // Renaming carries into conditions and speeds; removing takes them.
    e.set_speed_by(a, "speed");
    assert!(e.rename_parameter("speed", "pace"));
    assert_eq!(
        e.file.transitions[0].when[0],
        Condition::Above("pace".into(), 2.0)
    );
    assert_eq!(e.file.states[0].speed_by, "pace");
    assert!(!e.rename_parameter("pace", "moving"), "taken");
    assert!(!e.rename_parameter("pace", "two words"));
    assert!(e.delete_parameter("pace"));
    assert_eq!(
        e.file.transitions[0].when,
        [Condition::Not("moving".into())]
    );
    assert!(e.file.states[0].speed_by.is_empty());
    assert!(e.remove_condition(t, 0));
    assert!(!e.remove_condition(t, 0));
    // Order among those leaving the same state.
    e.join(Some(a), b);
    let t2 = e.file.transitions[1].id;
    assert!(e.move_transition(t2, true));
    assert_eq!(e.file.transitions[0].id, t2);
    assert!(!e.move_transition(t2, true), "already first");
    assert!(e.move_transition(t2, false));
    // Starts.
    assert!(e.set_parameter_start("moving", ParameterKind::Switch(true)));
    assert!(
        !e.set_parameter_start("moving", ParameterKind::Number(1.0)),
        "its kind stays"
    );
}

#[test]
fn undo_and_redo_step_through_changes_and_the_text_reads_back() {
    let mut e = editor();
    e.add_state([0.0, 0.0]);
    e.add_parameter(ParameterKind::Trigger);
    assert!(e.undo());
    assert!(e.file.parameters.is_empty());
    assert!(e.undo());
    assert!(e.file.states.is_empty());
    assert!(!e.undo());
    assert!(e.redo());
    assert!(e.redo());
    assert!(!e.redo());
    assert_eq!(e.file.parameters.len(), 1);
    e.undo();
    e.add_state([5.0, 5.0]);
    assert!(!e.redo(), "a new change forgets what could be redone");
    // Nothing changed: no undo step.
    let before = e.file.clone();
    assert!(!e.rename_state(before.states[0].id, &before.states[0].name.clone()));
    let back = animator_file::read(&e.text()).unwrap();
    assert!(back.notes.is_empty(), "{:?}", back.notes);
    assert_eq!(back.file, e.file);
}

#[test]
fn the_preview_runs_the_animator_on_its_own_values() {
    let mut e = editor();
    e.add_state([0.0, 0.0]);
    e.add_state([200.0, 0.0]);
    let (idle, tool) = (e.file.states[0].id, e.file.states[1].id);
    e.set_clip(idle, "walk_down");
    e.set_clip(tool, "hoe");
    e.add_parameter(ParameterKind::Trigger);
    e.join(None, tool);
    let t = e.file.transitions[0].id;
    e.add_condition(t);
    e.join(Some(tool), idle);
    let back = e.file.transitions[1].id;
    e.add_condition(back);
    assert!(e.set_condition(back, 0, Condition::Finished));
    let set = clips();
    e.play_preview(0.1, Some(&set));
    assert_eq!(e.preview_clip, "walk_down");
    e.set_preview("use", ParamValue::Trigger(true));
    e.play_preview(world::TICK, Some(&set));
    assert_eq!(e.preview_clip, "hoe");
    // The hoe plays once (0.2 s), then back.
    e.play_preview(0.25, Some(&set));
    e.play_preview(0.05, Some(&set));
    assert_eq!(e.preview_clip, "walk_down");
    // A change to the file keeps the preview's place by name.
    e.rename_state(idle, "Rest");
    assert_eq!(e.running.state_name(e.preview.state), "Rest");
    e.restart_preview();
    assert!(e.preview_clip.is_empty());
    // Nonsense times are ignored.
    e.play_preview(f64::NAN, Some(&set));
    e.play_preview(-1.0, None);
}

#[test]
fn its_own_saves_are_told_apart_from_changes_on_disk() {
    let mut e = editor();
    e.add_state([0.0, 0.0]);
    e.saving();
    let saved = e.file.clone();
    e.add_state([200.0, 0.0]);
    e.saving();
    assert!(e.is_own(&saved), "an older save arriving late");
    assert!(e.is_own(&e.file.clone()));
    let mut hand = e.file.clone();
    hand.states[0].name = "By hand".into();
    assert!(!e.is_own(&hand));
    e.reload(hand.clone());
    assert_eq!(e.file, hand);
    assert!(e.undo(), "a reload is one undo step");
}

#[test]
fn picking_adds_takes_away_and_forgets_what_s_gone() {
    let mut e = editor();
    e.add_state([0.0, 0.0]);
    e.add_state([200.0, 0.0]);
    let (a, b) = (e.file.states[0].id, e.file.states[1].id);
    e.pick_state(Some(a), false);
    e.pick_state(Some(b), true);
    e.pick_state(None, true);
    assert_eq!(e.picked, Picked::States([Some(a), Some(b), None].into()));
    assert!(e.picked_state().is_none(), "more than one");
    e.pick_state(Some(b), true);
    e.pick_state(None, true);
    assert_eq!(e.picked_state().map(|s| s.id), Some(a));
    e.pick_state(Some(a), true);
    assert_eq!(e.picked, Picked::Nothing);
    e.pick_state(Some(b), false);
    e.undo();
    assert_eq!(e.picked, Picked::Nothing, "b is gone");
    assert!(e.picked_transition().is_none());
}
