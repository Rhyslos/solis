//! A Lua author's mistakes with Solis's own calls: each says the file and
//! line of the author's code, in plain words, never the Lua library's
//! (BUG-49).

use super::*;
use crate::code::{CodeProblem, Compiled};
use crate::host::{Host, NoHost, call_with};
use crate::lua_components::lua_problem;
use crate::luau::{Limits, Luau};
use crate::{Code, Runtime};
use reflect::{ComponentData, EntityId};
use std::time::Duration;

/// Words only the Lua library uses: none may reach a game's author.
const LIBRARY_WORDS: [&str; 6] = [
    "error converting",
    "bad argument",
    "destructed",
    "callback",
    "userdata",
    "f64",
];

fn id(n: u64) -> EntityId {
    EntityId(reflect::Id::from_raw(n).unwrap())
}

/// The game's code with these Lua files and no Lust.
fn lua_code(files: &[(&str, &str)]) -> Compiled {
    Compiled {
        lust: None,
        lua: files
            .iter()
            .map(|(path, text)| ((*path).to_string(), (*text).to_string()))
            .collect(),
        schemas: Vec::new(),
        places: Default::default(),
        bytecode: Default::default(),
        screens: None,
    }
}

/// Keeps what's said.
#[derive(Default)]
struct Heard(Vec<String>);

