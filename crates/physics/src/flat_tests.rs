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
        one_way: false,
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

/// N-10: two characters starting on the very same spot, both walking the
/// same way, end beside one another near where walking takes them, not
/// flung apart.
#[test]
fn two_characters_on_one_spot_walking_stay_together() {
    for step in [Vec2::new(0.05, 0.0), Vec2::new(-0.03, 0.04)] {
        let mut flat = Flat::new();
        flat.add(id(1), Vec2::ZERO, circle(0.25), true);
        flat.add(id(2), Vec2::ZERO, circle(0.25), true);
        for _ in 0..120 {
            flat.move_characters(&[(id(1), step), (id(2), step)], 1.0 / 60.0);
        }
        let walked_to = step * 120.0;
        let places = [id(1), id(2)].map(|who| flat.state().place(who).unwrap());
        for at in places {
            let off = (at - walked_to).length();
            assert!(
                off < 1.0,
                "{step}: near where it walked to ({walked_to}): {at}"
            );
        }
        let apart = (places[0] - places[1]).length();
        assert!(apart > 0.2, "{step}: pushed apart: {apart}");
    }
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
        one_way: false,
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
            one_way: false,
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
            one_way: false,
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
        one_way: false,
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
                    one_way: false,
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

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
#[allow(clippy::print_stdout)]
fn scale_check_500_flat_characters_walking() {
    // The second review pass: no case walked 2D characters, so a slower
    // corner check (NEW-09's) would go unseen. A 64 × 64 tile map whose
    // walls run every 8 tiles with gaps (many corners), in chunks of 16 ×
    // 16 tiles as a map gives them, and 500 characters each walking its
    // own way into them for two seconds.
    const TARGET_MS: f64 = 8.0;
    const SIDE: u32 = 64;
    let wall = |x: u32, y: u32| {
        x == 0
            || y == 0
            || x == SIDE - 1
            || y == SIDE - 1
            || ((x.is_multiple_of(8) || y.is_multiple_of(8)) && !(x + y).is_multiple_of(3))
    };
    let mut flat = Flat::new();
    for cy in 0..SIDE / 16 {
        for cx in 0..SIDE / 16 {
            let mut rects = Vec::new();
            for y in cy * 16..cy * 16 + 16 {
                for x in cx * 16..cx * 16 + 16 {
                    if wall(x, y) {
                        rects.push(rect(x as f32, y as f32, x as f32 + 1.0, y as f32 + 1.0));
                    }
                }
            }
            flat.set_walls(id(9_999), [cx, cy], &rects);
        }
    }
    let open: Vec<(u32, u32)> = (0..SIDE * SIDE)
        .map(|i| (i % SIDE, i / SIDE))
        .filter(|&(x, y)| !wall(x, y))
        .collect();
    let walkers: Vec<(EntityId, Vec2)> = (0..500_u64)
        .map(|i| {
            let (x, y) = open[(i as usize * 7) % open.len()];
            flat.add(
                id(i + 1),
                Vec2::new(x as f32 + 0.5, y as f32 + 0.5),
                circle(0.25),
                true,
            );
            // Each its own way (the golden angle apart), 4 m a second.
            let angle = i as f32 * 2.399_963;
            (id(i + 1), Vec2::new(angle.cos(), angle.sin()) * 4.0 / 60.0)
        })
        .collect();
    let ticks = 120;
    let start = std::time::Instant::now();
    for _ in 0..ticks {
        flat.move_characters(&walkers, 1.0 / 60.0);
    }
    let ms = start.elapsed().as_secs_f64() * 1000.0 / f64::from(ticks);
    assert!(!flat.take_broke());
    // They met the walls: none went through to the far side of the map.
    for (who, _) in &walkers {
        let at = flat.state().place(*who).unwrap();
        assert!((1.0..SIDE as f32 - 1.0).contains(&at.x), "{at}");
        assert!((1.0..SIDE as f32 - 1.0).contains(&at.y), "{at}");
    }
    let verdict = if ms <= TARGET_MS { "ok" } else { "SLOW" };
    println!(
        "Scale check: 500 2D characters walking into a tile map's walls, physics alone: \
         {ms:.2} ms a tick (target {TARGET_MS} ms) {verdict}"
    );
    assert!(ms <= TARGET_MS, "{ms:.2} ms a tick");
}

/// A one-way platform 4 m wide, 0.5 m thick, its top at `top` (entity 5).
fn one_way_platform(flat: &mut Flat, top: f32) {
    flat.add(
        id(5),
        Vec2::new(0.0, top - 0.25),
        FlatCollider {
            shape: FlatShape::Box(Vec2::new(4.0, 0.5)),
            center: Vec2::ZERO,
            one_way: true,
        },
        false,
    );
}

#[test]
fn a_character_jumps_up_through_a_one_way_platform_and_lands_on_it() {
    let mut flat = world(Vec2::ZERO, &[]);
    one_way_platform(&mut flat, 2.0);
    // Up through it from below: nothing stops it.
    let at = walk(&mut flat, Vec2::new(0.0, 0.1), 40);
    assert!((at.y - 4.0).abs() < 1e-4, "passed up through: {at}");
    // Down onto it: it lands on top, grounded.
    let at = walk(&mut flat, Vec2::new(0.0, -0.1), 40);
    assert!(
        at.y - 0.25 >= 2.0 && at.y - 0.25 < 2.0 + 2.0 * SKIN,
        "landed on top: {at}"
    );
    assert!(flat.state().last_move(id(1)).unwrap().grounded);
    // Walking along it, pressed down as a platformer does, it stays on.
    let at = walk(&mut flat, Vec2::new(0.02, -0.02), 50);
    assert!(at.y - 0.25 >= 2.0 && at.x > 0.9, "walked along it: {at}");
}

#[test]
fn a_one_way_platform_lets_a_character_fall_back_down_from_inside_it() {
    // Jumped up into it but not past its top: coming down, it falls out
    // the bottom rather than landing halfway inside.
    let mut flat = world(Vec2::new(0.0, 1.9), &[]);
    one_way_platform(&mut flat, 2.0);
    let at = walk(&mut flat, Vec2::new(0.0, -0.1), 20);
    assert!(at.y < 0.0, "fell through: {at}");
}

#[test]
fn a_one_way_platform_is_passed_through_from_the_side() {
    let mut flat = world(Vec2::new(-3.0, 1.75), &[]);
    one_way_platform(&mut flat, 2.0);
    let at = walk(&mut flat, Vec2::new(0.1, 0.0), 60);
    assert!(at.x > 2.9, "walked through it sideways: {at}");
}

#[test]
fn a_one_way_platform_taken_out_stops_nothing() {
    let mut flat = world(Vec2::new(0.0, 3.0), &[]);
    one_way_platform(&mut flat, 2.0);
    flat.remove(id(5));
    assert!(flat.state().one_way.is_empty());
    let at = walk(&mut flat, Vec2::new(0.0, -0.1), 40);
    assert!(at.y < 0.0, "nothing to land on: {at}");
    // Added again as an ordinary solid, it stops from below too.
    flat.add(
        id(5),
        Vec2::new(0.0, 1.75),
        FlatCollider {
            shape: FlatShape::Box(Vec2::new(4.0, 0.5)),
            center: Vec2::ZERO,
            one_way: false,
        },
        false,
    );
    let at = walk(&mut flat, Vec2::new(0.0, 0.1), 40);
    assert!(at.y < 1.5 - 0.25 + 1e-3, "stopped below the solid: {at}");
}

/// A solid platform 4 m wide, its top at y = 0 (entity 6), and a
/// character standing on it (entity 1), settled by one move down.
fn on_a_platform() -> Flat {
    let mut flat = world(Vec2::new(0.0, 0.25 + SKIN), &[]);
    flat.add(
        id(6),
        Vec2::new(0.0, -0.25),
        FlatCollider {
            shape: FlatShape::Box(Vec2::new(4.0, 0.5)),
            center: Vec2::ZERO,
            one_way: false,
        },
        false,
    );
    walk(&mut flat, Vec2::new(0.0, -0.02), 1);
    assert!(flat.state().last_move(id(1)).unwrap().grounded);
    flat
}

/// The platform moved by `by` each tick for `ticks` ticks, the character
/// walked by `step` (or not at all when `None`) as it goes.
fn ride(flat: &mut Flat, by: Vec2, step: Option<Vec2>, ticks: usize) -> Vec2 {
    for _ in 0..ticks {
        let to = flat.state().place(id(6)).unwrap() + by;
        flat.set_place(id(6), to);
        let moves: Vec<(EntityId, Vec2)> = step.map(|s| (id(1), s)).into_iter().collect();
        flat.move_characters(&moves, 1.0 / 60.0);
    }
    flat.state().place(id(1)).unwrap()
}

#[test]
fn a_moving_platform_carries_the_character_standing_on_it() {
    let mut flat = on_a_platform();
    let at = ride(
        &mut flat,
        Vec2::new(0.02, 0.0),
        Some(Vec2::new(0.0, -0.02)),
        50,
    );
    assert!((at.x - 1.0).abs() < 1e-3, "carried a metre: {at}");
    assert!(
        at.y >= 0.25 && at.y < 0.25 + 2.0 * SKIN,
        "still on top: {at}"
    );
    assert!(flat.state().last_move(id(1)).unwrap().grounded);
    assert_eq!(flat.moved().last().map(|m| m.0), Some(id(1)));
}

#[test]
fn a_character_not_walked_is_carried_too() {
    let mut flat = on_a_platform();
    let at = ride(&mut flat, Vec2::new(-0.02, 0.0), None, 50);
    assert!((at.x + 1.0).abs() < 1e-3, "carried a metre left: {at}");
    assert_eq!(flat.moved(), vec![(id(1), at)], "the world follows it");
}

#[test]
fn a_platform_going_up_lifts_it_and_going_down_keeps_it_on() {
    let mut flat = on_a_platform();
    let at = ride(
        &mut flat,
        Vec2::new(0.0, 0.03),
        Some(Vec2::new(0.0, -0.02)),
        40,
    );
    assert!(
        at.y >= 1.2 + 0.25 && at.y < 1.2 + 0.25 + 3.0 * SKIN,
        "lifted: {at}"
    );
    let at = ride(
        &mut flat,
        Vec2::new(0.0, -0.03),
        Some(Vec2::new(0.0, -0.02)),
        60,
    );
    assert!(
        at.y >= -0.6 + 0.25 && at.y < -0.6 + 0.25 + 3.0 * SKIN,
        "came down with it: {at}"
    );
    assert!(flat.state().last_move(id(1)).unwrap().grounded);
}

#[test]
fn off_the_platform_it_is_carried_no_more() {
    let mut flat = on_a_platform();
    flat.set_walls(id(9), [0, 0], &[rect(-20.0, -1.0, -2.0, 0.0)]);
    // A jump: in the air, it isn't standing on the platform.
    walk(&mut flat, Vec2::new(0.0, 0.1), 3);
    let at = ride(&mut flat, Vec2::new(0.05, 0.0), Some(Vec2::ZERO), 20);
    assert!(at.x.abs() < 1e-4, "not carried while in the air: {at}");
    // Walked off onto the ground beside it, it isn't carried either.
    let mut flat = on_a_platform();
    flat.set_walls(id(9), [0, 0], &[rect(-20.0, -1.0, -2.0, 0.0)]);
    walk(&mut flat, Vec2::new(-0.1, -0.02), 30);
    let before = flat.state().place(id(1)).unwrap();
    assert!(before.x < -2.5, "on the ground: {before}");
    let at = ride(
        &mut flat,
        Vec2::new(0.05, 0.0),
        Some(Vec2::new(0.0, -0.02)),
        20,
    );
    assert!((at.x - before.x).abs() < 1e-4, "stays put: {at}");
}

#[test]
fn a_platform_taken_out_carries_nothing() {
    let mut flat = on_a_platform();
    flat.set_place(id(6), Vec2::new(1.0, -0.25));
    flat.remove(id(6));
    flat.move_characters(&[], 1.0 / 60.0);
    assert_eq!(flat.state().place(id(1)).unwrap().x, 0.0);
    assert!(flat.state().shifted.is_empty());
}

#[test]
fn a_ray_meets_walls_and_colliders_on_the_ground() {
    let mut flat = world(Vec2::new(0.0, 0.0), &[rect(3.0, -5.0, 4.0, 5.0)]);
    flat.add(
        id(6),
        Vec2::new(0.0, -2.0),
        FlatCollider {
            shape: FlatShape::Box(Vec2::new(2.0, 1.0)),
            center: Vec2::ZERO,
            one_way: false,
        },
        false,
    );
    flat.move_characters(&[], 1.0 / 60.0);
    let state = flat.state();
    // Right from the character's middle: it passes out of itself and meets
    // the map's wall, the map's entity.
    let hit = state
        .cast_ray(glam::Vec3::new(0.0, 0.0, 0.7), glam::Vec3::X, 10.0)
        .unwrap();
    assert_eq!(hit.entity, id(9));
    assert!((hit.distance - 3.0).abs() < 1e-4, "{}", hit.distance);
    assert_eq!(hit.normal, glam::Vec3::new(-1.0, 0.0, 0.0));
    assert!(
        (hit.point - glam::Vec3::new(3.0, 0.0, 0.7)).length() < 1e-4,
        "keeps its height"
    );
    // Down: the solid box, its top facing up.
    let hit = state
        .cast_ray(glam::Vec3::ZERO, glam::Vec3::new(0.0, -1.0, 5.0), 10.0)
        .unwrap();
    assert_eq!(hit.entity, id(6));
    assert!((hit.distance - 1.5).abs() < 1e-4, "{}", hit.distance);
    assert_eq!(hit.normal, glam::Vec3::Y);
    // Too short, nowhere to go, or not a number: nothing.
    assert!(
        state
            .cast_ray(glam::Vec3::ZERO, glam::Vec3::X, 2.0)
            .is_none()
    );
    assert!(
        state
            .cast_ray(glam::Vec3::ZERO, glam::Vec3::Z, 10.0)
            .is_none()
    );
    assert!(
        state
            .cast_ray(glam::Vec3::NAN, glam::Vec3::X, 10.0)
            .is_none()
    );
    assert!(
        state
            .cast_ray(glam::Vec3::ZERO, glam::Vec3::X, -1.0)
            .is_none()
    );
}

#[test]
fn a_ray_meets_a_one_way_platform_only_coming_down_onto_it() {
    let mut flat = world(Vec2::new(-10.0, 0.0), &[]);
    one_way_platform(&mut flat, 2.0);
    flat.move_characters(&[], 1.0 / 60.0);
    let state = flat.state();
    let down = state
        .cast_ray(glam::Vec3::new(0.0, 3.0, 0.0), -glam::Vec3::Y, 5.0)
        .unwrap();
    assert_eq!(down.entity, id(5));
    assert!((down.distance - 1.0).abs() < 1e-4);
    assert!(
        state
            .cast_ray(glam::Vec3::new(0.0, 0.0, 0.0), glam::Vec3::Y, 5.0)
            .is_none()
    );
    assert!(
        state
            .cast_ray(glam::Vec3::new(-5.0, 1.8, 0.0), glam::Vec3::X, 10.0)
            .is_none()
    );
}
