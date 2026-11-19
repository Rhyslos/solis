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
        scope.create_function(
            move |_, (me, component, from, to): (LuaValue, LuaValue, LuaValue, LuaValue)| {
                // Only a component's (Lust's runtime asks only when the
                // object has an entity, `me`): taken as Lust's text as it
                // is, so a change made ten thousand times a tick costs
                // little.
                let (
                    LuaValue::String(me),
                    LuaValue::String(component),
                    LuaValue::String(from),
                    LuaValue::String(to),
                ) = (me, component, from, to)
                else {
                    return Ok(());
                };
                let Some(entity) = std::str::from_utf8(&me.as_bytes())
                    .ok()
                    .and_then(|t| t.parse::<reflect::Id>().ok())
                    .map(reflect::EntityId)
                else {
                    return Ok(());
                };
                let (component, from, to) = (component.as_bytes(), from.as_bytes(), to.as_bytes());
                let (Ok(component), Ok(from), Ok(to)) = (
                    std::str::from_utf8(&component),
                    std::str::from_utf8(&from),
                    std::str::from_utf8(&to),
                ) else {
                    return Ok(());
                };
                h.borrow_mut().state_changed(entity, component, from, to);
                Ok(())
            },
        )?,
    )?;
    Ok(())
}
