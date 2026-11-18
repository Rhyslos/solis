//! A component's timers (stage 1, timers; decision 0124): `startTimer`,
//! `startTimerEvery`, `startTimerAtHour`, `stopTimer`, `timerRunning` and
//! `timeLeft`, on the component's own entity. A timer comes as its name
//! (`DoorOpens` for `Timer.DoorOpens`); Lua and Rust name it by that text.
//! A mistake (a timer started on another entity, a time that isn't one)
//! stops with its line, never a silent nothing (the guidelines' rule 11).

use super::Scope;
use crate::convert::entity_of;
use crate::host::Lent;
use crate::native_values::native;
use crate::timers::{Timing, valid_name};
use mlua::{Table, Value as LuaValue};
use reflect::EntityId;

/// Lends the timers' natives into `natives`, each answered by `h`.
pub(super) fn lend<'s>(
    scope: &'s Scope<'s, '_>,
    natives: &Table,
    h: &'s Lent<'_, '_>,
) -> mlua::Result<()> {
    type Start = fn(f64) -> Timing;
    let starts: [(&'static str, &'static str, Start); 3] = [
        ("entity_startTimer", "startTimer", Timing::After),
        ("entity_startTimerEvery", "startTimerEvery", Timing::Every),
        (
            "entity_startTimerAtHour",
            "startTimerAtHour",
            Timing::AtHour,
        ),
    ];
    for (key, call, timing) in starts {
        natives.raw_set(
            key,
            native(
                scope,
                key,
                move |_, (which, name, time): (LuaValue, mlua::LuaString, f64)| {
                    let (entity, name) = owner(key, &which, &name)?;
                    h.borrow_mut()
                        .start_timer(entity, &name, timing(time))
                        .map_err(|why| mistake(call, &why))
                },
            )?,
        )?;
    }
    natives.raw_set(
        "entity_stopTimer",
        native(
            scope,
            "entity_stopTimer",
            move |_, (which, name): (LuaValue, mlua::LuaString)| {
                let (entity, name) = owner("entity_stopTimer", &which, &name)?;
                h.borrow_mut()
                    .stop_timer(entity, &name)
                    .map_err(|why| mistake("stopTimer", &why))
            },
        )?,
    )?;
    natives.raw_set(
        "entity_timerRunning",
        native(
            scope,
            "entity_timerRunning",
            move |_, (which, name): (LuaValue, mlua::LuaString)| {
                let (entity, name) = owner("entity_timerRunning", &which, &name)?;
                h.borrow()
                    .timer_left(entity, &name)
                    .map(|left| left.is_some())
                    .map_err(|why| mistake("timerRunning", &why))
            },
        )?,
    )?;
    natives.raw_set(
        "entity_timeLeft",
        native(
            scope,
            "entity_timeLeft",
            move |_, (which, name): (LuaValue, mlua::LuaString)| {
                let (entity, name) = owner("entity_timeLeft", &which, &name)?;
                h.borrow()
                    .timer_left(entity, &name)
                    .map(|left| left.unwrap_or(0.0))
                    .map_err(|why| mistake("timeLeft", &why))
            },
        )?,
    )?;
    Ok(())
}

/// The entity a timer call was made on and the timer's name, or the
/// mistake, named as Lua writes the call (only Lua can make it: Lust's
/// entity is checked, and its names are its `Timer` state's options):
/// nothing to call it on, or a name a timer can't have.
fn owner(key: &str, which: &LuaValue, name: &mlua::LuaString) -> mlua::Result<(EntityId, String)> {
    let call = crate::describe::NATIVES
        .iter()
        .find(|native| native.key() == key)
        .map_or_else(|| key.to_string(), crate::native_values::written);
    let Some(entity) = entity_of(which) else {
        return Err(mlua::Error::runtime(format!(
            "{call} was called on nothing: call it inside a component, on the component's own \
             entity (self.me)."
        )));
    };
    let name = name.to_str()?.to_string();
    if !valid_name(&name) {
        return Err(mlua::Error::runtime(format!(
            "{call} wants a timer's name: a capital letter, then letters, digits and _ \
             (\"DoorOpens\"), at most {} long, and \"{}\" isn't one.",
            crate::timers::MAX_NAME,
            crate::luau::capped(&name, crate::timers::MAX_NAME + 8)
        )));
    }
    Ok((entity, name))
}

/// A timer call's mistake, as game code's error.
fn mistake(call: &str, why: &str) -> mlua::Error {
    mlua::Error::runtime(format!("{call}: {why}."))
}
