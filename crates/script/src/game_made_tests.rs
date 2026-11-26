//! What game code makes with `new` (the uniformity pass, U6a): sounds,
//! timers and shots, each a class Solis writes into `solis/made.lst`
//! with a native `made()`, landing as the calls they replace did, the
//! same on one core or four.

use super::tests::{TestWorld, id, unhurried};
use super::*;
use crate::code::Code;
use crate::host::SoundAt;

/// A bell that rings on its first tick: a horn at a place, a footstep on
/// itself, a chime on the gong (entity B, `gong`), a door timer, and a shot;
/// and that hears its timer finish.
const BELL: &str = "fetch \"solis/world\":World;\n\
    fetch \"solis/timers\":TimerFinished;\n\
    fetch \"solis/project\":Timer;\n\
    fetch \"solis/made\":[Horn, Footstep, Chime, DoorOpens, Shot];\n\
    \n\
    component Bell {\n\
    has {\n\
    int ticks = 0;\n\
    entity gong = nothing;\n\
    }\n\
    method update(float dt) {\n\
    ticks += 1;\n\
    is ticks == 1? then {\n\
    new Horn { at = [1, 2, 3] };\n\
    new Footstep {};\n\
    new Chime { on = gong };\n\
    new DoorOpens { after = 2.0 / 60.0 };\n\
    new Shot { from = [0, 1, 0], direction = [0, 0, 1], speed = 30 };\n\
    }\n\
    }\n\
    on {\n\
    \x20   TimerFinished(Timer which) {\n\
    \x20   is which == Timer.DoorOpens? then {\n\
    \x20   World:say(\"opens on \" + ticks.text());\n\
    \x20   }\n\
    \x20   }\n\
    }\n\
    }\n\
    pass Bell;\n\
    end\n\
    ";

fn compile(main: &str) -> Arc<Compiled> {
    let names = crate::project_names::Names::default()
        .and_sounds(&["sfx/horn", "sfx/footstep", "sfx/chime"])
        .and_timers(&["DoorOpens"]);
    let (mut files, _) = crate::describe::descriptions_for("Test 1.0", "solis", &names);
    files.push(("main.lst".into(), main.to_string()));
    let code = Code {
        files,
        lua: Vec::new(),
        notes: Vec::new(),
    };
    match crate::code::compile_within(&code, crate::luau::Limits::unhurried()) {
        Ok(Some(c)) => Arc::new(c),
        Ok(None) => panic!("no code"),
        Err(problems) => panic!("{problems:#?}"),
    }
}

/// The bell on A (its gong B), ticked four times on `shards` shards: what
/// it said, the sounds and the shots.
fn rung(shards: usize) -> (Vec<String>, Vec<Change>, usize) {
    let mut world = TestWorld::with(&["A", "B"]);
    let mut game = unhurried(shards);
    game.load(&compile(BELL), &mut world).unwrap();
    let mut bell = ComponentData::default();
    bell.set("gong", reflect::Value::Entity(Some(id(2))));
    assert!(
        game.sync_many(vec![(id(1), "Bell", &bell)], &mut world)
            .is_empty()
    );
    for tick in 1u32..=4 {
        world.set_clock(u64::from(tick), f64::from(tick) / 60.0);
        let ticked = game.tick(1.0 / 60.0, &mut world);
        assert!(ticked.failures.is_empty(), "{:?}", ticked.failures);
    }
    (world.said.clone(), world.sounds.clone(), world.shots.len())
}

#[test]
fn sounds_timers_and_shots_are_made_with_new() {
    let (said, sounds, shots) = rung(1);
    let played: Vec<(String, SoundAt)> = sounds
        .iter()
        .filter_map(|c| match c {
            Change::Sound(event, at, ..) => Some((event.clone(), *at)),
            _ => None,
        })
        .collect();
    assert_eq!(
        played,
        [
            (
                "sfx/horn".to_string(),
                SoundAt::Place(glam::Vec3::new(1.0, 2.0, 3.0))
            ),
            ("sfx/footstep".to_string(), SoundAt::On(id(1))),
            ("sfx/chime".to_string(), SoundAt::On(id(2))),
        ],
        "at a place, on the entity whose code made it, and on another"
    );
    assert_eq!(shots, 1);
    assert_eq!(said, ["opens on 3"], "the timer finishes two ticks on");
}

