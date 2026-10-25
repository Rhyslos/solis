//! Running game code ([decision 0028](../../../docs/decisions/0028-lust-runs-as-luau-while-editing.md)):
//! the Lua the Lust compiler made, the game's Lua components, and its Rust
//! components, together.
//!
//! A [`Runtime`] loads the game's code into one Luau and holds its running
//! components, one per entity and component name. Each tick runs every
//! component's `update`, entities in ID order and components in name
//! order (LUST.md §21), so a game runs the same way every time, then
//! delivers the events they sent (see [`crate::events`]). The native
//! methods (`say`, `moveTo`) reach the game through a [`Host`], lent for
//! exactly one call or tick, so game code never holds on to the world.
//!
//! Game code can't reach files, programs or the network (Luau has no such
//! libraries), can use at most [`MEMORY_LIMIT`] bytes, and is stopped if
//! one call runs longer than [`TIME_LIMIT`].

use crate::code::{CodeProblem, Compiled};
use crate::convert::{from_lua, to_lua};
use crate::events::{EventKinds, Queue, sender};
use crate::host::{Host, call_with, with_natives};
use crate::lua_components::{Field, LuaClass, PRELUDE, make, read_classes, swap_classes};
use crate::lust_program::{LustProgram, changes, located, prepare};
use crate::rust_component::RustKind;
use crate::ticking::{Instance, Running, RunningMap, Ticking};
use mlua::{Lua, Table, Value as LuaValue};
use reflect::{ComponentData, ComponentSchema, EntityId};
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, VecDeque};
use std::rc::Rc;
use std::time::{Duration, Instant};

/// The most memory game code may use.
pub const MEMORY_LIMIT: usize = 64 << 20;
/// The longest one call into game code may run before it's stopped.
pub const TIME_LIMIT: Duration = Duration::from_millis(100);

/// A failure while game code ran: which component, and where in the code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    /// The entity whose component failed (none while the program's own
    /// start ran).
    pub entity: Option<EntityId>,
    /// The component (empty while the program's own start ran).
    pub component: String,
    /// Where in the code, and why.
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

/// Game code, loaded and running.
pub struct Runtime {
    lua: Lua,
    /// The game's Lust, if it has any.
    lust: Option<LustProgram>,
    /// The game's Lua components, by name.
    lua_classes: BTreeMap<String, LuaClass>,
    /// The kinds of Rust component the game has, by name.
    rust: BTreeMap<String, RustKind>,
    running: RunningMap,
    deadline: Rc<Cell<Option<Instant>>>,
    /// Events on their way this tick.
    queue: Queue,
    /// What each event the Lust declares carries, for `send_event`.
    kinds: Rc<RefCell<Rc<EventKinds>>>,
    /// `send_event`, which Lust's `send` and Lua's `solis.send` call
    /// (none only if Luau couldn't make it, out of memory at the start).
    send: Option<mlua::Function>,
}

