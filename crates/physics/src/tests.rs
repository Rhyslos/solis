//! Physics: bodies fall and stack, the same every run, and nothing that
//! isn't a number gets in.

use super::*;

fn id(n: u64) -> EntityId {
    EntityId(reflect::Id::from_raw(n).unwrap())
}

/// A floor (a fixed box 20 m wide, its top at 0) as entity 1.
fn with_floor() -> Physics {
    let mut physics = Physics::new();
    assert!(physics.add(
        id(1),
        (Vec3::new(0.0, 0.0, -0.5), Quat::IDENTITY),
        Vec3::ONE,
        None,
        Some(ColliderSpec {
            shape: Shape::Box(Vec3::new(20.0, 20.0, 1.0)),
            ..ColliderSpec::default()
        }),
    ));
    physics
}

fn crate_spec() -> (Option<Body>, Option<ColliderSpec>) {
    (Some(Body::default()), Some(ColliderSpec::default()))
}

fn run(physics: &mut Physics, ticks: usize) {
    for _ in 0..ticks {
        physics.step(1.0 / 60.0);
    }
}

#[test]
fn a_crate_falls_and_lands_on_the_floor() {
    let mut physics = with_floor();
    let (body, collider) = crate_spec();
    physics.add(
        id(2),
        (Vec3::new(0.0, 0.0, 5.0), Quat::IDENTITY),
        Vec3::ONE,
        body,
        collider,
    );
    run(&mut physics, 30);
    let falling = physics.state().pose(id(2)).unwrap().0.z;
    assert!(falling < 5.0, "falling: {falling}");
    assert!(physics.state().velocity(id(2)).unwrap().z < 0.0);
    run(&mut physics, 300);
    let (rest, _) = physics.state().pose(id(2)).unwrap();
    // A 1 m crate on a floor whose top is at 0 rests with its middle at 0.5.
    assert!((rest.z - 0.5).abs() < 0.05, "resting at {rest}");
    // The floor never moved.
    assert_eq!(
        physics.state().pose(id(1)).unwrap().0,
        Vec3::new(0.0, 0.0, -0.5)
    );
}

/// A pile of `count` crates dropped in a column, run for `ticks`: the
/// checksum after each tick.
fn pile(count: u64, ticks: usize) -> Vec<u64> {
    let mut physics = with_floor();
    for n in 0..count {
        let (body, collider) = crate_spec();
        let x = (n % 3) as f32 * 0.3;
        physics.add(
            id(10 + n),
            (
                Vec3::new(x, 0.1 * n as f32, 1.0 + 1.2 * n as f32),
                Quat::from_rotation_z(0.3 * n as f32),
            ),
            Vec3::ONE,
            body,
            collider,
        );
    }
    (0..ticks)
        .map(|_| {
            physics.step(1.0 / 60.0);
            physics.state().checksum()
        })
        .collect()
}

#[test]
fn a_pile_falls_the_same_every_time() {
    let first = pile(12, 240);
    assert_eq!(pile(12, 240), first, "the same, tick by tick");
    assert_ne!(first[0], first[1], "it moves");
}

#[test]
fn rays_hit_the_nearest_thing_and_pass_out_of_what_they_start_in() {
    let mut physics = with_floor();
    let (body, collider) = crate_spec();
    physics.add(
        id(2),
        (Vec3::new(0.0, 0.0, 3.0), Quat::IDENTITY),
        Vec3::ONE,
        body,
        collider,
    );
    // Collisions are found by stepping once.
    physics.step(1.0 / 60.0);
    let down = physics
        .state()
        .cast_ray(Vec3::new(0.0, 0.0, 10.0), -Vec3::Z, 100.0)
        .unwrap();
    assert_eq!(down.entity, id(2), "the crate before the floor");
    assert!((down.distance - 6.5).abs() < 0.05, "{down:?}");
    assert!(down.normal.z > 0.99);
    // From inside the crate, it's passed through: the floor.
    let inside = physics
        .state()
        .cast_ray(Vec3::new(0.0, 0.0, 3.0), -Vec3::Z, 100.0)
        .unwrap();
    assert_eq!(inside.entity, id(1));
    // Out of reach, or nonsense: nothing.
    assert!(
        physics
            .state()
            .cast_ray(Vec3::new(0.0, 0.0, 10.0), -Vec3::Z, 1.0)
            .is_none()
    );
    for (from, direction, reach) in [
        (Vec3::NAN, -Vec3::Z, 10.0),
        (Vec3::ZERO, Vec3::ZERO, 10.0),
        (Vec3::ZERO, -Vec3::Z, f32::NAN),
        (Vec3::ZERO, -Vec3::Z, -1.0),
    ] {
        assert!(physics.state().cast_ray(from, direction, reach).is_none());
    }
}

