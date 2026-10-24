//! Components written in Lua (decision 0003), for people who know Lua
//! from Roblox, Garry's Mod or modding. Each `.lua` file in the assets
//! makes components with `component`, the way Lua code usually makes
//! classes:
//!
//! ```lua
//! local Spinner = component("Spinner", {
//!     { "speed", "float", 2 },
//!     { "target", "entity" },
//! })
//!
//! function Spinner:update(dt)
//!     solis.say(solis.nameOf(self.me) .. " spins")
//! end
//!
//! function Spinner:on_Hit(by, damage)
//!     self.speed = self.speed + damage
//! end
//! ```
//!
//! Fields are listed in order: name, kind (the same kinds as Lust's), and
//! starting value. They run in the same Luau as the game's Lust, in the
//! same ticks and order, with the same native methods (under `solis.`),
//! and send (`solis.send(to, "Hit", by, 2.5)`) and handle (`on_Hit`) the
//! same events.

use crate::code::CodeProblem;
use crate::convert::{empty, from_lua, kind_of, label, to_lua};
use crate::runtime::Swapped;
use crate::ticking::{Instance, RunningMap};
use mlua::{Lua, Table, Value as LuaValue};
use reflect::{ComponentData, ComponentSchema, EntityId, FieldInfo, FieldKind};
use std::collections::BTreeMap;

