//! Finding cracks two ways:
//!
//! - [`open_edges`]: the meshes welded by place, counting edges with a
//!   triangle on one side only. A closed ground has none (but at its
//!   own edges); skirts leave theirs underground, out of sight.
//! - [`Scene::look`]: rays from the camera to places on the ground; a
//!   ray that sees the inside of the ground first (a triangle's back) or
//!   nothing at all went through a crack. This is what a player would
//!   see.

use crate::mesh::ChunkMesh;
use glam::{IVec2, Vec2, Vec3};
use std::collections::HashMap;

/// How finely places are welded: a 256th of a metre.
const WELD: f32 = 256.0;

/// Edges of `meshes`, welded together, with a triangle on one side
/// only, leaving out those whose two ends `skip` names (the world's
/// edges, say).
pub fn open_edges(meshes: &[&ChunkMesh], skip: impl Fn(Vec3) -> bool) -> usize {
    let mut ids: HashMap<[i64; 3], u32> = HashMap::new();
    let mut edges: HashMap<(u32, u32), i64> = HashMap::new();
    let mut at = Vec::new();
    for mesh in meshes {
        let id: Vec<u32> = mesh
            .positions
            .iter()
            .map(|p| {
                let key = (*p * WELD).round().to_array().map(|v| v as i64);
                let next = ids.len() as u32;
                let i = *ids.entry(key).or_insert(next);
                if i == next {
                    at.push(*p);
                }
                i
            })
            .collect();
        for t in mesh.indices.chunks_exact(3) {
            let [a, b, c] = [t[0], t[1], t[2]].map(|i| id[i as usize]);
            if a == b || b == c || a == c {
                continue;
            }
            for (p, q) in [(a, b), (b, c), (c, a)] {
                *edges.entry((p.min(q), p.max(q))).or_default() += if p < q { 1 } else { -1 };
            }
        }
    }
    edges
        .iter()
        .filter(|((a, b), n)| **n != 0 && !(skip(at[*a as usize]) && skip(at[*b as usize])))
        .count()
}

/// Triangles sorted into squares of the ground, for casting rays.
pub struct Scene {
    cell: f32,
    lo: Vec2,
    across: IVec2,
    bins: Vec<Vec<u32>>,
    triangles: Vec<[Vec3; 3]>,
}

/// What rays saw.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Seen {
    /// Rays cast.
    pub rays: usize,
    /// Rays that saw the inside of the ground, or nothing: cracks.
    pub through: usize,
}

impl Scene {
    /// `meshes`' triangles in squares `cell` metres wide.
    pub fn new(meshes: &[&ChunkMesh], cell: f32) -> Self {
        let mut triangles = Vec::new();
        for mesh in meshes {
            for t in mesh.indices.chunks_exact(3) {
                triangles.push([t[0], t[1], t[2]].map(|i| mesh.positions[i as usize]));
            }
        }
        let (mut lo, mut hi) = (Vec2::splat(f32::MAX), Vec2::splat(f32::MIN));
        for t in &triangles {
            for p in t {
                lo = lo.min(p.truncate());
                hi = hi.max(p.truncate());
            }
        }
        if triangles.is_empty() {
            (lo, hi) = (Vec2::ZERO, Vec2::ONE);
        }
        let across = ((hi - lo) / cell).floor().as_ivec2() + 1;
        let mut bins = vec![Vec::new(); (across.x * across.y) as usize];
        for (i, t) in triangles.iter().enumerate() {
            let a = t[0].truncate().min(t[1].truncate()).min(t[2].truncate());
            let b = t[0].truncate().max(t[1].truncate()).max(t[2].truncate());
            let (first, last) = (
                ((a - lo) / cell).floor().as_ivec2(),
                ((b - lo) / cell).floor().as_ivec2(),
            );
            for y in first.y..=last.y {
                for x in first.x..=last.x {
                    bins[(x + across.x * y) as usize].push(i as u32);
                }
            }
        }
        Self {
            cell,
            lo,
            across,
            bins,
            triangles,
        }
    }

    /// The first triangle along the ray from `from` along `way` (of any
    /// length) within `reach` metres: how far, and whether it was its
    /// front that was seen.
    pub fn first_hit(&self, from: Vec3, way: Vec3, reach: f32) -> Option<(f32, bool)> {
        let way = way.normalize_or_zero();
        let flat = way.truncate();
        let start = (from.truncate() - self.lo) / self.cell;
        let mut cell = start.floor().as_ivec2();
        let step = IVec2::new(flat.x.signum() as i32, flat.y.signum() as i32);
        let next_edge = |c: i32, s: i32| (c + i32::from(s > 0)) as f32;
        let mut t_max = Vec2::new(
            if flat.x == 0.0 {
                f32::MAX
            } else {
                (next_edge(cell.x, step.x) - start.x) * self.cell / flat.x
            },
            if flat.y == 0.0 {
                f32::MAX
            } else {
                (next_edge(cell.y, step.y) - start.y) * self.cell / flat.y
            },
        );
        let t_step = Vec2::new(
            if flat.x == 0.0 {
                f32::MAX
            } else {
                self.cell / flat.x.abs()
            },
            if flat.y == 0.0 {
                f32::MAX
            } else {
                self.cell / flat.y.abs()
            },
        );
        let mut best: Option<(f32, bool)> = None;
        let mut t_here = 0.0;
        while t_here <= reach {
            if cell.cmpge(IVec2::ZERO).all() && cell.cmplt(self.across).all() {
                for &i in &self.bins[(cell.x + self.across.x * cell.y) as usize] {
                    if let Some(hit) = hit(&self.triangles[i as usize], from, way)
                        && hit.0 <= reach
                        && best.is_none_or(|b| nearer(hit, b))
                    {
                        best = Some(hit);
                    }
                }
            }
            let t_out = t_max.x.min(t_max.y);
            if best.is_some_and(|b| b.0 <= t_out) {
                break;
            }
            if t_max.x < t_max.y {
                cell.x += step.x;
                t_max.x += t_step.x;
            } else {
                cell.y += step.y;
                t_max.y += t_step.y;
            }
            t_here = t_out;
            if t_here == f32::MAX {
                break;
            }
        }
        best
    }

