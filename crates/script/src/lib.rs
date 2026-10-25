//! Game code ([decision 0028](../../../docs/decisions/0028-lust-runs-as-luau-while-editing.md)):
//! a project's Lust, checked and turned into Lua by the Lust compiler, and
//! run in Luau while the game plays.

pub mod code;
mod convert;
pub mod describe;
pub mod events;
pub mod host;
mod lua_components;
pub mod luau;
mod lust_program;
pub mod runtime;
pub mod rust_component;
mod syncing;
mod ticking;

pub use code::{Bytecode, Code, CodeProblem, Compiled, check, compile, gather};
pub use events::Event;
pub use host::{Host, NoHost};
/// Game code turned into Lua, ready for a [`Runtime`].
pub use lust::LuaChunk as Chunk;
pub use runtime::{Failure, Runtime, Swapped, Ticked};
pub use rust_component::{Context, RustComponent, RustKind};

/// The Lust compiler's version.
pub const LUST_VERSION: &str = lust::VERSION;