/// Lua run once in every runtime: `component`; `vec3(x, y, z)`, a vector
/// that adds, takes away and scales; and `solis`, which reaches the native
/// methods while game code runs.
pub(crate) const PRELUDE: &str = r#"
SOLIS_MADE = {}
function component(name, fields)
  if type(name) ~= "string" then
    error("component needs a name first, such as component(\"Spinner\", { … })", 2)
  end
  if type(fields) ~= "table" then
    error("component " .. name .. " needs its fields as a list after its name", 2)
  end
  local class = { __name = name, __fields = fields }
  class.__index = class
  SOLIS_MADE[#SOLIS_MADE + 1] = class
  return class
end
local Vec3 = {}
Vec3.__index = Vec3
function vec3(x, y, z)
  return setmetatable({ x = x or 0, y = y or 0, z = z or 0 }, Vec3)
end
Vec3.__add = function(a, b) return vec3(a.x + b.x, a.y + b.y, a.z + b.z) end
Vec3.__sub = function(a, b) return vec3(a.x - b.x, a.y - b.y, a.z - b.z) end
Vec3.__unm = function(a) return vec3(-a.x, -a.y, -a.z) end
Vec3.__mul = function(a, b)
  if type(a) == "number" then return vec3(a * b.x, a * b.y, a * b.z) end
  return vec3(a.x * b, a.y * b, a.z * b)
end
Vec3.__div = function(a, b) return vec3(a.x / b, a.y / b, a.z / b) end
Vec3.__tostring = function(a) return "[" .. a.x .. ", " .. a.y .. ", " .. a.z .. "]" end
solis = setmetatable({}, { __index = function(_, key)
  local natives = rawget(_G, "LUST_NATIVES")
  local f = natives and key ~= "send_event" and natives[key]
  if not f then
    error("solis." .. tostring(key) .. " isn't something Solis offers to game code", 2)
  end
  return f
end })
function solis.send(to, name, ...)
  local natives = rawget(_G, "LUST_NATIVES")
  local send = natives and natives.send_event
  if not send then error("events can only be sent while the game runs", 2) end
  send(to, name, table.pack(...))
end
"#;

/// A field as a component keeps it: its key in the component's table, and
/// kind. (Lust's keys can differ from the names; Lua's are the names.)
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Field {
    pub(crate) key: String,
    pub(crate) name: String,
    pub(crate) kind: FieldKind,
}

/// A Lua component's class: what `component` made.
pub(crate) struct LuaClass {
    pub(crate) class: Table,
    pub(crate) fields: Vec<Field>,
    pub(crate) schema: ComponentSchema,
}

/// A Lua error's place and message, from its text (`spinner.lua:5:
/// attempt to …`): the file it names, or `file` if it names none.
pub(crate) fn lua_problem(file: &str, err: &mlua::Error) -> CodeProblem {
    let text = err.to_string();
    let mut first = text.lines().next().unwrap_or("").trim();
    for prefix in ["runtime error: ", "syntax error: ", "callback error: "] {
        first = first.strip_prefix(prefix).unwrap_or(first);
    }
    let found = first.match_indices(':').find_map(|(i, _)| {
        let rest = &first[i + 1..];
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        let message = rest[digits.len()..].strip_prefix(": ")?;
        let line = digits.parse::<u32>().ok()?;
        Some((first[..i].to_string(), line, message.to_string()))
    });
    match found {
        Some((path, line, message)) => CodeProblem {
            file: if path.ends_with(".lua") {
                path
            } else {
                file.to_string()
            },
            line,
            column: 0,
            message,
        },
        None => CodeProblem {
            file: file.to_string(),
            line: 0,
            column: 0,
            message: first.to_string(),
        },
    }
}

/// The classes the Lua files made, by component name, after running them
/// (the caller lends the natives and the time limit). The first mistake
/// found stops it.
pub(crate) fn read_classes(
    lua: &Lua,
    files: &[(String, String)],
    mut run: impl FnMut(&mlua::Function) -> mlua::Result<()>,
) -> Result<BTreeMap<String, LuaClass>, CodeProblem> {
    let mut classes = BTreeMap::new();
    for (path, text) in files {
        let problem = |err: mlua::Error| lua_problem(path, &err);
        lua.globals()
            .set("SOLIS_MADE", lua.create_table().map_err(problem)?)
            .map_err(problem)?;
        let chunk = lua
            .load(text)
            .set_name(format!("={path}"))
            .into_function()
            .map_err(problem)?;
        run(&chunk).map_err(problem)?;
        let made: Table = lua.globals().get("SOLIS_MADE").map_err(problem)?;
        for class in made.sequence_values::<Table>() {
            let class = class.map_err(problem)?;
            let read = read_class(path, class)?;
            if classes.contains_key(&read.schema.name) {
                return Err(CodeProblem {
                    file: path.clone(),
                    line: 0,
                    column: 0,
                    message: format!(
                        "two Lua components are called {}; each needs a name of its own",
                        read.schema.name
                    ),
                });
            }
            classes.insert(read.schema.name.clone(), read);
        }
    }
    Ok(classes)
}

/// One class's fields and schema, from its list of fields.
fn read_class(path: &str, class: Table) -> Result<LuaClass, CodeProblem> {
    let mistake = |message: String| CodeProblem {
        file: path.to_string(),
        line: 0,
        column: 0,
        message,
    };
    let problem = |err: mlua::Error| lua_problem(path, &err);
    let name: String = class.get("__name").map_err(problem)?;
    let list: Table = class.get("__fields").map_err(problem)?;
    let mut fields = Vec::new();
    let mut infos = Vec::new();
    for (i, entry) in list.sequence_values::<Table>().enumerate() {
        let entry = entry.map_err(|_| {
            mistake(format!(
                "{name}'s field {} isn't a list like {{ \"speed\", \"float\", 2 }}",
                i + 1
            ))
        })?;
        let field: String = entry
            .get(1)
            .map_err(|_| mistake(format!("{name}'s field {} has no name first", i + 1)))?;
        let kind_name: String = entry.get(2).map_err(|_| {
            mistake(format!(
                "{name}'s field {field} has no kind (such as \"float\")"
            ))
        })?;
        let options: Vec<String> = match entry.get::<LuaValue>(4).map_err(problem)? {
            LuaValue::Table(t) => t
                .sequence_values::<String>()
                .collect::<Result<_, _>>()
                .map_err(problem)?,
            _ => Vec::new(),
        };
        let Some(kind) = kind_of(&kind_name, options) else {
            return Err(mistake(format!(
                "{name}'s field {field} is a {kind_name}, which isn't a kind a component can \
                 hold: bool, int, float, string, state, vec3, rotation or entity"
            )));
        };
        let start: LuaValue = entry.get(3).map_err(problem)?;
        let default = match start {
            LuaValue::Nil => empty(&kind),
            given => from_lua(&kind, &given).ok_or_else(|| {
                mistake(format!(
                    "{name}'s field {field} starts with a value that isn't a {kind_name}"
                ))
            })?,
        };
        infos.push(FieldInfo::new(
            &field,
            &label(&field),
            kind.clone(),
            default,
        ));
        fields.push(Field {
            key: field.clone(),
            name: field,
            kind,
        });
    }
    Ok(LuaClass {
        class,
        fields,
        schema: ComponentSchema {
            name: name.clone(),
            help: format!("{name}, from the game's Lua ({path})."),
            fields: infos,
        },
    })
}

/// A new running Lua component of `class` on `me`: its starting values,
/// then the area's `data` where it fits.
pub(crate) fn make(
    lua: &Lua,
    class: &LuaClass,
    me: EntityId,
    data: &ComponentData,
) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    table.set("me", me.to_string())?;
    for (field, info) in class.fields.iter().zip(&class.schema.fields) {
        let value = data
            .get(&field.name)
            .and_then(|v| to_lua(lua, &field.kind, v))
            .or_else(|| to_lua(lua, &field.kind, &info.default));
        if let Some(value) = value {
            table.set(field.key.as_str(), value)?;
        }
    }
    table.set_metatable(Some(class.class.clone()));
    Ok(table)
}

