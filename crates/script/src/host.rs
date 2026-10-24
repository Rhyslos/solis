//! What game code can ask of the engine: the [`Host`] that answers the
//! native methods described in `solis/*.lst`, and lending it to Luau as
//! the `LUST_NATIVES` table for exactly as long as game code runs.

use crate::convert::{QuatKeys, entity_of, quat_of_with, quat_table_with, vec3_of, vec3_value};
use crate::luau::{Luau, capped};
use glam::{Quat, Vec3};
use mlua::{Table, Value as LuaValue};
use reflect::EntityId;
use std::cell::{Cell, RefCell};

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
    /// Fires a projectile. A host without projectiles ignores it.
    fn fire(&mut self, shot: Shot) {
        let _ = shot;
    }
    /// Makes a copy of the assembly named `assembly` (its file in the
    /// project's assets) at `place`, and gives back its entity, which is
    /// in the game from the next tick; `None` when it can't be made.
    fn spawn(&mut self, assembly: &str, place: Vec3) -> Option<EntityId> {
        let _ = (assembly, place);
        None
    }
    /// Takes an entity (and everything under it) out of the game, at the
    /// end of the tick.
    fn despawn(&mut self, which: EntityId) {
        let _ = which;
    }
    /// How long the last frame took, in seconds of real time.
    fn frame_time(&self) -> f64 {
        1.0 / 60.0
    }

    /// The component about to run, and where what it does belongs in the
    /// tick's order ([decision 0033](../../../docs/decisions/0033-game-code-sees-the-start-of-the-tick.md)).
    /// A host that keeps changes to land later (game code on several
    /// cores) uses it; one that makes them at once needn't.
    fn at(&mut self, order: &crate::ticking::Order) {
        let _ = order;
    }
}

/// A projectile game code fires (`fire`): where from, which way, how fast
/// (metres a second), how long it lives (seconds), and its colour (linear
/// RGB; above 1 glows).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shot {
    /// Where it starts.
    pub from: Vec3,
    /// Which way it flies.
    pub direction: Vec3,
    /// How fast.
    pub speed: f32,
    /// How long it lives.
    pub life: f32,
    /// Its colour.
    pub colour: [f32; 3],
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

/// The most bytes of one line game code says; the rest is cut.
pub const MAX_SAID_BYTES: usize = 2_000;
/// The most lines game code says while the natives are lent once (a tick,
/// a load); the rest are counted, not shown.
pub const MAX_SAID_LINES: usize = 200;

/// Runs `run` with the natives lent from `host` and the time limit
/// watching it.
pub(crate) fn call_with<R>(
    luau: &Luau,
    lust: Option<&Table>,
    host: &mut dyn Host,
    send: Option<&mlua::Function>,
    run: impl FnOnce() -> mlua::Result<R>,
) -> mlua::Result<R> {
    with_natives(luau, lust, host, send, |_| luau.clock.limited(run))
}

/// The host while game code runs: shared by Lust's natives and, between
/// Lua calls, Rust components.
pub(crate) type Lent<'a, 'h> = RefCell<&'a mut (dyn Host + 'h)>;

