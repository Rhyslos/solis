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
    // Where it hit (mutation testing: never checked), and a direction of
    // any length is the same way.
    assert!(
        down.point.abs_diff_eq(Vec3::new(0.0, 0.0, 3.5), 0.05),
        "{down:?}"
    );
    let long = physics
        .state()
        .cast_ray(Vec3::new(0.0, 0.0, 10.0), Vec3::new(0.0, 0.0, -7.0), 100.0)
        .unwrap();
    assert_eq!(
        (long.entity, long.distance, long.point),
        (down.entity, down.distance, down.point)
    );
    // Sideways from off to one side: the crate's side, facing it.
    let side = physics
        .state()
        .cast_ray(Vec3::new(-5.0, 0.0, 3.0), Vec3::X, 100.0)
        .unwrap();
    assert_eq!(side.entity, id(2));
    assert!(
        side.point.abs_diff_eq(Vec3::new(-0.5, 0.0, 3.0), 0.05),
        "{side:?}"
    );
    assert!(side.normal.x < -0.99, "{side:?}");
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
    // The third security audit: huge but finite speeds and pushes are
    // kept to what a save keeps, so the game can still be saved.
    physics.set_velocity(id(3), Vec3::splat(3.0e38));
    assert!(physics.state().body_state(id(3)).unwrap().is_sensible());
    physics.set_velocity(id(3), Vec3::ZERO);
    physics.push(id(3), Vec3::new(-3.0e38, 0.0, 0.0));
    assert!(physics.state().body_state(id(3)).unwrap().is_sensible());
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

/// Builds physics again from `states` as a save game or an edit does:
/// empty, the same bodies added in entity order by `add_all`, then each
/// given its state.
fn rebuilt(
    physics: &mut Physics,
    add_all: &dyn Fn(&mut Physics),
    states: &[(EntityId, BodyState)],
) {
    physics.clear();
    add_all(physics);
    for (id, state) in states {
        physics.set_body_state(*id, *state, true);
    }
}

fn six_crates(physics: &mut Physics) {
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
}

#[test]
fn built_again_from_body_states_it_carries_on_the_same_both_ways() {
    let mut physics = Physics::new();
    six_crates(&mut physics);
    run(&mut physics, 40);
    // Saving builds the running physics again from its states; loading
    // builds a new one from the same states: the two carry on alike.
    let states = physics.state().body_states();
    assert_eq!(states.len(), 7);
    let spinning = states.iter().any(|(_, s)| s.spin.length() > 1e-3);
    assert!(spinning, "the falling crates spin");
    rebuilt(&mut physics, &six_crates, &states);
    assert_eq!(physics.state().body_states(), states, "kept exactly");
    let mut loaded = Physics::new();
    rebuilt(&mut loaded, &six_crates, &states);
    run(&mut physics, 120);
    run(&mut loaded, 120);
    assert_eq!(
        loaded.state().checksum(),
        physics.state().checksum(),
        "to the bit"
    );
    // Asleep stays asleep.
    run(&mut physics, 600);
    let resting = physics.state().body_states();
    assert!(
        resting
            .iter()
            .filter(|(i, _)| *i != id(1))
            .all(|(_, s)| s.sleeping)
    );
    let mut again = Physics::new();
    rebuilt(&mut again, &six_crates, &resting);
    assert_eq!(again.state().body_states(), resting);
}

#[test]
fn body_states_that_arent_sensible_are_left_out() {
    let mut physics = Physics::new();
    six_crates(&mut physics);
    let before = physics.state().body_states();
    let mut bad = before[1].1;
    bad.position = Vec3::NAN;
    physics.set_body_state(before[1].0, bad, true);
    bad = before[1].1;
    bad.rotation = Quat::from_xyzw(0.0, 0.0, 0.0, 0.0);
    physics.set_body_state(before[1].0, bad, true);
    bad = before[1].1;
    bad.spin = Vec3::splat(f32::INFINITY);
    physics.set_body_state(before[1].0, bad, true);
    // Not in physics: nothing happens.
    physics.set_body_state(id(999), before[1].1, true);
    assert_eq!(physics.state().body_states(), before);
}

