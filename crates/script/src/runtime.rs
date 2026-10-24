//! Running game code: the Lua the Lust compiler made, in Luau
//! ([decision 0028](../../../docs/decisions/0028-lust-runs-as-luau-while-editing.md)).
//!
//! A [`Runtime`] loads one program and holds its running components, one
//! per entity and component name. Each tick runs every component's
//! `update`, entities in ID order and components in name order (LUST.md
//! §21), so a game runs the same way every time. The native methods
//! (`say`, `moveTo`) reach the game through a [`Host`], lent for exactly
//! one call, so game code never holds on to the world.
//!
//! Game code can't reach files, programs or the network (Luau has no such
//! libraries), can use at most [`MEMORY_LIMIT`] bytes, and is stopped if
//! one call runs longer than [`TIME_LIMIT`].

use crate::code::CodeProblem;
use glam::{Quat, Vec3};
use mlua::{Lua, Table, Value as LuaValue};
use reflect::{ComponentData, ComponentSchema, EntityId, FieldInfo, FieldKind, Value};
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::{Duration, Instant};

/// The most memory game code may use.
pub const MEMORY_LIMIT: usize = 64 << 20;
/// The longest one call into game code may run before it's stopped.
pub const TIME_LIMIT: Duration = Duration::from_millis(100);

/// What game code can ask of the engine: the bodies of the native methods
/// described in `solis/*.lst` ([`crate::describe::NATIVES`]). Entities
/// that aren't in the game read as nothing, and changing them does
/// nothing.
pub trait Host {
    /// Writes words in the Console.
    fn say(&mut self, words: &str);
    /// How long the game has been running, in seconds of game time.
    fn time(&self) -> f64;
    /// The first entity with this name, in the area's tree order.
    fn find(&self, name: &str) -> Option<EntityId>;
    /// Where an entity is in the world.
    fn position(&self, which: EntityId) -> Option<Vec3>;
    /// Puts an entity at a place in the world.
    fn move_to(&mut self, which: EntityId, place: Vec3);
    /// Which way an entity faces in the world.
    fn rotation(&self, which: EntityId) -> Option<Quat>;
    /// Turns an entity to face this way in the world.
    fn turn_to(&mut self, which: EntityId, facing: Quat);
    /// An entity's name.
    fn name(&self, which: EntityId) -> Option<String>;
}

/// A host for when nothing is running (loading code to read its
/// components): it knows no entities, and words said go nowhere.
#[derive(Debug, Default)]
pub struct NoHost;

impl Host for NoHost {
    fn say(&mut self, _words: &str) {}
    fn time(&self) -> f64 {
        0.0
    }
    fn find(&self, _name: &str) -> Option<EntityId> {
        None
    }
    fn position(&self, _which: EntityId) -> Option<Vec3> {
        None
    }
    fn move_to(&mut self, _which: EntityId, _place: Vec3) {}
    fn rotation(&self, _which: EntityId) -> Option<Quat> {
        None
    }
    fn turn_to(&mut self, _which: EntityId, _facing: Quat) {}
    fn name(&self, _which: EntityId) -> Option<String> {
        None
    }
}

/// A failure while game code ran: which component, and where in the Lust.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    /// The entity whose component failed (none while the program's own
    /// start ran).
    pub entity: Option<EntityId>,
    /// The component (empty while the program's own start ran).
    pub component: String,
    /// Where in the Lust, and why.
    pub at: CodeProblem,
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.component.is_empty() {
            write!(f, "{}", self.at)
        } else {
            write!(f, "{} stopped: {}", self.component, self.at)
        }
    }
}

/// A field as Lust keeps it: its key in the component's table, and kind.
#[derive(Debug, Clone, PartialEq)]
struct Field {
    key: String,
    name: String,
    kind: FieldKind,
}

/// One running component.
struct Running {
    table: Table,
    /// The area's values it was last given, to send only what changes.
    seen: ComponentData,
    /// Stopped after a failure, until the code changes.
    stopped: bool,
}

/// The loaded program.
struct Program {
    api: Table,
    chunk: lust::LuaChunk,
    fields: BTreeMap<String, Vec<Field>>,
    schemas: Vec<ComponentSchema>,
}

