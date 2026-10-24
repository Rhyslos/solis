//! Building worlds from areas, stepping them, and finding things in them.

use super::*;
use area::EntityData;
use glam::Vec3;
use reflect::Value;

fn at(x: f32, y: f32, z: f32) -> Transform {
    Transform {
        position: Vec3::new(x, y, z),
        ..Transform::default()
    }
}

#[test]
fn every_entity_arrives_with_its_built_in_components_as_rust_types() {
    let area = area::builtin::starter_area();
    let world = World::from_area(&area);
    assert_eq!(world.len(), area.len());
    let suns = world.placed::<Light>();
    assert_eq!(suns.len(), 1);
    assert_eq!(suns[0].1, Light::sun());
    assert!(world.environment().is_some());
    assert!(world.camera().is_none());
    for (id, data) in area.iter() {
        assert_eq!(world.get::<Name>(id), Some(Name(data.name.clone())));
        assert_eq!(world.get::<Id>(id), Some(Id(id)));
    }
}

#[test]
fn children_are_placed_by_their_parents() {
    let mut area = Area::new();
    let boat = area.spawn(EntityData::new("Boat").with(&at(10.0, 0.0, 0.0)));
    let mut mast = EntityData::new("Mast").with(&at(0.0, 0.0, 5.0));
    mast.parent = Some(boat);
    let mast = area.spawn(mast);
    // No Transform: it sits where its parent is.
    let mut flag = EntityData::new("Flag");
    flag.parent = Some(mast);
    let flag = area.spawn(flag);
    let world = World::from_area(&area);
    let point = |id| world.global(id).unwrap().transform_point3(Vec3::ZERO);
    assert_eq!(point(boat), Vec3::new(10.0, 0.0, 0.0));
    assert_eq!(point(mast), Vec3::new(10.0, 0.0, 5.0));
    assert_eq!(point(flag), Vec3::new(10.0, 0.0, 5.0));
    assert_eq!(world.get::<Parent>(mast), Some(Parent(boat)));
    // The same as the area works it out.
    for id in [boat, mast, flag] {
        assert_eq!(world.global(id), Some(area.world_matrix(id)));
    }
}

#[test]
fn components_without_a_type_are_kept_as_values() {
    let mut area = Area::new();
    let mut data = EntityData::new("Turret");
    let mut aim = ComponentData::default();
    aim.set("speed", Value::Float(2.5));
    data.components.insert("Aim".into(), aim.clone());
    let id = area.spawn(data);
    let world = World::from_area(&area);
    let values = world.get::<Values>(id).unwrap();
    assert_eq!(values.0.get("Aim"), Some(&aim));
    assert!(world.get::<Transform>(id).is_none());
}

#[test]
fn the_first_camera_in_tree_order_is_the_one_looked_through() {
    let mut area = Area::new();
    let a = area.spawn(EntityData::new("A").with(&Camera {
        fov: 70.0,
        ..Camera::default()
    }));
    let b = area.spawn(EntityData::new("B").with(&Camera::default()));
    let world = World::from_area(&area);
    let first = area.depth_first()[0];
    assert_eq!(world.camera().unwrap().0, first);
    assert!([a, b].contains(&first));
    // A damaged camera is made usable.
    let mut area = Area::new();
    area.spawn(EntityData::new("Odd").with(&Camera {
        fov: -3.0,
        near: 0.0,
        far: f32::NAN,
    }));
    let (_, camera, _) = World::from_area(&area).camera().unwrap();
    assert_eq!(camera, camera.sanitized());
}

#[test]
fn the_clock_runs_in_whole_ticks_and_skips_ahead_after_a_stall() {
    let mut world = World::from_area(&Area::new());
    assert_eq!(world.step(TICK / 2.0), 0);
    assert_eq!(world.step(TICK / 2.0 + 1e-9), 1, "the halves add up");
    assert_eq!(world.step(1.0), MAX_TICKS_PER_STEP, "at most a few at once");
    assert_eq!(world.step(TICK * 0.99), 0, "the rest was dropped");
    for bad in [f64::NAN, f64::INFINITY, -1.0, 0.0] {
        assert_eq!(world.step(bad), 0);
    }
    let clock = world.clock();
    assert_eq!(clock.ticks, 1 + MAX_TICKS_PER_STEP);
    assert!((clock.time - clock.ticks as f64 * TICK).abs() < 1e-9);
}

