//! Game code that tries to break the editor (the stability review of
//! Phase 4): each of these once froze it, crashed it, or filled its
//! memory or Console. Now each ends in a failure, in time.

use super::*;
use crate::code::Compiled;
use crate::host::NoHost;
use std::time::{Duration, Instant};

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
    }
}

/// Runs `run`, and checks it ended well within a few seconds.
fn quickly<R>(run: impl FnOnce() -> R) -> R {
    let start = Instant::now();
    let result = run();
    assert!(
        start.elapsed() < Duration::from_secs(5),
        "took {:?}",
        start.elapsed()
    );
    result
}

/// Counts what's said.
#[derive(Default)]
struct Said(Vec<String>);

impl Host for Said {
    fn say(&mut self, words: &str) {
        self.0.push(words.to_string());
    }
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
}

const SPIN: &str = "local function spin() while true do end end\n";

#[test]
fn metatables_that_never_end_stop_in_time_wherever_solis_meets_them() {
    // A field list whose entries loop when read.
    let mut runtime = Runtime::new().unwrap();
    let fields = format!("{SPIN}component('S', {{ setmetatable({{}}, {{ __index = spin }}) }})");
    let result = quickly(|| runtime.load(&code("", &[("s.lua", &fields)]), &mut NoHost));
    assert!(result.is_err());
    // A class whose lookups loop: its update can't even be found.
    let class = format!(
        "{SPIN}local S = component('S', {{ {{ 'speed', 'float', 1 }} }})\n\
         S.__index = function() spin() end\n\
         S.__newindex = function() spin() end"
    );
    runtime
        .load(&code("", &[("s.lua", &class)]), &mut NoHost)
        .unwrap();
    let me = id(3);
    runtime
        .sync(me, "S", &ComponentData::default(), &mut NoHost)
        .unwrap();
    let ticked = quickly(|| runtime.tick(0.1, &mut NoHost));
    assert_eq!(ticked.failures.len(), 1, "{ticked:?}");
    // Changing a field writes it raw: the looping __newindex never runs.
    let mut faster = ComponentData::default();
    faster.set("speed", reflect::Value::Float(5.0));
    quickly(|| runtime.sync(me, "S", &faster, &mut NoHost)).unwrap();
    assert_eq!(
        runtime.values(me, "S").unwrap().get("speed"),
        Some(&reflect::Value::Float(5.0))
    );
}

#[test]
fn a_file_can_t_break_the_others_or_the_lust() {
    let vandal = "pcall(setmetatable, _G, { __newindex = function() while true do end end })\n\
                  pairs = function() return function() while true do end end end\n\
                  ipairs = nil\n\
                  pcall(function() table.sort = nil end)\n\
                  pcall(function() solis.say = nil end)\n\
                  component('Vandal', {})";
    let lust = "get \"solis/world\":[say];\n\
        component Counter {\n    has {\n        int ticks = 0;\n    }\n\
        \x20   method update(float dt) {\n        ticks = ticks + 1;\n        say(\"tick\");\n    }\n}\n\
        pass Counter;\nend\n";
    let spinner = "local S = component('Spinner', { { 'n', 'int', 0 } })\n\
                   function S:update(dt) for _ in pairs({ 1 }) do self.n = self.n + 1 end end";
    let both = code(lust, &[("a.lua", vandal), ("b.lua", spinner)]);
    let mut runtime = Runtime::new().unwrap();
    runtime.load(&both, &mut NoHost).unwrap();
    let me = id(4);
    for name in ["Counter", "Spinner", "Vandal"] {
        runtime
            .sync(me, name, &ComponentData::default(), &mut NoHost)
            .unwrap();
    }
    let mut said = Said::default();
    let ticked = quickly(|| runtime.tick(0.1, &mut said));
    assert!(ticked.failures.is_empty(), "{ticked:?}");
    assert_eq!(said.0, ["tick"]);
    // Swapping in the same code again (Lust's swap uses pairs) still works.
    quickly(|| runtime.swap(&both, &mut NoHost)).unwrap();
    assert_eq!(
        runtime.values(me, "Spinner").unwrap().get("n"),
        Some(&reflect::Value::Int(1))
    );
}

#[test]
fn a_lust_starting_value_that_never_ends_stops_the_load_in_time() {
    let lust = "method int spin() {\n    loop {\n    }\n    return 1;\n}\n\
        component Gun {\n    has {\n        int ammo = spin();\n    }\n}\n\
        pass Gun;\nend\n";
    let mut runtime = Runtime::new().unwrap();
    let failure = quickly(|| runtime.load(&code(lust, &[]), &mut NoHost)).unwrap_err();
    assert!(failure.at.message.contains("longer than"), "{failure}");
}

#[test]
fn many_slow_components_cut_the_tick_short_and_ask_for_a_pause() {
    let slow = "local S = component('Slow', {})\n\
                function S:update(dt)\n\
                  local start = os.clock()\n\
                  while os.clock() - start < 0.09 do end\n\
                end";
    let mut runtime = Runtime::new().unwrap();
    runtime
        .load(&code("", &[("slow.lua", slow)]), &mut NoHost)
        .unwrap();
    for n in 1..=30 {
        runtime
            .sync(id(n), "Slow", &ComponentData::default(), &mut NoHost)
            .unwrap();
    }
    let start = Instant::now();
    let ticked = runtime.tick(0.1, &mut NoHost);
    let took = start.elapsed();
    assert!(took < TICK_LIMIT + Duration::from_millis(150), "{took:?}");
    assert!(ticked.pause.is_some(), "{ticked:?}");
    // None was to blame on its own, so none stopped.
    assert!(ticked.failures.is_empty(), "{ticked:?}");
}

