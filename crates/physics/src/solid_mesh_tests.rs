//! Solid meshes: characters stand and walk on them, bodies rest on them,
//! and a mesh that can't be used collides with nothing, never a crash.

use super::*;
use crate::{Body, BodyKind, CharacterSpec, ColliderSpec, GRAVITY, Physics, Shape};
use glam::Quat;
use reflect::EntityId;
use std::sync::Arc;

fn id(n: u64) -> EntityId {
    EntityId(reflect::Id::from_raw(n).unwrap())
}

/// A box from -half to +half across and deep, 0 to `height` up, as twelve
/// triangles facing out.
fn block(half: f32, height: f32) -> SolidMesh {
    let points = vec![
        Vec3::new(-half, -half, 0.0),
        Vec3::new(half, -half, 0.0),
        Vec3::new(half, half, 0.0),
        Vec3::new(-half, half, 0.0),
        Vec3::new(-half, -half, height),
        Vec3::new(half, -half, height),
        Vec3::new(half, half, height),
        Vec3::new(-half, half, height),
    ];
    let quads = [
        [0, 3, 2, 1], // bottom
        [4, 5, 6, 7], // top
        [0, 1, 5, 4],
        [1, 2, 6, 5],
        [2, 3, 7, 6],
        [3, 0, 4, 7],
    ];
    let triangles = quads
        .iter()
        .flat_map(|[a, b, c, d]| [[*a, *b, *c], [*a, *c, *d]])
        .collect();
    SolidMesh { points, triangles }
}

fn add_mesh(physics: &mut Physics, n: u64, at: Vec3, scale: Vec3, mesh: SolidMesh) -> bool {
    physics.add(
        id(n),
        (at, Quat::IDENTITY),
        scale,
        None,
        Some(ColliderSpec {
            shape: Shape::Mesh(Arc::new(mesh)),
            ..ColliderSpec::default()
        }),
    )
}

fn walk(physics: &mut Physics, speed: Vec3, ticks: usize) {
    let dt = 1.0 / 60.0;
    let mut fall = 0.0f32;
    for _ in 0..ticks {
        let grounded = physics.state().character(id(9)).unwrap().grounded;
        fall = if grounded { 0.0 } else { fall + GRAVITY * dt };
        physics.move_character(id(9), (speed + Vec3::Z * fall) * dt, dt);
        physics.step(dt);
        let _ = physics.moved();
    }
}

fn feet(physics: &Physics) -> Vec3 {
    physics.state().pose(id(9)).unwrap().0
}

#[test]
fn a_character_lands_on_a_mesh_floor_and_walks_across_its_seams() {
    let mut physics = Physics::new();
    // A floor 20 m across, its top at 0, its top cut into two triangles.
    assert!(add_mesh(
        &mut physics,
        1,
        Vec3::new(0.0, 0.0, -1.0),
        Vec3::ONE,
        block(10.0, 1.0)
    ));
    assert!(physics.add_character(
        id(9),
        (Vec3::new(0.0, 0.0, 2.0), Quat::IDENTITY),
        CharacterSpec::default()
    ));
    walk(&mut physics, Vec3::ZERO, 90);
    assert!(
        feet(&physics).z.abs() < 0.05,
        "landed at {}",
        feet(&physics)
    );
    assert!(physics.state().character(id(9)).unwrap().grounded);
    // Across the floor's diagonal seam and on.
    walk(&mut physics, Vec3::new(3.0, 3.0, 0.0), 120);
    let walked = feet(&physics);
    assert!(
        walked.x > 5.0 && walked.z.abs() < 0.05,
        "walked to {walked}"
    );
}

