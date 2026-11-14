//! Blocky: each voxel in the ground is a one-voxel cube, and a square is
//! drawn wherever a cube faces the air. Merged, the squares of one slice
//! that face the same way and share a material grow into the largest
//! rectangles that fit ("greedy" meshing), which draws the same picture
//! with far fewer triangles.

use super::ChunkMesh;
use crate::ground::Samples;
use crate::voxel::{SIZE, SIZE_I};
use glam::{IVec3, Vec3};

const AXES: [IVec3; 3] = [IVec3::X, IVec3::Y, IVec3::Z];

/// The chunk's cube faces, merged into rectangles when `merge` is set.
pub(crate) fn mesh(samples: &Samples, merge: bool) -> ChunkMesh {
    mesh_with(samples, merge, |_, _| false)
}

/// The chunk's cube faces, and also the faces `extra` names (a solid
/// voxel at `p` and the way it faces) even where the voxel beyond is
/// solid too: the levels of detail's walls at a chunk's sides.
pub(crate) fn mesh_with(
    samples: &Samples,
    merge: bool,
    extra: impl Fn(IVec3, IVec3) -> bool,
) -> ChunkMesh {
    let mut out = ChunkMesh::default();
    let mut mask = vec![0u16; SIZE * SIZE];
    for (a, across) in AXES.iter().enumerate() {
        let (b, c) = ((a + 1) % 3, (a + 2) % 3);
        for outward in [1, -1] {
            let step = *across * outward;
            for k in 0..SIZE_I {
                for w in 0..SIZE_I {
                    for u in 0..SIZE_I {
                        let p = *across * k + AXES[b] * u + AXES[c] * w;
                        let here = samples.get(p);
                        mask[u as usize + SIZE * w as usize] = if here.is_solid()
                            && (!samples.get(p + step).is_solid() || extra(p, step))
                        {
                            u16::from(here.material) + 1
                        } else {
                            0
                        };
                    }
                }
                let face = Face {
                    a,
                    k,
                    outward,
                    origin: samples.origin,
                };
                if merge {
                    merged(&mut out, &mut mask, &face);
                } else {
                    for (i, m) in mask.iter().enumerate() {
                        if *m != 0 {
                            let (u, w) = ((i % SIZE) as i32, (i / SIZE) as i32);
                            face.push(&mut out, (u, w), (1, 1), (*m - 1) as u8);
                        }
                    }
                }
            }
        }
    }
    out
}

/// One slice of faces: across axis `a`, at `k`, facing `outward`.
struct Face {
    a: usize,
    k: i32,
    outward: i32,
    origin: IVec3,
}

impl Face {
    fn push(
        &self,
        out: &mut ChunkMesh,
        (u, w): (i32, i32),
        (width, height): (i32, i32),
        material: u8,
    ) {
        let (b, c) = ((self.a + 1) % 3, (self.a + 2) % 3);
        let normal = AXES[self.a].as_vec3() * self.outward as f32;
        let at = |du: i32, dw: i32| {
            let mut p = Vec3::ZERO;
            p[self.a] = self.k as f32 + 0.5 * self.outward as f32;
            p[b] = (u + du) as f32 - 0.5;
            p[c] = (w + dw) as f32 - 0.5;
            self.origin.as_vec3() + p
        };
        let mut corners = [at(0, 0), at(width, 0), at(width, height), at(0, height)];
        if self.outward < 0 {
            corners.reverse();
        }
        let first = out.positions.len() as u32;
        for corner in corners {
            out.push_point(corner, normal, material);
        }
        out.indices
            .extend_from_slice(&[first, first + 1, first + 2, first, first + 2, first + 3]);
    }
}

/// Grows each unused face into the widest, then tallest, rectangle of
/// the same material, and clears what it covers.
fn merged(out: &mut ChunkMesh, mask: &mut [u16], face: &Face) {
    for w in 0..SIZE {
        let mut u = 0;
        while u < SIZE {
            let m = mask[u + SIZE * w];
            if m == 0 {
                u += 1;
                continue;
            }
            let mut width = 1;
            while u + width < SIZE && mask[u + width + SIZE * w] == m {
                width += 1;
            }
            let mut height = 1;
            'grow: while w + height < SIZE {
                for du in 0..width {
                    if mask[u + du + SIZE * (w + height)] != m {
                        break 'grow;
                    }
                }
                height += 1;
            }
            for dw in 0..height {
                mask[u + SIZE * (w + dw)..u + width + SIZE * (w + dw)].fill(0);
            }
            face.push(
                out,
                (u as i32, w as i32),
                (width as i32, height as i32),
                (m - 1) as u8,
            );
            u += width;
        }
    }
}
