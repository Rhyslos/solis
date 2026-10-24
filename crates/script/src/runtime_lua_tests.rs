//! Lua components, alone and with Lust's.

use super::*;
use crate::code::Compiled;
use crate::host::NoHost;
use reflect::{FieldKind, Value};

fn id(n: u64) -> EntityId {
    EntityId(reflect::Id::from_raw(n).unwrap())
}

/// The code for `lua` files, and Lust's `main.lst` if `main` isn't empty.
fn code(main: &str, lua: &[(&str, &str)]) -> Compiled {
    let lust = (!main.is_empty()).then(|| {
        let mut files = crate::describe::descriptions("Test 1.0", "solis");
        files.push(("main.lst".into(), main.to_string()));
        match lust::to_lua("main.lst", &files) {
            Ok(chunk) => chunk,
            Err(problems) => panic!("{}", problems[0]),
        }
    });
    Compiled {
        lust,
        lua: lua
            .iter()
            .map(|(p, t)| ((*p).to_string(), (*t).to_string()))
            .collect(),
        schemas: Vec::new(),
        places: Default::default(),
    }
}

/// Says go in a list; positions are kept.
#[derive(Default)]
struct Log {
    said: Vec<String>,
    at: std::collections::HashMap<EntityId, glam::Vec3>,
}

impl Host for Log {
    fn say(&mut self, words: &str) {
        self.said.push(words.to_string());
    }
    fn time(&self) -> f64 {
        2.0
    }
    fn find(&self, _name: &str) -> Option<EntityId> {
        None
    }
    fn position(&self, which: EntityId) -> Option<glam::Vec3> {
        self.at.get(&which).copied()
    }
    fn move_to(&mut self, which: EntityId, place: glam::Vec3) {
        self.at.insert(which, place);
    }
    fn rotation(&self, _which: EntityId) -> Option<glam::Quat> {
        None
    }
    fn turn_to(&mut self, _which: EntityId, _facing: glam::Quat) {}
    fn name(&self, _which: EntityId) -> Option<String> {
        Some("Cart".into())
    }
}

const SPINNER: &str = r#"
local Spinner = component("Spinner", {
    { "speed", "float", 2 },
    { "turns", "int", 0 },
    { "mode", "state", "Slow", { "Slow", "Fast" } },
    { "target", "entity" },
    { "aim", "vec3", vec3(0, 0, 1) },
    { "label", "string", "spinner" },
})

function Spinner:update(dt)
    self.turns = self.turns + 1
    solis.moveTo(self.me, solis.positionOf(self.me) + vec3(1, 0, 0) * self.speed * dt)
    if self.turns == 1 then
        solis.say(solis.nameOf(self.me) .. " spins at " .. solis.time())
    end
end
"#;

#[test]
fn a_lua_component_has_its_fields_in_order_and_runs_with_the_natives() {
    let mut runtime = Runtime::new().unwrap();
    runtime
        .load(&code("", &[("spin/spinner.lua", SPINNER)]), &mut NoHost)
        .unwrap();
    let schemas = runtime.schemas();
    assert_eq!(schemas.len(), 1);
    let names: Vec<&str> = schemas[0].fields.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, ["speed", "turns", "mode", "target", "aim", "label"]);
    let field = |n: &str| schemas[0].field(n).unwrap().clone();
    assert_eq!(field("speed").default, Value::Float(2.0));
    assert_eq!(field("turns").default, Value::Int(0));
    assert_eq!(
        field("mode").kind,
        FieldKind::Choice(vec!["Slow".into(), "Fast".into()])
    );
    assert_eq!(field("target").default, Value::Entity(None));
    assert_eq!(field("aim").default, Value::floats(&[0.0, 0.0, 1.0]));
    let me = id(4);
    let mut data = ComponentData::default();
    data.set("speed", Value::Float(4.0));
    let mut log = Log::default();
    log.at.insert(me, glam::Vec3::ZERO);
    runtime.sync(me, "Spinner", &data, &mut log).unwrap();
    assert!(runtime.tick(0.5, &mut log).failures.is_empty());
    assert_eq!(log.at[&me], glam::Vec3::new(2.0, 0.0, 0.0));
    assert_eq!(log.said, ["Cart spins at 2"]);
    let values = runtime.values(me, "Spinner").unwrap();
    assert_eq!(values.get("turns"), Some(&Value::Int(1)));
    assert_eq!(values.get("label"), Some(&Value::Text("spinner".into())));
}

