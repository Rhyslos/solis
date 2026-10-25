//! How components describe themselves
//! ([decision 0019](../../../docs/decisions/0019-scene-document-now-game-storage-later.md)):
//! stable [`Id`]s, plain [`Value`]s, [`ComponentSchema`]s and a [`Registry`]
//! looked up by component name.
//!
//! Layer 0 (core). See this crate's README for its purpose and limits.

mod checksum;
mod component;
mod id;
mod registry;
mod schema;
mod value;

pub use checksum::Checksum;
pub use component::{Component, ComponentData, Fields};
pub use id::{AssetId, EntityId, ID_TEXT_LEN, Id, IdError, IdHasher, IdHashing, IdMap, IdSet};
pub use registry::{Registry, RegistryError};
pub use schema::{ComponentSchema, FieldInfo, FieldKind, Hint};
pub use value::{AssetRef, Value};
