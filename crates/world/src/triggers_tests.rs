//! The overlap pass: in, staying and out, in order, a teleport straight
//! through crossing nothing, once, saves, despawning, 2D and 3D, freeform
//! outlines, the checksum, and the same on one core or four.

use super::*;
use level::builtin::{CharacterBody, Collider2D, Transform};
use level::{EntityData, Level};

/// The zone's events as `(zone, who, in)`.
fn said(crossings: &[Crossing]) -> Vec<(EntityId, EntityId, bool)> {
    crossings
        .iter()
        .map(|c| (c.zone, c.who, c.entered))
        .collect()
}

fn at(x: f32, y: f32) -> Transform {
    Transform {
        position: Vec3::new(x, y, 0.0),
        ..Transform::default()
    }
}

/// A 2D character (a 0.25 m circle at 16 px a metre) at `x`, `y`.
fn walker(name: &str, x: f32, y: f32) -> EntityData {
    EntityData::new(name).with(&at(x, y)).with(&Collider2D {
        radius: 4.0,
        character: true,
        ..Collider2D::default()
    })
}

/// A box zone 2 m square at (5, 0), and the level it's in.
fn with_door(fires: Fires) -> (Level, EntityId) {
    let mut level = Level::new();
    let door = level.spawn(
        EntityData::new("Door")
            .with(&at(5.0, 0.0))
            .with(&TriggerZone {
                fires,
                ..TriggerZone::default()
            }),
    );
    (level, door)
}

/// One tick: each walker moves by `step`, physics steps, then the pass.
fn tick(world: &mut World, walkers: &[EntityId], step: Vec3) -> Vec<Crossing> {
    for &who in walkers {
        world.move_character(who, step);
    }
    world.step_physics();
    world.overlap_zones()
}

#[test]
fn a_walker_goes_in_stays_and_comes_out() {
    let (mut level, door) = with_door(Fires::EveryTime);
    let farmer = level.spawn(walker("Farmer", 0.0, 0.0));
    let mut world = World::from_level(&level);
    assert!(world.overlap_zones().is_empty(), "outside at the start");
    let mut heard = Vec::new();
    for n in 0..60 {
        for c in tick(&mut world, &[farmer], Vec3::new(0.25, 0.0, 0.0)) {
            heard.push((n, c.entered));
        }
        if n == 20 {
            assert_eq!(world.inside(door), [farmer], "inside, staying");
        }
    }
    // The zone spans x 4 to 6; the farmer's edge reaches 4 after
    // 3.75 m, its far edge leaves 6 after 6.25 m: ticks 14 and 25.
    assert_eq!(heard, [(14, true), (25, false)]);
    assert!(world.inside(door).is_empty());
}

#[test]
fn two_entering_in_the_same_tick_arrive_in_order() {
    let (mut level, door) = with_door(Fires::EveryTime);
    let a = level.spawn(walker("A", 3.0, 0.5));
    let b = level.spawn(walker("B", 3.0, -0.5));
    let mut world = World::from_level(&level);
    let crossings = tick(&mut world, &[b, a], Vec3::new(1.0, 0.0, 0.0));
    let mut both = [a, b];
    both.sort_unstable();
    assert_eq!(
        said(&crossings),
        [(door, both[0], true), (door, both[1], true)]
    );
}

#[test]
fn going_in_and_out_within_one_tick_fires_nothing() {
    let (mut level, door) = with_door(Fires::EveryTime);
    let farmer = level.spawn(walker("Farmer", 0.0, 0.0));
    let mut world = World::from_level(&level);
    // A teleport straight through: before the zone, then past it.
    world.set_transform(farmer, at(5.0, 0.0));
    world.set_transform(farmer, at(10.0, 0.0));
    let crossings = tick(&mut world, &[], Vec3::ZERO);
    assert!(crossings.is_empty(), "{crossings:?}");
    assert!(world.inside(door).is_empty());
}

