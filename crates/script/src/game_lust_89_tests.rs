//! What Lust alpha 81 to 89 changed for a game's code: handlers go in an
//! `on { }` block, Lust's own events stay in Lust, and game code waits
//! across the game's ticks (`wait`, `start`), on the game's own time, with
//! the work of a component that's gone never going on.

use super::tests::{TestWorld, compiled, id, unhurried};
use super::*;
use reflect::{ComponentData, Value};

/// A bell that, told to ring, waits half a second and then counts it and
/// tells the counter; a ringer that tells the bell once, in its first
/// update; a counter that counts what it's told.
const BELL: &str = r#"event Ring();
event Rang();

component Ringer {
    has {
        entity bell = nothing;
        bool rung = false;
    }

    method update(float dt) {
        is not rung? then {
            send Ring() to bell;
            rung = true;
        }
    }
}

component Bell {
    has {
        entity counter = nothing;
        int heard = 0;
    }

    on {
        Ring() {
            wait 0.5;
            heard = heard + 1;
            send Rang() to counter;
        }
    }
}

component Counter {
    has {
        int rang = 0;
    }

    on {
        Rang() {
            rang = rang + 1;
        }
    }
}

pass Ringer, Bell, Counter;
end
"#;

/// A game of [`BELL`] on `shards` cores: the ringer on 1, the bell on 2,
/// the counter on 3.
fn bells(shards: usize) -> (Game, TestWorld) {
    let mut world = TestWorld::with(&["Ringer", "Bell", "Counter"]);
    let mut game = unhurried(shards);
    game.load(&compiled(BELL), &mut world).unwrap();
    let mut ringer = ComponentData::default();
    ringer.set("bell", Value::Entity(Some(id(2))));
    let mut bell = ComponentData::default();
    bell.set("counter", Value::Entity(Some(id(3))));
    let none = ComponentData::default();
    assert!(
        game.sync_many(
            [
                (id(1), "Ringer", &ringer),
                (id(2), "Bell", &bell),
                (id(3), "Counter", &none),
            ],
            &mut world,
        )
        .is_empty()
    );
    (game, world)
}

fn int(game: &mut Game, which: u64, component: &str, field: &str) -> Option<i64> {
    match game.values(id(which), component)?.get(field) {
        Some(Value::Int(n)) => Some(*n),
        other => panic!("{other:?}"),
    }
}

fn ticks(game: &mut Game, world: &mut TestWorld, n: usize, dt: f64) {
    for _ in 0..n {
        let ticked = game.tick(dt, world);
        assert!(ticked.failures.is_empty(), "{:?}", ticked.failures);
    }
}

#[test]
fn a_handler_waits_on_the_game_s_ticks_and_goes_on_later() {
    for shards in [1, 4] {
        let (mut game, mut world) = bells(shards);
        // The ring is heard in the first tick, and the bell waits.
        ticks(&mut game, &mut world, 20, 1.0 / 60.0);
        assert_eq!(int(&mut game, 2, "Bell", "heard"), Some(0), "{shards}");
        // Half a second of ticks after it started waiting, it goes on, and
        // what it sends then arrives.
        ticks(&mut game, &mut world, 20, 1.0 / 60.0);
        assert_eq!(int(&mut game, 2, "Bell", "heard"), Some(1), "{shards}");
        assert_eq!(int(&mut game, 3, "Counter", "rang"), Some(1), "{shards}");
    }
}

#[test]
fn a_paused_game_moves_no_waits() {
    let (mut game, mut world) = bells(1);
    ticks(&mut game, &mut world, 1, 1.0 / 60.0);
    ticks(&mut game, &mut world, 200, 0.0);
    assert_eq!(int(&mut game, 2, "Bell", "heard"), Some(0));
    ticks(&mut game, &mut world, 40, 1.0 / 60.0);
    assert_eq!(int(&mut game, 2, "Bell", "heard"), Some(1));
}

#[test]
fn a_component_taken_off_while_it_waits_never_goes_on() {
    for shards in [1, 4] {
        let (mut game, mut world) = bells(shards);
        ticks(&mut game, &mut world, 5, 1.0 / 60.0);
        game.retain_on([(id(2), Vec::new())]);
        ticks(&mut game, &mut world, 60, 1.0 / 60.0);
        assert_eq!(game.values(id(2), "Bell"), None);
        assert_eq!(
            int(&mut game, 3, "Counter", "rang"),
            Some(0),
            "{shards}: its waiting handler was cancelled"
        );
    }
}