impl Host for Heard {
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

/// What the component Gun's `update` fails with, when its body is `body`
/// (which starts on line 3 of parts/gun.lua).
fn mistake(body: &str) -> CodeProblem {
    let text =
        format!("local Gun = component(\"Gun\", {{}})\nfunction Gun:update(dt)\n{body}\nend\n");
    let mut runtime = Runtime::new().unwrap();
    // The time limit is one of the mistakes tried: a short one, so the
    // test is quick. Nothing else here runs long enough to meet it.
    runtime.set_limits(Limits {
        call: Duration::from_millis(50),
        ..Limits::unhurried()
    });
    runtime
        .load(&lua_code(&[("parts/gun.lua", &text)]), &mut NoHost)
        .unwrap();
    runtime
        .sync(id(7), "Gun", &ComponentData::default(), &mut NoHost)
        .unwrap();
    let mut failures = runtime.tick(0.1, &mut NoHost).failures;
    assert_eq!(failures.len(), 1, "{body}: {failures:?}");
    failures.remove(0).at
}

#[test]
fn a_value_of_the_wrong_kind_is_said_by_the_call_s_name_with_file_and_line() {
    let colon = " Is a value missing, its name misspelt, or the colon before the call's name \
                 written as a dot?";
    let cases: &[(&str, String)] = &[
        // Nothing where a value should be: missing, misspelt, or a dot for
        // the colon (which makes every value the one before it).
        (
            "solis.World:say(nil)",
            format!("solis.World:say(words) wants text for words, and was given nothing (nil).{colon}"),
        ),
        (
            "solis.World:say()",
            format!("solis.World:say(words) wants text for words, and was given nothing (nil).{colon}"),
        ),
        (
            "solis.World.say(\"hello\")",
            format!("solis.World:say(words) wants text for words, and was given nothing (nil).{colon}"),
        ),
        // Too few values.
        (
            "solis.World:randomRange(1)",
            format!("solis.World:randomRange(low, high) wants a number for high, and was given nothing (nil).{colon}"),
        ),
        (
            "solis.Animators:setNumber(self.me, missing, 2)",
            format!("solis.Animators:setNumber(owner, name, value) wants text for name, and was given nothing (nil).{colon}"),
        ),
        // Each kind a native takes, given each kind it can't be read from.
        (
            "solis.World:say({})",
            "solis.World:say(words) wants text for words, and was given a table.".into(),
        ),
        (
            "solis.World:say(true)",
            "solis.World:say(words) wants text for words, and was given true or false.".into(),
        ),
        (
            "solis.World:randomRange(\"a\", 2)",
            "solis.World:randomRange(low, high) wants a number for low, and was given text that isn't a number.".into(),
        ),
        (
            "solis.World:setGameSpeed(vec3(1, 2, 3))",
            "solis.World:setGameSpeed(speed) wants a number for speed, and was given a vec3.".into(),
        ),
        (
            "solis.Physics:castRay(vec3(0, 0, 0), vec3(0, 0, 1), function() end)",
            "solis.Physics:castRay(from, direction, reach) wants a number for reach, and was given a function.".into(),
        ),
        (
            "solis.Maps:terrain(\"farm\", 1.5, 0, 0)",
            "solis.Maps:terrain(tileMap, layer, x, y) wants a whole number for layer, and was given a number that isn't whole, or is too big.".into(),
        ),
        (
            "solis.Maps:terrain(\"farm\", 0, \"2.5\", 0)",
            "solis.Maps:terrain(tileMap, layer, x, y) wants a whole number for x, and was given text that isn't a whole number.".into(),
        ),
        (
            "solis.Maps:terrain(\"farm\", 0, 0, \"north\")",
            "solis.Maps:terrain(tileMap, layer, x, y) wants a whole number for y, and was given text that isn't a number.".into(),
        ),
        (
            "solis.entity.turnTo(self.me, 5)",
            "solis.entity.turnTo(target, facing) wants a rotation (what solis.entity.rotation gives) for facing, and was given a number.".into(),
        ),
        (
            "solis.entity.turnTo(self.me)",
            "solis.entity.turnTo(target, facing) wants a rotation (what solis.entity.rotation gives) for facing, and was given nothing (nil). Is a value missing, or its name misspelt?".into(),
        ),
        // Text that isn't text.
        (
            "solis.Input:held(\"\\255\\254\")",
            "solis.Input:held(action) was given text it can't read: text for Solis must be UTF-8".into(),
        ),
        // Solis's own Lua in front of a native.
        (
            "solis.send(self.me, nil, 2.5)",
            "solis.send(to, name, …) wants the event's name as text after the entity it's for, such as solis.send(self.me, \"Hit\", 2.5)".into(),
        ),
        (
            "math.random(\"a\")",
            "math.random wants numbers: math.random() for 0 to 1, math.random(6) for 1 to 6, math.random(2, 12) for 2 to 12".into(),
        ),
        (
            "math.randomseed({})",
            "math.randomseed wants a number, such as math.randomseed(42)".into(),
        ),
    ];
    for (body, message) in cases {
        let at = mistake(body);
        assert_eq!(at.message, *message, "{body}");
        assert_eq!((at.file.as_str(), at.line), ("parts/gun.lua", 3), "{body}");
    }
}

#[test]
fn a_mistake_with_no_place_of_its_own_is_put_where_the_author_s_code_called_from() {
    // (the body, the line it fails on, what the message holds)
    let cases = [
        // The call is inside a helper: the helper's line, not its caller's.
        (
            "local function aim()\n    solis.World:say(nil)\nend\naim()",
            4,
            "solis.World:say(words) wants text",
        ),
        // Caught and thrown again as it was.
        (
            "local ok, why = pcall(solis.World.say, solis.World, nil)\nerror(why, 0)",
            3,
            "solis.World:say(words) wants text",
        ),
        // Called from inside one of Lua's own functions.
        (
            "local t = { 3, 1, 2 }\ntable.sort(t, function(a, b)\n    solis.World:say(a < b)\n    return a < b\nend)",
            5,
            "was given true or false",
        ),
        // The author's own errors that leave the place out.
        ("error(\"out of ammo\", 0)", 3, "out of ammo"),
        ("error()", 3, "nil"),
        // An event that can't be sent: Solis's own words, from a native.
        (
            "solis.send(self.me, string.rep(\"n\", 300))",
            3,
            "an event's name is at most",
        ),
        // Stopped by the time limit.
        ("while true do end", 3, "took longer than 50 ms"),
    ];
    for (body, line, holds) in cases {
        let at = mistake(body);
        assert_eq!(
            (at.file.as_str(), at.line),
            ("parts/gun.lua", line),
            "{body}: {at}"
        );
        assert!(at.message.contains(holds), "{body}: {at}");
        for word in LIBRARY_WORDS {
            assert!(!at.message.contains(word), "{body}: {at}");
        }
    }
}

#[test]
fn a_native_kept_in_a_variable_says_what_to_write_instead_and_where() {
    // A common Lua habit: the natives are lent only while game code runs,
    // so the one kept when the file loaded is gone by the first tick.
    let kept = "local say = solis.World.say\n\
                local Keeper = component(\"Keeper\", {})\n\
                function Keeper:update(dt)\n\
                \x20   say(solis.World, \"hello\")\n\
                end\n";
    // Keeping the object is fine, as the message says.
    let object = "local World = solis.World\n\
                  local Asker = component(\"Asker\", {})\n\
                  function Asker:update(dt)\n\
                  \x20   World:say(\"asked each time\")\n\
                  end\n";
    let mut runtime = Runtime::new().unwrap();
    runtime
        .load(
            &lua_code(&[("parts/keep.lua", kept), ("parts/object.lua", object)]),
            &mut NoHost,
        )
        .unwrap();
    for name in ["Keeper", "Asker"] {
        runtime
            .sync(id(7), name, &ComponentData::default(), &mut NoHost)
            .unwrap();
    }
    let mut heard = Heard::default();
    let failures = runtime.tick(0.1, &mut heard).failures;
    assert_eq!(heard.0, ["asked each time"]);
    assert_eq!(failures.len(), 1, "{failures:?}");
    let at = &failures[0].at;
    assert_eq!(failures[0].component, "Keeper");
    assert_eq!((at.file.as_str(), at.line), ("parts/keep.lua", 4));
    assert_eq!(at.message, KEPT);
    assert!(at.message.contains("solis.World:say(\"hello\")"));
    assert!(at.message.contains("local World = solis.World"));
    for word in LIBRARY_WORDS {
        assert!(!at.message.contains(word), "{at}");
    }
    // Caught and said by the game itself, the library's words still don't
    // show: thrown again as text, it has its place on its first line.
    let caught = "local say = solis.World.say\n\
                  local Catcher = component(\"Catcher\", {})\n\
                  function Catcher:update(dt)\n\
                  \x20   local ok, why = pcall(say, solis.World, \"hello\")\n\
                  \x20   error(tostring(why))\n\
                  end\n";
    let mut runtime = Runtime::new().unwrap();
    runtime
        .load(&lua_code(&[("parts/catch.lua", caught)]), &mut NoHost)
        .unwrap();
    runtime
        .sync(id(7), "Catcher", &ComponentData::default(), &mut NoHost)
        .unwrap();
    let failures = runtime.tick(0.1, &mut NoHost).failures;
    assert_eq!(failures[0].at.message, KEPT, "{failures:?}");
    assert_eq!(failures[0].at.file, "parts/catch.lua");
}

#[test]
fn every_native_given_every_wrong_kind_of_value_names_itself_and_the_line() {
    // Each native Lua can reach, with each kind of value in each place:
    // whatever it refuses, it refuses by its own name, on the author's
    // line. The other values are 1, which reads as a number, as text and
    // as true.
    let luau = Luau::new().unwrap();
    luau.clock.set_limits(Limits::unhurried());
    let mut host = NoHost;
    let mut refused = 0;
    call_with(&luau, None, &mut host, None, || {
        let lua = &luau.lua;
        let wrong: Vec<(&str, LuaValue)> = vec![
            ("nothing", LuaValue::Nil),
            ("true", LuaValue::Boolean(true)),
            ("a fraction", LuaValue::Number(1.5)),
            ("a huge number", LuaValue::Number(1e300)),
            ("text", LuaValue::String(lua.create_string("north")?)),
            (
                "unreadable text",
                LuaValue::String(lua.create_string([0xff, 0xfe])?),
            ),
            ("a table", LuaValue::Table(lua.create_table()?)),
            (
                "a function",
                LuaValue::Function(lua.create_function(|_, ()| Ok(()))?),
            ),
            ("a vec3", LuaValue::Vector(mlua::Vector::new(1.0, 2.0, 3.0))),
        ];
        let mut ways = luau.ways()?;
        ways.sort();
        for way in &ways {
            let (call, first) = match way.as_slice() {
                [owner, name] if owner == "entity" => (format!("solis.entity.{name}"), "\"1\", "),
                [owner, name] => (format!("solis.{owner}:{name}"), ""),
                [name] => (format!("solis.{name}"), ""),
                _ => panic!("{way:?}"),
            };
            for place in 0..6 {
                let values: Vec<&str> = (0..6)
                    .map(|i| if i == place { "wrong" } else { "1" })
                    .collect();
                let text = format!("local wrong = ...\n{call}({first}{})\n", values.join(", "));
                let chunk = lua
                    .load(text)
                    .set_name("=parts/sweep.lua")
                    .set_environment(luau.env()?)
                    .into_function()?;
                for (what, value) in &wrong {
                    let Err(err) = chunk.call::<LuaValue>(value.clone()) else {
                        continue;
                    };
                    let at = lua_problem("other.lua", &err);
                    let tried = format!("{call}, value {} as {what}: {at}", place + 1);
                    refused += 1;
                    assert_eq!(
                        (at.file.as_str(), at.line),
                        ("parts/sweep.lua", 2),
                        "{tried}"
                    );
                    assert!(at.message.starts_with(&format!("{call}(")), "{tried}");
                    // The value is one the call's own writing lists.
                    assert!(!at.message.contains(" for value "), "{tried}");
                    for word in LIBRARY_WORDS {
                        assert!(!at.message.contains(word), "{tried}");
                    }
                }
            }
        }
        Ok(())
    })
    .unwrap();
    assert!(refused > 100, "only {refused} calls were refused");
}

#[test]
fn every_native_s_lust_lists_its_values_as_a_kind_and_a_name() {
    // The messages take a value's name from the native's Lust.
    for native in NATIVES {
        let inside = native.lust.split_once('(').unwrap().1;
        let inside = inside.rsplit_once(')').unwrap().0;
        let listed = if inside.trim().is_empty() {
            0
        } else {
            inside.split(',').count()
        };
        assert_eq!(values_of(native.lust).count(), listed, "{}", native.lust);
        for (kind, name) in values_of(native.lust) {
            let word = |text: &str| !text.is_empty() && text.chars().all(char::is_alphanumeric);
            assert!(word(kind) && word(name), "{}", native.lust);
        }
    }
    let say = NATIVES.iter().find(|n| n.key() == "World_say").unwrap();
    assert_eq!(written(say), "solis.World:say(words)");
    let face = NATIVES.iter().find(|n| n.key() == "facingAlong").unwrap();
    assert_eq!(written(face), "solis.facingAlong(direction)");
    let name = NATIVES.iter().find(|n| n.key() == "entity_name").unwrap();
    assert_eq!(written(name), "solis.entity.name(target)");
}

#[test]
fn the_same_mistake_in_lust_is_found_before_the_game_runs_with_its_line() {
    // Lust is checked as it's compiled: a value of the wrong kind never
    // reaches a native. (Rust game code is checked by its own compiler.)
    let mut files = crate::describe::descriptions("Test 1.0", "solis");
    files.push((
        "main.lst".into(),
        "fetch \"solis/world\":World;\ncomponent Gun {\n    method update(float dt) {\n        \
         World:say(5);\n    }\n}\npass Gun;\nend\n"
            .into(),
    ));
    let problems = crate::code::compile_within(
        &Code {
            files,
            lua: Vec::new(),
            notes: Vec::new(),
        },
        Limits::unhurried(),
    )
    .unwrap_err();
    assert_eq!(
        (problems[0].file.as_str(), problems[0].line),
        ("main.lst", 4),
        "{problems:?}"
    );
}