#[test]
fn a_zone_that_fires_once_fires_only_the_first_time() {
    let (mut level, door) = with_door(Fires::Once);
    let farmer = level.spawn(walker("Farmer", 0.0, 0.0));
    let mut world = World::from_level(&level);
    let mut heard = Vec::new();
    for step in [1.0, -1.0, 1.0] {
        for _ in 0..10 {
            heard.extend(tick(&mut world, &[farmer], Vec3::new(step, 0.0, 0.0)));
        }
    }
    assert_eq!(said(&heard), [(door, farmer, true)]);
    // It still knows who is inside, for Inside now and inside().
    world.set_transform(farmer, at(5.0, 0.0));
    tick(&mut world, &[], Vec3::ZERO);
    assert_eq!(world.inside(door), [farmer]);
}

#[test]
fn once_holds_across_a_save() {
    let (mut level, door) = with_door(Fires::Once);
    let farmer = level.spawn(walker("Farmer", 5.0, 0.0));
    let mut world = World::from_level(&level);
    assert_eq!(said(&world.overlap_zones()), [(door, farmer, true)]);
    let (inside, spent) = world.zone_states();
    assert_eq!(spent, [door]);
    // Loaded, it walks out and in again: still nothing.
    let mut loaded = World::from_level(&level);
    loaded.set_zone_states(&inside, &spent);
    let mut heard = Vec::new();
    for step in [1.0, -1.0] {
        for _ in 0..5 {
            heard.extend(tick(&mut loaded, &[farmer], Vec3::new(step, 0.0, 0.0)));
        }
    }
    assert!(heard.is_empty(), "{heard:?}");
}

#[test]
fn saved_while_inside_and_loaded_says_nothing_again() {
    let (mut level, door) = with_door(Fires::EveryTime);
    let farmer = level.spawn(walker("Farmer", 5.0, 0.0));
    let mut world = World::from_level(&level);
    assert_eq!(said(&world.overlap_zones()), [(door, farmer, true)]);
    let (inside, spent) = world.zone_states();
    let mut loaded = World::from_level(&level);
    loaded.set_zone_states(&inside, &spent);
    assert_eq!(loaded.inside(door), [farmer], "read before the pass");
    assert!(loaded.overlap_zones().is_empty(), "no second Entered");
    assert_eq!(loaded.checksum(), world.checksum());
}

#[test]
fn rebuilding_the_world_keeps_who_is_inside() {
    let (mut level, door) = with_door(Fires::EveryTime);
    let farmer = level.spawn(walker("Farmer", 5.0, 0.0));
    let mut world = World::from_level(&level);
    world.overlap_zones();
    // An edit while playing builds the world and its physics again:
    // nobody leaves and comes back.
    world.rebuild(&level);
    assert!(world.overlap_zones().is_empty());
    assert_eq!(world.inside(door), [farmer]);
}

#[test]
fn something_despawned_inside_is_heard_leaving() {
    let (mut level, door) = with_door(Fires::EveryTime);
    let farmer = level.spawn(walker("Farmer", 5.0, 0.0));
    let mut world = World::from_level(&level);
    world.overlap_zones();
    world.remove_subtrees(&[farmer]);
    assert_eq!(said(&world.overlap_zones()), [(door, farmer, false)]);
}

#[test]
fn a_zone_despawned_says_nothing() {
    let (mut level, door) = with_door(Fires::EveryTime);
    level.spawn(walker("Farmer", 5.0, 0.0));
    let mut world = World::from_level(&level);
    world.overlap_zones();
    world.remove_subtrees(&[door]);
    assert!(world.overlap_zones().is_empty());
    assert!(world.zone_states().0.is_empty());
}