#[test]
fn rebuilding_keeps_the_clock_and_takes_the_new_area() {
    let mut area = area::builtin::starter_area();
    let mut world = World::from_area(&area);
    world.step(0.5);
    let clock = world.clock();
    let crate_id = area.spawn(EntityData::new("Crate").with(&at(1.0, 2.0, 3.0)));
    world.rebuild(&area);
    assert_eq!(world.clock(), clock);
    assert_eq!(
        world.global(crate_id).unwrap().transform_point3(Vec3::ZERO),
        Vec3::new(1.0, 2.0, 3.0)
    );
}

#[test]
fn rebuilding_keeps_where_the_game_put_things_unless_the_area_moved_them() {
    let mut area = area::builtin::starter_area();
    let cart = area.spawn(EntityData::new("Cart").with(&at(0.0, 0.0, 0.0)));
    let crate_id = area.spawn(EntityData::new("Crate").with(&at(5.0, 0.0, 0.0)));
    let mut world = World::from_area(&area);
    world.set_transform(cart, at(3.0, 0.0, 0.0));
    world.set_transform(crate_id, at(7.0, 0.0, 0.0));
    // Adding an entity builds everything again: the cart stays where the
    // game put it.
    area.spawn(EntityData::new("New"));
    // The crate was moved in the area meanwhile (an undo, say): the
    // area's place wins.
    area.get_mut(crate_id).unwrap().set(&at(9.0, 0.0, 0.0));
    world.rebuild(&area);
    let x = |w: &World, id| w.global(id).unwrap().w_axis.x;
    assert_eq!(x(&world, cart), 3.0);
    assert_eq!(x(&world, crate_id), 9.0);
    // And again, still.
    world.rebuild(&area);
    assert_eq!(x(&world, cart), 3.0);
}

#[test]
fn a_damaged_area_with_a_parent_loop_still_builds() {
    let mut area = Area::new();
    let a = EntityId::random();
    let b = EntityId::random();
    let mut ea = EntityData::new("A").with(&at(1.0, 0.0, 0.0));
    ea.parent = Some(b);
    let mut eb = EntityData::new("B").with(&at(0.0, 1.0, 0.0));
    eb.parent = Some(a);
    area.insert(a, ea);
    area.insert(b, eb);
    // A parent that isn't there at all.
    let mut lost = EntityData::new("Lost");
    lost.parent = Some(EntityId::random());
    let lost = area.spawn(lost);
    let mut world = World::from_area(&area);
    assert_eq!(world.len(), 3);
    assert!(world.global(a).unwrap().is_finite());
    assert!(world.global(lost).is_some());
    world.step(TICK);
}

#[test]
fn a_big_area_builds_and_steps() {
    // The scale target's order of size (DESIGN.md §2): checked for being
    // right here; timed by the scale check.
    let mut area = Area::new();
    let mut parent = None;
    for i in 0..20_000 {
        let mut e = EntityData::new(&format!("Post {i}")).with(&at(1.0, 0.0, 0.0));
        if i % 100 != 0 {
            e.parent = parent;
        }
        parent = Some(area.spawn(e));
    }
    let mut world = World::from_area(&area);
    assert_eq!(world.len(), 20_000);
    assert_eq!(world.step(TICK), 1);
    let last = parent.unwrap();
    // Each chain is 100 long, each link 1 m further along X.
    assert_eq!(
        world.global(last).unwrap().transform_point3(Vec3::ZERO),
        Vec3::new(100.0, 0.0, 0.0)
    );
}

