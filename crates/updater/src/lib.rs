//! Updates: finds the newest build of the program on GitHub, downloads its
//! installer, checks it against its published SHA-256, and starts it
//! ([decision 0043](../../../docs/decisions/0043-installed-and-updated-from-github.md)).
//!
//! Layer 3 (app): only the program uses it; games never do. See this
//! crate's README for its purpose and limits.

mod check;
mod fetch;
mod install;
pub mod release;

pub use check::{Copy, Remote, Status, download, run, should_offer, start, start_download};
pub use fetch::{Source, clear_folder};
pub use install::{
    INSTALLED_MARKER, installer_arguments, is_installed, parse_build, start_installer, this_build,
    updates_folder,
};
pub use release::{Asset, Release};

#[cfg(test)]
mod tests;