#[test]
fn a_3d_character_walks_into_a_box_and_a_sphere() {
    let mut level = Level::new();
    let room = level.spawn(
        EntityData::new("Room")
            .with(&at(5.0, 0.0))
            .with(&TriggerZone {
                size: Vec3::new(2.0, 2.0, 4.0),
                ..TriggerZone::default()
            }),
    );
    let pool = level.spawn(
        EntityData::new("Pool")
            .with(&at(10.0, 0.0))
            .with(&TriggerZone {
                form: ZoneForm::Round,
                radius: 1.0,
                ..TriggerZone::default()
            }),
    );
    let player = level.spawn(
        EntityData::new("Player")
            .with(&at(0.0, 0.0))
            .with(&CharacterBody::default()),
    );
    let mut world = World::from_level(&level);
    let mut heard = Vec::new();
    for _ in 0..80 {
        heard.extend(tick(&mut world, &[player], Vec3::new(0.25, 0.0, 0.0)));
    }
    let order: Vec<(EntityId, bool)> = heard.iter().map(|c| (c.zone, c.entered)).collect();
    assert_eq!(
        order,
        [(room, true), (room, false), (pool, true), (pool, false)]
    );
}

#[test]
fn a_freeform_zone_is_exact_about_its_notch() {
    // An L 4 m across from (0, 0), its notch top-right.
    let points = [
        (0.0, 0.0),
        (4.0, 0.0),
        (4.0, 2.0),
        (2.0, 2.0),
        (2.0, 4.0),
        (0.0, 4.0),
    ]
    .into_iter()
    .map(|(x, y)| Vec2::new(x, y))
    .collect();
    let mut level = Level::new();
    let yard = level.spawn(
        EntityData::new("Yard")
            .with(&at(0.0, 0.0))
            .with(&TriggerZone {
                form: ZoneForm::Freeform,
                points,
                ..TriggerZone::default()
            }),
    );
    let in_notch = level.spawn(walker("In the notch", 3.0, 3.0));
    let in_arm = level.spawn(walker("In an arm", 3.0, 1.0));
    let mut world = World::from_level(&level);
    world.overlap_zones();
    assert_eq!(world.inside(yard), [in_arm]);
    assert!(!world.inside(yard).contains(&in_notch));
}

#[test]
fn a_scaled_and_turned_zone_grows_and_turns_with_its_entity() {
    let mut level = Level::new();
    let zone = level.spawn(
        EntityData::new("Long")
            .with(&Transform {
                position: Vec3::new(5.0, 0.0, 0.0),
                rotation: glam::Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
                scale: Vec3::new(1.0, 3.0, 1.0),
            })
            .with(&TriggerZone {
                size: Vec3::new(1.0, 2.0, 1.0),
                ..TriggerZone::default()
            }),
    );
    // Turned a quarter round, its 6 m length lies along X: 2 to 8.
    let near = level.spawn(walker("Near", 2.5, 0.0));
    let far = level.spawn(walker("Far", 5.0, 2.0));
    let mut world = World::from_level(&level);
    world.overlap_zones();
    assert_eq!(world.inside(zone), [near]);
    assert!(!world.inside(zone).contains(&far));
}

#[test]
fn who_is_inside_is_in_the_checksum() {
    let (mut level, _) = with_door(Fires::EveryTime);
    let farmer = level.spawn(walker("Farmer", 0.0, 0.0));
    let mut world = World::from_level(&level);
    world.overlap_zones();
    let outside = world.checksum();
    world.set_transform(farmer, at(5.0, 0.0));
    world.step_physics();
    let moved = world.checksum();
    world.overlap_zones();
    assert_ne!(world.checksum(), moved, "inside counts");
    assert_ne!(moved, outside);
}

