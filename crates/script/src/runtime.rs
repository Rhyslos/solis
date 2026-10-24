//! Running game code: the Lua the Lust compiler made, in Luau
//! ([decision 0028](../../../docs/decisions/0028-lust-runs-as-luau-while-editing.md)).
//!
//! A [`Runtime`] loads one program and holds its running components, one
//! per entity and component name. Each tick runs every component's
//! `update`, entities in ID order and components in name order (LUST.md
//! §21), so a game runs the same way every time. The native methods
//! (`say`, `moveTo`) reach the game through a [`Host`], lent for exactly
//! one call, so game code never holds on to the world. After the updates,
//! the tick's events are delivered (see [`crate::events`]). Components
//! written in Rust ([`RustKind`]) run alongside, in the same order.
//!
//! Game code can't reach files, programs or the network (Luau has no such
//! libraries), can use at most [`MEMORY_LIMIT`] bytes, and is stopped if
//! one call runs longer than [`TIME_LIMIT`].

use crate::code::CodeProblem;
use crate::convert::{empty, from_lua, kind_of, label, to_lua};
use crate::events::{Event, EventKinds, MAX_DELIVERIES, Queue, read_events, sender, values_table};
use crate::host::{Host, Lent, call_with, limited, with_natives};
use crate::rust_component::{Context, RustComponent, RustKind};
use mlua::{Lua, Table, Value as LuaValue};
use reflect::{ComponentData, ComponentSchema, EntityId, FieldInfo, FieldKind};
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, VecDeque};
use std::rc::Rc;
use std::time::{Duration, Instant};

/// The most memory game code may use.
pub const MEMORY_LIMIT: usize = 64 << 20;
/// The longest one call into game code may run before it's stopped.
pub const TIME_LIMIT: Duration = Duration::from_millis(100);

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
    /// Whether the component stopped (a failure handling an event leaves
    /// it running: the sender's values may have been wrong).
    pub stopped: bool,
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (self.component.is_empty(), self.stopped) {
            (true, _) => write!(f, "{}", self.at),
            (false, true) => write!(f, "{} stopped: {}", self.component, self.at),
            (false, false) => write!(f, "{}: {}", self.component, self.at),
        }
    }
}

/// What swapping new code into the running game changed (see
/// [`Runtime::swap`]).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Swapped {
    /// Components whose fields changed: names, kinds or a state's options.
    pub changed: Vec<String>,
    /// Components no longer in the code; those running stopped.
    pub removed: Vec<String>,
    /// Components new in the code.
    pub added: Vec<String>,
    /// What started over, in plain words ("Gun's ammo is float now, not
    /// int, so it starts at 10.").
    pub notes: Vec<String>,
}

/// A field as Lust keeps it: its key in the component's table, and kind.
#[derive(Debug, Clone, PartialEq)]
struct Field {
    key: String,
    name: String,
    kind: FieldKind,
}

/// What a running component is.
enum Instance {
    /// Lust's: its table in Luau.
    Lust(Table),
    /// Written in Rust.
    Rust(Box<dyn RustComponent>),
}

/// One running component.
struct Running {
    instance: Instance,
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
    /// What each declared event carries.
    events: Rc<EventKinds>,
    /// `send_event`, which Lust's `send` calls.
    send: mlua::Function,
}

