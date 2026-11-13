//! 2D physics: characters stop at walls and slide along them, never snag
//! on the seams between walls, round corners, the same every run; walls
//! replaced chunk by chunk; nothing that isn't a number gets in.

use super::*;
use proptest::prelude::*;

fn id(n: u64) -> EntityId {
    EntityId(reflect::Id::from_raw(n).unwrap())
}

fn circle(r: f32) -> FlatCollider {
    FlatCollider {
        shape: FlatShape::Circle(r),
        center: Vec2::ZERO,
    }
}

/// A wall rectangle from its corners.
fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> (Vec2, Vec2) {
    (Vec2::new(x0, y0), Vec2::new(x1, y1))
}

/// A character of radius 0.25 m at `at` (entity 1), and walls from map 9.
fn world(at: Vec2, walls: &[(Vec2, Vec2)]) -> Flat {
    let mut flat = Flat::new();
    flat.add(id(1), at, circle(0.25), true);
    flat.set_walls(id(9), [0, 0], walls);
    flat
}

fn walk(flat: &mut Flat, step: Vec2, ticks: usize) -> Vec2 {
    for _ in 0..ticks {
        flat.move_characters(&[(id(1), step)], 1.0 / 60.0);
    }
    flat.state().place(id(1)).unwrap()
}

#[test]
fn a_character_walks_freely_and_stops_a_skin_short_of_a_wall() {
    let mut flat = world(Vec2::ZERO, &[rect(2.0, -5.0, 3.0, 5.0)]);
    let at = walk(&mut flat, Vec2::new(0.1, 0.0), 5);
    assert!((at.x - 0.5).abs() < 1e-5, "free: {at}");
    let at = walk(&mut flat, Vec2::new(0.1, 0.0), 40);
    let gap = 2.0 - (at.x + 0.25);
    assert!(
        gap > 0.0 && gap <= SKIN * 1.5,
        "stopped at the wall: gap {gap}"
    );
    assert_eq!(flat.moved(), vec![(id(1), at)]);
    assert!(flat.moved().is_empty(), "said once");
}

#[test]
fn it_slides_along_a_wall_it_walks_into_at_an_angle() {
    let mut flat = world(Vec2::new(1.5, 0.0), &[rect(2.0, -50.0, 3.0, 50.0)]);
    // Diagonally into the wall: right and up, the up part kept.
    let at = walk(&mut flat, Vec2::new(0.07, 0.07), 100);
    assert!(at.x < 2.0 - 0.25, "never through: {at}");
    assert!(at.y > 6.5, "slid up along it: {at}");
}

#[test]
fn walking_along_a_thousand_seams_never_bumps() {
    // A floor's edge of 1,000 wall tiles side by side, each its own
    // rectangle (as if each were in another chunk), the character pressed
    // against them and walking along: it never slows or catches.
    let walls: Vec<(Vec2, Vec2)> = (0..1000)
        .map(|i| rect(i as f32, -1.0, i as f32 + 1.0, 0.0))
        .collect();
    let mut flat = Flat::new();
    flat.add(id(1), Vec2::new(0.5, 0.25 + SKIN), circle(0.25), true);
    for (i, w) in walls.iter().enumerate() {
        flat.set_walls(id(9), [i as u32, 0], &[*w]);
    }
    let step = Vec2::new(0.11, -0.02); // pressed down into the walls
    let mut last = flat.state().place(id(1)).unwrap();
    for _ in 0..9500 {
        flat.move_characters(&[(id(1), step)], 1.0 / 60.0);
        let at = flat.state().place(id(1)).unwrap();
        if at.x > 998.0 {
            break;
        }
        // Never slower than asked (a catch); a hair faster is the slide's
        // rounding.
        let moved = at.x - last.x;
        assert!(
            moved > 0.11 - 1e-4 && moved < 0.11 + 1e-3,
            "a bump at x {}: moved {}",
            last.x,
            at.x - last.x
        );
        assert!(
            at.y >= 0.25 && at.y < 0.25 + 2.0 * SKIN,
            "stayed on top: {at}"
        );
        last = at;
    }
    assert!(last.x > 990.0, "{last}");
}

