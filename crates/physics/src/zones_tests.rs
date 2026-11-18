//! Trigger zones asking physics what overlaps them, in 3D and 2D: moving
//! things are found, still ones aren't, turned zones and concave outlines
//! are exact, and the answers are the same on one core or four.

use crate::flat::{Flat, FlatCollider, FlatShape, FlatZone, FlatZoneQuery};
use crate::{Body, BodyKind, CharacterSpec, ColliderSpec, Physics, Shape, ZoneQuery, ZoneShape};
use glam::{Quat, Vec2, Vec3};
use reflect::EntityId;

fn id(n: u64) -> EntityId {
    EntityId(reflect::Id::from_raw(n).unwrap())
}

/// A small ball of each kind of body: dynamic (2), kinematic (3), fixed
/// (4), and a character (5), along X, a metre apart, held up so nothing
/// falls (no gravity on them), stepped once so the lists are made.
fn bodies() -> Physics {
    let mut physics = Physics::new();
    let ball = Some(ColliderSpec {
        shape: Shape::Sphere(0.2),
        ..ColliderSpec::default()
    });
    for (n, kind) in [
        (2, BodyKind::Dynamic),
        (3, BodyKind::Kinematic),
        (4, BodyKind::Fixed),
    ] {
        let body = Body {
            kind,
            gravity: 0.0,
            ..Body::default()
        };
        physics.add(
            id(n),
            (Vec3::new(n as f32, 0.0, 0.0), Quat::IDENTITY),
            Vec3::ONE,
            Some(body),
            ball.clone(),
        );
    }
    physics.add_character(
        id(5),
        (Vec3::new(5.0, 0.0, 0.0), Quat::IDENTITY),
        CharacterSpec::default(),
    );
    physics.step(1.0 / 60.0);
    physics
}

fn zone(n: u64, place: Vec3, shape: ZoneShape) -> ZoneQuery {
    ZoneQuery {
        zone: id(n),
        place,
        rotation: Quat::IDENTITY,
        shape,
    }
}

#[test]
fn a_3d_zone_finds_moving_things_but_not_still_ones() {
    let physics = bodies();
    let everything = zone(
        9,
        Vec3::new(3.5, 0.0, 0.0),
        ZoneShape::Box(Vec3::new(8.0, 2.0, 4.0)),
    );
    let found = physics.state().overlapping(&[everything]);
    assert_eq!(found, [vec![id(2), id(3), id(5)]]);
}

#[test]
fn a_3d_ball_and_a_turned_box_are_exact() {
    let physics = bodies();
    // A ball at the dynamic body's middle, too small to reach the next.
    let ball = zone(9, Vec3::new(2.0, 0.0, 0.0), ZoneShape::Ball(0.5));
    // A long thin box along Y at x = 3, turned a quarter round Z: it then
    // lies along X and reaches the dynamic and the kinematic bodies.
    let thin = ZoneQuery {
        rotation: Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
        ..zone(
            10,
            Vec3::new(2.5, 0.0, 0.0),
            ZoneShape::Box(Vec3::new(0.1, 1.2, 0.1)),
        )
    };
    let found = physics.state().overlapping(&[ball, thin]);
    assert_eq!(found, [vec![id(2)], vec![id(2), id(3)]]);
}

#[test]
fn a_zone_never_finds_itself_and_odd_places_find_nothing() {
    let physics = bodies();
    let on_itself = zone(2, Vec3::new(2.0, 0.0, 0.0), ZoneShape::Ball(0.5));
    let lost = zone(9, Vec3::splat(f32::NAN), ZoneShape::Ball(1.0));
    let huge = zone(10, Vec3::ZERO, ZoneShape::Box(Vec3::splat(f32::INFINITY)));
    let found = physics.state().overlapping(&[on_itself, lost, huge]);
    assert_eq!(found[0], Vec::<EntityId>::new());
    assert_eq!(found[1], Vec::<EntityId>::new());
    assert_eq!(found[2], vec![id(2), id(3), id(5)]);
}

/// 2D: characters 2 and 3 a metre apart, a still house at 4, and a map's
/// wall at 6.
fn flat_world() -> Flat {
    let mut flat = Flat::new();
    let circle = FlatCollider {
        shape: FlatShape::Circle(0.2),
        center: Vec2::ZERO,
    };
    flat.add(id(2), Vec2::new(2.0, 0.0), circle, true);
    flat.add(id(3), Vec2::new(3.0, 0.0), circle, true);
    flat.add(id(4), Vec2::new(4.0, 0.0), circle, false);
    flat.set_walls(
        id(7),
        [0, 0],
        &[(Vec2::new(5.8, -0.2), Vec2::new(6.2, 0.2))],
    );
    flat
}

fn flat_zone(n: u64, place: Vec2, shape: &FlatZone) -> FlatZoneQuery<'_> {
    FlatZoneQuery {
        zone: id(n),
        place,
        angle: 0.0,
        shape,
    }
}