/// Game code, loaded and running.
pub struct Runtime {
    lua: Lua,
    program: Option<Program>,
    running: BTreeMap<(EntityId, String), Running>,
    deadline: Rc<Cell<Option<Instant>>>,
}

impl std::fmt::Debug for Runtime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Runtime")
            .field("loaded", &self.program.is_some())
            .field("running", &self.running.len())
            .finish_non_exhaustive()
    }
}

impl Runtime {
    /// An empty runtime, with its memory and time limits set.
    pub fn new() -> Self {
        let lua = Lua::new();
        let _ = lua.set_memory_limit(MEMORY_LIMIT);
        let deadline: Rc<Cell<Option<Instant>>> = Rc::new(Cell::new(None));
        let watch = Rc::clone(&deadline);
        lua.set_interrupt(move |_| {
            if watch.get().is_some_and(|at| Instant::now() > at) {
                return Err(mlua::Error::runtime(format!(
                    "this took longer than {} ms, so it was stopped (is there a loop that never ends?)",
                    TIME_LIMIT.as_millis()
                )));
            }
            Ok(mlua::VmState::Continue)
        });
        Self {
            lua,
            program: None,
            running: BTreeMap::new(),
            deadline,
        }
    }

    /// Loads `chunk` (the Lua the Lust compiler made), running the
    /// program's own start, and reads its components. Anything running is
    /// dropped. On a failure nothing is loaded.
    pub fn load(&mut self, chunk: &lust::LuaChunk, host: &mut dyn Host) -> Result<(), Failure> {
        self.running.clear();
        self.program = None;
        let failed = |err: mlua::Error| Failure {
            entity: None,
            component: String::new(),
            at: located(chunk, &err),
        };
        let api: Table = self
            .call(host, || {
                self.lua
                    .load(&chunk.code)
                    .set_name(lust::LuaChunk::NAME)
                    .eval()
            })
            .map_err(failed)?;
        let (fields, schemas) = self.read_components(&api).map_err(failed)?;
        self.program = Some(Program {
            api,
            chunk: chunk.clone(),
            fields,
            schemas,
        });
        Ok(())
    }

    /// The loaded program's components, as the editor shows them, sorted
    /// by name (none if nothing is loaded).
    pub fn schemas(&self) -> &[ComponentSchema] {
        self.program.as_ref().map_or(&[], |p| &p.schemas)
    }

    /// Whether the loaded program has a component called `name`.
    pub fn has_component(&self, name: &str) -> bool {
        self.program
            .as_ref()
            .is_some_and(|p| p.fields.contains_key(name))
    }

    /// How many components are running.
    pub fn len(&self) -> usize {
        self.running.len()
    }

    /// Whether no components are running.
    pub fn is_empty(&self) -> bool {
        self.running.is_empty()
    }

    /// Makes `entity`'s component `name` run, with the area's `data` (a
    /// missing or wrong value keeps the starting one), or gives it the
    /// changed fields if it's running already. Does nothing for a
    /// component the program doesn't have.
    pub fn sync(
        &mut self,
        entity: EntityId,
        name: &str,
        data: &ComponentData,
        host: &mut dyn Host,
    ) -> Result<(), Failure> {
        let Some(program) = &self.program else {
            return Ok(());
        };
        let Some(fields) = program.fields.get(name) else {
            return Ok(());
        };
        let key = (entity, name.to_string());
        let fail = |err: mlua::Error| Failure {
            entity: Some(entity),
            component: name.to_string(),
            at: located(&program.chunk, &err),
        };
        if let Some(running) = self.running.get_mut(&key) {
            for field in fields {
                let now = data.get(&field.name);
                if now != running.seen.get(&field.name)
                    && let Some(value) = now.and_then(|v| to_lua(&self.lua, &field.kind, v))
                {
                    running.table.set(field.key.as_str(), value).map_err(fail)?;
                }
            }
            running.seen = data.clone();
            return Ok(());
        }
        let new: mlua::Function = program.api.get("new").map_err(fail)?;
        let table: Table = self
            .call(host, || new.call((name, entity.to_string())))
            .map_err(fail)?;
        for field in fields {
            if let Some(value) = data
                .get(&field.name)
                .and_then(|v| to_lua(&self.lua, &field.kind, v))
            {
                table.set(field.key.as_str(), value).map_err(fail)?;
            }
        }
        self.running.insert(
            key,
            Running {
                table,
                seen: data.clone(),
                stopped: false,
            },
        );
        Ok(())
    }