#[test]
fn an_update_starts_work_that_waits() {
    let main = r#"component Door {
    has {
        bool started = false;
        bool open = false;
    }

    method update(float dt) {
        is not started? then {
            started = true;
            start opening();
        }
    }

    method opening() {
        wait 0.25;
        open = true;
    }
}

pass Door;
end
"#;
    let mut world = TestWorld::with(&["Door"]);
    let mut game = unhurried(1);
    game.load(&compiled(main), &mut world).unwrap();
    assert!(
        game.sync_many([(id(1), "Door", &ComponentData::default())], &mut world)
            .is_empty()
    );
    let open = |game: &mut Game| game.values(id(1), "Door").unwrap().get("open").cloned();
    ticks(&mut game, &mut world, 10, 1.0 / 60.0);
    assert_eq!(open(&mut game), Some(Value::Bool(false)));
    ticks(&mut game, &mut world, 10, 1.0 / 60.0);
    assert_eq!(open(&mut game), Some(Value::Bool(true)));
}

#[test]
fn a_handler_on_its_own_is_a_mistake_showing_the_on_block() {
    let main = "event Ring();\ncomponent Bell {\n    on Ring() {\n    }\n}\npass Bell;\nend\n";
    let code = crate::code::Code {
        files: vec![("main.lst".into(), main.to_string())],
        lua: Vec::new(),
        notes: Vec::new(),
    };
    let problems = crate::code::check(&code);
    assert!(
        problems
            .iter()
            .any(|p| p.line == 3 && p.message.contains("on { Ring() { … } }")),
        "{problems:?}"
    );
}

#[test]
fn lust_s_own_events_reach_an_object_from_its_update_and_change_it() {
    let main = r#"event Saved();

object Log {
    has {
        int saves = 0;
    }

    method update(float dt) {
        send Saved();
    }

    on {
        Saved() {
            saves = saves + 1;
        }
    }
}

component Keeper {
    has {
        int seen = 0;
    }

    method update(float dt) {
        seen = Log.saves;
    }
}

pass Keeper, Log;
end
"#;
    let mut world = TestWorld::with(&["Keeper"]);
    let mut game = unhurried(1);
    game.load(&compiled(main), &mut world).unwrap();
    assert!(
        game.sync_many([(id(1), "Keeper", &ComponentData::default())], &mut world)
            .is_empty()
    );
    // Heard as the object's update ends, before the components run.
    ticks(&mut game, &mut world, 3, 1.0 / 60.0);
    assert_eq!(int(&mut game, 1, "Keeper", "seen"), Some(3));
}

#[test]
fn a_component_s_lust_event_can_t_change_an_object() {
    let main = r#"event Saved();

object Log {
    has {
        int saves = 0;
    }

    on {
        Saved() {
            saves = saves + 1;
        }
    }
}

component Keeper {
    method update(float dt) {
        send Saved() to Log;
    }
}

pass Keeper, Log;
end
"#;
    let mut world = TestWorld::with(&["Keeper"]);
    let mut game = unhurried(1);
    game.load(&compiled(main), &mut world).unwrap();
    assert!(
        game.sync_many([(id(1), "Keeper", &ComponentData::default())], &mut world)
            .is_empty()
    );
    // Decision 0080 still holds: an object changes only in its own update.
    let failures = game.tick(1.0 / 60.0, &mut world).failures;
    assert_eq!(failures.len(), 1, "{failures:?}");
    assert_eq!(failures[0].component, "Keeper");
    assert!(
        failures[0].at.message.contains("only in its own update"),
        "{failures:?}"
    );
}

#[test]
fn a_wait_in_update_is_a_mistake_saying_to_start_it() {
    let main = "component Door {\n    method update(float dt) {\n        wait 1;\n    }\n}\npass Door;\nend\n";
    let code = crate::code::Code {
        files: vec![("main.lst".into(), main.to_string())],
        lua: Vec::new(),
        notes: Vec::new(),
    };
    let problems = crate::code::check(&code);
    assert!(
        problems.iter().any(|p| p.line == 3 && p.message.contains("start")),
        "{problems:?}"
    );
}
