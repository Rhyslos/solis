//! Trigger zones while playing (stage 1, triggers): a walker in a 2D
//! level walks through a zone, and the zone's Lust, Lua and Rust
//! components hear `Entered` and `Exited`, the walker's `EnteredZone` and
//! `LeftZone`, in order; a save made inside loads without a second
//! `Entered`; a zone that fires once stays quiet across a save; and the
//! same game is the same on one core or four.

use crate::test_support::*;
use crate::workbench::Workbench;
use glam::Vec3;
use level::EntityData;
use level::builtin::{Collider2D, Fires, Transform, TriggerZone};
use reflect::{ComponentData, EntityId, Value};
use script::{Context, Event, RustComponent, RustKind};
use std::sync::Arc;

const MAIN: &str = "fetch \"solis/world\":World;\n\
    fetch \"solis/zones\":[Entered, Exited, EnteredZone, LeftZone];\n\
    \n\
    component DoorLust {\n\
    on {\n\
    \x20   Entered(entity who) {\n\
    \x20   World:say(\"Lust: \" + who:name() + \" in, \" + (#inside()).text() + \" inside\");\n\
    \x20   }\n\
    \n\
    \x20   Exited(entity who) {\n\
    \x20   World:say(\"Lust: \" + who:name() + \" out\");\n\
    \x20   }\n\
    }\n\
    }\n\
    component Visitor {\n\
    has {\n\
    float speed = 3;\n\
    }\n\
    method update(float dt) {\n\
    walk([speed * dt, 0, 0]);\n\
    }\n\
    on {\n\
    \x20   EnteredZone(entity zone) {\n\
    \x20   World:say(name() + \" entered \" + zone:name());\n\
    \x20   }\n\
    \n\
    \x20   LeftZone(entity zone) {\n\
    \x20   World:say(name() + \" left \" + zone:name());\n\
    \x20   }\n\
    }\n\
    }\n\
    pass DoorLust, Visitor;\n\
    end\n\
    ";

const LUA: &str = "local D = component(\"DoorLua\", {})\n\
    function D:on_Entered(who) solis.World:say(\"Lua: \" .. solis.entity.name(who) .. \" in\") end\n\
    function D:on_Exited(who) solis.World:say(\"Lua: \" .. solis.entity.name(who) .. \" out\") end\n";

/// A game's Rust component on the zone: says who came and went.
struct DoorRust;

impl RustComponent for DoorRust {
    fn handle(&mut self, cx: &mut Context<'_>, event: &Event) -> bool {
        let Some(Value::Entity(Some(who))) = event.values.first() else {
            return false;
        };
        let name = cx.host().name(*who).unwrap_or_default();
        let words = match event.name.as_str() {
            "Entered" => format!("Rust: {name} in"),
            "Exited" => format!("Rust: {name} out"),
            _ => return false,
        };
        cx.host().say(&words);
        true
    }
    fn values(&self) -> ComponentData {
        ComponentData::default()
    }
    fn set(&mut self, _data: &ComponentData) {}
}

fn door_rust() -> RustKind {
    RustKind {
        schema: reflect::ComponentSchema {
            name: "DoorRust".into(),
            help: String::new(),
            fields: Vec::new(),
        },
        make: |_| Box::new(DoorRust),
    }
}

