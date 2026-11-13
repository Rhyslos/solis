//! The player's settings (decision 0091): `Screens` and `Window`.

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
    lend_reading(scope, natives, h)?;
    lend_screens(scope, natives, h)?;
    lend_window(scope, natives, h)?;
    Ok(())
}

/// Reading the settings.
fn lend_reading<'s>(
    scope: &'s Scope<'s, '_>,
    natives: &Table,
    h: &'s Lent<'_, '_>,
) -> mlua::Result<()> {
    // The player's settings (decision 0091).
    natives.raw_set(
        "Screens_textSize",
        scope.create_function(move |_, _w: LuaValue| Ok(h.borrow().settings().text_size))?,
    )?;
    natives.raw_set(
        "Screens_uiSize",
        scope.create_function(move |_, _w: LuaValue| Ok(h.borrow().settings().ui_size))?,
    )?;
    natives.raw_set(
        "Window_fullscreen",
        scope.create_function(move |_, _w: LuaValue| Ok(h.borrow().settings().fullscreen))?,
    )?;
    natives.raw_set(
        "Window_resolution",
        scope.create_function(move |_, _w: LuaValue| {
            let [w, h2] = h.borrow().settings().resolution;
            Ok(vec3_value(Vec3::new(w as f32, h2 as f32, 0.0)))
        })?,
    )?;
    Ok(())
}

/// Changing the text and UI sizes.
fn lend_screens<'s>(
    scope: &'s Scope<'s, '_>,
    natives: &Table,
    h: &'s Lent<'_, '_>,
) -> mlua::Result<()> {
    natives.raw_set(
        "Screens_setTextSize",
        native(
            scope,
            "Screens_setTextSize",
            move |_, (_w, size): (LuaValue, f64)| {
                h.borrow_mut()
                    .set_setting(crate::settings::Setting::TextSize(size));
                Ok(())
            },
        )?,
    )?;
    natives.raw_set(
        "Screens_setUiSize",
        native(
            scope,
            "Screens_setUiSize",
            move |_, (_w, size): (LuaValue, f64)| {
                h.borrow_mut()
                    .set_setting(crate::settings::Setting::UiSize(size));
                Ok(())
            },
        )?,
    )?;
    Ok(())
}

/// Changing the window, and quitting.
fn lend_window<'s>(
    scope: &'s Scope<'s, '_>,
    natives: &Table,
    h: &'s Lent<'_, '_>,
) -> mlua::Result<()> {
    natives.raw_set(
        "Window_setFullscreen",
        native(
            scope,
            "Window_setFullscreen",
            move |_, (_w, on): (LuaValue, bool)| {
                h.borrow_mut()
                    .set_setting(crate::settings::Setting::Fullscreen(on));
                Ok(())
            },
        )?,
    )?;
    natives.raw_set(
        "Window_setResolution",
        native(
            scope,
            "Window_setResolution",
            move |_, (_w, width, height): (LuaValue, i64, i64)| {
                let side = |v: i64| v.clamp(0, i64::from(u32::MAX)) as u32;
                h.borrow_mut()
                    .set_setting(crate::settings::Setting::Resolution(
                        side(width),
                        side(height),
                    ));
                Ok(())
            },
        )?,
    )?;
    natives.raw_set(
        "Window_quit",
        scope.create_function(move |_, _w: LuaValue| {
            h.borrow_mut().quit();
            Ok(())
        })?,
    )?;
    natives.raw_set(
        "Window_useNextResolution",
        scope.create_function(move |_, _w: LuaValue| {
            let [width, height] = h.borrow().settings().next_resolution();
            h.borrow_mut()
                .set_setting(crate::settings::Setting::Resolution(width, height));
            Ok(())
        })?,
    )?;
    Ok(())
}
