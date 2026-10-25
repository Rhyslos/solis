//! The game's input in the editor: actions reach game code, game code
//! rebinds, the editor's keys stay out of the game's way, and recordings
//! play back the same.

use crate::test_support::*;
use crate::workbench::{Workbench, WorkbenchAction};
use area::EntityData;
use area::builtin::Transform;
use reflect::EntityId;
use std::sync::Arc;

/// A walker: walks along Move, hops on Jump, says when Interact is
/// pressed and which key it is, and rebinds Jump to J when Sprint is
/// pressed.
const WALKER: &str = "get \"solis/world\":[say];\n\
    get \"solis/entity\":[positionOf, moveTo];\n\
    get \"solis/input\":[held, pressed, axis, bindingOf, rebind];\n\
    component Walker {\n\
        method update(float dt) {\n\
            vec3 way = axis(\"Move\");\n\
            vec3 step = [way.x, way.y, 0];\n\
            moveTo(me, positionOf(me) + step * 0.1);\n\
            is held(\"Jump\")? then {\n\
                moveTo(me, positionOf(me) + step * 0.1 + [0, 0, 1]);\n\
            }\n\
            is pressed(\"Interact\")? then {\n\
                say(\"interact with \" + bindingOf(\"Interact\"));\n\
            }\n\
            is pressed(\"Sprint\")? then {\n\
                rebind(\"Jump\", \"J\");\n\
            }\n\
        }\n\
    }\n\
    pass Walker;\nend\n";

fn program(main: &str) -> Arc<script::Compiled> {
    let mut files = script::describe::descriptions("Test Engine 9.9.9", "solis");
    files.push(("main.lst".into(), main.to_string()));
    let code = script::Code {
        files,
        lua: Vec::new(),
        notes: Vec::new(),
    };
    match script::compile(&code) {
        Ok(Some(c)) => Arc::new(c),
        Ok(None) => panic!("no code"),
        Err(problems) => panic!("{}", problems[0]),
    }
}

const WALKER_ID: u64 = 7;

/// A workbench with a walker in its area.
fn with_a_walker() -> Workbench {
    let mut w = Workbench::new(info(), None);
    w.set_program(Some(program(WALKER)));
    let mut data = EntityData::new("Walker").with(&Transform::default());
    data.components
        .insert("Walker".into(), reflect::ComponentData::default());
    w.document
        .edit(
            "Add",
            vec![area::Edit::Spawn {
                entities: vec![(EntityId(reflect::Id::from_raw(WALKER_ID).unwrap()), data)],
            }],
            None,
            None,
        )
        .unwrap();
    w
}

fn walker_at(w: &Workbench) -> glam::Vec3 {
    let id = EntityId(reflect::Id::from_raw(WALKER_ID).unwrap());
    w.world().unwrap().global(id).unwrap().w_axis.truncate()
}

/// Runs exactly one tick.
fn tick(w: &mut Workbench) {
    let before = w.world().unwrap().clock().ticks;
    while w.world().unwrap().clock().ticks == before {
        w.run_game((world::TICK * 0.6) as f32);
    }
}

#[test]
fn actions_reach_game_code_once_per_tick() {
    let mut w = with_a_walker();
    w.play();
    tick(&mut w);
    assert_eq!(walker_at(&w), glam::Vec3::ZERO, "no input: still");
    w.devices.set("W", true);
    tick(&mut w);
    let p = walker_at(&w);
    assert!((p.y - 0.1).abs() < 1e-5 && p.x.abs() < 1e-6, "forward: {p}");
    w.devices.set("Space", true);
    tick(&mut w);
    assert!(walker_at(&w).z > 0.9, "hopped");
    w.devices.set("Space", false);
    w.devices.set("W", false);
    // A tap of F between ticks is still a press, said once.
    w.devices.set("F", true);
    w.devices.set("F", false);
    tick(&mut w);
    tick(&mut w);
    let said: Vec<&str> = w
        .log_lines()
        .iter()
        .map(|l| l.text.as_str())
        .filter(|t| t.contains("interact with"))
        .collect();
    assert_eq!(said, ["interact with F"]);
}

#[test]
fn game_code_rebinds_for_this_play_only() {
    let mut w = with_a_walker();
    w.play();
    w.devices.set("Shift", true);
    tick(&mut w);
    w.devices.set("Shift", false);
    tick(&mut w);
    // Jump is J now; Space no longer.
    w.devices.set("Space", true);
    tick(&mut w);
    assert!(walker_at(&w).z < 0.5, "Space doesn't jump");
    w.devices.set("Space", false);
    w.devices.set("J", true);
    tick(&mut w);
    assert!(walker_at(&w).z > 0.9, "J jumps");
    assert_eq!(
        w.playing_bindings().unwrap().get("Jump").unwrap().inputs,
        ["Pad A", "J"]
    );
    // The project's own actions are as they were.
    w.stop();
    assert_eq!(w.bindings().get("Jump").unwrap().inputs, ["Space", "Pad A"]);
}

#[test]
fn the_game_has_the_keys_until_esc_and_the_editor_s_are_off_meanwhile() {
    let mut w = with_a_walker();
    w.play();
    let mut h = harness(w);
    h.run();
    h.state_mut().0.give_game_input(&egui::Context::default());
    assert!(h.state().0.game_has_input());
    let tool = h.state().0.tool();
    // W is the game's: the move tool isn't picked.
    h.key_down(egui::Key::R);
    h.run();
    assert_eq!(h.state().0.tool(), tool, "editor keys are off");
    h.key_up(egui::Key::R);
    h.key_press(egui::Key::Escape);
    h.run();
    assert!(!h.state().0.game_has_input(), "Esc gives them back");
    h.key_press(egui::Key::R);
    h.run();
    assert_eq!(h.state().0.tool(), crate::viewport::Tool::Rotate);
    // Stop gives them back too.
    h.state_mut().0.give_game_input(&egui::Context::default());
    h.state_mut().0.stop();
    assert!(!h.state().0.game_has_input());
}

