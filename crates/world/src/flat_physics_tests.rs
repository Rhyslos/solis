//! 2D physics in the running game: a character walked by game code stops
//! at a map's solid cells and slides along them, walls follow cells the
//! game changes, solid still colliders block, pixels become metres, and
//! all of it is in the checksum and the same every run.

use super::*;
use crate::{MapFiles, TICK};
use area::builtin::{TileMap as TileMapComponent, Transform};
use area::{Area, EntityData};
use reflect::AssetRef;
use tiles::TileMap;

/// A 20 × 10 map of grass with a fence (`#`) down column 10, its top-left
/// corner at the world's origin; a farmer (a circle of 4 px at 16 px a
/// metre, so a quarter metre) at cell (2, 5).
fn farm() -> (World, EntityId, EntityId) {
    let (map_asset, tileset) = (AssetId::random(), AssetId::random());
    let mut area = Area::new();
    let map = area.spawn(EntityData::new("Farm").with(&Transform::default()).with(
        &TileMapComponent {
            map: Some(AssetRef {
                asset: map_asset,
                part: None,
            }),
        },
    ));
    let farmer = area.spawn(
        EntityData::new("Farmer")
            .with(&Transform {
                position: Vec3::new(2.5, -5.5, 0.2),
                ..Transform::default()
            })
            .with(&Collider2D {
                radius: 4.0,
                character: true,
                ..Collider2D::default()
            }),
    );
    let mut world = World::from_area(&area);
    let mut file = TileMap::new([20, 10], Some(tileset), Some(b'.'));
    for y in 0..10 {
        file.set_code(1, 10, y, b'#').unwrap();
    }
    let mut files = MapFiles::new();
    files.insert(map_asset, Arc::new(file));
    world.set_tile_maps(Arc::new(files));
    world.start_maps();
    let mut solid = [false; 256];
    solid[b'#' as usize] = true;
    world.set_solid_sets(Arc::new(SolidSets::from([(tileset, solid)])));
    (world, map, farmer)
}

fn walk(world: &mut World, who: EntityId, step: Vec3, ticks: usize) -> Vec3 {
    for _ in 0..ticks {
        world.move_character(who, step);
        world.step_physics();
    }
    world.global(who).unwrap().w_axis.truncate()
}

#[test]
fn a_farmer_walks_to_the_fence_stops_and_keeps_its_layer() {
    let (mut world, _, farmer) = farm();
    assert!(world.flat().state().is_character(farmer));
    let at = walk(&mut world, farmer, Vec3::new(0.1, 0.0, 5.0), 120);
    // The fence's left edge is at x = 10 (a tile is a metre at 16 px).
    assert!(at.x < 10.0 - 0.25 && at.x > 10.0 - 0.25 - 0.02, "{at}");
    assert_eq!(at.y, -5.5, "walking straight right");
    assert_eq!(at.z, 0.2, "the layer is kept; Z of a move is ignored");
    assert_eq!(
        world.flat().state().wall_count(),
        1,
        "the fence is one wall"
    );
}

#[test]
fn walls_follow_cells_the_game_changes() {
    let (mut world, map, farmer) = farm();
    walk(&mut world, farmer, Vec3::new(0.1, 0.0, 0.0), 120);
    // A gate: the fence cell in the farmer's row is taken out.
    world.set_tile(map, 1, [10, 5], 0).unwrap();
    let at = walk(&mut world, farmer, Vec3::new(0.1, 0.0, 0.0), 40);
    assert!(at.x > 12.0, "through the gate: {at}");
    assert_eq!(world.flat().state().wall_count(), 2, "above and below it");
    // Fenced in again behind the farmer.
    world.set_tile(map, 1, [10, 5], b'#').unwrap();
    let at = walk(&mut world, farmer, Vec3::new(-0.1, 0.0, 0.0), 60);
    assert!(at.x > 11.0 - 0.01, "the gate's shut: {at}");
}

#[test]
fn nothing_is_solid_without_a_tileset_saying_so() {
    let (mut world, _, farmer) = farm();
    world.set_solid_sets(Arc::new(SolidSets::new()));
    let at = walk(&mut world, farmer, Vec3::new(0.1, 0.0, 0.0), 120);
    assert!(at.x > 13.0, "no walls: {at}");
    assert_eq!(world.flat().state().wall_count(), 0);
}

