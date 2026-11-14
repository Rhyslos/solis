//! What Lust 2.0.0-alpha.67 brought, in a running game (decision 0106):
//! tweens moved once a tick, after the game-wide objects and before the
//! components' updates; `vec2` and `colour` fields as Solis's own kinds,
//! kept in saves and the checksum, the same on any number of shards.

use super::tests::{TestWorld, compiled, id, unhurried};
use super::*;
use reflect::{FieldKind, Value};

/// Doors that glide open once, saying so, and note what each update saw.
const DOORS: &str = "fetch \"solis/world\":World;\n\n\
    component Door {\n\
        has {\n\
            float open = 0;\n\
            float seen = 0;\n\
            int opened = 0;\n\
            bool started = false;\n\
        }\n\
        method update(float dt) {\n\
            seen = open;\n\
            is not started? then {\n\
                started = true;\n\
                tween open to 1 over 0.5 [ease OutCubic] {\n\
                    opened += 1;\n\
                    World:say(\"open\");\n\
                }\n\
            }\n\
        }\n\
    }\n\
    pass Door;\nend\n";

/// A game on `shards` shards with a `Door` on each of `doors` entities.
fn doors(shards: usize, doors: u64) -> (Game, TestWorld) {
    let names: Vec<String> = (1..=doors).map(|n| format!("D{n}")).collect();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    let mut world = TestWorld::with(&names);
    let mut game = unhurried(shards);
    game.load(&compiled(DOORS), &mut world).unwrap();
    let empty = ComponentData::default();
    let all = (1..=doors).map(|n| (id(n), "Door", &empty));
    assert!(game.sync_many(all, &mut world).is_empty());
    (game, world)
}

/// Runs `ticks` ticks of a 60th of a second, with no failures.
fn run(game: &mut Game, world: &mut TestWorld, ticks: usize) {
    for _ in 0..ticks {
        let ticked = game.tick(1.0 / 60.0, world);
        assert!(ticked.failures.is_empty(), "{:?}", ticked.failures);
        assert_eq!(ticked.pause, None);
    }
}

fn float(game: &mut Game, n: u64, field: &str) -> f64 {
    match game.values(id(n), "Door").unwrap().get(field) {
        Some(Value::Float(x)) => *x,
        other => panic!("{field} is {other:?}"),
    }
}

#[test]
fn a_tween_moves_before_the_updates_and_its_done_block_runs_once() {
    let (mut game, mut world) = doors(1, 1);
    // The first tick's update starts the tween; it moves from the next.
    run(&mut game, &mut world, 1);
    assert_eq!(
        float(&mut game, 1, "open"),
        0.0,
        "a tween started in an update waits"
    );
    run(&mut game, &mut world, 1);
    let first = float(&mut game, 1, "open");
    assert!(first > 0.0 && first < 1.0, "{first}");
    assert_eq!(
        float(&mut game, 1, "seen"),
        first,
        "the update sees this tick's value"
    );
    // Half a second is 30 steps: the 31st tick ends it.
    run(&mut game, &mut world, 28);
    assert!(float(&mut game, 1, "open") < 1.0);
    assert!(world.said.is_empty());
    run(&mut game, &mut world, 1);
    assert_eq!(
        float(&mut game, 1, "open"),
        1.0,
        "set exactly where it's going"
    );
    assert_eq!(
        float(&mut game, 1, "seen"),
        1.0,
        "the done block's changes are there for the same tick's update"
    );
    let values = game.values(id(1), "Door").unwrap();
    assert_eq!(values.get("opened"), Some(&Value::Int(1)));
    assert_eq!(world.said, ["open"]);
    run(&mut game, &mut world, 10);
    assert_eq!(world.said, ["open"], "once");
}