#[test]
fn a_box_slides_along_seams_too() {
    let mut flat = Flat::new();
    let feet = FlatCollider {
        shape: FlatShape::Box(Vec2::new(0.6, 0.4)),
        center: Vec2::new(0.0, 0.2),
    };
    flat.add(id(1), Vec2::new(0.5, SKIN), feet, true);
    for i in 0..50 {
        flat.set_walls(id(9), [i, 0], &[rect(i as f32, -1.0, i as f32 + 1.0, 0.0)]);
    }
    let at = walk(&mut flat, Vec2::new(0.1, -0.05), 400);
    assert!(at.x > 40.0, "{at}");
    assert!(at.y >= 0.0, "{at}");
}

#[test]
fn a_round_character_slips_round_a_corner_it_clips() {
    // A wall ending at y = 0; the character walks up past its end, just
    // clipping the corner: a circle is pushed round it instead of stopping.
    let mut flat = world(Vec2::new(1.8, -2.0), &[rect(2.0, -10.0, 3.0, 0.0)]);
    let at = walk(&mut flat, Vec2::new(0.02, 0.1), 60);
    assert!(at.y > 3.0, "got past the corner: {at}");
}

#[test]
fn characters_bump_into_each_other_and_solid_things() {
    let mut flat = world(Vec2::ZERO, &[]);
    flat.add(id(2), Vec2::new(2.0, 0.0), circle(0.25), true);
    flat.add(
        id(3),
        Vec2::new(0.0, 3.0),
        FlatCollider {
            shape: FlatShape::Box(Vec2::new(4.0, 1.0)),
            center: Vec2::ZERO,
        },
        false,
    );
    let at = walk(&mut flat, Vec2::new(0.1, 0.0), 40);
    assert!(at.x < 1.5, "stopped at the other character: {at}");
    let at = walk(&mut flat, Vec2::new(0.0, 0.1), 60);
    assert!(at.y < 2.5 - 0.25 + 1e-3, "stopped at the box: {at}");
    // A solid thing isn't moved by asking.
    flat.move_characters(&[(id(3), Vec2::new(1.0, 0.0))], 1.0 / 60.0);
    assert_eq!(flat.state().place(id(3)), Some(Vec2::new(0.0, 3.0)));
}

#[test]
fn walls_are_replaced_chunk_by_chunk_and_taken_away() {
    let mut flat = world(Vec2::ZERO, &[rect(1.0, -1.0, 2.0, 1.0)]);
    flat.set_walls(
        id(9),
        [1, 0],
        &[rect(5.0, -1.0, 6.0, 1.0), rect(7.0, -1.0, 8.0, 1.0)],
    );
    assert_eq!(flat.state().wall_count(), 3);
    // The first chunk's wall is painted away: the way is open.
    flat.set_walls(id(9), [0, 0], &[]);
    assert_eq!(flat.state().wall_count(), 2);
    let at = walk(&mut flat, Vec2::new(0.1, 0.0), 60);
    assert!(at.x > 4.7 && at.x < 5.0 - 0.25, "{at}");
    flat.set_walls(id(8), [0, 0], &[rect(-3.0, -1.0, -2.0, 1.0)]);
    assert_eq!(flat.maps_with_walls(), vec![id(8), id(9)]);
    flat.remove_walls(id(9));
    assert_eq!(flat.state().wall_count(), 1);
    assert_eq!(flat.maps_with_walls(), vec![id(8)]);
    // Rectangles that aren't sensible aren't made.
    flat.set_walls(
        id(7),
        [0, 0],
        &[rect(1.0, 1.0, 1.0, 2.0), rect(f32::NAN, 0.0, 1.0, 1.0)],
    );
    assert_eq!(flat.state().wall_count(), 1);
}