/// Runs `run` with the natives filled from `host` (and `send` as their
/// `send_event`) for its length: as `LUST_NATIVES` in the Lust's globals
/// `lust`, and for Lua's `solis`. `run` is given the host too, for Rust
/// components to borrow between Lua calls. Past [`MAX_SAID_LINES`], what
/// game code says is counted, and the count said at the end.
pub(crate) fn with_natives<'h, R>(
    luau: &Luau,
    lust: Option<&Table>,
    host: &mut (dyn Host + 'h),
    send: Option<&mlua::Function>,
    run: impl FnOnce(&Lent<'_, 'h>) -> mlua::Result<R>,
) -> mlua::Result<R> {
    let lua = &luau.lua;
    let host: Lent<'_, 'h> = RefCell::new(host);
    let lines = Cell::new(0usize);
    // Made once for all the rotations crossing while the natives are lent.
    let keys = &QuatKeys::new(lua)?;
    let result = lua.scope(|scope| {
        let natives = lua.create_table()?;
        let h = &host;
        let said = &lines;
        natives.raw_set(
            "say",
            scope.create_function(move |_, words: mlua::String| {
                let n = said.get() + 1;
                said.set(n);
                if n <= MAX_SAID_LINES {
                    let words = words.as_bytes();
                    let text =
                        String::from_utf8_lossy(&words[..words.len().min(MAX_SAID_BYTES + 4)]);
                    h.borrow_mut().say(&capped(&text, MAX_SAID_BYTES));
                }
                Ok(())
            })?,
        )?;
        natives.raw_set(
            "time",
            scope.create_function(move |_, ()| Ok(h.borrow().time()))?,
        )?;
        natives.raw_set(
            "find",
            scope.create_function(move |_, name: mlua::String| {
                let name = name.to_str()?;
                Ok(h.borrow().find(&name).map(|id| id.to_string()))
            })?,
        )?;
        natives.raw_set(
            "positionOf",
            scope.create_function(move |_, which: LuaValue| {
                let at = entity_of(&which).and_then(|e| h.borrow().position(e));
                Ok(vec3_value(at.unwrap_or(Vec3::ZERO)))
            })?,
        )?;
        natives.raw_set(
            "moveTo",
            scope.create_function(move |_, (which, place): (LuaValue, LuaValue)| {
                if let (Some(e), Some(place)) = (entity_of(&which), vec3_of(&place)) {
                    h.borrow_mut().move_to(e, place);
                }
                Ok(())
            })?,
        )?;
        natives.raw_set(
            "rotationOf",
            scope.create_function(move |lua, which: LuaValue| {
                let facing = entity_of(&which).and_then(|e| h.borrow().rotation(e));
                quat_table_with(lua, facing.unwrap_or(Quat::IDENTITY), keys)
            })?,
        )?;
        natives.raw_set(
            "turnTo",
            scope.create_function(move |_, (which, facing): (LuaValue, Table)| {
                if let (Some(e), Some(facing)) = (entity_of(&which), quat_of_with(&facing, keys)) {
                    h.borrow_mut().turn_to(e, facing);
                }
                Ok(())
            })?,
        )?;
        natives.raw_set(
            "nameOf",
            scope.create_function(move |_, which: LuaValue| {
                Ok(entity_of(&which)
                    .and_then(|e| h.borrow().name(e))
                    .unwrap_or_default())
            })?,
        )?;
        natives.raw_set(
            "fire",
            scope.create_function(
                move |_,
                      (from, direction, speed, life, colour): (
                    LuaValue,
                    LuaValue,
                    f64,
                    f64,
                    LuaValue,
                )| {
                    if let (Some(from), Some(direction), Some(colour)) =
                        (vec3_of(&from), vec3_of(&direction), vec3_of(&colour))
                    {
                        h.borrow_mut().fire(Shot {
                            from,
                            direction,
                            speed: speed as f32,
                            life: life as f32,
                            colour: colour.to_array(),
                        });
                    }
                    Ok(())
                },
            )?,
        )?;
        natives.raw_set(
            "spawn",
            scope.create_function(move |_, (assembly, place): (mlua::String, LuaValue)| {
                let assembly = assembly.to_str()?;
                Ok(vec3_of(&place)
                    .and_then(|place| h.borrow_mut().spawn(&assembly, place))
                    .map(|id| id.to_string()))
            })?,
        )?;
        natives.raw_set(
            "despawn",
            scope.create_function(move |_, which: LuaValue| {
                if let Some(e) = entity_of(&which) {
                    h.borrow_mut().despawn(e);
                }
                Ok(())
            })?,
        )?;
        natives.raw_set(
            "frameTime",
            scope.create_function(move |_, ()| Ok(h.borrow().frame_time()))?,
        )?;
        if let Some(send) = send {
            natives.raw_set("send_event", send.clone())?;
        }
        if let Some(lust) = lust {
            lust.raw_set("LUST_NATIVES", &natives)?;
        }
        luau.lend(LuaValue::Table(natives))?;
        let result = run(&host);
        if let Some(lust) = lust {
            lust.raw_set("LUST_NATIVES", LuaValue::Nil)?;
        }
        luau.lend(LuaValue::Nil)?;
        result
    });
    let more = lines.get().saturating_sub(MAX_SAID_LINES);
    if more > 0 {
        host.borrow_mut().say(&format!(
            "(Game code said {more} more lines just now; only the first {MAX_SAID_LINES} are shown.)"
        ));
    }
    result
}
