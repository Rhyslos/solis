//! The Luau that game code runs in, made safe to share
//! ([decision 0028](../../../docs/decisions/0028-lust-runs-as-luau-while-editing.md)):
//!
//! - **Time.** Every call into game code has a deadline ([`TIME_LIMIT`]),
//!   and a whole tick or load has one too ([`TICK_LIMIT`], [`LOAD_LIMIT`]); Luau's
//!   interrupt stops code that runs past it, so no loop freezes the
//!   editor. Anything Solis does that could run game code (reading a
//!   table whose metatable is game code's, say) happens under a deadline,
//!   or uses raw reads and writes that run no game code at all.
//! - **Memory.** At most [`MEMORY_LIMIT`] bytes.
//! - **Each file to itself.** The standard libraries and Solis's own
//!   functions (`component`, `vec3`, `solis`) are read-only. Each Lua file,
//!   and the Lust, gets globals of its own that fall through to those, so
//!   one file can't break another, or the Lust, by replacing `pairs`.
//! - **Natives.** Lust finds the native methods in its own globals
//!   (`LUST_NATIVES`), and Lua's `solis` in a place only Solis can set:
//!   game code can't take them away from the components after it.

use mlua::{Function, Lua, Table, Value as LuaValue};
use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

/// The most memory game code may use.
pub const MEMORY_LIMIT: usize = 64 << 20;
/// The longest one call into game code may run before it's stopped.
pub const TIME_LIMIT: Duration = Duration::from_millis(100);
/// The longest all the game code in one tick may run: past it, the rest
/// of the tick waits and the game pauses.
pub const TICK_LIMIT: Duration = Duration::from_millis(250);
/// The longest the code's starts may run when it loads: the Lust's own
/// start, each Lua file's, and reading their components.
pub const LOAD_LIMIT: Duration = Duration::from_secs(1);

/// The deadlines game code runs against.
#[derive(Debug, Default)]
pub(crate) struct Clock {
    /// When the call running now must end.
    call: Cell<Option<Instant>>,
    /// When everything running now (a tick, a load) must end, and how
    /// long it was given.
    all: Cell<Option<(Instant, Duration)>>,
    /// Interrupts since the clock was last read.
    since: Cell<u32>,
    /// Whether the call running now is past its deadline: then every
    /// interrupt says so, not one in [`READ_EVERY`], so code that catches
    /// the error (`pcall` in a loop) is stopped at the loop around it too.
    expired: Cell<bool>,
}

/// How often the interrupt reads the clock: once in this many. Interrupts
/// come at every call and every turn of a loop, and reading the clock
/// costs several times what the interrupt does; a few thousandths of a
/// millisecond late is still in time.
const READ_EVERY: u32 = 32;

impl Clock {
    /// Runs one call into game code with the time limit watching it: it
    /// ends by [`TIME_LIMIT`] from now, or when the whole tick or load
    /// must, whichever comes first. A call inside another keeps the outer
    /// one's deadline.
    pub(crate) fn limited<R>(&self, run: impl FnOnce() -> mlua::Result<R>) -> mlua::Result<R> {
        if self.call.get().is_some() {
            return run();
        }
        let mut until = Instant::now() + TIME_LIMIT;
        if let Some((all, _)) = self.all.get() {
            until = until.min(all);
        }
        self.call.set(Some(until));
        self.expired.set(false);
        let watching = crate::interrupt::Watching::start(self);
        let result = run();
        drop(watching);
        self.call.set(None);
        result
    }

    /// Runs `run` (a tick, a load) with all the game code in it given
    /// `limit` together.
    pub(crate) fn within<R>(&self, limit: Duration, run: impl FnOnce() -> R) -> R {
        let before = self.all.replace(Some((Instant::now() + limit, limit)));
        let result = run();
        self.all.set(before);
        result
    }

    /// Starts giving all the game code from now on `limit` together, until
    /// [`Self::stop`] (a tick in phases, which [`Self::within`] can't
    /// span).
    pub(crate) fn start(&self, limit: Duration) {
        self.all.set(Some((Instant::now() + limit, limit)));
    }

    /// Ends what [`Self::start`] began.
    pub(crate) fn stop(&self) {
        self.all.set(None);
    }

