//! One tick of the running game: every component's `update` (Lust, Lua
//! and Rust alike, in one order), then delivering the events they sent.

use crate::code::CodeProblem;
use crate::events::{Event, EventKinds, Queue, arguments, values_table};
use crate::host::Lent;
use crate::lua_components::{LuaClass, lua_problem};
use crate::luau::Luau;
use crate::lust_program::{LustProgram, located};
use crate::runtime::Failure;
use crate::rust_component::{Context, RustComponent};
use mlua::{Function, Table, Value as LuaValue};
use reflect::{ComponentData, EntityId};
use std::cell::RefCell;
use std::collections::BTreeMap;

/// What a running component is.
pub(crate) enum Instance {
    /// Lust's: its table in Luau.
    Lust(Table),
    /// Lua's: its table in Luau, with its class as its metatable.
    Lua(Table),
    /// Written in Rust.
    Rust(Box<dyn RustComponent>),
}

/// One running component.
pub(crate) struct Running {
    pub(crate) instance: Instance,
    /// The area's values it was last given, to send only what changes.
    pub(crate) seen: ComponentData,
    /// Stopped after a failure, until the code changes.
    pub(crate) stopped: bool,
    /// Its failures handling events go unsaid, after the first, until the
    /// code changes: one that fails on every event would fill the
    /// Console.
    pub(crate) quiet: bool,
}

/// The running components, by entity and then name: the order they run in.
pub(crate) type RunningMap = BTreeMap<(EntityId, String), Running>;

impl Running {
    /// A newly made running component.
    pub(crate) fn new(instance: Instance, data: &ComponentData) -> Self {
        Self {
            instance,
            seen: data.clone(),
            stopped: false,
            quiet: false,
        }
    }

    /// The code changed: it runs again, and its failures are said again.
    pub(crate) fn fresh(&mut self) {
        self.stopped = false;
        self.quiet = false;
    }
}

/// What a tick needs, apart from the running components.
pub(crate) struct Ticking<'a> {
    pub(crate) luau: &'a Luau,
    pub(crate) queue: &'a Queue,
    /// The Lust's chunk, to point at the Lust when it fails.
    pub(crate) chunk: Option<&'a lust::LuaChunk>,
    /// The Lust, for its components' methods.
    pub(crate) lust: Option<&'a LustProgram>,
    /// The events the Lust declares.
    pub(crate) events: Option<&'a EventKinds>,
    /// The Lua components' classes, for which file a failure is in.
    pub(crate) lua_classes: &'a BTreeMap<String, LuaClass>,
    /// What every event carries, for Rust components' sends.
    pub(crate) kinds: &'a EventKinds,
    /// Failures that leave their component running (a Rust component's
    /// refused send), said once per component until the code changes.
    pub(crate) unstopped: RefCell<Vec<Failure>>,
}

/// Where something game code did belongs in the tick's order: during the
/// updates, by the entity (its ID) then the component (its name); while
/// delivering a round of events, by the event's number in the round then
/// the receiving component; then the order the component did things in.
/// Things done on any number of cores, put in this order, come out the
/// same (decision 0033).
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Order {
    /// The entity's ID (updates), or the event's number (a round).
    pub major: u64,
    /// The component's name.
    pub component: String,
    /// The order the component did it in.
    pub seq: u32,
}

/// What one phase of a tick (the updates, or a round of events) gave back.
#[derive(Debug, Default)]
pub(crate) struct Phase {
    /// The events sent, each with its place in the order.
    pub(crate) sent: Vec<(Order, Event)>,
    /// The components that failed.
    pub(crate) failures: Vec<(Order, Failure)>,
}

/// An event on its way to the components on its entity, with what they
/// all share made once.
pub(crate) struct Delivery {
    pub(crate) event: Event,
    /// The name of a Lua component's function for it: `on_Hit`.
    on: String,
    /// Its values as Lust takes them (`None`: they don't fit).
    values: std::cell::OnceCell<Option<Table>>,
}

/// A method of a Lua component, looked up the usual way (its
/// class's metatable may be game code's, so under the time limit).
fn method(table: &Table, name: &str) -> mlua::Result<Option<Function>> {
    Ok(match table.get::<LuaValue>(name)? {
        LuaValue::Function(f) => Some(f),
        _ => None,
    })
}

