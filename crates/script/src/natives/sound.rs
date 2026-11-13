//! Sounds: `Audio`, and the sounds on an entity.

use super::Scope;
use crate::convert::{entity_of, vec3_of};
use crate::host::{Lent, SoundAt};
use crate::native_values::native;
use mlua::{Table, Value as LuaValue};

/// Lends this file's natives into `natives`, each answered by `h`.
pub(super) fn lend<'s>(
    scope: &'s Scope<'s, '_>,
    natives: &Table,
    h: &'s Lent<'_, '_>,
) -> mlua::Result<()> {
    lend_audio(scope, natives, h)?;
    lend_on_entities(scope, natives, h)?;
    Ok(())
}

/// `Audio:playAt` and `setVolume`.
fn lend_audio<'s>(
    scope: &'s Scope<'s, '_>,
    natives: &Table,
    h: &'s Lent<'_, '_>,
) -> mlua::Result<()> {
    natives.raw_set(
        "Audio_playAt",
        native(
            scope,
            "Audio_playAt",
            move |_, (_audio, event, place): (LuaValue, mlua::LuaString, LuaValue)| {
                let event = crate::project_names::path_of(&event.to_str()?);
                if let (Some(event), Some(place)) =
                    (event, vec3_of(&place).filter(|p| p.is_finite()))
                {
                    h.borrow_mut().play_sound(&event, SoundAt::Place(place));
                }
                Ok(())
            },
        )?,
    )?;
    natives.raw_set(
        "Audio_setVolume",
        native(
            scope,
            "Audio_setVolume",
            move |_, (_audio, bus, share): (LuaValue, mlua::LuaString, f64)| {
                // Bus.None, a project with no buses, sets nothing.
                let bus = bus.to_str()?;
                if *bus != *crate::project_names::NO_OPTION {
                    h.borrow_mut().set_bus_volume(&bus, share as f32);
                }
                Ok(())
            },
        )?,
    )?;
    Ok(())
}

/// The sounds an entity plays.
fn lend_on_entities<'s>(
    scope: &'s Scope<'s, '_>,
    natives: &Table,
    h: &'s Lent<'_, '_>,
) -> mlua::Result<()> {
    natives.raw_set(
        "entity_playSound",
        native(
            scope,
            "entity_playSound",
            move |_, (which, event): (LuaValue, mlua::LuaString)| {
                let event = crate::project_names::path_of(&event.to_str()?);
                if let (Some(e), Some(event)) = (entity_of(&which), event) {
                    h.borrow_mut().play_sound(&event, SoundAt::On(e));
                }
                Ok(())
            },
        )?,
    )?;
    natives.raw_set(
        "entity_stopSounds",
        scope.create_function(move |_, which: LuaValue| {
            if let Some(e) = entity_of(&which) {
                h.borrow_mut().stop_sounds_on(e);
            }
            Ok(())
        })?,
    )?;
    natives.raw_set(
        "entity_setSoundPlaying",
        native(
            scope,
            "entity_setSoundPlaying",
            move |_, (which, playing): (LuaValue, bool)| {
                if let Some(e) = entity_of(&which) {
                    h.borrow_mut().set_sound_playing(e, playing);
                }
                Ok(())
            },
        )?,
    )?;
    Ok(())
}
