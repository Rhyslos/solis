use crate::properties::PropertyRequest;
use crate::test_support::*;
use crate::workbench::Workbench;
use area::EntityData;
use area::builtin::Transform;
use glam::Vec3;
use reflect::{EntityId, Value};
use std::sync::Arc;

/// The program for a `main.lst` holding `main`, with the engine's
/// description files.
fn program(main: &str) -> Arc<script::Chunk> {
    let mut files = script::describe::descriptions("Test Engine 9.9.9", "solis");
    files.push((
        "main.lst".into(),
        format!(
            "get \"solis/world\":[say, time, find];\n\
             get \"solis/entity\":[positionOf, moveTo, rotationOf, turnTo, nameOf];\n{main}"
        ),
    ));
    let code = script::Code {
        files,
        notes: Vec::new(),
    };
    match script::compile(&code) {
        Ok(Some(chunk)) => Arc::new(chunk),
        Ok(None) => panic!("no code"),
        Err(problems) => panic!("{}", problems[0]),
    }
}

const MOVER: &str = "component Mover {\n\
        has {\n\
            float speed = 6;\n\
            int steps = 0;\n\
            string label = \"mover\";\n\
        }\n\
        method update(float dt) {\n\
            vec3 ahead = [1, 0, 0];\n\
            moveTo(me, positionOf(me) + ahead * speed * dt);\n\
            steps = steps + 1;\n\
            is steps == 1? then {\n\
                say(nameOf(me) + \" sets off\");\n\
            }\n\
        }\n\
    }\n\
    pass Mover;\nend\n";

/// A workbench with the Mover program and an entity carrying it.
fn with_a_mover() -> (Workbench, EntityId) {
    let mut w = Workbench::new(info(), None);
    w.set_program(Some(program(MOVER)));
    let id = EntityId::random();
    w.document
        .edit(
            "Add",
            vec![area::Edit::Spawn {
                entities: vec![(id, EntityData::new("Cart").with(&Transform::default()))],
            }],
            None,
            None,
        )
        .unwrap();
    w.apply_property(
        PropertyRequest::AddComponent {
            entity: id,
            component: "Mover".into(),
        },
        1,
    );
    (w, id)
}

fn x_in_game(w: &Workbench, id: EntityId) -> f32 {
    w.world().unwrap().global(id).unwrap().w_axis.x
}

#[test]
fn the_program_s_components_join_the_registry_and_replace_the_last_ones() {
    let mut w = Workbench::new(info(), None);
    w.set_program(Some(program(MOVER)));
    let schema = w.document.registry.get("Mover").expect("registered");
    assert_eq!(schema.field("speed").unwrap().default, Value::Float(6.0));
    assert_eq!(w.game_components().collect::<Vec<_>>(), ["Mover"]);
    // New code: Mover is gone, Spinner arrives.
    w.set_program(Some(program(
        "component Spinner {\n    has {\n        float turn = 1;\n    }\n}\npass Spinner;\nend\n",
    )));
    assert!(w.document.registry.get("Mover").is_none());
    assert!(w.document.registry.get("Spinner").is_some());
    // Another project: none of this one's code is left.
    w.set_project(Some(crate::library::ProjectInfo {
        name: "Other".into(),
        root: "/other".into(),
        assets: "/other/assets".into(),
    }));
    assert!(w.document.registry.get("Spinner").is_none());
    w.set_program(Some(program(MOVER)));
    // No code: none left; the built-in ones stay.
    w.set_program(None);
    assert!(w.document.registry.get("Spinner").is_none());
    assert!(w.document.registry.get("Light").is_some());
}

#[test]
fn a_component_named_like_a_built_in_one_or_a_failing_start_is_a_problem() {
    let mut w = Workbench::new(info(), None);
    w.set_program(Some(program(
        "component Light {\n    has {\n        int on = 1;\n    }\n}\npass Light;\nend\n",
    )));
    assert_eq!(w.code_problems().len(), 1);
    assert!(w.code_problems()[0].message.contains("Light"));
    assert!(
        w.document
            .registry
            .get("Light")
            .unwrap()
            .field("on")
            .is_none()
    );
    let mut w = Workbench::new(info(), None);
    w.set_program(Some(program("int ways = 0;\nint each = 12 / ways;\nend\n")));
    assert_eq!(w.code_problems().len(), 1);
    assert_eq!(w.code_problems()[0].line, 4);
    assert!(
        w.log_lines()
            .last()
            .unwrap()
            .text
            .contains("stopped while starting")
    );
}

