//! Reading another entity's component (Fixes 2a): `other:knight()` gives
//! a copy of the knight as the tick started, `World:the(Component.Quest)`
//! the one entity with a quest, and `exists()` whether an entity is still
//! in the game; the same on one core or four.

use super::tests::{TestWorld, id, unhurried};
use super::*;
use crate::code::Code;

/// A knight who picks up a key on its third tick, a gate that reads it
/// each tick, and a quest the gate finds with `World:the`.
const GAME: &str = "fetch \"solis/world\":World;\n\
    fetch \"solis/components\":[Component, KnightReading];\n\n\
    state Mood = [Calm, Angry];\n\n\
    component Knight {\n\
        has {\n\
            bool hasKey = false;\n\
            int coins = 3;\n\
            Mood mood = Mood.Calm;\n\
            int ticks = 0;\n\
        }\n\
        method update(float dt) {\n\
            ticks = ticks + 1;\n\
            is ticks == 3? then {\n\
                hasKey = true;\n\
                coins = 7;\n\
                mood = Mood.Angry;\n\
            }\n\
        }\n\
    }\n\n\
    component Quest {\n\
        has {\n\
            entity hero = nothing;\n\
        }\n\
    }\n\n\
    component Gate {\n\
        has {\n\
            entity knight = nothing;\n\
        }\n\
        method update(float dt) {\n\
            entity quest = World:the(Component.Quest);\n\
            entity hero = quest:quest().hero;\n\
            KnightReading seen = hero:knight();\n\
            World:say(\"gate \" + seen.hasKey.text() + \" \" + seen.coins.text() + \" \" + \
                seen.mood.text() + \" \" + hero:exists().text());\n\
            KnightReading mine = this:knight();\n\
            is mine == nothing? then {\n\
                World:say(\"no knight here\");\n\
            }\n\
        }\n\
    }\n\
    end\n";

fn compile(main: &str) -> Result<Arc<Compiled>, Vec<crate::CodeProblem>> {
    let (mut files, _) = crate::describe::descriptions_for(
        "Test 1.0",
        "solis",
        &crate::project_names::Names::default(),
    );
    files.push(("main.lst".into(), main.to_string()));
    let code = Code {
        files,
        lua: Vec::new(),
        notes: Vec::new(),
    };
    crate::code::compile_within(&code, crate::luau::Limits::unhurried())
        .map(|c| Arc::new(c.expect("there's code")))
}

/// The knight on A, the gate on B and the quest, naming the knight as
/// its hero, on C.
fn started(shards: usize) -> (Game, TestWorld) {
    let mut world = TestWorld::with(&["A", "B", "C"]);
    let mut game = unhurried(shards);
    let code = compile(GAME).unwrap_or_else(|p| panic!("{p:#?}"));
    assert_eq!(code.components.reads, ["Knight", "Quest"]);
    game.load(&code, &mut world).unwrap();
    let empty = ComponentData::default();
    let mut quest = ComponentData::default();
    quest.set("hero", reflect::Value::Entity(Some(id(1))));
    let components = vec![
        (id(1), "Knight", &empty),
        (id(2), "Gate", &empty),
        (id(3), "Quest", &quest),
    ];
    assert!(game.sync_many(components, &mut world).is_empty());
    (game, world)
}

fn run(game: &mut Game, world: &mut TestWorld, ticks: u32) -> Vec<String> {
    let mut said = Vec::new();
    for tick in 1..=ticks {
        world.set_clock(u64::from(tick), f64::from(tick) / 60.0);
        let ticked = game.tick(1.0 / 60.0, world);
        assert!(ticked.failures.is_empty(), "{:?}", ticked.failures);
        said.extend(world.said.drain(..).map(|line| format!("{tick}: {line}")));
    }
    said
}

#[test]
fn a_gate_reads_the_knight_as_the_tick_started() {
    let (mut game, mut world) = started(1);
    let said = run(&mut game, &mut world, 5);
    // The knight takes the key in its third tick; the gate sees it from
    // the fourth, whichever of them runs first.
    let gate: Vec<&String> = said.iter().filter(|l| l.contains("gate")).collect();
    assert_eq!(gate.len(), 5, "{said:?}");
    // The gate has no knight of its own: its reading is nothing.
    assert_eq!(
        said.iter()
            .filter(|l| l.ends_with("no knight here"))
            .count(),
        5
    );
    assert!(gate[2].ends_with("false 3 Calm true"), "{said:?}");
    assert!(gate[3].ends_with("true 7 Angry true"), "{said:?}");
}

#[test]
fn the_same_on_four_cores() {
    let (mut one, mut world) = started(1);
    let on_one = run(&mut one, &mut world, 6);
    let (mut four, mut world) = started(4);
    assert_eq!(run(&mut four, &mut world, 6), on_one);
}

#[test]
fn a_game_that_reads_nothing_copies_nothing() {
    let code = compile(
        "fetch \"solis/world\":World;\n\
         component Plain {\n    has {\n        int n = 0;\n    }\n}\nend\n",
    )
    .unwrap();
    assert!(code.components.reads.is_empty());
    assert_eq!(code.components.readable, [("Plain".into(), "plain".into())]);
}

