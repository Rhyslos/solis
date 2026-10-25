//! Save games: a loaded game carries on exactly as the saved one would
//! have (checksums and all), after quitting too; damaged saves are
//! refused; game code's `save` and `load`.

use super::*;
use crate::test_support::*;
use area::EntityData;
use area::builtin::{Collider, RigidBody};
use glam::Vec3;

/// A busy game: a counter that staggers randomly, kicks a crate on its
/// first tick, fires a laser now and then, and saves or loads when its
/// `request` field says to.
const BUSY: &str = "get \"solis/world\":[randomRange, find, fire, saveGame, loadGame];\n\
    get \"solis/entity\":[positionOf, moveTo];\n\
    get \"solis/physics\":[push];\n\
    component Busy {\n\
        has {\n\
            int ticks = 0;\n\
            float wander = 0;\n\
            int request = 0;\n\
        }\n\
        method update(float dt) {\n\
            is ticks == 0? then {\n\
                push(find(\"Crate\"), [3, 1, 0]);\n\
            }\n\
            ticks = ticks + 1;\n\
            wander = wander + randomRange(-1, 1);\n\
            moveTo(me, positionOf(me) + [randomRange(-0.1, 0.1), 0.05, 0]);\n\
            is ticks % 20 == 0? then {\n\
                fire(positionOf(me), [1, 0, 0], 30, 1, [2, 0.5, 0.2]);\n\
            }\n\
            is request == 1? then {\n\
                saveGame(\"quick\");\n\
                request = 0;\n\
            }\n\
            is request == 2? then {\n\
                loadGame(\"quick\");\n\
                request = 0;\n\
            }\n\
        }\n\
    }\n\
    pass Busy;\nend\n";

fn program(main: &str) -> Arc<script::Compiled> {
    let mut files = script::describe::descriptions("Test Engine 9.9.9", "solis");
    files.push(("main.lst".into(), main.to_string()));
    let code = script::Code {
        files,
        lua: Vec::new(),
        notes: Vec::new(),
    };
    Arc::new(script::compile(&code).unwrap().unwrap())
}

fn raw(n: u64) -> EntityId {
    EntityId(reflect::Id::from_raw(n).unwrap())
}

/// A workbench in a project called `project`, with the busy game: a
/// floor, a crate on it, and the busy entity.
fn busy(project: &str) -> Workbench {
    let mut w = Workbench::new(info(), None);
    w.project = Some(crate::library::ProjectInfo {
        name: project.into(),
        root: std::env::temp_dir(),
        assets: std::env::temp_dir(),
    });
    w.set_program(Some(program(BUSY)));
    let floor = EntityData::new("Floor")
        .with(&Transform {
            position: Vec3::new(0.0, 0.0, -0.5),
            ..Transform::default()
        })
        .with(&Collider {
            size: Vec3::new(40.0, 40.0, 1.0),
            ..Collider::default()
        });
    let crate_data = EntityData::new("Crate")
        .with(&Transform {
            position: Vec3::new(0.0, 0.0, 0.5),
            ..Transform::default()
        })
        .with(&RigidBody {
            mass: 1.0,
            ..RigidBody::default()
        })
        .with(&Collider::default());
    let mut busy = EntityData::new("Busy").with(&Transform::default());
    busy.components
        .insert("Busy".into(), reflect::ComponentData::default());
    w.document
        .edit(
            "Add",
            vec![
                area::Edit::Spawn {
                    entities: vec![(raw(1), floor)],
                },
                area::Edit::Spawn {
                    entities: vec![(raw(2), crate_data)],
                },
                area::Edit::Spawn {
                    entities: vec![(raw(3), busy)],
                },
            ],
            None,
            None,
        )
        .unwrap();
    w.fixed_seed = Some(11);
    w
}

/// Runs `w` until `ticks` have run in all.
fn run_to(w: &mut Workbench, ticks: u64) {
    while w.world().unwrap().clock().ticks < ticks {
        let left = ticks - w.world().unwrap().clock().ticks;
        w.run_game((world::TICK * (left.min(3) as f64) + 1e-9) as f32);
    }
}

/// The game's checksums now: the world's and game code's.
fn sums(w: &mut Workbench) -> (u64, u64) {
    let playing = w.playing.as_mut().unwrap();
    (
        playing.world.checksum(),
        playing.game.as_mut().unwrap().checksum(),
    )
}

#[test]
fn a_loaded_game_carries_on_exactly_as_the_saved_one_would_have() {
    let mut w = busy("Harbour");
    w.play();
    run_to(&mut w, 50);
    let text = w.save_of_game().unwrap().write();
    let saved_at = sums(&mut w);
    run_to(&mut w, 130);
    let carried_on = sums(&mut w);
    assert!(
        !w.world().unwrap().projectiles().is_empty(),
        "lasers in flight"
    );
    // Load, in the same play: back to the save, then the same future.
    w.load_game(&text, "quick");
    assert_eq!(w.world().unwrap().clock().ticks, 50);
    assert_eq!(sums(&mut w), saved_at, "the same as when saved");
    run_to(&mut w, 130);
    assert_eq!(sums(&mut w), carried_on, "the same, to the bit");
    // Stop goes back to the area as it was before Play.
    w.stop();
    assert_eq!(
        w.document
            .area()
            .get(raw(3))
            .unwrap()
            .get::<Transform>()
            .unwrap()
            .position,
        Vec3::ZERO
    );
}

