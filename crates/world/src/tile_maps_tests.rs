//! Tile maps in the running game: started from their file, changed cell
//! by cell, in the checksum, kept over edits, started again when the file
//! changes, and given back by a save.

use super::*;
use area::builtin::Transform;
use area::{Area, EntityData};
use reflect::AssetRef;
use tiles::ValueKind;

fn file() -> TileMap {
    let mut map = TileMap::new([40, 40], None, Some(b'.'));
    map.add_value("watered", ValueKind::Switch).unwrap();
    map
}

fn farm() -> (World, Area, EntityId, AssetId) {
    let asset = AssetId::random();
    let mut area = Area::new();
    let id = area.spawn(EntityData::new("Farm").with(&Transform::default()).with(
        &TileMapComponent {
            map: Some(AssetRef { asset, part: None }),
        },
    ));
    let mut world = World::from_area(&area);
    let mut files = MapFiles::new();
    files.insert(asset, Arc::new(file()));
    world.set_tile_maps(Arc::new(files));
    (world, area, id, asset)
}

#[test]
fn a_map_starts_from_its_file_and_changes_as_the_game_plays() {
    let (mut world, _, id, _) = farm();
    assert_eq!(world.tile_map(id).unwrap().code(0, 3, 3), Some(b'.'));
    assert_eq!(world.set_tile(id, 0, [3, 3], b't'), Ok(true));
    assert_eq!(
        world.set_cell_value(id, 0, [3, 3], CellValue::Switch(true)),
        Ok(true)
    );
    let map = world.tile_map(id).unwrap();
    assert_eq!(map.code(0, 3, 3), Some(b't'));
    assert_eq!(map.value(0, 3, 3), Some(CellValue::Switch(true)));
    // The file itself is untouched.
    assert_eq!(
        world
            .tile_map_files()
            .values()
            .next()
            .unwrap()
            .code(0, 3, 3),
        Some(b'.')
    );
    assert_eq!(world.set_tile(id, 0, [99, 3], b't'), Err(MapError::Outside));
    assert_eq!(
        world.set_tile(EntityId::random(), 0, [3, 3], b't'),
        Err(MapError::NoSuch)
    );
}

#[test]
fn the_checksum_follows_the_map() {
    let (mut a, _, id, _) = farm();
    let before = a.checksum();
    a.set_tile(id, 0, [1, 1], b't').unwrap();
    assert_ne!(a.checksum(), before);
    a.set_tile(id, 0, [1, 1], b'.').unwrap();
    assert_eq!(a.checksum(), before);
}

#[test]
fn edits_keep_the_game_s_map_and_a_changed_file_starts_it_again() {
    let (mut world, mut area, id, asset) = farm();
    world.set_tile(id, 0, [2, 2], b't').unwrap();
    // Moving the map's entity in the editor keeps the game's cells.
    area.get_mut(id).unwrap().set(&Transform {
        position: glam::Vec3::new(1.0, 0.0, 0.0),
        ..Transform::default()
    });
    world.update(&area, [(id, true)]);
    assert_eq!(world.tile_map(id).unwrap().code(0, 2, 2), Some(b't'));
    world.rebuild(&area);
    assert_eq!(world.tile_map(id).unwrap().code(0, 2, 2), Some(b't'));
    // The file changes (its design, in the editor): it starts again.
    let mut files = MapFiles::new();
    let mut changed = file();
    changed.set_code(0, 0, 0, b'~').unwrap();
    files.insert(asset, Arc::new(changed));
    world.set_tile_maps(Arc::new(files));
    let map = world.tile_map(id).unwrap();
    assert_eq!(map.code(0, 2, 2), Some(b'.'));
    assert_eq!(map.code(0, 0, 0), Some(b'~'));
}

#[test]
fn a_save_s_maps_come_back_and_sum_the_same() {
    let (mut world, area, id, _) = farm();
    world.set_tile(id, 0, [5, 6], b't').unwrap();
    world
        .set_cell_value(id, 0, [5, 6], CellValue::Switch(true))
        .unwrap();
    let saved = world.tile_maps();
    assert_eq!(saved.len(), 1);
    let mut loaded = World::from_area(&area);
    // A save's maps are given before (or after) the files arrive.
    loaded.restore_tile_maps(saved);
    // Read before its file arrives: as the save had it.
    assert_eq!(loaded.tile_map(id).unwrap().code(0, 5, 6), Some(b't'));
    assert_eq!(loaded.checksum(), world.checksum());
    loaded.set_tile_maps(Arc::clone(world.tile_map_files()));
    assert_eq!(loaded.tile_map(id).unwrap().code(0, 5, 6), Some(b't'));
    assert_eq!(loaded.checksum(), world.checksum());
}

#[test]
fn an_entity_without_a_file_has_no_map() {
    let mut area = Area::new();
    let id = area.spawn(EntityData::new("Empty").with(&TileMapComponent::default()));
    let mut world = World::from_area(&area);
    world.set_tile_maps(Arc::new(MapFiles::new()));
    assert!(world.tile_map(id).is_none());
    assert!(world.tile_maps().is_empty());
}
