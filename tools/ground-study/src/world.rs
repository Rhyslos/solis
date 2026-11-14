//! The open-worlds study's test world: 20 × 20 km of mountain ranges,
//! valleys and rolling hills, 128 m high, with winding caves under the
//! surface, made from formulas so any part of it can be made at any
//! level of detail without the rest.
//!
//! Level 0 has a voxel a metre; each level up doubles it. A level's
//! sample `i` sits at `i · 2^level` metres (see [`crate::lod`]), so a
//! coarse level's samples are every other one of the level below.

use crate::lod;
use crate::noise::{layered2, noise2, noise3};
use crate::shapes::{DIRT, GRASS, ROCK, SAND};
use crate::voxel::{Dense, SIZE, SIZE_I, Voxel, index};
use glam::{IVec3, UVec3, Vec2, Vec3};

/// Snow, on the highest peaks.
pub const SNOW: u8 = 4;
/// The world's height in metres.
pub const HEIGHT: i32 = 128;
/// The full world's side in metres: 80 regions of 256 m.
pub const SIDE: i32 = 20_480;

/// Caves wind between these depths below the surface, in metres.
const CAVE_TOP: f32 = 1.0;
const CAVE_BOTTOM: f32 = 28.0;

/// The surface's height at `q` (metres), and how steep it is there
/// (rise over run).
pub fn height(q: Vec2) -> f32 {
    let ranges = layered2(11, q / 4200.0, 3);
    let mountains = smoothstep(-0.05, 0.35, ranges);
    let ridge = 1.0 - noise2(12, q / 1300.0).abs();
    let hills = layered2(1, q / 180.0, 5);
    let h = 34.0 + 46.0 * ranges + 40.0 * mountains * ridge * ridge + 12.0 * hills;
    h.clamp(4.0, 118.0)
}

fn smoothstep(lo: f32, hi: f32, x: f32) -> f32 {
    let t = ((x - lo) / (hi - lo)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The ground kind on top of the ground at height `h` with `slope`, or
/// `depth` metres under it.
fn kind(h: f32, slope: f32, depth: f32, cave_wall: bool) -> u8 {
    if cave_wall || slope > 0.8 || depth > 4.0 {
        ROCK
    } else if h > 96.0 && depth < 2.0 {
        SNOW
    } else if depth > 1.5 {
        DIRT
    } else if h < 18.0 {
        SAND
    } else {
        GRASS
    }
}

/// One column's surface: its height and slope.
#[derive(Debug, Clone, Copy)]
struct Column {
    height: f32,
    slope: f32,
}

fn column(q: Vec2, step: f32) -> Column {
    let h = height(q);
    let dx = height(q + Vec2::X * step) - height(q - Vec2::X * step);
    let dy = height(q + Vec2::Y * step) - height(q - Vec2::Y * step);
    Column {
        height: h,
        slope: 0.5 * Vec2::new(dx, dy).length() / step,
    }
}

/// The ground's distance from the surface at `p` (metres, negative
/// inside), and its kind, seen at voxels `voxel` metres wide: caves
/// narrower than about a voxel close up, as averaging would close them.
fn sample(p: Vec3, c: Column, voxel: f32) -> (f32, u8) {
    let ground = (p.z - c.height) / (1.0 + c.slope * c.slope).sqrt();
    let depth = c.height - p.z;
    let mut d = ground;
    let mut wall = false;
    if depth > CAVE_TOP && depth < CAVE_BOTTOM {
        let q = p / 28.0;
        let tunnel = (noise3(2, q).abs() + noise3(3, q).abs() - 0.07) * 10.0;
        d = d.max(-(tunnel + 0.75 * (voxel - 1.0)));
        wall = tunnel < 1.5;
    }
    // Seen from afar, a voxel shows the kind of the surface it holds.
    let seen_depth = (depth - (voxel - 1.0)).max(0.0);
    (d, kind(c.height, c.slope, seen_depth, wall))
}

/// The world's distance and kind at `p` (metres) at level 0.
pub fn at(p: Vec3) -> (f32, u8) {
    sample(p, column(p.truncate(), 1.0), 1.0)
}

/// Fills the chunk `key` of `level` (in that level's chunks) straight
/// from the formulas: each sample is the world at its own place.
pub fn fill(level: u32, key: IVec3, dense: &mut Dense) {
    let step = lod::voxel(level);
    let origin = key * SIZE_I;
    let place = |i: i32| i as f32 * step;
    let columns: Vec<Column> = (0..SIZE * SIZE)
        .map(|i| {
            let x = origin.x + (i % SIZE) as i32;
            let y = origin.y + (i / SIZE) as i32;
            column(Vec2::new(place(x), place(y)), step)
        })
        .collect();
    let (lowest, highest) = columns.iter().fold((f32::MAX, f32::MIN), |(lo, hi), c| {
        (lo.min(c.height), hi.max(c.height))
    });
    let bottom = place(origin.z);
    let top = place(origin.z + SIZE_I - 1);
    let margin = 3.0 * step;
    if bottom > highest + margin {
        dense.voxels_mut().fill(Voxel::AIR);
        return;
    }
    if top < lowest - CAVE_BOTTOM - margin {
        dense.voxels_mut().fill(Voxel::solid(ROCK));
        return;
    }
    for z in 0..SIZE {
        let pz = place(origin.z + z as i32);
        for (i, c) in columns.iter().enumerate() {
            let p = Vec3::new(
                place(origin.x + (i % SIZE) as i32),
                place(origin.y + (i / SIZE) as i32),
                pz,
            );
            let (d, k) = sample(p, *c, step);
            let at = index(UVec3::new((i % SIZE) as u32, (i / SIZE) as u32, z as u32));
            dense.voxels_mut()[at] = Voxel::at(d / step, k);
        }
    }
}

/// How many chunks of `level` stand on each other to hold the world's
/// height.
pub fn chunks_high(level: u32) -> u32 {
    let chunk = lod::chunk_metres(level);
    (HEIGHT + chunk - 1).div_euclid(chunk).max(1) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heights_stay_inside_the_world_and_vary() {
        let mut lo = f32::MAX;
        let mut hi = f32::MIN;
        for i in 0..400 {
            let q = Vec2::new((i * 97 % SIDE) as f32, (i * 389 % SIDE) as f32);
            let h = height(q);
            lo = lo.min(h);
            hi = hi.max(h);
        }
        assert!(lo >= 4.0 && hi <= 118.0, "{lo} to {hi}");
        assert!(hi - lo > 40.0, "mountains and valleys: {lo} to {hi}");
    }

    #[test]
    fn a_coarse_sample_is_the_world_at_its_place() {
        let mut fine = Dense::filled(Voxel::AIR);
        let mut coarse = Dense::filled(Voxel::AIR);
        fill(0, IVec3::new(4, 4, 1), &mut fine);
        fill(1, IVec3::new(2, 2, 0), &mut coarse);
        // Level 1's sample (0, 0, 16) is level 0's (128, 128, 32): the
        // first of both chunks, measured in each one's own voxels.
        let f = fine.get(UVec3::ZERO);
        let c = coarse.get(UVec3::new(0, 0, 16));
        assert_eq!(f.is_solid(), c.is_solid());
    }

    #[test]
    fn the_world_is_air_above_and_rock_below() {
        let mut dense = Dense::filled(Voxel::AIR);
        fill(0, IVec3::new(0, 0, 3), &mut dense);
        assert_eq!(dense.uniform(), Some(Voxel::AIR));
        assert_eq!(chunks_high(0), 4);
        assert_eq!(chunks_high(3), 1);
    }
}