/// The character as entity 9, feet at `at`.
fn character_at(physics: &mut Physics, at: Vec3) {
    assert!(physics.add_character(id(9), (at, Quat::IDENTITY), CharacterSpec::default()));
}

/// Walks the character `ticks` ticks at `speed` metres a second (with
/// gravity's fall, as the ready-made controller does), stepping physics.
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

fn fixed_box(physics: &mut Physics, n: u64, at: Vec3, size: Vec3) {
    assert!(physics.add(
        id(n),
        (at, Quat::IDENTITY),
        Vec3::ONE,
        None,
        Some(ColliderSpec {
            shape: Shape::Box(size),
            ..ColliderSpec::default()
        }),
    ));
}

#[test]
fn a_character_falls_lands_and_walks() {
    let mut physics = with_floor();
    character_at(&mut physics, Vec3::new(0.0, 0.0, 2.0));
    walk(&mut physics, Vec3::ZERO, 90);
    let landed = feet(&physics);
    assert!(landed.z.abs() < 0.05, "landed at {landed}");
    assert!(physics.state().character(id(9)).unwrap().grounded);
    walk(&mut physics, Vec3::new(3.0, 0.0, 0.0), 60);
    let walked = feet(&physics);
    assert!((walked.x - 3.0).abs() < 0.1, "walked to {walked}");
    assert!(walked.z.abs() < 0.05);
    // Moved characters are reported once, after the step.
    physics.move_character(id(9), Vec3::X * 0.01, 1.0 / 60.0);
    physics.step(1.0 / 60.0);
    assert_eq!(physics.moved().len(), 1);
    assert!(physics.moved().is_empty());
    // One standing still isn't reported (the third review: an idle crowd
    // had every entity follow its body each tick).
    walk(&mut physics, Vec3::ZERO, 10);
    let _ = physics.moved();
    physics.move_character(id(9), Vec3::ZERO, 1.0 / 60.0);
    physics.step(1.0 / 60.0);
    assert!(physics.moved().is_empty());
}

#[test]
fn a_character_climbs_a_step_but_not_a_wall() {
    let mut physics = with_floor();
    // A 0.25 m step at x = 2, and a 2 m wall at y = 3.
    fixed_box(
        &mut physics,
        2,
        Vec3::new(4.0, 0.0, 0.125),
        Vec3::new(4.0, 4.0, 0.25),
    );
    fixed_box(
        &mut physics,
        3,
        Vec3::new(0.0, 4.0, 1.0),
        Vec3::new(1.0, 2.0, 2.0),
    );
    character_at(&mut physics, Vec3::new(0.0, 0.0, 0.0));
    walk(&mut physics, Vec3::ZERO, 10);
    walk(&mut physics, Vec3::new(3.0, 0.0, 0.0), 60);
    let up = feet(&physics);
    assert!(up.x > 2.5, "past the step: {up}");
    assert!((up.z - 0.25).abs() < 0.06, "on the step: {up}");
    // Back down, then into the wall: it stops at the wall's face.
    let mut physics = with_floor();
    fixed_box(
        &mut physics,
        3,
        Vec3::new(0.0, 4.0, 1.0),
        Vec3::new(1.0, 2.0, 2.0),
    );
    character_at(&mut physics, Vec3::ZERO);
    walk(&mut physics, Vec3::new(0.0, 4.0, 0.0), 120);
    let stopped = feet(&physics);
    // The wall's face is at y = 3; the character is 0.35 m wide.
    assert!(
        stopped.y < 3.0 - 0.3 && stopped.y > 2.5,
        "stopped at {stopped}"
    );
}

