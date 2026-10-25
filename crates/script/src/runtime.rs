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
//! one call runs longer than [`TIME_LIMIT`]; a whole tick that runs past
//! [`TICK_LIMIT`] is cut short and asks for the game to pause. See
//! [`crate::luau`].

use crate::code::{CodeProblem, Compiled};
use crate::convert::{from_lua, to_lua};
use crate::events::{EventKinds, Queue, sender};
use crate::host::{Host, call_with, with_natives};
use crate::lua_components::{
    Field, LuaClass, compile_files, lua_problem, make, read_classes, swap_classes,
};
use crate::luau::Luau;
pub use crate::luau::{LOAD_LIMIT, MEMORY_LIMIT, TICK_LIMIT, TIME_LIMIT};
use crate::lust_program::{self, LustProgram, changes, located};
use crate::rust_component::RustKind;
use crate::ticking::{Instance, Running, RunningMap, Ticking, caught, plain};
use mlua::{Table, Value as LuaValue};
use reflect::{ComponentData, ComponentSchema, EntityId};
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

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

/// What one tick did (see [`Runtime::tick`]).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Ticked {
    /// The components that failed, and why.
    pub failures: Vec<Failure>,
    /// Why the game should pause, in plain words, if it should: the tick
    /// ran past [`TICK_LIMIT`] (the rest of it waited), or couldn't run
    /// at all. Running on would only do the same again, slowly.
    pub pause: Option<String>,
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
    luau: Luau,
    /// The game's Lust, if it has any.
    lust: Option<LustProgram>,
    /// The game's Lua components, by name.
    lua_classes: BTreeMap<String, LuaClass>,
    /// The kinds of Rust component the game has, by name.
    rust: BTreeMap<String, RustKind>,
    running: RunningMap,
    /// Events on their way this tick.
    queue: Queue,
    /// What each event the Lust declares carries, for `send_event`.
    kinds: Rc<RefCell<Rc<EventKinds>>>,
    /// `send_event`, which Lust's `send` and Lua's `solis.send` call.
    send: mlua::Function,
    /// Whether too many events in a tick was said, since the code changed.
    told: Cell<bool>,
    /// What events carry when there's no Lust: nothing declared.
    no_events: EventKinds,
    /// Names whose Lust or Lua components don't run (the editor refused
    /// them: the name is taken by one of its own, or a Rust component).
    left_out: std::collections::BTreeSet<String>,
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
    /// An empty runtime, with its memory and time limits set. Fails only
    /// if Luau can't set itself up (out of memory).
    pub fn new() -> Result<Self, Failure> {
        let failed = |err: mlua::Error| Failure {
            entity: None,
            component: String::new(),
            at: plain(format!("the game's code couldn't be set up to run: {err}")),
            stopped: true,
        };
        let luau = Luau::new().map_err(failed)?;
        let queue: Queue = Rc::default();
        let kinds = Rc::new(RefCell::new(Rc::new(EventKinds::new())));
        let send = sender(&luau.lua, &queue, Rc::clone(&kinds)).map_err(failed)?;
        Ok(Self {
            luau,
            lust: None,
            lua_classes: BTreeMap::new(),
            rust: BTreeMap::new(),
            running: BTreeMap::new(),
            queue,
            kinds,
            send,
            told: Cell::new(false),
            no_events: EventKinds::new(),
            left_out: std::collections::BTreeSet::new(),
        })
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
    /// Stopped components run again, since the code changed. If the new
    /// code's start fails, the old code keeps running. If giving the
    /// running components the new code fails partway (out of time or
    /// memory), they all stop, and the next [`Runtime::sync`] starts them
    /// again from the area's values, with a note saying so. Rust
    /// components carry on.
    pub fn swap(&mut self, code: &Compiled, host: &mut dyn Host) -> Result<Swapped, Failure> {
        let (lust, classes) = self.prepare(code, host)?;
        let mut swapped = Swapped::default();
        if let (Some(old), Some(new)) = (&self.lust, &lust) {
            // Only reads both programs: on a failure, nothing changed yet.
            changes(&self.luau, new, old.api.clone(), &mut swapped).map_err(|err| Failure {
                entity: None,
                component: String::new(),
                at: located(&new.chunk, &err),
                stopped: false,
            })?;
        }
        let luau = &self.luau;
        let running = &mut self.running;
        let old_classes = &self.lua_classes;
        let old_lust = self.lust.as_ref();
        let in_place = luau.clock.within(LOAD_LIMIT, || {
            let lent = lust.as_ref().map(|p| &p.env);
            with_natives(luau, lent, host, Some(&luau.discard), |_| {
                swap_lust(luau, old_lust, lust.as_ref(), running, &mut swapped)?;
                luau.clock.limited(|| {
                    swap_classes(&luau.lua, old_classes, &classes, running, &mut swapped)
                })
            })
        });
        if let Err(err) = in_place {
            self.running
                .retain(|_, r| matches!(r.instance, Instance::Rust(_)));
            swapped.notes.push(format!(
                "The running components couldn't all take the new code ({}), so they started \
                 again from the area's values.",
                lua_problem("", &err).message
            ));
        }
        for running in self.running.values_mut() {
            if !matches!(running.instance, Instance::Rust(_)) {
                running.fresh();
            }
        }
        self.told.set(false);
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

    /// Whether there's a component called `name` to run: a Rust one, or
    /// the Lust's or a Lua one not left out.
    pub fn has_component(&self, name: &str) -> bool {
        self.rust.contains_key(name) || self.code_component(name)
    }

    /// Whether the Lust or a Lua file has a component called `name` that
    /// isn't left out.
    fn code_component(&self, name: &str) -> bool {
        !self.left_out.contains(name)
            && (self.lua_classes.contains_key(name)
                || self
                    .lust
                    .as_ref()
                    .is_some_and(|p| p.fields.contains_key(name)))
    }

    /// The Lust and Lua components called one of `names` won't run (the
    /// editor refused them: the name is one of the engine's own
    /// components, or a Rust one's). Rust components by those names still
    /// do. Kept across loads; any running stop.
    pub fn leave_out(&mut self, names: std::collections::BTreeSet<String>) {
        self.running.retain(|(_, name), r| {
            matches!(r.instance, Instance::Rust(_)) || !names.contains(name)
        });
        self.left_out = names;
    }

    /// The file a Lua component is written in.
    pub fn lua_file(&self, name: &str) -> Option<&str> {
        self.lua_classes.get(name).map(|c| c.path.as_str())
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
                    running.seen = data.clone();
                    if let Instance::Rust(component) = &mut running.instance
                        && !running.stopped
                        && let Err(at) = caught(|| component.set(&changed))
                    {
                        running.stopped = true;
                        return Err(Failure {
                            entity: Some(entity),
                            component: name.to_string(),
                            at,
                            stopped: true,
                        });
                    }
                }
                None => {
                    let made = caught(|| (kind.make)(data)).map_err(|at| Failure {
                        entity: Some(entity),
                        component: name.to_string(),
                        at,
                        stopped: true,
                    })?;
                    self.running
                        .insert(key, Running::new(Instance::Rust(made), data));
                }
            }
            return Ok(());
        }
        if self.left_out.contains(name) {
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
            (class, _) => lua_problem(class.map_or("", |c| &c.path), &err),
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
                        && let Some(value) =
                            now.and_then(|v| to_lua(&self.luau.lua, &field.kind, v))
                    {
                        // Raw: the field is the component's own, and its
                        // metatable is game code's.
                        table
                            .raw_set(field.key.as_str(), value)
                            .map_err(|e| fail(locate(e)))?;
                    }
                }
            }
            running.seen = data.clone();
            return Ok(());
        }
        let instance = match (lua_class, lust) {
            (Some(class), _) => Instance::Lua(
                make(&self.luau.lua, class, entity, data).map_err(|e| fail(locate(e)))?,
            ),
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
            Instance::Rust(component) => return caught(|| component.values()).ok(),
            Instance::Lua(table) => (table, &self.lua_classes.get(name)?.fields),
            Instance::Lust(table) => (table, self.lust.as_ref()?.fields.get(name)?),
        };
        let mut data = ComponentData::default();
        for field in fields {
            if let Ok(raw) = table.raw_get::<LuaValue>(field.key.as_str())
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
    /// and its failure is given back; the others carry on. A tick that
    /// runs past [`TICK_LIMIT`] is cut short, and asks for a pause.
    pub fn tick(&mut self, dt: f64, host: &mut dyn Host) -> Ticked {
        let mut ticked = Ticked::default();
        let ticking = Ticking {
            luau: &self.luau,
            queue: &self.queue,
            chunk: self.lust.as_ref().map(|p| &p.chunk),
            events: self.lust.as_ref().map(|p| &*p.events),
            told: &self.told,
            lua_classes: &self.lua_classes,
            kinds: self.lust.as_ref().map_or(&self.no_events, |p| &*p.events),
            unstopped: RefCell::default(),
        };
        let running = &mut self.running;
        let failures = &mut ticked.failures;
        let lust_env = self.lust.as_ref().map(|p| &p.env);
        let clock = &self.luau.clock;
        // The natives are lent once for the whole tick; the time limit
        // watches each component's update on its own, and the tick's
        // budget all of them.
        let (lent, over) = clock.within(TICK_LIMIT, || {
            let lent = with_natives(&self.luau, lust_env, host, Some(&self.send), |lent| {
                for ((entity, name), component) in running.iter_mut() {
                    if clock.over() {
                        break;
                    }
                    if !component.stopped
                        && let Err(at) = ticking.update(lent, (*entity, name), component, dt)
                    {
                        if clock.over() {
                            break;
                        }
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
            (lent, clock.over())
        });
        ticked.failures.extend(ticking.unstopped.take());
        if over {
            self.queue.borrow_mut().clear();
            ticked.pause = Some(format!(
                "The game's code took longer than {} ms in one tick, so the game paused (the \
                 rest of that tick waited). Is something slow, or a loop that never ends?",
                TICK_LIMIT.as_millis()
            ));
        } else if let Err(err) = lent {
            ticked.pause = Some(format!(
                "The game's code couldn't run this tick, so the game paused: {}",
                ticking.located(&err)
            ));
        }
        ticked
    }

    /// The Lust and Lua of `code`, loaded in this runtime's Luau but not
    /// yet in use, with their component names checked against each other
    /// and the Rust ones. Compiling isn't timed; the starts share
    /// [`LOAD_LIMIT`].
    fn prepare(
        &self,
        code: &Compiled,
        host: &mut dyn Host,
    ) -> Result<(Option<LustProgram>, BTreeMap<String, LuaClass>), Failure> {
        let failed = |at: CodeProblem| Failure {
            entity: None,
            component: String::new(),
            at,
            stopped: true,
        };
        let lust_code = match &code.lust {
            Some(chunk) => Some((chunk, lust_program::compile(&self.luau, chunk)?)),
            None => None,
        };
        let files = compile_files(&self.luau, &code.lua).map_err(failed)?;
        let luau = &self.luau;
        let (lust, classes) = luau.clock.within(LOAD_LIMIT, || {
            let lust = match lust_code {
                Some((chunk, compiled)) => {
                    Some(lust_program::prepare(luau, host, chunk, compiled)?)
                }
                None => None,
            };
            let classes = read_classes(luau, &files, |chunk| {
                call_with(luau, None, host, Some(&luau.discard), || {
                    chunk.call::<()>(())
                })
            })
            .map_err(failed)?;
            Ok::<_, Failure>((lust, classes))
        })?;
        // A name a Rust component has too is the editor's to refuse (see
        // `leave_out`): the Rust one runs.
        for (name, class) in &classes {
            if lust.as_ref().is_some_and(|p| p.fields.contains_key(name)) {
                let mut at = plain(format!(
                    "the Lua component {name} has the same name as a Lust component; each needs \
                     a name of its own"
                ));
                at.file.clone_from(&class.path);
                return Err(failed(at));
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
        self.told.set(false);
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
        let new: mlua::Function = program.api.raw_get("new")?;
        let table: Table = call_with(
            &self.luau,
            Some(&program.env),
            host,
            Some(&self.send),
            || new.call((name, entity.to_string())),
        )?;
        for field in fields {
            if let Some(value) = data
                .get(&field.name)
                .and_then(|v| to_lua(&self.luau.lua, &field.kind, v))
            {
                table.raw_set(field.key.as_str(), value)?;
            }
        }
        Ok(table)
    }
}

/// Gives every running Lust component the `new` Lust's code in place
/// (Lust's `swap`), or stops them all if the Lust is gone. Says what
/// changed in `swapped`.
fn swap_lust(
    luau: &Luau,
    old: Option<&LustProgram>,
    new: Option<&LustProgram>,
    running: &mut RunningMap,
    swapped: &mut Swapped,
) -> mlua::Result<()> {
    match (old, new) {
        (Some(_), Some(new)) => {
            let swap: mlua::Function = new.api.raw_get("swap")?;
            let mut gone = Vec::new();
            for (key, component) in running.iter_mut() {
                let Instance::Lust(table) = &component.instance else {
                    continue;
                };
                let (now, notes): (LuaValue, Table) =
                    luau.clock.limited(|| swap.call(table.clone()))?;
                for note in notes.sequence_values::<String>() {
                    swapped.notes.push(note?);
                }
                if now.is_nil() {
                    gone.push(key.clone());
                }
            }
            for key in gone {
                running.remove(&key);
            }
        }
        (None, Some(new)) => swapped.added.extend(new.fields.keys().cloned()),
        (Some(old), None) => {
            swapped.removed.extend(old.fields.keys().cloned());
            running.retain(|_, r| !matches!(r.instance, Instance::Lust(_)));
        }
        (None, None) => {}
    }
    Ok(())
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

#[cfg(test)]
#[path = "runtime_hostile_tests.rs"]
mod hostile_tests;

#[cfg(test)]
#[path = "runtime_seams_tests.rs"]
mod seams_tests;
