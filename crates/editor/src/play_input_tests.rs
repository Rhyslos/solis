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
const WALKER: &str = "fetch \"solis/project\":Action;\nfetch \"solis/input\":Input;\nfetch \"solis/world\":World;\n\n\
    \n\
    \n\
    component Walker {\n\
        method update(float dt) {\n\
            vec3 way = Input:axis(Action.Move);\n\
            vec3 step = [way.x, way.y, 0];\n\
            moveTo(position() + step * 0.1);\n\
            is Input:held(Action.Jump)? then {\n\
                moveTo(position() + step * 0.1 + [0, 0, 1]);\n\
            }\n\
            is Input:pressed(Action.Interact)? then {\n\
                World:say(\"interact with \" + Input:binding(Action.Interact));\n\
            }\n\
            is Input:pressed(Action.Sprint)? then {\n\
                Input:rebind(Action.Jump, \"J\");\n\
            }\n\
        }\n\
    }\n\
    pass Walker;\nend\n";

fn program(main: &str) -> Arc<script::Compiled> {
    let (mut files, _) = script::describe::descriptions_for(
        "Test Engine 9.9.9",
        "solis",
        &script::project_names::Names::with_assemblies(&["drones/drone", "nowhere"]),
    );
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
    let mut w = Workbench::new(info());
    w.set_program(Some(program(WALKER)));
    let mut data = EntityData::new("Walker").with(&Transform::default());
    data.components
        .insert("Walker".into(), reflect::ComponentData::default());
    w.open
        .document
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
    let walker = EntityId(reflect::Id::from_raw(WALKER_ID).unwrap());
    h.state_mut().0.open.document.selection.select_only(walker);
    h.key_press(egui::Key::Escape);
    h.run();
    assert!(!h.state().0.game_has_input(), "Esc gives them back");
    assert!(
        h.state().0.open.document.selection.contains(walker),
        "and isn't also the editor's Esc (Select nothing)"
    );
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
    w.open.fixed_seed = Some(5);
    w.play();
    let playing = w.open.playing.as_mut().unwrap();
    playing.recording = Some(input::Recording {
        area: None,
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
        .open
        .outbox
        .iter()
        .find_map(|a| match a {
            WorkbenchAction::SaveRecording(text) => Some(text.clone()),
            _ => None,
        })
        .expect("Stop saves the recording");
    w.open.outbox.clear();
    saved
}

#[test]
fn a_recording_plays_back_exactly_and_a_changed_game_is_caught() {
    let mut w = with_a_walker();
    unhurried(&mut w);
    let text = record(&mut w, 90);
    let recording = input::Recording::read(&text).unwrap();
    assert_eq!(recording.ticks.len(), 90);
    assert_eq!(recording.seed, 5);
    // Played back (in frames of two ticks, where it was recorded in
    // frames of under one: the game mustn't depend on that): the same,
    // tick by tick, then paused with the verdict.
    w.open.fixed_seed = None;
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
        w.open.fixed_seed, None,
        "the recorded seed was for that play only"
    );
    w.stop();
    w.open.outbox.clear();
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
            .any(|l| l.text.contains("stopped matching the recording by tick"))
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
    assert!(w.open.playing.is_none());
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
    assert!(w.open.playing.is_none());
}

#[test]
fn the_input_actions_window_binds_what_is_pressed_and_saves() {
    let mut w = Workbench::new(info());
    w.open.project = Some(crate::library::ProjectInfo {
        name: "Test".into(),
        root: std::env::temp_dir(),
        assets: std::env::temp_dir(),
    });
    w.open_input_page();
    let mut h = harness(w);
    h.run();
    // Jump's +: listening, as its button starts (Jump is the third
    // action); the click itself isn't bound, so a frame or two pass.
    h.state_mut().0.open.input_page.as_mut().unwrap().listening =
        Some((2, crate::file_pages::Slot::Inputs, 0));
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
    let mut bindings = w.open.bindings.clone();
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

#[test]
fn a_recording_keeps_play_s_seed_not_the_one_a_start_set() {
    // The second review: a start that set the seed made the recording
    // keep that one, so a replay drew different numbers from Play's.
    let code = "fetch \"solis/world\":World;\n\n\
        \n\
        component Drifter {\n\
            has {\n\
                float start = World:randomRange(0, 10);\n\
            }\n\
            method update(float dt) {\n\
                moveTo(position() + [World:randomRange(-1, 1), start * 0.01, 0]);\n\
            }\n\
        }\n\
        World:setSeed(42);\n\
        pass Drifter;\nend\n";
    let mut w = with_a_walker();
    unhurried(&mut w);
    w.set_program(Some(program(code)));
    let id = EntityId(reflect::Id::from_raw(WALKER_ID).unwrap());
    let mut data = w.open.document.area().get(id).unwrap().clone();
    data.components.clear();
    data.set(&Transform::default());
    data.components
        .insert("Drifter".into(), reflect::ComponentData::default());
    w.open
        .document
        .edit(
            "Drifter",
            vec![
                area::Edit::Despawn { root: id },
                area::Edit::Spawn {
                    entities: vec![(id, data)],
                },
            ],
            None,
            None,
        )
        .unwrap();
    w.open.project = Some(crate::library::ProjectInfo {
        name: "Seeds".into(),
        root: std::env::temp_dir(),
        assets: std::env::temp_dir(),
    });
    w.open.fixed_seed = Some(7);
    w.play_and_record();
    for _ in 0..40 {
        tick(&mut w);
    }
    w.stop();
    let text = w
        .open
        .outbox
        .iter()
        .find_map(|a| match a {
            WorkbenchAction::SaveRecording(text) => Some(text.clone()),
            _ => None,
        })
        .unwrap();
    assert_eq!(input::Recording::read(&text).unwrap().seed, 7);
    w.open.fixed_seed = None;
    w.replay(&text, "drift.solrec");
    for _ in 0..200 {
        if w.play_state() == crate::play::PlayState::Paused {
            break;
        }
        w.run_game((world::TICK * 2.0) as f32);
    }
    assert!(
        w.log_lines()
            .iter()
            .any(|l| l.text.contains("matched the recording on all 40 ticks")),
        "{:?}",
        w.log_lines().iter().map(|l| &l.text).collect::<Vec<_>>()
    );
}

#[test]
fn the_arrow_keys_reach_the_ready_made_actions() {
    // The user test: egui calls the arrow keys "Up", "Left" and so on,
    // and the ready-made bindings said "ArrowUp", so the RTS camera's
    // keys never did anything.
    let b = input::Bindings::default();
    let mut devices = input::Devices::new();
    devices.set(crate::play_input::key_name(egui::Key::ArrowUp), true);
    devices.set(crate::play_input::key_name(egui::Key::ArrowLeft), true);
    let frame = devices.sample(&b, world::TICK as f32);
    let pan = frame.get("Pan").axis;
    assert!(pan.x < -0.5 && pan.y > 0.5, "{pan}");
    devices.set("Alt", true);
    let frame = devices.sample(&b, world::TICK as f32);
    assert_eq!(frame.get("Turn").axis, glam::Vec2::new(-1.0, 0.0));
    // Every key a ready-made action names is one egui can give.
    let keys: Vec<&str> = egui::Key::ALL.iter().map(|k| k.name()).collect();
    for action in &b.actions {
        for input in action.all_inputs() {
            if input.starts_with("Mouse ") || input.starts_with("Pad ") {
                continue;
            }
            let (_, key) = input::split_combination(input).unwrap();
            assert!(
                keys.contains(&key) || input::MODIFIERS.contains(&key),
                "{} uses {input}",
                action.name
            );
        }
    }
}

#[test]
fn alt_enter_switches_the_player_s_fullscreen_and_the_game_doesn_t_see_it() {
    let mut w = with_a_walker();
    w.play();
    let mut h = harness(w);
    h.run();
    h.state_mut().0.give_game_input(&egui::Context::default());
    h.key_press_modifiers(egui::Modifiers::ALT, egui::Key::Enter);
    h.run();
    assert!(h.state().0.player_settings.fullscreen);
    assert!(
        h.state()
            .0
            .open
            .playing
            .as_ref()
            .unwrap()
            .view
            .settings
            .fullscreen,
        "game code sees it from the next tick"
    );
    assert!(
        h.state()
            .0
            .log_lines()
            .iter()
            .any(|l| l.text.starts_with("Alt+Enter")),
        "said in the Console"
    );
    h.key_press_modifiers(egui::Modifiers::ALT, egui::Key::Enter);
    h.run();
    assert!(!h.state().0.player_settings.fullscreen, "and back");
}

#[test]
fn alt_enter_held_switches_once_and_lets_enter_go() {
    let mut w = with_a_walker();
    w.play();
    let mut h = harness(w);
    h.run();
    h.state_mut().0.give_game_input(&egui::Context::default());
    let key = |pressed: bool, repeat: bool, modifiers: egui::Modifiers| egui::Event::Key {
        key: egui::Key::Enter,
        physical_key: None,
        pressed,
        repeat,
        modifiers,
    };
    // Held: the first press switches it; the repeats don't.
    h.event(key(true, false, egui::Modifiers::ALT));
    h.event(key(true, true, egui::Modifiers::ALT));
    h.event(key(true, true, egui::Modifiers::ALT));
    h.run();
    assert!(h.state().0.player_settings.fullscreen);
    // Enter pressed for the game, then Alt, then Enter let go: the game
    // sees it let go.
    h.event(key(true, false, egui::Modifiers::NONE));
    h.run();
    h.event(key(false, false, egui::Modifiers::ALT));
    h.run();
    assert!(!h.state().0.devices.is_held("Enter"), "not stuck down");
    assert!(
        h.state().0.player_settings.fullscreen,
        "a let-go switches nothing"
    );
}
