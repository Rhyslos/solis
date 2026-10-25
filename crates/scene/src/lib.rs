//! The scene document
//! ([decision 0019](../../../docs/decisions/0019-scene-document-now-game-storage-later.md)):
//! entities with stable IDs, and their components as values.
//!
//! Layer 0 (core). See this crate's README for its purpose and limits.

pub mod builtin;
mod document;

pub use document::{EntityData, Scene};
