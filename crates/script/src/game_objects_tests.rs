//! Game-wide objects (decision 0080): ticked once, before the components,
//! on the first shard, with their values copied to the others; changed
//! only in their own update; kept by name across new code.

use super::tests::{TestWorld, compiled, id, unhurried};
use super::*;
use reflect::Value;

/// A clock that counts ticks, lamps that read it, and a cheat that tries
/// to change it.
const CLOCK: &str = "fetch \"solis/world\":World;\n\n\
    object Clock {\n\
        has {\n\
            int ticks = 0;\n\
            float hour = 6;\n\
            string phase = \"night\";\n\
        }\n\
        method update(float dt) {\n\
            ticks = ticks + 1;\n\
            hour = hour + 1;\n\
            is hour >= 12? then {\n\
                phase = \"day\";\n\
            }\n\
        }\n\
        method bool isDay() {\n\
            return hour >= 12;\n\
        }\n\
    }\n\
    component Lamp {\n\
        has {\n\
            int seen = 0;\n\
            int sum = 0;\n\
            bool lit = false;\n\
        }\n\
        method update(float dt) {\n\
            seen = Clock.ticks;\n\
            sum = sum + Clock.ticks;\n\
            lit = not Clock:isDay();\n\
        }\n\
    }\n\
    component Cheat {\n\
        has {\n\
            int tries = 0;\n\
        }\n\
        method update(float dt) {\n\
            tries = tries + 1;\n\
            Clock.ticks = 100;\n\
        }\n\
    }\n\
    pass Clock, Lamp, Cheat;\nend\n";

/// `lamps` lamps on `shards` shards (and a cheat, if `cheat`), run for
/// `ticks`: each lamp's values, the objects', the failures and the
/// checksum.
fn run_clock(
    shards: usize,
    lamps: u64,
    ticks: usize,
    cheat: bool,
) -> (Vec<ComponentData>, crate::ObjectValues, Vec<Failure>, u64) {
    let names: Vec<String> = (0..=lamps).map(|n| format!("L{n}")).collect();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    let mut world = TestWorld::with(&names);
    let mut game = unhurried(shards);
    game.load(&compiled(CLOCK), &mut world).unwrap();
    let empty = ComponentData::default();
    let mut components: Vec<(EntityId, &str, &ComponentData)> =
        (1..=lamps).map(|n| (id(n), "Lamp", &empty)).collect();
    if cheat {
        components.push((id(lamps + 1), "Cheat", &empty));
    }
    assert!(game.sync_many(components, &mut world).is_empty());
    let mut failures = Vec::new();
    for _ in 0..ticks {
        failures.extend(game.tick(1.0 / 60.0, &mut world).failures);
    }
    let values = (1..=lamps)
        .map(|n| game.values(id(n), "Lamp").unwrap())
        .collect();
    let sum = game.checksum();
    (values, game.object_values(), failures, sum)
}

#[test]
fn every_component_on_every_core_reads_the_object_as_its_update_left_it() {
    let one = run_clock(1, 30, 7, false);
    let three = run_clock(3, 30, 7, false);
    assert!(one.2.is_empty(), "{:?}", one.2);
    assert!(three.2.is_empty(), "{:?}", three.2);
    for lamp in one.0.iter().chain(&three.0) {
        // The clock ticked first each time: 1, 2, … 7.
        assert_eq!(lamp.get("seen"), Some(&Value::Int(7)));
        assert_eq!(lamp.get("sum"), Some(&Value::Int(28)));
        // Its hour is 13: day, so the lamps are out.
        assert_eq!(lamp.get("lit"), Some(&Value::Bool(false)));
    }
    let clock = &one.1[0];
    assert_eq!(clock.0, "Clock");
    assert_eq!(clock.1.get("ticks"), Some(&Value::Int(7)));
    assert_eq!(clock.1.get("phase"), Some(&Value::Text("day".into())));
    assert_eq!(one.1, three.1, "the same objects on any number of cores");
    assert_eq!(one.3, three.3, "and the same checksum");
}

#[test]
fn a_component_changing_an_object_stops_there_and_the_object_is_untouched() {
    for shards in [1, 2] {
        let (lamps, objects, failures, _) = run_clock(shards, 4, 3, true);
        assert_eq!(failures.len(), 1, "said once: {failures:?}");
        let failure = &failures[0];
        assert_eq!(failure.component, "Cheat");
        assert!(failure.stopped);
        assert!(
            failure.at.message.contains(
                "Clock.ticks can't be changed here: an object changes only in its own update"
            ),
            "{failure}"
        );
        assert_eq!(failure.at.file, "main.lst", "{failure}");
        assert_eq!(failure.at.line, 38, "the line that wrote it: {failure}");
        assert_eq!(objects[0].1.get("ticks"), Some(&Value::Int(3)));
        for lamp in lamps {
            assert_eq!(lamp.get("seen"), Some(&Value::Int(3)));
        }
    }
}