#[test]
fn pushes_velocities_and_kinematic_moves() {
    let mut physics = Physics::new();
    let (body, collider) = crate_spec();
    // No gravity: it floats where it's pushed.
    let floating = Body {
        gravity: 0.0,
        ..body.unwrap()
    };
    physics.add(
        id(2),
        (Vec3::ZERO, Quat::IDENTITY),
        Vec3::ONE,
        Some(floating),
        collider,
    );
    physics.set_velocity(id(2), Vec3::new(2.0, 0.0, 0.0));
    run(&mut physics, 60);
    let x = physics.state().pose(id(2)).unwrap().0.x;
    assert!((x - 2.0).abs() < 0.05, "two metres in a second: {x}");
    let before = physics.state().velocity(id(2)).unwrap().y;
    physics.push(id(2), Vec3::new(0.0, 5.0, 0.0));
    let after = physics.state().velocity(id(2)).unwrap().y;
    assert!(after > before + 0.1, "pushed: {before} → {after}");
    // A kinematic body goes where it's put, over a step.
    physics.add(
        id(3),
        (Vec3::new(0.0, 10.0, 0.0), Quat::IDENTITY),
        Vec3::ONE,
        Some(Body {
            kind: BodyKind::Kinematic,
            ..Body::default()
        }),
        Some(ColliderSpec::default()),
    );
    physics.set_pose(id(3), Vec3::new(1.0, 10.0, 0.0), Quat::IDENTITY);
    physics.step(1.0 / 60.0);
    assert!((physics.state().pose(id(3)).unwrap().0.x - 1.0).abs() < 1e-4);
    // Moved bodies are the dynamic ones, awake.
    let moved: Vec<EntityId> = physics.moved().into_iter().map(|(e, _, _)| e).collect();
    assert_eq!(moved, [id(2)]);
}

#[test]
fn nothing_that_isnt_a_number_gets_in() {
    let mut physics = Physics::new();
    let (body, collider) = crate_spec();
    assert!(!physics.add(
        id(2),
        (Vec3::NAN, Quat::IDENTITY),
        Vec3::ONE,
        body,
        collider
    ));
    assert!(!physics.add(
        id(2),
        (Vec3::ZERO, Quat::from_xyzw(0.0, 0.0, 0.0, 0.0)),
        Vec3::ONE,
        body,
        collider
    ));
    assert!(!physics.add(
        id(2),
        (Vec3::ZERO, Quat::IDENTITY),
        Vec3::INFINITY,
        body,
        collider
    ));
    // Wild settings are made sensible, not refused.
    let wild = Body {
        mass: f32::NAN,
        linear_damping: f32::INFINITY,
        gravity: f32::NAN,
        ..Body::default()
    };
    let tiny = ColliderSpec {
        shape: Shape::Capsule {
            radius: f32::NAN,
            height: -3.0,
        },
        friction: f32::NAN,
        bounciness: 7.0,
        center: Vec3::NAN,
    };
    assert!(physics.add(
        id(3),
        (Vec3::ZERO, Quat::IDENTITY),
        Vec3::ONE,
        Some(wild),
        Some(tiny)
    ));
    physics.set_pose(id(3), Vec3::NAN, Quat::IDENTITY);
    physics.set_velocity(id(3), Vec3::INFINITY);
    physics.push(id(3), Vec3::NAN);
    run(&mut physics, 10);
    let (p, r) = physics.state().pose(id(3)).unwrap();
    assert!(p.is_finite() && r.is_finite());
    physics.step(f32::NAN);
    physics.step(-1.0);
    // Taking out what isn't there is nothing.
    physics.remove(id(99));
    physics.remove(id(3));
    assert!(physics.state().is_empty());
}

#[test]
fn past_the_most_bodies_the_rest_are_counted_not_made() {
    let mut physics = Physics::new();
    let (body, collider) = crate_spec();
    for n in 0..MAX_BODIES as u64 + 3 {
        physics.add(
            id(n + 1),
            (Vec3::new(n as f32 * 2.0, 0.0, 0.0), Quat::IDENTITY),
            Vec3::ONE,
            body,
            collider,
        );
    }
    assert_eq!(physics.state().len(), MAX_BODIES);
    assert_eq!(physics.refused(), 3);
}

#[test]
fn a_snapshot_carries_on_exactly_as_physics_would_have() {
    let mut physics = with_floor();
    for n in 0..6 {
        let (body, collider) = crate_spec();
        physics.add(
            id(10 + n),
            (
                Vec3::new(0.2 * n as f32, 0.0, 1.0 + 1.2 * n as f32),
                Quat::from_rotation_z(0.4 * n as f32),
            ),
            Vec3::ONE,
            body,
            collider,
        );
    }
    run(&mut physics, 40);
    let saved = physics.snapshot();
    run(&mut physics, 60);
    let carried_on = physics.state().checksum();
    let mut restored = Physics::new();
    restored.restore(&saved).unwrap();
    run(&mut restored, 60);
    assert_eq!(restored.state().checksum(), carried_on, "to the bit");
    // Damaged bytes are refused, never a crash, and leave it as it was.
    let before = restored.state().checksum();
    assert!(restored.restore(&saved[..saved.len() / 2]).is_err());
    let mut seed = 0x2545_f491_u64;
    for _ in 0..300 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let mut bytes = saved.clone();
        let at = (seed as usize) % bytes.len();
        bytes[at] ^= (seed >> 40) as u8 | 1;
        let _ = Physics::new().restore(&bytes);
    }
    assert_eq!(restored.state().checksum(), before);
}
