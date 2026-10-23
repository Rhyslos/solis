//! The area document
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
pub mod text;

pub use document::{Area, EntityData, order_after};
pub use edit::{Edit, EditError};
pub use history::{DEFAULT_LIMIT_BYTES, History, MergeKey, Step, Undone};