#[test]
fn tweens_give_the_same_game_on_any_number_of_shards() {
    let sums: Vec<u64> = [1, 3]
        .into_iter()
        .map(|shards| {
            let (mut game, mut world) = doors(shards, 7);
            let mut sums = Vec::new();
            for _ in 0..40 {
                run(&mut game, &mut world, 1);
                sums.push(game.checksum());
            }
            assert_eq!(world.said.len(), 7, "every door opened once");
            sums.iter().fold(0, |a, s| a ^ s.rotate_left(7))
        })
        .collect();
    assert_eq!(sums[0], sums[1]);
}

#[test]
fn new_code_stops_the_old_code_s_tweens_where_they_are() {
    let (mut game, mut world) = doors(1, 1);
    run(&mut game, &mut world, 10);
    let before = float(&mut game, 1, "open");
    assert!(before > 0.0 && before < 1.0);
    game.swap(&compiled(DOORS), &mut world).unwrap();
    run(&mut game, &mut world, 60);
    assert_eq!(float(&mut game, 1, "open"), before);
    assert!(world.said.is_empty(), "its done block never runs");
}

#[test]
fn a_tween_can_t_move_an_object_and_says_so_once() {
    let code = "object Sky {\n\
            has {\n\
                float light = 0;\n\
                bool started = false;\n\
            }\n\
            method update(float dt) {\n\
                is not started? then {\n\
                    started = true;\n\
                    tween light to 1 over 1;\n\
                }\n\
            }\n\
        }\n\
        component Door {\n\
            has {\n\
                float open = 0;\n\
                bool started = false;\n\
            }\n\
            method update(float dt) {\n\
                is not started? then {\n\
                    started = true;\n\
                    tween open to 1 over 0.5;\n\
                }\n\
            }\n\
        }\n\
        pass Sky, Door;\nend\n";
    let mut world = TestWorld::with(&["D1"]);
    let mut game = unhurried(1);
    game.load(&compiled(code), &mut world).unwrap();
    let empty = ComponentData::default();
    assert!(
        game.sync_many([(id(1), "Door", &empty)], &mut world)
            .is_empty()
    );
    let mut failures = Vec::new();
    for _ in 0..100 {
        failures.extend(game.tick(1.0 / 60.0, &mut world).failures);
    }
    assert_eq!(failures.len(), 1, "said once: {failures:?}");
    let said = failures[0].to_string();
    assert!(
        said.contains("Sky.light can't be moved by a tween"),
        "{said}"
    );
    assert!(
        !failures[0].stopped,
        "nothing stops: Lust doesn't say whose it was"
    );
    let sky = &game.object_values()[0].1;
    assert_eq!(sky.get("light"), Some(&Value::Float(0.0)));
    // The door's tween, started after the sky's, waited while the sky's
    // failed, and moved once the sky's had run its time.
    assert_eq!(float(&mut game, 1, "open"), 1.0);
}

/// A sprite with a vec2 and two colours, one see-through, that it changes.
const SPRITE: &str = "event Flash(colour glow, vec2 at);\n\
    component Sprite {\n\
        has {\n\
            vec2 speed = [2, -1];\n\
            vec2 place = [0, 0];\n\
            colour tint = [1, 0.5, 0];\n\
            colour shadow = [0, 0, 0, 0.5];\n\
            int flashes = 0;\n\
        }\n\
        method update(float dt) {\n\
            place = place + speed;\n\
            shadow = shadow:fade(shadow.a * 0.5);\n\
            send Flash(tint, place) to me;\n\
        }\n\
        on Flash(colour glow, vec2 at) {\n\
            is glow == tint? then {\n\
                flashes += 1;\n\
            }\n\
        }\n\
    }\n\
    pass Sprite;\nend\n";

