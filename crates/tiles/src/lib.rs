//! Tile maps (Phase 9, D4): maps that store meaning, not art, and change
//! as the game plays.
//!
//! - [`tileset`]: a tileset (`.soltileset`): the terrains a cell can be
//!   (grass, tilled, water), each with the one-character code a map file
//!   writes, a colour, how its art is chosen (its kind); the picture its art
//!   comes from, its pieces (variations, animation) and rules (D5).
//! - [`map`]: a tile map: cells in layers (each a terrain's code) and named
//!   values per cell (number, switch or choice), changes marking their
//!   32 × 32 chunk, and a checksum worked out again only where it changed.
//! - [`autotile`]: which piece of art each cell shows, from its
//!   neighbours (16-tile on the dual grid, 47-tile, edges, rules), variations
//!   picked by weight, the same every time; templates, grid and kind guesses.
//! - [`map_file`]: a map saved as readable text (`.soltilemap`), in chunks,
//!   so a Git diff shows exactly the cells that changed, and the exact
//!   tiles painted (D6).
//! - [`paint`]: what the tile painter's tools touch (D6): a line's and a
//!   rectangle's cells, a fill's reach, and stamps copied, flipped and
//!   turned.

pub mod autotile;
pub mod map;
pub mod map_file;
pub mod paint;
pub mod tileset;

pub use map::{CHUNK, CellValue, Layer, MapError, TileMap, ValueDef, ValueKind};
pub use tileset::{Animation, Cond, Piece, Rule, Terrain, TileKind, Tileset};
