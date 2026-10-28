//! Save games: a loaded game carries on exactly as the saved one would
//! have (checksums and all), after quitting too; damaged saves are
//! refused; game code's `save` and `load`.

use super::*;
use crate::test_support::*;
use crate::workbench::WorkbenchAction;
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
        text.replacen("solis save 4", "solis save 9", 1),
        text.replacen("solis save 4", "solis save 2", 1),
        text.replacen("solis save 4", "solis save 1", 1),
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
    // The third review: this version's header with Windows line endings
    // or a space after it is damaged, not "from a newer Solis".
    for header in ["solis save 4\r", "solis save 4 "] {
        let why = Save::read(&text.replacen("solis save 4", header, 1)).unwrap_err();
        assert!(why.contains("damaged"), "{header:?}: {why}");
    }
    assert!(
        Save::read(&text.replacen("solis save 4", "solis save 9", 1))
            .unwrap_err()
            .contains("newer")
    );
}

#[test]
fn a_project_name_with_a_line_break_still_loads_its_saves() {
    // The third review: a name with a line break was written on one line
    // but compared whole, so its saves never loaded.
    let mut w = busy("Harbour\nNorth");
    w.play();
    run_to(&mut w, 10);
    let text = w.save_of_game().unwrap().write();
    w.load_game(&text, "quick");
    assert!(
        w.log_lines()
            .last()
            .unwrap()
            .text
            .starts_with("Loaded quick"),
        "{}",
        w.log_lines().last().unwrap().text
    );
}

