//! The ground in the running game: characters stand and walk on it once
//! its chunks' colliders arrive, are held at the edge of a chunk whose
//! collider is still being built, and playing leaves the editor's ground
//! as it was.

use crate::World;
use glam::{IVec3, UVec3, Vec3};
use level::builtin::{CharacterBody, Transform};
use level::{EntityData, Level};
use reflect::EntityId;
use std::sync::Arc;

fn id(n: u64) -> EntityId {
    EntityId(reflect::Id::from_raw(n).unwrap())
}

/// A level with a character standing 2 m above where the ground will be
/// (10 m up) at `x`, 16 m along.
fn walker_at(x: f32) -> Level {
    let mut level = Level::new();
    level.insert(id(1), EntityData::new("Ground"));
    level.insert(
        id(2),
        EntityData::new("Walker")
            .with(&Transform {
                position: Vec3::new(x, 16.0, 12.0),
                ..Transform::default()
            })
            .with(&CharacterBody::default()),
    );
    level
}

/// A flat ground two chunks across, 10 m high, and every chunk that has
/// triangles with its collider, made as a worker would.
fn flat() -> (Arc<ground::Ground>, Vec<(IVec3, Option<physics::PreparedShape>)>) {
    let g = Arc::new(ground::Start::Flat { height: 10.0 }.ground(UVec3::new(2, 1, 1)));
    let mut samples = ground::Samples::default();
    let shapes = ground::store::keys(g.extent())
        .map(|key| {
            assert!(g.gather(key, &mut samples).known().is_some());
            let mesh = ground::smooth::mesh(&samples);
            (key, crate::prepare_ground_chunk(&mesh))
        })
        .collect();
    (g, shapes)
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

fn asset() -> reflect::AssetId {
    reflect::AssetId(reflect::Id::from_raw(77).unwrap())
}

fn needed(shapes: &[(IVec3, Option<physics::PreparedShape>)]) -> Vec<IVec3> {
    shapes.iter().map(|(k, _)| *k).collect()
}

#[test]
fn a_character_stands_and_walks_on_the_ground() {
    let (g, shapes) = flat();
    let mut world = World::from_level(&walker_at(10.0));
    world.set_ground(Arc::clone(&g), (id(1), asset(), true), &needed(&shapes), shapes);
    assert!(world.ground_is_solid());
    walk(&mut world, Vec3::ZERO, 90);
    assert!((feet(&world).z - 10.0).abs() < 0.05, "standing at {}", feet(&world));
    walk(&mut world, Vec3::new(4.0, 0.0, 0.0), 120);
    let walked = feet(&world);
    assert!(walked.x > 35.0, "across the seam to {walked}");
    assert!((walked.z - 10.0).abs() < 0.05, "{walked}");
}

#[test]
fn a_character_is_held_at_a_chunk_whose_collider_is_still_coming() {
    let (g, mut shapes) = flat();
    let second = shapes.remove(1);
    let mut world = World::from_level(&walker_at(20.0));
    let all = vec![shapes[0].0, second.0];
    world.set_ground(Arc::clone(&g), (id(1), asset(), true), &all, shapes);
    assert!(!world.ground_is_solid());
    walk(&mut world, Vec3::ZERO, 60);
    walk(&mut world, Vec3::new(4.0, 0.0, 0.0), 180);
    let held = feet(&world);
    assert!(held.x < 32.0 && held.x > 31.0, "held at {held}");
    assert!((held.z - 10.0).abs() < 0.05, "not dropped: {held}");
    // Its collider arrives (on the game's thread): the way is open.
    world.add_ground_shapes(vec![second]);
    assert!(world.ground_is_solid());
    walk(&mut world, Vec3::new(4.0, 0.0, 0.0), 60);
    let on = feet(&world);
    assert!(on.x > 33.0 && (on.z - 10.0).abs() < 0.05, "{on}");
}

#[test]
fn playing_shares_the_editor_s_ground_and_leaves_it_as_it_was() {
    let (g, shapes) = flat();
    let before = (*g).clone();
    let mut world = World::from_level(&walker_at(10.0));
    world.set_ground(Arc::clone(&g), (id(1), asset(), true), &needed(&shapes), shapes);
    assert!(Arc::ptr_eq(world.ground().unwrap(), &g), "no copy made");
    walk(&mut world, Vec3::new(2.0, 0.0, 0.0), 60);
    // A rebuild (an edit while playing) keeps it, colliders and all.
    world.rebuild(&walker_at(10.0));
    assert!(world.ground_is_solid());
    drop(world);
    assert_eq!(*g, before);
    assert_eq!(Arc::strong_count(&g), 1);
}

#[test]
fn a_ground_that_isnt_solid_is_passed_through() {
    let (g, shapes) = flat();
    let mut world = World::from_level(&walker_at(10.0));
    world.set_ground(g, (id(1), asset(), false), &needed(&shapes), shapes);
    walk(&mut world, Vec3::ZERO, 120);
    assert!(feet(&world).z < 5.0, "fell to {}", feet(&world));
}