    /// Stops every running component `keep` says no to (its entity or
    /// component is gone from the area).
    pub fn retain(&mut self, mut keep: impl FnMut(EntityId, &str) -> bool) {
        self.running.retain(|(entity, name), _| keep(*entity, name));
    }

    /// The fields of `entity`'s running component `name`, as they are now.
    pub fn values(&self, entity: EntityId, name: &str) -> Option<ComponentData> {
        let program = self.program.as_ref()?;
        let fields = program.fields.get(name)?;
        let running = self.running.get(&(entity, name.to_string()))?;
        let mut data = ComponentData::default();
        for field in fields {
            if let Ok(raw) = running.table.get::<LuaValue>(field.key.as_str())
                && let Some(value) = from_lua(&field.kind, &raw)
            {
                data.set(&field.name, value);
            }
        }
        Some(data)
    }

    /// Runs one tick of `dt` seconds: every running component's `update`,
    /// entities in ID order and components in name order. A component that
    /// fails stops (until the code changes), and its failure is given
    /// back; the others carry on.
    pub fn tick(&mut self, dt: f64, host: &mut dyn Host) -> Vec<Failure> {
        let Some(program) = &self.program else {
            return Vec::new();
        };
        let mut failures = Vec::new();
        let (lua, deadline, running) = (&self.lua, &self.deadline, &mut self.running);
        // The natives are lent once for the whole tick; the time limit
        // watches each component's update on its own.
        let lent = with_natives(lua, host, || {
            for ((entity, name), component) in running.iter_mut() {
                if component.stopped {
                    continue;
                }
                let update = match component.table.get::<LuaValue>("update") {
                    Ok(LuaValue::Function(f)) => f,
                    _ => continue,
                };
                let table = component.table.clone();
                let result = limited(deadline, || update.call::<()>((table, dt)));
                if let Err(err) = result {
                    component.stopped = true;
                    failures.push(Failure {
                        entity: Some(*entity),
                        component: name.clone(),
                        at: located(&program.chunk, &err),
                    });
                }
            }
            Ok(())
        });
        if let Err(err) = lent {
            failures.push(Failure {
                entity: None,
                component: String::new(),
                at: located(&program.chunk, &err),
            });
        }
        failures
    }

    /// Runs `run` with the natives lent from `host` and the time limit.
    fn call<R>(
        &self,
        host: &mut dyn Host,
        run: impl FnOnce() -> mlua::Result<R>,
    ) -> mlua::Result<R> {
        call_with(&self.lua, &self.deadline, host, run)
    }

    /// Each component's fields, and its schema for the editor (starting
    /// values from a new one).
    #[allow(clippy::type_complexity)]
    fn read_components(
        &self,
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
                help: format!("{name}, from the game's code."),
                fields: infos,
            });
            fields.insert(name, mine);
        }
        schemas.sort_by(|a, b| a.name.cmp(&b.name));
        Ok((fields, schemas))
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

/// Runs `run` with `LUST_NATIVES` filled from `host` for its length, and
/// the time limit watching it.
fn call_with<R>(
    lua: &Lua,
    deadline: &Cell<Option<Instant>>,
    host: &mut dyn Host,
    run: impl FnOnce() -> mlua::Result<R>,
) -> mlua::Result<R> {
    with_natives(lua, host, || limited(deadline, run))
}

/// Runs `run` with the time limit watching it.
fn limited<R>(
    deadline: &Cell<Option<Instant>>,
    run: impl FnOnce() -> mlua::Result<R>,
) -> mlua::Result<R> {
    deadline.set(Some(Instant::now() + TIME_LIMIT));
    let result = run();
    deadline.set(None);
    result
}

