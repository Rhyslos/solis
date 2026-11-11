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
        &script::project_names::Names::with_assemblies(&["drones/drone", "nowhere"]),
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
    w.open
        .document
        .edit(
            "Add",
            vec![spawn(FARMER, farmer), spawn(FENCE, fence)],
            None,
            None,
        )
        .unwrap();
    w.open.fixed_seed = Some(3);
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

/// A 2D character falling onto a floor, running right: game code's
/// `grounded()` and `velocity()` tell it so.
const FALLER: &str = "fetch \"solis/world\":World;\n\
component Faller {\n\
    has { int said = 0; }\n\
    method update(float dt) {\n\
        walk([3 * dt, 0 - 6 * dt, 0]);\n\
        is grounded() && said == 0? then {\n\
            said = 1;\n\
            World:say(\"landed going \" + velocity().x.text());\n\
        }\n\
    }\n\
}\n\
pass Faller;\nend\n";

#[test]
fn a_2d_character_knows_it_s_grounded_and_how_fast_it_goes() {
    let (mut files, _) = script::describe::descriptions_for(
        "Test Engine 9.9.9",
        "solis",
        &script::project_names::Names::with_assemblies(&[]),
    );
    files.push(("main.lst".into(), FALLER.to_string()));
    let code = script::Code {
        files,
        lua: Vec::new(),
        notes: Vec::new(),
    };
    let program = match script::compile(&code) {
        Ok(Some(c)) => Arc::new(c),
        Ok(None) => panic!("no code"),
        Err(problems) => panic!("{}", problems[0]),
    };
    let mut w = Workbench::new(info(), None);
    w.set_program(Some(program));
    let mut hero = EntityData::new("Hero")
        .with(&Transform {
            position: Vec3::new(0.0, 1.0, 0.0),
            ..Transform::default()
        })
        .with(&Collider2D {
            radius: 4.0,
            character: true,
            ..Collider2D::default()
        });
    hero.components
        .insert("Faller".into(), ComponentData::default());
    let floor = EntityData::new("Floor")
        .with(&Transform {
            position: Vec3::new(0.0, -0.5, 0.0),
            ..Transform::default()
        })
        .with(&Collider2D {
            shape: Shape2D::Box,
            width: 3200.0,
            height: 16.0,
            ..Collider2D::default()
        });
    w.open
        .document
        .edit(
            "Add",
            vec![
                area::Edit::Spawn {
                    entities: vec![(id(1), hero)],
                },
                area::Edit::Spawn {
                    entities: vec![(id(2), floor)],
                },
            ],
            None,
            None,
        )
        .unwrap();
    w.play();
    ticks(&mut w, 30);
    let landed: Vec<String> = w
        .log_lines()
        .iter()
        .filter(|l| l.text.contains("landed going"))
        .map(|l| l.text.clone())
        .collect();
    assert_eq!(landed.len(), 1, "it landed: {:?}", w.log_lines());
    let speed: f32 = landed[0].rsplit(' ').next().unwrap().parse().unwrap();
    assert!((speed - 3.0).abs() < 0.05, "running at 3 m/s: {speed}");
    // Standing on the floor, not in it.
    assert!(at(&w).y > -0.5 + 0.25, "{}", at(&w));
}

/// One of the platformer guide's code files, from its first line (`-- name`)
/// to its `end`.
fn guide_file(page: &str, first: &str) -> String {
    let start = page.find(first).expect(first);
    let end = start + page[start..].find("\nend\n").unwrap() + 5;
    page[start..end].to_string()
}

