//! The ready-made components (Phase 6, M5) played with input: the
//! character walks the way the camera looks, sprints, jumps and lands,
//! climbs a step and stops at a wall; the third-person camera keeps out of
//! walls and zooms only when allowed; the first-person camera turns the
//! character; the RTS camera pans, drags, turns and zooms with the chosen
//! controls; and a recorded walk plays back exactly.

use crate::test_support::*;
use crate::workbench::{Workbench, WorkbenchAction};
use area::EntityData;
use area::builtin::{Camera, CharacterBody, Collider, Transform};
use glam::{Quat, Vec3};
use reflect::{ComponentData, EntityId, Value};
use std::sync::Arc;

const MAIN: &str = "fetch \"solis/kit/character\":[CharacterController];\n\
    fetch \"solis/kit/cameras\":[ThirdPersonCamera, FirstPersonCamera, RtsCamera];\n\
    end\n";

fn program() -> Arc<script::Compiled> {
    let mut files = script::describe::descriptions("Test Engine 9.9.9", "solis");
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

const FLOOR: u64 = 1;
const PLAYER: u64 = 2;
const CAMERA: u64 = 3;
const WALL: u64 = 4;
const STEP: u64 = 5;

fn block(at: Vec3, size: Vec3) -> EntityData {
    EntityData::new("Block")
        .with(&Transform {
            position: at,
            scale: size,
            ..Transform::default()
        })
        .with(&Collider::default())
}

/// A workbench with a floor, a wall across +Y at y = 8 (its face at
/// 7.75), a 25 cm step along +X from x = 4, a character at the origin and
/// a camera with `camera` (a component name and its fields).
fn level(camera: &str, fields: &[(&str, Value)]) -> Workbench {
    let mut w = Workbench::new(info(), None);
    w.set_program(Some(program()));
    let mut player = EntityData::new("Player")
        .with(&Transform::default())
        .with(&CharacterBody::default());
    let mut controller = ComponentData::default();
    controller.set("camera", Value::Entity(Some(id(CAMERA))));
    player
        .components
        .insert("CharacterController".into(), controller);
    let mut eye = EntityData::new("Camera")
        .with(&Transform {
            position: Vec3::new(0.0, -4.0, 2.0),
            ..Transform::default()
        })
        .with(&Camera::default());
    let mut data = ComponentData::default();
    for (name, value) in fields {
        data.set(name, value.clone());
    }
    eye.components.insert(camera.into(), data);
    let spawn = |n: u64, data: EntityData| area::Edit::Spawn {
        entities: vec![(id(n), data)],
    };
    w.document
        .edit(
            "Add",
            vec![
                spawn(
                    FLOOR,
                    block(Vec3::new(0.0, 0.0, -0.5), Vec3::new(60.0, 60.0, 1.0)),
                ),
                spawn(PLAYER, player),
                spawn(CAMERA, eye),
                spawn(
                    WALL,
                    block(Vec3::new(0.0, 8.0, 2.0), Vec3::new(20.0, 0.5, 4.0)),
                ),
                spawn(
                    STEP,
                    block(Vec3::new(8.0, -20.0, 0.125), Vec3::new(8.0, 4.0, 0.25)),
                ),
            ],
            None,
            None,
        )
        .unwrap();
    w.fixed_seed = Some(3);
    w
}

fn third_person() -> Workbench {
    level(
        "ThirdPersonCamera",
        &[("target", Value::Entity(Some(id(PLAYER))))],
    )
}

/// Runs exactly `n` ticks.
fn ticks(w: &mut Workbench, n: u64) {
    let until = w.world().unwrap().clock().ticks + n;
    while w.world().unwrap().clock().ticks < until {
        w.run_game((world::TICK * 0.6) as f32);
    }
}

fn at(w: &Workbench, n: u64) -> Vec3 {
    w.world().unwrap().global(id(n)).unwrap().w_axis.truncate()
}

fn facing(w: &Workbench, n: u64) -> Quat {
    w.world()
        .unwrap()
        .global(id(n))
        .unwrap()
        .to_scale_rotation_translation()
        .1
}

fn no_problems(w: &Workbench) {
    let problems: Vec<&str> = w
        .log_lines()
        .iter()
        .map(|l| l.text.as_str())
        .filter(|t| t.contains("stopped"))
        .collect();
    assert!(problems.is_empty(), "{problems:?}");
}

#[test]
fn the_character_walks_the_way_the_camera_looks_and_sprints() {
    let mut w = third_person();
    w.play();
    ticks(&mut w, 30);
    assert!(at(&w, PLAYER).length() < 0.05, "standing still");
    // The camera sits behind, looking along +Y: W walks along +Y.
    w.devices.set("W", true);
    ticks(&mut w, 60);
    let walked = at(&w, PLAYER);
    assert!(
        (walked.y - 4.0).abs() < 0.3 && walked.x.abs() < 0.05,
        "walked to {walked}"
    );
    // It turned to face where it walks (+Y: no turn).
    assert!(facing(&w, PLAYER).angle_between(Quat::IDENTITY) < 0.05);
    // Turn the camera a quarter to the left (mouse left), then walk: now
    // along -X, the character turning to face it.
    w.devices.set("W", false);
    ticks(&mut w, 10);
    w.devices.mouse_moved(glam::Vec2::new(-600.0, 0.0));
    ticks(&mut w, 1);
    let from = at(&w, PLAYER);
    w.devices.set("W", true);
    ticks(&mut w, 60);
    let went = at(&w, PLAYER) - from;
    assert!(went.x < -3.0 && went.y.abs() < 0.3, "went {went}");
    let forward = facing(&w, PLAYER) * Vec3::Y;
    assert!(forward.dot(Vec3::NEG_X) > 0.95, "faces {forward}");
    // Sprinting goes 7 m/s instead of 4.
    let from = at(&w, PLAYER);
    w.devices.set("Shift", true);
    ticks(&mut w, 60);
    let sprinted = (at(&w, PLAYER) - from).length();
    assert!((sprinted - 7.0).abs() < 0.4, "sprinted {sprinted}");
    no_problems(&w);
}

#[test]
fn the_character_jumps_lands_climbs_a_step_and_stops_at_a_wall() {
    let mut w = third_person();
    w.play();
    ticks(&mut w, 20);
    // Space jumps about 1.2 m, and it lands again.
    w.devices.set("Space", true);
    ticks(&mut w, 1);
    w.devices.set("Space", false);
    let mut highest: f32 = 0.0;
    for _ in 0..60 {
        ticks(&mut w, 1);
        highest = highest.max(at(&w, PLAYER).z);
    }
    assert!((highest - 1.2).abs() < 0.15, "jumped {highest}");
    assert!(at(&w, PLAYER).z.abs() < 0.05, "landed");
    // Walking into the wall stops at its face (7.75, less its 0.35 m).
    w.devices.set("W", true);
    ticks(&mut w, 150);
    let stopped = at(&w, PLAYER);
    assert!(stopped.y > 7.2 && stopped.y < 7.45, "stopped at {stopped}");
    // The camera, behind, never went through the wall either way.
    assert!(at(&w, CAMERA).y < 7.75);
    no_problems(&w);
}

#[test]
fn the_character_climbs_a_step() {
    let mut w = third_person();
    // Put the character beside the step, facing it along +X, and the
    // camera turned to look along +X.
    w.play();
    let p = w.playing.as_mut().unwrap();
    p.world.set_transform(
        id(PLAYER),
        Transform {
            position: Vec3::new(2.0, -20.0, 0.0),
            ..Transform::default()
        },
    );
    ticks(&mut w, 30);
    w.devices.mouse_moved(glam::Vec2::new(600.0, 0.0));
    ticks(&mut w, 1);
    w.devices.set("W", true);
    ticks(&mut w, 60);
    let up = at(&w, PLAYER);
    assert!(up.x > 4.5, "walked on: {up}");
    assert!((up.z - 0.25).abs() < 0.05, "up the step: {up}");
    no_problems(&w);
}

#[test]
fn the_third_person_camera_keeps_out_of_walls_and_zooms_only_when_allowed() {
    let mut w = third_person();
    w.play();
    ticks(&mut w, 10);
    let normal = at(&w, CAMERA) - at(&w, PLAYER);
    // 4 m from the pivot, 1.6 m up.
    let pivot = at(&w, PLAYER) + Vec3::Z * 1.6;
    assert!(((at(&w, CAMERA) - pivot).length() - 4.0).abs() < 0.05);
    // The wheel does nothing with wheelZooms off (the default).
    w.devices.wheel_turned(5.0);
    ticks(&mut w, 2);
    assert!((at(&w, CAMERA) - at(&w, PLAYER) - normal).length() < 0.01);
    // Turned round to look back at the wall from close by: the camera
    // stays on this side of it, pulled in.
    w.devices.set("W", true);
    ticks(&mut w, 120);
    w.devices.set("W", false);
    w.devices.mouse_moved(glam::Vec2::new(1200.0, 0.0));
    ticks(&mut w, 2);
    let eye = at(&w, CAMERA);
    assert!(eye.y < 7.75 - 0.05, "kept out of the wall: {eye}");
    let pivot = at(&w, PLAYER) + Vec3::Z * 1.6;
    assert!((eye - pivot).length() < 1.0, "pulled in: {eye}");
    no_problems(&w);
    // With wheelZooms on, the wheel brings it closer, down to 1.5 m.
    let mut w = level(
        "ThirdPersonCamera",
        &[
            ("target", Value::Entity(Some(id(PLAYER)))),
            ("wheelZooms", Value::Bool(true)),
        ],
    );
    w.play();
    ticks(&mut w, 10);
    w.devices.wheel_turned(2.0);
    ticks(&mut w, 2);
    let pivot = at(&w, PLAYER) + Vec3::Z * 1.6;
    let near = (at(&w, CAMERA) - pivot).length();
    assert!((near - 4.0 * 0.85 * 0.85).abs() < 0.05, "zoomed to {near}");
    w.devices.wheel_turned(50.0);
    ticks(&mut w, 2);
    let pivot = at(&w, PLAYER) + Vec3::Z * 1.6;
    assert!(((at(&w, CAMERA) - pivot).length() - 1.5).abs() < 0.05);
}

#[test]
fn the_first_person_camera_sees_from_the_eyes_and_turns_the_body() {
    let mut w = level(
        "FirstPersonCamera",
        &[("body", Value::Entity(Some(id(PLAYER))))],
    );
    w.play();
    ticks(&mut w, 10);
    let eye = at(&w, CAMERA);
    assert!(
        (eye - (at(&w, PLAYER) + Vec3::Z * 1.65)).length() < 0.02,
        "{eye}"
    );
    // Looking a quarter right turns the body with it, and W walks that
    // way.
    w.devices.mouse_moved(glam::Vec2::new(600.0, 0.0));
    ticks(&mut w, 2);
    let forward = facing(&w, PLAYER) * Vec3::Y;
    assert!(forward.dot(Vec3::X) > 0.99, "body faces {forward}");
    let looks = facing(&w, CAMERA) * Vec3::NEG_Z;
    assert!(looks.dot(Vec3::X) > 0.99, "camera looks {looks}");
    let from = at(&w, PLAYER);
    w.devices.set("W", true);
    ticks(&mut w, 30);
    let went = at(&w, PLAYER) - from;
    assert!(went.x > 1.5 && went.y.abs() < 0.1, "went {went}");
    // Looking up is held at 85 degrees.
    w.devices.mouse_moved(glam::Vec2::new(0.0, -5000.0));
    ticks(&mut w, 2);
    let looks = facing(&w, CAMERA) * Vec3::NEG_Z;
    assert!((looks.z.asin().to_degrees() - 85.0).abs() < 0.5, "{looks}");
    no_problems(&w);
}

#[test]
fn the_rts_camera_pans_drags_turns_and_zooms_with_the_chosen_controls() {
    let mut w = level("RtsCamera", &[]);
    w.play();
    ticks(&mut w, 2);
    // 30 m up, looking along +Y tilted 55 degrees down at the focus.
    let start = at(&w, CAMERA);
    assert!((start.z - 30.0).abs() < 0.01, "{start}");
    let looks = facing(&w, CAMERA) * Vec3::NEG_Z;
    assert!((looks.z.asin().to_degrees() + 55.0).abs() < 0.1, "{looks}");
    // The up arrow pans along +Y at 20 m/s.
    w.devices.set("ArrowUp", true);
    ticks(&mut w, 60);
    w.devices.set("ArrowUp", false);
    let panned = at(&w, CAMERA) - start;
    assert!(
        (panned.y - 20.0).abs() < 0.5 && panned.x.abs() < 0.01,
        "{panned}"
    );
    // Alt + left arrow turns it left, and doesn't pan.
    let before = at(&w, CAMERA);
    w.devices.set("Alt", true);
    w.devices.set("ArrowLeft", true);
    ticks(&mut w, 60);
    w.devices.set("ArrowLeft", false);
    w.devices.set("Alt", false);
    let looks = facing(&w, CAMERA) * Vec3::NEG_Z;
    let flat = Vec3::new(looks.x, looks.y, 0.0).normalize();
    assert!(flat.dot(Vec3::NEG_X) > 0.99, "turned to look {flat}");
    // (It orbits round its focus, so its place changed, but the focus
    // didn't: the same distance from where it was looking.)
    let focus_of = |w: &Workbench| {
        let eye = at(w, CAMERA);
        let looks = facing(w, CAMERA) * Vec3::NEG_Z;
        eye + looks * (eye.z / -looks.z)
    };
    let focus = focus_of(&w);
    assert!(
        (focus - (before + Vec3::new(0.0, 30.0 / 55_f32.to_radians().tan(), -30.0))).length() < 0.2
    );
    // The wheel zooms in; Page Down zooms out.
    w.devices.wheel_turned(2.0);
    ticks(&mut w, 1);
    assert!((at(&w, CAMERA).z - 30.0 * 0.88 * 0.88).abs() < 0.05);
    w.devices.set("PageDown", true);
    ticks(&mut w, 60);
    w.devices.set("PageDown", false);
    assert!(
        at(&w, CAMERA).z > 30.0,
        "zoomed out to {}",
        at(&w, CAMERA).z
    );
    // Holding the middle button and moving the mouse drags the map.
    let focus = focus_of(&w);
    w.devices.set("Mouse Middle", true);
    w.devices.mouse_moved(glam::Vec2::new(0.0, 100.0));
    ticks(&mut w, 1);
    w.devices.set("Mouse Middle", false);
    assert!((focus_of(&w) - focus).length() > 1.0, "dragged");
    // Without the button, moving the mouse doesn't.
    let focus = focus_of(&w);
    w.devices.mouse_moved(glam::Vec2::new(0.0, 100.0));
    ticks(&mut w, 1);
    assert!((focus_of(&w) - focus).length() < 0.01);
    no_problems(&w);
}

#[test]
fn a_recorded_walk_plays_back_exactly() {
    let mut w = third_person();
    w.play();
    let playing = w.playing.as_mut().unwrap();
    playing.recording = Some(input::Recording {
        seed: playing.world.seed(),
        start: playing.world.checksum(),
        ticks: Vec::new(),
    });
    for n in 0..180u64 {
        w.devices.set("W", n % 60 < 40);
        w.devices.set("Space", n % 45 == 0);
        w.devices.set("Shift", n > 90);
        if n % 30 == 0 {
            w.devices.mouse_moved(glam::Vec2::new(57.0, -13.0));
        }
        ticks(&mut w, 1);
    }
    w.devices.release_all();
    w.stop();
    let text = w
        .outbox
        .iter()
        .find_map(|a| match a {
            WorkbenchAction::SaveRecording(text) => Some(text.clone()),
            _ => None,
        })
        .expect("Stop saves the recording");
    w.fixed_seed = None;
    w.replay(&text, "walk.solrec");
    for _ in 0..400 {
        if w.play_state() == crate::play::PlayState::Paused {
            break;
        }
        w.run_game((world::TICK * 3.0) as f32);
    }
    assert!(
        w.log_lines()
            .iter()
            .any(|l| l.text.contains("matched the recording on all 180 ticks")),
        "{:?}",
        w.log_lines().iter().map(|l| &l.text).collect::<Vec<_>>()
    );
}

#[test]
fn the_manual_s_hopper_is_sound_code_and_hops() {
    // docs/manual/characters-and-cameras.md, "Moving a character from
    // your own code".
    let page = include_str!("../../../docs/manual/characters-and-cameras.md");
    let start = page
        .find("get \"solis/physics\":[moveCharacter, onGround];")
        .unwrap();
    let end = start + page[start..].find("end\n").unwrap() + 4;
    let code = &page[start..end];
    let mut files = script::describe::descriptions("Test Engine 9.9.9", "solis");
    files.push(("main.lst".into(), code.to_string()));
    let compiled = script::compile(&script::Code {
        files,
        lua: Vec::new(),
        notes: Vec::new(),
    });
    let Ok(Some(compiled)) = compiled else {
        panic!("{:?}", compiled.err());
    };
    let mut w = level("ThirdPersonCamera", &[]);
    w.set_program(Some(Arc::new(compiled)));
    let mut data = w.document.area().get(id(PLAYER)).unwrap().clone();
    data.components.clear();
    data.set(&Transform::default());
    data.set(&CharacterBody::default());
    data.components
        .insert("Hopper".into(), ComponentData::default());
    w.document
        .edit(
            "Hopper",
            vec![
                area::Edit::Despawn { root: id(PLAYER) },
                area::Edit::Spawn {
                    entities: vec![(id(PLAYER), data)],
                },
            ],
            None,
            None,
        )
        .unwrap();
    w.play();
    ticks(&mut w, 10);
    w.devices.set("Space", true);
    ticks(&mut w, 10);
    assert!(at(&w, PLAYER).z > 0.3, "hopped: {}", at(&w, PLAYER));
    no_problems(&w);
}
