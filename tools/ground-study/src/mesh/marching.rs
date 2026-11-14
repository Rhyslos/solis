//! Smooth by marching cubes: each cube of eight samples looks up its
//! triangles in the table (`cases`), with their points on the cube's
//! edges where the distance crosses zero. Points on an edge are shared
//! by every cube round it, so the surface is one piece.

use super::ChunkMesh;
use super::cases::{EDGES, table};
use crate::ground::Samples;
use crate::voxel::SIZE_I;
use glam::IVec3;

const AXES: [IVec3; 3] = [IVec3::X, IVec3::Y, IVec3::Z];
const SPAN: usize = SIZE_I as usize + 1;
const NONE: u32 = u32::MAX;

/// The chunk's mesh: its cubes from its own first sample to the next
/// chunk's first.
pub(crate) fn mesh(samples: &Samples) -> ChunkMesh {
    let mut out = ChunkMesh::default();
    let mut points = vec![NONE; 3 * SPAN * SPAN * SPAN];
    let corner = |c: u8| IVec3::new(i32::from(c & 1), i32::from((c >> 1) & 1), i32::from(c >> 2));
    for z in 0..SIZE_I {
        for y in 0..SIZE_I {
            for x in 0..SIZE_I {
                let cell = IVec3::new(x, y, z);
                let mut mask = 0usize;
                for c in 0..8u8 {
                    if samples.get(cell + corner(c)).is_solid() {
                        mask |= 1 << c;
                    }
                }
                let triangles = &table()[mask];
                if triangles.is_empty() {
                    continue;
                }
                for triangle in triangles {
                    for edge in triangle {
                        let (a, b) = EDGES[usize::from(*edge)];
                        let base = cell + corner(a);
                        let axis = (b - a).trailing_zeros() as usize;
                        let index = point(&mut out, &mut points, samples, base, axis);
                        out.indices.push(index);
                    }
                }
            }
        }
    }
    out
}

/// The point on the edge from `base` along `axis`, made once.
fn point(
    out: &mut ChunkMesh,
    points: &mut [u32],
    samples: &Samples,
    base: IVec3,
    axis: usize,
) -> u32 {
    let slot = axis + 3 * (base.x as usize + SPAN * (base.y as usize + SPAN * base.z as usize));
    if points[slot] != NONE {
        return points[slot];
    }
    let end = base + AXES[axis];
    let (a, b) = (samples.get(base), samples.get(end));
    let (da, db) = (a.distance(), b.distance());
    let t = (da / (da - db)).clamp(0.0, 1.0);
    let position = (samples.origin + base).as_vec3() + AXES[axis].as_vec3() * t;
    let normal = samples
        .gradient(base)
        .lerp(samples.gradient(end), t)
        .normalize_or(AXES[axis].as_vec3());
    let material = if a.is_solid() { a.material } else { b.material };
    let index = out.push_point(position, normal, material);
    points[slot] = index;
    index
}
