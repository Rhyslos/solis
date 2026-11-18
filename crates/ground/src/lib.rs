//! The ground (Tools 1b; decisions 0111, 0116 and 0117): one per level,
//! with caves and overhangs, kept as voxels in chunks and drawn in a look
//! chosen per ground. G1 gives the Smooth look; sculpting, the other
//! looks and painting come in G2 to G4.
//!
//! - [`voxel`]: one voxel (its distance from the surface in 1/64ths of a
//!   voxel, and its ground kind: two bytes), and where it sits in a chunk.
//! - [`store`]: chunks of 32 × 32 × 32 voxels behind the [`store::Store`]
//!   interface, a chunk the same all through kept as one voxel, the rest
//!   shared through `Arc`; a store can say a chunk isn't loaded.
//! - [`ground`]: a ground of chunks, a voxel at any place, and the samples
//!   a chunk is meshed from (its voxels and one more all round).
//! - [`smooth`]: the Smooth look's mesher (surface nets), chunk by chunk,
//!   seams shared exactly, facings from the voxels.
//! - [`query`]: the height at a place, the kind at a place, and a ray
//!   against the ground, each "unknown" where a chunk isn't loaded.
//! - [`start`]: a new ground's starting shape, flat or gently rolling.
//! - [`noise`]: smooth random numbers, the same on every PC.
//! - [`settings`]: the settings file (`farm.ground.lst`, Lust).
//! - [`region`]: the binary region files (one per 256 × 256 m) beside it.
//!
//! It knows nothing of the graphics card, the editor or the physics
//! library (rule 2): meshes come out as plain lists, and files as bytes
//! and text for the app to read and write.

pub mod ground;
pub mod noise;
pub mod query;
pub mod region;
pub mod settings;
pub mod smooth;
pub mod start;
pub mod store;
pub mod voxel;

pub use ground::{Answer, Ground, Samples};
pub use query::Hit;
pub use settings::{GroundKind, Look, Settings};
pub use smooth::{ChunkMesh, Mesher};
pub use start::Start;
pub use store::{Chunk, Fixed, Slot, Store};
pub use voxel::Voxel;
