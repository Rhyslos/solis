//! The tree's tests: its order and places, the walk up through parents,
//! and loops of parents (a damaged file) found, mended and never followed
//! round.

use super::*;
use crate::document::EntityData;
use glam::Vec3;

fn at(x: f32) -> Transform {
    Transform {
        position: Vec3::new(x, 0.0, 0.0),
        ..Transform::default()
    }
}

#[test]
fn the_tree_lists_parents_first_and_places_everything_like_the_area() {
    let mut area = Area::default();
    let ship = area.spawn(EntityData::new("Ship").with(&at(10.0)));
    let mut mast = EntityData::new("Mast").with(&at(1.0));
    mast.parent = Some(ship);
    let mast = area.spawn(mast);
    let mut sail = EntityData::new("Sail").with(&at(0.5));
    sail.parent = Some(mast);
    let sail = area.spawn(sail);
    let buoy = area.spawn(EntityData::new("Buoy"));
    let tree = Tree::of(&area);
    assert_eq!(tree.order(), area.depth_first().as_slice());
    assert_eq!(tree.children(Some(ship)), [mast]);
    assert_eq!(
        tree.first_of([sail, buoy, mast]),
        area.first_in_tree([sail, buoy, mast])
    );
    assert_eq!(tree.first_of([EntityId::random()]), None);
    let world = tree.world_matrices(&area);
    for id in [ship, mast, sail, buoy] {
        assert_eq!(world[&id], area.world_matrix(id));
    }
    assert!(!tree.in_loop(sail));
}

#[test]
fn a_loop_of_parents_is_listed_last_and_still_placed() {
    let mut area = Area::default();
    let a = area.spawn(EntityData::new("A").with(&at(1.0)));
    let b = area.spawn(EntityData::new("B").with(&at(2.0)));
    let top = area.spawn(EntityData::new("Top"));
    area.get_mut(a).unwrap().parent = Some(b);
    area.get_mut(b).unwrap().parent = Some(a);
    let tree = Tree::of(&area);
    assert_eq!(tree.reachable(), [top]);
    assert_eq!(tree.order().len(), 3);
    assert!(tree.in_loop(a) && tree.in_loop(b));
    let world = tree.world_matrices(&area);
    assert_eq!(world[&a], area.world_matrix(a));
    assert_eq!(world.len(), 3);
    // Each stands alone, at its own Transform: gone round the loop,
    // A would be at 3, 6, 9…
    assert_eq!(world[&a], at(1.0).matrix());
    assert_eq!(world[&b], at(2.0).matrix());
}

fn id(n: u64) -> EntityId {
    EntityId(reflect::Id::from_raw(n).unwrap())
}

/// An area of entities 1 to `parents.len()`, each under the one its
/// number in `parents` names (0: none; a number past them all: an
/// entity that isn't there), each with a Transform that scales.
fn area_of(parents: &[u64]) -> Area {
    let mut area = Area::default();
    for (n, &parent) in (1..).zip(parents) {
        let mut data = EntityData::new(&format!("E{n}")).with(&Transform {
            position: Vec3::new(n as f32, 0.0, 0.0),
            scale: Vec3::splat(2.0),
            ..Transform::default()
        });
        data.parent = (parent > 0).then(|| id(parent));
        area.insert(id(n), data);
    }
    area
}

/// A chain of `tail` entities leading up into a loop of `round`:
/// entity 1 is at the bottom.
fn chain_into_a_loop(tail: u64, round: u64) -> Area {
    let count = tail + round;
    let parents: Vec<u64> = (1..=count)
        .map(|n| if n == count { tail + 1 } else { n + 1 })
        .collect();
    area_of(&parents)
}

#[test]
fn the_walk_up_ends_at_the_top_at_a_missing_parent_and_in_a_loop() {
    // 1 under 2 under 3; 4 under an entity that isn't there.
    let area = area_of(&[2, 3, 0, 99]);
    let above = |n| -> Vec<EntityId> { area.ancestors(id(n)).map(|(id, _)| id).collect() };
    assert_eq!(above(1), [id(2), id(3)]);
    assert_eq!(above(3), []);
    assert_eq!(above(4), []);
    assert_eq!(above(99), [], "an entity that isn't there");
    let mut walk = area.ancestors(id(1));
    assert_eq!(walk.by_ref().count(), 2);
    assert!(!walk.looped());
    // Its own parent: nothing above it, and it says why.
    let alone = area_of(&[1]);
    let mut walk = alone.ancestors(id(1));
    assert_eq!(walk.next(), None);
    assert!(walk.looped());
}

