use super::*;
use crate::code::Compiled;
use crate::convert::label;
use crate::host::NoHost;
use glam::{Quat, Vec3};
use reflect::{FieldKind, Value};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// A small world for game code: entities with names, places and turns,
/// and everything said.
#[derive(Default)]
struct TestHost {
    names: Vec<(EntityId, String)>,
    places: HashMap<EntityId, Vec3>,
    turns: HashMap<EntityId, Quat>,
    said: Vec<String>,
    time: f64,
}

impl Host for TestHost {
    fn say(&mut self, words: &str) {
        self.said.push(words.to_string());
    }
    fn time(&self) -> f64 {
        self.time
    }
    fn find(&self, name: &str) -> Option<EntityId> {
        self.names
            .iter()
            .find(|(_, n)| n == name)
            .map(|(id, _)| *id)
    }
    fn position(&self, which: EntityId) -> Option<Vec3> {
        self.places.get(&which).copied()
    }
    fn move_to(&mut self, which: EntityId, place: Vec3) {
        if self.places.contains_key(&which) {
            self.places.insert(which, place);
        }
    }
    fn rotation(&self, which: EntityId) -> Option<Quat> {
        self.turns.get(&which).copied()
    }
    fn turn_to(&mut self, which: EntityId, facing: Quat) {
        self.turns.insert(which, facing);
    }
    fn name(&self, which: EntityId) -> Option<String> {
        self.names
            .iter()
            .find(|(id, _)| *id == which)
            .map(|(_, n)| n.clone())
    }
}

const NATIVES: &str = "get \"solis/world\":[say, time, find];\n\
                       get \"solis/entity\":[positionOf, moveTo, rotationOf, turnTo, nameOf];\n";

/// The Lua for a program whose `main.lst` is `main`, with the engine's
/// description files.
fn chunk(main: &str) -> Compiled {
    let mut files = crate::describe::descriptions("Test 1.0", "solis");
    files.push(("main.lst".into(), format!("{NATIVES}{main}")));
    match lust::to_lua("main.lst", &files) {
        Ok(chunk) => Compiled {
            lust: Some(chunk),
            lua: Vec::new(),
            schemas: Vec::new(),
            places: Default::default(),
            bytecode: Default::default(),
        },
        Err(problems) => panic!("{}", problems[0]),
    }
}

fn loaded(main: &str) -> Runtime {
    let mut runtime = Runtime::new().unwrap();
    runtime.load(&chunk(main), &mut NoHost).unwrap();
    runtime
}

fn id(n: u64) -> EntityId {
    EntityId(reflect::Id::from_raw(n).unwrap())
}

const GUN: &str = "state Mode = [Single, Burst];\n\
    component Gun {\n\
        has {\n\
            int ammo = 30;\n\
            float lastShot = 0.5;\n\
            Mode mode = Mode.Burst;\n\
            entity target = nothing;\n\
            vec3 aim = [1, 2, 3];\n\
            rotation turn = [0, 0, 90];\n\
            string label = \"gun\";\n\
            bool ready = true;\n\
        }\n\
        method update(float dt) {\n\
            ammo = ammo - 1;\n\
            lastShot = lastShot + dt;\n\
        }\n\
    }\n\
    pass Gun;\nend\n";

