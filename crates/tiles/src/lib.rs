//! Tile maps (Phase 9, D4): maps that store meaning, not art, and change
//! as the game plays.
//!
//! - [`tileset`]: a tileset (`.soltileset`): the terrains a cell can be
//!   (grass, tilled, water), each with the one-character code a map file
//!   writes and, until tileset art arrives (D5), a colour.
//! - [`map`]: a tile map: cells in layers (each a terrain's code) and named
//!   values per cell (number, switch or choice), changes marking their
//!   32 × 32 chunk, and a checksum worked out again only where it changed.
//! - [`map_file`]: a map saved as readable text (`.soltilemap`), in chunks,
//!   so a Git diff shows exactly the cells that changed.

pub mod map;
pub mod map_file;
pub mod tileset;

pub use map::{CHUNK, CellValue, Layer, MapError, TileMap, ValueDef, ValueKind};
pub use tileset::{Terrain, Tileset};