#[test]
fn the_platformer_guide_s_code_runs_jumps_and_collects_coins() {
    // docs/manual/first-platformer.md: its three files, as written there.
    let page = include_str!("../../../docs/manual/first-platformer.md").replace("\r\n", "\n");
    let mut main = guide_file(&page, "-- main.lst");
    // Step 7 adds the coin's fetch under the hero's.
    assert!(page.contains("```lust\nfetch \"coin\":[Coin];\n```"));
    main = main.replace(
        "fetch \"hero\":[Hero];\n",
        "fetch \"hero\":[Hero];\nfetch \"coin\":[Coin];\n",
    );
    let (mut files, _) = script::describe::descriptions_for(
        "Test Engine 9.9.9",
        "solis",
        &script::project_names::Names::with_assemblies(&[]),
    );
    files.push(("main.lst".into(), main));
    files.push(("hero.lst".into(), guide_file(&page, "-- hero.lst")));
    files.push(("coin.lst".into(), guide_file(&page, "-- coin.lst")));
    let code = script::Code {
        files,
        lua: Vec::new(),
        notes: Vec::new(),
    };
    let program = match script::compile(&code) {
        Ok(Some(c)) => Arc::new(c),
        Ok(None) => panic!("no code"),
        Err(problems) => panic!("{}", problems[0]),
    };
    const HERO: u64 = 1;
    const FLOOR: u64 = 2;
    const COIN: u64 = 3;
    let mut w = Workbench::new(info(), None);
    w.set_program(Some(program));
    // The guide's hero: a 12 × 16 px box raised 8 px onto its feet, a
    // tile above a floor whose top is at y = 0.
    let mut hero = EntityData::new("hero")
        .with(&Transform {
            position: Vec3::new(0.0, 1.0, 0.0),
            ..Transform::default()
        })
        .with(&Collider2D {
            shape: Shape2D::Box,
            width: 12.0,
            height: 16.0,
            up: 8.0,
            character: true,
            ..Collider2D::default()
        });
    hero.components
        .insert("Hero".into(), ComponentData::default());
    let floor = EntityData::new("Floor")
        .with(&Transform {
            position: Vec3::new(0.0, -0.5, 0.0),
            ..Transform::default()
        })
        .with(&Collider2D {
            shape: Shape2D::Box,
            width: 3200.0,
            height: 16.0,
            ..Collider2D::default()
        });
    let mut coin = EntityData::new("coin").with(&Transform {
        position: Vec3::new(5.0, 0.0, 0.0),
        ..Transform::default()
    });
    let mut collects = ComponentData::default();
    collects.set("player", Value::Entity(Some(id(HERO))));
    coin.components.insert("Coin".into(), collects);
    w.open
        .document
        .edit(
            "Add",
            vec![
                area::Edit::Spawn {
                    entities: vec![(id(HERO), hero)],
                },
                area::Edit::Spawn {
                    entities: vec![(id(FLOOR), floor)],
                },
                area::Edit::Spawn {
                    entities: vec![(id(COIN), coin)],
                },
            ],
            None,
            None,
        )
        .unwrap();
    w.open.fixed_seed = Some(3);
    w.play();
    let hero_at = |w: &Workbench| {
        w.world()
            .unwrap()
            .global(id(HERO))
            .unwrap()
            .w_axis
            .truncate()
    };
    // It falls and lands with its feet on the floor.
    ticks(&mut w, 60);
    let landed = hero_at(&w);
    assert!(landed.y.abs() < 0.05, "standing on the floor: {landed}");
    // A jump, pressed at any moment while standing, goes about 2.4 m up.
    for wait in [1, 2, 3, 7] {
        ticks(&mut w, wait);
        w.devices.set("Space", true);
        let mut top = 0.0_f32;
        for _ in 0..90 {
            ticks(&mut w, 1);
            w.devices.set("Space", false);
            top = top.max(hero_at(&w).y);
        }
        assert!(
            top > 2.2 && top < 2.6,
            "jumped {top} m after waiting {wait}"
        );
        assert!(hero_at(&w).y.abs() < 0.05, "and landed again");
    }
    // D runs right at 6 m/s, onto the coin, which is collected once.
    let from = hero_at(&w);
    w.devices.set("D", true);
    ticks(&mut w, 60);
    w.devices.set("D", false);
    let ran = hero_at(&w) - from;
    assert!(ran.x > 5.8 && ran.x < 6.1 && ran.y.abs() < 0.05, "{ran}");
    assert!(
        w.world().unwrap().global(id(COIN)).is_none(),
        "the coin is gone"
    );
    let said = w
        .log_lines()
        .iter()
        .filter(|l| l.text.contains("Got a coin!"))
        .count();
    assert_eq!(said, 1);
    assert!(
        !w.log_lines().iter().any(|l| l.text.contains("stopped")),
        "{:?}",
        w.log_lines()
    );
}

/// 2D physics stopping mid-game (Rapier failing on something) used to
/// leave every 2D character standing still for the rest of the play,
/// with nothing said (the full review). It's started again: the farmer
/// walks on and still stops at the fence, the game isn't paused, and the
/// Console says why once, however often it happens.
#[test]
fn when_2d_physics_stops_it_s_started_again_and_the_console_says_so_once() {
    use crate::play::PlayState;
    let said = |w: &Workbench| {
        w.log_lines()
            .iter()
            .filter(|line| line.text.starts_with("2D physics stopped"))
            .count()
    };
    let fail = |w: &mut Workbench| {
        let playing = w.open.playing.as_mut().unwrap();
        playing.world.fail_flat_next();
    };
    let mut w = farm(&[]);
    w.play();
    ticks(&mut w, 5);
    walk(&mut w, &["D"], 30);
    assert_eq!(said(&w), 0);
    fail(&mut w);
    let went = walk(&mut w, &["D"], 1);
    assert!(
        went.x > 0.05,
        "the tick's walk was made all the same: {went}"
    );
    assert_eq!(said(&w), 1);
    assert_eq!(w.play_state(), PlayState::Playing, "the game carries on");
    // Again the tick after: the farmer stands still until it's started
    // again (within a second), then walks on to the fence. Nothing more
    // is said.
    fail(&mut w);
    let went = walk(&mut w, &["D"], 1);
    assert_eq!(went, Vec3::ZERO, "standing still");
    walk(&mut w, &["D"], 200);
    let x = at(&w).x;
    assert!(x < 6.0 - 0.25 && x > 6.0 - 0.25 - 0.02, "{x}");
    assert_eq!(said(&w), 1, "said once a play");
    assert_eq!(w.play_state(), PlayState::Playing);
    // The next play says it again.
    w.stop();
    w.play();
    ticks(&mut w, 5);
    fail(&mut w);
    walk(&mut w, &["D"], 2);
    assert_eq!(said(&w), 2);
}
