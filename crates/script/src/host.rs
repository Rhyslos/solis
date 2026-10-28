//! What game code can ask of the engine: the [`Host`] that answers the
//! native methods described in `solis/*.lst`, and lending it to Luau as
//! the `LUST_NATIVES` table for exactly as long as game code runs.

use crate::convert::{QuatKeys, entity_of, quat_of_with, quat_table_with, vec3_of, vec3_value};
use crate::luau::{Luau, capped};
use glam::{Quat, Vec3};
use mlua::{Table, Value as LuaValue};
use reflect::EntityId;
use std::cell::{Cell, RefCell};

/// Where a sound game code plays is heard from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SoundAt {
    /// Not placed: the same everywhere (a 3D event plays at the listener).
    Nowhere,
    /// At a place in the world.
    Place(Vec3),
    /// On an entity, following it as it moves.
    On(EntityId),
}

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
    /// A random number from 0 up to (not including) 1, the same for the
    /// same seed and the same place in the tick's order. A host without
    /// randomness gives a half.
    fn random(&mut self) -> f64 {
        0.5
    }
    /// Starts the game's randomness again from `seed` (at the end of the
    /// tick, like a move).
    fn set_seed(&mut self, seed: u64) {
        let _ = seed;
    }
    /// Gives an entity's body a push (kilograms times metres a second),
    /// made at the next physics step.
    fn push(&mut self, which: EntityId, impulse: Vec3) {
        let _ = (which, impulse);
    }
    /// How fast an entity's body moves, metres a second (none without a
    /// body).
    fn velocity(&self, which: EntityId) -> Option<Vec3> {
        let _ = which;
        None
    }
    /// Sets how fast an entity's body moves, at the next physics step.
    fn set_velocity(&mut self, which: EntityId, velocity: Vec3) {
        let _ = (which, velocity);
    }
    /// Moves a character (an entity with a `CharacterBody`) by `wanted`
    /// metres, as far as it can, at the next physics step.
    fn move_character(&mut self, which: EntityId, wanted: Vec3) {
        let _ = (which, wanted);
    }
    /// Whether a character stood on something after its last move.
    fn on_ground(&self, which: EntityId) -> bool {
        let _ = which;
        false
    }
    /// Saves the game in `slot` (or, with `load`, loads the game saved
    /// there), at the end of the tick (decision 0038).
    fn save_game(&mut self, slot: &str, load: bool) {
        let _ = (slot, load);
    }
    /// An action's state this tick (decision 0037): at rest for a host
    /// without input.
    fn action(&self, name: &str) -> input::ActionState {
        let _ = name;
        input::ActionState::default()
    }
    /// The input that sets an action off (a key first, then the mouse,
    /// then the pad), for telling a player which to press.
    fn binding(&self, action: &str) -> Option<String> {
        let _ = action;
        None
    }
    /// Binds `input` to `action` from the next tick, in place of its
    /// inputs of the same kind.
    fn rebind(&mut self, action: &str, input: &str) {
        let _ = (action, input);
    }
    /// Plays the sound event named `event` (its file in the project's
    /// assets, without `.solsound`), at the end of the tick. Nothing comes
    /// back: sound never changes the game (decision 0056).
    fn play_sound(&mut self, event: &str, at: SoundAt) {
        let _ = (event, at);
    }
    /// Stops the sounds playing on an entity, with a short fade, at the
    /// end of the tick.
    fn stop_sounds_on(&mut self, which: EntityId) {
        let _ = which;
    }
    /// Turns the entity's `AudioSource` on or off, at the end of the tick.
    fn set_sound_playing(&mut self, which: EntityId, playing: bool) {
        let _ = (which, playing);
    }
    /// Sets a bus's volume for the rest of the play, as a share of its
    /// Mixer fader (1 as set, 0 silent), at the end of the tick: an
    /// options menu's sliders.
    fn set_bus_volume(&mut self, bus: &str, share: f32) {
        let _ = (bus, share);
    }
    /// The first collider a ray meets (see `physics::State::cast_ray`).
    fn cast_ray(&self, from: Vec3, direction: Vec3, reach: f32) -> Option<physics::Hit> {
        let _ = (from, direction, reach);
        None
    }

    /// The component about to run, and where what it does belongs in the
    /// tick's order ([decision 0033](../../../docs/decisions/0033-game-code-sees-the-start-of-the-tick.md)).
    /// A host that keeps changes to land later (game code on several
    /// cores) uses it; one that makes them at once needn't.
    fn at(&mut self, order: &crate::ticking::Order) {
        let _ = order;
    }
}