#[test]
fn moved_by_hand_it_goes_there_without_sliding() {
    let mut flat = world(Vec2::ZERO, &[rect(1.0, -1.0, 2.0, 1.0)]);
    flat.set_place(id(1), Vec2::new(5.0, 0.0));
    assert_eq!(flat.state().place(id(1)), Some(Vec2::new(5.0, 0.0)));
    assert!(flat.moved().is_empty(), "not physics' move");
    let at = walk(&mut flat, Vec2::new(-0.1, 0.0), 40);
    assert!(
        at.x > 2.0 + 0.25,
        "and walls still stop it from there: {at}"
    );
    flat.set_place(id(1), Vec2::new(f32::INFINITY, 0.0));
    assert_eq!(flat.state().place(id(1)), Some(at), "not a number: ignored");
}

#[test]
fn nothing_that_isn_t_a_number_gets_in() {
    let mut flat = world(Vec2::ZERO, &[]);
    flat.move_characters(&[(id(1), Vec2::new(f32::NAN, 0.0))], 1.0 / 60.0);
    flat.move_characters(&[(id(1), Vec2::new(1.0e30, 0.0))], 1.0 / 60.0);
    let at = flat.state().place(id(1)).unwrap();
    assert_eq!(at, Vec2::new(MAX_FLAT_MOVE, 0.0), "cut short");
    flat.add(id(2), Vec2::new(f32::NAN, 0.0), circle(0.5), true);
    assert!(!flat.state().contains(id(2)));
    flat.add(
        id(3),
        Vec2::ZERO,
        FlatCollider {
            shape: FlatShape::Circle(f32::NAN),
            center: Vec2::new(f32::INFINITY, 0.0),
        },
        true,
    );
    assert_eq!(
        flat.state().place(id(3)),
        Some(Vec2::ZERO),
        "sizes made sensible"
    );
    flat.move_characters(&[(id(42), Vec2::ONE)], 1.0 / 60.0);
}

#[test]
fn a_character_knows_when_it_stands_on_the_ground_and_how_fast_it_goes() {
    // A floor under a character a skin above it, and a wall to its right.
    let mut flat = world(
        Vec2::new(0.0, 0.25 + SKIN),
        &[rect(-10.0, -1.0, 10.0, 0.0), rect(2.0, 0.0, 3.0, 5.0)],
    );
    let fall = Vec2::new(0.0, -0.1);
    walk(&mut flat, fall, 1);
    let last = flat.state().last_move(id(1)).unwrap();
    assert!(last.grounded, "falling onto the floor: on the ground");
    assert!(last.velocity.y.abs() < 0.5, "stopped going down: {last:?}");
    // Running right along it: 0.05 a tick at 60 a second is 3 m/s.
    walk(&mut flat, Vec2::new(0.05, -0.1), 1);
    let last = flat.state().last_move(id(1)).unwrap();
    assert!(last.grounded);
    assert!((last.velocity.x - 3.0).abs() < 0.01, "{last:?}");
    // Into the wall: stopped, and a wall isn't ground.
    walk(&mut flat, Vec2::new(0.1, 0.0), 60);
    let last = flat.state().last_move(id(1)).unwrap();
    assert!(!last.grounded && last.velocity.x.abs() < 0.01, "{last:?}");
    // Jumping up: off the ground, going up.
    walk(&mut flat, Vec2::new(0.0, 0.1), 1);
    let last = flat.state().last_move(id(1)).unwrap();
    assert!(
        !last.grounded && (last.velocity.y - 6.0).abs() < 0.01,
        "{last:?}"
    );
    // A tick it isn't moved: standing still, not grounded; paused: no speed.
    flat.move_characters(&[], 1.0 / 60.0);
    assert_eq!(flat.state().last_move(id(1)), Some(FlatMove::default()));
    flat.move_characters(&[(id(1), Vec2::new(0.0, -0.1))], 0.0);
    assert_eq!(flat.state().last_move(id(1)).unwrap().velocity, Vec2::ZERO);
    // Not a character: nothing to say.
    flat.add(id(5), Vec2::new(-5.0, 3.0), circle(0.2), false);
    assert_eq!(flat.state().last_move(id(5)), None);
}

