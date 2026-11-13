//! The world's own objects: `World` (words, spawning, chance and the clock), `Physics` and `Saves`.

use super::Scope;
use crate::convert::{vec3_of, vec3_value};
use crate::host::{Lent, MAX_SAID_BYTES, MAX_SAID_LINES, Shot, SoundAt, random_int};
use crate::luau::capped;
use crate::native_values::native;
use glam::Vec3;
use mlua::{Table, Value as LuaValue};
use std::cell::Cell;

/// Lends this file's natives into `natives`, each answered by `h`.
pub(super) fn lend<'s>(
    scope: &'s Scope<'s, '_>,
    natives: &Table,
    h: &'s Lent<'_, '_>,
    said: &'s Cell<usize>,
) -> mlua::Result<()> {
    lend_words(scope, natives, h, said)?;
    lend_spawning(scope, natives, h)?;
    lend_chance(scope, natives, h)?;
    lend_clock(scope, natives, h)?;
    lend_physics(scope, natives, h)?;
    lend_saves(scope, natives, h)?;
    Ok(())
}

/// What game code says and finds: `World:say`, `time` and `find`.
fn lend_words<'s>(
    scope: &'s Scope<'s, '_>,
    natives: &Table,
    h: &'s Lent<'_, '_>,
    said: &'s Cell<usize>,
) -> mlua::Result<()> {
    natives.raw_set(
        "World_say",
        native(
            scope,
            "World_say",
            move |_, (_world, words): (LuaValue, mlua::LuaString)| {
                let n = said.get() + 1;
                said.set(n);
                if n <= MAX_SAID_LINES {
                    let words = words.as_bytes();
                    let text =
                        String::from_utf8_lossy(&words[..words.len().min(MAX_SAID_BYTES + 4)]);
                    h.borrow_mut().say(&capped(&text, MAX_SAID_BYTES));
                }
                Ok(())
            },
        )?,
    )?;
    natives.raw_set(
        "World_time",
        scope.create_function(move |_, _world: LuaValue| Ok(h.borrow().time()))?,
    )?;
    natives.raw_set(
        "World_find",
        native(
            scope,
            "World_find",
            move |_, (_world, name): (LuaValue, mlua::LuaString)| {
                let name = name.to_str()?;
                Ok(h.borrow().find(&name).map(|id| id.to_string()))
            },
        )?,
    )?;
    Ok(())
}

/// Making things: `World:fire` and `spawn`.
fn lend_spawning<'s>(
    scope: &'s Scope<'s, '_>,
    natives: &Table,
    h: &'s Lent<'_, '_>,
) -> mlua::Result<()> {
    natives.raw_set(
        "World_fire",
        native(
            scope,
            "World_fire",
            move |_,
                  (_world, from, direction, speed, life, colour): (
                LuaValue,
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
        "World_spawn",
        native(
            scope,
            "World_spawn",
            move |_, (_world, assembly, place): (LuaValue, mlua::LuaString, LuaValue)| {
                // Assembly.Ships.Fighter comes as "Ships.Fighter": its file
                // is ships/fighter (the Library ignores capitals).
                let assembly = crate::project_names::path_of(&assembly.to_str()?);
                Ok(assembly
                    .zip(vec3_of(&place))
                    .and_then(|(assembly, place)| h.borrow_mut().spawn(&assembly, place))
                    .map(|id| id.to_string()))
            },
        )?,
    )?;
    Ok(())
}

/// Random numbers and their seed.
fn lend_chance<'s>(
    scope: &'s Scope<'s, '_>,
    natives: &Table,
    h: &'s Lent<'_, '_>,
) -> mlua::Result<()> {
    natives.raw_set(
        "World_random",
        scope.create_function(move |_, _world: LuaValue| Ok(h.borrow_mut().random()))?,
    )?;
    natives.raw_set(
        "World_randomRange",
        native(
            scope,
            "World_randomRange",
            move |_, (_world, low, high): (LuaValue, f64, f64)| {
                let x = h.borrow_mut().random();
                Ok(if low.is_finite() && high.is_finite() {
                    low + (high - low) * x
                } else {
                    low
                })
            },
        )?,
    )?;
    natives.raw_set(
        "World_randomInt",
        native(
            scope,
            "World_randomInt",
            move |_, (_world, low, high): (LuaValue, f64, f64)| {
                let x = h.borrow_mut().random();
                Ok(random_int(low, high, x))
            },
        )?,
    )?;
    natives.raw_set(
        "World_setSeed",
        native(
            scope,
            "World_setSeed",
            move |_, (_world, seed): (LuaValue, f64)| {
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
            },
        )?,
    )?;
    Ok(())
}

/// The game's clock, speed and time of day, and the view's size.
fn lend_clock<'s>(
    scope: &'s Scope<'s, '_>,
    natives: &Table,
    h: &'s Lent<'_, '_>,
) -> mlua::Result<()> {
    natives.raw_set(
        "World_frameTime",
        scope.create_function(move |_, _world: LuaValue| Ok(h.borrow().frame_time()))?,
    )?;
    natives.raw_set(
        "World_setGameSpeed",
        native(
            scope,
            "World_setGameSpeed",
            move |_, (_world, speed): (LuaValue, f64)| {
                if speed.is_finite() {
                    h.borrow_mut().set_game_speed(speed);
                }
                Ok(())
            },
        )?,
    )?;
    natives.raw_set(
        "World_gameSpeed",
        scope.create_function(move |_, _world: LuaValue| Ok(h.borrow().game_speed()))?,
    )?;
    natives.raw_set(
        "World_timeOfDay",
        scope.create_function(move |_, _world: LuaValue| Ok(h.borrow().time_of_day()))?,
    )?;
    natives.raw_set(
        "World_setTimeOfDay",
        native(
            scope,
            "World_setTimeOfDay",
            move |_, (_world, hour): (LuaValue, f64)| {
                if hour.is_finite() {
                    h.borrow_mut().set_time_of_day(hour);
                }
                Ok(())
            },
        )?,
    )?;
    natives.raw_set(
        "World_viewSize",
        scope.create_function(move |_, _world: LuaValue| Ok(vec3_value(h.borrow().view_size())))?,
    )?;
    Ok(())
}

