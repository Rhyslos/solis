//! Blocky at any level: a voxel of level `L` is a cube `2^L` metres
//! wide, standing on the level-0 cubes it covers, so cubes of every
//! level nest. Merged cube faces, as the ground study chose.
//!
//! Two ways of meeting a neighbour of another level:
//!
//! - **Walls between sizes** ([`Seams::Transition`], and
//!   [`Seams::Stitched`], which comes to the same for cubes): each side
//!   draws its faces against the other side's cubes at the other's size.
//!   The finer side sees a coarse cube where the coarse level has one;
//!   the coarser side sees its neighbour's place as solid only if every
//!   finer cube in it is. Every face between ground and air is then
//!   drawn by one side or the other, so nothing is open.
//! - **Skirts** ([`Seams::Skirts`]): each chunk draws walls down its
//!   sides two cubes deep under its surface, whatever is beyond.

use super::layout::Layout;
use super::nets::Seams;
use super::{Levels, voxel};
use crate::ground::Samples;
use crate::mesh::{ChunkMesh, blocky_with};
use crate::voxel::{Dense, SIZE_I, Voxel};
use glam::{IVec3, Vec2};

/// How deep the skirts' walls reach under the surface, in cubes.
const SKIRT_CUBES: i32 = 2;

/// The chunk `key` of `level` as merged cube faces in metres, meeting
/// neighbours of other levels (as `layout` places them) by `seams`.
pub fn mesh(
    levels: &impl Levels,
    layout: &Layout,
    level: u32,
    key: IVec3,
    seams: Seams,
) -> ChunkMesh {
    let mut samples = Samples::default();
    let mut scratch = Dense::filled(Voxel::AIR);
    levels.gather(level, key, &mut scratch, &mut samples);
    if matches!(seams, Seams::Transition | Seams::Stitched) {
        walls_between_sizes(levels, layout, level, &mut samples);
    }
    let skirts = seams == Seams::Skirts;
    let mut mesh = blocky_with(&samples, true, |p, step| {
        skirts && on_side(p, step) && air_above(&samples, p)
    });
    let width = voxel(level);
    for p in &mut mesh.positions {
        *p = (*p + 0.5) * width - 0.5;
    }
    mesh
}

/// Whether the face of `p` facing `step` is on the chunk's outside.
fn on_side(p: IVec3, step: IVec3) -> bool {
    (step.x == 1 && p.x == SIZE_I - 1)
        || (step.x == -1 && p.x == 0)
        || (step.y == 1 && p.y == SIZE_I - 1)
        || (step.y == -1 && p.y == 0)
}

fn air_above(samples: &Samples, p: IVec3) -> bool {
    (1..=SKIRT_CUBES).any(|d| !samples.get(p + IVec3::Z * d).is_solid())
}

/// Sets the samples just past the chunk's sides to what the level drawn
/// there shows (see the module's documentation).
fn walls_between_sizes(levels: &impl Levels, layout: &Layout, level: u32, samples: &mut Samples) {
    let origin = samples.origin;
    for z in 0..SIZE_I {
        for side in 0..SIZE_I {
            for p in [
                IVec3::new(-1, side, z),
                IVec3::new(SIZE_I, side, z),
                IVec3::new(side, -1, z),
                IVec3::new(side, SIZE_I, z),
            ] {
                let g = origin + p;
                let first = (g.truncate() << level).as_vec2() + Vec2::splat(0.5);
                let there = layout.level_at(first);
                let solid = if there > level {
                    let k = there - level;
                    levels
                        .voxel(there, g.div_euclid(IVec3::splat(1 << k)))
                        .is_solid()
                } else if there < level {
                    let k = level - there;
                    let n = 1 << k;
                    crate::ground::boxed(g * n, g * n + n)
                        .all(|q| levels.voxel(there, q).is_solid())
                } else {
                    continue;
                };
                let was = samples.get(p);
                if was.is_solid() != solid {
                    let material = if solid { was.material } else { 0 };
                    samples.set(p, Voxel::at(if solid { -1.0 } else { 1.0 }, material));
                }
            }
        }
    }
}