#[test]
fn an_update_takes_in_changed_entities_and_keeps_the_rest() {
    let mut area = Area::new();
    let boat = area.spawn(EntityData::new("Boat").with(&at(10.0, 0.0, 0.0)));
    let mut mast = EntityData::new("Mast").with(&at(0.0, 0.0, 5.0));
    mast.parent = Some(boat);
    let mast = area.spawn(mast);
    let mut world = World::from_area(&area);
    world.step(0.5);
    let clock = world.clock();
    // The boat moves and gets a light: its child follows.
    let mut moved = area.get(boat).unwrap().clone().with(&at(20.0, 0.0, 0.0));
    moved.set(&Light::point());
    area.insert(boat, moved);
    world.update(&area, [(boat, true)]);
    assert_eq!(world.clock(), clock);
    assert_eq!(world.len(), 2);
    assert!(world.get::<Light>(boat).is_some());
    let point = |w: &World, id| w.global(id).unwrap().transform_point3(Vec3::ZERO);
    assert_eq!(point(&world, mast), Vec3::new(20.0, 0.0, 5.0));
    assert_eq!(world.placed::<Light>().len(), 1);
    // A new parent, or an entity the world doesn't have, builds it all
    // again, and the answer is the same as building from scratch.
    let mut freed = area.get(mast).unwrap().clone();
    freed.parent = None;
    area.insert(mast, freed);
    let extra = area.spawn(EntityData::new("Crate").with(&at(1.0, 1.0, 1.0)));
    world.update(&area, [(mast, true), (extra, true)]);
    let fresh = World::from_area(&area);
    for id in [boat, mast, extra] {
        assert_eq!(world.global(id), fresh.global(id));
    }
    assert_eq!(world.len(), 3);
    // An entity that's gone does the same.
    let mut smaller = area.clone();
    let _ = smaller.remove(extra);
    world.update(&smaller, [(extra, true)]);
    assert_eq!(world.len(), 2);
}

#[test]
fn an_update_that_didn_t_move_an_entity_keeps_where_the_game_put_it() {
    let mut area = Area::new();
    let cart = area.spawn(EntityData::new("Cart").with(&at(0.0, 0.0, 0.0)));
    let mut world = World::from_area(&area);
    world.set_transform(cart, at(5.0, 0.0, 0.0));
    world.settle();
    // A light is added in the editor: the cart stays where the game put it.
    let mut lit = area.get(cart).unwrap().clone();
    lit.set(&Light::point());
    area.insert(cart, lit);
    world.update(&area, [(cart, false)]);
    assert!(world.get::<Light>(cart).is_some());
    assert_eq!(world.global(cart).unwrap().w_axis.x, 5.0);
    // Moved in the editor: the editor's place wins.
    world.update(&area, [(cart, true)]);
    assert_eq!(world.global(cart).unwrap().w_axis.x, 0.0);
}

#[test]
fn game_code_moves_an_entity_and_those_under_it_follow_when_settled() {
    let mut area = Area::default();
    let ship = area.spawn(EntityData::new("Ship").with(&at(10.0, 0.0, 0.0)));
    let mut mast = EntityData::new("Mast").with(&at(0.0, 0.0, 2.0));
    mast.parent = Some(ship);
    let mast = area.spawn(mast);
    let mut flag = EntityData::new("Flag").with(&at(0.0, 0.0, 1.0));
    flag.parent = Some(mast);
    let flag = area.spawn(flag);
    let bare = area.spawn(EntityData::new("Bare"));
    let mut world = World::from_area(&area);
    assert_eq!(world.find("Mast"), Some(mast));
    assert_eq!(world.find("Nobody"), None);
    assert_eq!(
        world.parent_global(mast).w_axis.truncate(),
        Vec3::new(10.0, 0.0, 0.0)
    );
    // The mast moves: its own place follows at once, the flag's on settle.
    assert!(world.set_transform(mast, at(1.0, 0.0, 2.0)));
    assert_eq!(
        world.global(mast).unwrap().w_axis.truncate(),
        Vec3::new(11.0, 0.0, 2.0)
    );
    assert_eq!(
        world.global(flag).unwrap().w_axis.truncate(),
        Vec3::new(10.0, 0.0, 3.0)
    );
    world.settle();
    assert_eq!(
        world.global(flag).unwrap().w_axis.truncate(),
        Vec3::new(11.0, 0.0, 3.0)
    );
    // An entity without a transform gets one.
    assert!(world.set_transform(bare, at(0.0, 5.0, 0.0)));
    assert_eq!(world.get::<Transform>(bare), Some(at(0.0, 5.0, 0.0)));
    assert!(!world.set_transform(EntityId::random(), at(0.0, 0.0, 0.0)));
}
