//! Tools 1b, G0: the ground study (CONTRIBUTING.md rule 22). Small
//! working versions of the ways a ground with caves and overhangs could
//! be kept and drawn, so they can be measured against each other. The
//! findings are in `docs/reports/ground-study.md`; the program beside
//! this library (`main.rs`) makes its numbers and pictures.
//!
//! - [`voxel`]: one voxel (distance from the surface and material), and
//!   a chunk of them laid out in full.
//! - [`store`]: the three ways of keeping a chunk compared: fixed
//!   chunks, sparse chunks, an octree.
//! - [`ground`]: a box of chunks, gathering a chunk's samples, and the
//!   brush applied across chunks.
//! - [`brush`]: one dab of the round sculpting brush.
//! - [`mesh`]: the looks' competing ways of making triangles.
//! - [`collision`]: chunks made solid, and a character walking on them.
//! - [`file`](mod@file): the ground's file as text and as bytes.
//! - [`shapes`] and [`noise`]: the test grounds.
//!
//! The open-worlds study (after Tools 1b; `docs/reports/open-worlds-study.md`)
//! adds:
//!
//! - [`world`]: the 20 × 20 km test world, made at any level of detail.
//! - [`lod`]: far ground drawn coarser: levels made by averaging, which
//!   size is drawn where, the looks' meshers at any level and the ways of
//!   closing the seams between levels.
//! - [`far`]: the far field, a height map made from the voxels.
//! - [`stream`]: region files, and streaming chunks in and out.

pub mod brush;
pub mod collision;
pub mod far;
pub mod file;
pub mod ground;
pub mod lod;
pub mod mesh;
pub mod noise;
pub mod shapes;
pub mod store;
pub mod stream;
pub mod voxel;
pub mod world;