#[test]
fn a_walk_up_into_any_loop_ends_within_a_few_turns() {
    // Every chain of up to 24 entities into every loop of up to 24:
    // the walk passes each of them, and ends having taken a number of
    // steps that grows only with how many there are.
    for tail in 0..24 {
        for round in 1..24 {
            let area = chain_into_a_loop(tail, round);
            let mut walk = area.ancestors(id(1));
            let passed: Vec<EntityId> = walk.by_ref().map(|(id, _)| id).collect();
            assert!(walk.looped(), "{tail} into {round}");
            let all = tail + round;
            assert!(passed.len() as u64 <= 4 * all, "{tail} into {round}");
            for n in 2..=all {
                assert!(passed.contains(&id(n)), "{n} of {tail} into {round}");
            }
        }
    }
    // And a long one: 20,000 in a ring.
    let ring = chain_into_a_loop(0, 20_000);
    let mut walk = ring.ancestors(id(7));
    assert!(walk.by_ref().count() <= 80_000);
    assert!(walk.looped());
}

#[test]
fn loops_are_mended_by_moving_one_entity_of_each_to_the_top() {
    // 1 and 2 in a loop; 3, 4 and 5 in another; 6 its own parent; 7
    // and 8 hanging under the first loop; 9 sound, with 10 under it;
    // 11 under an entity that isn't there.
    let mut area = area_of(&[2, 1, 4, 5, 3, 6, 2, 7, 0, 9, 99]);
    let before = area.clone();
    let mended = area.mend_loops();
    let cut: Vec<(EntityId, EntityId)> = mended.iter().map(|m| (m.entity, m.parent)).collect();
    assert_eq!(cut, [(id(1), id(2)), (id(3), id(4)), (id(6), id(6))]);
    assert_eq!(mended[0].name, "E1");
    assert_eq!(
        mended[0].to_string(),
        format!(
            "E1 ({}) was inside itself: its parents led back to it. It's now at the top level",
            id(1)
        )
    );
    // Nothing else changed: the rest of each loop hangs under the one
    // moved, and a parent that isn't there is kept as written.
    for (entity, data) in before.iter() {
        let now = area.get(entity).unwrap();
        if [id(1), id(3), id(6)].contains(&entity) {
            assert_eq!(now.parent, None);
            assert_eq!(now.components, data.components);
        } else {
            assert_eq!(now, data);
        }
    }
    let tree = Tree::of(&area);
    assert_eq!(tree.reachable().len(), area.len());
    assert!(
        area.iter()
            .all(|(id, _)| !area.ancestors(id).by_ref().looped())
    );
    assert_eq!(
        area.ancestors(id(8)).count(),
        3,
        "8 under 7 under 2 under 1"
    );
    // Mended once is mended: nothing more to do, nothing changed.
    let again = area.clone();
    assert_eq!(area.mend_loops(), []);
    assert_eq!(area, again);
}

#[test]
fn a_sound_area_is_left_as_it_is() {
    let mut area = area_of(&[0, 1, 2, 2, 99, 0]);
    let before = area.clone();
    assert_eq!(area.mend_loops(), []);
    assert_eq!(area, before);
    assert_eq!(Area::new().mend_loops(), []);
}

#[test]
fn every_chain_into_every_loop_is_mended_with_one_move() {
    for tail in 0..12 {
        for round in 1..12 {
            let mut area = chain_into_a_loop(tail, round);
            let mended = area.mend_loops();
            // The loop's lowest ID is the first entity past the chain.
            assert_eq!(mended.len(), 1, "{tail} into {round}");
            assert_eq!(mended[0].entity, id(tail + 1));
            assert_eq!(Tree::of(&area).reachable().len(), area.len());
            let top = area.get(id(tail + 1)).unwrap();
            assert_eq!(top.parent, None);
        }
    }
    // 20,000 in one ring: one pass, one move.
    let mut ring = chain_into_a_loop(0, 20_000);
    assert_eq!(ring.mend_loops().len(), 1);
    assert_eq!(Tree::of(&ring).reachable().len(), 20_000);
}