#[test]
fn save_quit_open_and_load_carries_on_the_same() {
    let mut w = busy("Harbour");
    w.play();
    run_to(&mut w, 70);
    let text = w.save_of_game().unwrap().write();
    run_to(&mut w, 150);
    let carried_on = sums(&mut w);
    drop(w);
    // Solis again: a new workbench with the project's code, nothing
    // playing. Loading plays.
    let mut again = busy("Harbour");
    again.fixed_seed = None;
    again.load_game(&text, "quick.solsave");
    assert_eq!(again.play_state(), crate::play::PlayState::Playing);
    run_to(&mut again, 150);
    assert_eq!(sums(&mut again), carried_on);
}

#[test]
fn game_code_saves_and_loads_at_the_end_of_the_tick() {
    let mut w = busy("Harbour");
    w.play();
    run_to(&mut w, 10);
    let set_ask = |w: &mut Workbench, ask: i64| {
        w.apply_property(
            crate::properties::PropertyRequest::SetField {
                entity: raw(3),
                component: "Busy".into(),
                field: "request".into(),
                value: reflect::Value::Int(ask),
            },
            900 + ask as u64,
        );
    };
    set_ask(&mut w, 1);
    run_to(&mut w, 12);
    let saved = w
        .outbox
        .iter()
        .find_map(|a| match a {
            WorkbenchAction::SaveGame { slot, text } => Some((slot.clone(), text.clone())),
            _ => None,
        })
        .expect("save asked the app to write it");
    assert_eq!(saved.0, "quick");
    w.outbox.clear();
    set_ask(&mut w, 2);
    let before = w.world().unwrap().clock().ticks;
    w.run_game((world::TICK * 4.0) as f32);
    assert!(
        w.outbox
            .iter()
            .any(|a| matches!(a, WorkbenchAction::LoadGame(slot) if slot == "quick"))
    );
    // No ticks run past the one that asked to load.
    assert!(w.world().unwrap().clock().ticks <= before + 2);
    w.load_game(&saved.1, &saved.0);
    assert!(
        w.log_lines()
            .last()
            .unwrap()
            .text
            .starts_with("Loaded quick")
    );
}

#[test]
fn saves_from_elsewhere_or_damaged_are_refused_and_nothing_changes() {
    let mut w = busy("Harbour");
    w.play();
    run_to(&mut w, 30);
    let text = w.save_of_game().unwrap().write();
    run_to(&mut w, 40);
    let now = sums(&mut w);
    // Another project's.
    let mut other = busy("Lighthouse");
    other.load_game(&text, "theirs");
    assert!(
        other
            .log_lines()
            .last()
            .unwrap()
            .text
            .contains("saved by the project Harbour")
    );
    assert!(other.playing.is_none());
    // Damaged: cut short, a byte changed, a newer version, not a save.
    let mut damaged = vec![
        text[..text.len() / 2].to_string(),
        text.replacen("solis save 2", "solis save 9", 1),
        text.replacen("solis save 2", "solis save 1", 1),
        "hello".to_string(),
        String::new(),
    ];
    let mut seed = 0x9e37_79b9_u64;
    for _ in 0..200 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let mut bytes = text.clone().into_bytes();
        let at = (seed as usize) % bytes.len();
        bytes[at] = bytes[at].wrapping_add(1 + (seed >> 50) as u8 % 50);
        if let Ok(t) = String::from_utf8(bytes) {
            damaged.push(t);
        }
    }
    for bad in damaged {
        if bad == text {
            continue;
        }
        w.load_game(&bad, "bad");
        assert!(
            w.log_lines()
                .last()
                .unwrap()
                .text
                .starts_with("Couldn't load bad"),
            "{}",
            w.log_lines().last().unwrap().text
        );
    }
    assert_eq!(sums(&mut w), now, "nothing changed");
    assert!(Save::read(&"x".repeat(MAX_SAVE_BYTES + 1)).is_err());
}

#[test]
fn saving_needs_a_project_and_a_running_game() {
    let mut w = busy("Harbour");
    assert_eq!(w.save_of_game().unwrap_err(), "nothing is running");
    w.project = None;
    w.play();
    assert_eq!(
        w.save_of_game().unwrap_err(),
        "save games are kept in a project"
    );
    w.save_game("quick");
    assert!(
        w.log_lines()
            .last()
            .unwrap()
            .text
            .contains("kept in a project")
    );
}

/// Asks the busy entity to save (1) or load (2) on its next tick.
fn ask(w: &mut Workbench, n: u64, request: i64, change: u64) {
    w.apply_property(
        crate::properties::PropertyRequest::SetField {
            entity: raw(n),
            component: "Busy".into(),
            field: "request".into(),
            value: reflect::Value::Int(request),
        },
        change,
    );
}

