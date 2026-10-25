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
use crate::convert::from_lua;
use crate::events::{Event, MAX_DELIVERIES};
use crate::events::{EventKinds, Queue, sender};
use crate::host::{Host, call_with, with_natives};
use crate::lua_components::{LuaClass, compile_files, lua_problem, read_classes, swap_classes};
use crate::luau::Luau;
pub use crate::luau::{LOAD_LIMIT, MEMORY_LIMIT, TICK_LIMIT, TIME_LIMIT};
use crate::lust_program::{self, LustProgram, changes, located};
use crate::rust_component::RustKind;
use crate::syncing::Syncing;
use crate::ticking::{Instance, Phase, RunningMap, Ticking, caught, plain};
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

    /// Every running component's values, by entity then name (a save
    /// game, decision 0038).
    pub fn all_values(&self) -> Vec<(EntityId, String, ComponentData)> {
        self.running
            .keys()
            .filter_map(|(entity, name)| {
                Some((*entity, name.clone(), self.values(*entity, name)?))
            })
            .collect()
    }

    /// Whether no components are running.
    pub fn is_empty(&self) -> bool {
        self.running.is_empty()
    }

    /// Makes `entity`'s component `name` run, with the area's `data` (a
    /// missing or wrong value keeps the starting one), or gives it the
    /// changed fields if it's running already. Does nothing for a
    /// component there's no code for. See [`Runtime::sync_many`].
    pub fn sync(
        &mut self,
        entity: EntityId,
        name: &str,
        data: &ComponentData,
        host: &mut dyn Host,
    ) -> Result<(), Failure> {
        match self.sync_many([(entity, name, data)], host).pop() {
            Some(failure) => Err(failure),
            None => Ok(()),
        }
    }

    /// [`Runtime::sync`] for many components at once, with the natives
    /// lent once and all the game code it runs (new components' starts)
    /// sharing [`LOAD_LIMIT`]. Gives back the failures.
    pub fn sync_many<'a>(
        &mut self,
        components: impl IntoIterator<Item = (EntityId, &'a str, &'a ComponentData)>,
        host: &mut dyn Host,
    ) -> Vec<Failure> {
        let code = Syncing {
            luau: &self.luau,
            lust: self.lust.as_ref(),
            lua_classes: &self.lua_classes,
            rust: &self.rust,
            left_out: &self.left_out,
        };
        let running = &mut self.running;
        let mut failures = Vec::new();
        let lust_env = self.lust.as_ref().map(|p| &p.env);
        let luau = &self.luau;
        let lent = luau.clock.within(LOAD_LIMIT, || {
            with_natives(luau, lust_env, host, Some(&self.send), |lent| {
                for (entity, name, data) in components {
                    // What a new component's start changes lands in order.
                    lent.borrow_mut().at(&crate::ticking::Order {
                        major: entity.0.raw(),
                        component: name.into(),
                        seq: 0,
                    });
                    if let Err(failure) = code.sync(running, entity, name, data) {
                        failures.push(failure);
                    }
                }
                Ok(())
            })
        });
        if let Err(err) = lent {
            failures.push(Failure {
                entity: None,
                component: String::new(),
                at: lua_problem("", &err),
                stopped: false,
            });
        }
        failures
    }

    /// Stops `entity`'s running components that `keep` says no to (gone
    /// from the entity), or all of them if it's gone.
    pub fn retain_on(&mut self, entity: EntityId, mut keep: impl FnMut(&str) -> bool) {
        let gone: Vec<(EntityId, String)> = self
            .running
            .range((entity, String::new())..)
            .take_while(|((e, _), _)| *e == entity)
            .filter(|((_, name), _)| !keep(name))
            .map(|(key, _)| key.clone())
            .collect();
        for key in gone {
            self.running.remove(&key);
        }
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

    /// A checksum of every running component's values, each summed on
    /// its own (with its entity and name) and added up, so components
    /// split between runtimes add up the same (see `Game::checksum`).
    /// A stopped component counts as stopped.
    pub fn checksum(&self) -> u64 {
        let mut total = 0_u64;
        for ((entity, name), running) in &self.running {
            let mut sum = reflect::Checksum::new();
            sum.entity(*entity);
            sum.bytes(name.as_bytes());
            sum.u64(u64::from(running.stopped));
            if let Some(data) = self.values(*entity, name) {
                sum.data(&data);
            }
            total = total.wrapping_add(sum.finish());
        }
        total
    }

    /// Runs one tick of `dt` seconds: every running component's `update`,
    /// entities in ID order and components in name order, then the events
    /// they sent, in rounds. A component that fails stops (until the code
    /// changes), and its failure is given back; the others carry on. A
    /// tick that runs past [`TICK_LIMIT`] is cut short, and asks for a
    /// pause. What game code changes reaches `host` at once (one runtime
    /// on one core; [`crate::Game`] runs several, decision 0033).
    pub fn tick(&mut self, dt: f64, host: &mut dyn Host) -> Ticked {
        self.start_tick();
        let told = self.told.get();
        let ticking = Ticking {
            luau: &self.luau,
            queue: &self.queue,
            chunk: self.lust.as_ref().map(|p| &p.chunk),
            lust: self.lust.as_ref(),
            events: self.lust.as_ref().map(|p| &*p.events),
            lua_classes: &self.lua_classes,
            kinds: self.lust.as_ref().map_or(&self.no_events, |p| &*p.events),
            unstopped: RefCell::default(),
        };
        let running = RefCell::new(&mut self.running);
        let lust_env = self.lust.as_ref().map(|p| &p.env);
        // The natives are lent once for the whole tick, all its rounds.
        let mut outcome = (Vec::new(), None);
        let lent = with_natives(&self.luau, lust_env, host, Some(&self.send), |lent| {
            outcome = in_rounds(
                || Ok(ticking.updates(lent, &mut running.borrow_mut(), dt)),
                |events| Ok(ticking.round(lent, &mut running.borrow_mut(), events)),
                told,
            );
            Ok(())
        });
        let (failures, mut broke) = outcome;
        if let Err(err) = lent {
            broke = Some(ticking.located(&err).to_string());
        }
        if failures.iter().any(is_flood) {
            self.told.set(true);
        }
        let over = self.finish_tick();
        Ticked {
            failures,
            pause: pause_for(over, broke),
        }
    }

    /// Starts a tick in phases: all of its game code shares
    /// [`TICK_LIMIT`].
    pub(crate) fn start_tick(&self) {
        self.luau.clock.start(TICK_LIMIT);
    }

    /// Ends a tick in phases; whether it ran past its budget (the rest of
    /// it waited, and events still waiting are dropped).
    pub(crate) fn finish_tick(&self) -> bool {
        let over = self.luau.clock.over();
        self.luau.clock.stop();
        if over {
            self.queue.borrow_mut().clear();
        }
        over
    }

    /// The updates of a tick in phases, with the natives lent from `host`.
    /// `Err`: the natives couldn't be lent (out of memory), why.
    pub(crate) fn run_updates(&mut self, dt: f64, host: &mut dyn Host) -> Result<Phase, String> {
        self.phase(host, |ticking, lent, running| {
            ticking.updates(lent, running, dt)
        })
    }

    /// One round of events of a tick in phases.
    pub(crate) fn run_round(
        &mut self,
        events: &[(u32, Event)],
        host: &mut dyn Host,
    ) -> Result<Phase, String> {
        self.phase(host, |ticking, lent, running| {
            ticking.round(lent, running, events)
        })
    }

    /// Runs `run` with the natives lent once.
    fn phase(
        &mut self,
        host: &mut dyn Host,
        run: impl FnOnce(&Ticking<'_>, &crate::host::Lent<'_, '_>, &mut RunningMap) -> Phase,
    ) -> Result<Phase, String> {
        let ticking = Ticking {
            luau: &self.luau,
            queue: &self.queue,
            chunk: self.lust.as_ref().map(|p| &p.chunk),
            lust: self.lust.as_ref(),
            events: self.lust.as_ref().map(|p| &*p.events),
            lua_classes: &self.lua_classes,
            kinds: self.lust.as_ref().map_or(&self.no_events, |p| &*p.events),
            unstopped: RefCell::default(),
        };
        let running = &mut self.running;
        let lust_env = self.lust.as_ref().map(|p| &p.env);
        let mut phase = None;
        with_natives(&self.luau, lust_env, host, Some(&self.send), |lent| {
            phase = Some(run(&ticking, lent, running));
            Ok(())
        })
        .map_err(|err| ticking.located(&err).to_string())?;
        Ok(phase.unwrap_or_default())
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
            Some(chunk) => Some((
                chunk,
                lust_program::compile(&self.luau, chunk, code.bytecode.lust.as_deref())?,
            )),
            None => None,
        };
        let files = compile_files(&self.luau, &code.lua, &code.bytecode.lua).map_err(failed)?;
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

/// A tick's updates, then its events in rounds (each round the events the
/// one before sent, so the first sent is the first delivered), until none
/// are left. `updates` and `round` give back what was sent and what failed,
/// in any order: they're put in the tick's order here. Past
/// [`MAX_DELIVERIES`] events, the rest are dropped with a failure (unless
/// `told` already). Gives back the failures, in order, and why the tick
/// couldn't run, if it couldn't.
pub(crate) fn in_rounds(
    updates: impl FnOnce() -> Result<Phase, String>,
    mut round: impl FnMut(&[(u32, Event)]) -> Result<Phase, String>,
    told: bool,
) -> (Vec<Failure>, Option<String>) {
    let mut failures = Vec::new();
    let take = |phase: Phase, failures: &mut Vec<Failure>| {
        let mut phase_failures = phase.failures;
        phase_failures.sort_by(|a, b| a.0.cmp(&b.0));
        failures.extend(phase_failures.into_iter().map(|(_, f)| f));
        let mut sent = phase.sent;
        sent.sort_by(|a, b| a.0.cmp(&b.0));
        sent.into_iter().map(|(_, e)| e).collect::<Vec<Event>>()
    };
    let mut events = match updates() {
        Ok(phase) => take(phase, &mut failures),
        Err(why) => return (failures, Some(why)),
    };
    let mut delivered = 0;
    while !events.is_empty() {
        if delivered + events.len() > MAX_DELIVERIES {
            let room = MAX_DELIVERIES - delivered;
            let dropped = events.len() - room;
            events.truncate(room);
            if !told {
                failures.push(flood(dropped));
            }
            if events.is_empty() {
                break;
            }
        }
        delivered += events.len();
        let numbered: Vec<(u32, Event)> = events
            .into_iter()
            .enumerate()
            .map(|(i, e)| (i as u32, e))
            .collect();
        events = match round(&numbered) {
            Ok(phase) => take(phase, &mut failures),
            Err(why) => return (failures, Some(why)),
        };
    }
    (failures, None)
}

/// The failure saying too many events were sent in one tick.
fn flood(dropped: usize) -> Failure {
    Failure {
        entity: None,
        component: String::new(),
        at: plain(format!(
            "more than {MAX_DELIVERIES} events were sent in one tick, so the last {dropped} \
             were dropped. Do components keep sending events to each other? (This isn't said \
             again until the code changes.)"
        )),
        stopped: false,
    }
}

/// Whether `failure` is the one saying too many events were sent.
pub(crate) fn is_flood(failure: &Failure) -> bool {
    failure.entity.is_none() && failure.at.message.starts_with("more than ")
}

/// Why a tick asks for a pause: it ran past its budget, or couldn't run.
pub(crate) fn pause_for(over: bool, broke: Option<String>) -> Option<String> {
    if over {
        Some(format!(
            "The game's code took longer than {} ms in one tick, so the game paused (the rest \
             of that tick waited). Is something slow, or a loop that never ends?",
            TICK_LIMIT.as_millis()
        ))
    } else {
        broke
            .map(|why| format!("The game's code couldn't run this tick, so the game paused: {why}"))
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

#[cfg(test)]
#[path = "runtime_hostile_tests.rs"]
mod hostile_tests;

#[cfg(test)]
#[path = "runtime_seams_tests.rs"]
mod seams_tests;

#[cfg(test)]
#[path = "runtime_speed_tests.rs"]
mod speed_tests;