/// The finding's file: `count` entities, each its own parent, each
/// with a Transform that scales.
fn every_entity_its_own_parent(count: u64) -> String {
    let mut text = String::from("area version 1;\n");
    for n in 1..=count {
        let id = id(n);
        text.push_str(&format!(
            "entity {id} {{\n    name = \"E{n}\";\n    parent = \"{id}\";\n    \
             Transform.scale = [2.0, 2.0, 2.0];\n}}\n"
        ));
    }
    text
}

#[test]
fn twenty_thousand_entities_each_their_own_parent_are_read_and_placed() {
    // The review of Phase 9, BUG-12: this file took 4 × 10^8 steps to
    // place (the frame loop hung for minutes), and each entity came
    // out scaled 2 to the power of 20,001: not a number.
    let registry = crate::builtin::registry().unwrap();
    let area = crate::text::read(&every_entity_its_own_parent(20_000), &registry).unwrap();
    assert_eq!(area.len(), 20_000);
    // Read, they're all at the top level.
    assert!(area.iter().all(|(_, e)| e.parent.is_none()));
    let tree = Tree::of(&area);
    assert_eq!(tree.reachable().len(), 20_000);
    let world = tree.world_matrices(&area);
    for (id, entity) in area.iter() {
        let own = entity.get::<Transform>().unwrap().matrix();
        assert_eq!(world[&id], own, "{}", entity.name);
    }
}

#[test]
fn loops_that_were_never_mended_are_still_placed_in_one_pass() {
    // An area handed over with its loops (none of the readers makes
    // one): 20,000 entities each their own parent, then 20,000 in one
    // ring. Each stands alone, and the tree and the walk agree.
    let own_parents: Vec<u64> = (1..=20_000).collect();
    for area in [area_of(&own_parents), chain_into_a_loop(0, 20_000)] {
        let tree = Tree::of(&area);
        assert_eq!(tree.reachable().len(), 0);
        let world = tree.world_matrices(&area);
        assert_eq!(world.len(), 20_000);
        for (id, entity) in area.iter() {
            let own = entity.get::<Transform>().unwrap().matrix();
            assert_eq!(world[&id], own, "{}", entity.name);
        }
        for n in [1, 2, 9_999, 20_000] {
            assert_eq!(area.world_matrix(id(n)), world[&id(n)]);
        }
    }
}

#[test]
fn the_tree_and_the_walk_place_every_damaged_area_alike() {
    // Chains into loops, with entities hanging under the loop's
    // members too.
    for tail in 0..6 {
        for round in 1..6 {
            let mut area = chain_into_a_loop(tail, round);
            let mut hanger = EntityData::new("Hanger").with(&at(5.0));
            hanger.parent = Some(id(tail + round));
            area.insert(id(100), hanger);
            let world = Tree::of(&area).world_matrices(&area);
            for (id, entity) in area.iter() {
                let own = entity.get::<Transform>().unwrap().matrix();
                assert_eq!(world[&id], own);
                assert_eq!(area.world_matrix(id), own, "{}", entity.name);
            }
        }
    }
}

#[test]
fn the_fuzzers_seed_of_loops_reads_and_is_mended() {
    // fuzz/seeds/area_file/parent-loops.solarea: what the area fuzzer
    // starts from, so loops are tried every week. One of each kind.
    let text = include_str!("../../../fuzz/seeds/area_file/parent-loops.solarea");
    let registry = crate::builtin::registry().unwrap();
    let file = crate::text::read_file(text, &registry).unwrap();
    assert_eq!(Tree::of(&file.area).reachable().len(), 2, "as written");
    let mut area = file.area.clone();
    let mended: Vec<EntityId> = area.mend_loops().iter().map(|m| m.entity).collect();
    assert_eq!(mended, [id(1), id(2), id(4)]);
    assert_eq!(area, crate::text::read(text, &registry).unwrap());
    assert_eq!(Tree::of(&area).reachable().len(), 10);
    // The one whose parent isn't in the file keeps it, at the top level.
    assert!(area.get(id(9)).unwrap().parent.is_some());
    assert!(Tree::of(&area).children(None).contains(&id(9)));
    let world = Tree::of(&area).world_matrices(&area);
    assert!(world.values().all(Mat4::is_finite));
}