/// Runs `run` with `LUST_NATIVES` filled from `host` for its length.
fn with_natives<R>(
    lua: &Lua,
    host: &mut dyn Host,
    run: impl FnOnce() -> mlua::Result<R>,
) -> mlua::Result<R> {
    let host = RefCell::new(host);
    lua.scope(|scope| {
        let natives = lua.create_table()?;
        let h = &host;
        natives.set(
            "say",
            scope.create_function(move |_, words: String| {
                h.borrow_mut().say(&words);
                Ok(())
            })?,
        )?;
        natives.set(
            "time",
            scope.create_function(move |_, ()| Ok(h.borrow().time()))?,
        )?;
        natives.set(
            "find",
            scope.create_function(move |_, name: String| {
                Ok(h.borrow().find(&name).map(|id| id.to_string()))
            })?,
        )?;
        natives.set(
            "positionOf",
            scope.create_function(move |lua, which: LuaValue| {
                let at = entity_of(&which).and_then(|e| h.borrow().position(e));
                vec3_table(lua, at.unwrap_or(Vec3::ZERO))
            })?,
        )?;
        natives.set(
            "moveTo",
            scope.create_function(move |_, (which, place): (LuaValue, Table)| {
                if let (Some(e), Some(place)) = (entity_of(&which), vec3_of(&place)) {
                    h.borrow_mut().move_to(e, place);
                }
                Ok(())
            })?,
        )?;
        natives.set(
            "rotationOf",
            scope.create_function(move |lua, which: LuaValue| {
                let facing = entity_of(&which).and_then(|e| h.borrow().rotation(e));
                quat_table(lua, facing.unwrap_or(Quat::IDENTITY))
            })?,
        )?;
        natives.set(
            "turnTo",
            scope.create_function(move |_, (which, facing): (LuaValue, Table)| {
                if let (Some(e), Some(facing)) = (entity_of(&which), quat_of(&facing)) {
                    h.borrow_mut().turn_to(e, facing);
                }
                Ok(())
            })?,
        )?;
        natives.set(
            "nameOf",
            scope.create_function(move |_, which: LuaValue| {
                Ok(entity_of(&which)
                    .and_then(|e| h.borrow().name(e))
                    .unwrap_or_default())
            })?,
        )?;
        lua.globals().set("LUST_NATIVES", natives)?;
        let result = run();
        lua.globals().set("LUST_NATIVES", LuaValue::Nil)?;
        result
    })
}

/// Where in the Lust `err` came from.
fn located(chunk: &lust::LuaChunk, err: &mlua::Error) -> CodeProblem {
    let at = chunk.locate(&err.to_string());
    CodeProblem {
        file: at.file,
        line: at.line.unwrap_or(0),
        column: at.column.unwrap_or(0),
        message: at.message,
    }
}

/// A Lust field kind as the editor's.
fn kind_of(name: &str, options: Vec<String>) -> Option<FieldKind> {
    Some(match name {
        "bool" => FieldKind::Bool,
        "int" => FieldKind::Int,
        "float" => FieldKind::Float,
        "string" => FieldKind::Text,
        "state" => FieldKind::Choice(options),
        "vec3" => FieldKind::Vec3,
        "rotation" => FieldKind::Rotation,
        "entity" => FieldKind::Entity,
        _ => return None,
    })
}

/// A value of `kind` when a new component somehow has none.
fn empty(kind: &FieldKind) -> Value {
    match kind {
        FieldKind::Bool => Value::Bool(false),
        FieldKind::Int => Value::Int(0),
        FieldKind::Float => Value::Float(0.0),
        FieldKind::Choice(options) => Value::Text(options.first().cloned().unwrap_or_default()),
        FieldKind::Vec3 | FieldKind::Color => Value::floats(&[0.0, 0.0, 0.0]),
        FieldKind::Rotation => Value::floats(&[0.0, 0.0, 0.0, 1.0]),
        FieldKind::Entity => Value::Entity(None),
        FieldKind::Asset => Value::Asset(None),
        FieldKind::Text => Value::Text(String::new()),
    }
}