#[test]
fn a_save_can_t_give_an_entity_a_component_it_doesn_t_have() {
    // The third review: the values section was given to the game whole,
    // so a hand-made save could start components the area doesn't have.
    let mut w = busy("Harbour");
    w.play();
    run_to(&mut w, 10);
    let mut save = w.save_of_game().unwrap();
    let floor = w
        .document
        .area()
        .iter()
        .find(|(_, e)| e.name == "Floor")
        .map(|(id, _)| id)
        .unwrap();
    let mut extra = area::Area::new();
    let mut holder = EntityData::new("");
    holder
        .components
        .insert("Busy".into(), reflect::ComponentData::default());
    extra.insert(floor, holder);
    let registry = w.document.registry.clone();
    let values = area::text::read(&save.values, &registry).unwrap();
    for (id, data) in values.iter() {
        extra.insert(id, data.clone());
    }
    save.values = area::text::write(&extra, &registry, &|_| None);
    let text = save.write();
    w.load_game(&text, "hand-made");
    assert!(
        w.log_lines().last().unwrap().text.starts_with("Loaded"),
        "{}",
        w.log_lines().last().unwrap().text
    );
    let running = w.playing.as_mut().unwrap().game.as_mut().unwrap();
    assert!(
        running
            .all_values()
            .iter()
            .all(|(entity, _, _)| *entity != floor),
        "the floor runs nothing"
    );
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
    assert!(w.save_text().is_none());
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
    let physics = physics.split("\nplaces ").next().unwrap();
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

/// The busy game with a quick-save key (F6), a quick-load key (F7), and
/// a component reading them.
fn busy_with_keys() -> Workbench {
    let code = BUSY
        .replacen(
            "get \"solis/physics\":[push];",
            "get \"solis/physics\":[push];\nget \"solis/input\":[pressed];",
            1,
        )
        .replacen(
            "pass Busy;",
            "component Keys {\n\
                method update(float dt) {\n\
                    is pressed(\"QuickSave\")? then {\n\
                        saveGame(\"quick\");\n\
                    }\n\
                    is pressed(\"QuickLoad\")? then {\n\
                        loadGame(\"quick\");\n\
                    }\n\
                }\n\
            }\n\
            pass Busy, Keys;",
            1,
        );
    assert!(code.contains("pass Busy, Keys;"));
    let mut w = busy("Harbour");
    w.set_program(Some(program(&code)));
    let mut bindings = input::Bindings::default();
    bindings
        .actions
        .push(input::Action::button("QuickSave", &["F6"]));
    bindings
        .actions
        .push(input::Action::button("QuickLoad", &["F7"]));
    w.set_bindings(bindings);
    let mut keys = EntityData::new("Keys");
    keys.components
        .insert("Keys".into(), reflect::ComponentData::default());
    w.document
        .edit(
            "Keys",
            vec![area::Edit::Spawn {
                entities: vec![(raw(4), keys)],
            }],
            None,
            None,
        )
        .unwrap();
    w
}

#[test]
fn a_recording_with_a_quick_save_in_it_replays_matching() {
    // The third review: a replay skipped the save, and so the physics
    // built again with it, and stopped matching right after.
    let mut w = busy_with_keys();
    w.project = Some(crate::library::ProjectInfo {
        name: "Harbour".into(),
        root: std::env::temp_dir(),
        assets: std::env::temp_dir(),
    });
    w.play_and_record();
    w.give_game_input(&egui::Context::default());
    run_to(&mut w, 30);
    w.devices.set("F6", true);
    run_to(&mut w, 32);
    w.devices.set("F6", false);
    run_to(&mut w, 90);
    w.stop();
    let text = w
        .outbox
        .iter()
        .find_map(|a| match a {
            WorkbenchAction::SaveRecording(text) => Some(text.clone()),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        saves_asked(&w),
        1,
        "saved while recording: {:?} {:?}",
        w.log_lines().iter().map(|l| &l.text).collect::<Vec<_>>(),
        w.outbox.len()
    );
    w.outbox.clear();
    w.replay(&text, "quick.solrec");
    for _ in 0..200 {
        if w.play_state() == crate::play::PlayState::Paused {
            break;
        }
        w.run_game((world::TICK * 2.0) as f32);
    }
    assert!(
        w.log_lines()
            .iter()
            .any(|l| l.text.contains("matched the recording")),
        "{:?}",
        w.log_lines().iter().map(|l| &l.text).collect::<Vec<_>>()
    );
    assert_eq!(saves_asked(&w), 0, "a replay doesn't write saves");
}

#[test]
fn a_quick_load_keeps_the_game_s_keys_and_keeps_what_was_recorded() {
    let mut w = busy_with_keys();
    w.project = Some(crate::library::ProjectInfo {
        name: "Harbour".into(),
        root: std::env::temp_dir(),
        assets: std::env::temp_dir(),
    });
    w.play_and_record();
    w.give_game_input(&egui::Context::default());
    run_to(&mut w, 20);
    let text = w.save_text().unwrap();
    run_to(&mut w, 40);
    // Game code's loadGame (the app hands it the save's text).
    w.load_game_from_code(&text, "quick");
    assert!(w.game_has_input(), "still the game's");
    let kept = w
        .outbox
        .iter()
        .find_map(|a| match a {
            WorkbenchAction::SaveRecording(text) => Some(text.clone()),
            _ => None,
        })
        .expect("what was recorded is saved");
    assert_eq!(input::Recording::read(&kept).unwrap().ticks.len(), 40);
    // A load from the Play menu gives the editor its keys back.
    w.load_game(&text, "quick.solsave");
    assert!(!w.game_has_input());
}

/// Writes real files for the fuzzers to start from (decision 0042): a save
/// game, a recording, input actions and an area, into the folder
/// `SOLIS_FUZZ_SEEDS` names (`fuzz/seeds`). Run by `fuzz/make-seeds.sh`.
#[test]
#[ignore = "writes the fuzzers' seed files; run by fuzz/make-seeds.sh"]
fn write_fuzz_seeds() {
    let Ok(dir) = std::env::var("SOLIS_FUZZ_SEEDS") else {
        return;
    };
    let dir = std::path::PathBuf::from(dir);
    let put = |target: &str, name: &str, text: &str| {
        let folder = dir.join(target);
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join(name), text).unwrap();
    };
    let mut w = busy_with_keys();
    w.play_and_record();
    w.give_game_input(&egui::Context::default());
    run_to(&mut w, 20);
    w.devices.set("F6", true);
    run_to(&mut w, 22);
    w.devices.set("F6", false);
    run_to(&mut w, 40);
    put("save_game", "busy.solsave", &w.save_text().unwrap());
    w.stop();
    let recording = w
        .outbox
        .iter()
        .find_map(|a| match a {
            WorkbenchAction::SaveRecording(text) => Some(text.clone()),
            _ => None,
        })
        .unwrap();
    put("recording", "busy.solrec", &recording);
    put("input_actions", "busy.solinput", &w.bindings().write());
    put(
        "input_actions",
        "default.solinput",
        &input::Bindings::default().write(),
    );
    put(
        "area_file",
        "busy.solarea",
        &area::text::write(w.document.area(), &w.document.registry, &|_| None),
    );
    put("game_code", "busy.lst", BUSY);
}

#[test]
fn a_sound_turned_off_stays_off_after_a_load() {
    use area::builtin::AudioSource;
    let mut w = Workbench::new(info(), None);
    w.project = Some(crate::library::ProjectInfo {
        name: "Radio".into(),
        root: std::path::PathBuf::from("/nowhere"),
        assets: std::path::PathBuf::from("/nowhere/assets"),
    });
    let radio = w.add_entity();
    let source = AudioSource {
        event: "radio".into(),
        ..AudioSource::default()
    };
    w.document
        .edit(
            "Add a sound",
            crate::commands::component_edits(radio, None, &source),
            None,
            None,
        )
        .unwrap();
    w.play();
    assert_eq!(w.audio_sources().len(), 1, "on at Play");
    let before = w.world().unwrap().checksum();
    w.playing
        .as_mut()
        .unwrap()
        .world
        .set_sound_playing(radio, false);
    assert!(w.audio_sources().is_empty(), "turned off");
    assert_ne!(w.world().unwrap().checksum(), before, "in the checksum");
    let text = w.save_of_game().unwrap().write();
    w.stop();
    w.play();
    assert_eq!(w.audio_sources().len(), 1, "a new play starts it again");
    w.load_game(&text, "quick");
    assert!(w.audio_sources().is_empty(), "the save keeps it off");
}

/// The example saves of every version (Phase 8, C2).
fn save_fixtures() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/save")
}