#[test]
fn a_2d_zone_finds_characters_only() {
    let flat = flat_world();
    let wide = FlatZone::rectangle(Vec2::new(10.0, 2.0));
    let found = flat.overlapping(&[flat_zone(9, Vec2::new(4.0, 0.0), &wide)]);
    assert_eq!(found, [vec![id(2), id(3)]]);
}

#[test]
fn a_2d_circle_and_a_turned_box_are_exact() {
    let flat = flat_world();
    let small = FlatZone::circle(0.5);
    let thin = FlatZone::rectangle(Vec2::new(0.1, 1.2));
    let turned = FlatZoneQuery {
        angle: std::f32::consts::FRAC_PI_2,
        ..flat_zone(10, Vec2::new(2.5, 0.0), &thin)
    };
    let found = flat.overlapping(&[flat_zone(9, Vec2::new(2.0, 0.0), &small), turned]);
    assert_eq!(found, [vec![id(2)], vec![id(2), id(3)]]);
}

#[test]
fn a_concave_2d_zone_is_checked_exactly() {
    // An L 4 m across, its notch top-right: a character in the notch is
    // outside; one in each arm is inside.
    let shape = FlatZone::pieces(&ell_pieces());
    let mut flat = Flat::new();
    let dot = FlatCollider {
        shape: FlatShape::Circle(0.1),
        center: Vec2::ZERO,
    };
    flat.add(id(2), Vec2::new(3.0, 3.0), dot, true); // the notch
    flat.add(id(3), Vec2::new(3.0, 1.0), dot, true); // the lower arm
    flat.add(id(4), Vec2::new(1.0, 3.0), dot, true); // the upper arm
    flat.add(id(5), Vec2::new(2.15, 2.15), dot, true); // just outside the inner corner
    let found = flat.overlapping(&[flat_zone(9, Vec2::ZERO, &shape)]);
    assert_eq!(found, [vec![id(3), id(4)]]);
}

/// An L's convex pieces, as `mesh::outline` cuts them (it isn't a
/// dependency of physics, so they're written out): 4 m across, its notch
/// top-right.
fn ell_pieces() -> Vec<Vec<Vec2>> {
    vec![
        vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(4.0, 0.0),
            Vec2::new(4.0, 2.0),
            Vec2::new(2.0, 2.0),
            Vec2::new(0.0, 2.0),
        ],
        vec![
            Vec2::new(0.0, 2.0),
            Vec2::new(2.0, 2.0),
            Vec2::new(2.0, 4.0),
            Vec2::new(0.0, 4.0),
        ],
    ]
}

#[test]
fn a_character_put_somewhere_is_found_there_at_once() {
    let mut flat = flat_world();
    let small = FlatZone::circle(0.5);
    flat.set_place(id(2), Vec2::new(10.0, 10.0));
    let found = flat.overlapping(&[flat_zone(9, Vec2::new(10.0, 10.0), &small)]);
    assert_eq!(found, [vec![id(2)]]);
}

#[test]
fn unusable_pieces_are_left_out() {
    let odd = FlatZone::pieces(&[
        vec![Vec2::ZERO, Vec2::X],
        vec![Vec2::splat(f32::NAN), Vec2::X, Vec2::Y],
    ]);
    assert_eq!(odd.piece_count(), 0);
    let flat = flat_world();
    let found = flat.overlapping(&[flat_zone(9, Vec2::new(2.0, 0.0), &odd)]);
    assert_eq!(found, [Vec::<EntityId>::new()]);
}

#[test]
fn many_zones_give_the_same_answers_on_one_core_or_four() {
    let mut flat = Flat::new();
    let dot = FlatCollider {
        shape: FlatShape::Circle(0.25),
        center: Vec2::ZERO,
    };
    for n in 0..400u64 {
        let at = Vec2::new((n % 20) as f32, (n / 20) as f32) * 0.7;
        flat.add(id(n + 100), at, dot, true);
    }
    let shape = FlatZone::rectangle(Vec2::splat(1.5));
    let zones: Vec<FlatZoneQuery<'_>> = (0..100u64)
        .map(|n| {
            flat_zone(
                n + 1,
                Vec2::new((n % 10) as f32, (n / 10) as f32) * 1.3,
                &shape,
            )
        })
        .collect();
    let on = |threads: usize, flat: &Flat| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(|| flat.overlapping(&zones))
    };
    let one = on(1, &flat);
    let four = on(4, &flat);
    assert_eq!(one, four);
    assert!(one.iter().map(Vec::len).sum::<usize>() > 100);
}

#[test]
fn a_body_just_added_is_found_before_any_step() {
    // As after physics is built again (an edit while playing, a save, a
    // load): nothing has stepped since, yet the zone sees it.
    let mut physics = Physics::new();
    physics.add_character(
        id(5),
        (Vec3::new(1.0, 1.0, 0.0), Quat::IDENTITY),
        CharacterSpec::default(),
    );
    let found =
        physics
            .state()
            .overlapping(&[zone(9, Vec3::new(1.0, 1.0, 1.0), ZoneShape::Ball(0.5))]);
    assert_eq!(found, [vec![id(5)]]);
}
