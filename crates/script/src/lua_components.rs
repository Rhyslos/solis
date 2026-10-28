//! Components written in Lua (decision 0003), for people who know Lua
//! from other games' scripting or modding. Each `.lua` file in the assets
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
//!
//! Each file has globals of its own (see [`crate::luau`]): a helper made
//! global in one file isn't seen by another, and none can change the
//! standard libraries.

use crate::code::CodeProblem;
use crate::convert::{empty, from_lua, kind_of, label, to_lua};
use crate::luau::{Luau, capped};
use crate::runtime::Swapped;
use crate::ticking::{Instance, RunningMap};
use mlua::{Lua, Table, Value as LuaValue};
use reflect::{ComponentData, ComponentSchema, EntityId, FieldInfo, FieldKind};
use std::collections::BTreeMap;

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
    /// The file that made it, inside the assets.
    pub(crate) path: String,
}

/// A Lua error's place and message, from its text (`spinner.lua:5:
/// attempt to …`): the file it names, or `file` if it names none.
pub(crate) fn lua_problem(file: &str, err: &mlua::Error) -> CodeProblem {
    let mut problem = lua_problem_whole(file, err);
    problem.message = capped(&problem.message, MAX_MESSAGE_BYTES);
    problem
}

/// The most bytes of a mistake's message kept: game code's `error` can
/// throw any text.
pub(crate) const MAX_MESSAGE_BYTES: usize = 1_000;

fn lua_problem_whole(file: &str, err: &mlua::Error) -> CodeProblem {
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
            // The error's text is game code's to write (`error(msg, 0)`):
            // only a plain path inside the project is taken from it.
            file: if path.ends_with(".lua")
                && !path.contains(':')
                && path
                    .split(['/', '\\'])
                    .all(|part| !part.is_empty() && part != "." && part != "..")
            {
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

/// The Lua files, compiled, each with globals of its own (not run yet).
/// `bytecode`, when it has one for every file, is the files already
/// compiled (on the worker).
pub(crate) fn compile_files(
    luau: &Luau,
    files: &[(String, String)],
    bytecode: &[Vec<u8>],
) -> Result<Vec<(String, mlua::Function)>, CodeProblem> {
    let mut out = Vec::with_capacity(files.len());
    let compiled = bytecode.len() == files.len();
    for (i, (path, text)) in files.iter().enumerate() {
        let problem = |err: mlua::Error| lua_problem(path, &err);
        let load = if compiled {
            luau.lua
                .load(&bytecode[i][..])
                .set_mode(mlua::chunk::ChunkMode::Binary)
        } else {
            crate::luau::as_text(luau.lua.load(text))
        };
        let chunk = load
            .set_name(format!("={path}"))
            .set_environment(luau.env().map_err(problem)?)
            .into_function()
            .map_err(problem)?;
        out.push((path.clone(), chunk));
    }
    Ok(out)
}

/// The classes the Lua files made, by component name, after running each
/// (`run` lends the natives and the time limit). The first mistake found
/// stops it.
pub(crate) fn read_classes(
    luau: &Luau,
    files: &[(String, mlua::Function)],
    mut run: impl FnMut(&mlua::Function) -> mlua::Result<()>,
) -> Result<BTreeMap<String, LuaClass>, CodeProblem> {
    let mut classes = BTreeMap::new();
    for (path, chunk) in files {
        let problem = |err: mlua::Error| lua_problem(path, &err);
        // Anything made before (by a file that failed) isn't this file's.
        luau.made().map_err(problem)?;
        run(chunk).map_err(problem)?;
        let made = luau.made().map_err(problem)?;
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
    // Raw reads: the tables are game code's, and their metatables
    // mustn't run here.
    let name: String = class.raw_get("__name").map_err(problem)?;
    let list: Table = class.raw_get("__fields").map_err(problem)?;
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
            .raw_get(1)
            .map_err(|_| mistake(format!("{name}'s field {} has no name first", i + 1)))?;
        // `me` is the entity, and `update` and `on_…` are its methods: a
        // field with one of those names would hide them.
        if field == "me" || field == "update" || field.starts_with("on_") || field.starts_with("__")
        {
            return Err(mistake(format!(
                "{name}'s field can't be called {field}: me is the entity the component is on, \
                 update and on_ name its methods, and __ names are Lua's own. Call it something \
                 else"
            )));
        }
        let kind_name: String = entry.raw_get(2).map_err(|_| {
            mistake(format!(
                "{name}'s field {field} has no kind (such as \"float\")"
            ))
        })?;
        let options: Vec<String> = match entry.raw_get::<LuaValue>(4).map_err(problem)? {
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
        // A state lists its states (found by the hardening milestone's
        // tests: with none, its starting value wasn't one of its own).
        if matches!(&kind, FieldKind::Choice(options) if options.is_empty()) {
            return Err(mistake(format!(
                "{name}'s field {field} is a state, which needs its states listed after its \
                 starting one, such as {{ \"{field}\", \"state\", \"Single\", {{ \"Single\", \"Burst\" }} }}"
            )));
        }
        let start: LuaValue = entry.raw_get(3).map_err(problem)?;
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
        path: path.to_string(),
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
    table.raw_set("me", me.to_string())?;
    for (field, info) in class.fields.iter().zip(&class.schema.fields) {
        let value = data
            .get(&field.name)
            .and_then(|v| to_lua(lua, &field.kind, v))
            .or_else(|| to_lua(lua, &field.kind, &info.default));
        if let Some(value) = value {
            table.raw_set(field.key.as_str(), value)?;
        }
    }
    table.set_metatable(Some(class.class.clone()))?;
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
                table.raw_set(field.key.as_str(), value)?;
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
                table.raw_set(field.key.as_str(), LuaValue::Nil)?;
            }
        }
        table.set_metatable(Some(class.class.clone()))?;
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

    #[test]
    fn a_state_field_without_its_states_is_a_mistake() {
        let checked = crate::code::compile(&crate::Code {
            files: Vec::new(),
            lua: vec![(
                "mode.lua".into(),
                "local M = component(\"Mode\", { { \"mode\", \"state\" } })".into(),
            )],
            notes: Vec::new(),
        });
        let problems = checked.expect_err("refused");
        assert!(
            problems[0].message.contains("needs its states listed"),
            "{problems:?}"
        );
        // With its states, it's fine, starting at the first.
        let fine = crate::code::compile(&crate::Code {
            files: Vec::new(),
            lua: vec![(
                "mode.lua".into(),
                "local M = component(\"Mode\", { { \"mode\", \"state\", nil, { \"Single\", \"Burst\" } } })"
                    .into(),
            )],
            notes: Vec::new(),
        });
        let compiled = fine.expect("checked").expect("code");
        let field = &compiled.schemas[0].fields[0];
        assert_eq!(field.default, reflect::Value::Text("Single".into()));
    }

    #[test]
    fn an_error_s_text_can_t_name_a_file_outside_the_project() {
        // The third security audit: game code writes the error's text, so
        // it could point a click on the problem at a network share.
        let lua = Lua::new();
        for named in [
            "//attacker.example/share/a.lua",
            "\\\\attacker\\share\\a.lua",
            "C:/Windows/a.lua",
            "../outside.lua",
            "/etc/a.lua",
        ] {
            let err = lua
                .load(format!(
                    "error({:?}, 0)",
                    format!("{named}:1: double-click me")
                ))
                .exec()
                .unwrap_err();
            let problem = lua_problem("mine.lua", &err);
            assert_eq!(problem.file, "mine.lua", "{named}");
        }
    }
}