/// Game code, loaded and running.
pub struct Runtime {
    lua: Lua,
    program: Option<Program>,
    running: BTreeMap<(EntityId, String), Running>,
    deadline: Rc<Cell<Option<Instant>>>,
    /// Events on their way this tick.
    queue: Queue,
    /// The kinds of Rust component the game has, by name.
    rust: BTreeMap<String, RustKind>,
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
            queue: Rc::new(RefCell::new(VecDeque::new())),
            rust: BTreeMap::new(),
        }
    }

    /// Adds a kind of Rust component, kept across loads.
    pub fn add_rust(&mut self, kind: RustKind) {
        self.rust.insert(kind.schema.name.clone(), kind);
    }

    /// Loads `chunk` (the Lua the Lust compiler made), running the
    /// program's own start, and reads its components. Anything running is
    /// dropped. On a failure nothing is loaded.
    pub fn load(&mut self, chunk: &lust::LuaChunk, host: &mut dyn Host) -> Result<(), Failure> {
        self.running.clear();
        self.program = None;
        self.queue.borrow_mut().clear();
        self.program = Some(self.prepare(chunk, host)?);
        Ok(())
    }

    /// Swaps new code into the running game (LUST.md §21, `swap`): every
    /// running Lust component gets the new code in place, keeping each
    /// field's value by name (a field whose kind changed, or a new one,
    /// takes its starting value). Components gone from the code stop.
    /// Stopped components run again, since the code changed. On a
    /// failure (the new code's start fails) the old code keeps running.
    /// With nothing loaded, it's a [`Self::load`] that keeps Rust
    /// components running.
    pub fn swap(
        &mut self,
        chunk: &lust::LuaChunk,
        host: &mut dyn Host,
    ) -> Result<Swapped, Failure> {
        let Some(old) = self.program.as_ref().map(|p| p.api.clone()) else {
            self.program = Some(self.prepare(chunk, host)?);
            return Ok(Swapped {
                added: self.schemas().iter().map(|s| s.name.clone()).collect(),
                ..Swapped::default()
            });
        };
        let program = self.prepare(chunk, host)?;
        let failed = |err: mlua::Error| Failure {
            entity: None,
            component: String::new(),
            at: located(chunk, &err),
            stopped: false,
        };
        let mut swapped = Swapped::default();
        let changes: Table = program
            .api
            .get::<mlua::Function>("changes")
            .and_then(|f| f.call(old))
            .map_err(failed)?;
        for (what, list) in [
            ("changed", &mut swapped.changed),
            ("removed", &mut swapped.removed),
            ("added", &mut swapped.added),
        ] {
            let names: Table = changes.get(what).map_err(failed)?;
            *list = names
                .sequence_values::<String>()
                .collect::<Result<_, _>>()
                .map_err(failed)?;
        }
        let swap: mlua::Function = program.api.get("swap").map_err(failed)?;
        let mut gone = Vec::new();
        for (key, running) in &mut self.running {
            let Instance::Lust(table) = &running.instance else {
                continue;
            };
            let (now, notes): (LuaValue, Table) = swap.call(table.clone()).map_err(failed)?;
            for note in notes.sequence_values::<String>() {
                swapped.notes.push(note.map_err(failed)?);
            }
            if now.is_nil() {
                gone.push(key.clone());
            }
            running.stopped = false;
        }
        for key in gone {
            self.running.remove(&key);
        }
        self.queue.borrow_mut().clear();
        self.program = Some(program);
        Ok(swapped)
    }

    /// Stops the game's Lust: its components stop and its code goes (the
    /// code was deleted). Rust components carry on.
    pub fn unload(&mut self) {
        self.program = None;
        self.running
            .retain(|_, running| matches!(running.instance, Instance::Rust(_)));
    }

    /// Evaluates `chunk` in this runtime's Luau (its own start runs), and
    /// reads its components and events.
    fn prepare(&self, chunk: &lust::LuaChunk, host: &mut dyn Host) -> Result<Program, Failure> {
        let failed = |err: mlua::Error| Failure {
            entity: None,
            component: String::new(),
            at: located(chunk, &err),
            stopped: true,
        };
        // Sends during the program's own start go nowhere: nothing runs yet.
        let api: Table = call_with(&self.lua, &self.deadline, host, None, || {
            self.lua
                .load(&chunk.code)
                .set_name(lust::LuaChunk::NAME)
                .eval()
        })
        .map_err(failed)?;
        let (fields, schemas) = self.read_components(&api).map_err(failed)?;
        let events = Rc::new(read_events(&api).map_err(failed)?);
        let send = sender(&self.lua, &self.queue, Rc::clone(&events)).map_err(failed)?;
        Ok(Program {
            api,
            chunk: chunk.clone(),
            fields,
            schemas,
            events,
            send,
        })
    }

    /// The loaded program's components, as the editor shows them, sorted
    /// by name (none if nothing is loaded).
    pub fn schemas(&self) -> &[ComponentSchema] {
        self.program.as_ref().map_or(&[], |p| &p.schemas)
    }

    /// Whether there's a component called `name` to run: the loaded
    /// program's, or a Rust one.
    pub fn has_component(&self, name: &str) -> bool {
        self.rust.contains_key(name)
            || self
                .program
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
    /// component there's no code for.
    pub fn sync(
        &mut self,
        entity: EntityId,
        name: &str,
        data: &ComponentData,
        host: &mut dyn Host,
    ) -> Result<(), Failure> {
        let key = (entity, name.to_string());
        if let Some(kind) = self.rust.get(name) {
            match self.running.get_mut(&key) {
                Some(running) => {
                    let mut changed = ComponentData::default();
                    for field in &kind.schema.fields {
                        if let Some(now) = data.get(&field.name)
                            && running.seen.get(&field.name) != Some(now)
                        {
                            changed.set(&field.name, now.clone());
                        }
                    }
                    if let Instance::Rust(component) = &mut running.instance {
                        component.set(&changed);
                    }
                    running.seen = data.clone();
                }
                None => {
                    let made = (kind.make)(data);
                    self.running
                        .insert(key, running(Instance::Rust(made), data));
                }
            }
            return Ok(());
        }
        let Some(program) = &self.program else {
            return Ok(());
        };
        let Some(fields) = program.fields.get(name) else {
            return Ok(());
        };
        let fail = |err: mlua::Error| Failure {
            entity: Some(entity),
            component: name.to_string(),
            at: located(&program.chunk, &err),
            stopped: true,
        };
        if let Some(running) = self.running.get_mut(&key) {
            if let Instance::Lust(table) = &running.instance {
                for field in fields {
                    let now = data.get(&field.name);
                    if now != running.seen.get(&field.name)
                        && let Some(value) = now.and_then(|v| to_lua(&self.lua, &field.kind, v))
                    {
                        table.set(field.key.as_str(), value).map_err(fail)?;
                    }
                }
            }
            running.seen = data.clone();
            return Ok(());
        }
        let new: mlua::Function = program.api.get("new").map_err(fail)?;
        let table: Table = call_with(&self.lua, &self.deadline, host, Some(&program.send), || {
            new.call((name, entity.to_string()))
        })
        .map_err(fail)?;
        for field in fields {
            if let Some(value) = data
                .get(&field.name)
                .and_then(|v| to_lua(&self.lua, &field.kind, v))
            {
                table.set(field.key.as_str(), value).map_err(fail)?;
            }
        }
        self.running
            .insert(key, running(Instance::Lust(table), data));
        Ok(())
    }

    /// Stops every running component `keep` says no to (its entity or
    /// component is gone from the area).
    pub fn retain(&mut self, mut keep: impl FnMut(EntityId, &str) -> bool) {
        self.running.retain(|(entity, name), _| keep(*entity, name));
    }

    /// The fields of `entity`'s running component `name`, as they are now.
    pub fn values(&self, entity: EntityId, name: &str) -> Option<ComponentData> {
        let running = self.running.get(&(entity, name.to_string()))?;
        let table = match &running.instance {
            Instance::Rust(component) => return Some(component.values()),
            Instance::Lust(table) => table,
        };
        let fields = self.program.as_ref()?.fields.get(name)?;
        let mut data = ComponentData::default();
        for field in fields {
            if let Ok(raw) = table.get::<LuaValue>(field.key.as_str())
                && let Some(value) = from_lua(&field.kind, &raw)
            {
                data.set(&field.name, value);
            }
        }
        Some(data)
    }

    /// Runs one tick of `dt` seconds: every running component's `update`,
    /// entities in ID order and components in name order, then the events
    /// they sent. A component that fails stops (until the code changes),
    /// and its failure is given back; the others carry on.
    pub fn tick(&mut self, dt: f64, host: &mut dyn Host) -> Vec<Failure> {
        let mut failures = Vec::new();
        let program = self.program.as_ref();
        let ticking = Ticking {
            lua: &self.lua,
            deadline: &self.deadline,
            queue: &self.queue,
            chunk: program.map(|p| &p.chunk),
            events: program.map(|p| &*p.events),
        };
        let running = &mut self.running;
        // The natives are lent once for the whole tick; the time limit
        // watches each component's update on its own.
        let lent = with_natives(&self.lua, host, program.map(|p| &p.send), |lent| {
            for ((entity, name), component) in running.iter_mut() {
                if !component.stopped
                    && let Err(at) = ticking.update(lent, *entity, component, dt)
                {
                    component.stopped = true;
                    failures.push(Failure {
                        entity: Some(*entity),
                        component: name.clone(),
                        at,
                        stopped: true,
                    });
                }
            }
            failures.extend(ticking.deliver(lent, running));
            Ok(())
        });
        if let Err(err) = lent {
            failures.push(Failure {
                entity: None,
                component: String::new(),
                at: ticking.located(&err),
                stopped: false,
            });
        }
        failures
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

/// A newly made running component.
fn running(instance: Instance, data: &ComponentData) -> Running {
    Running {
        instance,
        seen: data.clone(),
        stopped: false,
    }
}

/// What a tick needs, apart from the running components.
struct Ticking<'a> {
    lua: &'a Lua,
    deadline: &'a Cell<Option<Instant>>,
    queue: &'a Queue,
    chunk: Option<&'a lust::LuaChunk>,
    events: Option<&'a EventKinds>,
}

impl Ticking<'_> {
    /// Where in the game's code `err` came from.
    fn located(&self, err: &mlua::Error) -> CodeProblem {
        match self.chunk {
            Some(chunk) => located(chunk, err),
            None => CodeProblem {
                file: String::new(),
                line: 0,
                column: 0,
                message: err.to_string(),
            },
        }
    }

    /// Runs one component's `update`.
    fn update(
        &self,
        lent: &Lent<'_, '_>,
        me: EntityId,
        component: &mut Running,
        dt: f64,
    ) -> Result<(), CodeProblem> {
        match &mut component.instance {
            Instance::Lust(table) => {
                let update = match table.get::<LuaValue>("update") {
                    Ok(LuaValue::Function(f)) => f,
                    _ => return Ok(()),
                };
                let table = table.clone();
                limited(self.deadline, || update.call::<()>((table, dt)))
                    .map_err(|err| self.located(&err))
            }
            Instance::Rust(rust) => {
                let mut host = lent.borrow_mut();
                let mut cx = Context::new(me, &mut **host, self.queue);
                caught(|| rust.update(&mut cx, dt))
            }
        }
    }

    /// Delivers the tick's events, in order, to each running component on
    /// the entity, in order of their names; events sent meanwhile join the
    /// end. A failure handling one is reported, and the component keeps
    /// running. Past [`MAX_DELIVERIES`], the rest are dropped.
    fn deliver(
        &self,
        lent: &Lent<'_, '_>,
        running: &mut BTreeMap<(EntityId, String), Running>,
    ) -> Vec<Failure> {
        let mut failures = Vec::new();
        let mut delivered = 0;
        loop {
            let Some(event) = self.queue.borrow_mut().pop_front() else {
                break;
            };
            if delivered >= MAX_DELIVERIES {
                let dropped = self.queue.borrow().len() + 1;
                self.queue.borrow_mut().clear();
                failures.push(Failure {
                    entity: Some(event.to),
                    component: String::new(),
                    at: CodeProblem {
                        file: String::new(),
                        line: 0,
                        column: 0,
                        message: format!(
                            "more than {MAX_DELIVERIES} events were sent in one tick, so the last \
                             {dropped} were dropped. Do components keep sending events to each \
                             other?"
                        ),
                    },
                    stopped: false,
                });
                break;
            }
            delivered += 1;
            let from = (event.to, String::new());
            for ((entity, name), component) in running.range_mut(from..) {
                if *entity != event.to {
                    break;
                }
                if component.stopped {
                    continue;
                }
                if let Err(mut at) = self.handle(lent, *entity, component, &event) {
                    at.message = format!("couldn't take the event {}: {}", event.name, at.message);
                    failures.push(Failure {
                        entity: Some(*entity),
                        component: name.clone(),
                        at,
                        stopped: false,
                    });
                }
            }
        }
        failures
    }

    /// Gives one component an event.
    fn handle(
        &self,
        lent: &Lent<'_, '_>,
        me: EntityId,
        component: &mut Running,
        event: &Event,
    ) -> Result<(), CodeProblem> {
        match &mut component.instance {
            Instance::Lust(table) => {
                // Only events the Lust declares reach Lust components.
                let Some(kinds) = self.events.and_then(|e| e.get(&event.name)) else {
                    return Ok(());
                };
                let Some(values) = values_table(self.lua, kinds, &event.values) else {
                    return Err(CodeProblem {
                        file: String::new(),
                        line: 0,
                        column: 0,
                        message: format!(
                            "it carries {} values, not the {} it was declared with, or one of \
                             the wrong kind",
                            event.values.len(),
                            kinds.len()
                        ),
                    });
                };
                let handle = match table.get::<LuaValue>("handle") {
                    Ok(LuaValue::Function(f)) => f,
                    _ => return Ok(()),
                };
                let table = table.clone();
                let name = event.name.as_str();
                limited(self.deadline, || handle.call::<bool>((table, name, values)))
                    .map(|_| ())
                    .map_err(|err| self.located(&err))
            }
            Instance::Rust(rust) => {
                let mut host = lent.borrow_mut();
                let mut cx = Context::new(me, &mut **host, self.queue);
                caught(|| {
                    rust.handle(&mut cx, event);
                })
            }
        }
    }
}

/// Runs a Rust component's code, turning a panic into a failure rather
/// than a crash.
fn caught(run: impl FnOnce()) -> Result<(), CodeProblem> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(run)).map_err(|why| {
        let message = why
            .downcast_ref::<&str>()
            .map(|s| (*s).to_string())
            .or_else(|| why.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "it stopped unexpectedly".to_string());
        CodeProblem {
            file: String::new(),
            line: 0,
            column: 0,
            message,
        }
    })
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
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

#[cfg(test)]
#[path = "runtime_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "runtime_events_tests.rs"]
mod events_tests;

#[cfg(test)]
#[path = "runtime_swap_tests.rs"]
mod swap_tests;