#[test]
fn play_runs_the_game_s_components_and_stop_puts_everything_back() {
    let (mut w, id) = with_a_mover();
    w.play();
    // Ten ticks of 1/60 s at 6 m/s: 1 m.
    for _ in 0..10 {
        w.run_game(1.0 / 60.0);
    }
    assert!(
        (x_in_game(&w, id) - 1.0).abs() < 1e-3,
        "{}",
        x_in_game(&w, id)
    );
    assert!(
        w.log_lines().iter().any(|l| l.text == "Cart sets off"),
        "said once, in the Console"
    );
    // The area itself didn't move: only the game's world.
    assert_eq!(
        w.document().area().world_matrix(id).w_axis.x,
        0.0,
        "the editor's area is untouched"
    );
    w.stop();
    assert!(w.world().is_none());
}

#[test]
fn a_change_in_properties_while_playing_reaches_the_running_component() {
    let (mut w, id) = with_a_mover();
    w.play();
    w.run_game(1.0 / 60.0);
    w.apply_property(
        PropertyRequest::SetField {
            entity: id,
            component: "Mover".into(),
            field: "speed".into(),
            value: Value::Float(0.0),
        },
        2,
    );
    let before = x_in_game(&w, id);
    for _ in 0..5 {
        w.run_game(1.0 / 60.0);
    }
    assert!((x_in_game(&w, id) - before).abs() < 1e-5, "stopped moving");
    let running = w.playing.as_ref().unwrap().game.as_ref().unwrap();
    assert_eq!(
        running.values(id, "Mover").unwrap().get("steps"),
        Some(&Value::Int(6)),
        "the steps counted so far are kept"
    );
    // Taking the component off stops it.
    w.apply_property(
        PropertyRequest::RemoveComponent {
            entity: id,
            component: "Mover".into(),
        },
        3,
    );
    w.run_game(1.0 / 60.0);
    let running = w.playing.as_ref().unwrap().game.as_ref().unwrap();
    assert!(running.is_empty());
}

#[test]
fn a_failing_component_is_reported_once_naming_its_entity() {
    let mut w = Workbench::new(info(), None);
    w.set_program(Some(program(
        "component Divider {\n    has {\n        int ways = 0;\n    }\n    method update(float dt) {\n        int each = 12 / ways;\n    }\n}\npass Divider;\nend\n",
    )));
    let id = EntityId::random();
    let mut data = EntityData::new("Splitter");
    data.set(&Transform {
        position: Vec3::ZERO,
        ..Transform::default()
    });
    data.components
        .insert("Divider".into(), reflect::ComponentData::default());
    w.document
        .edit(
            "Add",
            vec![area::Edit::Spawn {
                entities: vec![(id, data)],
            }],
            None,
            None,
        )
        .unwrap();
    w.play();
    for _ in 0..5 {
        w.run_game(1.0 / 60.0);
    }
    let reports: Vec<_> = w
        .log_lines()
        .iter()
        .filter(|l| {
            l.text
                .starts_with("Divider on Splitter stopped: main.lst, line 8")
        })
        .collect();
    assert_eq!(reports.len(), 1, "{:?}", w.log_lines().last());
}

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
fn scale_check_playing_with_2000_lust_components() {
    // Each of 2,000 entities carries a Lust component that moves it every
    // tick: a frame of play mode runs one tick of all of them.
    const TARGET_MS: f64 = 16.0;
    let mut w = Workbench::new(info(), None);
    w.set_program(Some(program(MOVER)));
    let spawns: Vec<_> = (0..2_000)
        .map(|i| {
            let mut data = EntityData::new(&format!("Cart {i}")).with(&Transform {
                position: Vec3::new(0.0, i as f32, 0.0),
                ..Transform::default()
            });
            data.components
                .insert("Mover".into(), reflect::ComponentData::default());
            area::Edit::Spawn {
                entities: vec![(EntityId::random(), data)],
            }
        })
        .collect();
    w.document.edit("Fill", spawns, None, None).unwrap();
    w.play();
    w.run_game(1.0 / 60.0);
    let frames = 60;
    let start = std::time::Instant::now();
    for _ in 0..frames {
        w.run_game(1.0 / 60.0);
    }
    let ms = start.elapsed().as_secs_f64() * 1000.0 / f64::from(frames);
    let verdict = if ms <= TARGET_MS { "ok" } else { "SLOW" };
    println!(
        "Scale check: a play-mode tick of 2,000 Lust components that move their entities {ms:.2} ms (target {TARGET_MS} ms, one frame) {verdict}"
    );
    assert!(ms <= TARGET_MS, "a tick takes {ms:.2} ms");
}
