//! `Maps`: a tile map reached by its path and places.

use super::Scope;
use crate::convert::{entity_of, vec3_of, vec3_value};
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
    lend_places(scope, natives, h)?;
    lend_tiles(scope, natives, h)?;
    lend_values(scope, natives, h)?;
    lend_set_values(scope, natives, h)?;
    Ok(())
}

/// A cell's place on a map from game code's whole numbers: none off the
/// map's low edges.
fn cell(x: i64, y: i64) -> Option<[u32; 2]> {
    Some([u32::try_from(x).ok()?, u32::try_from(y).ok()?])
}

/// Cells at places, and an entity's map bounds.
fn lend_places<'s>(
    scope: &'s Scope<'s, '_>,
    natives: &Table,
    h: &'s Lent<'_, '_>,
) -> mlua::Result<()> {
    natives.raw_set(
        "Maps_cellAt",
        native(
            scope,
            "Maps_cellAt",
            move |lua, (_m, file, place): (LuaValue, mlua::LuaString, LuaValue)| {
                let found = vec3_of(&place)
                    .zip(h.borrow().map_place(&file.to_str()?))
                    .map(|(place, map)| map.cell_at(place));
                let ([x, y], inside) = found.unwrap_or(([0, 0], false));
                let t = lua.create_table()?;
                t.raw_set("inside", inside)?;
                t.raw_set("x", x as f64)?;
                t.raw_set("y", y as f64)?;
                Ok(t)
            },
        )?,
    )?;
    natives.raw_set(
        "Maps_placeOf",
        native(
            scope,
            "Maps_placeOf",
            move |_, (_m, file, x, y): (LuaValue, mlua::LuaString, i64, i64)| {
                let place = h
                    .borrow()
                    .map_place(&file.to_str()?)
                    .map_or(Vec3::ZERO, |map| map.place_of(x, y));
                Ok(vec3_value(place))
            },
        )?,
    )?;
    natives.raw_set(
        "entity_mapBounds",
        scope.create_function(move |lua, which: LuaValue| {
            let bounds = entity_of(&which).and_then(|e| h.borrow().map_bounds(e));
            let t = lua.create_table()?;
            t.raw_set("found", bounds.is_some())?;
            let (low, high) = bounds.unwrap_or((Vec3::ZERO, Vec3::ZERO));
            t.raw_set("low", vec3_value(low))?;
            t.raw_set("high", vec3_value(high))?;
            Ok(t)
        })?,
    )?;
    Ok(())
}

/// A map's size and its tiles' terrains.
fn lend_tiles<'s>(
    scope: &'s Scope<'s, '_>,
    natives: &Table,
    h: &'s Lent<'_, '_>,
) -> mlua::Result<()> {
    natives.raw_set(
        "Maps_width",
        native(
            scope,
            "Maps_width",
            move |_, (_m, file): (LuaValue, mlua::LuaString)| {
                Ok(h.borrow().map(&file.to_str()?).map_or(0, |m| m.size()[0]))
            },
        )?,
    )?;
    natives.raw_set(
        "Maps_height",
        native(
            scope,
            "Maps_height",
            move |_, (_m, file): (LuaValue, mlua::LuaString)| {
                Ok(h.borrow().map(&file.to_str()?).map_or(0, |m| m.size()[1]))
            },
        )?,
    )?;
    natives.raw_set(
        "Maps_terrain",
        native(
            scope,
            "Maps_terrain",
            move |_, (_m, file, layer, x, y): (LuaValue, mlua::LuaString, i64, i64, i64)| {
                let map = h.borrow().map(&file.to_str()?);
                Ok(map
                    .zip(cell(x, y))
                    .zip(usize::try_from(layer).ok())
                    .and_then(|((m, [x, y]), layer)| m.code(layer, x, y))
                    .map_or(0, i64::from))
            },
        )?,
    )?;
    natives.raw_set(
        "Maps_setTerrain",
        native(
            scope,
            "Maps_setTerrain",
            move |_,
                  (_m, file, layer, x, y, code): (
                LuaValue,
                mlua::LuaString,
                i64,
                i64,
                i64,
                i64,
            )| {
                if let (Some(at), Ok(layer), Ok(code)) =
                    (cell(x, y), usize::try_from(layer), u8::try_from(code))
                {
                    h.borrow_mut()
                        .set_map_tile(&file.to_str()?, layer, at, code);
                }
                Ok(())
            },
        )?,
    )?;
    Ok(())
}