#[test]
fn the_same_moves_give_the_same_places_and_checksum() {
    let run = || {
        let mut flat = world(
            Vec2::ZERO,
            &[rect(2.0, -2.0, 3.0, 2.0), rect(-1.0, 3.0, 4.0, 4.0)],
        );
        flat.add(id(2), Vec2::new(0.0, -1.5), circle(0.3), true);
        for i in 0..300 {
            let a = i as f32 * 0.07;
            flat.move_characters(
                &[
                    (id(1), Vec2::new(a.cos(), a.sin()) * 0.08),
                    (id(2), Vec2::new(-a.sin(), a.cos()) * 0.05),
                ],
                1.0 / 60.0,
            );
        }
        (
            flat.state().place(id(1)),
            flat.state().place(id(2)),
            flat.state().checksum(),
        )
    };
    assert_eq!(run(), run());
    let empty = Flat::new();
    assert_ne!(empty.state().checksum(), run().2);
}

#[test]
fn cleared_it_empties_and_a_clone_carries_on_alike() {
    let mut flat = world(Vec2::ZERO, &[rect(2.0, -2.0, 3.0, 2.0)]);
    walk(&mut flat, Vec2::new(0.1, 0.0), 3);
    let mut copy = flat.clone();
    let a = walk(&mut flat, Vec2::new(0.1, 0.05), 30);
    let b = walk(&mut copy, Vec2::new(0.1, 0.05), 30);
    assert_eq!(a, b);
    flat.clear();
    assert!(flat.state().is_empty());
}

/// A second character (entity 2) of the same size at `at`.
fn another(flat: &mut Flat, at: Vec2) {
    flat.add(id(2), at, circle(0.25), true);
}

/// Rapier failing never crashes Solis: 2D physics empties, walls and
/// all, and says so once. It used to say nothing, so nobody built it
/// again and every 2D character stood still for the rest of the play (the
/// full review).
#[test]
fn a_break_empties_2d_physics_and_says_so_once() {
    let wall = [rect(2.0, -5.0, 3.0, 5.0)];
    let mut flat = world(Vec2::ZERO, &wall);
    assert!(!flat.take_broke(), "nothing broke yet");
    flat.fail_next();
    flat.move_characters(&[(id(1), Vec2::new(0.1, 0.0))], 1.0 / 60.0);
    assert!(flat.state().is_empty(), "emptied, walls too");
    assert!(flat.moved().is_empty(), "nothing for the world to follow");
    assert!(flat.take_broke());
    assert!(!flat.take_broke(), "said once");
    // Emptied, a walk moves no one and breaks nothing more.
    flat.move_characters(&[(id(1), Vec2::new(0.1, 0.0))], 1.0 / 60.0);
    assert!(!flat.take_broke());
    // Built again (as the world does), it works as it did.
    flat.add(id(1), Vec2::ZERO, circle(0.25), true);
    flat.set_walls(id(9), [0, 0], &wall);
    let at = walk(&mut flat, Vec2::new(0.1, 0.0), 60);
    assert!(
        (at.x - (2.0 - 0.25 - SKIN)).abs() < 1e-3,
        "at the wall: {at}"
    );
    assert!(!flat.take_broke());
}

/// What it broke on is known only when a character's move is all that
/// had changed since Rapier last caught up.
#[test]
fn a_break_names_the_move_it_broke_on_when_that_s_all_that_changed() {
    let step = Vec2::new(0.1, 0.0);
    let mut flat = world(Vec2::ZERO, &[rect(2.0, -5.0, 3.0, 5.0)]);
    another(&mut flat, Vec2::new(0.0, 3.0));
    assert_eq!(flat.broke_on(), None, "it never broke");
    // Entities and walls joined since Rapier last caught up: it fails
    // before anyone moves, on any of them.
    flat.fail_next();
    flat.move_characters(&[(id(1), step)], 1.0 / 60.0);
    assert!(flat.take_broke());
    assert_eq!(flat.broke_on(), None);

    // Caught up (a tick with no moves), then one walks: it fails catching
    // up with that walk.
    let mut flat = world(Vec2::ZERO, &[rect(2.0, -5.0, 3.0, 5.0)]);
    another(&mut flat, Vec2::new(0.0, 3.0));
    flat.move_characters(&[], 1.0 / 60.0);
    flat.fail_next();
    flat.move_characters(&[(id(2), step)], 1.0 / 60.0);
    assert!(flat.take_broke());
    assert_eq!(flat.broke_on(), Some(id(2)));

    // Two walk: it fails catching up with the first's, before the
    // second's is made.
    let mut flat = world(Vec2::ZERO, &[rect(2.0, -5.0, 3.0, 5.0)]);
    another(&mut flat, Vec2::new(0.0, 3.0));
    flat.move_characters(&[], 1.0 / 60.0);
    flat.fail_next();
    flat.move_characters(&[(id(2), step), (id(1), step)], 1.0 / 60.0);
    assert!(flat.take_broke());
    assert_eq!(flat.broke_on(), Some(id(2)));

    // One asked to walk nowhere changed nothing Rapier knows of: no move
    // to name.
    let mut flat = world(Vec2::ZERO, &[rect(2.0, -5.0, 3.0, 5.0)]);
    flat.move_characters(&[], 1.0 / 60.0);
    flat.fail_next();
    flat.move_characters(&[(id(1), Vec2::ZERO)], 1.0 / 60.0);
    assert!(flat.take_broke());
    assert_eq!(flat.broke_on(), None);
}

