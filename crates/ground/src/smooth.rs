//! The Smooth look: surface nets (decision 0116, the study's choice).
//!
//! One point in each cube the surface passes through, at the middle of
//! where it crosses the cube's edges, and a square joining the four cubes
//! round each edge the surface crosses. A chunk owns the edges starting
//! at its own voxels, so each square is made by exactly one chunk.
//!
//! - **Each crossed edge is worked out once** (the study's busiest chunk
//!   spent most of its 4.5 ms working each one out four times): where it
//!   is crossed is kept by edge and read by the four cubes round it.
//! - **Facings come from the voxels**, not the triangles: each point
//!   faces the way the distance grows across its cube's eight corners,
//!   so two chunks give a shared point the same facing and the seam
//!   shades smoothly.
//! - **Places are chunk-local** (decision 0117: a far origin later), and
//!   each point's place within its cube is held to 1/1024 of a metre, so
//!   a point two chunks share lands on exactly the same place in both.

use crate::ground::{BORDER, SPAN, Samples};
use crate::store::origin;
use crate::voxel::{SIZE, SIZE_I, Voxel};
use glam::{IVec3, Vec3};

/// A chunk's triangles, ready to draw and to make solid.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct ChunkMesh {
    /// The chunk's key.
    pub key: IVec3,
    /// Where the chunk's first voxel is in the world, in metres.
    pub origin: IVec3,
    /// Each point's place from [`Self::origin`], in metres.
    pub positions: Vec<[f32; 3]>,
    /// Each point's facing (towards the air), of length one.
    pub normals: Vec<[f32; 3]>,
    /// Each point's ground kind.
    pub kinds: Vec<u8>,
    /// Three points per triangle, counter-clockwise seen from the air.
    pub indices: Vec<u32>,
}

impl ChunkMesh {
    /// How many triangles it has.
    pub fn triangles(&self) -> usize {
        self.indices.len() / 3
    }

    /// Whether it has no triangles.
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    /// Point `i`'s place in the world, in metres.
    pub fn world(&self, i: u32) -> Vec3 {
        let p = self.positions.get(i as usize).copied().unwrap_or_default();
        self.origin.as_vec3() + Vec3::from_array(p)
    }

    /// Its bytes, in a fixed order: two meshes of the same samples give
    /// the same bytes on every PC.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for p in self.positions.iter().chain(&self.normals) {
            for f in p {
                out.extend_from_slice(&f.to_le_bytes());
            }
        }
        out.extend_from_slice(&self.kinds);
        for i in &self.indices {
            out.extend_from_slice(&i.to_le_bytes());
        }
        out
    }
}

const AXES: [IVec3; 3] = [IVec3::X, IVec3::Y, IVec3::Z];
/// Cubes along each side the mesher looks at: from one before the
/// chunk's first voxel to its last.
const CUBES: usize = SIZE + 1;
const NONE: u32 = u32::MAX;
/// How finely a point's place in its cube is kept: 1/1024 of a metre.
const PLACE_STEPS: f32 = 1024.0;

/// What the mesher keeps between chunks, so a worker meshing many reuses
/// its memory.
#[derive(Debug, Clone)]
pub struct Mesher {
    points: Vec<u32>,
    crossings: Vec<f32>,
}

impl Default for Mesher {
    fn default() -> Self {
        Self {
            points: vec![NONE; CUBES * CUBES * CUBES],
            crossings: vec![f32::NAN; SPAN * SPAN * SPAN * 3],
        }
    }
}

/// The chunk's mesh in the Smooth look, made from `samples`.
pub fn mesh(samples: &Samples) -> ChunkMesh {
    Mesher::default().mesh(samples)
}

