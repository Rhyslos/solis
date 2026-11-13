//! An entity's own methods: where it is and faces, how it moves, its screens and its clips.

use super::Scope;
use crate::convert::{QuatKeys, entity_of, quat_of_with, quat_table_with, vec3_of, vec3_value};
use crate::host::{Lent, facing_along};
use crate::native_values::native;
use glam::{Quat, Vec3};
use mlua::{Table, Value as LuaValue};

/// Lends this file's natives into `natives`, each answered by `h`.
pub(super) fn lend<'s>(
    scope: &'s Scope<'s, '_>,
    natives: &Table,
    h: &'s Lent<'_, '_>,
    keys: &'s QuatKeys,
) -> mlua::Result<()> {
    lend_place(scope, natives, h)?;
    lend_facing(scope, natives, h, keys)?;
    lend_motion(scope, natives, h)?;
    lend_screens(scope, natives, h)?;
    lend_clips(scope, natives, h)?;
    Ok(())
}

/// Where an entity is, its name, and despawning it.
fn lend_place<'s>(
    scope: &'s Scope<'s, '_>,
    natives: &Table,
    h: &'s Lent<'_, '_>,
) -> mlua::Result<()> {
    natives.raw_set(
        "entity_position",
        scope.create_function(move |_, which: LuaValue| {
            let at = entity_of(&which).and_then(|e| h.borrow().position(e));
            Ok(vec3_value(at.unwrap_or(Vec3::ZERO)))
        })?,
    )?;
    natives.raw_set(
        "entity_moveTo",
        native(
            scope,
            "entity_moveTo",
            move |_, (which, place): (LuaValue, LuaValue)| {
                if let (Some(e), Some(place)) = (entity_of(&which), vec3_of(&place)) {
                    h.borrow_mut().move_to(e, place);
                }
                Ok(())
            },
        )?,
    )?;
    natives.raw_set(
        "entity_name",
        scope.create_function(move |_, which: LuaValue| {
            Ok(entity_of(&which)
                .and_then(|e| h.borrow().name(e))
                .unwrap_or_default())
        })?,
    )?;
    natives.raw_set(
        "entity_despawn",
        scope.create_function(move |_, which: LuaValue| {
            if let Some(e) = entity_of(&which) {
                h.borrow_mut().despawn(e);
            }
            Ok(())
        })?,
    )?;
    Ok(())
}

/// Which way an entity faces, and `facingAlong`.
fn lend_facing<'s>(
    scope: &'s Scope<'s, '_>,
    natives: &Table,
    h: &'s Lent<'_, '_>,
    keys: &'s QuatKeys,
) -> mlua::Result<()> {
    natives.raw_set(
        "entity_rotation",
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
        "entity_face",
        native(
            scope,
            "entity_face",
            move |_, (which, direction): (LuaValue, LuaValue)| {
                if let (Some(e), Some(direction)) = (entity_of(&which), vec3_of(&direction)) {
                    h.borrow_mut().turn_to(e, facing_along(direction));
                }
                Ok(())
            },
        )?,
    )?;
    natives.raw_set(
        "entity_turnTo",
        native(
            scope,
            "entity_turnTo",
            move |_, (which, facing): (LuaValue, Table)| {
                if let (Some(e), Some(facing)) = (entity_of(&which), quat_of_with(&facing, keys)) {
                    h.borrow_mut().turn_to(e, facing);
                }
                Ok(())
            },
        )?,
    )?;
    Ok(())
}

/// Pushing and walking an entity.
fn lend_motion<'s>(
    scope: &'s Scope<'s, '_>,
    natives: &Table,
    h: &'s Lent<'_, '_>,
) -> mlua::Result<()> {
    natives.raw_set(
        "entity_push",
        native(
            scope,
            "entity_push",
            move |_, (which, impulse): (LuaValue, LuaValue)| {
                if let (Some(e), Some(impulse)) = (entity_of(&which), vec3_of(&impulse)) {
                    h.borrow_mut().push(e, impulse);
                }
                Ok(())
            },
        )?,
    )?;
    natives.raw_set(
        "entity_velocity",
        scope.create_function(move |_, which: LuaValue| {
            let v = entity_of(&which).and_then(|e| h.borrow().velocity(e));
            Ok(vec3_value(v.unwrap_or(Vec3::ZERO)))
        })?,
    )?;
    natives.raw_set(
        "entity_setVelocity",
        native(
            scope,
            "entity_setVelocity",
            move |_, (which, velocity): (LuaValue, LuaValue)| {
                if let (Some(e), Some(velocity)) = (entity_of(&which), vec3_of(&velocity)) {
                    h.borrow_mut().set_velocity(e, velocity);
                }
                Ok(())
            },
        )?,
    )?;
    natives.raw_set(
        "entity_walk",
        native(
            scope,
            "entity_walk",
            move |_, (which, wanted): (LuaValue, LuaValue)| {
                if let (Some(e), Some(wanted)) = (entity_of(&which), vec3_of(&wanted)) {
                    h.borrow_mut().walk(e, wanted);
                }
                Ok(())
            },
        )?,
    )?;
    natives.raw_set(
        "entity_grounded",
        scope.create_function(move |_, which: LuaValue| {
            Ok(entity_of(&which).is_some_and(|e| h.borrow().grounded(e)))
        })?,
    )?;
    Ok(())
}

