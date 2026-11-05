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
        match lust::to_lua_with(
            &lust::Engine {
                folders: crate::describe::engine_folders(&files),
            },
            "main.lst",
            &files,
        ) {
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
        bytecode: Default::default(),
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
    fn cast_ray(
        &self,
        from: glam::Vec3,
        _direction: glam::Vec3,
        reach: f32,
    ) -> Option<physics::Hit> {
        // Something two metres below, within reach; nothing past it.
        (reach >= 2.0).then(|| physics::Hit {
            entity: id(9),
            distance: 2.0,
            point: from - glam::Vec3::Z * 2.0,
            normal: glam::Vec3::Z,
        })
    }
}

#[test]
fn a_ray_gives_back_all_its_answers_at_once_in_lust_and_lua() {
    let main = "fetch \"solis/world\":World;\nfetch \"solis/physics\":[Physics, RayHit];\n\
        component Feeler {\n    method update(float dt) {\n\
        RayHit near = Physics:castRay(position(), [0, 0, -1], 1);\n\
        RayHit far = Physics:castRay(position(), [0, 0, -1], 5);\n\
        World:say(near.found.text() + \" \" + near.distance.text() + \" \" + (near.entity == nothing).text());\n\
        World:say(far.found.text() + \" \" + far.distance.text() + \" \" + far.entity:name() + \" \" + far.normal.z.text());\n\
        }\n}\npass Feeler;\nend\n";
    let lua = "local Probe = component(\"Probe\", {})\n\
        function Probe:update(dt)\n\
            local hit = solis.Physics:castRay(vec3(0, 0, 0), vec3(0, 0, -1), 5)\n\
            solis.World:say(\"lua \" .. tostring(hit.found) .. \" \" .. hit.distance .. \" \" .. hit.entity)\n\
        end\n";
    let mut runtime = Runtime::new().unwrap();
    runtime
        .load(&code(main, &[("probe.lua", lua)]), &mut NoHost)
        .unwrap();
    let mut log = Log::default();
    let me = id(4);
    log.at.insert(me, glam::Vec3::ZERO);
    runtime
        .sync(me, "Feeler", &ComponentData::default(), &mut log)
        .unwrap();
    runtime
        .sync(me, "Probe", &ComponentData::default(), &mut log)
        .unwrap();
    let failures = runtime.tick(0.5, &mut log).failures;
    assert!(failures.is_empty(), "{failures:?}");
    assert!(
        log.said.contains(&"false -1.0 true".to_string()),
        "{:?}",
        log.said
    );
    assert!(
        log.said.contains(&"true 2.0 Cart 1.0".to_string()),
        "{:?}",
        log.said
    );
    assert!(
        log.said.contains(&format!("lua true 2 {}", id(9))),
        "{:?}",
        log.said
    );
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
    solis.entity.moveTo(self.me, solis.entity.position(self.me) + vec3(1, 0, 0) * self.speed * dt)
    if self.turns == 1 then
        solis.World:say(solis.entity.name(self.me) .. " spins at " .. solis.World:time())
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
    let lust = "fetch \"solis/world\":World;\n\n\
        event Hit(entity by, float damage);\n\
        event Healed(int amount);\n\
        component Health {\n\
            has {\n\
                float hp = 10;\n\
            }\n\
            on Hit(entity by, float damage) {\n\
                hp = hp - damage;\n\
                World:say(\"lust hp \" + hp.text());\n\
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
    solis.World:say("lua healed " .. amount)
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

/// Keeps what an animator's file asked, and says it's walking.
#[derive(Default)]
struct Animated {
    set: Vec<(EntityId, String, crate::AnimatorValue)>,
    jumps: Vec<(EntityId, String)>,
    speeds: Vec<(EntityId, f32)>,
}

impl Host for Animated {
    fn say(&mut self, _words: &str) {}
    fn time(&self) -> f64 {
        0.0
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
    fn animator_set(&mut self, which: EntityId, name: &str, value: crate::AnimatorValue) {
        self.set.push((which, name.to_string(), value));
    }
    fn animator_state(&self, _which: EntityId) -> Option<String> {
        Some("Use tool".into())
    }
    fn animator_jump(&mut self, which: EntityId, state: &str) {
        self.jumps.push((which, state.to_string()));
    }
    fn animator_speed(&mut self, which: EntityId, scale: f32) {
        self.speeds.push((which, scale));
    }
}

#[test]
fn a_component_drives_its_animator_through_the_animator_s_own_file() {
    use crate::animator_code::{AnimatorNames, ParamKind, animator_file};
    let farmer = AnimatorNames {
        path: "farmer".into(),
        parameters: vec![
            ("moving".into(), ParamKind::Switch),
            ("speed".into(), ParamKind::Number),
            ("facing".into(), ParamKind::Direction),
            ("swing".into(), ParamKind::Trigger),
        ],
        states: vec!["Idle".into(), "Walk".into(), "Use tool".into()],
    };
    let (path, text) = animator_file("Test 1.0", "solis", &farmer).0.unwrap();
    let mut files = crate::describe::descriptions("Test 1.0", "solis");
    files.push((path, text));
    files.push((
        "main.lst".into(),
        "fetch \"solis/animators/farmer\":[FarmerAnimator, FarmerState];\n\
         component Farmer {\n    has {\n        FarmerAnimator anim;\n    }\n\
         \x20   method update(float dt) {\n        anim:setMoving(true);\n        anim:setSpeed(2.5);\n\
         \x20       anim:setFacing([0, 3, 0]);\n        anim:swing();\n\
         \x20       is anim:current() == FarmerState.UseTool? then {\n            anim:jumpTo(FarmerState.Idle);\n        }\n\
         \x20       anim:setPlaybackSpeed(2);\n    }\n}\npass Farmer;\nend\n"
            .into(),
    ));
    let chunk = match lust::to_lua_with(
        &lust::Engine {
            folders: crate::describe::engine_folders(&files),
        },
        "main.lst",
        &files,
    ) {
        Ok(chunk) => chunk,
        Err(problems) => panic!("{}", problems[0]),
    };
    let compiled = Compiled {
        lust: Some(chunk),
        lua: Vec::new(),
        schemas: Vec::new(),
        places: Default::default(),
        bytecode: Default::default(),
    };
    let mut runtime = Runtime::new().unwrap();
    runtime.load(&compiled, &mut NoHost).unwrap();
    // The engine's field isn't something Properties shows.
    let schema = runtime
        .schemas()
        .iter()
        .find(|s| s.name == "Farmer")
        .unwrap()
        .clone();
    assert!(schema.fields.is_empty(), "{:?}", schema.fields);
    let me = id(4);
    let mut host = Animated::default();
    runtime
        .sync(me, "Farmer", &ComponentData::default(), &mut host)
        .unwrap();
    let failures = runtime.tick(0.5, &mut host).failures;
    assert!(failures.is_empty(), "{failures:?}");
    assert_eq!(
        host.set,
        [
            (me, "moving".to_string(), crate::AnimatorValue::Switch(true)),
            (me, "speed".to_string(), crate::AnimatorValue::Number(2.5)),
            (
                me,
                "facing".to_string(),
                crate::AnimatorValue::Direction(glam::Vec3::new(0.0, 3.0, 0.0))
            ),
            (me, "swing".to_string(), crate::AnimatorValue::Fire),
        ]
    );
    assert_eq!(host.jumps, [(me, "Idle".to_string())]);
    assert_eq!(host.speeds, [(me, 2.0)]);
}
