//! The natives game code calls, one file per engine object, each lent
//! into the `LUST_NATIVES` table by `host::with_natives` for as long as
//! game code runs. Each is registered under the key `describe::NATIVES`
//! describes it by (`World_say`); a test in `interface.rs` checks the
//! lent table against that list, both ways.

use crate::convert::QuatKeys;
use crate::host::Lent;
use mlua::Table;
use std::cell::Cell;

mod animators;
mod entity;
mod input;
mod maps;
mod settings;
mod sound;
mod world;

/// The scope the natives live in: they stop working when it ends.
type Scope<'s, 'env> = mlua::Scope<'s, 'env>;

/// Lends every native into `natives`, each answered by `h`; `keys` makes
/// rotations and `said` counts the lines game code says.
pub(crate) fn lend<'s>(
    scope: &'s Scope<'s, '_>,
    natives: &Table,
    h: &'s Lent<'_, '_>,
    keys: &'s QuatKeys,
    said: &'s Cell<usize>,
) -> mlua::Result<()> {
    world::lend(scope, natives, h, said)?;
    entity::lend(scope, natives, h, keys)?;
    sound::lend(scope, natives, h)?;
    animators::lend(scope, natives, h)?;
    maps::lend(scope, natives, h)?;
    settings::lend(scope, natives, h)?;
    input::lend(scope, natives, h)
}
