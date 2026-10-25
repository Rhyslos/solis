//! Where the runtime meets the rest (the seams review of Phase 4): what
//! the code's start can do, which side an event's mistake is blamed on,
//! names Lua can't use, and which file a failure is in.

use super::*;
use crate::code::{Code, Compiled};
use crate::host::NoHost;
use reflect::Value;

fn id(n: u64) -> EntityId {
    EntityId(reflect::Id::from_raw(n).unwrap())
}

/// The code for `lua` files and Lust's `main.lst` (if not empty), as the
/// worker compiles it.
fn compiled(main: &str, lua: &[(&str, &str)]) -> Result<Compiled, Vec<CodeProblem>> {
    let mut files = Vec::new();
    if !main.is_empty() {
        files = crate::describe::descriptions("Test 1.0", "solis");
        files.push(("main.lst".into(), main.to_string()));
    }
    let code = Code {
        files,
        lua: lua
            .iter()
            .map(|(p, t)| ((*p).to_string(), (*t).to_string()))
            .collect(),
        notes: Vec::new(),
    };
    crate::compile(&code).map(|c| c.expect("there's code"))
}

/// Keeps what's said.
#[derive(Default)]
struct Said(Vec<String>);

impl Host for Said {
    fn say(&mut self, words: &str) {
        self.0.push(words.to_string());
    }
    fn time(&self) -> f64 {
        1.5
    }
    fn find(&self, _name: &str) -> Option<EntityId> {
        None
    }
    fn position(&self, _which: EntityId) -> Option<glam::Vec3> {
        None
    }
    fn move_to(&mut self, _which: EntityId, _place: glam::Vec3) {}
    fn rotation(&self, _which: EntityId) -> Option<glam::Quat> {
        None
    }
    fn turn_to(&mut self, _which: EntityId, _facing: glam::Quat) {}
    fn name(&self, _which: EntityId) -> Option<String> {
        None
    }
}

#[test]
fn the_code_s_start_can_send_and_call_natives_and_its_components_know_where_they_are() {
    let main = "get \"solis/world\":[say, time];\n\
        event Hello();\n\
        send Hello() to nothing;\n\
        component Clock {\n    has {\n        float born = time();\n    }\n}\n\
        pass Clock;\nend\n";
    let spin = "local S = component('Spinner', { { 'n', 'int', 0 } })";
    let compiled = compiled(main, &[("parts/spin.lua", spin)]).unwrap();
    let names: Vec<&str> = compiled.schemas.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["Clock", "Spinner"]);
    assert_eq!(compiled.places["Clock"], ("main.lst".to_string(), 4));
    assert_eq!(compiled.places["Spinner"].0, "parts/spin.lua");
}

#[test]
fn an_event_sent_with_the_wrong_values_is_the_sender_s_mistake() {
    let main = "event Hit(entity by, float damage);\n\
        component Health {\n    has {\n        float hp = 10;\n    }\n\
        \x20   on Hit(entity by, float damage) {\n        hp = hp - damage;\n    }\n}\n\
        pass Health;\nend\n";
    let lua = "local G = component('Gun', {})\n\
               function G:update(dt) solis.send(self.me, 'Hit', self.me, 'lots') end\n\
               local Q = component('Quiet', {})";
    let compiled = compiled(main, &[("gun.lua", lua)]).unwrap();
    let mut runtime = Runtime::new().unwrap();
    runtime.load(&compiled, &mut NoHost).unwrap();
    let me = id(3);
    for name in ["Gun", "Health", "Quiet"] {
        runtime
            .sync(me, name, &ComponentData::default(), &mut NoHost)
            .unwrap();
    }
    let failures = runtime.tick(0.1, &mut NoHost).failures;
    assert_eq!(failures.len(), 1, "{failures:?}");
    assert_eq!(failures[0].component, "Gun");
    assert_eq!(failures[0].at.file, "gun.lua");
    assert!(
        failures[0]
            .at
            .message
            .contains("Hit's value 2 should be a float"),
        "{}",
        failures[0]
    );
}

#[test]
fn a_number_in_an_event_no_lust_declares_is_a_float_even_when_whole() {
    use std::cell::RefCell;
    thread_local! {
        static GOT: RefCell<Vec<Value>> = const { RefCell::new(Vec::new()) };
    }
    struct Catcher;
    impl crate::RustComponent for Catcher {
        fn handle(&mut self, _cx: &mut crate::Context<'_>, event: &crate::Event) -> bool {
            GOT.with(|g| g.borrow_mut().extend(event.values.iter().cloned()));
            true
        }
        fn values(&self) -> ComponentData {
            ComponentData::default()
        }
        fn set(&mut self, _data: &ComponentData) {}
    }
    let lua = "local B = component('Boomer', {})\n\
               function B:update(dt) solis.send(self.me, 'Boom', 2, 2.5) end";
    let compiled = compiled("", &[("boom.lua", lua)]).unwrap();
    let mut runtime = Runtime::new().unwrap();
    runtime.add_rust(RustKind {
        schema: reflect::ComponentSchema {
            name: "Catcher".into(),
            help: String::new(),
            fields: Vec::new(),
        },
        make: |_| Box::new(Catcher),
    });
    runtime.load(&compiled, &mut NoHost).unwrap();
    for name in ["Boomer", "Catcher"] {
        runtime
            .sync(id(2), name, &ComponentData::default(), &mut NoHost)
            .unwrap();
    }
    assert!(runtime.tick(0.1, &mut NoHost).failures.is_empty());
    GOT.with(|g| assert_eq!(*g.borrow(), [Value::Float(2.0), Value::Float(2.5)]));
}