#[test]
fn an_object_whose_update_fails_stops_alone_and_says_so_once() {
    let code = "fetch \"solis/world\":World;\n\n\
        object Broken {\n\
            has {\n\
                int n = 0;\n\
            }\n\
            method update(float dt) {\n\
                n = n + 1;\n\
                int zero = 0;\n\
                World:say((n / zero).text());\n\
            }\n\
        }\n\
        object Counter {\n\
            has {\n\
                int n = 0;\n\
            }\n\
            method update(float dt) {\n\
                n = n + 1;\n\
            }\n\
        }\n\
        pass Broken, Counter;\nend\n";
    let mut world = TestWorld::with(&["A"]);
    let mut game = unhurried(2);
    game.load(&compiled(code), &mut world).unwrap();
    let first = game.tick(1.0 / 60.0, &mut world).failures;
    assert_eq!(first.len(), 1, "{first:?}");
    assert_eq!(first[0].component, "Broken");
    assert_eq!(first[0].entity, None);
    assert!(first[0].stopped);
    assert_eq!(first[0].at.file, "main.lst");
    for _ in 0..3 {
        assert!(game.tick(1.0 / 60.0, &mut world).failures.is_empty());
    }
    let objects = game.object_values();
    assert_eq!(objects[0].1.get("n"), Some(&Value::Int(1)), "stopped");
    assert_eq!(objects[1].1.get("n"), Some(&Value::Int(4)), "carried on");
}

#[test]
fn objects_take_saved_values_that_fit_and_say_which_don_t() {
    let mut world = TestWorld::with(&["A", "B", "C"]);
    let mut game = unhurried(3);
    game.load(&compiled(CLOCK), &mut world).unwrap();
    let empty = ComponentData::default();
    let lamps: Vec<(EntityId, &str, &ComponentData)> =
        (1..=3).map(|n| (id(n), "Lamp", &empty)).collect();
    assert!(game.sync_many(lamps, &mut world).is_empty());
    let mut clock = ComponentData::default();
    clock.set("ticks", Value::Int(40));
    clock.set("hour", Value::Text("noon".into()));
    clock.set("gone", Value::Int(1));
    let mut left_out = game.set_object_values(vec![
        ("Clock".into(), clock),
        ("Nobody".into(), ComponentData::default()),
    ]);
    left_out.sort();
    assert_eq!(left_out, [("Clock".to_string(), "hour".to_string())]);
    game.tick(1.0 / 60.0, &mut world);
    // Every core's lamps read the loaded clock, ticked once.
    for n in 1..=3 {
        let lamp = game.values(id(n), "Lamp").unwrap();
        assert_eq!(lamp.get("seen"), Some(&Value::Int(41)));
    }
    let (schemas, unkept) = game.object_info();
    assert_eq!(schemas[0].name, "Clock");
    assert_eq!(schemas[0].fields.len(), 3);
    assert!(unkept.is_empty());
}

#[test]
fn new_code_keeps_each_object_s_values_by_name() {
    let mut world = TestWorld::with(&["A"]);
    let mut game = unhurried(2);
    game.load(&compiled(CLOCK), &mut world).unwrap();
    for _ in 0..5 {
        game.tick(1.0 / 60.0, &mut world);
    }
    let changed = CLOCK
        .replace("float hour = 6;", "string hour = \"six\";")
        .replace("hour = hour + 1;", "hour = \"seven\";")
        .replace("is hour >= 12? then", "is hour == \"noon\"? then")
        .replace("return hour >= 12;", "return hour == \"noon\";");
    let swapped = game.swap(&compiled(&changed), &mut world).unwrap();
    assert!(
        swapped
            .notes
            .contains(&"Clock's hour is a different kind now, so it starts again.".to_string()),
        "{:?}",
        swapped.notes
    );
    let clock = &game.object_values()[0].1;
    assert_eq!(clock.get("ticks"), Some(&Value::Int(5)), "kept");
    assert_eq!(clock.get("hour"), Some(&Value::Text("six".into())));
    assert_eq!(clock.get("phase"), Some(&Value::Text("night".into())));
}

#[test]
fn one_runtime_ticks_objects_first_and_names_the_fields_it_can_t_keep() {
    let code = "fetch \"solis/world\":World;\n\n\
        object Shop {\n\
            has {\n\
                int coins = 10;\n\
                int array sold = [];\n\
            }\n\
            method update(float dt) {\n\
                coins = coins + Bank.interest;\n\
                Bank.interest = Bank.interest + 1;\n\
            }\n\
        }\n\
        object Bank {\n\
            has {\n\
                int interest = 1;\n\
            }\n\
        }\n\
        component Till {\n\
            has {\n\
                int seen = 0;\n\
            }\n\
            method update(float dt) {\n\
                seen = Shop.coins;\n\
            }\n\
        }\n\
        pass Shop, Bank, Till;\nend\n";
    let mut runtime = Runtime::new().unwrap();
    runtime
        .load(&compiled(code), &mut crate::host::NoHost)
        .unwrap();
    runtime
        .sync(
            id(1),
            "Till",
            &ComponentData::default(),
            &mut crate::host::NoHost,
        )
        .unwrap();
    for _ in 0..3 {
        let failures = runtime.tick(1.0 / 60.0, &mut crate::host::NoHost).failures;
        assert!(failures.is_empty(), "{failures:?}");
    }
    // One object's update may change another: 10 + 1 + 2 + 3.
    let till = runtime.values(id(1), "Till").unwrap();
    assert_eq!(till.get("seen"), Some(&Value::Int(16)));
    assert_eq!(
        runtime.unkept_object_fields(),
        [("Shop".to_string(), "sold".to_string())]
    );
    let names: Vec<String> = runtime
        .object_values()
        .into_iter()
        .map(|(n, _)| n)
        .collect();
    assert_eq!(names, ["Bank", "Shop"], "in name order");
}
