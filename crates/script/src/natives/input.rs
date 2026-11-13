//! `Input`: actions and their bindings.

use super::Scope;
use crate::convert::vec3_value;
use crate::host::Lent;
use crate::native_values::native;
use glam::Vec3;
use mlua::{Table, Value as LuaValue};

/// Lends this file's natives into `natives`, each answered by `h`.
pub(super) fn lend<'s>(
    scope: &'s Scope<'s, '_>,
    natives: &Table,
    h: &'s Lent<'_, '_>,
) -> mlua::Result<()> {
    lend_actions(scope, natives, h)?;
    Ok(())
}

/// `Input:held`, `pressed`, `released`, `axis`, `binding` and `rebind`.
fn lend_actions<'s>(
    scope: &'s Scope<'s, '_>,
    natives: &Table,
    h: &'s Lent<'_, '_>,
) -> mlua::Result<()> {
    natives.raw_set(
        "Input_held",
        native(
            scope,
            "Input_held",
            move |_, (_input, name): (LuaValue, mlua::LuaString)| {
                Ok(h.borrow().action(&name.to_str()?).held)
            },
        )?,
    )?;
    natives.raw_set(
        "Input_pressed",
        native(
            scope,
            "Input_pressed",
            move |_, (_input, name): (LuaValue, mlua::LuaString)| {
                Ok(h.borrow().action(&name.to_str()?).pressed)
            },
        )?,
    )?;
    natives.raw_set(
        "Input_released",
        native(
            scope,
            "Input_released",
            move |_, (_input, name): (LuaValue, mlua::LuaString)| {
                Ok(h.borrow().action(&name.to_str()?).released)
            },
        )?,
    )?;
    natives.raw_set(
        "Input_axis",
        native(
            scope,
            "Input_axis",
            move |_, (_input, name): (LuaValue, mlua::LuaString)| {
                let a = h.borrow().action(&name.to_str()?).axis;
                Ok(vec3_value(Vec3::new(a.x, a.y, 0.0)))
            },
        )?,
    )?;
    natives.raw_set(
        "Input_binding",
        native(
            scope,
            "Input_binding",
            move |_, (_input, name): (LuaValue, mlua::LuaString)| {
                Ok(h.borrow().binding(&name.to_str()?).unwrap_or_default())
            },
        )?,
    )?;
    natives.raw_set(
        "Input_rebind",
        native(
            scope,
            "Input_rebind",
            move |_, (_input, action, input): (LuaValue, mlua::LuaString, mlua::LuaString)| {
                h.borrow_mut().rebind(&action.to_str()?, &input.to_str()?);
                Ok(())
            },
        )?,
    )?;
    Ok(())
}
