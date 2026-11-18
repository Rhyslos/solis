//! The running game's state part by part: the world's parts with each
//! running component's values and each game-wide object's, the same for
//! the same game, and nothing when no game runs.

use crate::test_support::*;
use crate::workbench::Workbench;
use level::EntityData;
use level::builtin::Transform;
use reflect::{ComponentData, EntityId};
use std::sync::Arc;

const MAIN: &str = "fetch \"solis/world\":World;\n\
    object Score {\n\
        has {\n\
            int points = 0;\n\
        }\n\
        method update(float dt) {\n\
            points = points + 1;\n\
        }\n\
    }\n\
    component Spinner {\n\
        has {\n\
            float angle = 0;\n\
        }\n\
        method update(float dt) {\n\
            angle = angle + World:randomRange(0, 1) * dt;\n\
        }\n\
    }\n\
    pass Score, Spinner;\nend\n";

fn program() -> Arc<script::Compiled> {
    let (mut files, _) = script::describe::descriptions_for(
        "Test Engine 9.9.9",
        "solis",
        &script::project_names::Names::standard(),
    );
    files.push(("main.lst".into(), MAIN.to_string()));
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

const WHEEL: u64 = 7;

/// A wheel with a Spinner, its seed fixed.
fn wheel() -> Workbench {
    let mut w = Workbench::new(info());
    unhurried(&mut w);
    w.set_program(Some(program()));
    let mut data = EntityData::new("Wheel").with(&Transform::default());
    data.components
        .insert("Spinner".into(), ComponentData::default());
    let id = EntityId(reflect::Id::from_raw(WHEEL).unwrap());
    w.open
        .document
        .edit(
            "Add",
            vec![level::Edit::Spawn {
                entities: vec![(id, data)],
            }],
            None,
            None,
        )
        .unwrap();
    w.open.fixed_seed = Some(5);
    w
}

fn ticks(w: &mut Workbench, n: u64) {
    let until = w.world().unwrap().clock().ticks + n;
    while w.world().unwrap().clock().ticks < until {
        w.run_game((world::TICK * 0.6) as f32);
    }
}

#[test]
fn the_state_holds_running_components_and_objects() {
    let mut w = wheel();
    assert_eq!(w.game_state(), None, "no game runs");
    assert_eq!(w.game_checksums(), None);
    w.play();
    ticks(&mut w, 10);
    let state = w.game_state().unwrap();
    let id = EntityId(reflect::Id::from_raw(WHEEL).unwrap());
    let spinner = &state[&format!("{id} \"Wheel\" Spinner (running)")];
    assert!(spinner.starts_with("angle = 0."), "{spinner}");
    assert_eq!(state["object Score"], "points = 10");
    assert!(state.contains_key(&format!("{id} \"Wheel\" Transform (in the world)")));
    let (world_sum, game_sum) = w.game_checksums().unwrap();
    assert_eq!(world_sum, w.world().unwrap().checksum());
    assert_ne!(game_sum, 0);
}

#[test]
fn the_same_game_has_the_same_state_tick_by_tick() {
    // Only the wheel's parts and the objects: each workbench's starting
    // level has entities of its own random IDs.
    let wheel_id = EntityId(reflect::Id::from_raw(WHEEL).unwrap()).to_string();
    let run = || {
        let mut w = wheel();
        w.play();
        (0..5)
            .map(|_| {
                ticks(&mut w, 3);
                let mut state = w.game_state().unwrap();
                state.retain(|k, _| k.starts_with(&wheel_id) || k.starts_with("object "));
                (state, w.game_checksums().unwrap().1)
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(run(), run());
}