#[test]
fn a_character_slides_along_a_wall_it_walks_into_at_an_angle() {
    let mut physics = with_floor();
    fixed_box(
        &mut physics,
        3,
        Vec3::new(0.0, 2.0, 1.0),
        Vec3::new(20.0, 1.0, 2.0),
    );
    character_at(&mut physics, Vec3::ZERO);
    walk(&mut physics, Vec3::new(2.0, 2.0, 0.0), 120);
    let slid = feet(&physics);
    assert!(slid.x > 2.5, "slid along: {slid}");
    assert!(slid.y < 1.5 - 0.3, "not through: {slid}");
}

#[test]
fn a_character_rides_a_moving_platform() {
    let mut physics = with_floor();
    // A kinematic platform 4 m wide, its top at 1.
    assert!(physics.add(
        id(4),
        (Vec3::new(0.0, 0.0, 0.75), Quat::IDENTITY),
        Vec3::ONE,
        Some(Body {
            kind: BodyKind::Kinematic,
            ..Body::default()
        }),
        Some(ColliderSpec {
            shape: Shape::Box(Vec3::new(4.0, 4.0, 0.5)),
            ..ColliderSpec::default()
        }),
    ));
    character_at(&mut physics, Vec3::new(0.0, 0.0, 1.2));
    walk(&mut physics, Vec3::ZERO, 30);
    assert!(physics.state().character(id(9)).unwrap().grounded);
    // The platform slides 2 m along x over two seconds; the character,
    // standing still, goes with it.
    let dt = 1.0 / 60.0;
    for tick in 1..=120 {
        let x = 2.0 * tick as f32 / 120.0;
        physics.set_pose(id(4), Vec3::new(x, 0.0, 0.75), Quat::IDENTITY);
        let grounded = physics.state().character(id(9)).unwrap().grounded;
        let fall = if grounded { 0.0 } else { -3.0 };
        physics.move_character(id(9), Vec3::Z * fall * dt, dt);
        physics.step(dt);
    }
    let rode = feet(&physics);
    // Carried by the platform's speed, which physics knows a tick late.
    assert!((rode.x - 2.0).abs() < 0.15, "rode to {rode}");
    assert!((rode.z - 1.0).abs() < 0.06, "on top: {rode}");
}

#[test]
fn a_character_pushes_a_crate() {
    let mut physics = with_floor();
    let (body, collider) = crate_spec();
    physics.add(
        id(5),
        (Vec3::new(1.5, 0.0, 0.5), Quat::IDENTITY),
        Vec3::ONE,
        body,
        collider,
    );
    character_at(&mut physics, Vec3::ZERO);
    walk(&mut physics, Vec3::ZERO, 10);
    walk(&mut physics, Vec3::new(2.0, 0.0, 0.0), 120);
    let crate_at = physics.state().pose(id(5)).unwrap().0;
    assert!(crate_at.x > 2.0, "pushed to {crate_at}");
}