#[test]
fn every_version_of_a_save_loads_and_plays_on() {
    assert!(super::FORMAT.is_complete());
    for version in 1..=super::SAVE_VERSION {
        let dir = save_fixtures().join(format!("v{version}"));
        let mut files: Vec<_> = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
            .flatten()
            .map(|e| e.path())
            .collect();
        files.sort();
        assert!(!files.is_empty(), "no example of save version {version}");
        for file in files {
            let text = std::fs::read_to_string(&file).unwrap();
            assert_eq!((super::FORMAT.version_of)(&text), Some(version));
            let save = Save::read(&text).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
            // Only version 1's physics can't be carried over.
            assert_eq!(
                save.notes.len(),
                usize::from(version == 1),
                "{}",
                file.display()
            );
            let mut w = busy("Harbour");
            w.play();
            w.load_game(&text, "the save");
            assert!(
                !w.log_lines().iter().any(|e| e.level == LogLevel::Error),
                "{}: {:?}",
                file.display(),
                w.log_lines().last().map(|l| &l.text)
            );
            assert_eq!(w.world().unwrap().clock().ticks, 40, "{}", file.display());
            // It plays on: the counter's values came back and go on.
            run_to(&mut w, 60);
            let values = w
                .playing
                .as_mut()
                .unwrap()
                .game
                .as_mut()
                .unwrap()
                .all_values();
            let ticks = values
                .iter()
                .find(|(_, name, _)| name == "Busy")
                .and_then(|(_, _, data)| data.get("ticks").cloned());
            assert_eq!(ticks, Some(reflect::Value::Int(60)), "{}", file.display());
        }
    }
    assert!(
        !save_fixtures()
            .join(format!("v{}", super::SAVE_VERSION + 1))
            .exists()
    );
}

#[test]
fn a_save_whose_component_the_game_no_longer_has_still_loads() {
    // The game's code changed since the save: its component is gone. The
    // save loads, and the Console says what was left out.
    let text = std::fs::read_to_string(save_fixtures().join("v3/harbour.solsave")).unwrap();
    let mut w = busy("Harbour");
    w.set_program(Some(program(
        "component Other {\n method update(float dt) { }\n}\npass Other;\nend\n",
    )));
    w.play();
    w.load_game(&text, "the save");
    assert_eq!(w.world().unwrap().clock().ticks, 40);
    let said: Vec<&str> = w.log_lines().iter().map(|l| l.text.as_str()).collect();
    assert!(said.iter().any(|t| t.contains("Busy")), "{said:?}");
    run_to(&mut w, 50);
}

#[test]
fn sections_a_save_never_has_are_refused() {
    let text = std::fs::read_to_string(save_fixtures().join("v3/harbour.solsave")).unwrap();
    for bad in [
        format!("{text}extra 0\n\n"),
        format!("{text}places 0\n\n"),
        text.replacen("solis save 3", "solis save 0", 1),
    ] {
        assert!(Save::read(&resummed(&bad)).is_err());
    }
    // Version 1 without its physics: refused, not half-upgraded.
    let v1 = std::fs::read_to_string(save_fixtures().join("v1/harbour.solsave")).unwrap();
    let cut = v1.find("physics ").unwrap();
    let mut c = reflect::Checksum::new();
    let lines: Vec<&str> = v1[..cut].splitn(7, '\n').collect();
    c.bytes(lines[6].as_bytes());
    let without = v1[..cut].replacen(lines[5], &format!("sum {:016x}", c.finish()), 1);
    let refused = Save::read(&without).unwrap_err();
    assert!(refused.contains("physics"), "{refused}");
}

