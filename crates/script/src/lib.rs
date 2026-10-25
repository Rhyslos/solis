//! Game code ([decision 0028](../../../docs/decisions/0028-lust-runs-as-luau-while-editing.md)):
//! a project's Lust, checked and turned into Lua by the Lust compiler, and
//! run in Luau while the game plays.

pub mod code;
mod convert;
pub mod describe;
pub mod events;
pub mod game;
pub mod host;
#[allow(unsafe_code)]
mod interrupt;
mod lua_components;
pub mod luau;
mod lust_program;
pub mod runtime;
pub mod rust_component;
mod syncing;
mod ticking;

pub use code::{Bytecode, Code, CodeProblem, Compiled, check, compile, gather};
pub use events::Event;
pub use game::{Change, Game, GameWorld, ViewIndex, WorldView};
pub use host::{Host, NoHost, Shot};
/// Game code turned into Lua, ready for a [`Runtime`].
pub use lust::LuaChunk as Chunk;
pub use runtime::{Failure, Runtime, Swapped, Ticked};
pub use rust_component::{Context, RustComponent, RustKind};
pub use ticking::Order;

/// The Lust compiler's version.
pub const LUST_VERSION: &str = lust::VERSION;

/// Whether `slot` is a usable save slot's name (decision 0038): letters,
/// digits, `_` and `-`, at most 64 long, and not a name Windows keeps
/// for a device (`CON`, `NUL`, `COM1` …). It becomes a file's name, so
/// nothing that could reach outside the saves folder.
pub fn valid_slot(slot: &str) -> bool {
    const DEVICES: [&str; 4] = ["CON", "PRN", "AUX", "NUL"];
    let upper = slot.to_ascii_uppercase();
    let device = DEVICES.contains(&upper.as_str())
        || ["COM", "LPT"].iter().any(|d| {
            upper
                .strip_prefix(d)
                .is_some_and(|n| n.len() == 1 && n.chars().all(|c| c.is_ascii_digit()))
        });
    !slot.is_empty()
        && slot.len() <= 64
        && !device
        && slot
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}
