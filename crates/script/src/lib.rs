//! Game code ([decision 0028](../../../docs/decisions/0028-lust-runs-as-luau-while-editing.md)):
//! a project's Lust, checked and turned into Lua by the Lust compiler, and
//! run in Luau while the game plays.

pub mod code;
mod convert;
pub mod describe;
pub mod events;
pub mod host;
pub mod runtime;
pub mod rust_component;

pub use code::{Code, CodeProblem, check, compile, gather};
pub use events::Event;
pub use host::{Host, NoHost};
/// Game code turned into Lua, ready for a [`Runtime`].
pub use lust::LuaChunk as Chunk;
pub use runtime::{Failure, Runtime};
pub use rust_component::{Context, RustComponent, RustKind};

/// The Lust compiler's version.
pub const LUST_VERSION: &str = lust::VERSION;
