//! Game code ([decision 0028](../../../docs/decisions/0028-lust-runs-as-luau-while-editing.md)):
//! a project's Lust, checked and turned into Lua by the Lust compiler, and
//! run in Luau while the game plays.

pub mod code;
pub mod describe;
pub mod runtime;

pub use code::{Code, CodeProblem, check, compile, gather};
/// Game code turned into Lua, ready for a [`Runtime`].
pub use lust::LuaChunk as Chunk;
pub use runtime::{Failure, Host, NoHost, Runtime};

/// The Lust compiler's version.
pub const LUST_VERSION: &str = lust::VERSION;
