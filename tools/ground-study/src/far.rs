//! The far field: past some distance the ground is drawn as a height
//! map made from its voxels (the top of each column of samples), a grid
//! of squares per column of chunks, cheap all the way to the horizon.
//! Caves and overhangs that far away can't be seen anyway. Each tile
//! hangs skirts from its sides, so tiles of different levels, and the
//! voxel chunks nearer the camera, meet without a crack showing.

use crate::lod::{Levels, chunk_metres, voxel};
use crate::mesh::ChunkMesh;
use crate::voxel::SIZE_I;
use glam::{IVec2, IVec3, Vec2, Vec3};

/// The top of the ground over sample `xy` of `level`: its height in
/// metres, where the topmost solid sample meets the air above it, and
/// that sample's kind.
pub fn top(levels: &impl Levels, level: u32, xy: IVec2) -> (f32, u8) {
    let high = levels.size(level).z;
    let width = voxel(level);
    let mut above = levels.voxel(level, xy.extend(high - 1));
    if above.is_solid() {
        return ((high - 1) as f32 * width, above.material);
    }
    for k in (0..high - 1).rev() {
        let here = levels.voxel(level, xy.extend(k));
        if here.is_solid() {
            let (a, b) = (here.distance(), above.distance());
            let t = (a / (a - b)).clamp(0.0, 1.0);
            return ((k as f32 + t) * width, here.material);
        }
        above = here;
    }
    (0.0, 0)
}

/// The tile for the column `key` of `level`: a grid of 32 × 32 squares,
/// a sample's width each, in metres, with skirts `2 · 2^(level + 1)`
/// metres deep down its sides when `skirts` is set.
pub fn tile(levels: &impl Levels, level: u32, key: IVec2, skirts: bool) -> ChunkMesh {
    const SIDE: i32 = SIZE_I + 1;
    const RIM: i32 = SIDE + 2;
    let width = voxel(level);
    let origin = key * SIZE_I;
    let mut heights = vec![(0.0f32, 0u8); (RIM * RIM) as usize];
    for y in 0..RIM {
        for x in 0..RIM {
            heights[(x + RIM * y) as usize] = top(levels, level, origin + IVec2::new(x - 1, y - 1));
        }
    }
    let at = |x: i32, y: i32| heights[(x + 1 + RIM * (y + 1)) as usize];
    let mut out = ChunkMesh::default();
    for y in 0..SIDE {
        for x in 0..SIDE {
            let (h, kind) = at(x, y);
            let slope = Vec2::new(
                at(x + 1, y).0 - at(x - 1, y).0,
                at(x, y + 1).0 - at(x, y - 1).0,
            ) / (2.0 * width);
            let place = ((origin + IVec2::new(x, y)).as_vec2() * width).extend(h);
            out.push_point(place, Vec3::new(-slope.x, -slope.y, 1.0).normalize(), kind);
        }
    }
    let point = |x: i32, y: i32| (x + SIDE * y) as u32;
    for y in 0..SIZE_I {
        for x in 0..SIZE_I {
            let [a, b, c, d] = [
                point(x, y),
                point(x + 1, y),
                point(x + 1, y + 1),
                point(x, y + 1),
            ];
            out.indices.extend_from_slice(&[a, b, c, a, c, d]);
        }
    }
    if skirts {
        // Round the tile counter-clockwise, as its triangles go along
        // its sides.
        let mut rim = Vec::new();
        rim.extend((0..SIZE_I).map(|x| (point(x, 0), point(x + 1, 0))));
        rim.extend((0..SIZE_I).map(|y| (point(SIZE_I, y), point(SIZE_I, y + 1))));
        rim.extend((0..SIZE_I).map(|x| (point(x + 1, SIZE_I), point(x, SIZE_I))));
        rim.extend((0..SIZE_I).map(|y| (point(0, y + 1), point(0, y))));
        let depth = 2.0 * voxel(level + 1);
        for (a, b) in rim {
            let (pa, pb) = (out.positions[a as usize], out.positions[b as usize]);
            let (na, nb) = (out.normals[a as usize], out.normals[b as usize]);
            let (ka, kb) = (out.materials[a as usize], out.materials[b as usize]);
            let top_b = out.push_point(pb, nb, kb);
            let top_a = out.push_point(pa, na, ka);
            let low_a = out.push_point(pa - Vec3::Z * depth, na, ka);
            let low_b = out.push_point(pb - Vec3::Z * depth, nb, kb);
            out.indices
                .extend_from_slice(&[top_b, top_a, low_a, top_b, low_a, low_b]);
        }
    }
    out
}