#[test]
fn a_character_bumps_into_a_mesh_wall() {
    let mut physics = Physics::new();
    assert!(add_mesh(
        &mut physics,
        1,
        Vec3::new(0.0, 0.0, -1.0),
        Vec3::ONE,
        block(10.0, 1.0)
    ));
    // A block 2 m tall, its near face at x = 3.
    assert!(add_mesh(
        &mut physics,
        2,
        Vec3::new(4.0, 0.0, 0.0),
        Vec3::ONE,
        block(1.0, 2.0)
    ));
    assert!(physics.add_character(
        id(9),
        (Vec3::new(0.0, 0.0, 0.0), Quat::IDENTITY),
        CharacterSpec::default()
    ));
    walk(&mut physics, Vec3::new(3.0, 0.0, 0.0), 120);
    let stopped = feet(&physics);
    assert!(stopped.x < 3.0 && stopped.x > 2.0, "stopped at {stopped}");
}

#[test]
fn a_crate_rests_on_a_mesh_scaled_and_mirrored() {
    for scale in [Vec3::new(2.0, 2.0, 1.0), Vec3::new(-2.0, 2.0, 1.0)] {
        let mut physics = Physics::new();
        assert!(add_mesh(
            &mut physics,
            1,
            Vec3::new(0.0, 0.0, -1.0),
            scale,
            block(5.0, 1.0)
        ));
        physics.add(
            id(2),
            (Vec3::new(0.0, 0.0, 3.0), Quat::IDENTITY),
            Vec3::ONE,
            Some(Body::default()),
            Some(ColliderSpec::default()),
        );
        for _ in 0..240 {
            physics.step(1.0 / 60.0);
        }
        let rest = physics.state().pose(id(2)).unwrap().0;
        assert!((rest.z - 0.5).abs() < 0.05, "{scale}: resting at {rest}");
    }
}

#[test]
fn meshes_that_cant_be_used_collide_with_nothing() {
    let nan = SolidMesh {
        points: vec![Vec3::NAN; 3],
        triangles: vec![[0, 1, 2]],
    };
    let outside = SolidMesh {
        points: vec![Vec3::ZERO; 3],
        triangles: vec![[0, 1, 7]],
    };
    let flat = SolidMesh {
        points: vec![Vec3::ZERO; 3],
        triangles: vec![[0, 1, 2]],
    };
    let too_many = SolidMesh {
        points: vec![Vec3::ZERO, Vec3::X, Vec3::Y],
        triangles: vec![[0, 1, 2]; MAX_TRIANGLES + 1],
    };
    for (n, mesh) in [SolidMesh::default(), nan, outside, flat, too_many]
        .into_iter()
        .enumerate()
    {
        let mut physics = Physics::new();
        // The body is made (still), whatever its collider.
        assert!(add_mesh(
            &mut physics,
            n as u64 + 1,
            Vec3::ZERO,
            Vec3::ONE,
            mesh
        ));
        physics.add(
            id(50),
            (Vec3::new(0.0, 0.0, 1.0), Quat::IDENTITY),
            Vec3::ONE,
            Some(Body::default()),
            Some(ColliderSpec::default()),
        );
        for _ in 0..30 {
            physics.step(1.0 / 60.0);
        }
        assert!(!physics.take_broke());
    }
    // Huge scales are held in range.
    let mut physics = Physics::new();
    assert!(add_mesh(
        &mut physics,
        1,
        Vec3::ZERO,
        Vec3::splat(1e30),
        block(1.0, 1.0)
    ));
    physics.step(1.0 / 60.0);
    assert!(!physics.take_broke());
}

#[test]
fn a_mesh_body_is_always_still() {
    let mut physics = Physics::new();
    physics.add(
        id(1),
        (Vec3::new(0.0, 0.0, 5.0), Quat::IDENTITY),
        Vec3::ONE,
        Some(Body {
            kind: BodyKind::Fixed,
            ..Body::default()
        }),
        Some(ColliderSpec {
            shape: Shape::Mesh(Arc::new(block(1.0, 1.0))),
            ..ColliderSpec::default()
        }),
    );
    for _ in 0..30 {
        physics.step(1.0 / 60.0);
    }
    assert_eq!(
        physics.state().pose(id(1)).unwrap().0,
        Vec3::new(0.0, 0.0, 5.0)
    );
}