/// `busy`, with an animator (Idle and Walk, moving to go between them;
/// a speed number) on a farmer (Phase 9, D3b).
fn busy_with_an_animator() -> (Workbench, EntityId) {
    use animation::{AnimatorFile, Condition, Parameter, ParameterKind, Plays, State, Transition};
    let mut w = busy("Harbour");
    let anim = reflect::AssetId(reflect::Id::from_raw(4242).unwrap());
    let rid = |n| reflect::Id::from_raw(n).unwrap();
    let state = |n, name: &str| State {
        id: rid(n),
        name: name.into(),
        place: [0.0, 0.0],
        plays: Plays::Clip(String::new()),
        speed: 1.0,
        speed_by: String::new(),
    };
    let file = AnimatorFile {
        parameters: vec![
            Parameter {
                id: rid(10),
                name: "moving".into(),
                kind: ParameterKind::Switch(false),
            },
            Parameter {
                id: rid(11),
                name: "speed".into(),
                kind: ParameterKind::Number(0.0),
            },
        ],
        states: vec![state(1, "Idle"), state(2, "Walk")],
        transitions: vec![
            Transition {
                id: rid(20),
                from: Some(rid(1)),
                to: rid(2),
                when: vec![Condition::Is("moving".into())],
            },
            Transition {
                id: rid(21),
                from: Some(rid(2)),
                to: rid(1),
                when: vec![Condition::Below("speed".into(), 0.5)],
            },
        ],
        ..AnimatorFile::default()
    };
    let mut sheets = crate::sprite_sheets::SpriteSheets::default();
    sheets.animators.insert(
        "farmer.solanim".into(),
        crate::sprite_sheets::AnimatorState::Read(Arc::new(file), Vec::new()),
    );
    sheets.paths.insert(anim, "farmer.solanim".into());
    w.set_sprite_sheets(sheets);
    let farmer = w.spawn_selected(EntityData::new("Farmer").with(&Transform::default()).with(
        &area::builtin::Animator {
            animator: Some(reflect::AssetRef {
                asset: anim,
                part: None,
            }),
            ..area::builtin::Animator::default()
        },
    ));
    (w, farmer)
}

#[test]
fn an_animator_s_state_and_values_survive_a_save() {
    let (mut w, farmer) = busy_with_an_animator();
    w.play();
    run_to(&mut w, 5);
    let world = &mut w.playing.as_mut().unwrap().world;
    world
        .set_animator_value(farmer, "moving", animation::ParamValue::Switch(true))
        .unwrap();
    world
        .set_animator_value(farmer, "speed", animation::ParamValue::Number(2.5))
        .unwrap();
    run_to(&mut w, 8);
    assert_eq!(w.world().unwrap().animator_of(farmer).unwrap().0, "Walk");
    let text = w.save_text().expect("saved");
    assert!(text.starts_with("solis save 4\n"));
    assert!(text.contains("animators "), "{text}");
    assert!(text.contains("moving=s1 speed=n40200000"), "{text}");
    let saved = sums(&mut w);
    let (mut loaded, _) = busy_with_an_animator();
    loaded.play();
    loaded.load_game(&text, "the save");
    assert!(
        !loaded
            .log_lines()
            .iter()
            .any(|e| e.level == LogLevel::Error),
        "{:?}",
        loaded.log_lines().last().map(|l| &l.text)
    );
    assert_eq!(sums(&mut loaded), saved, "as saved, before a tick");
    for t in 9..30 {
        run_to(&mut w, t);
        run_to(&mut loaded, t);
        assert_eq!(sums(&mut loaded), sums(&mut w), "tick {t}");
    }
    assert_eq!(
        loaded.world().unwrap().animator_of(farmer).unwrap().1[1],
        ("speed".to_owned(), animation::ParamValue::Number(2.5))
    );
}

#[test]
fn a_damaged_animators_section_is_refused() {
    let (mut w, _) = busy_with_an_animator();
    w.play();
    run_to(&mut w, 3);
    let text = w.save_text().unwrap();
    let at = text.find("animators ").unwrap();
    let body = &text[at..];
    let line = body.lines().nth(1).unwrap().to_owned();
    for bad in [
        line.replacen("moving=s0", "moving=s7", 1),
        line.replacen("moving=s0", "two words=s0", 1),
        line.replacen("moving=s0", "moving", 1),
        format!("NOTANID{}", &line[13..]),
    ] {
        let damaged = resummed(&text.replacen(&line, &bad, 1).replacen(
            &format!("animators {}", line.len() + 1),
            &format!("animators {}", bad.len() + 1),
            1,
        ));
        assert!(Save::read(&damaged).is_err(), "{bad}");
    }
}
