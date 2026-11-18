//! A component's own state (stage 1, states; decision 0126):
//! `keepStarted()`, and the hidden native Lust's runtime tells Solis
//! through when a state changes.

use super::Scope;
use crate::convert::entity_of;
use crate::host::Lent;
use mlua::{Table, Value as LuaValue};

/// Lends `keepStarted` and the runtime's own [`crate::states::CHANGED`]
/// into `natives`, each answered by `h`.
pub(super) fn lend<'s>(
    scope: &'s Scope<'s, '_>,
    natives: &Table,
    h: &'s Lent<'_, '_>,
) -> mlua::Result<()> {
    natives.raw_set(
        "entity_keepStarted",
        scope.create_function(move |lua, which: LuaValue| {
            let Some(entity) = entity_of(&which) else {
                return Err(mlua::Error::runtime(
                    "solis.entity.keepStarted(e) was called on nothing: call it inside a \
                     component, on the component's own entity (self.me).",
                ));
            };
            h.borrow_mut()
                .keep_started(entity)
                .map_err(|why| mlua::Error::runtime(format!("keepStarted: {why}.")))?;
            crate::states::keep_tweens(lua)
        })?,
    )?;
    natives.raw_set(
        crate::states::CHANGED,
        scope.create_function(move |_, (thing, from, to): (Table, LuaValue, LuaValue)| {
            // Only a component has an entity of its own (`me`): a plain
            // object's or a game-wide one's state belongs to Lust alone.
            let Some(entity) = entity_of(&thing.raw_get::<LuaValue>("me")?) else {
                return Ok(());
            };
            let Some(class) = thing.metatable() else {
                return Ok(());
            };
            let (LuaValue::String(component), LuaValue::String(from), LuaValue::String(to)) =
                (class.raw_get::<LuaValue>("__name")?, from, to)
            else {
                return Ok(());
            };
            h.borrow_mut().state_changed(
                entity,
                &component.to_str()?,
                &from.to_str()?,
                &to.to_str()?,
            );
            Ok(())
        })?,
    )?;
    Ok(())
}
