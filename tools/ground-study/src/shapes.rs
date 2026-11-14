//! The grounds the study measures and draws, made from distance
//! functions, so every run gets the same voxels.

use crate::noise::{layered2, noise2, noise3};
use crate::voxel::{Dense, SIZE_I, Voxel, place};
use glam::{IVec3, Vec2, Vec3};

/// The palette's materials, as the approved mock-up names them.
pub const GRASS: u8 = 0;
/// Rock.
pub const ROCK: u8 = 1;
/// Sand.
pub const SAND: u8 = 2;
/// Dirt.
pub const DIRT: u8 = 3;

/// Fills the chunk `key` from `voxel` (given each voxel's world place).
pub fn fill(key: IVec3, dense: &mut Dense, voxel: impl Fn(Vec3) -> Voxel) {
    let origin = key * SIZE_I;
    for (i, v) in dense.voxels_mut().iter_mut().enumerate() {
        *v = voxel((origin + place(i).as_ivec3()).as_vec3());
    }
}

/// How far `p` is above a height field `height` (negative below),
/// corrected for its slope so it reads as a distance.
fn above(p: Vec3, height: impl Fn(Vec2) -> f32) -> (f32, f32, f32) {
    let flat = p.truncate();
    let h = height(flat);
    let dx = height(flat + Vec2::X) - height(flat - Vec2::X);
    let dy = height(flat + Vec2::Y) - height(flat - Vec2::Y);
    let slope = 0.5 * Vec2::new(dx, dy).length();
    ((p.z - h) / (1.0 + slope * slope).sqrt(), h, slope)
}

fn capsule(p: Vec3, a: Vec3, b: Vec3, radius: f32) -> f32 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0);
    (p - (a + ab * t)).length() - radius
}

fn ellipsoid(p: Vec3, middle: Vec3, radii: Vec3) -> f32 {
    ((p - middle) / radii)
        .length()
        .mul_add(radii.min_element(), -radii.min_element())
}

fn hill_height(q: Vec2) -> f32 {
    let r2 = (q - Vec2::new(56.0, 54.0)).length_squared();
    12.0 + 22.0 * (-r2 / (2.0 * 15.0 * 15.0)).exp() + 1.2 * noise2(5, q / 14.0)
}

/// The report's picture: a hill on a meadow (96 by 96 voxels), a cave
/// dug into its south side, a rock shelf overhanging its west side, a
/// dirt path to the cave and sand to the east.
pub fn hill_with_cave(p: Vec3) -> Voxel {
    let (ground, h, slope) = above(p, hill_height);
    let shelf = ellipsoid(p, Vec3::new(39.0, 58.0, 25.0), Vec3::new(9.0, 7.0, 2.2));
    let cave = capsule(
        p,
        Vec3::new(56.0, 30.0, 13.5),
        Vec3::new(56.0, 50.0, 16.0),
        4.2,
    )
    .min((p - Vec3::new(56.0, 53.0, 17.0)).length() - 6.5);
    let d = ground.min(shelf).max(-cave);
    let depth = h - p.z;
    let path = segment_distance(p.truncate(), Vec2::new(14.0, 6.0), Vec2::new(56.0, 32.0));
    let material = if shelf < 0.5 || slope > 0.9 || depth > 2.5 || cave < 1.5 {
        ROCK
    } else if path < 2.5 {
        DIRT
    } else if (p.truncate() - Vec2::new(82.0, 22.0)).length() < 10.0 {
        SAND
    } else {
        GRASS
    };
    Voxel::at(d, material)
}

fn segment_distance(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0);
    (p - (a + ab * t)).length()
}

fn rolling_height(q: Vec2) -> f32 {
    44.0 + 56.0 * layered2(1, q / 180.0, 5)
}

/// The scale case's ground: rolling hills and valleys (heights about 16
/// to 72 voxels), with winding caves under them.
pub fn rolling_chunk(key: IVec3, dense: &mut Dense) {
    let origin = key * SIZE_I;
    let (mut lowest, mut highest) = (f32::MAX, f32::MIN);
    for y in -1..=SIZE_I {
        for x in -1..=SIZE_I {
            let h = rolling_height((origin.truncate() + glam::IVec2::new(x, y)).as_vec2());
            lowest = lowest.min(h);
            highest = highest.max(h);
        }
    }
    let (bottom, top) = (origin.z as f32, (origin.z + SIZE_I) as f32);
    if bottom > highest + 3.0 {
        dense.voxels_mut().fill(Voxel::AIR);
    } else if top < lowest - CAVE_DEPTH - 3.0 {
        dense.voxels_mut().fill(Voxel::solid(ROCK));
    } else {
        fill(key, dense, rolling);
    }
}

const CAVE_DEPTH: f32 = 28.0;

fn rolling(p: Vec3) -> Voxel {
    let (ground, h, slope) = above(p, rolling_height);
    let depth = h - p.z;
    let mut d = ground;
    let mut in_cave_wall = false;
    if depth > 1.0 && depth < CAVE_DEPTH {
        let q = p / 28.0;
        let tunnel = (noise3(2, q).abs() + noise3(3, q).abs() - 0.07) * 10.0;
        d = d.max(-tunnel);
        in_cave_wall = tunnel < 1.5;
    }
    let material = if in_cave_wall || slope > 0.8 || depth > 4.0 {
        ROCK
    } else if depth > 1.5 {
        DIRT
    } else if h < 24.0 {
        SAND
    } else {
        GRASS
    };
    Voxel::at(d, material)
}

/// The worst case for every look: a chunk full of small lumps and holes
/// (about 3.5 voxels across), every voxel a material picked at random.
pub fn busy(p: Vec3) -> Voxel {
    let d = noise3(9, p / 3.5) * 3.5;
    let pick = (p.x as i32).wrapping_mul(73_856_093)
        ^ (p.y as i32).wrapping_mul(19_349_663)
        ^ (p.z as i32).wrapping_mul(83_492_791);
    Voxel::at(d, (pick.rem_euclid(4)) as u8)
}

/// A lumpy ball, closed all round, crossing chunk borders (for tests).
pub fn ball(middle: Vec3, radius: f32) -> impl Fn(Vec3) -> Voxel + Sync {
    move |p| {
        let lumps = 2.0 * noise3(4, p / 6.0);
        Voxel::at(
            (p - middle).length() - radius + lumps,
            (p.z as i32).rem_euclid(3) as u8,
        )
    }
}