/// `Physics:castRay`.
fn lend_physics<'s>(
    scope: &'s Scope<'s, '_>,
    natives: &Table,
    h: &'s Lent<'_, '_>,
) -> mlua::Result<()> {
    // One question, one call: a ray's answers in one RayHit (a table
    // with its fields: found, distance, entity, normal).
    natives.raw_set(
        "Physics_castRay",
        native(
            scope,
            "Physics_castRay",
            move |lua, (_physics, from, direction, reach): (LuaValue, LuaValue, LuaValue, f64)| {
                let hit = vec3_of(&from)
                    .zip(vec3_of(&direction))
                    .and_then(|(from, direction)| {
                        h.borrow().cast_ray(from, direction, reach as f32)
                    });
                let t = lua.create_table()?;
                t.raw_set("found", hit.is_some())?;
                t.raw_set("distance", hit.map_or(-1.0, |hit| f64::from(hit.distance)))?;
                t.raw_set("entity", hit.map(|hit| hit.entity.to_string()))?;
                t.raw_set(
                    "normal",
                    vec3_value(hit.map_or(Vec3::ZERO, |hit| hit.normal)),
                )?;
                Ok(t)
            },
        )?,
    )?;
    natives.raw_set(
        "Audio_play",
        native(
            scope,
            "Audio_play",
            move |_, (_audio, event): (LuaValue, mlua::LuaString)| {
                // Sound.Sfx.Footstep comes as "Sfx.Footstep": the event
                // in sfx/footstep.solsound (found ignoring capitals).
                if let Some(event) = crate::project_names::path_of(&event.to_str()?) {
                    h.borrow_mut().play_sound(&event, SoundAt::Nowhere);
                }
                Ok(())
            },
        )?,
    )?;
    Ok(())
}

/// `Saves:saveTo` and `loadFrom`.
fn lend_saves<'s>(
    scope: &'s Scope<'s, '_>,
    natives: &Table,
    h: &'s Lent<'_, '_>,
) -> mlua::Result<()> {
    natives.raw_set(
        "Saves_saveTo",
        native(
            scope,
            "Saves_saveTo",
            move |_, (_saves, slot): (LuaValue, mlua::LuaString)| {
                h.borrow_mut().save_game(&slot.to_str()?, false);
                Ok(())
            },
        )?,
    )?;
    natives.raw_set(
        "Saves_loadFrom",
        native(
            scope,
            "Saves_loadFrom",
            move |_, (_saves, slot): (LuaValue, mlua::LuaString)| {
                h.borrow_mut().save_game(&slot.to_str()?, true);
                Ok(())
            },
        )?,
    )?;
    Ok(())
}
