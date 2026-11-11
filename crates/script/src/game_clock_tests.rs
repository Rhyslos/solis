//! The same game whatever its speed (the review, BUG-47). Game time moves
//! at the game's speed, so it stands still while the speed is 0, and game
//! code still runs then: random numbers and spawned entities are told
//! apart by the tick's number, which always goes up.

use super::tests::{SpawnWorld, TestWorld, compiled, id};
use super::*;
use reflect::Value;
use std::collections::HashSet;

/// Each roller rolls a number and spawns a drone, every tick.
const ROLLERS: &str = "fetch \"solis/world\":World;\nfetch \"solis/project\":Assembly;\n\n\
    component Roller {\n\
        has {\n\
            float rolled = 0;\n\
            entity made = nothing;\n\
        }\n\
        method update(float dt) {\n\
            rolled = World:random();\n\
            made = World:spawn(Assembly.Drone, [0, 0, 0]);\n\
        }\n\
    }\n\
    pass Roller;\nend\n";

/// How many rollers play.
const ROLLERS_PLAYING: u64 = 6;

/// What each roller rolled (the number's exact bits) and spawned, tick by
/// tick, in a game on `shards` shards that runs one tick at each of
/// `speeds`; and every entity the world was asked to make.
fn rolls(shards: usize, speeds: &[f64]) -> (Vec<(u64, EntityId)>, Vec<EntityId>) {
    let names: Vec<String> = (1..=ROLLERS_PLAYING).map(|n| format!("R{n}")).collect();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    let mut world = SpawnWorld(TestWorld::with(&names), Vec::new());
    let mut game = Game::new(shards).unwrap();
    assert_eq!(game.shards(), shards);
    game.load(&compiled(ROLLERS), &mut world).unwrap();
    let empty = ComponentData::default();
    let rollers = (1..=ROLLERS_PLAYING).map(|n| (id(n), "Roller", &empty));
    assert!(game.sync_many(rollers, &mut world).is_empty());
    let mut rolled = Vec::new();
    let mut time = 0.0;
    for (ticks_before, speed) in speeds.iter().enumerate() {
        // As the world's clock goes: one more tick, and a tick's length
        // of game time at the game's speed.
        let dt = speed / 60.0;
        time += dt;
        world.0.set_clock(ticks_before as u64 + 1, time);
        let ticked = game.tick(dt, &mut world);
        assert!(ticked.failures.is_empty(), "{:?}", ticked.failures);
        for n in 1..=ROLLERS_PLAYING {
            let values = game.values(id(n), "Roller").unwrap();
            match (values.get("rolled"), values.get("made")) {
                (Some(Value::Float(number)), Some(Value::Entity(Some(made)))) => {
                    rolled.push((number.to_bits(), *made));
                }
                other => panic!("R{n} rolled and made {other:?}"),
            }
        }
    }
    let asked = world
        .1
        .iter()
        .filter_map(|change| match change {
            Change::Spawn(made, ..) => Some(*made),
            _ => None,
        })
        .collect();
    (rolled, asked)
}

/// Checks a game run at `speeds`: no number and no entity comes twice,
/// and it comes out the same on any number of shards and when run again.
fn never_repeats_and_always_matches(speeds: &[f64]) {
    let (rolled, asked) = rolls(1, speeds);
    let all = speeds.len() * ROLLERS_PLAYING as usize;
    assert_eq!(rolled.len(), all);
    let numbers: HashSet<u64> = rolled.iter().map(|(number, _)| *number).collect();
    assert_eq!(numbers.len(), all, "every roll its own number");
    let entities: HashSet<EntityId> = rolled.iter().map(|(_, made)| *made).collect();
    assert_eq!(entities.len(), all, "every spawn its own entity");
    // What `spawn` gave back is what the world was asked to make.
    assert_eq!(asked.len(), all);
    assert_eq!(asked.iter().copied().collect::<HashSet<_>>(), entities);
    assert_eq!(rolls(1, speeds), (rolled.clone(), asked.clone()), "again");
    for shards in [2, 4] {
        let same = (rolled.clone(), asked.clone());
        assert_eq!(rolls(shards, speeds), same, "on {shards} shards");
    }
}

#[test]
fn random_numbers_and_spawned_entities_never_repeat_while_the_game_s_speed_is_0() {
    // `World:setGameSpeed(0)` is how a game pauses itself: its time
    // stands still from the first tick to the last here.
    never_repeats_and_always_matches(&[0.0; 8]);
}

#[test]
fn random_numbers_and_spawned_entities_never_repeat_as_the_game_s_speed_changes() {
    // As usual, paused, slow motion, paused again, fast, and back.
    never_repeats_and_always_matches(&[1.0, 1.0, 0.0, 0.0, 0.5, 0.5, 0.0, 4.0, 4.0, 1.0]);
}

#[test]
fn a_tick_s_number_tells_its_numbers_and_entities_from_another_tick_s() {
    let mut world = TestWorld::with(&["A"]);
    let mut roll_and_spawn = |tick: u64, time: f64| {
        world.set_clock(tick, time);
        let mut host = ViewHost::new(world.view(), 0);
        (host.random().to_bits(), host.spawn("drone", Vec3::ZERO))
    };
    let first = roll_and_spawn(7, 2.0);
    assert!(first.1.is_some());
    assert_eq!(roll_and_spawn(7, 2.0), first, "the same tick, the same");
    assert_ne!(roll_and_spawn(8, 2.0).0, first.0, "the next tick, paused");
    assert_ne!(roll_and_spawn(8, 2.0).1, first.1);
    // The last tick a game could ever count is still a tick like any.
    let last = roll_and_spawn(u64::MAX, 2.0);
    assert!(last.1.is_some());
    assert_ne!(last, first);
}