#[test]
fn many_walkers_among_many_zones_are_the_same_on_one_core_or_four() {
    let mut level = Level::new();
    for n in 0..40 {
        level.spawn(
            EntityData::new(&format!("Zone {n}"))
                .with(&at((n % 8) as f32 * 3.0, (n / 8) as f32 * 3.0))
                .with(&TriggerZone::default()),
        );
    }
    let walkers: Vec<EntityId> = (0..120)
        .map(|n| {
            level.spawn(walker(
                &format!("W{n}"),
                (n % 12) as f32 * 2.0,
                (n / 12) as f32,
            ))
        })
        .collect();
    let run = |threads: usize| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(|| {
                let mut world = World::from_level(&level);
                let mut heard = Vec::new();
                for t in 0..60 {
                    let step = Vec3::new(0.1, if t % 20 < 10 { 0.15 } else { -0.15 }, 0.0);
                    heard.extend(tick(&mut world, &walkers, step));
                }
                (heard, world.checksum())
            })
    };
    let one = run(1);
    let four = run(4);
    assert!(one.0.len() > 50, "{} crossings", one.0.len());
    assert_eq!(one, four);
}

/// A level of `zones` zones in a grid (boxes, circles and freeform Ls in
/// turn) and `walkers` 2D characters spread among them.
fn crowded(zones: usize, walkers: usize) -> (World, Vec<EntityId>) {
    let mut level = Level::new();
    let ell: Vec<Vec2> = [
        (-1.5, -1.5),
        (1.5, -1.5),
        (1.5, 0.0),
        (0.0, 0.0),
        (0.0, 1.5),
        (-1.5, 1.5),
    ]
    .into_iter()
    .map(|(x, y)| Vec2::new(x, y))
    .collect();
    let across = (zones as f32).sqrt().ceil() as usize;
    for n in 0..zones {
        let zone = match n % 3 {
            0 => TriggerZone::default(),
            1 => TriggerZone {
                form: ZoneForm::Round,
                radius: 1.5,
                ..TriggerZone::default()
            },
            _ => TriggerZone {
                form: ZoneForm::Freeform,
                points: ell.clone(),
                ..TriggerZone::default()
            },
        };
        let at = Vec2::new((n % across) as f32, (n / across) as f32) * 5.0;
        level.spawn(
            EntityData::new(&format!("Zone {n}"))
                .with(&at_place(at))
                .with(&zone),
        );
    }
    let side = across as f32 * 5.0;
    let ids = (0..walkers)
        .map(|n| {
            let x = (n as f32 * 7.31) % side;
            let y = (n as f32 * 3.17) % side;
            level.spawn(walker(&format!("W{n}"), x, y))
        })
        .collect();
    (World::from_level(&level), ids)
}

fn at_place(p: Vec2) -> Transform {
    at(p.x, p.y)
}

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
fn scale_check_2000_characters_among_500_zones() {
    // Stage 1's scale case: 2,000 characters walking among 500 zones (a
    // third of them freeform), the overlap pass once a tick. Its share of
    // a 60th of a second.
    const TARGET_MS: f64 = 2.0;
    let (mut world, walkers) = crowded(500, 2_000);
    let ticks = 120;
    let mut pass = std::time::Duration::ZERO;
    let mut crossings = 0;
    for t in 0..ticks {
        let way = if t % 60 < 30 { 1.0 } else { -1.0 };
        for (i, &who) in walkers.iter().enumerate() {
            let turn = if i % 2 == 0 { way } else { -way };
            world.move_character(who, Vec3::new(0.08 * turn, 0.05 * way, 0.0));
        }
        world.step_physics();
        let started = std::time::Instant::now();
        crossings += world.overlap_zones().len();
        pass += started.elapsed();
    }
    let ms = pass.as_secs_f64() * 1000.0 / f64::from(ticks);
    let verdict = if ms <= TARGET_MS { "ok" } else { "SLOW" };
    println!(
        "Scale check: the overlap pass, 2,000 characters among 500 zones {ms:.3} ms a tick \
         ({crossings} crossings; target {TARGET_MS} ms) {verdict}"
    );
    assert!(crossings > 0, "they walk in and out");
    assert!(ms <= TARGET_MS, "the overlap pass takes {ms:.3} ms a tick");
}