    /// Whether the tick or load running now has used up its time.
    pub(crate) fn over(&self) -> bool {
        self.all
            .get()
            .is_some_and(|(until, _)| Instant::now() >= until)
    }

    /// The interrupt's verdict: why the code must stop, once the call's
    /// deadline passes.
    pub(crate) fn verdict(&self) -> Option<String> {
        let until = self.call.get()?;
        let now = if self.expired.get() {
            Instant::now()
        } else {
            let since = self.since.get() + 1;
            if since < READ_EVERY {
                self.since.set(since);
                return None;
            }
            self.since.set(0);
            let now = Instant::now();
            if now <= until {
                return None;
            }
            self.expired.set(true);
            now
        };
        Some(match self.all.get() {
            Some((all, limit)) if now >= all => format!(
                "the game's code took longer than {} ms all together, so it was stopped",
                limit.as_millis()
            ),
            _ => format!(
                "this took longer than {} ms, so it was stopped (is there a loop that never \
                 ends?)",
                TIME_LIMIT.as_millis()
            ),
        })
    }
}

/// Lua run once in every Luau: `component`; `vec3(x, y, z)`, a Luau
/// vector (they add, take away and scale, as Lust's do); and `solis`, which reaches the native
/// methods while game code runs. It gives back two functions for Solis
/// alone: one lends `solis` the natives, the other hands over the
/// components `component` made.
const PRELUDE: &str = r#"
local made = {}
local natives = nil
function component(name, fields)
  if type(name) ~= "string" then
    error("component needs a name first, such as component(\"Spinner\", { … })", 2)
  end
  if type(fields) ~= "table" then
    error("component " .. name .. " needs its fields as a list after its name", 2)
  end
  local class = { __name = name, __fields = fields }
  class.__index = class
  made[#made + 1] = class
  return class
end
function vec3(x, y, z)
  return vector.create(x or 0, y or 0, z or 0)
end
-- Lua's own random numbers follow the game's seed, like Lust's random(),
-- so the same game gives the same numbers on any computer.
math.random = function(m, n)
  local r = natives and natives.random
  if not r then error("random numbers can only be had while the game runs", 2) end
  if m == nil then return r() end
  if n == nil then m, n = 1, m end
  return natives.randomInt(m, n)
end
math.randomseed = function(seed)
  local s = natives and natives.setSeed
  if not s then error("the seed can only be set while the game runs", 2) end
  s(seed)
end
local reach = { __index = function(_, key)
  local f = natives and key ~= "send_event" and natives[key]
  if not f then
    error("solis." .. tostring(key) .. " isn't something Solis offers to game code", 2)
  end
  return f
end, __metatable = "solis" }
table.freeze(reach)
solis = setmetatable({}, reach)
function solis.send(to, name, ...)
  local send = natives and natives.send_event
  if not send then error("events can only be sent while the game runs", 2) end
  send(to, name, table.pack(...))
end
return function(n) natives = n end, function() local m = made; made = {}; return m end
"#;

/// The Luau game code runs in, with its deadlines and Solis's own parts.
pub(crate) struct Luau {
    pub(crate) lua: Lua,
    pub(crate) clock: Rc<Clock>,
    /// The shared, read-only globals: the libraries and the prelude.
    globals: Table,
    /// The metatable of every file's globals: reads fall through to
    /// `globals`.
    through: Table,
    /// Lends `solis` the natives (or takes them back, with nil).
    lend: Function,
    /// The classes `component` made since it was last asked.
    made: Function,
    /// A `send_event` that drops what it's given, for the code's starts.
    pub(crate) discard: Function,
}

impl Luau {
    /// A new Luau, with its limits set and its globals made read-only.
    pub(crate) fn new() -> mlua::Result<Self> {
        let lua = Lua::new();
        lua.set_memory_limit(MEMORY_LIMIT)?;
        let clock = Rc::new(Clock::default());
        crate::interrupt::watch(&lua)?;
        let globals = lua.globals();
        let (lend, made): (Function, Function) = lua.load(PRELUDE).set_name("=solis").eval()?;
        // The libraries, the prelude's tables and the globals themselves
        // become read-only.
        lua.sandbox(true)?;
        let through = lua.create_table()?;
        through.raw_set("__index", &globals)?;
        through.raw_set("__metatable", "globals")?;
        through.set_readonly(true);
        let discard = lua.create_function(|_, _: mlua::MultiValue| Ok(()))?;
        Ok(Self {
            lua,
            clock,
            globals,
            through,
            lend,
            made,
            discard,
        })
    }

    /// Globals of their own for one file (or the Lust): what's set in
    /// them stays there, and reads fall through to the shared ones.
    pub(crate) fn env(&self) -> mlua::Result<Table> {
        let env = self.lua.create_table()?;
        env.set_metatable(Some(self.through.clone()));
        env.raw_set("_G", &env)?;
        // Lust looks for Luau's vectors with rawget(_G, "vector").
        let vector: LuaValue = self.globals.raw_get("vector")?;
        env.raw_set("vector", vector)?;
        // Not marked "safeenv": Luau then answers rawget(_G, …) wrongly
        // for globals set after loading, as LUST_NATIVES is.
        Ok(env)
    }

    /// Lends `solis` the natives, or takes them back (nil).
    pub(crate) fn lend(&self, natives: LuaValue) -> mlua::Result<()> {
        self.lend.call(natives)
    }

    /// The classes `component` made since this was last asked.
    pub(crate) fn made(&self) -> mlua::Result<Table> {
        self.made.call(())
    }
}

/// `text` cut to at most `most` bytes (at a character), with "…" if cut.
pub(crate) fn capped(text: &str, most: usize) -> String {
    if text.len() <= most {
        return text.to_string();
    }
    let mut end = most;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &text[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn libraries_and_solis_are_read_only_and_each_file_has_its_own_globals() {
        let luau = Luau::new().unwrap();
        let run = |text: &str, env: &Table| {
            luau.lua
                .load(text)
                .set_environment(env.clone())
                .exec()
                .map_err(|e| e.to_string())
        };
        let one = luau.env().unwrap();
        let two = luau.env().unwrap();
        for vandal in [
            "string.rep = nil",
            "table.insert = nil",
            "solis.say = print",
            "getmetatable(solis).__index = nil",
            "vector.create = nil",
            "setmetatable(_G, {})",
        ] {
            assert!(run(vandal, &one).is_err(), "{vandal} should be refused");
        }
        // A file's own globals stay its own.
        run("pairs = nil; helper = 1", &one).unwrap();
        run("assert(pairs ~= nil); assert(helper == nil)", &two).unwrap();
        assert!(luau.made().unwrap().is_empty());
    }

    #[test]
    fn a_loop_is_stopped_by_the_call_or_the_whole_budget() {
        let luau = Luau::new().unwrap();
        let spin = luau
            .lua
            .load("while true do end")
            .set_environment(luau.env().unwrap())
            .into_function()
            .unwrap();
        let start = Instant::now();
        let err = luau.clock.limited(|| spin.call::<()>(())).unwrap_err();
        assert!(err.to_string().contains("100 ms"), "{err}");
        assert!(start.elapsed() < Duration::from_secs(2));
        let err = luau
            .clock
            .within(Duration::from_millis(20), || {
                luau.clock.limited(|| spin.call::<()>(()))
            })
            .unwrap_err();
        assert!(err.to_string().contains("all together"), "{err}");
        // Outside any deadline, nothing is stopped by the old one.
        assert!(!luau.clock.over());
    }

    #[test]
    fn catching_the_time_limit_s_error_in_a_loop_doesn_t_escape_it() {
        // Each time the limit's error is caught, the loop around tries
        // again: once past its time, every interrupt says so, and the loop
        // itself is stopped.
        let luau = Luau::new().unwrap();
        let stubborn = luau
            .lua
            .load("while true do pcall(function() while true do end end) end")
            .set_environment(luau.env().unwrap())
            .into_function()
            .unwrap();
        let start = Instant::now();
        let err = luau.clock.limited(|| stubborn.call::<()>(())).unwrap_err();
        assert!(err.to_string().contains("100 ms"), "{err}");
        assert!(start.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn long_text_is_cut_at_a_character() {
        assert_eq!(capped("short", 10), "short");
        assert_eq!(capped("ééé", 3), "é…");
    }
}