#[test]
fn a_rust_component_s_wrong_send_is_said_once_and_it_keeps_running() {
    struct Sloppy(i64);
    impl crate::RustComponent for Sloppy {
        fn update(&mut self, cx: &mut crate::Context<'_>, _dt: f64) {
            self.0 += 1;
            let me = cx.me();
            assert!(!cx.send(Some(me), "Hit", vec![Value::Int(1)]));
        }
        fn values(&self) -> ComponentData {
            let mut data = ComponentData::default();
            data.set("ticks", Value::Int(self.0));
            data
        }
        fn set(&mut self, _data: &ComponentData) {}
    }
    let main = "event Hit(entity by, float damage);\nend\n";
    let compiled = compiled(main, &[]).unwrap();
    let mut runtime = Runtime::new().unwrap();
    runtime.add_rust(RustKind {
        schema: reflect::ComponentSchema {
            name: "Sloppy".into(),
            help: String::new(),
            fields: Vec::new(),
        },
        make: |_| Box::new(Sloppy(0)),
    });
    runtime.load(&compiled, &mut NoHost).unwrap();
    runtime
        .sync(id(2), "Sloppy", &ComponentData::default(), &mut NoHost)
        .unwrap();
    let failures = runtime.tick(0.1, &mut NoHost).failures;
    assert_eq!(failures.len(), 1, "{failures:?}");
    assert!(!failures[0].stopped);
    assert!(failures[0].at.message.contains("Hit carries 2 values"));
    assert!(runtime.tick(0.1, &mut NoHost).failures.is_empty());
    assert_eq!(
        runtime.values(id(2), "Sloppy").unwrap().get("ticks"),
        Some(&Value::Int(2))
    );
}

#[test]
fn a_lua_field_can_t_hide_the_entity_or_a_method() {
    for name in ["me", "update", "on_Hit", "__index"] {
        let lua = format!("component('Bad', {{ {{ '{name}', 'int', 0 }} }})");
        let problems = compiled("", &[("bad.lua", &lua)]).unwrap_err();
        assert!(problems[0].message.contains("can't be called"), "{name}");
        assert_eq!(problems[0].file, "bad.lua");
    }
}

#[test]
fn a_left_out_component_doesn_t_run_but_a_rust_one_of_that_name_does() {
    let lua = "local A = component('Armour', {})\nfunction A:update(dt) solis.say('lua') end";
    let compiled = compiled("", &[("armour.lua", lua)]).unwrap();
    let mut runtime = Runtime::new().unwrap();
    runtime.load(&compiled, &mut NoHost).unwrap();
    runtime.leave_out(["Armour".to_string()].into());
    assert!(!runtime.has_component("Armour"));
    runtime
        .sync(id(2), "Armour", &ComponentData::default(), &mut NoHost)
        .unwrap();
    let mut said = Said::default();
    runtime.tick(0.1, &mut said);
    assert!(said.0.is_empty(), "{:?}", said.0);
}

#[test]
fn a_failure_with_no_place_reads_cleanly() {
    let at = crate::ticking::plain("too many".into());
    assert_eq!(at.to_string(), "too many");
}

#[test]
fn vec3_values_are_luau_vectors_in_lust_and_lua_alike() {
    let main = "get \"solis/entity\":[positionOf, moveTo];\n\
        component Walker {\n    has {\n        vec3 step = [1, 0, 0];\n    }\n\
        \x20   method update(float dt) {\n        moveTo(me, positionOf(me) + step);\n    }\n}\n\
        pass Walker;\nend\n";
    let lua = "local A = component('Aimer', { { 'aim', 'vec3', vec3(0, 0, 1) } })\n\
               function A:update(dt)\n\
                 assert(type(self.aim) == 'vector', type(self.aim))\n\
                 self.aim = self.aim * 2\n\
               end";
    let compiled = compiled(main, &[("aim.lua", lua)]).unwrap();
    let mut runtime = Runtime::new().unwrap();
    runtime.load(&compiled, &mut NoHost).unwrap();
    let me = id(2);
    let mut step = ComponentData::default();
    step.set("step", Value::floats(&[0.0, 2.0, 0.0]));
    runtime.sync(me, "Walker", &step, &mut NoHost).unwrap();
    runtime
        .sync(me, "Aimer", &ComponentData::default(), &mut NoHost)
        .unwrap();
    assert!(runtime.tick(0.1, &mut NoHost).failures.is_empty());
    assert_eq!(
        runtime.values(me, "Aimer").unwrap().get("aim"),
        Some(&Value::floats(&[0.0, 0.0, 2.0]))
    );
    assert_eq!(
        runtime.values(me, "Walker").unwrap().get("step"),
        Some(&Value::floats(&[0.0, 2.0, 0.0]))
    );
}