fn saves_asked(w: &Workbench) -> usize {
    w.outbox
        .iter()
        .filter(|a| matches!(a, WorkbenchAction::SaveGame { .. }))
        .count()
}

#[test]
fn many_saves_of_one_slot_in_a_tick_are_made_once() {
    let mut w = busy("Harbour");
    // Forty more busy entities, every one asking to save in the same
    // tick: one save, not forty-one.
    let mut spawns = Vec::new();
    for n in 0..40 {
        let mut more = EntityData::new("More").with(&Transform {
            position: Vec3::new(n as f32, 5.0, 0.0),
            ..Transform::default()
        });
        let mut data = reflect::ComponentData::default();
        data.set("request", reflect::Value::Int(1));
        more.components.insert("Busy".into(), data);
        spawns.push(area::Edit::Spawn {
            entities: vec![(raw(100 + n), more)],
        });
    }
    w.document.edit("More", spawns, None, None).unwrap();
    w.play();
    run_to(&mut w, 3);
    assert_eq!(saves_asked(&w), 1);
    assert_eq!(
        crate::play::collapse_saves(vec![
            ("a".into(), false),
            ("b".into(), true),
            ("a".into(), false),
            ("c".into(), false),
            ("d".into(), false),
            ("e".into(), false),
        ]),
        (
            vec![
                ("b".into(), true),
                ("a".into(), false),
                ("c".into(), false),
                ("d".into(), false)
            ],
            1
        )
    );
}

#[test]
fn a_save_asked_for_on_a_tick_that_pauses_is_still_made() {
    let mut w = busy("Harbour");
    w.play();
    run_to(&mut w, 5);
    // A recording that's full: the next tick pauses the game.
    let playing = w.playing.as_mut().unwrap();
    playing.recording = Some(input::Recording::default());
    playing.recording_bytes = input::MAX_RECORDING_BYTES;
    ask(&mut w, 3, 1, 950);
    w.outbox.clear();
    run_to(&mut w, 6);
    assert_eq!(w.play_state(), crate::play::PlayState::Paused);
    assert_eq!(saves_asked(&w), 1, "saved, then paused");
}

#[test]
fn a_save_keeps_bodies_as_places_and_motion_not_rapier_s_insides() {
    let mut w = busy("Harbour");
    w.play();
    run_to(&mut w, 20);
    let text = w.save_of_game().unwrap().write();
    let physics = text.split("\nphysics ").nth(1).unwrap();
    // The floor and the crate: an ID, 13 numbers, asleep, character.
    let lines: Vec<&str> = physics.lines().skip(1).filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 2, "{physics}");
    assert!(lines.iter().all(|l| l.split(' ').count() == 16));
    assert!(text.len() < 8 << 10, "small: {} bytes", text.len());
    // A body that isn't a sensible number is a damaged save.
    let id = lines[1].split(' ').next().unwrap();
    let bad = text.replacen(
        lines[1],
        &format!("{id} 7fc00000{}", &lines[1][id.len() + 9..]),
        1,
    );
    assert!(Save::read(&resummed(&bad)).is_err(), "NaN refused");
    // A header changed by hand is damage too, and a clock out of range is
    // refused even with its sum made again.
    let seed_line = text.lines().nth(4).unwrap().to_string();
    assert!(Save::read(&text.replacen(&seed_line, "seed 12345", 1)).is_err());
    let tick_line = text.lines().nth(2).unwrap().to_string();
    let far = text.replacen(&tick_line, "tick 18446744073709551615", 1);
    assert!(Save::read(&resummed(&far)).is_err());
    assert!(Save::read(&resummed(&text)).is_ok());
}

/// `text` with its checksum made again (as a hand-made save would be).
fn resummed(text: &str) -> String {
    let lines: Vec<&str> = text.splitn(7, '\n').collect();
    let head = format!("{}\n{}\n{}\n{}\n", lines[1], lines[2], lines[3], lines[4]);
    text.replacen(
        lines[5],
        &format!("sum {:016x}", super::sum_of(&head, lines[6])),
        1,
    )
}

#[test]
fn loading_doesn_t_run_the_game_s_starts_again_on_top_of_the_save() {
    // The code's own start moves the crate up and sets the seed: right
    // at Play, wrong on top of a loaded game.
    let code = BUSY
        .replacen(
            "component Busy {",
            "moveTo(find(\"Crate\"), [0, 0, 4]);\nsetSeed(99);\ncomponent Busy {",
            1,
        )
        .replacen(
            "get \"solis/world\":[randomRange, find, fire, saveGame, loadGame];",
            "get \"solis/world\":[randomRange, find, fire, saveGame, loadGame, setSeed];",
            1,
        );
    let mut w = busy("Harbour");
    w.set_program(Some(program(&code)));
    w.play();
    run_to(&mut w, 60);
    let text = w.save_of_game().unwrap().write();
    run_to(&mut w, 120);
    let carried_on = sums(&mut w);
    w.load_game(&text, "quick");
    run_to(&mut w, 120);
    assert_eq!(sums(&mut w), carried_on, "the same, not moved or reseeded");
}