#[test]
fn a_solid_box_blocks_and_pixels_are_metres_by_the_project_s_settings() {
    let (mut world, _, farmer) = farm();
    // A bin 32 px wide at x = 5: 2 m at 16 px a metre, so 4 to 6.
    let bin = EntityId::random();
    let data = EntityData::new("Bin")
        .with(&Transform {
            position: Vec3::new(5.0, -5.5, 0.0),
            ..Transform::default()
        })
        .with(&Collider2D {
            shape: Shape2D::Box,
            width: 32.0,
            height: 8.0,
            ..Collider2D::default()
        });
    world.add_subtree(&[(bin, data)]);
    assert!(world.flat().state().contains(bin));
    assert!(!world.flat().state().is_character(bin), "solid and still");
    let at = walk(&mut world, farmer, Vec3::new(0.1, 0.0, 0.0), 40);
    assert!(
        at.x < 4.0 - 0.25 && at.x > 4.0 - 0.27,
        "stopped at the bin: {at}"
    );
    // At 32 px a metre everything halves: the bin runs 4.5 to 5.5 and the
    // farmer's circle is an eighth of a metre.
    world.set_pixels(area::pixels::PixelSettings {
        pixels_per_unit: 32,
        ..area::pixels::PixelSettings::default()
    });
    assert_eq!(world.pixels().pixels_per_unit, 32);
    let at = walk(&mut world, farmer, Vec3::new(0.1, 0.0, 0.0), 40);
    assert!(at.x < 4.5 - 0.125 && at.x > 4.5 - 0.145, "{at}");
}

#[test]
fn moved_by_hand_it_walks_on_from_there() {
    let (mut world, _, farmer) = farm();
    world.change_transform(farmer, |_, mut t| {
        t.position = Vec3::new(15.0, -5.5, 0.2);
        Some(t)
    });
    let at = walk(&mut world, farmer, Vec3::new(-0.1, 0.0, 0.0), 80);
    assert!(
        at.x > 11.0 && at.x < 11.0 + 0.25 + 0.02,
        "the fence's other side: {at}"
    );
}

#[test]
fn the_same_walks_end_in_the_same_place_and_a_walk_changes_the_checksum() {
    let run = |world: &mut World, farmer: EntityId, ticks: std::ops::Range<usize>| {
        for i in ticks {
            let a = i as f32 * 0.1;
            world.move_character(farmer, Vec3::new(a.cos(), a.sin(), 0.0) * 0.12);
            world.step_physics();
        }
        world.global(farmer).unwrap().w_axis.truncate()
    };
    let (mut a, _, fa) = farm();
    let (mut b, _, fb) = farm();
    assert_eq!(run(&mut a, fa, 0..200), run(&mut b, fb, 0..200));
    let before = a.checksum();
    run(&mut a, fa, 200..201);
    assert_ne!(a.checksum(), before);
    let _ = TICK;
}

#[test]
fn a_despawned_collider_and_map_leave_2d_physics_with_them() {
    let (mut world, map, farmer) = farm();
    // A bin in the farmer's way, then despawned: no invisible wall left.
    let bin = EntityId::random();
    let data = EntityData::new("Bin")
        .with(&Transform {
            position: Vec3::new(5.0, -5.5, 0.0),
            ..Transform::default()
        })
        .with(&Collider2D {
            shape: Shape2D::Box,
            width: 32.0,
            height: 8.0,
            ..Collider2D::default()
        });
    world.add_subtree(&[(bin, data)]);
    world.remove_subtrees(&[bin]);
    assert!(!world.flat().state().contains(bin));
    let at = walk(&mut world, farmer, Vec3::new(0.1, 0.0, 0.0), 40);
    assert!(at.x > 6.0, "walked on past where the bin was: {at}");
    // The map despawned: its fence's walls go too.
    world.remove_subtrees(&[map]);
    world.step_physics();
    let at = walk(&mut world, farmer, Vec3::new(0.1, 0.0, 0.0), 80);
    assert!(at.x > 11.0, "on past the fence: {at}");
}

/// Where something carried sits from its carrier: half a metre right, a
/// metre up and a layer above.
fn carried() -> Transform {
    Transform {
        position: Vec3::new(0.5, 1.0, 0.3),
        ..Transform::default()
    }
}

/// `data` under `carrier`, at [`carried`].
fn carry(area: &mut Area, carrier: EntityId, name: &str) -> EntityId {
    let mut data = EntityData::new(name).with(&carried());
    data.parent = Some(carrier);
    area.spawn(data)
}