impl std::fmt::Debug for Runtime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Runtime")
            .field("lust", &self.lust.is_some())
            .field("lua", &self.lua_classes.len())
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
        // Both are Solis's own, tested code: failing here is a bug the
        // tests catch, and the runtime then simply has no Lua side.
        let _ = lua.load(PRELUDE).set_name("=solis").exec();
        let queue: Queue = Rc::new(RefCell::new(VecDeque::new()));
        let kinds = Rc::new(RefCell::new(Rc::new(EventKinds::new())));
        let send = sender(&lua, &queue, Rc::clone(&kinds)).ok();
        Self {
            lua,
            lust: None,
            lua_classes: BTreeMap::new(),
            rust: BTreeMap::new(),
            running: BTreeMap::new(),
            deadline,
            queue,
            kinds,
            send,
        }
    }

    /// Adds a kind of Rust component, kept across loads.
    pub fn add_rust(&mut self, kind: RustKind) {
        self.rust.insert(kind.schema.name.clone(), kind);
    }

    /// Loads the game's code: the Lust (its own start runs) and the Lua
    /// files, reading their components. Anything running is dropped. On a
    /// failure nothing is loaded.
    pub fn load(&mut self, code: &Compiled, host: &mut dyn Host) -> Result<(), Failure> {
        self.running.clear();
        self.lust = None;
        self.lua_classes.clear();
        self.queue.borrow_mut().clear();
        let (lust, classes) = self.prepare(code, host)?;
        self.commit(lust, classes);
        Ok(())
    }

    /// Swaps new code into the running game (LUST.md §21, `swap`): every
    /// running Lust and Lua component gets the new code in place, keeping
    /// each field's value by name (a field whose kind changed, or a new
    /// one, takes its starting value). Components gone from the code stop.
    /// Stopped components run again, since the code changed. On a failure
    /// (the new code's start fails) the old code keeps running. Rust
    /// components carry on.
    pub fn swap(&mut self, code: &Compiled, host: &mut dyn Host) -> Result<Swapped, Failure> {
        let (lust, classes) = self.prepare(code, host)?;
        let failed = |err: mlua::Error| Failure {
            entity: None,
            component: String::new(),
            at: crate::lua_components::lua_problem("", &err),
            stopped: false,
        };
        let mut swapped = Swapped::default();
        match (&self.lust, &lust) {
            (Some(old), Some(new)) => {
                changes(new, old.api.clone(), &mut swapped).map_err(failed)?;
                let swap: mlua::Function = new.api.get("swap").map_err(failed)?;
                let mut gone = Vec::new();
                for (key, running) in &mut self.running {
                    let Instance::Lust(table) = &running.instance else {
                        continue;
                    };
                    let (now, notes): (LuaValue, Table) =
                        swap.call(table.clone()).map_err(failed)?;
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
            }
            (None, Some(new)) => swapped.added.extend(new.fields.keys().cloned()),
            (Some(old), None) => {
                swapped.removed.extend(old.fields.keys().cloned());
                self.running
                    .retain(|_, r| !matches!(r.instance, Instance::Lust(_)));
            }
            (None, None) => {}
        }
        swap_classes(
            &self.lua,
            &self.lua_classes,
            &classes,
            &mut self.running,
            &mut swapped,
        )
        .map_err(failed)?;
        self.queue.borrow_mut().clear();
        self.commit(lust, classes);
        Ok(swapped)
    }

    /// Stops the game's Lust and Lua: their components stop and their code
    /// goes (the code was deleted). Rust components carry on.
    pub fn unload(&mut self) {
        self.lust = None;
        self.lua_classes.clear();
        *self.kinds.borrow_mut() = Rc::new(EventKinds::new());
        self.running
            .retain(|_, running| matches!(running.instance, Instance::Rust(_)));
    }

    /// The game code's components (Lust and Lua), as the editor shows
    /// them, sorted by name. Rust components' schemas come with their
    /// kinds.
    pub fn schemas(&self) -> Vec<ComponentSchema> {
        let mut all: Vec<ComponentSchema> = self
            .lust
            .iter()
            .flat_map(|p| p.schemas.iter().cloned())
            .chain(self.lua_classes.values().map(|c| c.schema.clone()))
            .collect();
        all.sort_by(|a, b| a.name.cmp(&b.name));
        all
    }

    /// Whether there's a component called `name` to run: the Lust's, a Lua
    /// one, or a Rust one.
    pub fn has_component(&self, name: &str) -> bool {
        self.rust.contains_key(name)
            || self.lua_classes.contains_key(name)
            || self
                .lust
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
                        .insert(key, Running::new(Instance::Rust(made), data));
                }
            }
            return Ok(());
        }
        let fail = |at: CodeProblem| Failure {
            entity: Some(entity),
            component: name.to_string(),
            at,
            stopped: true,
        };
        let lua_class = self.lua_classes.get(name);
        let lust = self.lust.as_ref();
        let locate = |err: mlua::Error| match (lua_class, lust) {
            (None, Some(program)) => located(&program.chunk, &err),
            _ => crate::lua_components::lua_problem("", &err),
        };
        let Some(fields) = lua_class
            .map(|c| &c.fields)
            .or_else(|| lust.and_then(|p| p.fields.get(name)))
        else {
            return Ok(());
        };
        if let Some(running) = self.running.get_mut(&key) {
            if let Instance::Lust(table) | Instance::Lua(table) = &running.instance {
                for field in fields {
                    let now = data.get(&field.name);
                    if now != running.seen.get(&field.name)
                        && let Some(value) = now.and_then(|v| to_lua(&self.lua, &field.kind, v))
                    {
                        table
                            .set(field.key.as_str(), value)
                            .map_err(|e| fail(locate(e)))?;
                    }
                }
            }
            running.seen = data.clone();
            return Ok(());
        }
        let instance = match (lua_class, lust) {
            (Some(class), _) => {
                Instance::Lua(make(&self.lua, class, entity, data).map_err(|e| fail(locate(e)))?)
            }
            (None, Some(program)) => Instance::Lust(
                self.make_lust(program, fields, entity, name, data, host)
                    .map_err(|e| fail(locate(e)))?,
            ),
            (None, None) => return Ok(()),
        };
        self.running.insert(key, Running::new(instance, data));
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
        let (table, fields) = match &running.instance {
            Instance::Rust(component) => return Some(component.values()),
            Instance::Lua(table) => (table, &self.lua_classes.get(name)?.fields),
            Instance::Lust(table) => (table, self.lust.as_ref()?.fields.get(name)?),
        };
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
        let ticking = Ticking {
            lua: &self.lua,
            deadline: &self.deadline,
            queue: &self.queue,
            chunk: self.lust.as_ref().map(|p| &p.chunk),
            events: self.lust.as_ref().map(|p| &*p.events),
        };
        let running = &mut self.running;
        // The natives are lent once for the whole tick; the time limit
        // watches each component's update on its own.
        let lent = with_natives(&self.lua, host, self.send.as_ref(), |lent| {
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

    /// The Lust and Lua of `code`, loaded in this runtime's Luau but not
    /// yet in use, with their component names checked against each other
    /// and the Rust ones.
    fn prepare(
        &self,
        code: &Compiled,
        host: &mut dyn Host,
    ) -> Result<(Option<LustProgram>, BTreeMap<String, LuaClass>), Failure> {
        let lust = match &code.lust {
            Some(chunk) => Some(prepare(&self.lua, &self.deadline, host, chunk)?),
            None => None,
        };
        let failed = |at: CodeProblem| Failure {
            entity: None,
            component: String::new(),
            at,
            stopped: true,
        };
        let classes = read_classes(&self.lua, &code.lua, |chunk| {
            call_with(&self.lua, &self.deadline, host, None, || {
                chunk.call::<()>(())
            })
        })
        .map_err(failed)?;
        for name in classes.keys() {
            let clash = if self.rust.contains_key(name) {
                Some("a Rust component")
            } else if lust.as_ref().is_some_and(|p| p.fields.contains_key(name)) {
                Some("a Lust component")
            } else {
                None
            };
            if let Some(other) = clash {
                return Err(failed(CodeProblem {
                    file: String::new(),
                    line: 0,
                    column: 0,
                    message: format!(
                        "the Lua component {name} has the same name as {other}; each needs a \
                         name of its own"
                    ),
                }));
            }
        }
        Ok((lust, classes))
    }

    /// Puts prepared code in use.
    fn commit(&mut self, lust: Option<LustProgram>, classes: BTreeMap<String, LuaClass>) {
        *self.kinds.borrow_mut() = lust
            .as_ref()
            .map_or_else(|| Rc::new(EventKinds::new()), |p| Rc::clone(&p.events));
        self.lust = lust;
        self.lua_classes = classes;
    }

    /// A new Lust component (Lust's `new`, its start values), with the
    /// area's values where they fit.
    fn make_lust(
        &self,
        program: &LustProgram,
        fields: &[Field],
        entity: EntityId,
        name: &str,
        data: &ComponentData,
        host: &mut dyn Host,
    ) -> mlua::Result<Table> {
        let new: mlua::Function = program.api.get("new")?;
        let table: Table = call_with(&self.lua, &self.deadline, host, self.send.as_ref(), || {
            new.call((name, entity.to_string()))
        })?;
        for field in fields {
            if let Some(value) = data
                .get(&field.name)
                .and_then(|v| to_lua(&self.lua, &field.kind, v))
            {
                table.set(field.key.as_str(), value)?;
            }
        }
        Ok(table)
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
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

#[cfg(test)]
#[path = "runtime_lua_tests.rs"]
mod lua_tests;