#[test]
fn writing_to_a_reading_is_a_mistake_before_the_game_runs() {
    let wrong = GAME.replace(
        "KnightReading seen = hero:knight();",
        "KnightReading seen = hero:knight();\n            seen.coins = 1;",
    );
    assert!(compile(&wrong).is_err());
}

#[test]
fn the_wants_exactly_one() {
    let mut world = TestWorld::with(&["A", "B", "C"]);
    let mut game = unhurried(1);
    let code = compile(GAME).unwrap();
    game.load(&code, &mut world).unwrap();
    let empty = ComponentData::default();
    // Two quests: World:the stops the gate with a message saying so.
    let components = vec![
        (id(1), "Quest", &empty),
        (id(2), "Gate", &empty),
        (id(3), "Quest", &empty),
    ];
    assert!(game.sync_many(components, &mut world).is_empty());
    world.set_clock(1, 1.0 / 60.0);
    let ticked = game.tick(1.0 / 60.0, &mut world);
    let failed = format!("{:?}", ticked.failures);
    assert!(failed.contains("there are 2"), "{failed}");
}

/// A chest that spawns a key once, giving its Pickup the red key and how
/// many, and a key whose first update says what it was given.
const SPAWNING: &str = "fetch \"solis/world\":World;\n\
    fetch \"solis/project\":Module;\n\
    fetch \"solis/components\":PickupValues;\n\n\
    state Item = [Blue, Red];\n\n\
    component Pickup {\n\
        has {\n\
            Item item = Item.Blue;\n\
            int count = 1;\n\
            bool told = false;\n\
        }\n\
        method update(float dt) {\n\
            is told == false? then {\n\
                World:say(\"key \" + item.text() + \" \" + count.text());\n\
                told = true;\n\
            }\n\
        }\n\
    }\n\n\
    component Chest {\n\
        has {\n\
            bool opened = false;\n\
        }\n\
        method update(float dt) {\n\
            is opened == false? then {\n\
                PickupValues given = new PickupValues { item = Item.Red };\n\
                World:say(\"unwritten \" + given.count.text());\n\
                given.count = 2;\n\
                entity key = World:spawnWith(Module.Key, [0, 0, 0], [given]);\n\
                opened = true;\n\
            }\n\
        }\n\
    }\n\
    end\n";

fn compile_spawning() -> Arc<Compiled> {
    let (mut files, _) = crate::describe::descriptions_for(
        "Test 1.0",
        "solis",
        &crate::project_names::Names::with_modules(&["key"]),
    );
    files.push(("main.lst".into(), SPAWNING.to_string()));
    let code = Code {
        files,
        lua: Vec::new(),
        notes: Vec::new(),
    };
    crate::code::compile_within(&code, crate::luau::Limits::unhurried())
        .unwrap_or_else(|p| panic!("{p:#?}"))
        .map(Arc::new)
        .expect("there's code")
}

#[test]
fn a_spawn_carries_its_values_to_the_world() {
    let mut world = TestWorld::with(&["A"]);
    let mut game = unhurried(1);
    let code = compile_spawning();
    assert_eq!(
        code.components.readable.len(),
        2,
        "{:?}",
        code.components.notes
    );
    game.load(&code, &mut world).unwrap();
    let empty = ComponentData::default();
    assert!(
        game.sync_many(vec![(id(1), "Chest", &empty)], &mut world)
            .is_empty()
    );
    let said = run(&mut game, &mut world, 2);
    // A field not written yet reads as its starting value.
    assert_eq!(said, ["1: unwritten 1"]);
    let spawns: Vec<&Change> = world
        .state_changes
        .iter()
        .filter(|c| matches!(c, Change::Spawn(..)))
        .collect();
    assert_eq!(spawns.len(), 1, "{spawns:?}");
    let Change::Spawn(_, module, _, values) = spawns[0] else {
        unreachable!()
    };
    assert_eq!(module, "key");
    let mut pickup = ComponentData::default();
    pickup.set("item", reflect::Value::Text("Red".into()));
    pickup.set("count", reflect::Value::Int(2));
    assert_eq!(values, &vec![("Pickup".to_string(), pickup)]);
}

#[test]
fn a_component_started_with_the_values_sees_them_in_its_first_update() {
    // What the editor does with a landed spawn: the Pickup starts with
    // the values in its data.
    let mut world = TestWorld::with(&["A", "B"]);
    let mut game = unhurried(1);
    game.load(&compile_spawning(), &mut world).unwrap();
    let mut given = ComponentData::default();
    given.set("item", reflect::Value::Text("Red".into()));
    given.set("count", reflect::Value::Int(2));
    assert!(
        game.sync_many(vec![(id(2), "Pickup", &given)], &mut world)
            .is_empty()
    );
    let said = run(&mut game, &mut world, 1);
    assert_eq!(said, ["1: key Red 2"]);
}

#[test]
fn values_for_a_field_of_the_wrong_kind_are_a_mistake_before_the_game_runs() {
    let wrong = SPAWNING.replace("given.count = 2;", "given.count = \"two\";");
    let (mut files, _) = crate::describe::descriptions_for(
        "Test 1.0",
        "solis",
        &crate::project_names::Names::with_modules(&["key"]),
    );
    files.push(("main.lst".into(), wrong));
    let code = Code {
        files,
        lua: Vec::new(),
        notes: Vec::new(),
    };
    assert!(crate::code::compile_within(&code, crate::luau::Limits::unhurried()).is_err());
}
