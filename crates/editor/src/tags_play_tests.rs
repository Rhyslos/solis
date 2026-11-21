//! Tags while playing (stage 1, tags): a hunter asks the world for the
//! nearest enemy and those within reach, in Lust, Lua and Rust; tags game
//! code adds and takes away land at the end of the tick; a 2D project
//! measures across the ground; a save keeps the tags; and the same game is
//! the same on one core or four.

use crate::test_support::*;
use crate::workbench::Workbench;
use glam::Vec3;
use level::EntityData;
use level::builtin::{Tags, Transform};
use reflect::{ComponentData, EntityId, TagId};
use script::{Context, RustComponent, RustKind};
use std::sync::Arc;

const MAIN: &str = "fetch \"solis/world\":World;\nfetch \"solis/project\":Tag;\n\n\
    component Hunter {\n\
        has {\n\
            int ticks = 0;\n\
        }\n\
        method update(float dt) {\n\
            ticks = ticks + 1;\n\
            entity closest = World:nearest(Tag.Enemy, position());\n\
            entity array near = World:within(Tag.Enemy, position(), 4);\n\
            string seen = \"none\";\n\
            is closest != nothing? then {\n\
                seen = closest:name();\n\
            }\n\
            string mine = \"false\";\n\
            is hasTag(Tag.Enemy)? then {\n\
                mine = \"true\";\n\
            }\n\
            World:say(\"Lust: tick \" + ticks.text() + \" nearest \" + seen + \", \" + (#near).text() + \" near, enemy \" + mine);\n\
            is ticks == 2? then {\n\
                addTag(Tag.Enemy);\n\
            }\n\
            is ticks == 3 && closest != nothing? then {\n\
                closest:removeTag(Tag.Enemy);\n\
            }\n\
        }\n\
    }\n\
    pass Hunter;\nend\n";

const LUA: &str = "local S = component(\"SeekerLua\", {})\n\
    function S:update(dt)\n\
        local near = solis.World:within(\"Enemy\", solis.entity.position(self.me), 4)\n\
        solis.World:say(\"Lua: \" .. #near .. \" near\")\n\
    end\n";

/// A game's Rust component: says the nearest enemy's name.
struct SeekerRust;

impl RustComponent for SeekerRust {
    fn update(&mut self, cx: &mut Context<'_>, _dt: f64) {
        let me = cx.me();
        let place = cx.host().position(me).unwrap_or(Vec3::ZERO);
        let nearest = cx.host().nearest("Enemy", place);
        let name = nearest
            .and_then(|e| cx.host().name(e))
            .unwrap_or_else(|| "none".into());
        cx.host().say(&format!("Rust: nearest {name}"));
    }
    fn values(&self) -> ComponentData {
        ComponentData::default()
    }
    fn set(&mut self, _data: &ComponentData) {}
}

fn seeker_rust() -> RustKind {
    RustKind {
        schema: reflect::ComponentSchema {
            name: "SeekerRust".into(),
            help: String::new(),
            fields: Vec::new(),
        },
        reads: &[],
        make: |_| Box::new(SeekerRust),
    }
}

