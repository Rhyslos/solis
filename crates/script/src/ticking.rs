//! One tick of the running game: every component's `update` (Lust, Lua
//! and Rust alike, in one order), then delivering the events they sent.

use crate::code::CodeProblem;
use crate::events::{Event, EventKinds, MAX_DELIVERIES, Queue, arguments, values_table};
use crate::host::{Lent, limited};
use crate::lua_components::lua_problem;
use crate::lust_program::located;
use crate::runtime::Failure;
use crate::rust_component::{Context, RustComponent};
use mlua::{Lua, Table, Value as LuaValue};
use reflect::{ComponentData, EntityId};
use std::cell::Cell;
use std::collections::BTreeMap;
use std::time::Instant;

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
        }
    }
}

/// What a tick needs, apart from the running components.
pub(crate) struct Ticking<'a> {
    pub(crate) lua: &'a Lua,
    pub(crate) deadline: &'a Cell<Option<Instant>>,
    pub(crate) queue: &'a Queue,
    /// The Lust's chunk, to point at the Lust when it fails.
    pub(crate) chunk: Option<&'a lust::LuaChunk>,
    /// The events the Lust declares.
    pub(crate) events: Option<&'a EventKinds>,
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

    /// Where an error from `instance`'s code came from.
    fn located_in(&self, instance: &Instance, err: &mlua::Error) -> CodeProblem {
        match instance {
            Instance::Lua(_) => lua_problem("", err),
            _ => self.located(err),
        }
    }

    /// Runs one component's `update`.
    pub(crate) fn update(
        &self,
        lent: &Lent<'_, '_>,
        me: EntityId,
        component: &mut Running,
        dt: f64,
    ) -> Result<(), CodeProblem> {
        match &mut component.instance {
            Instance::Lust(table) | Instance::Lua(table) => {
                let update = match table.get::<LuaValue>("update") {
                    Ok(LuaValue::Function(f)) => f,
                    _ => return Ok(()),
                };
                let table = table.clone();
                limited(self.deadline, || update.call::<()>((table, dt)))
                    .map_err(|err| self.located_in(&component.instance, &err))
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
    pub(crate) fn deliver(&self, lent: &Lent<'_, '_>, running: &mut RunningMap) -> Vec<Failure> {
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
                    at: plain(format!(
                        "more than {MAX_DELIVERIES} events were sent in one tick, so the last \
                         {dropped} were dropped. Do components keep sending events to each \
                         other?"
                    )),
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
                let values = values_table(self.lua, kinds, &event.values)
                    .ok_or_else(|| wrong(kinds.len()))?;
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
            Instance::Lua(table) => {
                let on = match table.get::<LuaValue>(format!("on_{}", event.name)) {
                    Ok(LuaValue::Function(f)) => f,
                    _ => return Ok(()),
                };
                let mut args = arguments(self.lua, declared.map(Vec::as_slice), &event.values)
                    .ok_or_else(|| wrong(declared.map_or(0, Vec::len)))?;
                args.push_front(LuaValue::Table(table.clone()));
                limited(self.deadline, || on.call::<()>(args)).map_err(|err| lua_problem("", &err))
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

/// A failure with no place in the code.
fn plain(message: String) -> CodeProblem {
    CodeProblem {
        file: String::new(),
        line: 0,
        column: 0,
        message,
    }
}

/// Runs a Rust component's code, turning a panic into a failure rather
/// than a crash.
fn caught(run: impl FnOnce()) -> Result<(), CodeProblem> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(run)).map_err(|why| {
        plain(
            why.downcast_ref::<&str>()
                .map(|s| (*s).to_string())
                .or_else(|| why.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "it stopped unexpectedly".to_string()),
        )
    })
}