#[test]
fn a_character_moves_the_same_every_run_and_after_being_built_again() {
    let course = || {
        let mut physics = with_floor();
        fixed_box(
            &mut physics,
            2,
            Vec3::new(4.0, 0.0, 0.125),
            Vec3::new(4.0, 4.0, 0.25),
        );
        let (body, collider) = crate_spec();
        physics.add(
            id(5),
            (Vec3::new(1.0, 1.0, 0.5), Quat::IDENTITY),
            Vec3::ONE,
            body,
            collider,
        );
        character_at(&mut physics, Vec3::new(0.0, 0.0, 1.0));
        physics
    };
    let mut a = course();
    let mut b = course();
    walk(&mut a, Vec3::new(2.0, 1.0, 0.0), 40);
    walk(&mut b, Vec3::new(2.0, 1.0, 0.0), 40);
    assert_eq!(a.state().checksum(), b.state().checksum());
    let add_all = |physics: &mut Physics| {
        physics.add(
            id(1),
            (Vec3::new(0.0, 0.0, -0.5), Quat::IDENTITY),
            Vec3::ONE,
            None,
            Some(ColliderSpec {
                shape: Shape::Box(Vec3::new(20.0, 20.0, 1.0)),
                ..ColliderSpec::default()
            }),
        );
        fixed_box(
            physics,
            2,
            Vec3::new(4.0, 0.0, 0.125),
            Vec3::new(4.0, 4.0, 0.25),
        );
        let (body, collider) = crate_spec();
        physics.add(
            id(5),
            (Vec3::ZERO, Quat::IDENTITY),
            Vec3::ONE,
            body,
            collider,
        );
        character_at(physics, Vec3::ZERO);
    };
    let states = a.state().body_states();
    let mut c = Physics::new();
    rebuilt(&mut c, &add_all, &states);
    rebuilt(&mut a, &add_all, &states);
    assert!(
        c.state().character(id(9)).unwrap().grounded,
        "kept its finding"
    );
    walk(&mut a, Vec3::new(1.0, 2.0, 0.0), 60);
    walk(&mut c, Vec3::new(1.0, 2.0, 0.0), 60);
    assert_eq!(a.state().checksum(), c.state().checksum());
}

#[test]
fn character_moves_that_arent_sensible_are_refused() {
    let mut physics = with_floor();
    character_at(&mut physics, Vec3::new(0.0, 0.0, 1.0));
    // Not a character: nothing happens.
    assert!(!physics.move_character(id(1), Vec3::X, 1.0 / 60.0));
    assert!(!physics.move_character(id(77), Vec3::X, 1.0 / 60.0));
    for bad in [Vec3::NAN, Vec3::splat(f32::INFINITY)] {
        assert!(physics.move_character(id(9), bad, 1.0 / 60.0));
    }
    assert!(physics.move_character(id(9), Vec3::X, f32::NAN));
    physics.step(1.0 / 60.0);
    assert_eq!(feet(&physics), Vec3::new(0.0, 0.0, 1.0));
    // A huge move goes at most MAX_CHARACTER_MOVE.
    physics.move_character(id(9), Vec3::new(1.0e9, 0.0, 0.0), 1.0 / 60.0);
    physics.step(1.0 / 60.0);
    assert!(feet(&physics).x <= MAX_CHARACTER_MOVE + 0.01);
    // A character's numbers are made sensible.
    assert!(physics.add_character(
        id(10),
        (Vec3::new(5.0, 5.0, 0.0), Quat::IDENTITY),
        CharacterSpec {
            height: f32::NAN,
            radius: -3.0,
            step: f32::INFINITY,
            slope: 400.0,
            snap: f32::NAN,
            mass: 0.0,
        },
    ));
    physics.move_character(id(10), Vec3::X * 0.1, 1.0 / 60.0);
    physics.step(1.0 / 60.0);
}

#[test]
fn a_character_walks_up_a_gentle_ramp_but_not_a_steep_one() {
    let climb = |degrees: f32| {
        let mut physics = with_floor();
        // A long ramp rising along +x from x = 1.
        let turn = Quat::from_rotation_y(-degrees.to_radians());
        let half = 4.0;
        let middle = Vec3::new(1.0, 0.0, 0.0) + turn * Vec3::new(half, 0.0, -0.05);
        assert!(physics.add(
            id(2),
            (middle, turn),
            Vec3::ONE,
            None,
            Some(ColliderSpec {
                shape: Shape::Box(Vec3::new(2.0 * half, 4.0, 0.1)),
                ..ColliderSpec::default()
            }),
        ));
        character_at(&mut physics, Vec3::new(-0.5, 0.0, 0.0));
        walk(&mut physics, Vec3::ZERO, 10);
        walk(&mut physics, Vec3::new(2.0, 0.0, 0.0), 120);
        feet(&physics).z
    };
    let gentle = climb(25.0);
    assert!(gentle > 0.6, "up the gentle ramp to {gentle}");
    let steep = climb(65.0);
    assert!(steep < 0.5, "not up the steep one: {steep}");
}