#[test]
fn made_things_land_the_same_on_four_cores() {
    assert_eq!(rung(4), rung(1));
}

#[test]
fn a_timer_takes_one_timing_and_belongs_to_an_entity() {
    let main = "fetch \"solis/made\":DoorOpens;\n\
        component Door {\n    method update(float dt) {\n        \
        new DoorOpens { after = 1, every = 2 };\n    }\n}\npass Door;\nend\n";
    let mut world = TestWorld::with(&["A"]);
    let mut game = unhurried(1);
    game.load(&compile(main), &mut world).unwrap();
    assert!(
        game.sync_many(vec![(id(1), "Door", &ComponentData::default())], &mut world)
            .is_empty()
    );
    let ticked = game.tick(1.0 / 60.0, &mut world);
    let failed = format!("{:?}", ticked.failures);
    assert!(
        failed.contains("takes one of after, every and atHour"),
        "{failed}"
    );
}

#[test]
fn the_old_calls_say_how_they_re_made_now() {
    let main = "fetch \"solis/project\":[Sound, Timer];\nfetch \"solis/sound\":Audio;\n\
        component Door {\n    method update(float dt) {\n        \
        Audio:playAt(Sound.Horn, [0, 0, 0]);\n        startTimer(Timer.DoorOpens, 2.0);\n    }\n}\n\
        pass Door;\nend\n";
    let names = crate::project_names::Names::default()
        .and_sounds(&["sfx/horn"])
        .and_timers(&["DoorOpens"]);
    let (mut files, _) = crate::describe::descriptions_for("Test 1.0", "solis", &names);
    files.push(("main.lst".into(), main.to_string()));
    let problems = crate::code::compile(&Code {
        files,
        lua: Vec::new(),
        notes: Vec::new(),
    })
    .expect_err("the old calls aren't Lust's any more");
    let said: Vec<&str> = problems.iter().map(|p| p.message.as_str()).collect();
    assert_eq!(said.len(), 2, "{said:#?}");
    assert!(
        said[0].starts_with(
            "Audio:playAt(…) is made with new now: write new Footstep { at = place };"
        ),
        "{}",
        said[0]
    );
    assert!(
        said[1].contains("new DoorOpens { after = 2.0 };"),
        "{}",
        said[1]
    );
}

#[test]
fn timers_are_named_where_new_makes_them() {
    let files = vec![
        (
            "solis/project.lst".to_string(),
            "state Sound = [Horn];\n-- @Sound.Horn = sfx/horn\nend\n".to_string(),
        ),
        (
            "main.lst".to_string(),
            "class Crate {\n    has {\n        float after = 0;\n    }\n}\n\
             component Door {\n    method update(float dt) {\n        \
             new DoorOpens { after = 2.0 };\n        new Wave { every = [1, 2][1] };\n        \
             new Dawn { atHour = 6 };\n        new Horn { at = [0, 0, 0] };\n        \
             Crate c = new Crate { after = 1 };\n        new Gate { open = true };\n        \
             -- new Ghost { after = 1 };\n        World:say(\"new Text { after = 1 }\");\n    }\n}\nend\n"
                .to_string(),
        ),
    ];
    assert_eq!(
        crate::timer_names::names_in(&files),
        ["Dawn", "DoorOpens", "Wave"],
        "not a sound, nor the game's own class, nor one without a timing, nor in a comment or text"
    );
}

/// Two modules: a ship whose root has a Ship and a Health (both with
/// `hp`), and a component of the game's own called Crate beside a module
/// called crate.
fn compile_modules(main: &str) -> Result<Arc<Compiled>, Vec<crate::CodeProblem>> {
    let names = crate::project_names::Names::with_modules(&["ships/fighter", "crate"])
        .and_module_roots(&[
            ("ships/fighter", &["Ship", "Health"]),
            ("crate", &["Crate"]),
        ]);
    let (mut files, _) = crate::describe::descriptions_for("Test 1.0", "solis", &names);
    files.push(("main.lst".into(), main.to_string()));
    crate::code::compile_within(
        &Code {
            files,
            lua: Vec::new(),
            notes: Vec::new(),
        },
        crate::luau::Limits::unhurried(),
    )
    .map(|c| Arc::new(c.expect("there's code")))
}

