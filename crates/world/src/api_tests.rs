//! The world's answers the editor, save games and drawing rely on: seeds,
//! the clock, the entity list, physics asks, cameras between ticks.

use super::*;
use area::builtin::{CharacterBody, Collider};

#[test]
fn the_seed_and_clock_are_kept_as_set_and_count_as_changes() {
    let mut world = World::from_area(&area::builtin::starter_area());
    let before = world.changes();
    world.set_seed(42);
    assert_eq!(world.seed(), 42);
    let after_seed = world.changes();
    assert_ne!(after_seed, before, "a new seed is a change");
    world.set_seed(7);
    assert_eq!(world.seed(), 7);
    world.set_clock(120, 2.5);
    assert_eq!(world.clock().ticks, 120);
    assert_eq!(world.clock().time, 2.5);
    assert_eq!(world.progress(), 0.0, "nothing waits for the next tick");
    assert_ne!(world.changes(), after_seed, "a set clock is a change");
    // A damaged save's time never reaches the clock.
    world.set_clock(3, f64::NAN);
    assert_eq!((world.clock().ticks, world.clock().time), (3, 0.0));
}

#[test]
fn the_entity_list_is_every_entity_in_tree_order_with_names() {
    let mut area = Area::new();
    let parent = area.spawn(EntityData::new("Parent").with(&at(1.0, 0.0, 0.0)));
    let mut data = EntityData::new("Child").with(&at(0.0, 2.0, 0.0));
    data.parent = Some(parent);
    let child = area.spawn(data);
    let world = World::from_area(&area);
    assert_eq!(
        world.entities(),
        vec![
            Some((parent, "Parent".to_string())),
            Some((child, "Child".to_string()))
        ]
    );
    assert!(!world.is_empty());
    assert_eq!(world.ecs().len() as usize, world.len());
    // Number i in the list is number i in the places.
    let mut globals = Vec::new();
    world.globals(&mut globals);
    assert_eq!(globals.len(), 2);
    assert_eq!(globals[0].w_axis.x, 1.0);
    assert_eq!((globals[1].w_axis.x, globals[1].w_axis.y), (1.0, 2.0));
    assert!(World::from_area(&Area::new()).is_empty());
}

#[test]
fn a_push_lands_at_the_next_physics_step() {
    let (area, _, crate_id) = floor_and_crate(0.5);
    let mut world = World::from_area(&area);
    world.push(crate_id, Vec3::new(50.0, 0.0, 0.0));
    let speed = |w: &World| w.physics().state().velocity(crate_id).unwrap().x;
    assert_eq!(speed(&world), 0.0, "not yet");
    world.step_physics();
    assert!(speed(&world) > 1.0, "pushed: {}", speed(&world));
}

#[test]
fn a_character_moves_at_the_next_physics_step() {
    let mut area = Area::new();
    area.spawn(
        EntityData::new("Floor")
            .with(&at(0.0, 0.0, -0.5))
            .with(&Collider {
                size: Vec3::new(40.0, 40.0, 1.0),
                ..Collider::default()
            }),
    );
    let walker = area.spawn(
        EntityData::new("Walker")
            .with(&at(0.0, 0.0, 0.05))
            .with(&CharacterBody::default()),
    );
    let mut world = World::from_area(&area);
    assert!(
        world.get::<CharacterBody>(walker).is_some(),
        "kept as its type"
    );
    world.move_character(walker, Vec3::new(0.5, 0.0, 0.0));
    assert_eq!(world.global(walker).unwrap().w_axis.x, 0.0, "not yet");
    world.step_physics();
    let x = world.global(walker).unwrap().w_axis.x;
    assert!((x - 0.5).abs() < 0.05, "walked half a metre: {x}");
}

#[test]
fn physics_states_hold_every_body_and_nothing_broke() {
    let (area, floor, crate_id) = floor_and_crate(3.0);
    let mut world = World::from_area(&area);
    for _ in 0..10 {
        world.step_physics();
    }
    let states = world.physics_states();
    let ids: Vec<EntityId> = states.iter().map(|(id, _)| *id).collect();
    assert_eq!(ids, vec![floor, crate_id]);
    assert!(!world.take_physics_broke());
    assert_eq!(world.physics_refused(), 0);
}

#[test]
fn the_camera_is_drawn_between_ticks_too() {
    let mut area = Area::new();
    let camera = area.spawn(
        EntityData::new("Camera")
            .with(&at(0.0, 0.0, 0.0))
            .with(&Camera::default()),
    );
    let mut world = World::from_area(&area);
    let x = |world: &World, t: f32| world.camera_between(t).unwrap().2.w_axis.x;
    assert_eq!(x(&world, 0.5), 0.0, "no tick yet: where it is");
    world.remember();
    world.set_transform(camera, at(10.0, 0.0, 0.0));
    assert_eq!(x(&world, 0.5), 5.0);
    assert_eq!(x(&world, 1.0), 10.0);
    assert_eq!(world.camera_between(0.5).unwrap().0, camera);
    assert!(World::from_area(&Area::new()).camera_between(0.5).is_none());
}

#[test]
fn models_and_the_environment_arrive_as_they_are_in_the_area() {
    use area::builtin::{Environment, Model};
    let mut area = Area::new();
    let environment = Environment {
        fog: true,
        fog_distance: 77.0,
        sky_strength: 0.25,
        ..Environment::default()
    };
    area.spawn(EntityData::new("Sky").with(&environment));
    let ship = area.spawn(EntityData::new("Ship").with(&Model::default()));
    let world = World::from_area(&area);
    assert_eq!(world.environment(), Some(environment));
    assert_eq!(world.get::<Model>(ship), Some(Model::default()));
}

#[test]
fn projectiles_count_while_flying_and_go_when_their_time_is_up() {
    let mut world = World::from_area(&Area::new());
    assert!(world.projectiles().is_empty());
    world.projectiles_mut().fire(Shot {
        from: Vec3::ZERO,
        direction: Vec3::X,
        speed: 10.0,
        life: 0.5,
        colour: [1.0; 3],
    });
    assert!(!world.projectiles().is_empty());
    let projectiles = world.projectiles_mut();
    projectiles.tick(0.25);
    assert_eq!(projectiles.len(), 1);
    assert!((projectiles.flying()[0].position.x - 2.5).abs() < 1e-5);
    projectiles.tick(0.0);
    assert_eq!(projectiles.len(), 1, "no time passed");
    projectiles.tick(0.25);
    assert!(projectiles.is_empty(), "its half second is up");
}