#[test]
fn many_characters_moved_together_go_exactly_where_one_by_one_would() {
    let crowd = || {
        let mut physics = with_floor();
        for n in 0..40u64 {
            let at = Vec3::new((n % 8) as f32 * 1.5, (n / 8) as f32 * 1.5, 0.0);
            assert!(physics.add_character(
                id(100 + n),
                (at, Quat::IDENTITY),
                CharacterSpec::default()
            ));
        }
        let (body, collider) = crate_spec();
        physics.add(
            id(5),
            (Vec3::new(2.0, 2.0, 0.5), Quat::IDENTITY),
            Vec3::ONE,
            body,
            collider,
        );
        physics
    };
    let moves = |tick: u32| -> Vec<(EntityId, Vec3)> {
        let mut moves: Vec<(EntityId, Vec3)> = (0..40u64)
            .map(|n| {
                let a = tick as f32 * 0.1 + n as f32;
                (id(100 + n), Vec3::new(a.cos(), a.sin(), -0.5) * 0.07)
            })
            .collect();
        // One moved twice in a tick, and one that isn't a character.
        moves.push((id(103), Vec3::new(0.05, 0.0, 0.0)));
        moves.push((id(5), Vec3::X));
        moves
    };
    let (mut together, mut alone) = (crowd(), crowd());
    for tick in 0..60 {
        let m = moves(tick);
        let answers = together.move_characters(&m, 1.0 / 60.0);
        assert!(!answers[answers.len() - 1], "a crate isn't a character");
        for (who, wanted) in &m {
            alone.move_character(*who, *wanted, 1.0 / 60.0);
        }
        together.step(1.0 / 60.0);
        alone.step(1.0 / 60.0);
    }
    assert_eq!(together.state().checksum(), alone.state().checksum());
}

#[test]
fn numbers_too_big_for_physics_are_refused_or_held_in_range() {
    // The second review: a rotation whose length overflows passed as
    // finite and gave bodies NaN places; a collider 1e20 m big made
    // Rapier's maths fail and emptied physics.
    let mut physics = with_floor();
    let (body, collider) = crate_spec();
    assert!(!physics.add(
        id(2),
        (Vec3::ZERO, Quat::from_xyzw(2.0e19, 0.0, 0.0, 0.0)),
        Vec3::ONE,
        body,
        collider,
    ));
    assert!(!physics.add(
        id(3),
        (Vec3::splat(2.0e9), Quat::IDENTITY),
        Vec3::ONE,
        body,
        collider
    ));
    let mut state = physics.state().body_state(id(1)).unwrap();
    state.velocity = Vec3::splat(1.0e30);
    assert!(!state.is_sensible());
    // Huge, but held to a thousand kilometres: physics keeps working.
    assert!(physics.add(
        id(4),
        (Vec3::new(0.0, 0.0, 5.0), Quat::IDENTITY),
        Vec3::splat(1.0e20),
        body,
        collider
    ));
    run(&mut physics, 5);
    assert!(!physics.take_broke());
    assert!(
        physics
            .state()
            .body_states()
            .iter()
            .all(|(_, s)| s.is_sensible())
    );
    let _ = physics
        .state()
        .cast_ray(Vec3::new(0.0, 0.0, 9.0e8), -Vec3::Z, 1.0e7);
}

#[test]
fn a_fixed_or_kinematic_body_built_again_stays_where_its_entity_put_it() {
    let mut physics = with_floor();
    assert!(physics.add(
        id(6),
        (Vec3::new(1.0, 0.0, 2.0), Quat::IDENTITY),
        Vec3::ONE,
        Some(Body {
            kind: BodyKind::Kinematic,
            ..Body::default()
        }),
        Some(ColliderSpec::default()),
    ));
    let mut stale = physics.state().body_state(id(6)).unwrap();
    stale.position = Vec3::new(9.0, 9.0, 9.0);
    physics.set_body_state(id(6), stale, true);
    assert_eq!(
        physics.state().pose(id(6)).unwrap().0,
        Vec3::new(1.0, 0.0, 2.0)
    );
}