impl Ticking<'_> {
    /// Where in the game's code `err` (from Lust's code, or the engine's
    /// own) came from.
    pub(crate) fn located(&self, err: &mlua::Error) -> CodeProblem {
        match self.chunk {
            Some(chunk) => located(chunk, err),
            None => lua_problem("", err),
        }
    }

    /// Where an error from the code of the component `name` came from.
    fn located_in(&self, instance: &Instance, name: &str, err: &mlua::Error) -> CodeProblem {
        match instance {
            Instance::Lua(_) => {
                lua_problem(self.lua_classes.get(name).map_or("", |c| &c.path), err)
            }
            _ => self.located(err),
        }
    }

    /// Runs one component's `update`.
    pub(crate) fn update(
        &self,
        lent: &Lent<'_, '_>,
        (me, name): (EntityId, &str),
        component: &mut Running,
        dt: f64,
    ) -> Result<(), CodeProblem> {
        match &mut component.instance {
            Instance::Lust(table) => {
                let Some(update) = self
                    .lust
                    .and_then(|p| p.methods.get(name))
                    .and_then(|m| m.update.as_ref())
                else {
                    return Ok(());
                };
                self.luau
                    .clock
                    .limited(|| update.call::<()>((&*table, dt)))
                    .map_err(|err| self.located(&err))
            }
            Instance::Lua(table) => {
                let table = table.clone();
                self.luau
                    .clock
                    .limited(|| match method(&table, "update")? {
                        Some(update) => update.call::<()>((table.clone(), dt)),
                        None => Ok(()),
                    })
                    .map_err(|err| self.located_in(&component.instance, name, &err))
            }
            Instance::Rust(rust) => {
                let mut host = lent.borrow_mut();
                let mut cx = Context::new(me, &mut **host, self.queue, self.kinds);
                let ran = caught(|| rust.update(&mut cx, dt));
                let refused = cx.refused.take();
                self.refused(me, name, &mut component.quiet, refused);
                ran
            }
        }
    }

    /// Notes a Rust component's refused send, once until the code
    /// changes.
    fn refused(&self, me: EntityId, name: &str, quiet: &mut bool, why: Option<String>) {
        let Some(why) = why else {
            return;
        };
        if std::mem::replace(quiet, true) {
            return;
        }
        self.unstopped.borrow_mut().push(Failure {
            entity: Some(me),
            component: name.to_string(),
            at: plain(format!(
                "an event it sent wasn't sent: {why} (Its later mistakes with events aren't \
                 said until the code changes.)"
            )),
            stopped: false,
        });
    }

    /// Runs every running component's `update`, in order, telling the
    /// host which one runs (so what it changes is put in order), and gives
    /// back the events they sent and their failures, each with its place
    /// in the order. A component that fails stops. Once the budget is
    /// used up, the rest wait.
    pub(crate) fn updates(&self, lent: &Lent<'_, '_>, running: &mut RunningMap, dt: f64) -> Phase {
        let mut phase = Phase::default();
        for ((entity, name), component) in running.iter_mut() {
            if self.luau.clock.over() {
                break;
            }
            if component.stopped {
                continue;
            }
            let at = Order {
                major: entity.0.raw(),
                component: name.clone(),
                seq: 0,
            };
            lent.borrow_mut().at(&at);
            let result = self.update(lent, (*entity, name), component, dt);
            self.collect_sent(&at, &mut phase);
            if let Err(problem) = result {
                if self.luau.clock.over() {
                    break;
                }
                component.stopped = true;
                phase.failures.push((
                    at,
                    Failure {
                        entity: Some(*entity),
                        component: name.clone(),
                        at: problem,
                        stopped: true,
                    },
                ));
            }
        }
        phase
    }

    /// Delivers one round of `events` (each with its number in the round)
    /// to each running component on their entity, in order of their names,
    /// telling the host which one runs; gives back what they sent and
    /// their failures, in order. A failure handling one is reported once,
    /// until the code changes, and the component keeps running. Once the
    /// budget is used up, the rest wait.
    pub(crate) fn round(
        &self,
        lent: &Lent<'_, '_>,
        running: &mut RunningMap,
        events: &[(u32, Event)],
    ) -> Phase {
        let mut phase = Phase::default();
        for (number, event) in events {
            if self.luau.clock.over() {
                break;
            }
            let delivery = Delivery {
                on: format!("on_{}", event.name),
                values: std::cell::OnceCell::new(),
                event: event.clone(),
            };
            let from = (event.to, String::new());
            for ((entity, name), component) in running.range_mut(from..) {
                if *entity != event.to {
                    break;
                }
                if component.stopped {
                    continue;
                }
                let at = Order {
                    major: u64::from(*number),
                    component: name.clone(),
                    seq: 0,
                };
                lent.borrow_mut().at(&at);
                let result = self.handle(lent, (*entity, name), component, &delivery);
                self.collect_sent(&at, &mut phase);
                let Err(mut problem) = result else {
                    continue;
                };
                if self.luau.clock.over() {
                    // Out of time: said once, for the whole tick.
                    break;
                }
                if component.quiet {
                    continue;
                }
                component.quiet = true;
                problem.message = format!(
                    "couldn't take the event {}: {} (Its later failures with events aren't \
                     said until the code changes.)",
                    event.name, problem.message
                );
                phase.failures.push((
                    at,
                    Failure {
                        entity: Some(*entity),
                        component: name.clone(),
                        at: problem,
                        stopped: false,
                    },
                ));
            }
        }
        phase
    }

    /// Takes what the component at `at` just sent out of the queue, in the
    /// order it sent it.
    fn collect_sent(&self, at: &Order, phase: &mut Phase) {
        for failure in self.unstopped.take() {
            phase.failures.push((at.clone(), failure));
        }
        let mut queue = self.queue.borrow_mut();
        let mut seq = 0;
        while let Some(event) = queue.pop() {
            phase.sent.push((Order { seq, ..at.clone() }, event));
            seq += 1;
        }
    }

    /// Gives one component an event.
    fn handle(
        &self,
        lent: &Lent<'_, '_>,
        (me, name): (EntityId, &str),
        component: &mut Running,
        delivery: &Delivery,
    ) -> Result<(), CodeProblem> {
        let event = &delivery.event;
        let declared = self.events.and_then(|e| e.get(&event.name));
        let wrong = |declared: usize| {
            plain(format!(
                "it carries {} values, not the {declared} it was declared with, or one of the \
                 wrong kind",
                event.values.len()
            ))
        };
        match &mut component.instance {
            Instance::Lust(table) => {
                // Only events the Lust declares reach Lust components.
                let Some(kinds) = declared else {
                    return Ok(());
                };
                let Some(handle) = self
                    .lust
                    .and_then(|p| p.methods.get(name))
                    .and_then(|m| m.handle.as_ref())
                else {
                    return Ok(());
                };
                // One list of values for every Lust component the event
                // reaches.
                let values = delivery
                    .values
                    .get_or_init(|| values_table(&self.luau.lua, kinds, &event.values))
                    .as_ref()
                    .ok_or_else(|| wrong(kinds.len()))?;
                let event_name = event.name.as_str();
                self.luau
                    .clock
                    .limited(|| {
                        handle
                            .call::<bool>((&*table, event_name, values))
                            .map(|_| ())
                    })
                    .map_err(|err| self.located(&err))
            }
            Instance::Lua(table) => {
                let lua = &self.luau.lua;
                let table = table.clone();
                let file = self.lua_classes.get(name).map_or("", |c| c.path.as_str());
                let on = self
                    .luau
                    .clock
                    .limited(|| method(&table, &delivery.on))
                    .map_err(|err| lua_problem(file, &err))?;
                let Some(on) = on else {
                    return Ok(());
                };
                let mut args = arguments(lua, declared.map(Vec::as_slice), &event.values)
                    .ok_or_else(|| wrong(declared.map_or(0, Vec::len)))?;
                args.push_front(LuaValue::Table(table));
                self.luau
                    .clock
                    .limited(|| on.call::<()>(args))
                    .map_err(|err| lua_problem(file, &err))
            }
            Instance::Rust(rust) => {
                let mut host = lent.borrow_mut();
                let mut cx = Context::new(me, &mut **host, self.queue, self.kinds);
                let ran = caught(|| {
                    rust.handle(&mut cx, event);
                });
                let refused = cx.refused.take();
                self.refused(me, name, &mut component.quiet, refused);
                ran
            }
        }
    }
}

/// A failure with no place in the code.
pub(crate) fn plain(message: String) -> CodeProblem {
    CodeProblem {
        file: String::new(),
        line: 0,
        column: 0,
        message,
    }
}

/// Runs a Rust component's code, turning a panic into a failure rather
/// than a crash.
pub(crate) fn caught<R>(run: impl FnOnce() -> R) -> Result<R, CodeProblem> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(run)).map_err(|why| {
        plain(
            why.downcast_ref::<&str>()
                .map(|s| (*s).to_string())
                .or_else(|| why.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "it stopped unexpectedly".to_string()),
        )
    })
}