/// Plays the walker for `ticks` while recording, pressing W then Space
/// on the way; the recording's text.
fn record(w: &mut Workbench, ticks: u64) -> String {
    w.fixed_seed = Some(5);
    w.play();
    let playing = w.playing.as_mut().unwrap();
    playing.recording = Some(input::Recording {
        seed: playing.world.seed(),
        start: playing.world.checksum(),
        ticks: Vec::new(),
    });
    for n in 0..ticks {
        w.devices.set("W", n % 20 < 10);
        w.devices.set("Space", n % 30 == 0);
        tick(w);
    }
    w.devices.release_all();
    w.stop();
    let saved = w
        .outbox
        .iter()
        .find_map(|a| match a {
            WorkbenchAction::SaveRecording(text) => Some(text.clone()),
            _ => None,
        })
        .expect("Stop saves the recording");
    w.outbox.clear();
    saved
}

#[test]
fn a_recording_plays_back_exactly_and_a_changed_game_is_caught() {
    let mut w = with_a_walker();
    let text = record(&mut w, 90);
    let recording = input::Recording::read(&text).unwrap();
    assert_eq!(recording.ticks.len(), 90);
    assert_eq!(recording.seed, 5);
    // Played back (in frames of two ticks, where it was recorded in
    // frames of under one: the game mustn't depend on that): the same,
    // tick by tick, then paused with the verdict.
    w.fixed_seed = None;
    w.replay(&text, "walk.solrec");
    for _ in 0..200 {
        if w.play_state() == crate::play::PlayState::Paused {
            break;
        }
        w.run_game((world::TICK * 2.0) as f32);
    }
    assert_eq!(w.play_state(), crate::play::PlayState::Paused);
    assert!(
        w.log_lines()
            .iter()
            .any(|l| l.text.contains("matched the recording on all 90 ticks")),
        "{:?}",
        w.log_lines().iter().map(|l| &l.text).collect::<Vec<_>>()
    );
    assert_eq!(
        w.fixed_seed, None,
        "the recorded seed was for that play only"
    );
    w.stop();
    w.outbox.clear();
    // The walker's code changed since: the replay says where it stopped
    // matching.
    w.set_program(Some(program(&WALKER.replace("* 0.1", "* 0.2"))));
    w.replay(&text, "walk.solrec");
    for _ in 0..200 {
        if w.play_state() == crate::play::PlayState::Paused {
            break;
        }
        w.run_game((world::TICK * 2.0) as f32);
    }
    assert!(
        w.log_lines()
            .iter()
            .any(|l| l.text.contains("stopped matching the recording at tick"))
    );
    // A damaged recording is refused with why.
    w.stop();
    w.replay("solis recording 1\nseed x\n", "bad.solrec");
    assert!(
        w.log_lines()
            .last()
            .unwrap()
            .text
            .contains("Couldn't play back bad.solrec")
    );
    assert!(w.playing.is_none());
}

#[test]
fn recording_needs_a_project_and_starts_from_play() {
    let mut w = with_a_walker();
    w.play_and_record();
    assert!(
        w.log_lines()
            .last()
            .unwrap()
            .text
            .contains("kept in a project")
    );
    assert!(w.playing.is_none());
}

#[test]
fn the_input_actions_window_binds_what_is_pressed_and_saves() {
    let mut w = Workbench::new(info(), None);
    w.project = Some(crate::library::ProjectInfo {
        name: "Test".into(),
        root: std::env::temp_dir(),
        assets: std::env::temp_dir(),
    });
    w.open_input_window();
    let mut h = harness(w);
    h.run();
    // Jump's +: listening, as its button starts (Jump is the third
    // action); the click itself isn't bound, so a frame or two pass.
    h.state_mut().0.input_window.as_mut().unwrap().listening =
        Some((2, crate::input_window::Slot::Inputs, 0));
    h.run();
    h.run();
    h.run();
    h.key_press(egui::Key::K);
    h.run();
    let jump = h.state().0.bindings().get("Jump").unwrap().inputs.clone();
    assert!(jump.contains(&"K".to_string()), "{jump:?}");
    assert!(
        h.state()
            .1
            .actions
            .iter()
            .any(|a| matches!(a, WorkbenchAction::SaveBindings(text) if text.contains("\"K\""))),
        "saved"
    );
}

#[test]
fn input_actions_changed_while_playing_reach_the_game_and_paused_input_waits_for_nothing() {
    let mut w = with_a_walker();
    w.play();
    tick(&mut w);
    // Bind Move's up to I in the window's way: the game has it at once.
    let mut bindings = w.bindings.clone();
    bindings.get_mut("Move").unwrap().up = vec!["I".into()];
    w.change_bindings(bindings);
    w.devices.set("I", true);
    tick(&mut w);
    assert!(walker_at(&w).y > 0.05, "I walks now");
    w.devices.set("I", false);
    // While paused, a tap of F doesn't wait to be pressed when it goes on.
    w.pause();
    w.devices.set("F", true);
    w.devices.set("F", false);
    w.run_game(0.1);
    w.pause();
    tick(&mut w);
    assert!(
        !w.log_lines()
            .iter()
            .any(|l| l.text.contains("interact with"))
    );
}