    /// Casts a ray from `eye` through each of `targets` (on the ground)
    /// and a little past it, counting those that went through: that saw
    /// the back of a triangle first, or passed deep into the ground
    /// (`inside` says where that is, checked every `step` metres, given
    /// the place) before meeting any.
    pub fn look(
        &self,
        eye: Vec3,
        targets: &[Vec3],
        inside: impl Fn(Vec3) -> bool,
        step: impl Fn(Vec3) -> f32,
    ) -> Seen {
        let mut seen = Seen::default();
        for target in targets {
            let way = *target - eye;
            let reach = way.length() * 1.5 + 4.0;
            seen.rays += 1;
            let first = self.first_hit(eye, way, reach);
            let until = first.map_or(reach, |(t, _)| t);
            let dir = way.normalize_or_zero();
            let mut t = step(eye).max(0.01);
            let mut went_in = false;
            while t < until {
                let p = eye + dir * t;
                if inside(p) {
                    went_in = true;
                    break;
                }
                t += step(p).max(0.01);
            }
            if went_in || matches!(first, Some((_, false))) {
                seen.through += 1;
            }
        }
        seen
    }
}

/// Whether `p` (metres) is well inside the ground both at the level
/// drawn there and at the one above: a voxel and a half deep in each, so
/// that two levels' surfaces differing never counts as a crack.
pub fn deep(levels: &impl super::Levels, layout: &super::layout::Layout, p: Vec3) -> bool {
    let here = layout.level_at(p.truncate());
    [here, here + 1].iter().all(|level| {
        let size = levels.size(*level);
        if size.cmple(glam::IVec3::ZERO).any() {
            return true;
        }
        let g = (p / super::voxel(*level)).round().as_ivec3();
        levels.voxel(*level, g).distance() < -1.5
    })
}

/// Whether `hit` comes before `best`; two at the same place (a ray
/// meeting a skirt just where the surface it hangs from ends) count as
/// the front one, as a picture would show it.
fn nearer(hit: (f32, bool), best: (f32, bool)) -> bool {
    let tie = 1e-4 * best.0.max(1.0);
    if (hit.0 - best.0).abs() <= tie {
        hit.1 && !best.1
    } else {
        hit.0 < best.0
    }
}

/// Where a ray meets a triangle (either side): how far, and whether its
/// front (counter-clockwise from outside) faces the ray.
fn hit(t: &[Vec3; 3], from: Vec3, way: Vec3) -> Option<(f32, bool)> {
    let (e1, e2) = (t[1] - t[0], t[2] - t[0]);
    let p = way.cross(e2);
    let det = e1.dot(p);
    if det.abs() < 1e-12 {
        return None;
    }
    let inv = 1.0 / det;
    let s = from - t[0];
    // A hair of slack, so a ray passing exactly along an edge between
    // two triangles meets one of them (a test without it lets such rays
    // through, which would count as cracks).
    const SLACK: f32 = 1e-4;
    let u = s.dot(p) * inv;
    if !(-SLACK..=1.0 + SLACK).contains(&u) {
        return None;
    }
    let q = s.cross(e1);
    let v = way.dot(q) * inv;
    if v < -SLACK || u + v > 1.0 + SLACK {
        return None;
    }
    let distance = e2.dot(q) * inv;
    (distance > 1e-4).then_some((distance, det > 0.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn floor(lo: f32, hi: f32, z: f32) -> ChunkMesh {
        ChunkMesh {
            positions: vec![
                Vec3::new(lo, lo, z),
                Vec3::new(hi, lo, z),
                Vec3::new(hi, hi, z),
                Vec3::new(lo, hi, z),
            ],
            normals: vec![Vec3::Z; 4],
            materials: vec![0; 4],
            indices: vec![0, 1, 2, 0, 2, 3],
        }
    }

    #[test]
    fn a_ray_sees_the_front_of_a_floor_and_the_back_from_under_it() {
        let scene = Scene::new(&[&floor(0.0, 10.0, 0.0)], 2.0);
        let down = scene.first_hit(Vec3::new(5.0, 5.0, 3.0), Vec3::new(0.3, 0.1, -1.0), 10.0);
        assert!(matches!(down, Some((_, true))));
        let up = scene.first_hit(Vec3::new(5.0, 5.0, -3.0), Vec3::Z, 10.0);
        assert!(matches!(up, Some((_, false))));
        assert_eq!(
            scene.first_hit(Vec3::new(5.0, 5.0, 3.0), Vec3::Z, 10.0),
            None
        );
    }

    #[test]
    fn a_gap_between_two_floors_is_seen_through_and_open() {
        let (a, b) = (floor(0.0, 4.0, 0.0), floor(4.5, 9.0, 0.0));
        let scene = Scene::new(&[&a, &b], 1.0);
        let seen = scene.look(
            Vec3::new(4.25, -5.0, 5.0),
            &[Vec3::new(4.25, 4.0, 0.0), Vec3::new(2.0, 4.0, 0.0)],
            |p| p.z < -0.5,
            |_| 0.25,
        );
        assert_eq!(
            seen,
            Seen {
                rays: 2,
                through: 1
            }
        );
        assert_eq!(open_edges(&[&a, &b], |_| false), 8);
        assert_eq!(open_edges(&[&a], |p| p.x == 0.0), 3);
    }
}
