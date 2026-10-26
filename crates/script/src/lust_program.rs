//! The game's Lust, loaded: the chunk the Lust compiler made, evaluated in
//! the runtime's Luau, with its components' fields and schemas and the
//! events it declares (LUST.md §21, "The compiler inside an engine").

use crate::code::CodeProblem;
use crate::convert::{empty, from_lua, kind_of, label};
use crate::events::{EventKinds, read_events};
use crate::host::{Host, call_with};
use crate::lua_components::{Field, MAX_MESSAGE_BYTES};
use crate::luau::{Luau, capped};
use crate::runtime::{Failure, Swapped};
use mlua::{Function, Table, Value as LuaValue};
use reflect::{ComponentSchema, FieldInfo};
use std::collections::BTreeMap;
use std::rc::Rc;

/// The loaded Lust.
pub(crate) struct LustProgram {
    /// The table the chunk gave back: `components`, `new`, `swap`.
    pub(crate) api: Table,
    /// Its globals, where its natives are lent (`LUST_NATIVES`).
    pub(crate) env: Table,
    pub(crate) chunk: lust::LuaChunk,
    /// Each component's fields, by component name.
    pub(crate) fields: BTreeMap<String, Vec<Field>>,
    pub(crate) schemas: Vec<ComponentSchema>,
    /// What each declared event carries.
    pub(crate) events: Rc<EventKinds>,
    /// Each component's methods, looked up once (Lust's classes never
    /// change while the code runs), by component name.
    pub(crate) methods: BTreeMap<String, Methods>,
}

/// A Lust component's `update` and `handle`, if it has them.
#[derive(Default)]
pub(crate) struct Methods {
    pub(crate) update: Option<Function>,
    pub(crate) handle: Option<Function>,
}

/// `chunk` compiled in `luau`, with globals of its own (not run yet).
/// `bytecode`, if given, is the chunk already compiled (on the worker).
pub(crate) fn compile(
    luau: &Luau,
    chunk: &lust::LuaChunk,
    bytecode: Option<&[u8]>,
) -> Result<(Function, Table), Failure> {
    let failed = |err: mlua::Error| start_failure(chunk, &err);
    let env = luau.env().map_err(failed)?;
    let load = match bytecode {
        Some(bytes) => luau
            .lua
            .load(bytes)
            .set_mode(mlua::chunk::ChunkMode::Binary),
        None => crate::luau::as_text(luau.lua.load(&chunk.code)),
    };
    let function = load
        .set_name(lust::LuaChunk::NAME)
        .set_environment(env.clone())
        .into_function()
        .map_err(failed)?;
    Ok((function, env))
}

/// Runs the compiled `chunk` (the program's own start, with the natives
/// lent from `host`; what it sends goes nowhere, since nothing runs yet),
/// and reads its components and events.
pub(crate) fn prepare(
    luau: &Luau,
    host: &mut dyn Host,
    chunk: &lust::LuaChunk,
    (function, env): (Function, Table),
) -> Result<LustProgram, Failure> {
    let failed = |err: mlua::Error| start_failure(chunk, &err);
    let discard = Some(&luau.discard);
    let api: Table =
        call_with(luau, Some(&env), host, discard, || function.call(())).map_err(failed)?;
    // Solis hands vec3 values over as Luau vectors: the Lust must keep
    // them so too.
    let vectors: Option<String> = api.raw_get("vectors").map_err(failed)?;
    if vectors.as_deref() != Some("luau") {
        return Err(Failure {
            entity: None,
            component: String::new(),
            at: crate::ticking::plain(format!(
                "the Lust ({}) keeps vectors as {}, not as Luau vectors. This is a bug in \
                 Solis or Lust; please report it.",
                lust::VERSION,
                vectors.as_deref().unwrap_or("nothing it says")
            )),
            stopped: true,
        });
    }
    // Reading the components makes one of each, which works out their
    // fields' starting values: game code, so the natives are lent too.
    let ((fields, schemas, methods), events) = call_with(luau, Some(&env), host, discard, || {
        Ok((read_components(&api)?, read_events(&api)?))
    })
    .map_err(failed)?;
    Ok(LustProgram {
        api,
        env,
        chunk: chunk.clone(),
        fields,
        schemas,
        events: Rc::new(events),
        methods,
    })
}

/// A failure while the program started.
fn start_failure(chunk: &lust::LuaChunk, err: &mlua::Error) -> Failure {
    Failure {
        entity: None,
        component: String::new(),
        at: located(chunk, err),
        stopped: true,
    }
}

/// What [`read_components`] reads.
type Components = (
    BTreeMap<String, Vec<Field>>,
    Vec<ComponentSchema>,
    BTreeMap<String, Methods>,
);

/// Each component's fields, its schema for the editor (starting values
/// from a new one), and its methods.
fn read_components(api: &Table) -> mlua::Result<Components> {
    let components: Table = api.get("components")?;
    let new: mlua::Function = api.get("new")?;
    let mut fields = BTreeMap::new();
    let mut schemas = Vec::new();
    let mut methods = BTreeMap::new();
    for pair in components.pairs::<String, Table>() {
        let (name, class) = pair?;
        let mut mine = Vec::new();
        let mut infos = Vec::new();
        let fresh: Table = new.call((name.as_str(), LuaValue::Nil))?;
        let method = |key: &str| -> mlua::Result<Option<Function>> {
            Ok(match fresh.get::<LuaValue>(key)? {
                LuaValue::Function(f) => Some(f),
                _ => None,
            })
        };
        methods.insert(
            name.clone(),
            Methods {
                update: method("update")?,
                handle: method("handle")?,
            },
        );
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
    Ok((fields, schemas, methods))
}

/// What changed between the Lust whose table is `old` and `new` (Lust's
/// `changes`), into `swapped`.
pub(crate) fn changes(
    luau: &Luau,
    new: &LustProgram,
    old: Table,
    swapped: &mut Swapped,
) -> mlua::Result<()> {
    let compare: Function = new.api.raw_get("changes")?;
    let changes: Table = luau.clock.limited(|| compare.call(old))?;
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
        message: capped(&at.message, MAX_MESSAGE_BYTES),
    }
}