#[test]
fn a_lust_component_becomes_a_schema_with_its_kinds_and_starting_values() {
    let runtime = loaded(GUN);
    assert_eq!(runtime.schemas().len(), 1);
    let gun = &runtime.schemas()[0];
    assert_eq!(gun.name, "Gun");
    let names: Vec<&str> = gun.fields.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "ammo", "lastShot", "mode", "target", "aim", "turn", "label", "ready"
        ],
        "in the order written"
    );
    let field = |n: &str| gun.field(n).unwrap();
    assert_eq!(field("ammo").default, Value::Int(30));
    assert_eq!(field("lastShot").kind, FieldKind::Float);
    assert_eq!(field("lastShot").label, "Last shot");
    assert_eq!(
        field("mode").kind,
        FieldKind::Choice(vec!["Single".into(), "Burst".into()])
    );
    assert_eq!(field("mode").default, Value::Text("Burst".into()));
    assert_eq!(field("target").default, Value::Entity(None));
    assert_eq!(field("aim").default, Value::floats(&[1.0, 2.0, 3.0]));
    let turn = field("turn").default.as_floats::<4>().unwrap();
    assert!(
        (turn[2] - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-6,
        "{turn:?}"
    );
    assert_eq!(field("label").default, Value::Text("gun".into()));
    assert_eq!(field("ready").default, Value::Bool(true));
    assert!(runtime.has_component("Gun") && !runtime.has_component("Shield"));
}

#[test]
fn the_area_s_values_reach_a_component_and_only_changes_are_sent_again() {
    let mut runtime = loaded(GUN);
    let me = id(7);
    let target = id(9);
    let mut data = ComponentData::default();
    data.set("ammo", Value::Int(5));
    data.set("target", Value::Entity(Some(target)));
    data.set("mode", Value::Text("Nonsense".into()));
    data.set("label", Value::Int(3));
    runtime.sync(me, "Gun", &data, &mut NoHost).unwrap();
    let values = runtime.values(me, "Gun").unwrap();
    assert_eq!(values.get("ammo"), Some(&Value::Int(5)));
    assert_eq!(values.get("target"), Some(&Value::Entity(Some(target))));
    assert_eq!(
        values.get("mode"),
        Some(&Value::Text("Burst".into())),
        "a value that doesn't fit keeps the starting one"
    );
    assert_eq!(values.get("label"), Some(&Value::Text("gun".into())));
    // The game uses ammo up; the editor changes another field: the ammo
    // left stays.
    runtime.tick(1.0 / 60.0, &mut NoHost);
    data.set("label", Value::Text("left gun".into()));
    runtime.sync(me, "Gun", &data, &mut NoHost).unwrap();
    let values = runtime.values(me, "Gun").unwrap();
    assert_eq!(values.get("ammo"), Some(&Value::Int(4)));
    assert_eq!(values.get("label"), Some(&Value::Text("left gun".into())));
    // A component the program doesn't have is left alone.
    runtime.sync(me, "Shield", &data, &mut NoHost).unwrap();
    assert_eq!(runtime.len(), 1);
    runtime.retain(|_, _| false);
    assert!(runtime.is_empty());
}

#[test]
fn ticks_run_in_order_and_natives_reach_the_host() {
    let mut runtime = loaded(
        "component Mover {\n\
             has {\n\
                 float speed = 2;\n\
             }\n\
             method update(float dt) {\n\
                 vec3 ahead = [1, 0, 0];\n\
                 moveTo(me, positionOf(me) + ahead * speed * dt);\n\
                 turnTo(me, rotationOf(find(\"Pole\")));\n\
                 say(nameOf(me) + \" at \" + time().text());\n\
             }\n\
         }\n\
         pass Mover;\nend\n",
    );
    let (a, b, pole) = (id(2), id(1), id(3));
    let mut host = TestHost {
        names: vec![(b, "B".into()), (a, "A".into()), (pole, "Pole".into())],
        time: 1.5,
        ..TestHost::default()
    };
    host.places.insert(a, Vec3::ZERO);
    host.places.insert(b, Vec3::new(0.0, 5.0, 0.0));
    host.turns.insert(pole, Quat::from_rotation_z(1.0));
    for e in [a, b] {
        runtime
            .sync(e, "Mover", &ComponentData::default(), &mut host)
            .unwrap();
    }
    assert!(runtime.tick(0.5, &mut host).failures.is_empty());
    assert_eq!(host.places[&a], Vec3::new(1.0, 0.0, 0.0));
    assert_eq!(host.places[&b], Vec3::new(1.0, 5.0, 0.0));
    assert!(host.turns[&a].abs_diff_eq(Quat::from_rotation_z(1.0), 1e-6));
    assert_eq!(host.said, ["B at 1.5", "A at 1.5"], "entities in ID order");
}

