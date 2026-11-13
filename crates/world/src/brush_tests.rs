//! Brushes in the running game: a Solid one is something characters
//! stand on and bodies rest on; one that isn't Solid is passed through.

use crate::World;
use area::builtin::{Brush, CharacterBody, RigidBody, Transform};
use area::{Area, EntityData};
use glam::{Quat, Vec3};
use reflect::{Component, EntityId};

fn id(n: u64) -> EntityId {
    EntityId(reflect::Id::from_raw(n).unwrap())
}

fn at(x: f32, y: f32, z: f32) -> Transform {
    Transform {
        position: Vec3::new(x, y, z),
        ..Transform::default()
    }
}

/// An area with a box brush 20 m across and 1 m tall (its top at 0,
/// Solid or not), and a character standing 2 m above it.
fn brush_floor(solid: bool) -> Area {
    let mut area = Area::new();
    let floor = Brush {
        solid,
        ..Brush::default()
    };
    area.insert(
        id(1),
        EntityData::new("Floor")
            .with(&Transform {
                position: Vec3::new(0.0, 0.0, -1.0),
                scale: Vec3::new(10.0, 10.0, 0.5),
                ..Transform::default()
            })
            .with(&floor),
    );
    area.insert(
        id(2),
        EntityData::new("Walker")
            .with(&at(0.0, 0.0, 2.0))
            .with(&CharacterBody::default()),
    );
    area
}

fn feet(world: &World) -> Vec3 {
    world.global(id(2)).unwrap().w_axis.truncate()
}

fn walk(world: &mut World, speed: Vec3, ticks: usize) {
    let mut fall = 0.0f32;
    for _ in 0..ticks {
        let grounded = world
            .physics()
            .state()
            .character(id(2))
            .is_some_and(|c| c.grounded);
        fall = if grounded { 0.0 } else { fall - 9.81 / 60.0 };
        world.move_character(id(2), (speed + Vec3::Z * fall) / 60.0);
        world.step_physics();
    }
}

#[test]
fn a_character_stands_and_walks_on_a_solid_brush() {
    let mut world = World::from_area(&brush_floor(true));
    walk(&mut world, Vec3::ZERO, 90);
    assert!(feet(&world).z.abs() < 0.05, "standing at {}", feet(&world));
    walk(&mut world, Vec3::new(2.0, 0.0, 0.0), 60);
    let walked = feet(&world);
    assert!(
        walked.x > 1.5 && walked.z.abs() < 0.05,
        "walked to {walked}"
    );
}

#[test]
fn a_brush_that_isnt_solid_is_passed_through() {
    let mut world = World::from_area(&brush_floor(false));
    assert_eq!(world.physics().state().len(), 1, "only the character");
    walk(&mut world, Vec3::ZERO, 90);
    assert!(feet(&world).z < -0.5, "fell through to {}", feet(&world));
}

#[test]
fn a_solid_brush_never_falls_even_with_a_body() {
    let mut area = brush_floor(true);
    area.get_mut(id(1)).unwrap().set(&RigidBody::default());
    let mut world = World::from_area(&area);
    for _ in 0..60 {
        world.step_physics();
    }
    let floor = world.physics().state().pose(id(1)).unwrap();
    assert_eq!(floor, (Vec3::new(0.0, 0.0, -1.0), Quat::IDENTITY));
}

#[test]
fn a_brush_made_solid_while_playing_joins_physics() {
    let mut area = brush_floor(false);
    let mut world = World::from_area(&area);
    assert_eq!(world.physics().state().len(), 1);
    let entity = area.get_mut(id(1)).unwrap();
    let mut brush = entity.get::<Brush>().unwrap();
    brush.solid = true;
    entity.set(&brush);
    world.update(&area, [(id(1), false)]);
    assert_eq!(world.physics().state().len(), 2);
    assert!(world.component_data(id(1), Brush::NAME).is_some());
}

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
#[allow(clippy::print_stdout)]
fn scale_check_playing_on_10000_solid_brushes() {
    // Play with 10,000 box brushes, each its own size: the world built
    // (every brush a physics surface), then ticks of physics with a
    // character walking over them.
    const BUILD_MS: f64 = 1_000.0;
    const TICK_MS: f64 = 4.0;
    let mut area = Area::new();
    for i in 0..10_000u64 {
        let size = Vec3::new(
            1.0 + (i % 100) as f32 * 0.01,
            1.0 + (i / 100) as f32 * 0.01,
            1.0,
        );
        let brush = Brush {
            mesh: std::sync::Arc::new(mesh::Shape::Box.build(&mesh::ShapeSettings {
                size,
                ..mesh::Shape::Box.standard()
            })),
            ..Brush::default()
        };
        area.insert(
            id(10 + i),
            EntityData::new("Box")
                .with(&at((i % 100) as f32 * 3.0, (i / 100) as f32 * 3.0, -1.0))
                .with(&brush),
        );
    }
    area.insert(
        id(2),
        EntityData::new("Walker")
            .with(&at(0.0, 0.0, 2.0))
            .with(&CharacterBody::default()),
    );
    let start = std::time::Instant::now();
    let mut world = World::from_area(&area);
    let build_ms = start.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(world.physics().state().len(), 10_001);
    let ticks = 120;
    let start = std::time::Instant::now();
    walk(&mut world, Vec3::new(3.0, 0.0, 0.0), ticks);
    let tick_ms = start.elapsed().as_secs_f64() * 1000.0 / ticks as f64;
    let verdict = |ms: f64, target: f64| if ms <= target { "ok" } else { "SLOW" };
    println!(
        "Scale check: Play with 10,000 solid brushes: the world built {build_ms:.0} ms (target \
         {BUILD_MS} ms) {}; a tick of physics, a character walking on them {tick_ms:.2} ms \
         (target {TICK_MS} ms) {}",
        verdict(build_ms, BUILD_MS),
        verdict(tick_ms, TICK_MS)
    );
    assert!(build_ms <= BUILD_MS && tick_ms <= TICK_MS);
}