#[test]
fn vec2_and_colour_fields_are_solis_s_own_kinds_and_cross_both_ways() {
    let mut world = TestWorld::with(&["S1"]);
    let mut game = unhurried(1);
    let code = compiled(SPRITE);
    let schema = &code.schemas[0];
    let field = |n: &str| schema.field(n).unwrap();
    assert_eq!(field("speed").kind, FieldKind::Vec2);
    assert_eq!(field("speed").default, Value::floats(&[2.0, -1.0]));
    assert_eq!(field("tint").kind, FieldKind::Color);
    assert_eq!(
        field("tint").default,
        Value::floats(&[1.0, 0.5, 0.0]),
        "a solid colour is three numbers, as Solis's always were"
    );
    assert_eq!(
        field("shadow").default,
        Value::floats(&[0.0, 0.0, 0.0, 0.5])
    );
    game.load(&code, &mut world).unwrap();
    // The area's values reach game code, a see-through colour's too.
    let mut data = ComponentData::default();
    data.set("speed", Value::floats(&[0.5, 0.25]));
    data.set("shadow", Value::floats(&[0.0, 0.0, 1.0, 0.8]));
    assert!(
        game.sync_many([(id(1), "Sprite", &data)], &mut world)
            .is_empty()
    );
    run_sprite(&mut game, &mut world, 2);
    let values = game.values(id(1), "Sprite").unwrap();
    assert_eq!(values.get("place"), Some(&Value::floats(&[1.0, 0.5])));
    let shadow = values.get("shadow").unwrap().as_colour().unwrap();
    assert!((shadow[3] - 0.2).abs() < 1e-6, "{shadow:?}");
    assert_eq!(&shadow[..3], &[0.0, 0.0, 1.0]);
    // The events carried a colour and a vec2, declared, and arrived.
    assert_eq!(values.get("flashes"), Some(&Value::Int(2)));
}

fn run_sprite(game: &mut Game, world: &mut TestWorld, ticks: usize) {
    for _ in 0..ticks {
        let ticked = game.tick(1.0 / 60.0, world);
        assert!(ticked.failures.is_empty(), "{:?}", ticked.failures);
    }
}

#[test]
fn a_saved_vec2_and_see_through_colour_load_back_exactly() {
    let run_for = |ticks: usize, from: Option<&ComponentData>| {
        let mut world = TestWorld::with(&["S1"]);
        let mut game = unhurried(2);
        game.load(&compiled(SPRITE), &mut world).unwrap();
        let start = from.cloned().unwrap_or_default();
        assert!(
            game.sync_many([(id(1), "Sprite", &start)], &mut world)
                .is_empty()
        );
        run_sprite(&mut game, &mut world, ticks);
        (game.values(id(1), "Sprite").unwrap(), game.checksum())
    };
    // A save after 3 ticks, loaded and run 2 more, is the game run 5.
    let (saved, _) = run_for(3, None);
    let (loaded, loaded_sum) = run_for(2, Some(&saved));
    let (straight, straight_sum) = run_for(5, None);
    assert_eq!(loaded.get("place"), straight.get("place"));
    assert_eq!(loaded.get("shadow"), straight.get("shadow"));
    assert_eq!(loaded_sum, straight_sum);
}

#[test]
fn the_manual_s_door_glides_open_as_it_says() {
    let page = include_str!("../../../docs/manual/game-code.md").replace("\r\n", "\n");
    let section = page
        .split("\n## Tweens\n")
        .nth(1)
        .expect("the Tweens section");
    let lust = section
        .split("```lust\n")
        .nth(1)
        .and_then(|example| example.split("```\n").next())
        .expect("its Lust example")
        .to_string();
    let mut world = TestWorld::with(&["D1"]);
    let mut game = unhurried(1);
    game.load(&compiled(&lust), &mut world).unwrap();
    let empty = ComponentData::default();
    assert!(
        game.sync_many([(id(1), "Door", &empty)], &mut world)
            .is_empty()
    );
    let opened = crate::Event {
        to: id(1),
        name: "Opened".into(),
        values: Vec::new(),
    };
    let ticked = game.tick_with_events(1.0 / 60.0, &mut world, vec![opened]);
    assert!(ticked.failures.is_empty(), "{:?}", ticked.failures);
    run(&mut game, &mut world, 29);
    assert!(float(&mut game, 1, "open") < 1.0);
    run(&mut game, &mut world, 1);
    assert_eq!(float(&mut game, 1, "open"), 1.0);
    assert_eq!(world.said, ["the door is open"]);
}
