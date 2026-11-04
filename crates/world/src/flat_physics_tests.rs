//! 2D physics in the running game: a character walked by game code stops
//! at a map's solid cells and slides along them, walls follow cells the
//! game changes, solid still colliders block, pixels become metres, and
//! all of it is in the checksum and the same every run.

use super::*;
use crate::{MapFiles, TICK};
use area::builtin::{Transform, TileMap as TileMapComponent};
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
    assert_eq!(world.flat().state().wall_count(), 1, "the fence is one wall");
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
    assert!(at.x < 4.0 - 0.25 && at.x > 4.0 - 0.27, "stopped at the bin: {at}");
    // At 32 px a metre everything halves: the bin runs 4.5 to 5.5 and the
    // farmer's circle is an eighth of a metre.
    let mut pixels = area::pixels::PixelSettings::default();
    pixels.pixels_per_unit = 32;
    world.set_pixels(pixels);
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
    assert!(at.x > 11.0 && at.x < 11.0 + 0.25 + 0.02, "the fence's other side: {at}");
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
