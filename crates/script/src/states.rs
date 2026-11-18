//! States that know when they start and stop (stage 1;
//! [decision 0126](../../../docs/decisions/0126-things-started-in-a-state-end-with-it.md)).
//!
//! A Lust component's own state (`state = Mood.Calm;` in its `has`) is
//! the tool: no new Lust words. Lust already ends the objects a thing
//! makes in a state when it leaves it (LUST.md §8); Solis does the same
//! for what a component starts in the engine while it's in an option:
//!
//! - **its timers, tweens and looping sounds belong to that option** and
//!   end when the component leaves it (options inside options count
//!   separately, as in Lust: [`still_in`]). A timer ended so never
//!   finishes; a tween stops where it is, without its done block; a
//!   looping sound stops. Spawned entities stay.
//! - **`keepStarted()`** lets what the component started so far carry
//!   on: it no longer belongs to the state, as Lust's `keep` holds onto
//!   an object.
//! - **The component hears its own state change**, once a tick, if the
//!   game declares the event for its state (`event MoodChanged(Mood from,
//!   Mood to);` for a `Mood`): the first `from` and the last `to`, in the
//!   next tick's first round, after the timers' events.
//!
//! How it's done, without reading any component's state every tick:
//!
//! - Lust's runtime is told ([`own_states`]): its `setstate` (every change
//!   of an object's own state, however written) ends the component's
//!   tweens of the options it leaves and tells Solis through a hidden
//!   native ([`CHANGED`]), which keeps the change in the tick's order
//!   (`Change::State`); its `tween` notes which option of the running
//!   component each tween starts in.
//! - A Lust component with a state of its own runs with itself noted as
//!   the one running ([`RUNNING`], set around its `update` and handlers),
//!   so a timer, tween or sound it starts finds its option at the moment
//!   it starts ([`started_in`]), the value its code has just set.
//! - The game ends timers in order with their starts, and adds up each
//!   component's changes as the tick ends: the event says the first
//!   `from` and the last `to`, and nothing when they're the same.
//!
//! Lua and Rust components have no state of their own, so what they start
//! belongs to no option and ends only as before: Lust's `state` is the
//! thing this extends.

use mlua::{Function, Lua, Table, Value as LuaValue};
use reflect::EntityId;
use std::sync::Arc;

/// Whether something started during `option` is still going while the
/// state is `now`: the same option, or one inside the other, as Lust
/// says (`Combat.Melee` and `Combat` are; `Combat.Melee` and
/// `Combat.Ranged` aren't).
pub fn still_in(now: &str, option: &str) -> bool {
    let inside =
        |a: &str, b: &str| a.len() > b.len() && a.starts_with(b) && a[b.len()..].starts_with('.');
    now == option || inside(now, option) || inside(option, now)
}

/// A component's own state changed: what it started in the options it
/// left ends, and the change is heard as the tick ends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateChange {
    /// The entity the component is on.
    pub entity: EntityId,
    /// The component.
    pub component: Arc<str>,
    /// The option it was in.
    pub from: Arc<str>,
    /// The option it's in now.
    pub to: Arc<str>,
}

/// What a sound started in a state belongs to: the component that
/// started it, and the option of its own state it was in.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Started {
    /// The entity the component is on.
    pub entity: EntityId,
    /// The component.
    pub component: Arc<str>,
    /// The option of its own state.
    pub option: Arc<str>,
}

/// The hidden native Lust's runtime calls when an object's own state
/// changes (never described, so game code can't name it).
pub(crate) const CHANGED: &str = "__solisState";

/// Solis's own Lua for the component running now, made once in each
/// Luau before any game code: the table its Lust's runtime reaches as
/// `__solis` (`[1]`, the Lust component with a state of its own that runs
/// now; `owned`, each component's tweens started in its state), the
/// wrappers that note a component as running around its `update` and
/// handlers, and `keep`, which lets a component's tweens carry on past
/// its state.
pub(crate) const RUNNING: &str = r#"
local run = { owned = setmetatable({}, { __mode = "k" }) }
local function update(f)
  return function(self, dt)
    run[1] = self
    f(self, dt)
    run[1] = nil
  end
end
local function handle(f)
  return function(self, event, values)
    run[1] = self
    local taken = f(self, event, values)
    run[1] = nil
    return taken
  end
end
local function keep(o)
  local mine = o and run.owned[o]
  if mine then
    for rec in pairs(mine) do rec.during = nil end
    run.owned[o] = nil
  end
end
return run, update, handle, keep
"#;

/// The global name Lust's runtime reaches [`RUNNING`]'s table by, set in
/// the Lust's own globals.
pub(crate) const GLOBAL: &str = "__solis";

/// [`RUNNING`], made: kept in its Luau's app data, so a native finds the
/// component running now.
pub(crate) struct Running {
    /// The table (`[1]`: the running component).
    pub(crate) table: Table,
    /// Wraps a component's `update`.
    pub(crate) update: Function,
    /// Wraps a component's `handle`.
    pub(crate) handle: Function,
    /// Lets a component's tweens carry on past its state.
    keep: Function,
}

impl Running {
    /// Makes [`RUNNING`] in `lua` and keeps it there.
    pub(crate) fn make(lua: &Lua) -> mlua::Result<()> {
        let (table, update, handle, keep) = lua.load(RUNNING).set_name("=states").eval()?;
        lua.set_app_data(Self {
            table,
            update,
            handle,
            keep,
        });
        Ok(())
    }

