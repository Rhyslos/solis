use super::*;
use crate::code::Compiled;
use crate::convert::label;
use crate::host::NoHost;
use glam::{Quat, Vec3};
use reflect::{FieldKind, Value};
use std::collections::{BTreeSet, HashMap};
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

const NATIVES: &str = "\n\
                       \n";

/// The Lua for a program whose `main.lst` is `main`, with the engine's
/// description files.
fn chunk(main: &str) -> Compiled {
    let mut files = crate::describe::descriptions("Test 1.0", "solis");
    files.push(("main.lst".into(), format!("{NATIVES}{main}")));
    match lust::to_lua_with(
        &lust::Engine {
            folders: crate::describe::engine_folders(&files),
        },
        "main.lst",
        &files,
    ) {
        Ok(chunk) => Compiled {
            lust: Some(chunk),
            lua: Vec::new(),
            schemas: Vec::new(),
            places: Default::default(),
            bytecode: Default::default(),
            screens: None,
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
        "fetch \"solis/world\":World;\ncomponent Mover {\n\
             has {\n\
                 float speed = 2;\n\
             }\n\
             method update(float dt) {\n\
                 vec3 ahead = [1, 0, 0];\n\
                 moveTo(position() + ahead * speed * dt);\n\
                 turnTo(World:find(\"Pole\"):rotation());\n\
                 World:say(name() + \" at \" + World:time().text());\n\
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

/// Gives `runtime` a call's real limit, and an hour for the rest (the
/// review's TEST-02): a test of one call that runs too long is about that
/// call. Were the machine to keep the test off the processor past the
/// tick's limit too, the tick would be cut short instead, and nothing
/// would be said of the call.
fn only_the_call_s_limit(runtime: &Runtime) {
    runtime.set_limits(crate::luau::Limits {
        call: crate::luau::TIME_LIMIT,
        ..crate::luau::Limits::unhurried()
    });
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
    only_the_call_s_limit(&runtime);
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
    only_the_call_s_limit(&runtime);
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

/// A Rust component counting its ticks.
struct Tally(i64);

impl crate::RustComponent for Tally {
    fn update(&mut self, _cx: &mut crate::Context<'_>, _dt: f64) {
        self.0 += 1;
    }
    fn values(&self) -> ComponentData {
        let mut data = ComponentData::default();
        data.set("count", Value::Int(self.0));
        data
    }
    fn set(&mut self, data: &ComponentData) {
        if let Some(Value::Int(count)) = data.get("count") {
            self.0 = *count;
        }
    }
}

/// The review, BUG-54 (issue 57): the editor shows a running component's
/// values as the game has them, so a field set there to what the area
/// already holds is a change too; but the area's values are what they
/// were, and nothing was given. In each language.
#[test]
fn a_field_set_by_hand_is_given_whatever_the_area_held_in_lust_lua_and_rust() {
    let mut compiled = chunk(GUN);
    compiled.lua = vec![(
        "spinner.lua".into(),
        "local S = component(\"Spinner\", { { \"turns\", \"int\", 0 } })\n\
         function S:update(dt) self.turns = self.turns + 1 end\n"
            .into(),
    )];
    let mut runtime = Runtime::new().unwrap();
    runtime.add_rust(RustKind {
        schema: ComponentSchema {
            name: "Tally".into(),
            help: String::new(),
            fields: vec![reflect::FieldInfo::new(
                "count",
                "Count",
                FieldKind::Int,
                Value::Int(0),
            )],
        },
        make: |data| Box::new(Tally(reflect::Fields(data).int("count", 0))),
    });
    runtime.load(&compiled, &mut NoHost).unwrap();
    let me = id(7);
    // Each component's counting field, and what the area holds for it.
    let kinds = [
        ("Gun", "ammo", 30),
        ("Spinner", "turns", 0),
        ("Tally", "count", 0),
    ];
    let area = kinds.map(|(_, field, start)| {
        let mut data = ComponentData::default();
        data.set(field, Value::Int(start));
        data
    });
    let all = || (0..3).map(|i| (me, kinds[i].0, &area[i]));
    let now = |runtime: &Runtime| {
        kinds.map(
            |(name, field, _)| match runtime.values(me, name).unwrap().get(field) {
                Some(Value::Int(n)) => *n,
                other => panic!("{name}.{field} is {other:?}"),
            },
        )
    };
    assert!(runtime.sync_many(all(), &mut NoHost).is_empty());
    for _ in 0..5 {
        assert!(runtime.tick(1.0 / 60.0, &mut NoHost).failures.is_empty());
    }
    assert_eq!(now(&runtime), [25, 5, 5]);
    // The area's values again, nothing set by hand (another field of the
    // entity changed, say): what the game counted is kept.
    assert!(runtime.sync_many(all(), &mut NoHost).is_empty());
    assert_eq!(now(&runtime), [25, 5, 5]);

    // Each set by hand to what the area holds: given.
    let set = kinds.map(|(_, field, _)| BTreeSet::from([field.to_string()]));
    let edited = (0..3).map(|i| (me, kinds[i].0, &area[i], &set[i]));
    assert!(runtime.sync_edited(edited, &mut NoHost).is_empty());
    assert_eq!(now(&runtime), [30, 0, 0]);
    let shot = runtime.values(me, "Gun").unwrap().get("lastShot").cloned();
    let Some(Value::Float(shot)) = shot else {
        panic!("{shot:?}");
    };
    assert!(
        shot > 0.55,
        "a field not set keeps the game's value: {shot}"
    );

    // Once: the game counts on, and the next sync leaves it be.
    assert!(runtime.tick(1.0 / 60.0, &mut NoHost).failures.is_empty());
    assert!(runtime.sync_many(all(), &mut NoHost).is_empty());
    assert_eq!(now(&runtime), [29, 1, 1]);

    // A field the component hasn't, or one the area has no value for:
    // nothing is given and nothing fails.
    let odd = BTreeSet::from(["nonsense".to_string(), "lastShot".to_string()]);
    let edited = (0..3).map(|i| (me, kinds[i].0, &area[i], &odd));
    assert!(runtime.sync_edited(edited, &mut NoHost).is_empty());
    assert_eq!(now(&runtime), [29, 1, 1]);
}