/// Reading a cell's values.
fn lend_values<'s>(
    scope: &'s Scope<'s, '_>,
    natives: &Table,
    h: &'s Lent<'_, '_>,
) -> mlua::Result<()> {
    let value_at = move |file: &mlua::LuaString, value: i64, x: i64, y: i64| {
        let map = h.borrow().map(&file.to_str().ok()?)?;
        let [x, y] = cell(x, y)?;
        map.value(usize::try_from(value).ok()?, x, y)
    };
    natives.raw_set(
        "Maps_number",
        native(
            scope,
            "Maps_number",
            move |_, (_m, file, value, x, y): (LuaValue, mlua::LuaString, i64, i64, i64)| {
                Ok(match value_at(&file, value, x, y) {
                    Some(tiles::CellValue::Number(n)) => i64::from(n),
                    _ => 0,
                })
            },
        )?,
    )?;
    natives.raw_set(
        "Maps_isOn",
        native(
            scope,
            "Maps_isOn",
            move |_, (_m, file, value, x, y): (LuaValue, mlua::LuaString, i64, i64, i64)| {
                Ok(matches!(
                    value_at(&file, value, x, y),
                    Some(tiles::CellValue::Switch(true))
                ))
            },
        )?,
    )?;
    natives.raw_set(
        "Maps_choice",
        native(
            scope,
            "Maps_choice",
            move |_, (_m, file, value, x, y): (LuaValue, mlua::LuaString, i64, i64, i64)| {
                Ok(match value_at(&file, value, x, y) {
                    Some(tiles::CellValue::Choice(Some(i))) => i64::from(i),
                    _ => -1,
                })
            },
        )?,
    )?;
    Ok(())
}

/// Changing a cell's values.
fn lend_set_values<'s>(
    scope: &'s Scope<'s, '_>,
    natives: &Table,
    h: &'s Lent<'_, '_>,
) -> mlua::Result<()> {
    let set_value =
        move |file: &mlua::LuaString, value: i64, x: i64, y: i64, to: tiles::CellValue| {
            if let (Some(at), Ok(value), Ok(file)) =
                (cell(x, y), usize::try_from(value), file.to_str())
            {
                h.borrow_mut().set_map_value(&file, value, at, to);
            }
        };
    natives.raw_set(
        "Maps_setNumber",
        native(
            scope,
            "Maps_setNumber",
            move |_,
                  (_m, file, value, x, y, to): (
                LuaValue,
                mlua::LuaString,
                i64,
                i64,
                i64,
                f64,
            )| {
                if to.is_finite() {
                    let to = to.clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32;
                    set_value(&file, value, x, y, tiles::CellValue::Number(to));
                }
                Ok(())
            },
        )?,
    )?;
    natives.raw_set(
        "Maps_setSwitch",
        native(
            scope,
            "Maps_setSwitch",
            move |_,
                  (_m, file, value, x, y, on): (
                LuaValue,
                mlua::LuaString,
                i64,
                i64,
                i64,
                bool,
            )| {
                set_value(&file, value, x, y, tiles::CellValue::Switch(on));
                Ok(())
            },
        )?,
    )?;
    natives.raw_set(
        "Maps_setChoice",
        native(
            scope,
            "Maps_setChoice",
            move |_,
                  (_m, file, value, x, y, chosen): (
                LuaValue,
                mlua::LuaString,
                i64,
                i64,
                i64,
                i64,
            )| {
                let chosen = u8::try_from(chosen).ok();
                set_value(&file, value, x, y, tiles::CellValue::Choice(chosen));
                Ok(())
            },
        )?,
    )?;
    Ok(())
}