#[test]
fn what_game_code_says_and_sends_is_kept_small() {
    let loud = "local S = component('Loud', {})\n\
                function S:update(dt)\n\
                  local big = string.rep('x', 1000000)\n\
                  for i = 1, 1000 do solis.say(big) end\n\
                end";
    let mut runtime = Runtime::new().unwrap();
    runtime
        .load(&code("", &[("loud.lua", loud)]), &mut NoHost)
        .unwrap();
    runtime
        .sync(id(2), "Loud", &ComponentData::default(), &mut NoHost)
        .unwrap();
    let mut said = Said::default();
    runtime.tick(0.1, &mut said);
    assert_eq!(said.0.len(), crate::host::MAX_SAID_LINES + 1);
    assert!(
        said.0
            .iter()
            .all(|l| l.len() <= crate::host::MAX_SAID_BYTES + 4)
    );
    assert!(said.0.last().unwrap().contains("800 more lines"));
    // Sending: past the queue's limits, a send fails, in the sender.
    let sender = "local S = component('Sender', {})\n\
                  function S:update(dt)\n\
                    local big = string.rep('x', 4000000)\n\
                    for i = 1, 100 do solis.send(self.me, 'Big', big) end\n\
                  end";
    runtime
        .load(&code("", &[("send.lua", sender)]), &mut NoHost)
        .unwrap();
    runtime
        .sync(id(2), "Sender", &ComponentData::default(), &mut NoHost)
        .unwrap();
    let ticked = runtime.tick(0.1, &mut NoHost);
    assert_eq!(ticked.failures.len(), 1, "{ticked:?}");
    assert!(ticked.failures[0].at.message.contains("MB of text"));
    // A huge error message is cut too.
    let thrower = "local S = component('Thrower', {})\n\
                   function S:update(dt) error(string.rep('y', 1000000)) end";
    runtime
        .load(&code("", &[("throw.lua", thrower)]), &mut NoHost)
        .unwrap();
    runtime
        .sync(id(2), "Thrower", &ComponentData::default(), &mut NoHost)
        .unwrap();
    let ticked = runtime.tick(0.1, &mut NoHost);
    assert!(ticked.failures[0].at.message.len() < 1_100);
}

#[test]
fn a_handler_that_always_fails_is_said_once_until_the_code_changes() {
    let noisy = "local S = component('Noisy', {})\n\
                 function S:update(dt) for i = 1, 500 do solis.send(self.me, 'Poke') end end\n\
                 function S:on_Poke() error('no') end";
    let program = code("", &[("noisy.lua", noisy)]);
    let mut runtime = Runtime::new().unwrap();
    runtime.load(&program, &mut NoHost).unwrap();
    runtime
        .sync(id(2), "Noisy", &ComponentData::default(), &mut NoHost)
        .unwrap();
    assert_eq!(runtime.tick(0.1, &mut NoHost).failures.len(), 1);
    assert!(runtime.tick(0.1, &mut NoHost).failures.is_empty());
    runtime.swap(&program, &mut NoHost).unwrap();
    assert_eq!(runtime.tick(0.1, &mut NoHost).failures.len(), 1);
}

/// A Rust component that panics wherever it can.
struct Fragile;

impl crate::RustComponent for Fragile {
    fn values(&self) -> ComponentData {
        panic!("values broke")
    }
    fn set(&mut self, _data: &ComponentData) {
        panic!("set broke")
    }
}

#[test]
fn a_rust_component_that_panics_anywhere_fails_without_a_crash() {
    let schema = reflect::ComponentSchema {
        name: "Fragile".into(),
        help: String::new(),
        fields: vec![reflect::FieldInfo::new(
            "n",
            "N",
            reflect::FieldKind::Int,
            reflect::Value::Int(0),
        )],
    };
    let mut runtime = Runtime::new().unwrap();
    runtime.add_rust(RustKind {
        schema: schema.clone(),
        make: |_| Box::new(Fragile),
    });
    runtime.add_rust(RustKind {
        schema: reflect::ComponentSchema {
            name: "Broken".into(),
            ..schema
        },
        make: |_| panic!("make broke"),
    });
    let me = id(2);
    let failure = runtime
        .sync(me, "Broken", &ComponentData::default(), &mut NoHost)
        .unwrap_err();
    assert!(failure.at.message.contains("make broke"));
    runtime
        .sync(me, "Fragile", &ComponentData::default(), &mut NoHost)
        .unwrap();
    assert!(runtime.values(me, "Fragile").is_none());
    let mut more = ComponentData::default();
    more.set("n", reflect::Value::Int(3));
    let failure = runtime.sync(me, "Fragile", &more, &mut NoHost).unwrap_err();
    assert!(failure.at.message.contains("set broke"));
}