/// The wall a round character (radius 0.25, as `world` makes it) at `at`
/// is inside, if any: its middle nearer the wall than its radius (less
/// a millimetre for rounding).
fn wall_it_is_inside(at: Vec2, walls: &[(Vec2, Vec2)]) -> Option<(Vec2, Vec2)> {
    walls
        .iter()
        .copied()
        .find(|(lo, hi)| at.clamp(*lo, *hi).distance(at) < 0.25 - 1e-3)
}

#[test]
fn the_inside_check_knows_a_circle_from_its_box() {
    let wall = [rect(-4.0, -4.0, -1.0, -1.0)];
    // Where NEW-09's character ended: 0.208 from the corner, so 4 cm into it.
    assert!(wall_it_is_inside(Vec2::new(-0.967_717_3, -0.794_273_7), &wall).is_some());
    // Off the corner diagonally, the circle's box would overlap the wall's
    // and the circle doesn't.
    assert!(wall_it_is_inside(Vec2::new(-0.8, -0.8), &wall).is_none());
    assert!(wall_it_is_inside(Vec2::new(-0.76, -2.0), &wall).is_some());
    assert!(wall_it_is_inside(Vec2::new(-0.74, -2.0), &wall).is_none());
}

/// The full review's NEW-09, which the property test below rolled: a
/// circle swept past a box's corner, and Rapier's cast (parry's GJK) closed
/// in on the time it touched, stalled a hair short of proving it and said
/// nothing was in the way, so the move went 4 cm into the corner. This is
/// the place and move it failed on, to the bit.
#[test]
fn a_move_the_cast_misses_still_stops_at_the_corner() {
    let wall = [rect(-4.0, -4.0, -1.0, -1.0)];
    let mut flat = world(Vec2::new(-0.796_123_6, -0.716_439_8), &wall);
    let at = walk(&mut flat, Vec2::new(-0.171_593_64, -0.077_833_92), 1);
    assert_eq!(wall_it_is_inside(at, &wall), None, "inside at {at}");
    // Stopped a skin out at the corner, then slid on round it, which
    // takes it a little further off.
    let gap = at.distance(Vec2::new(-1.0, -1.0)) - 0.25;
    assert!((SKIN * 0.9..0.02).contains(&gap), "at the corner: {gap}");
    let last = flat.state().last_move(id(1)).unwrap();
    assert!(
        last.velocity.x < 0.0,
        "slid round it, still going: {last:?}"
    );
}

#[test]
fn a_character_put_inside_a_wall_can_still_walk_out() {
    // Checking where a move ends counts only what it didn't start in.
    let wall = [rect(-1.0, -1.0, 1.0, 1.0)];
    let mut flat = world(Vec2::new(0.5, 0.0), &wall);
    let at = walk(&mut flat, Vec2::new(0.1, 0.0), 20);
    assert!((at.x - 2.5).abs() < 1e-4, "walked out: {at}");
}