#[test]
fn a_failing_component_stops_alone_and_says_where_in_the_lust() {
    let mut runtime = loaded(
        "component Divider {\n\
             has {\n\
                 int ways = 0;\n\
             }\n\
             method update(float dt) {\n\
                 int each = 12 / ways;\n\
             }\n\
         }\n\
         component Counter {\n\
             has {\n\
                 int ticks = 0;\n\
             }\n\
             method update(float dt) {\n\
                 ticks = ticks + 1;\n\
             }\n\
         }\n\
         pass Divider, Counter;\nend\n",
    );
    let me = id(4);
    for name in ["Divider", "Counter"] {
        runtime
            .sync(me, name, &ComponentData::default(), &mut NoHost)
            .unwrap();
    }
    let failures = runtime.tick(0.1, &mut NoHost).failures;
    assert_eq!(failures.len(), 1);
    let failure = &failures[0];
    assert_eq!(
        (failure.entity, failure.component.as_str()),
        (Some(me), "Divider")
    );
    assert_eq!(failure.at.file, "main.lst");
    assert_eq!(failure.at.line, 8, "{failure}");
    // Stopped: no second report; the counter carries on.
    assert!(runtime.tick(0.1, &mut NoHost).failures.is_empty());
    assert_eq!(
        runtime.values(me, "Counter").unwrap().get("ticks"),
        Some(&Value::Int(2))
    );
}

#[test]
fn a_loop_that_never_ends_is_stopped_by_the_time_limit() {
    let mut runtime = loaded(
        "component Spinner {\n\
             has {\n\
                 int turns = 0;\n\
             }\n\
             method update(float dt) {\n\
                 loop {\n\
                     turns = turns + 1;\n\
                 }\n\
             }\n\
         }\n\
         pass Spinner;\nend\n",
    );
    runtime
        .sync(id(5), "Spinner", &ComponentData::default(), &mut NoHost)
        .unwrap();
    let start = Instant::now();
    let failures = runtime.tick(0.1, &mut NoHost).failures;
    assert!(start.elapsed() < Duration::from_secs(5));
    assert_eq!(failures.len(), 1);
    assert!(
        failures[0].at.message.contains("longer than"),
        "{}",
        failures[0]
    );
}

#[test]
fn a_program_whose_start_fails_or_takes_all_memory_is_refused() {
    let mut runtime = Runtime::new().unwrap();
    let failure = runtime
        .load(
            &chunk("int ways = 0;\nint each = 12 / ways;\nend\n"),
            &mut NoHost,
        )
        .unwrap_err();
    assert!(failure.component.is_empty());
    assert!(runtime.schemas().is_empty());
    // Text that doubles until memory runs out: refused, not a crash.
    let hungry = chunk(
        "component Hungry {\n\
             has {\n\
                 string text = \"x\";\n\
             }\n\
             method update(float dt) {\n\
                 loop {\n\
                     text = text + text;\n\
                 }\n\
             }\n\
         }\n\
         pass Hungry;\nend\n",
    );
    runtime.load(&hungry, &mut NoHost).unwrap();
    runtime
        .sync(id(6), "Hungry", &ComponentData::default(), &mut NoHost)
        .unwrap();
    assert_eq!(runtime.tick(0.1, &mut NoHost).failures.len(), 1);
    // The runtime still works afterwards.
    runtime.load(&chunk(GUN), &mut NoHost).unwrap();
    assert_eq!(runtime.schemas().len(), 1);
}

#[test]
fn labels_read_naturally() {
    assert_eq!(label("ammo"), "Ammo");
    assert_eq!(label("lastShot"), "Last shot");
    assert_eq!(label("max_hp"), "Max hp");
}
