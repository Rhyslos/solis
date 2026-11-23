//! The level document
//! ([decision 0019](../../../docs/decisions/0019-scene-document-now-game-storage-later.md)):
//! entities with stable IDs, and their components as values, changed only
//! through reversible edits, which undo and redo are made of
//! ([decision 0022](../../../docs/decisions/0022-undo-as-reversible-edits.md)).
//!
//! Layer 0 (core). See this crate's README for its purpose and limits.

pub mod builtin;
mod document;
mod edit;
mod history;
pub mod languages;
mod lust_check;
pub mod lust_data;
pub mod lust_fields;
pub mod lust_guard;
pub mod module;
pub mod module_file;
pub mod module_follow;
pub mod pixels;
pub mod settings;
pub mod table;
pub mod tags;
pub mod text;
mod tree;

pub use document::{EntityData, Level, Origin, order_after};
pub use edit::{Edit, EditError};
pub use history::{DEFAULT_LIMIT_BYTES, Followed, History, MergeKey, Step, Undone, Weigh};
pub use tree::{Ancestors, Mended, Tree};

#[cfg(test)]
mod gap_tests;