/// The upright rotation that turns +Y (an entity's forward) to point
/// along `direction`: turned like a compass, then tipped up or down; no
/// turn for no direction.
pub fn facing_along(direction: Vec3) -> Quat {
    let flat = direction.truncate().length();
    if !direction.is_finite() || direction.length_squared() < 1e-12 {
        return Quat::IDENTITY;
    }
    let compass = (-direction.x).atan2(direction.y);
    let tip = direction.z.atan2(flat);
    Quat::from_euler(glam::EulerRot::ZXY, compass, tip, 0.0)
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

/// A whole number from `low` to `high` (both included, either way round)
/// for a random `x` from 0 to 1; `low` when they aren't sensible numbers.
pub(crate) fn random_int(low: f64, high: f64, x: f64) -> f64 {
    if !(low.is_finite() && high.is_finite()) {
        return 0.0;
    }
    let (low, high) = (low.min(high).ceil(), low.max(high).floor());
    if high < low {
        return low;
    }
    (low + ((high - low + 1.0) * x).floor()).min(high)
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

/// Lends each renamed native under its old name too: the same function.
fn lend_old_names(natives: &Table, renamed: &[crate::describe::Renamed]) -> mlua::Result<()> {
    for r in renamed {
        let same: LuaValue = natives.raw_get(r.new)?;
        natives.raw_set(r.old, same)?;
    }
    Ok(())
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
            scope.create_function(move |_, words: mlua::LuaString| {
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
            scope.create_function(move |_, name: mlua::LuaString| {
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
            "facingAlong",
            scope.create_function(move |lua, direction: LuaValue| {
                let facing = vec3_of(&direction).map_or(Quat::IDENTITY, facing_along);
                quat_table_with(lua, facing, keys)
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
            scope.create_function(move |_, (assembly, place): (mlua::LuaString, LuaValue)| {
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
        natives.raw_set(
            "random",
            scope.create_function(move |_, ()| Ok(h.borrow_mut().random()))?,
        )?;
        natives.raw_set(
            "randomRange",
            scope.create_function(move |_, (low, high): (f64, f64)| {
                let x = h.borrow_mut().random();
                Ok(if low.is_finite() && high.is_finite() {
                    low + (high - low) * x
                } else {
                    low
                })
            })?,
        )?;
        natives.raw_set(
            "randomInt",
            scope.create_function(move |_, (low, high): (f64, f64)| {
                let x = h.borrow_mut().random();
                Ok(random_int(low, high, x))
            })?,
        )?;
        natives.raw_set(
            "push",
            scope.create_function(move |_, (which, impulse): (LuaValue, LuaValue)| {
                if let (Some(e), Some(impulse)) = (entity_of(&which), vec3_of(&impulse)) {
                    h.borrow_mut().push(e, impulse);
                }
                Ok(())
            })?,
        )?;
        natives.raw_set(
            "velocityOf",
            scope.create_function(move |_, which: LuaValue| {
                let v = entity_of(&which).and_then(|e| h.borrow().velocity(e));
                Ok(vec3_value(v.unwrap_or(Vec3::ZERO)))
            })?,
        )?;
        natives.raw_set(
            "setVelocity",
            scope.create_function(move |_, (which, velocity): (LuaValue, LuaValue)| {
                if let (Some(e), Some(velocity)) = (entity_of(&which), vec3_of(&velocity)) {
                    h.borrow_mut().set_velocity(e, velocity);
                }
                Ok(())
            })?,
        )?;
        natives.raw_set(
            "moveCharacter",
            scope.create_function(move |_, (which, wanted): (LuaValue, LuaValue)| {
                if let (Some(e), Some(wanted)) = (entity_of(&which), vec3_of(&wanted)) {
                    h.borrow_mut().move_character(e, wanted);
                }
                Ok(())
            })?,
        )?;
        natives.raw_set(
            "onGround",
            scope.create_function(move |_, which: LuaValue| {
                Ok(entity_of(&which).is_some_and(|e| h.borrow().on_ground(e)))
            })?,
        )?;
        // A ray's three answers, each its own native (Lust gives back one
        // value).
        let ray = move |(from, direction, reach): (LuaValue, LuaValue, f64)| {
            let (from, direction) = (vec3_of(&from)?, vec3_of(&direction)?);
            h.borrow().cast_ray(from, direction, reach as f32)
        };
        natives.raw_set(
            "castRay",
            scope.create_function(move |_, args: (LuaValue, LuaValue, f64)| {
                Ok(ray(args).map_or(-1.0, |hit| f64::from(hit.distance)))
            })?,
        )?;
        natives.raw_set(
            "castRayHit",
            scope.create_function(move |_, args: (LuaValue, LuaValue, f64)| {
                Ok(ray(args).map(|hit| hit.entity.to_string()))
            })?,
        )?;
        natives.raw_set(
            "castRayNormal",
            scope.create_function(move |_, args: (LuaValue, LuaValue, f64)| {
                Ok(vec3_value(ray(args).map_or(Vec3::ZERO, |hit| hit.normal)))
            })?,
        )?;
        natives.raw_set(
            "playSound",
            scope.create_function(move |_, event: mlua::LuaString| {
                h.borrow_mut()
                    .play_sound(&event.to_str()?, SoundAt::Nowhere);
                Ok(())
            })?,
        )?;
        natives.raw_set(
            "playSoundAt",
            scope.create_function(move |_, (event, place): (mlua::LuaString, LuaValue)| {
                if let Some(place) = vec3_of(&place).filter(|p| p.is_finite()) {
                    h.borrow_mut()
                        .play_sound(&event.to_str()?, SoundAt::Place(place));
                }
                Ok(())
            })?,
        )?;
        natives.raw_set(
            "playSoundOn",
            scope.create_function(move |_, (event, which): (mlua::LuaString, LuaValue)| {
                if let Some(e) = entity_of(&which) {
                    h.borrow_mut().play_sound(&event.to_str()?, SoundAt::On(e));
                }
                Ok(())
            })?,
        )?;
        natives.raw_set(
            "stopSoundsOn",
            scope.create_function(move |_, which: LuaValue| {
                if let Some(e) = entity_of(&which) {
                    h.borrow_mut().stop_sounds_on(e);
                }
                Ok(())
            })?,
        )?;
        natives.raw_set(
            "setSoundPlaying",
            scope.create_function(move |_, (which, playing): (LuaValue, bool)| {
                if let Some(e) = entity_of(&which) {
                    h.borrow_mut().set_sound_playing(e, playing);
                }
                Ok(())
            })?,
        )?;
        natives.raw_set(
            "setBusVolume",
            scope.create_function(move |_, (bus, share): (mlua::LuaString, f64)| {
                h.borrow_mut().set_bus_volume(&bus.to_str()?, share as f32);
                Ok(())
            })?,
        )?;
        natives.raw_set(
            "saveGame",
            scope.create_function(move |_, slot: mlua::LuaString| {
                h.borrow_mut().save_game(&slot.to_str()?, false);
                Ok(())
            })?,
        )?;
        natives.raw_set(
            "loadGame",
            scope.create_function(move |_, slot: mlua::LuaString| {
                h.borrow_mut().save_game(&slot.to_str()?, true);
                Ok(())
            })?,
        )?;
        natives.raw_set(
            "held",
            scope.create_function(move |_, name: mlua::LuaString| {
                Ok(h.borrow().action(&name.to_str()?).held)
            })?,
        )?;
        natives.raw_set(
            "pressed",
            scope.create_function(move |_, name: mlua::LuaString| {
                Ok(h.borrow().action(&name.to_str()?).pressed)
            })?,
        )?;
        natives.raw_set(
            "released",
            scope.create_function(move |_, name: mlua::LuaString| {
                Ok(h.borrow().action(&name.to_str()?).released)
            })?,
        )?;
        natives.raw_set(
            "axis",
            scope.create_function(move |_, name: mlua::LuaString| {
                let a = h.borrow().action(&name.to_str()?).axis;
                Ok(vec3_value(Vec3::new(a.x, a.y, 0.0)))
            })?,
        )?;
        natives.raw_set(
            "bindingOf",
            scope.create_function(move |_, name: mlua::LuaString| {
                Ok(h.borrow().binding(&name.to_str()?).unwrap_or_default())
            })?,
        )?;
        natives.raw_set(
            "rebind",
            scope.create_function(
                move |_, (action, input): (mlua::LuaString, mlua::LuaString)| {
                    h.borrow_mut().rebind(&action.to_str()?, &input.to_str()?);
                    Ok(())
                },
            )?,
        )?;
        natives.raw_set(
            "setSeed",
            scope.create_function(move |_, seed: f64| {
                if seed.is_finite() {
                    // Whole numbers as they are; others by their bits.
                    let seed = if seed.fract() == 0.0 && seed.abs() < 9.0e15 {
                        seed as i64 as u64
                    } else {
                        seed.to_bits()
                    };
                    h.borrow_mut().set_seed(seed);
                }
                Ok(())
            })?,
        )?;
        // A native renamed on purpose keeps working under its old name for
        // a while (Phase 8, C3): the same function, lent twice.
        lend_old_names(&natives, crate::describe::RENAMED)?;
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

#[cfg(test)]
mod random_tests {
    use super::random_int;

    #[test]
    fn random_whole_numbers_cover_their_range_and_never_leave_it() {
        assert_eq!(random_int(1.0, 6.0, 0.0), 1.0);
        assert_eq!(random_int(1.0, 6.0, 0.999_999_9), 6.0);
        assert_eq!(random_int(6.0, 1.0, 0.0), 1.0, "either way round");
        assert_eq!(random_int(3.0, 3.0, 0.7), 3.0);
        assert_eq!(random_int(1.5, 2.5, 0.9), 2.0, "whole numbers between");
        assert_eq!(
            random_int(1.2, 1.8, 0.5),
            2.0,
            "none between: the low end rounded up"
        );
        assert_eq!(random_int(f64::NAN, 2.0, 0.5), 0.0);
        for i in 0..=100 {
            let x = f64::from(i) / 100.0 * 0.999_999;
            let n = random_int(-3.0, 3.0, x);
            assert!((-3.0..=3.0).contains(&n) && n.fract() == 0.0);
        }
    }
}

#[cfg(test)]
mod facing_tests {
    use super::*;

    #[test]
    fn facing_along_turns_forward_to_the_direction() {
        for d in [
            Vec3::X,
            Vec3::NEG_X,
            Vec3::Y,
            Vec3::NEG_Y,
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(-2.0, 1.0, 1.0),
            Vec3::new(0.3, -0.2, -0.9),
        ] {
            let forward = facing_along(d) * Vec3::Y;
            assert!((forward - d.normalize()).length() < 1e-5, "{d}: {forward}");
            // Upright: its right side stays level.
            assert!((facing_along(d) * Vec3::X).z.abs() < 1e-5);
        }
        for nonsense in [Vec3::ZERO, Vec3::NAN, Vec3::splat(f32::INFINITY)] {
            assert_eq!(facing_along(nonsense), Quat::IDENTITY);
        }
    }
}

#[cfg(test)]
mod old_name_tests {
    use super::*;

    #[test]
    fn an_old_name_is_lent_as_the_same_function() {
        let lua = mlua::Lua::new();
        let natives = lua.create_table().unwrap();
        let f = lua.create_function(|_, ()| Ok(7)).unwrap();
        natives.raw_set("spawn", f).unwrap();
        let renamed = [crate::describe::Renamed {
            file: "world",
            old: "spawnAt",
            new: "spawn",
            until: "Phase 12",
        }];
        lend_old_names(&natives, &renamed).unwrap();
        let old: mlua::Function = natives.raw_get("spawnAt").unwrap();
        assert_eq!(old.call::<i32>(()).unwrap(), 7);
    }
}
