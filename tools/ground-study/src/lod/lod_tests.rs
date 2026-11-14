use super::*;
use crate::brush::Action;
use crate::shapes::{ball, fill};
use glam::Vec3;

/// A lumpy ball in a 256 m box, 128 m high (8 × 8 × 4 chunks), across
/// the borders of the levels in the tests' layouts.
pub(crate) fn ball_pyramid(levels: u32) -> Pyramid {
    let shape = ball(Vec3::new(70.0, 70.0, 64.0), 48.0);
    let base = Ground::build(UVec3::new(8, 8, 4), |key, dense| fill(key, dense, &shape));
    Pyramid::build(base, levels)
}

#[test]
fn averaging_keeps_the_same_all_through_the_same() {
    for v in [Voxel::AIR, Voxel::solid(3)] {
        assert_eq!(average(|_| v, IVec3::new(5, 7, 1)), v);
    }
    // A flat surface stays where it was, measured in the coarser voxels.
    let flat = |p: IVec3| Voxel::at(p.z as f32 - 10.5, 2);
    let v = average(flat, IVec3::new(3, 3, 5));
    assert!(
        (v.distance() - (10.0 - 10.5) / 2.0).abs() < 1.0 / 64.0,
        "{v:?}"
    );
    assert_eq!(v.material, 2);
}

#[test]
fn each_level_is_half_as_wide_and_keeps_the_ball() {
    let p = ball_pyramid(3);
    assert_eq!(p.size(0), IVec3::new(256, 256, 128));
    assert_eq!(p.size(1), IVec3::new(128, 128, 64));
    assert_eq!(p.size(2), IVec3::new(64, 64, 32));
    assert!(
        p.voxel(2, IVec3::new(17, 17, 16)).is_solid(),
        "the middle is ground"
    );
    assert!(
        !p.voxel(2, IVec3::new(60, 60, 30)).is_solid(),
        "a corner is air"
    );
}

#[test]
fn sculpting_averages_the_levels_above_as_building_them_again_would() {
    let mut p = ball_pyramid(3);
    let brush = Brush {
        center: Vec3::new(70.0, 70.0, 108.0),
        radius: 6.0,
        strength: 1.0,
        action: Action::Add(1),
    };
    let changed = p.apply(&brush);
    assert!(!changed[0].is_empty() && !changed[1].is_empty());
    let Some(base) = p.level(0) else {
        panic!("no level 0")
    };
    let again = Pyramid::build(
        Ground::build(UVec3::new(8, 8, 4), |key, dense| {
            if let Some(c) = base.chunk(key) {
                c.decode(dense);
            }
        }),
        3,
    );
    for level in 1..3 {
        let size = p.size(level);
        for g in crate::ground::boxed(IVec3::ZERO, size) {
            assert_eq!(
                p.voxel(level, g),
                again.voxel(level, g),
                "level {level} at {g}"
            );
        }
    }
}

#[test]
fn kept_chunks_read_as_the_pyramid_and_others_as_air() {
    let p = ball_pyramid(2);
    let mut kept = Kept::new(vec![p.size(0), p.size(1)]);
    let Some(ground) = p.level(1) else {
        panic!("no level 1")
    };
    for key in ground.keys() {
        if let Some(chunk) = ground.chunk(key) {
            kept.insert((1, key), chunk.clone());
        }
    }
    for g in [
        IVec3::splat(32),
        IVec3::new(0, 63, 5),
        IVec3::new(-4, 170, 31),
    ] {
        assert_eq!(kept.voxel(1, g), p.voxel(1, g));
    }
    assert_eq!(kept.voxel(0, IVec3::splat(64)), Voxel::AIR);
    assert!(kept.bytes() > 0 && kept.len() == 32);
}
