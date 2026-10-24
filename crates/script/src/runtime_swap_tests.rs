//! Swapping new code into a running game.

use super::*;
use crate::code::Compiled;
use crate::host::NoHost;
use reflect::Value;

fn chunk(main: &str) -> Compiled {
    let mut files = crate::describe::descriptions("Test 1.0", "solis");
    files.push(("main.lst".into(), main.to_string()));
    match lust::to_lua("main.lst", &files) {
        Ok(chunk) => Compiled {
            lust: Some(chunk),
            lua: Vec::new(),
            schemas: Vec::new(),
            places: Default::default(),
        },
        Err(problems) => panic!("{}", problems[0]),
    }
}

fn id(n: u64) -> EntityId {
    EntityId(reflect::Id::from_raw(n).unwrap())
}

const V1: &str = "component Gun {\n\
        has {\n\
            int ammo = 30;\n\
            int heat = 0;\n\
            string label = \"gun\";\n\
        }\n\
        method update(float dt) {\n\
            ammo = ammo - 1;\n\
        }\n\
    }\n\
    component Shield {\n\
        has {\n\
            int layers = 2;\n\
        }\n\
    }\n\
    pass Gun, Shield;\nend\n";

/// Gun's ammo uses 2 a tick now, heat became a float, label is gone and
/// range is new; Shield is gone and Radar is new.
const V2: &str = "component Gun {\n\
        has {\n\
            int ammo = 30;\n\
            float heat = 0.5;\n\
            float range = 40;\n\
        }\n\
        method update(float dt) {\n\
            ammo = ammo - 2;\n\
        }\n\
    }\n\
    component Radar {\n\
        has {\n\
            float reach = 100;\n\
        }\n\
    }\n\
    pass Gun, Radar;\nend\n";

fn running_v1() -> Runtime {
    let mut runtime = Runtime::new().unwrap();
    runtime.load(&chunk(V1), &mut NoHost).unwrap();
    runtime
        .sync(id(1), "Gun", &ComponentData::default(), &mut NoHost)
        .unwrap();
    runtime
        .sync(id(1), "Shield", &ComponentData::default(), &mut NoHost)
        .unwrap();
    for _ in 0..5 {
        runtime.tick(0.1, &mut NoHost);
    }
    runtime
}

#[test]
fn new_code_keeps_values_by_name_and_says_what_started_over() {
    let mut runtime = running_v1();
    let swapped = runtime.swap(&chunk(V2), &mut NoHost).unwrap();
    assert_eq!(swapped.changed, ["Gun"]);
    assert_eq!(swapped.removed, ["Shield"]);
    assert_eq!(swapped.added, ["Radar"]);
    assert!(
        swapped.notes.iter().any(|n| n.contains("heat")),
        "{:?}",
        swapped.notes
    );
    let gun = runtime.values(id(1), "Gun").unwrap();
    assert_eq!(
        gun.get("ammo"),
        Some(&Value::Int(25)),
        "the ammo used so far"
    );
    assert_eq!(
        gun.get("heat"),
        Some(&Value::Float(0.5)),
        "a new kind starts over"
    );
    assert_eq!(gun.get("range"), Some(&Value::Float(40.0)), "a new field");
    assert!(gun.get("label").is_none(), "a removed field is dropped");
    assert!(runtime.values(id(1), "Shield").is_none(), "Shield stopped");
    assert_eq!(runtime.len(), 1);
    // The new update runs from now on.
    runtime.tick(0.1, &mut NoHost);
    assert_eq!(
        runtime.values(id(1), "Gun").unwrap().get("ammo"),
        Some(&Value::Int(23))
    );
    assert!(runtime.has_component("Radar") && !runtime.has_component("Shield"));
}

#[test]
fn new_code_that_fails_to_start_leaves_the_old_code_running() {
    let mut runtime = running_v1();
    let failure = runtime
        .swap(
            &chunk("int ways = 0;\nint each = 1 / ways;\nend\n"),
            &mut NoHost,
        )
        .unwrap_err();
    assert!(failure.component.is_empty());
    runtime.tick(0.1, &mut NoHost);
    assert_eq!(
        runtime.values(id(1), "Gun").unwrap().get("ammo"),
        Some(&Value::Int(24)),
        "still the old update"
    );
}

#[test]
fn a_stopped_component_runs_again_once_its_code_is_fixed() {
    let broken = "component Divider {\n    has {\n        int ways = 0;\n        int done = 0;\n    }\n    method update(float dt) {\n        done = done + 12 / ways;\n    }\n}\npass Divider;\nend\n";
    let fixed = "component Divider {\n    has {\n        int ways = 0;\n        int done = 0;\n    }\n    method update(float dt) {\n        done = done + 1;\n    }\n}\npass Divider;\nend\n";
    let mut runtime = Runtime::new().unwrap();
    runtime.load(&chunk(broken), &mut NoHost).unwrap();
    runtime
        .sync(id(2), "Divider", &ComponentData::default(), &mut NoHost)
        .unwrap();
    assert_eq!(runtime.tick(0.1, &mut NoHost).failures.len(), 1);
    assert!(
        runtime.tick(0.1, &mut NoHost).failures.is_empty(),
        "stopped"
    );
    let swapped = runtime.swap(&chunk(fixed), &mut NoHost).unwrap();
    assert!(swapped.changed.is_empty(), "same fields");
    assert!(runtime.tick(0.1, &mut NoHost).failures.is_empty());
    assert_eq!(
        runtime.values(id(2), "Divider").unwrap().get("done"),
        Some(&Value::Int(1)),
        "running again"
    );
}

#[test]
fn code_deleted_stops_the_lust_and_code_added_starts_it() {
    let mut runtime = running_v1();
    runtime.unload();
    assert!(runtime.is_empty() && !runtime.has_component("Gun"));
    let swapped = runtime.swap(&chunk(V1), &mut NoHost).unwrap();
    assert_eq!(swapped.added, ["Gun", "Shield"]);
    assert!(runtime.has_component("Gun"));
}