#[test]
fn lua_and_lust_components_talk_through_the_same_events() {
    let lust = "get \"solis/world\":[say];\n\
        event Hit(entity by, float damage);\n\
        event Healed(int amount);\n\
        component Health {\n\
            has {\n\
                float hp = 10;\n\
            }\n\
            on Hit(entity by, float damage) {\n\
                hp = hp - damage;\n\
                say(\"lust hp \" + hp.text());\n\
                send Healed(1) to me;\n\
            }\n\
        }\n\
        pass Health;\nend\n";
    let lua = r#"
local Gun = component("Gun", {
    { "shots", "int", 0 },
    { "healed", "int", 0 },
})

function Gun:update(dt)
    if self.shots == 0 then
        self.shots = 1
        solis.send(self.me, "Hit", self.me, 2.5)
    end
end

function Gun:on_Healed(amount)
    self.healed = self.healed + amount
    solis.say("lua healed " .. amount)
end
"#;
    let mut runtime = Runtime::new().unwrap();
    runtime
        .load(&code(lust, &[("gun.lua", lua)]), &mut NoHost)
        .unwrap();
    let me = id(5);
    for name in ["Gun", "Health"] {
        runtime
            .sync(me, name, &ComponentData::default(), &mut NoHost)
            .unwrap();
    }
    let mut log = Log::default();
    let failures = runtime.tick(0.1, &mut log).failures;
    assert!(failures.is_empty(), "{failures:?}");
    assert_eq!(log.said, ["lust hp 7.5", "lua healed 1"]);
    assert_eq!(
        runtime.values(me, "Gun").unwrap().get("healed"),
        Some(&Value::Int(1))
    );
}

#[test]
fn new_lua_while_running_keeps_values_and_says_what_started_over() {
    let v1 = "local C = component(\"Counter\", { { \"n\", \"int\", 0 }, { \"step\", \"int\", 1 }, { \"tag\", \"string\", \"a\" } })\nfunction C:update(dt) self.n = self.n + self.step end\n";
    let v2 = "local C = component(\"Counter\", { { \"n\", \"int\", 0 }, { \"step\", \"float\", 10 }, { \"loud\", \"bool\", true } })\nfunction C:update(dt) self.n = self.n - 1 end\n";
    let mut runtime = Runtime::new().unwrap();
    runtime
        .load(&code("", &[("c.lua", v1)]), &mut NoHost)
        .unwrap();
    runtime
        .sync(id(6), "Counter", &ComponentData::default(), &mut NoHost)
        .unwrap();
    for _ in 0..3 {
        runtime.tick(0.1, &mut NoHost);
    }
    let swapped = runtime
        .swap(&code("", &[("c.lua", v2)]), &mut NoHost)
        .unwrap();
    assert_eq!(swapped.changed, ["Counter"]);
    assert!(
        swapped.notes.iter().any(|n| n.contains("step")),
        "{swapped:?}"
    );
    let values = runtime.values(id(6), "Counter").unwrap();
    assert_eq!(values.get("n"), Some(&Value::Int(3)), "kept");
    assert_eq!(
        values.get("step"),
        Some(&Value::Float(10.0)),
        "started over"
    );
    assert_eq!(values.get("loud"), Some(&Value::Bool(true)), "new");
    assert!(values.get("tag").is_none(), "dropped");
    runtime.tick(0.1, &mut NoHost);
    assert_eq!(
        runtime.values(id(6), "Counter").unwrap().get("n"),
        Some(&Value::Int(2)),
        "the new update"
    );
    // The Lua deleted: the component stops.
    let swapped = runtime.swap(&code("", &[]), &mut NoHost).unwrap();
    assert_eq!(swapped.removed, ["Counter"]);
    assert!(runtime.is_empty());
}

#[test]
fn mistakes_in_lua_say_their_file_and_line() {
    let mut runtime = Runtime::new().unwrap();
    let bad_kind = runtime
        .load(
            &code(
                "",
                &[(
                    "odd.lua",
                    "component(\"Odd\", { { \"x\", \"colour\", 1 } })\n",
                )],
            ),
            &mut NoHost,
        )
        .unwrap_err();
    assert_eq!(bad_kind.at.file, "odd.lua");
    assert!(bad_kind.at.message.contains("colour"), "{bad_kind}");
    let clash = runtime
        .load(
            &code(
                "component Gun {\n    has {\n        int n = 0;\n    }\n}\npass Gun;\nend\n",
                &[("gun.lua", "component(\"Gun\", {})\n")],
            ),
            &mut NoHost,
        )
        .unwrap_err();
    assert!(
        clash.at.message.contains("same name as a Lust component"),
        "{clash}"
    );
    runtime
        .load(
            &code(
                "",
                &[("broken.lua", "local B = component(\"Broken\", {})\nfunction B:update(dt)\n    local x = nil + 1\nend\n")],
            ),
            &mut NoHost,
        )
        .unwrap();
    runtime
        .sync(id(7), "Broken", &ComponentData::default(), &mut NoHost)
        .unwrap();
    let failures = runtime.tick(0.1, &mut NoHost).failures;
    assert_eq!(failures.len(), 1);
    assert_eq!(
        (failures[0].at.file.as_str(), failures[0].at.line),
        ("broken.lua", 3),
        "{}",
        failures[0]
    );
    // Written wrongly: found before anything runs, with its line.
    let mut found = crate::Code::default();
    found
        .lua
        .push(("typo.lua".into(), "local x = \nfunction (".into()));
    let problems = crate::check(&found);
    assert_eq!(
        (problems[0].file.as_str(), problems[0].line),
        ("typo.lua", 2)
    );
}
