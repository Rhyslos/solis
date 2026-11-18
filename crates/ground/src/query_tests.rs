use super::*;
use crate::start::Start;
use glam::UVec3;

fn flat() -> Ground {
    Start::Flat { height: 40.25 }.ground(UVec3::new(2, 2, 2))
}

#[test]
fn the_height_of_a_flat_ground_is_its_height() {
    let g = flat();
    for (x, y) in [(0.0, 0.0), (10.5, 3.25), (63.0, 63.0), (31.9, 32.1)] {
        let h = g.height_at(x, y).known().flatten().unwrap();
        assert!((h - 40.25).abs() < 0.02, "{h} at {x}, {y}");
    }
    assert_eq!(g.height_at(-1.0, 5.0), Answer::Known(None));
    assert_eq!(g.height_at(5.0, 65.0), Answer::Known(None));
    assert_eq!(g.height_at(f32::NAN, 5.0), Answer::Known(None));
}

#[test]
fn the_kind_is_known_inside_and_none_in_the_air() {
    let g = flat();
    assert_eq!(g.kind_at(Vec3::new(5.0, 5.0, 30.0)), Answer::Known(Some(0)));
    assert_eq!(g.kind_at(Vec3::new(5.0, 5.0, 50.0)), Answer::Known(None));
    assert_eq!(g.kind_at(Vec3::new(500.0, 5.0, 5.0)), Answer::Known(None));
}

#[test]
fn a_ray_down_meets_the_surface_facing_up() {
    let g = flat();
    let hit = g
        .ray(Vec3::new(20.0, 20.0, 60.0), Vec3::NEG_Z, 100.0)
        .known()
        .flatten()
        .unwrap();
    assert!((hit.place.z - 40.25).abs() < 0.01, "{hit:?}");
    assert!((hit.distance - 19.75).abs() < 0.01);
    assert!(hit.normal.dot(Vec3::Z) > 0.99);
    // Upwards, sideways in the air, too short, and from outside the box.
    assert_eq!(
        g.ray(Vec3::new(20.0, 20.0, 60.0), Vec3::Z, 100.0),
        Answer::Known(None)
    );
    assert_eq!(
        g.ray(Vec3::new(20.0, 20.0, 60.0), Vec3::X, 100.0),
        Answer::Known(None)
    );
    assert_eq!(
        g.ray(Vec3::new(20.0, 20.0, 60.0), Vec3::NEG_Z, 5.0),
        Answer::Known(None)
    );
    let slanted = g
        .ray(
            Vec3::new(-10.0, 30.0, 60.0),
            Vec3::new(1.0, 0.0, -1.0),
            200.0,
        )
        .known()
        .flatten()
        .unwrap();
    assert!((slanted.place.z - 40.25).abs() < 0.01);
    assert_eq!(g.ray(Vec3::ZERO, Vec3::ZERO, 10.0), Answer::Known(None));
}

#[test]
fn questions_about_an_absent_chunk_are_unknown_never_air() {
    let mut g = flat();
    g.store_mut().unload(IVec3::new(0, 0, 1));
    assert_eq!(g.voxel(IVec3::new(3, 3, 40)), Answer::Unknown);
    assert_eq!(g.height_at(3.0, 3.0), Answer::Unknown);
    assert_eq!(g.kind_at(Vec3::new(3.0, 3.0, 40.0)), Answer::Unknown);
    assert_eq!(
        g.ray(Vec3::new(3.0, 3.0, 60.0), Vec3::NEG_Z, 100.0),
        Answer::Unknown
    );
    // Elsewhere is still known.
    assert!(g.height_at(40.0, 40.0).known().flatten().is_some());
    assert!(
        g.ray(Vec3::new(40.0, 40.0, 60.0), Vec3::NEG_Z, 100.0)
            .known()
            .is_some()
    );
}