/// Gives every running Lua component the new `classes` in place: a field
/// kept by name and kind keeps its value; a field whose kind changed, or a
/// new one, takes its starting value (noted); a removed field is dropped;
/// a component no longer in the code stops. What changed goes in
/// `swapped`.
pub(crate) fn swap_classes(
    lua: &Lua,
    old: &BTreeMap<String, LuaClass>,
    new: &BTreeMap<String, LuaClass>,
    running: &mut RunningMap,
    swapped: &mut Swapped,
) -> mlua::Result<()> {
    for (name, class) in new {
        match old.get(name) {
            None => swapped.added.push(name.clone()),
            Some(before) if before.fields != class.fields => swapped.changed.push(name.clone()),
            Some(_) => {}
        }
    }
    swapped
        .removed
        .extend(old.keys().filter(|n| !new.contains_key(*n)).cloned());
    let mut gone = Vec::new();
    for (key, component) in running.iter_mut() {
        let Instance::Lua(table) = &component.instance else {
            continue;
        };
        let Some(class) = new.get(&key.1) else {
            gone.push(key.clone());
            continue;
        };
        let before = old.get(&key.1).map_or(&[][..], |c| c.fields.as_slice());
        for (field, info) in class.fields.iter().zip(&class.schema.fields) {
            let kept = before.iter().find(|f| f.name == field.name);
            if kept.is_some_and(|f| f.kind == field.kind) {
                continue;
            }
            if let Some(value) = to_lua(lua, &field.kind, &info.default) {
                table.set(field.key.as_str(), value)?;
            }
            if kept.is_some() {
                swapped.notes.push(format!(
                    "{}'s {} holds another kind now, so it starts again.",
                    key.1, field.name
                ));
            }
        }
        for field in before {
            if !class.fields.iter().any(|f| f.name == field.name) {
                table.set(field.key.as_str(), LuaValue::Nil)?;
            }
        }
        table.set_metatable(Some(class.class.clone()));
        component.stopped = false;
    }
    for key in gone {
        running.remove(&key);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lua_errors_say_their_file_and_line() {
        let lua = Lua::new();
        let err = lua
            .load("local x = 1\nlocal y = nil + 1\n")
            .set_name("=parts/spin.lua")
            .exec()
            .unwrap_err();
        let problem = lua_problem("other.lua", &err);
        assert_eq!(problem.file, "parts/spin.lua");
        assert_eq!(problem.line, 2);
        let syntax = lua
            .load("function (")
            .set_name("=bad.lua")
            .into_function()
            .unwrap_err();
        let problem = lua_problem("bad.lua", &syntax);
        assert_eq!((problem.file.as_str(), problem.line), ("bad.lua", 1));
    }
}