/// `lastShot` as the inspector shows it: "Last shot".
fn label(name: &str) -> String {
    let mut out = String::new();
    for (i, c) in name.chars().enumerate() {
        if c == '_' {
            out.push(' ');
        } else if i == 0 {
            out.extend(c.to_uppercase());
        } else if c.is_uppercase() {
            out.push(' ');
            out.extend(c.to_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

/// A Lua number as an `f64`.
fn number(value: &LuaValue) -> Option<f64> {
    match value {
        LuaValue::Integer(n) => Some(*n as f64),
        LuaValue::Number(n) if n.is_finite() => Some(*n),
        _ => None,
    }
}

/// An entity as Lust hands it over (its 13-letter text, or nil).
fn entity_of(value: &LuaValue) -> Option<EntityId> {
    match value {
        LuaValue::String(text) => text.to_str().ok()?.parse().ok(),
        _ => None,
    }
}

fn vec3_of(table: &Table) -> Option<Vec3> {
    let n = |k: &str| number(&table.get::<LuaValue>(k).ok()?).map(|v| v as f32);
    Some(Vec3::new(n("x")?, n("y")?, n("z")?)).filter(|v| v.is_finite())
}

fn quat_of(table: &Table) -> Option<Quat> {
    let n = |k: &str| number(&table.get::<LuaValue>(k).ok()?).map(|v| v as f32);
    let q = Quat::from_xyzw(n("x")?, n("y")?, n("z")?, n("w")?);
    (q.is_finite() && q.length_squared() > 1e-12).then(|| q.normalize())
}

fn vec3_table(lua: &Lua, v: Vec3) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    t.set("x", v.x)?;
    t.set("y", v.y)?;
    t.set("z", v.z)?;
    Ok(t)
}

fn quat_table(lua: &Lua, q: Quat) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    t.set("x", q.x)?;
    t.set("y", q.y)?;
    t.set("z", q.z)?;
    t.set("w", q.w)?;
    Ok(t)
}

/// An editor value as Lust keeps it, if it's the right kind.
fn to_lua(lua: &Lua, kind: &FieldKind, value: &Value) -> Option<LuaValue> {
    if !kind.accepts(value) {
        return None;
    }
    Some(match (kind, value) {
        (FieldKind::Bool, Value::Bool(b)) => LuaValue::Boolean(*b),
        (FieldKind::Int, Value::Int(n)) => LuaValue::Number(*n as f64),
        (FieldKind::Float, v) => LuaValue::Number(v.as_f64()?),
        (FieldKind::Text | FieldKind::Choice(_), Value::Text(t)) => {
            LuaValue::String(lua.create_string(t).ok()?)
        }
        (FieldKind::Vec3, v) => {
            let [x, y, z] = v.as_floats::<3>()?;
            LuaValue::Table(vec3_table(lua, Vec3::new(x, y, z)).ok()?)
        }
        (FieldKind::Rotation, v) => {
            let [x, y, z, w] = v.as_floats::<4>()?;
            LuaValue::Table(quat_table(lua, Quat::from_xyzw(x, y, z, w)).ok()?)
        }
        (FieldKind::Entity, Value::Entity(None)) => LuaValue::Nil,
        (FieldKind::Entity, Value::Entity(Some(id))) => {
            LuaValue::String(lua.create_string(id.to_string()).ok()?)
        }
        _ => return None,
    })
}

/// A Lust field's value as the editor's, if it's the right kind.
fn from_lua(kind: &FieldKind, value: &LuaValue) -> Option<Value> {
    Some(match (kind, value) {
        (FieldKind::Bool, LuaValue::Boolean(b)) => Value::Bool(*b),
        (FieldKind::Int, v) => {
            let n = number(v)?;
            (n.fract() == 0.0 && n.abs() < 9.0e15).then_some(Value::Int(n as i64))?
        }
        (FieldKind::Float, v) => Value::Float(number(v)?),
        (FieldKind::Text, LuaValue::String(s)) => Value::Text(s.to_str().ok()?.to_string()),
        (FieldKind::Choice(options), LuaValue::String(s)) => {
            let text = s.to_str().ok()?.to_string();
            options.contains(&text).then_some(Value::Text(text))?
        }
        (FieldKind::Vec3, LuaValue::Table(t)) => {
            let v = vec3_of(t)?;
            Value::floats(&[v.x, v.y, v.z])
        }
        (FieldKind::Rotation, LuaValue::Table(t)) => {
            let n = |k: &str| number(&t.get::<LuaValue>(k).ok()?).map(|v| v as f32);
            Value::floats(&[n("x")?, n("y")?, n("z")?, n("w")?])
        }
        (FieldKind::Entity, LuaValue::Nil) => Value::Entity(None),
        (FieldKind::Entity, v) => Value::Entity(Some(entity_of(v)?)),
        _ => return None,
    })
}

#[cfg(test)]
#[path = "runtime_tests.rs"]
mod tests;
