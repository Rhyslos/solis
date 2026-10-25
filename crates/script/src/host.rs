//! What game code can ask of the engine: the [`Host`] that answers the
//! native methods described in `solis/*.lst`, and lending it to Luau as
//! the `LUST_NATIVES` table for exactly as long as game code runs.

use crate::convert::{entity_of, quat_of, quat_table, vec3_of, vec3_table};
use glam::{Quat, Vec3};
use mlua::{Lua, Table, Value as LuaValue};
use reflect::EntityId;
use std::cell::{Cell, RefCell};
use std::time::Instant;

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

/// Runs `run` with `LUST_NATIVES` filled from `host` for its length, and
/// the time limit watching it.
pub(crate) fn call_with<R>(
    lua: &Lua,
    deadline: &Cell<Option<Instant>>,
    host: &mut dyn Host,
    send: Option<&mlua::Function>,
    run: impl FnOnce() -> mlua::Result<R>,
) -> mlua::Result<R> {
    with_natives(lua, host, send, |_| limited(deadline, run))
}

/// The host while game code runs: shared by Lust's natives and, between
/// Lua calls, Rust components.
pub(crate) type Lent<'a, 'h> = RefCell<&'a mut (dyn Host + 'h)>;

/// Runs `run` with the time limit watching it.
pub(crate) fn limited<R>(
    deadline: &Cell<Option<Instant>>,
    run: impl FnOnce() -> mlua::Result<R>,
) -> mlua::Result<R> {
    deadline.set(Some(Instant::now() + crate::runtime::TIME_LIMIT));
    let result = run();
    deadline.set(None);
    result
}

/// Runs `run` with `LUST_NATIVES` filled from `host` (and `send` as its
/// `send_event`) for its length. `run` is given the host too, for Rust
/// components to borrow between Lua calls.
pub(crate) fn with_natives<'h, R>(
    lua: &Lua,
    host: &mut (dyn Host + 'h),
    send: Option<&mlua::Function>,
    run: impl FnOnce(&Lent<'_, 'h>) -> mlua::Result<R>,
) -> mlua::Result<R> {
    let host: Lent<'_, 'h> = RefCell::new(host);
    lua.scope(|scope| {
        let natives = lua.create_table()?;
        let h = &host;
        natives.set(
            "say",
            scope.create_function(move |_, words: String| {
                h.borrow_mut().say(&words);
                Ok(())
            })?,
        )?;
        natives.set(
            "time",
            scope.create_function(move |_, ()| Ok(h.borrow().time()))?,
        )?;
        natives.set(
            "find",
            scope.create_function(move |_, name: String| {
                Ok(h.borrow().find(&name).map(|id| id.to_string()))
            })?,
        )?;
        natives.set(
            "positionOf",
            scope.create_function(move |lua, which: LuaValue| {
                let at = entity_of(&which).and_then(|e| h.borrow().position(e));
                vec3_table(lua, at.unwrap_or(Vec3::ZERO))
            })?,
        )?;
        natives.set(
            "moveTo",
            scope.create_function(move |_, (which, place): (LuaValue, Table)| {
                if let (Some(e), Some(place)) = (entity_of(&which), vec3_of(&place)) {
                    h.borrow_mut().move_to(e, place);
                }
                Ok(())
            })?,
        )?;
        natives.set(
            "rotationOf",
            scope.create_function(move |lua, which: LuaValue| {
                let facing = entity_of(&which).and_then(|e| h.borrow().rotation(e));
                quat_table(lua, facing.unwrap_or(Quat::IDENTITY))
            })?,
        )?;
        natives.set(
            "turnTo",
            scope.create_function(move |_, (which, facing): (LuaValue, Table)| {
                if let (Some(e), Some(facing)) = (entity_of(&which), quat_of(&facing)) {
                    h.borrow_mut().turn_to(e, facing);
                }
                Ok(())
            })?,
        )?;
        natives.set(
            "nameOf",
            scope.create_function(move |_, which: LuaValue| {
                Ok(entity_of(&which)
                    .and_then(|e| h.borrow().name(e))
                    .unwrap_or_default())
            })?,
        )?;
        if let Some(send) = send {
            natives.set("send_event", send.clone())?;
        }
        lua.globals().set("LUST_NATIVES", natives)?;
        let result = run(&host);
        lua.globals().set("LUST_NATIVES", LuaValue::Nil)?;
        result
    })
}
