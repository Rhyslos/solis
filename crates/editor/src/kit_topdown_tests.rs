//! The kit's top-down walker (decision 0071) played with input: WASD and
//! the arrow keys walk at its speed, never faster diagonally, speeding up
//! and slowing down smoothly; four directions drop diagonals; it stops at
//! a solid 2D collider and slides along it; and game code can walk it
//! instead of the keys.

use crate::test_support::*;
use crate::workbench::Workbench;
use area::EntityData;
use area::builtin::{Collider2D, Shape2D, Transform};
use glam::Vec3;
use reflect::{ComponentData, EntityId, Value};
use std::sync::Arc;

const MAIN: &str = "fetch \"solis/kit/topdown\":[TopDownController];\nend\n";

fn program() -> Arc<script::Compiled> {
    let (mut files, _) = script::describe::descriptions_for(
        "Test Engine 9.9.9",
        "solis",
        &["drones/drone".to_string(), "nowhere".to_string()],
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

fn id(n: u64) -> EntityId {
    EntityId(reflect::Id::from_raw(n).unwrap())
}

const FARMER: u64 = 1;
const FENCE: u64 = 2;

/// A farmer (a 4 px circle, walked by the kit) at the origin and a fence
/// (a solid box 16 px wide, 20 m tall) whose left face is at x = 6.
fn farm(fields: &[(&str, Value)]) -> Workbench {
    let mut w = Workbench::new(info(), None);
    w.set_program(Some(program()));
    let mut farmer = EntityData::new("Farmer")
        .with(&Transform::default())
        .with(&Collider2D {
            radius: 4.0,
            character: true,
            ..Collider2D::default()
        });
    let mut walker = ComponentData::default();
    for (name, value) in fields {
        walker.set(name, value.clone());
    }
    farmer.components.insert("TopDownController".into(), walker);
    let fence = EntityData::new("Fence")
        .with(&Transform {
            position: Vec3::new(6.5, 0.0, 0.0),
            ..Transform::default()
        })
        .with(&Collider2D {
            shape: Shape2D::Box,
            width: 16.0,
            height: 320.0,
            ..Collider2D::default()
        });
    let spawn = |n: u64, data: EntityData| area::Edit::Spawn {
        entities: vec![(id(n), data)],
    };
    w.document
        .edit(
            "Add",
            vec![spawn(FARMER, farmer), spawn(FENCE, fence)],
            None,
            None,
        )
        .unwrap();
    w.fixed_seed = Some(3);
    w
}

fn ticks(w: &mut Workbench, n: u64) {
    let until = w.world().unwrap().clock().ticks + n;
    while w.world().unwrap().clock().ticks < until {
        w.run_game((world::TICK * 0.6) as f32);
    }
}

fn at(w: &Workbench) -> Vec3 {
    w.world()
        .unwrap()
        .global(id(FARMER))
        .unwrap()
        .w_axis
        .truncate()
}

fn walk(w: &mut Workbench, keys: &[&str], n: u64) -> Vec3 {
    let from = at(w);
    for k in keys {
        w.devices.set(k, true);
    }
    ticks(w, n);
    for k in keys {
        w.devices.set(k, false);
    }
    at(w) - from
}

#[test]
fn wasd_and_the_arrows_walk_at_its_speed_and_never_faster_diagonally() {
    let mut w = farm(&[]);
    w.play();
    ticks(&mut w, 10);
    assert!(at(&w).length() < 1e-4, "standing still");
    // Up for a second: 4 m/s, less the moment speeding up takes.
    let up = walk(&mut w, &["W"], 60);
    assert!(up.y > 3.7 && up.y < 4.0 && up.x.abs() < 1e-4, "{up}");
    // It slows down smoothly after: on a little, then still.
    let after = walk(&mut w, &[], 30);
    assert!(after.y > 0.05 && after.y < 0.3, "slowed: {after}");
    // The arrow keys walk too.
    let down = walk(&mut w, &["Down"], 60);
    assert!(down.y < -3.7, "{down}");
    walk(&mut w, &[], 30);
    // Diagonally (W and A at once), no faster than straight.
    let diagonal = walk(&mut w, &["W", "A"], 60);
    let speed = diagonal.truncate().length();
    assert!(speed > 3.7 && speed < 4.0, "{diagonal}");
    assert!((diagonal.x + diagonal.y).abs() < 1e-3, "both ways alike");
}

#[test]
fn four_directions_drop_diagonals() {
    let mut w = farm(&[("directions", Value::Int(4))]);
    w.play();
    ticks(&mut w, 5);
    let went = walk(&mut w, &["W", "D"], 30);
    assert!(
        went.x.abs() < 1e-4 || went.y.abs() < 1e-4,
        "one way only: {went}"
    );
    assert!(went.length() > 1.7, "as fast as straight: {went}");
}

#[test]
fn it_stops_at_a_solid_collider_and_slides_along_it() {
    let mut w = farm(&[]);
    w.play();
    ticks(&mut w, 5);
    walk(&mut w, &["D"], 150);
    let x = at(&w).x;
    // The fence's face is at 6, the farmer's circle a quarter metre.
    assert!(x < 6.0 - 0.25 && x > 6.0 - 0.25 - 0.02, "{x}");
    // Pressing into it and up: it slides up along it.
    let went = walk(&mut w, &["D", "W"], 60);
    assert!(went.y > 2.5 && went.x.abs() < 0.02, "{went}");
}

#[test]
fn game_code_can_walk_it_instead_of_the_keys() {
    let mut w = farm(&[
        ("usesInput", Value::Bool(false)),
        ("wanted", Value::vec3(Vec3::new(-1.0, 0.0, 0.0))),
    ]);
    w.play();
    // Keys do nothing; it heads where it's told.
    let went = walk(&mut w, &["W"], 60);
    assert!(went.x < -3.7 && went.y.abs() < 1e-4, "{went}");
}