/// A whole world's height map at one spacing, kept in memory: what
/// units far from the camera and the editor's map read.
#[derive(Debug, Clone)]
pub struct Heights {
    /// Metres between samples.
    pub spacing: f32,
    /// Samples along each side.
    pub across: IVec2,
    /// Each sample's height in metres, x fastest.
    pub heights: Vec<f32>,
    /// Each sample's ground kind.
    pub kinds: Vec<u8>,
}

impl Heights {
    /// Made by asking `height` (given a place in metres) at every
    /// sample, `spacing` metres apart, over a world `side` metres across.
    pub fn sampled(side: f32, spacing: f32, height: impl Fn(Vec2) -> (f32, u8)) -> Self {
        let n = (side / spacing).round() as i32 + 1;
        let mut heights = Vec::with_capacity((n * n) as usize);
        let mut kinds = Vec::with_capacity((n * n) as usize);
        for y in 0..n {
            for x in 0..n {
                let (h, k) = height(IVec2::new(x, y).as_vec2() * spacing);
                heights.push(h);
                kinds.push(k);
            }
        }
        Self {
            spacing,
            across: IVec2::splat(n),
            heights,
            kinds,
        }
    }

    /// Made from `level`'s voxels, a sample per column of samples.
    pub fn of(levels: &impl Levels, level: u32) -> Self {
        let size = levels.size(level);
        let width = voxel(level);
        Self::sampled((size.x - 1) as f32 * width, width, |p| {
            top(levels, level, (p / width).round().as_ivec2())
        })
    }

    /// The height at `p` (metres), blended between samples.
    pub fn at(&self, p: Vec2) -> f32 {
        let q = (p / self.spacing).clamp(Vec2::ZERO, (self.across - 1).as_vec2());
        let base = q.floor().as_ivec2().min(self.across - 2).max(IVec2::ZERO);
        let t = q - base.as_vec2();
        let h = |d: IVec2| {
            let c = (base + d).min(self.across - 1);
            self.heights[(c.x + self.across.x * c.y) as usize]
        };
        let low = h(IVec2::ZERO) + (h(IVec2::X) - h(IVec2::ZERO)) * t.x;
        let high = h(IVec2::Y) + (h(IVec2::ONE) - h(IVec2::Y)) * t.x;
        low + (high - low) * t.y
    }

    /// The memory it takes, in bytes.
    pub fn bytes(&self) -> usize {
        self.heights.len() * 4 + self.kinds.len()
    }
}

/// Where the tile `key` of `level` is (its lowest corner), in metres.
pub fn corner(level: u32, key: IVec2) -> Vec2 {
    (key * chunk_metres(level)).as_vec2()
}

/// The sample of `level` nearest `p` (metres), as a voxel index with
/// height 0.
pub fn nearest(level: u32, p: Vec2) -> IVec3 {
    (p / voxel(level)).round().as_ivec2().extend(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ground::Ground;
    use crate::lod::Pyramid;
    use crate::voxel::Voxel;
    use glam::UVec3;

    fn slope() -> Pyramid {
        let ground = Ground::build(UVec3::new(2, 2, 2), |key, dense| {
            crate::shapes::fill(key, dense, |p| Voxel::at(p.z - 10.0 - p.x * 0.25, 1));
        });
        Pyramid::build(ground, 2)
    }

    #[test]
    fn the_top_is_where_the_ground_meets_the_air() {
        let p = slope();
        let (h, kind) = top(&p, 0, IVec2::new(8, 3));
        assert!((h - 12.0).abs() < 0.1, "{h}");
        assert_eq!(kind, 1);
        let (h1, _) = top(&p, 1, IVec2::new(4, 2));
        assert!((h1 - 12.0).abs() < 0.3, "level 1: {h1}");
    }

    #[test]
    fn a_tile_is_a_grid_with_skirts_round_it() {
        let p = slope();
        let plain = tile(&p, 0, IVec2::ZERO, false);
        assert_eq!(plain.triangles(), 32 * 32 * 2);
        let skirted = tile(&p, 0, IVec2::ZERO, true);
        assert_eq!(skirted.triangles(), 32 * 32 * 2 + 4 * 32 * 2);
        assert!(plain.normals.iter().all(|n| n.z > 0.9));
    }

    #[test]
    fn heights_blend_between_samples() {
        let h = Heights::sampled(64.0, 32.0, |p| (p.x * 0.5, 0));
        assert_eq!(h.across, IVec2::splat(3));
        assert!((h.at(Vec2::new(16.0, 5.0)) - 8.0).abs() < 1e-4);
        assert!((h.at(Vec2::new(500.0, 5.0)) - 32.0).abs() < 1e-4);
    }
}