impl Mesher {
    /// The chunk's mesh in the Smooth look, made from `samples`.
    pub fn mesh(&mut self, samples: &Samples) -> ChunkMesh {
        let mut out = ChunkMesh {
            key: samples.key,
            origin: origin(samples.key),
            ..ChunkMesh::default()
        };
        if samples.is_bare() {
            return out;
        }
        self.points.fill(NONE);
        self.crossings.fill(f32::NAN);
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
                        let corner = quad.map(|cube| self.point(&mut out, samples, cube));
                        push_quad(&mut out, corner);
                    }
                }
            }
        }
        out
    }

    /// The point in `cube` (named by its lowest corner), made once.
    fn point(&mut self, out: &mut ChunkMesh, samples: &Samples, cube: IVec3) -> u32 {
        let q = (cube + 1).as_uvec3();
        let slot = q.x as usize + CUBES * (q.y as usize + CUBES * q.z as usize);
        if self.points[slot] != NONE {
            return self.points[slot];
        }
        let corners: [Voxel; 8] = std::array::from_fn(|i| samples.get(cube + corner(i)));
        let mut sum = Vec3::ZERO;
        let mut count = 0u32;
        for (i, v) in corners.iter().enumerate() {
            let c = corner(i);
            for (a, axis) in AXES.iter().enumerate() {
                if c[a] == 1 || corners[i + (1 << a)].is_solid() == v.is_solid() {
                    continue;
                }
                let t = self.crossing(samples, cube + c, a);
                sum += c.as_vec3() + axis.as_vec3() * t;
                count += 1;
            }
        }
        let local = (sum / count.max(1) as f32).clamp(Vec3::ZERO, Vec3::ONE);
        let local = (local * PLACE_STEPS).round() / PLACE_STEPS;
        let normal = facing(&corners, local);
        let index = out.positions.len() as u32;
        out.positions.push((cube.as_vec3() + local).to_array());
        out.normals.push(normal.to_array());
        out.kinds.push(most_common_solid(&corners));
        self.points[slot] = index;
        index
    }

    /// How far along the edge from `from` along axis `a` the surface
    /// crosses it (0 to 1), worked out once per edge.
    fn crossing(&mut self, samples: &Samples, from: IVec3, a: usize) -> f32 {
        let q = (from + BORDER).as_uvec3();
        let slot = 3 * (q.x as usize + SPAN * (q.y as usize + SPAN * q.z as usize)) + a;
        let kept = self.crossings[slot];
        if !kept.is_nan() {
            return kept;
        }
        let dv = samples.get(from).distance();
        let dw = samples.get(from + AXES[a]).distance();
        let t = if dv == dw {
            0.5
        } else {
            (dv / (dv - dw)).clamp(0.0, 1.0)
        };
        self.crossings[slot] = t;
        t
    }
}

/// Corner `i` of a cube (bit 0 x, bit 1 y, bit 2 z).
fn corner(i: usize) -> IVec3 {
    IVec3::new((i & 1) as i32, ((i >> 1) & 1) as i32, ((i >> 2) & 1) as i32)
}

/// The way the distance grows at `local` in a cube whose corners are
/// `c`: the slope of the distance blended between them.
fn facing(c: &[Voxel; 8], local: Vec3) -> Vec3 {
    let d = |i: usize| c[i].distance();
    let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
    let Vec3 { x, y, z } = local;
    let gx = lerp(
        lerp(d(1) - d(0), d(3) - d(2), y),
        lerp(d(5) - d(4), d(7) - d(6), y),
        z,
    );
    let gy = lerp(
        lerp(d(2) - d(0), d(3) - d(1), x),
        lerp(d(6) - d(4), d(7) - d(5), x),
        z,
    );
    let gz = lerp(
        lerp(d(4) - d(0), d(5) - d(1), x),
        lerp(d(6) - d(2), d(7) - d(3), x),
        y,
    );
    Vec3::new(gx, gy, gz).normalize_or(Vec3::Z)
}

/// The most common kind among the solid corners, the lowest of equals.
fn most_common_solid(corners: &[Voxel; 8]) -> u8 {
    let mut best = (0, 0u8);
    for v in corners.iter().filter(|v| v.is_solid()) {
        let count = corners
            .iter()
            .filter(|w| w.is_solid() && w.kind == v.kind)
            .count();
        if count > best.0 || (count == best.0 && v.kind < best.1) {
            best = (count, v.kind);
        }
    }
    best.1
}

/// Two triangles, split along the shorter diagonal.
fn push_quad(out: &mut ChunkMesh, [a, b, c, d]: [u32; 4]) {
    let p = |i: u32| Vec3::from_array(out.positions[i as usize]);
    if p(a).distance_squared(p(c)) <= p(b).distance_squared(p(d)) {
        out.indices.extend_from_slice(&[a, b, c, a, c, d]);
    } else {
        out.indices.extend_from_slice(&[b, c, d, b, d, a]);
    }
}

#[cfg(test)]
#[path = "smooth_tests.rs"]
mod tests;