    /// No component runs any more (its code failed part way).
    pub(crate) fn clear(lua: &Lua) {
        if let Some(running) = lua.app_data_ref::<Self>() {
            let _ = running.table.raw_set(1, LuaValue::Nil);
        }
    }
}

/// The option of the running Lust component's own state, as its code has
/// just set it: what a timer, tween or sound it starts now belongs to.
/// None when no Lust component with a state of its own runs (a Lua or
/// Rust component, a game-wide object, a tween's done block).
pub(crate) fn started_in(lua: &Lua) -> Option<Arc<str>> {
    let running = lua.app_data_ref::<Running>()?;
    let LuaValue::Table(by) = running.table.raw_get::<LuaValue>(1).ok()? else {
        return None;
    };
    match by.raw_get::<LuaValue>("state").ok()? {
        LuaValue::String(option) => Some(Arc::from(&*option.to_str().ok()?)),
        _ => None,
    }
}

/// `keepStarted()`'s part in Luau: the running component's tweens no
/// longer belong to its state.
pub(crate) fn keep_tweens(lua: &Lua) -> mlua::Result<()> {
    let Some(running) = lua.app_data_ref::<Running>() else {
        return Ok(());
    };
    let by: LuaValue = running.table.raw_get(1)?;
    running.keep.call::<()>(by)
}

/// What [`own_states`] changes in Lust's runtime, each found exactly once:
/// on one line, so no line moves and Lust still says which line of a
/// game's file failed.
const PATCHES: [(&str, &str); 5] = [
    // Every change of an object's own state: its tweens of the options
    // it leaves stop, and Solis hears it.
    (
        "  o.state = new\n  local list = owners[o]\n",
        "  local was = o.state; o.state = new; if was ~= new then L.solis_left(o, was, new) end\n  local list = owners[o]\n",
    ),
    // A tween replaced by a new one on the same place no longer belongs
    // to anyone.
    (
        "    tweens[old] = false\n",
        "    L.solis_drop(tweens[old]); tweens[old] = false\n",
    ),
    // A tween started: whose option it belongs to.
    (
        "ticks = 0, extra = 0 }\n",
        "ticks = 0, extra = 0 }; L.solis_own(tweens[i])\n",
    ),
    // A tween that reached its end.
    (
        "        tweens[i] = false\n",
        "        L.solis_drop(tw); tweens[i] = false\n",
    ),
    // The functions those call, beside the tweens' own list.
    ("local stepping, viewing = false, 0\n", RUNTIME_ADDED),
];

/// The functions [`PATCHES`] adds to Lust's runtime, on the line after its
/// tweens' list (so they reach it): `solis_own` notes a new tween's
/// option, `solis_drop` forgets an ended one, and `solis_left` stops the
/// tweens of the options a state leaves and tells Solis ([`CHANGED`]).
const RUNTIME_ADDED: &str = "local stepping, viewing = false, 0; \
function L.solis_own(rec) local S = __solis; local by = S and S[1]; local now = by and rawget(by, \"state\"); if now == nil then return end; rec.by = by; rec.during = now; local mine = S.owned[by]; if not mine then mine = {}; S.owned[by] = mine end; mine[rec] = true end; \
function L.solis_drop(rec) local S = __solis; local mine = S and rec and rec.by and S.owned[rec.by]; if mine then mine[rec] = nil end end; \
function L.solis_left(o, was, new) local S = __solis; local mine = S and S.owned[o]; if mine then for rec in pairs(mine) do if rec.during ~= nil and not L.sin(new, rec.during) then mine[rec] = nil; local here = places[rec.owner]; local at = here and here[rec.name]; if at and tweens[at] == rec then tweens[at] = false; here[rec.name] = nil; live = live - 1 end end end end; local n = LUST_NATIVES; local h = n and n.__solisState; if h then h(o, was, new) end end\n";

/// Where Lust's chunk closes its runtime: the game's own code comes after,
/// and is never changed.
const RUNTIME_END: &str = "\nend)()\n";

/// Lust's chunk with its runtime told about states ([`PATCHES`]), or why
/// it can't be: a pinned Lust whose runtime reads otherwise.
pub(crate) fn own_states(mut chunk: lust::LuaChunk) -> Result<lust::LuaChunk, String> {
    let end = chunk
        .code
        .find(RUNTIME_END)
        .map_or(0, |at| at + RUNTIME_END.len());
    let (runtime, game) = chunk.code.split_at(end);
    let mut runtime = runtime.to_string();
    for (from, to) in PATCHES {
        if runtime.matches(from).count() != 1 {
            return Err(format!(
                "Lust {}'s runtime doesn't read as Solis expects, so states can't end what was \
                 started in them. This is a bug in Solis; please report it.",
                lust::VERSION
            ));
        }
        runtime = runtime.replacen(from, to, 1);
    }
    runtime.push_str(game);
    chunk.code = runtime;
    Ok(chunk)
}

/// Whether a component's fields (`(key, kind)` from its schema) give it a
/// state of its own: Lust's `state = …;` in its `has`.
pub(crate) fn has_own_state<'a>(mut fields: impl Iterator<Item = (&'a str, &'a str)>) -> bool {
    fields.any(|(key, kind)| key == "state" && kind == "state")
}

#[cfg(test)]
#[path = "states_tests.rs"]
mod tests;
