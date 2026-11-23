//! What game code switches off (Fixes 2c): a floor turned off lets a crate
//! fall through it and a ray pass, a 2D collider leaves 2D physics and
//! comes back, a hidden parent hides what's under it, and all of it is in
//! the checksum, put back from a save and forgotten with its entity.

use super::*;
use glam::Vec3;
use level::builtin::{Collider, Collider2D, RigidBody, Transform};
use level::{EntityData, Level};

fn id(n: u64) -> EntityId {
    EntityId(reflect::Id::from_raw(n).unwrap())
}

fn at(z: f32) -> Transform {
    Transform {
        position: Vec3::new(0.0, 0.0, z),
        ..Transform::default()
    }
}

/// A floor (its top at 0), a crate resting on it, a 2D post, and a lamp
/// with a glow under it.
fn world() -> World {
    let mut level = Level::new();
    level.insert(
        id(1),
        EntityData::new("Floor").with(&at(-0.5)).with(&Collider {
            size: Vec3::new(20.0, 20.0, 1.0),
            ..Collider::default()
        }),
    );
    level.insert(
        id(2),
        EntityData::new("Crate")
            .with(&at(0.5))
            .with(&RigidBody::default())
            .with(&Collider::default()),
    );
    level.insert(
        id(3),
        EntityData::new("Post")
            .with(&at(0.0))
            .with(&Collider2D::default()),
    );
    level.insert(id(4), EntityData::new("Lamp").with(&at(0.0)));
    let mut glow = EntityData::new("Glow").with(&at(0.0));
    glow.parent = Some(id(4));
    level.insert(id(5), glow);
    World::from_level(&level)
}

fn z_of(world: &World, which: EntityId) -> f32 {
    world.global(which).unwrap().w_axis.z
}

#[test]
fn a_floor_turned_off_lets_the_crate_fall_and_rays_pass() {
    let mut world = world();
    let down = |w: &World| {
        w.physics()
            .state()
            .cast_ray(Vec3::new(5.0, 5.0, 3.0), Vec3::NEG_Z, 10.0)
    };
    world.step_physics();
    assert!(down(&world).is_some());
    world.set_solid(id(1), false);
    assert!(!world.is_solid(id(1)));
    world.step_physics();
    assert!(down(&world).is_none(), "the ray passes through");
    for _ in 0..60 {
        world.step_physics();
    }
    assert!(z_of(&world, id(2)) < 0.0, "the crate fell through");
    // On again: rays meet it, and the floor never moved.
    world.set_solid(id(1), true);
    world.step_physics();
    assert!(down(&world).is_some());
    assert_eq!(z_of(&world, id(1)), -0.5);
}

#[test]
fn a_2d_collider_leaves_2d_physics_and_comes_back() {
    let mut world = world();
    assert!(world.flat().state().contains(id(3)));
    world.set_solid(id(3), false);
    assert!(!world.flat().state().contains(id(3)));
    // Built again for an edit while playing: still off.
    world.join_all();
    assert!(!world.flat().state().contains(id(3)));
    world.set_solid(id(3), true);
    assert!(world.flat().state().contains(id(3)));
}

#[test]
fn a_hidden_parent_hides_what_is_under_it() {
    let mut world = world();
    assert!(world.is_shown(id(5)));
    world.set_visible(id(4), false);
    assert!(!world.is_shown(id(4)) && !world.is_shown(id(5)));
    assert!(world.is_shown(id(2)));
    world.set_visible(id(4), true);
    assert!(world.is_shown(id(5)));
    // An entity that isn't there changes nothing.
    world.set_visible(id(99), false);
    assert!(world.switches().hidden.is_empty());
}

#[test]
fn switches_are_in_the_checksum_come_back_from_a_save_and_leave_with_their_entity() {
    let mut world = world();
    let before = world.checksum();
    world.set_visible(id(4), false);
    world.set_solid(id(3), false);
    assert_ne!(world.checksum(), before);
    let saved = world.switches().clone();
    let mut loaded = self::world();
    loaded.set_switches(saved.clone());
    assert_eq!(loaded.switches(), &saved);
    assert!(!loaded.flat().state().contains(id(3)));
    assert_eq!(loaded.checksum(), world.checksum());
    world.remove_subtrees(&[id(4), id(3)]);
    assert!(world.switches().hidden.is_empty() && world.switches().unsolid.is_empty());
    // Back as it was: the sum is as if nothing was ever switched.
    loaded.set_switches(Switches::default());
    assert_eq!(loaded.checksum(), before);
}