fn program() -> Arc<script::Compiled> {
    let (mut files, _) = script::describe::descriptions_for(
        "Test Engine 9.9.9",
        "solis",
        &script::project_names::Names::standard().and_tags(&["Enemy", "Flying"]),
    );
    files.push(("main.lst".into(), MAIN.to_string()));
    let code = script::Code {
        files,
        lua: vec![("seeker.lua".into(), LUA.into())],
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

const ENEMY: u64 = 501;
const HUNTER: u64 = 1;
const BAT: u64 = 2;
const SLUG: u64 = 3;

fn enemy() -> TagId {
    TagId(reflect::Id::from_raw(ENEMY).unwrap())
}

/// A hunter at the origin with its components in each language, a bat 1
/// m across and 50 m up, and a slug 3 m away on the ground, both enemies;
/// in a project of `kind`.
fn hunt(kind: crate::launcher::ProjectKind) -> Workbench {
    let mut w = Workbench::new(info());
    unhurried(&mut w);
    w.open.project = Some(crate::library::ProjectInfo {
        name: "Hunt".into(),
        root: std::env::temp_dir(),
        assets: std::env::temp_dir(),
    });
    w.open.kind = kind;
    let mut tags = level::tags::TagList::default();
    tags.add_with_id(enemy(), "Enemy").unwrap();
    tags.add("Flying").unwrap();
    w.set_tags(tags);
    w.set_rust_components(vec![seeker_rust()]);
    w.set_program(Some(program()));
    let at = |x: f32, z: f32| Transform {
        position: Vec3::new(x, 0.0, z),
        ..Transform::default()
    };
    let mut hunter = EntityData::new("Hunter").with(&at(0.0, 0.0));
    for name in ["Hunter", "SeekerLua", "SeekerRust"] {
        hunter
            .components
            .insert(name.into(), ComponentData::default());
    }
    let bat = EntityData::new("Bat")
        .with(&at(1.0, 50.0))
        .with(&Tags::of([enemy()]));
    let slug = EntityData::new("Slug")
        .with(&at(3.0, 0.0))
        .with(&Tags::of([enemy()]));
    let spawn = |n: u64, data: EntityData| level::Edit::Spawn {
        entities: vec![(id(n), data)],
    };
    w.open
        .document
        .edit(
            "Add",
            vec![spawn(HUNTER, hunter), spawn(BAT, bat), spawn(SLUG, slug)],
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

/// What game code said, in order.
fn heard(w: &Workbench) -> Vec<String> {
    w.log_lines()
        .iter()
        .map(|l| l.text.clone())
        .filter(|t| t.starts_with("Lust: ") || t.starts_with("Lua: ") || t.starts_with("Rust: "))
        .collect()
}

#[test]
fn lust_lua_and_rust_ask_the_same_world_and_tags_land_at_the_tick_s_end() {
    let mut w = hunt(crate::launcher::ProjectKind::Both);
    w.play();
    ticks(&mut w, 4);
    assert_eq!(
        heard(&w),
        [
            // In 3D the bat is 50 m up: the slug is nearest, the only one
            // within 4 m.
            "Lust: tick 1 nearest Slug, 1 near, enemy false",
            "Lua: 1 near",
            "Rust: nearest Slug",
            "Lust: tick 2 nearest Slug, 1 near, enemy false",
            "Lua: 1 near",
            "Rust: nearest Slug",
            // The tag added in tick 2 is the hunter's from tick 3: it's
            // nearest itself now.
            "Lust: tick 3 nearest Hunter, 2 near, enemy true",
            "Lua: 2 near",
            "Rust: nearest Hunter",
            // Tick 3 took the tag off the nearest, the hunter itself.
            "Lust: tick 4 nearest Slug, 1 near, enemy false",
            "Lua: 1 near",
            "Rust: nearest Slug",
        ]
    );
    let world = w.world().unwrap();
    assert!(world.tags_of(id(HUNTER)).ids.is_empty());
    assert_eq!(world.tags_of(id(SLUG)).ids, [enemy()]);
}

#[test]
fn a_2d_project_measures_across_the_ground() {
    let mut w = hunt(crate::launcher::ProjectKind::TwoD);
    w.play();
    ticks(&mut w, 1);
    assert_eq!(
        heard(&w)[0],
        "Lust: tick 1 nearest Bat, 2 near, enemy false",
        "the bat is 1 m away across the ground"
    );
}

#[test]
fn a_save_keeps_the_tags_game_code_changed() {
    let mut w = hunt(crate::launcher::ProjectKind::Both);
    w.play();
    ticks(&mut w, 2);
    let world = w.world().unwrap();
    assert_eq!(world.tags_of(id(HUNTER)).ids, [enemy()]);
    let text = w.save_of_game().unwrap().write().unwrap();
    let sum = w.world().unwrap().checksum();
    ticks(&mut w, 2);
    w.load_game(&text, "quick");
    assert_eq!(w.world().unwrap().tags_of(id(HUNTER)).ids, [enemy()]);
    assert_eq!(w.world().unwrap().checksum(), sum, "as saved");
    // And Stop puts the level's tags back: the game's were the game's.
    w.stop();
    let hunter = w.open.document.level().get(id(HUNTER)).unwrap();
    assert!(hunter.get::<Tags>().is_none());
}

#[test]
fn the_same_game_on_one_core_or_four_comes_out_the_same() {
    let run = |threads: usize| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(|| {
                let mut w = hunt(crate::launcher::ProjectKind::Both);
                w.play();
                ticks(&mut w, 6);
                // Not the checksum: each workbench's starting level has
                // entities of its own random IDs.
                let world = w.world().unwrap();
                (heard(&w), world.all_tags())
            })
    };
    assert_eq!(run(1), run(4));
}

/// Each unit asks for the nearest enemy and those within 3 m, every tick.
const ASKER: &str = "fetch \"solis/world\":World;\nfetch \"solis/project\":Tag;\n\n\
    component Asker {\n\
        has {\n\
            int seen = 0;\n\
        }\n\
        method update(float dt) {\n\
            entity closest = World:nearest(Tag.Enemy, position());\n\
            entity array near = World:within(Tag.Enemy, position(), 3);\n\
            is closest != nothing? then {\n\
                seen = seen + #near;\n\
            }\n\
        }\n\
    }\n\
    pass Asker;\nend\n";

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
fn scale_check_2000_lust_units_asking_by_tag_each_tick() {
    // Stage 1's scale case, end to end: 2,000 units in a 200 m square,
    // half of them enemies, each with a Lust component asking for the
    // nearest enemy and every enemy within 3 m, every tick. A frame of
    // play mode runs one tick of all of them.
    const TARGET_MS: f64 = 16.0;
    let (mut files, _) = script::describe::descriptions_for(
        "Test Engine 9.9.9",
        "solis",
        &script::project_names::Names::standard().and_tags(&["Enemy"]),
    );
    files.push(("main.lst".into(), ASKER.to_string()));
    let code = script::Code {
        files,
        lua: Vec::new(),
        notes: Vec::new(),
    };
    let program = Arc::new(script::compile(&code).unwrap().unwrap());
    let mut w = Workbench::new(info());
    let mut tags = level::tags::TagList::default();
    tags.add_with_id(enemy(), "Enemy").unwrap();
    w.set_tags(tags);
    w.set_program(Some(program));
    let spawns: Vec<level::Edit> = (0..2_000)
        .map(|i| {
            let t = i as f32 * 0.618_034;
            let mut data = EntityData::new(&format!("Unit {i}")).with(&Transform {
                position: Vec3::new((t * 97.0) % 200.0, (t * 61.0) % 200.0, 0.0),
                ..Transform::default()
            });
            if i % 2 == 0 {
                data.set(&Tags::of([enemy()]));
            }
            data.components
                .insert("Asker".into(), ComponentData::default());
            level::Edit::Spawn {
                entities: vec![(EntityId::random(), data)],
            }
        })
        .collect();
    w.open.document.edit("Fill", spawns, None, None).unwrap();
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
        "Scale check: a play-mode tick of 2,000 Lust units each asking for the nearest enemy and \
         those within 3 m {ms:.2} ms (target {TARGET_MS} ms, one frame) {verdict}"
    );
    assert!(ms <= TARGET_MS, "a tick takes {ms:.2} ms");
}