const HANGAR: &str = "fetch \"solis/world\":World;\n\
    fetch \"solis/made\":[Fighter, CrateModule];\n\n\
    component Ship {\n    has {\n        float speed = 9;\n        int hp = 3;\n    }\n}\n\
    component Health {\n    has {\n        int hp = 10;\n    }\n}\n\
    component Crate {\n    has {\n        bool open = false;\n    }\n}\n\
    component Hangar {\n\
        method update(float dt) {\n\
            entity ship = new Fighter { position = [1, 2, 0], speed = 12, healthHp = 5 };\n\
            entity box = new CrateModule { open = true };\n\
            World:say((ship != nothing).text() + \" \" + (box != nothing).text());\n\
        }\n\
    }\n\
    pass Ship, Health, Crate, Hangar;\nend\n";

#[test]
fn modules_are_made_with_new_their_root_s_fields_given() {
    let code = compile_modules(HANGAR).unwrap_or_else(|p| panic!("{p:#?}"));
    let mut world = TestWorld::with(&["A"]);
    let mut game = unhurried(1);
    game.load(&code, &mut world).unwrap();
    assert!(
        game.sync_many(
            vec![(id(1), "Hangar", &ComponentData::default())],
            &mut world
        )
        .is_empty()
    );
    world.set_clock(1, 1.0 / 60.0);
    let ticked = game.tick(1.0 / 60.0, &mut world);
    assert!(ticked.failures.is_empty(), "{:?}", ticked.failures);
    assert_eq!(world.said, ["true true"]);
    let spawns: Vec<(String, glam::Vec3, crate::SpawnValues)> = world
        .state_changes
        .iter()
        .filter_map(|c| match c {
            Change::Spawn(_, module, place, values) => {
                Some((module.clone(), *place, values.clone()))
            }
            _ => None,
        })
        .collect();
    let mut ship = ComponentData::default();
    ship.set("speed", reflect::Value::Float(12.0));
    let mut health = ComponentData::default();
    health.set("hp", reflect::Value::Int(5));
    let mut open = ComponentData::default();
    open.set("open", reflect::Value::Bool(true));
    assert_eq!(
        spawns,
        [
            (
                "ships/fighter".to_string(),
                glam::Vec3::new(1.0, 2.0, 0.0),
                vec![("Ship".to_string(), ship), ("Health".to_string(), health)]
            ),
            (
                "crate".to_string(),
                glam::Vec3::ZERO,
                vec![("Crate".to_string(), open)]
            ),
        ],
        "only the fields written; a field two components share has its component in front; \
         a module named like a component has Module added; no position is the world's middle"
    );
}

#[test]
fn a_module_s_shared_field_is_only_known_with_its_component_in_front() {
    let wrong = HANGAR.replace("healthHp = 5", "hp = 5");
    let problems = compile_modules(&wrong).expect_err("hp is two fields");
    assert!(
        problems.iter().any(|p| p.message.contains("hp")),
        "{problems:#?}"
    );
}

#[test]
fn a_sound_named_like_one_of_lust_s_own_states_is_made_under_another_name() {
    let names = crate::project_names::Names::default().and_sounds(&["sfx/run", "sfx/day"]);
    let (mut files, _) = crate::describe::descriptions_for("Test 1.0", "solis", &names);
    files.push((
        "main.lst".into(),
        "fetch \"solis/made\":[RunSound, DaySound];\n\
         component Feet {\n    method update(float dt) {\n        new RunSound {};\n        \
         new DaySound {};\n    }\n}\npass Feet;\nend\n"
            .into(),
    ));
    let code = Code {
        files,
        lua: Vec::new(),
        notes: Vec::new(),
    };
    assert!(
        crate::code::check(&code).is_empty(),
        "{:?}",
        crate::code::check(&code)
    );
}
