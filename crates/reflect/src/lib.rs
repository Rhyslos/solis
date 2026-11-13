//! How components describe themselves
//! ([decision 0019](../../../docs/decisions/0019-scene-document-now-game-storage-later.md)):
//! stable [`Id`]s, plain [`Value`]s, [`ComponentSchema`]s and a [`Registry`]
//! looked up by component name.
//!
//! Also the one way a file is opened ([`ordinary_file`]), shared by every
//! crate that reads one.
//!
//! Layer 0 (core). See this crate's README for its purpose and limits.

mod checksum;
mod component;
pub mod data_files;
mod id;
pub mod ordinary_file;
mod registry;
mod schema;
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;
pub mod upgrade;
mod value;

pub use checksum::Checksum;
pub use component::{Component, ComponentData, Fields};
pub use id::{AssetId, EntityId, ID_TEXT_LEN, Id, IdError, IdHasher, IdHashing, IdMap, IdSet};
pub use registry::{Registry, RegistryError};
pub use schema::{ComponentSchema, FieldInfo, FieldKind, Hint, Offer};
pub use value::{AssetRef, Value};