fn program() -> Arc<script::Compiled> {
    let (mut files, _) = script::describe::descriptions_for(
        "Test Engine 9.9.9",
        "solis",
        &script::project_names::Names::standard(),
    );
    files.push(("main.lst".into(), MAIN.to_string()));
    let code = script::Code {
        files,
        lua: vec![("door.lua".into(), LUA.into())],
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

const DOOR: u64 = 1;
const FARMER: u64 = 2;

/// A door zone (2 m square at x = 5) with a component in each language,
/// and a farmer (a 4 px circle) at the origin walking right at 3 m/s.
fn walk_through(fires: Fires) -> Workbench {
    let mut w = Workbench::new(info());
    unhurried(&mut w);
    // Save games are kept in a project.
    w.open.project = Some(crate::library::ProjectInfo {
        name: "Doors".into(),
        root: std::env::temp_dir(),
        assets: std::env::temp_dir(),
    });
    w.set_rust_components(vec![door_rust()]);
    w.set_program(Some(program()));
    let mut door = EntityData::new("Door")
        .with(&Transform {
            position: Vec3::new(5.0, 0.0, 0.0),
            ..Transform::default()
        })
        .with(&TriggerZone {
            fires,
            ..TriggerZone::default()
        });
    for name in ["DoorLust", "DoorLua", "DoorRust"] {
        door.components
            .insert(name.into(), ComponentData::default());
    }
    let mut farmer = EntityData::new("Farmer")
        .with(&Transform::default())
        .with(&Collider2D {
            radius: 4.0,
            character: true,
            ..Collider2D::default()
        });
    farmer
        .components
        .insert("Visitor".into(), ComponentData::default());
    let spawn = |n: u64, data: EntityData| level::Edit::Spawn {
        entities: vec![(id(n), data)],
    };
    w.open
        .document
        .edit(
            "Add",
            vec![spawn(DOOR, door), spawn(FARMER, farmer)],
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

/// What game code said about the zone, in order.
fn heard(w: &Workbench) -> Vec<String> {
    w.log_lines()
        .iter()
        .map(|l| l.text.clone())
        .filter(|t| {
            t.starts_with("Lust: ")
                || t.starts_with("Lua: ")
                || t.starts_with("Rust: ")
                || t.starts_with("Farmer ")
        })
        .collect()
}

/// Each event reaches the zone's components in order of their names
/// (DoorLua, DoorLust, DoorRust), then the walker's.
#[test]
fn each_event_reaches_lust_lua_and_rust_in_order() {
    let mut w = walk_through(Fires::EveryTime);
    w.play();
    ticks(&mut w, 150);
    assert_eq!(
        heard(&w),
        [
            "Lua: Farmer in",
            "Lust: Farmer in, 1 inside",
            "Rust: Farmer in",
            "Farmer entered Door",
            "Lua: Farmer out",
            "Lust: Farmer out",
            "Rust: Farmer out",
            "Farmer left Door",
        ]
    );
}

#[test]
fn inside_now_is_filled_while_playing() {
    let mut w = walk_through(Fires::EveryTime);
    w.play();
    ticks(&mut w, 85);
    assert_eq!(w.world().unwrap().inside(id(DOOR)), [id(FARMER)]);
    ticks(&mut w, 60);
    assert!(w.world().unwrap().inside(id(DOOR)).is_empty());
}

#[test]
fn a_save_made_inside_loads_without_a_second_entered() {
    let mut w = walk_through(Fires::EveryTime);
    w.play();
    ticks(&mut w, 85);
    assert_eq!(heard(&w).len(), 4, "in: {:?}", heard(&w));
    let text = w.save_of_game().unwrap().write().unwrap();
    assert!(text.contains("\nin "), "the save keeps who is inside");
    w.load_game(&text, "quick");
    ticks(&mut w, 5);
    assert_eq!(heard(&w).len(), 4, "no second Entered: {:?}", heard(&w));
    assert_eq!(w.world().unwrap().inside(id(DOOR)), [id(FARMER)]);
    ticks(&mut w, 60);
    assert_eq!(heard(&w).len(), 8, "it still leaves: {:?}", heard(&w));
}

#[test]
fn a_zone_that_fires_once_stays_quiet_across_a_save() {
    let mut w = walk_through(Fires::Once);
    w.play();
    ticks(&mut w, 85);
    let text = w.save_of_game().unwrap().write().unwrap();
    assert!(text.contains("\nonce "), "the save keeps that it fired");
    w.stop();
    w.play();
    // Loaded, the farmer walks out, is put back before the door, and
    // walks in again: nothing more is heard.
    w.load_game(&text, "quick");
    ticks(&mut w, 60);
    let before = heard(&w).len();
    let playing = w.open.playing.as_mut().unwrap();
    playing
        .world
        .set_transform(id(FARMER), Transform::default());
    ticks(&mut w, 120);
    let said = heard(&w);
    assert_eq!(
        said[before..].len(),
        0,
        "nothing after the first: {:?}",
        &said[before..]
    );
}

#[test]
fn the_same_game_on_one_core_or_four_hears_the_same() {
    let run = |threads: usize| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(|| {
                let mut w = walk_through(Fires::EveryTime);
                w.play();
                ticks(&mut w, 150);
                // Not the checksum: each workbench's starting level has
                // entities of its own random IDs.
                let world = w.world().unwrap();
                (heard(&w), world.zone_states(), world.global(id(FARMER)))
            })
    };
    assert_eq!(run(1), run(4));
}