/// The screens an entity shows.
fn lend_screens<'s>(
    scope: &'s Scope<'s, '_>,
    natives: &Table,
    h: &'s Lent<'_, '_>,
) -> mlua::Result<()> {
    // Screen.PauseMenu comes as "PauseMenu" (Screen.None, a project
    // with no screens, shows nothing).
    let screen_named = |screen: &mlua::LuaString| -> Option<String> {
        let screen = screen.to_str().ok()?;
        (*screen != *crate::project_names::NO_OPTION).then(|| screen.to_string())
    };
    natives.raw_set(
        "entity_showScreen",
        native(
            scope,
            "entity_showScreen",
            move |_, (which, screen): (LuaValue, mlua::LuaString)| {
                if let (Some(e), Some(screen)) = (entity_of(&which), screen_named(&screen)) {
                    h.borrow_mut().show_screen(e, &screen, true);
                }
                Ok(())
            },
        )?,
    )?;
    natives.raw_set(
        "entity_hideScreen",
        native(
            scope,
            "entity_hideScreen",
            move |_, (which, screen): (LuaValue, mlua::LuaString)| {
                if let (Some(e), Some(screen)) = (entity_of(&which), screen_named(&screen)) {
                    h.borrow_mut().show_screen(e, &screen, false);
                }
                Ok(())
            },
        )?,
    )?;
    natives.raw_set(
        "entity_screenShown",
        native(
            scope,
            "entity_screenShown",
            move |_, (which, screen): (LuaValue, mlua::LuaString)| {
                Ok(match (entity_of(&which), screen_named(&screen)) {
                    (Some(e), Some(screen)) => h.borrow().screen_shown(e, &screen),
                    _ => false,
                })
            },
        )?,
    )?;
    Ok(())
}

/// The clip an entity's sprite plays.
fn lend_clips<'s>(
    scope: &'s Scope<'s, '_>,
    natives: &Table,
    h: &'s Lent<'_, '_>,
) -> mlua::Result<()> {
    natives.raw_set(
        "entity_animate",
        native(
            scope,
            "entity_animate",
            move |_, (which, clip): (LuaValue, mlua::LuaString)| {
                // Clip.Sprites.Farmer.Walk comes as "Sprites.Farmer.Walk": the
                // clip is the last part (found on the Sprite's sheet ignoring
                // capitals).
                let clip = clip.to_str()?;
                let name = clip.rsplit('.').next().unwrap_or_default();
                if let Some(e) = entity_of(&which)
                    && *clip != *crate::project_names::NO_OPTION
                {
                    h.borrow_mut().play_clip(e, name, false);
                }
                Ok(())
            },
        )?,
    )?;
    natives.raw_set(
        "entity_animateFromStart",
        native(
            scope,
            "entity_animateFromStart",
            move |_, (which, clip): (LuaValue, mlua::LuaString)| {
                // Clip.Sprites.Farmer.Walk comes as "Sprites.Farmer.Walk": the
                // clip is the last part (found on the Sprite's sheet ignoring
                // capitals).
                let clip = clip.to_str()?;
                let name = clip.rsplit('.').next().unwrap_or_default();
                if let Some(e) = entity_of(&which)
                    && *clip != *crate::project_names::NO_OPTION
                {
                    h.borrow_mut().play_clip(e, name, true);
                }
                Ok(())
            },
        )?,
    )?;
    natives.raw_set(
        "entity_setFlip",
        native(
            scope,
            "entity_setFlip",
            move |_, (which, across, down): (LuaValue, bool, bool)| {
                if let Some(e) = entity_of(&which) {
                    h.borrow_mut().set_flip(e, across, down);
                }
                Ok(())
            },
        )?,
    )?;
    natives.raw_set(
        "entity_stopAnimation",
        scope.create_function(move |_, which: LuaValue| {
            if let Some(e) = entity_of(&which) {
                h.borrow_mut().stop_clip(e);
            }
            Ok(())
        })?,
    )?;
    Ok(())
}
