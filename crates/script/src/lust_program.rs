//! The game's Lust, loaded: the chunk the Lust compiler made, evaluated in
//! the runtime's Luau, with its components' fields and schemas and the
//! events it declares (LUST.md §21, "The compiler inside an engine").

use crate::code::CodeProblem;
use crate::convert::{empty, from_lua, kind_of, label};
use crate::events::{EventKinds, read_events};
use crate::host::{Host, call_with};
use crate::lua_components::Field;
use crate::runtime::{Failure, Swapped};
use mlua::{Lua, Table, Value as LuaValue};
use reflect::{ComponentSchema, FieldInfo};
use std::cell::Cell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::Instant;

/// The loaded Lust.
pub(crate) struct LustProgram {
    /// The table the chunk gave back: `components`, `new`, `swap`.
    pub(crate) api: Table,
    pub(crate) chunk: lust::LuaChunk,
    /// Each component's fields, by component name.
    pub(crate) fields: BTreeMap<String, Vec<Field>>,
    pub(crate) schemas: Vec<ComponentSchema>,
    /// What each declared event carries.
    pub(crate) events: Rc<EventKinds>,
}

/// Evaluates `chunk` in `lua` (the program's own start runs, with the
/// natives lent from `host`), and reads its components and events.
pub(crate) fn prepare(
    lua: &Lua,
    deadline: &Cell<Option<Instant>>,
    host: &mut dyn Host,
    chunk: &lust::LuaChunk,
) -> Result<LustProgram, Failure> {
    let failed = |err: mlua::Error| Failure {
        entity: None,
        component: String::new(),
        at: located(chunk, &err),
        stopped: true,
    };
    // Sends during the program's own start go nowhere: nothing runs yet.
    let api: Table = call_with(lua, deadline, host, None, || {
        lua.load(&chunk.code).set_name(lust::LuaChunk::NAME).eval()
    })
    .map_err(failed)?;
    let (fields, schemas) = read_components(&api).map_err(failed)?;
    let events = Rc::new(read_events(&api).map_err(failed)?);
    Ok(LustProgram {
        api,
        chunk: chunk.clone(),
        fields,
        schemas,
        events,
    })
}

/// Each component's fields, and its schema for the editor (starting
/// values from a new one).
#[allow(clippy::type_complexity)]
fn read_components(
    api: &Table,
) -> mlua::Result<(BTreeMap<String, Vec<Field>>, Vec<ComponentSchema>)> {
    let components: Table = api.get("components")?;
    let new: mlua::Function = api.get("new")?;
    let mut fields = BTreeMap::new();
    let mut schemas = Vec::new();
    for pair in components.pairs::<String, Table>() {
        let (name, class) = pair?;
        let mut mine = Vec::new();
        let mut infos = Vec::new();
        let fresh: Table = new.call((name.as_str(), LuaValue::Nil))?;
        let schema: Table = class.get("__schema")?;
        for entry in schema.sequence_values::<Table>() {
            let entry = entry?;
            let key: String = entry.get(1)?;
            let field_name: String = entry.get(2)?;
            let kind_name: String = entry.get(3)?;
            let options: Vec<String> = match entry.get::<LuaValue>(4)? {
                LuaValue::Table(list) => {
                    list.sequence_values::<String>().collect::<Result<_, _>>()?
                }
                _ => Vec::new(),
            };
            let Some(kind) = kind_of(&kind_name, options) else {
                continue;
            };
            let default = from_lua(&kind, &fresh.get::<LuaValue>(key.as_str())?)
                .unwrap_or_else(|| empty(&kind));
            infos.push(FieldInfo::new(
                &field_name,
                &label(&field_name),
                kind.clone(),
                default,
            ));
            mine.push(Field {
                key,
                name: field_name,
                kind,
            });
        }
        schemas.push(ComponentSchema {
            name: name.clone(),
            help: format!("{name}, from the game's Lust."),
            fields: infos,
        });
        fields.insert(name, mine);
    }
    schemas.sort_by(|a, b| a.name.cmp(&b.name));
    Ok((fields, schemas))
}

/// What changed between the Lust whose table is `old` and `new` (Lust's
/// `changes`), into `swapped`.
pub(crate) fn changes(new: &LustProgram, old: Table, swapped: &mut Swapped) -> mlua::Result<()> {
    let changes: Table = new.api.get::<mlua::Function>("changes")?.call(old)?;
    for (what, list) in [
        ("changed", &mut swapped.changed),
        ("removed", &mut swapped.removed),
        ("added", &mut swapped.added),
    ] {
        let names: Table = changes.get(what)?;
        for name in names.sequence_values::<String>() {
            list.push(name?);
        }
    }
    Ok(())
}

/// Where in the Lust `err` came from.
pub(crate) fn located(chunk: &lust::LuaChunk, err: &mlua::Error) -> CodeProblem {
    let at = chunk.locate(&err.to_string());
    CodeProblem {
        file: at.file,
        line: at.line.unwrap_or(0),
        column: at.column.unwrap_or(0),
        message: at.message,
    }
}
