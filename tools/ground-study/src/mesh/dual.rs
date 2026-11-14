//! Surface nets and dual contouring: one point in each cube the surface
//! passes through, and a square joining the four cubes round each edge
//! the surface crosses. They differ only in where the point goes.

use super::ChunkMesh;
use super::normals::sharpen;
use super::qef;
use crate::ground::Samples;
use crate::voxel::{SIZE, SIZE_I};
use glam::{IVec3, Vec3};

/// Where a cube's point goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Placement {
    /// The middle of where the surface crosses the cube's edges: smooth
    /// and rounded, corners worn away.
    Middle,
    /// Where the planes the surface crosses at meet: keeps corners and
    /// cliff edges sharp.
    Planes,
    /// The middle, pulled a few steps towards those planes: most of the
    /// sharpness for less work.
    Pulled,
}

/// The faces at more than this angle to each other get a hard edge.
const HARD_EDGE_DEGREES: f32 = 40.0;
const PULL_STEPS: usize = 4;
const AXES: [IVec3; 3] = [IVec3::X, IVec3::Y, IVec3::Z];
/// Cubes from one before the chunk's first sample to its last.
const CELLS: usize = SIZE + 1;
const NONE: u32 = u32::MAX;

/// The chunk's mesh: a square for every edge the surface crosses from
/// one of the chunk's own samples.
pub(crate) fn mesh(samples: &Samples, placement: Placement) -> ChunkMesh {
    let mut out = ChunkMesh::default();
    let mut points = vec![NONE; CELLS * CELLS * CELLS];
    for z in 0..SIZE_I {
        for y in 0..SIZE_I {
            for x in 0..SIZE_I {
                let s = IVec3::new(x, y, z);
                let inside = samples.get(s).is_solid();
                for (a, axis) in AXES.iter().enumerate() {
                    if samples.get(s + *axis).is_solid() == inside {
                        continue;
                    }
                    let (b, c) = (AXES[(a + 1) % 3], AXES[(a + 2) % 3]);
                    let mut quad = [s - b - c, s - c, s, s - b];
                    if !inside {
                        quad.reverse();
                    }
                    let mut corner = [0u32; 4];
                    for (k, cell) in quad.iter().enumerate() {
                        corner[k] = point(&mut out, &mut points, samples, *cell, placement);
                    }
                    push_quad(&mut out, corner);
                }
            }
        }
    }
    match placement {
        Placement::Middle => out,
        Placement::Planes | Placement::Pulled => sharpen(&out, HARD_EDGE_DEGREES),
    }
}

fn push_quad(out: &mut ChunkMesh, [a, b, c, d]: [u32; 4]) {
    let p = |i: u32| out.positions[i as usize];
    if p(a).distance_squared(p(c)) <= p(b).distance_squared(p(d)) {
        out.indices.extend_from_slice(&[a, b, c, a, c, d]);
    } else {
        out.indices.extend_from_slice(&[b, c, d, b, d, a]);
    }
}

/// The point in `cell` (named by its lowest corner), made once.
fn point(
    out: &mut ChunkMesh,
    points: &mut [u32],
    samples: &Samples,
    cell: IVec3,
    placement: Placement,
) -> u32 {
    let q = (cell + 1).as_uvec3();
    let slot = q.x as usize + CELLS * (q.y as usize + CELLS * q.z as usize);
    if points[slot] != NONE {
        return points[slot];
    }
    let found = crossings(samples, cell);
    let middle = found.places.iter().sum::<Vec3>() / found.count as f32;
    let local = match placement {
        Placement::Middle => middle,
        Placement::Planes => qef::solve(found.places(), found.normals(), middle),
        Placement::Pulled => pulled(&found, middle),
    }
    .clamp(Vec3::ZERO, Vec3::ONE);
    let normal = gradient_in(samples, cell, local);
    let position = (samples.origin + cell).as_vec3() + local;
    let index = out.push_point(position, normal, found.material);
    points[slot] = index;
    index
}

/// Where the surface crosses a cube's edges, from its lowest corner.
struct Crossings {
    places: [Vec3; 12],
    normals: [Vec3; 12],
    count: usize,
    material: u8,
}

impl Crossings {
    fn places(&self) -> &[Vec3] {
        &self.places[..self.count]
    }

    fn normals(&self) -> &[Vec3] {
        &self.normals[..self.count]
    }
}

fn crossings(samples: &Samples, cell: IVec3) -> Crossings {
    let mut found = Crossings {
        places: [Vec3::ZERO; 12],
        normals: [Vec3::ZERO; 12],
        count: 0,
        material: 0,
    };
    let mut materials = [0u8; 8];
    let mut solid = 0;
    for corner in 0..8 {
        let c = IVec3::new(corner & 1, (corner >> 1) & 1, corner >> 2);
        let v = samples.get(cell + c);
        if v.is_solid() {
            materials[solid] = v.material;
            solid += 1;
        }
        for (a, axis) in AXES.iter().enumerate() {
            if c[a] == 1 {
                continue;
            }
            let end = c + *axis;
            let w = samples.get(cell + end);
            if v.is_solid() == w.is_solid() {
                continue;
            }
            let (dv, dw) = (v.distance(), w.distance());
            let t = (dv / (dv - dw)).clamp(0.0, 1.0);
            let normal = samples
                .gradient(cell + c)
                .lerp(samples.gradient(cell + end), t)
                .normalize_or(axis.as_vec3());
            found.places[found.count] = c.as_vec3() + axis.as_vec3() * t;
            found.normals[found.count] = normal;
            found.count += 1;
        }
    }
    found.material = most_common(&materials[..solid]);
    found
}

/// The most common material, the lowest of equals.
fn most_common(materials: &[u8]) -> u8 {
    let mut best = (0, 0u8);
    for m in materials {
        let count = materials.iter().filter(|n| *n == m).count();
        if count > best.0 || (count == best.0 && *m < best.1) {
            best = (count, *m);
        }
    }
    best.1
}

fn pulled(found: &Crossings, middle: Vec3) -> Vec3 {
    let mut x = middle;
    for _ in 0..PULL_STEPS {
        let mut step = Vec3::ZERO;
        for (p, n) in found.places().iter().zip(found.normals()) {
            step += *n * n.dot(*p - x);
        }
        x += step / found.count as f32;
    }
    x
}

/// The surface's facing at `local` in `cell`, from its corners'.
fn gradient_in(samples: &Samples, cell: IVec3, local: Vec3) -> Vec3 {
    let g = |dx, dy, dz| samples.gradient(cell + IVec3::new(dx, dy, dz));
    let x0 = g(0, 0, 0).lerp(g(1, 0, 0), local.x);
    let x1 = g(0, 1, 0).lerp(g(1, 1, 0), local.x);
    let x2 = g(0, 0, 1).lerp(g(1, 0, 1), local.x);
    let x3 = g(0, 1, 1).lerp(g(1, 1, 1), local.x);
    x0.lerp(x1, local.y)
        .lerp(x2.lerp(x3, local.y), local.z)
        .normalize_or(Vec3::Z)
}