proptest! {
    /// Any walls and any moves: the character never ends inside a wall.
    #[test]
    fn a_character_never_ends_inside_a_wall(
        walls in prop::collection::vec((-8i32..8, -8i32..8, 1i32..4, 1i32..4), 0..12),
        moves in prop::collection::vec((-0.3f32..0.3, -0.3f32..0.3), 1..120),
    ) {
        let walls: Vec<(Vec2, Vec2)> = walls
            .iter()
            .map(|(x, y, w, h)| rect(*x as f32, *y as f32, (x + w) as f32, (y + h) as f32))
            .filter(|(lo, hi)| !(lo.x < 0.6 && hi.x > -0.6 && lo.y < 0.6 && hi.y > -0.6))
            .collect();
        let mut flat = world(Vec2::ZERO, &walls);
        for (x, y) in moves {
            flat.move_characters(&[(id(1), Vec2::new(x, y))], 1.0 / 60.0);
            let at = flat.state().place(id(1)).unwrap();
            prop_assert_eq!(wall_it_is_inside(at, &walls), None, "inside at {}", at);
        }
    }
}

/// Whether what Rapier was handed is in range: each entity's collider
/// within a kilometre of its `place`, and no bigger than two across.
fn colliders_in_range(flat: &Flat, place: Vec2) -> bool {
    flat.state.by_entity.values().all(|entry| {
        let c = &flat.state.colliders[entry.collider];
        let off = c.translation() - place;
        let size = c.shape().compute_local_aabb().extents();
        off.is_finite()
            && off.abs().max_element() <= 1000.0
            && size.is_finite()
            && size.max_element() <= 2000.0
    })
}

#[test]
fn a_collider_s_centre_and_size_are_held_in_range_in_2d_too() {
    // SEC-12's case on the 2D side: in 3D a centre of 1e30 on an entity
    // scaled 1e30 put a collider an endless way off. A 2D collider isn't
    // scaled with its entity, and its centre and size are each held to a
    // kilometre: this keeps it so.
    let place = Vec2::new(2.0, 3.0);
    let mut flat = world(place, &[rect(4.0, 2.0, 5.0, 4.0)]);
    let far = FlatCollider {
        shape: FlatShape::Box(Vec2::splat(1.0e30)),
        center: Vec2::new(1.0e30, -f32::MAX),
    };
    flat.add(id(1), place, far, true);
    assert!(colliders_in_range(&flat, place));
    let made = &flat.state.colliders[flat.state.by_entity[&id(1)].collider];
    assert_eq!(made.translation(), place + Vec2::new(1000.0, -1000.0));
    assert_eq!(flat.state().place(id(1)), Some(place));
    flat.move_characters(&[(id(1), Vec2::new(0.1, 0.0))], 1.0 / 60.0);
    assert_eq!(flat.state().wall_count(), 1, "2D physics carries on");
    assert!(flat.state().place(id(1)).unwrap().is_finite());
}

#[test]
fn no_centre_or_size_hands_2d_rapier_a_number_out_of_range() {
    // A sweep: every mix of extreme (but finite) centres and sizes, on
    // each shape, a character or not.
    let extremes = [0.0, 1.0e-30, 1.0, -2.5, 1.0e9, -1.0e19, 1.0e30, f32::MAX];
    let place = Vec2::new(-3.0, 1.5);
    for (i, &across) in extremes.iter().enumerate() {
        for (j, &up) in extremes.iter().enumerate() {
            for (k, &size) in extremes.iter().enumerate() {
                let collider = FlatCollider {
                    shape: match (i + j + k) % 2 {
                        0 => FlatShape::Circle(size),
                        _ => FlatShape::Box(Vec2::new(size, extremes[(k + 3) % 8])),
                    },
                    center: Vec2::new(across, -up),
                };
                let mut flat = world(Vec2::ZERO, &[rect(4.0, 2.0, 5.0, 4.0)]);
                flat.add(id(1), place, collider, (i + k) % 2 == 0);
                assert!(colliders_in_range(&flat, place), "{collider:?}");
                flat.move_characters(&[(id(1), Vec2::new(0.1, -0.1))], 1.0 / 60.0);
                assert_eq!(flat.state().wall_count(), 1, "{collider:?}");
                assert!(flat.state().place(id(1)).unwrap().is_finite());
            }
        }
    }
}