/// A crowd's moves for a property test: (which character, x, y, z) per
/// move, a tick's worth at a time.
type Ticks = Vec<Vec<(u8, i8, i8, i8)>>;

/// Plays `ticks` of moves on a level with `crowd` characters at random
/// places, crates, and a platform that moves or not; moved `together`
/// (as the world does) or one by one. Returns the state's checksum.
fn crowd_played(places: &[(i8, i8)], platform: bool, ticks: &Ticks, together: bool) -> u64 {
    let mut physics = with_floor();
    for (n, &(x, y)) in places.iter().enumerate() {
        physics.add_character(
            id(100 + n as u64),
            (
                Vec3::new(f32::from(x) * 0.4, f32::from(y) * 0.4, 0.0),
                Quat::IDENTITY,
            ),
            CharacterSpec::default(),
        );
    }
    for n in 0..3u64 {
        let (body, collider) = crate_spec();
        physics.add(
            id(5 + n),
            (Vec3::new(n as f32, 1.0, 0.5 + n as f32), Quat::IDENTITY),
            Vec3::splat(0.5),
            body,
            collider,
        );
    }
    physics.add(
        id(9),
        (Vec3::new(0.0, 0.0, 0.05), Quat::IDENTITY),
        Vec3::new(3.0, 3.0, 0.1),
        Some(Body {
            kind: BodyKind::Kinematic,
            ..Body::default()
        }),
        Some(ColliderSpec::default()),
    );
    for (tick, moves) in ticks.iter().enumerate() {
        if platform {
            let x = (tick as f32 * 0.05).sin();
            physics.set_pose(id(9), Vec3::new(x, 0.0, 0.05), Quat::IDENTITY);
        }
        let wanted: Vec<(EntityId, Vec3)> = moves
            .iter()
            .map(|&(who, x, y, z)| {
                (
                    // Now and then something that isn't a character.
                    id(100 + u64::from(who) % (places.len() as u64 + 2)),
                    Vec3::new(f32::from(x), f32::from(y), f32::from(z)) * 0.002,
                )
            })
            .collect();
        if together {
            physics.move_characters(&wanted, 1.0 / 60.0);
        } else {
            for (who, w) in &wanted {
                physics.move_character(*who, *w, 1.0 / 60.0);
            }
        }
        physics.step(1.0 / 60.0);
    }
    physics.state().checksum()
}

proptest::proptest! {
    #![proptest_config(proptest::prelude::ProptestConfig {
        cases: std::env::var("PROPTEST_CASES")
            .ok()
            .and_then(|n| n.parse().ok())
            .unwrap_or(16),
        ..proptest::prelude::ProptestConfig::default()
    })]

    /// Characters moved together (worked out on every core at once) end
    /// exactly where moving them one by one puts them: any crowd, any
    /// moves, a character moved twice in a tick, a platform moving or
    /// not (decision 0042's invariants).
    #[test]
    fn a_crowd_moved_together_ends_where_one_by_one_would(
        places in proptest::collection::vec((-20i8..20, -20i8..20), 1..24),
        platform in proptest::prelude::any::<bool>(),
        ticks in proptest::collection::vec(
            proptest::collection::vec(
                (proptest::prelude::any::<u8>(), proptest::prelude::any::<i8>(),
                 proptest::prelude::any::<i8>(), -40i8..40),
                0..30,
            ),
            1..30,
        ),
    ) {
        proptest::prop_assert_eq!(
            crowd_played(&places, platform, &ticks, true),
            crowd_played(&places, platform, &ticks, false)
        );
    }
}
