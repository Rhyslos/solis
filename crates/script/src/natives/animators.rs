//! `Animators`: an entity's animator, reached by its graph's names.

use super::Scope;
use crate::convert::{entity_of, vec3_of};
use crate::host::{AnimatorValue, Lent};
use crate::native_values::native;
use mlua::{Table, Value as LuaValue};

/// Lends this file's natives into `natives`, each answered by `h`.
pub(super) fn lend<'s>(
    scope: &'s Scope<'s, '_>,
    natives: &Table,
    h: &'s Lent<'_, '_>,
) -> mlua::Result<()> {
    lend_values(scope, natives, h)?;
    lend_states(scope, natives, h)?;
    Ok(())
}

/// Setting an animator's values.
fn lend_values<'s>(
    scope: &'s Scope<'s, '_>,
    natives: &Table,
    h: &'s Lent<'_, '_>,
) -> mlua::Result<()> {
    // An animator's own Lust file reaches it by its graph's names.
    let animator = move |which: &LuaValue, name: &mlua::LuaString, value: AnimatorValue| {
        if let (Some(e), Ok(name)) = (entity_of(which), name.to_str())
            && name.len() <= 256
        {
            h.borrow_mut().animator_set(e, &name, value);
        }
    };
    natives.raw_set(
        "Animators_setNumber",
        native(
            scope,
            "Animators_setNumber",
            move |_, (_a, which, name, value): (LuaValue, LuaValue, mlua::LuaString, f64)| {
                if value.is_finite() {
                    animator(&which, &name, AnimatorValue::Number(value as f32));
                }
                Ok(())
            },
        )?,
    )?;
    natives.raw_set(
        "Animators_setSwitch",
        native(
            scope,
            "Animators_setSwitch",
            move |_, (_a, which, name, on): (LuaValue, LuaValue, mlua::LuaString, bool)| {
                animator(&which, &name, AnimatorValue::Switch(on));
                Ok(())
            },
        )?,
    )?;
    natives.raw_set(
        "Animators_fire",
        native(
            scope,
            "Animators_fire",
            move |_, (_a, which, name): (LuaValue, LuaValue, mlua::LuaString)| {
                animator(&which, &name, AnimatorValue::Fire);
                Ok(())
            },
        )?,
    )?;
    natives.raw_set(
        "Animators_setDirection",
        native(
            scope,
            "Animators_setDirection",
            move |_, (_a, which, name, way): (LuaValue, LuaValue, mlua::LuaString, LuaValue)| {
                if let Some(way) = vec3_of(&way).filter(|w| w.is_finite()) {
                    animator(&which, &name, AnimatorValue::Direction(way));
                }
                Ok(())
            },
        )?,
    )?;
    Ok(())
}

/// An animator's state and speed.
fn lend_states<'s>(
    scope: &'s Scope<'s, '_>,
    natives: &Table,
    h: &'s Lent<'_, '_>,
) -> mlua::Result<()> {
    natives.raw_set(
        "Animators_stateOf",
        native(
            scope,
            "Animators_stateOf",
            move |_, (_a, which): (LuaValue, LuaValue)| {
                Ok(entity_of(&which)
                    .and_then(|e| h.borrow().animator_state(e))
                    .unwrap_or_default())
            },
        )?,
    )?;
    natives.raw_set(
        "Animators_jumpTo",
        native(
            scope,
            "Animators_jumpTo",
            move |_, (_a, which, state): (LuaValue, LuaValue, mlua::LuaString)| {
                if let (Some(e), Ok(state)) = (entity_of(&which), state.to_str())
                    && state.len() <= 256
                {
                    h.borrow_mut().animator_jump(e, &state);
                }
                Ok(())
            },
        )?,
    )?;
    natives.raw_set(
        "Animators_setPlaybackSpeed",
        native(
            scope,
            "Animators_setPlaybackSpeed",
            move |_, (_a, which, scale): (LuaValue, LuaValue, f64)| {
                if let Some(e) = entity_of(&which)
                    && scale.is_finite()
                {
                    h.borrow_mut().animator_speed(e, scale as f32);
                }
                Ok(())
            },
        )?,
    )?;
    Ok(())
}