/// A world with, if `flat`, a 2D character carrying a lantern, and, if
/// `bodies`, a crate 3D physics drops with a label on it and a 3D
/// character carrying a torch: each pair as (carrier, carried).
fn carriers(flat: bool, bodies: bool) -> (World, Vec<(EntityId, EntityId)>) {
    use area::builtin::{CharacterBody, Collider, RigidBody};
    let mut area = Area::new();
    let mut pairs = Vec::new();
    if flat {
        let walker = area.spawn(
            EntityData::new("Walker")
                .with(&Transform {
                    position: Vec3::new(2.5, -5.5, 0.2),
                    ..Transform::default()
                })
                .with(&Collider2D {
                    character: true,
                    ..Collider2D::default()
                }),
        );
        pairs.push((walker, carry(&mut area, walker, "Lantern")));
    }
    if bodies {
        let falling = area.spawn(
            EntityData::new("Crate")
                .with(&Transform {
                    position: Vec3::new(20.0, 0.0, 10.0),
                    ..Transform::default()
                })
                .with(&RigidBody::default())
                .with(&Collider::default()),
        );
        pairs.push((falling, carry(&mut area, falling, "Label")));
        let hero = area.spawn(
            EntityData::new("Hero")
                .with(&Transform {
                    position: Vec3::new(-20.0, 0.0, 0.0),
                    ..Transform::default()
                })
                .with(&CharacterBody::default()),
        );
        pairs.push((hero, carry(&mut area, hero, "Torch")));
    }
    (World::from_area(&area), pairs)
}

/// One tick: every carrier that walks is asked to, and physics steps.
/// Returns where each carrier was before.
fn carry_on(world: &mut World, pairs: &[(EntityId, EntityId)]) -> Vec<glam::Mat4> {
    let before = pairs
        .iter()
        .map(|(carrier, _)| world.global(*carrier).unwrap())
        .collect();
    for (carrier, _) in pairs {
        world.move_character(*carrier, Vec3::new(0.1, 0.05, 0.0));
    }
    world.step_physics();
    before
}

/// Everything carried is exactly at its carrier's place and its own
/// offset, the checksum is of a world with nothing left to settle, and
/// what drawing between ticks moves from is where it is.
fn all_carried_are_with_their_carriers(world: &mut World, pairs: &[(EntityId, EntityId)]) {
    for &(carrier, held) in pairs {
        let expected = world.global(carrier).unwrap() * carried().matrix();
        assert_eq!(world.global(held), Some(expected));
    }
    let sum = world.checksum();
    world.settle();
    assert_eq!(world.checksum(), sum, "nothing was left to settle");
    world.remember();
    for &(carrier, held) in pairs {
        let expected = world.global(carrier).unwrap() * carried().matrix();
        assert_eq!(world.global_between(held, 0.0), Some(expected));
    }
}

/// The physics step used to return early in a world with no 3D bodies,
/// before those under a walked 2D character had followed it: a camera or
/// a held item was drawn and summed a tick behind (the full review).
#[test]
fn what_a_2d_character_carries_is_with_it_on_the_same_tick_in_a_2d_only_world() {
    let (mut world, pairs) = carriers(true, false);
    assert!(world.physics().state().is_empty(), "no 3D bodies at all");
    for _ in 0..5 {
        let before = carry_on(&mut world, &pairs);
        assert_ne!(world.global(pairs[0].0).unwrap(), before[0], "it walked");
        all_carried_are_with_their_carriers(&mut world, &pairs);
    }
    // Paused (the game's speed 0), a walk asked for still lands, and
    // what's carried still goes with it.
    world.set_speed(0.0);
    let before = carry_on(&mut world, &pairs);
    assert_ne!(world.global(pairs[0].0).unwrap(), before[0]);
    all_carried_are_with_their_carriers(&mut world, &pairs);
}

#[test]
fn what_3d_physics_moves_carries_what_s_under_it_on_the_same_tick_in_a_3d_only_world() {
    let (mut world, pairs) = carriers(false, true);
    assert!(world.flat().state().is_empty(), "no 2D physics at all");
    for _ in 0..5 {
        let before = carry_on(&mut world, &pairs);
        for (i, (carrier, _)) in pairs.iter().enumerate() {
            // The crate fell (physics moved it) and the hero walked.
            assert_ne!(world.global(*carrier).unwrap(), before[i]);
        }
        all_carried_are_with_their_carriers(&mut world, &pairs);
    }
}

#[test]
fn in_a_world_with_both_everything_carried_is_with_its_carrier_on_the_same_tick() {
    let (mut world, pairs) = carriers(true, true);
    for _ in 0..5 {
        let before = carry_on(&mut world, &pairs);
        for (i, (carrier, _)) in pairs.iter().enumerate() {
            assert_ne!(world.global(*carrier).unwrap(), before[i]);
        }
        all_carried_are_with_their_carriers(&mut world, &pairs);
    }
    // Paused, 3D physics stands still; the 2D walk still lands.
    world.set_speed(0.0);
    let before = carry_on(&mut world, &pairs);
    assert_ne!(world.global(pairs[0].0).unwrap(), before[0]);
    assert_eq!(world.global(pairs[1].0).unwrap(), before[1], "the crate");
    all_carried_are_with_their_carriers(&mut world, &pairs);
}
