//! Tile maps drawn: each layer's picture in its terrains' colours, kept
//! until its layer changes; the batches under and over sprites; finding
//! the cell at a point; and the overlay's menu.

use super::*;
use reflect::Id;
use std::collections::HashSet;

fn tileset() -> (AssetId, tiles::Tileset) {
    (AssetId::random(), crate::workbench_tiles::starter_tileset())
}

/// Sheets holding `set` as `id`.
fn sheets_with(id: AssetId, set: tiles::Tileset) -> SpriteSheets {
    let mut sheets = SpriteSheets::default();
    sheets.tilesets.insert(
        "farm.soltileset".into(),
        crate::sprite_sheets::TilesetState::Read(Arc::new(set), Vec::new()),
    );
    sheets.paths.insert(id, "farm.soltileset".into());
    sheets
}

fn entity() -> EntityId {
    EntityId(Id::from_raw(7).unwrap())
}

#[test]
fn a_layer_is_a_texel_per_cell_in_its_terrain_s_colour() {
    let (id, set) = tileset();
    let mut map = TileMap::new([3, 2], Some(id), Some(b'.'));
    map.set_code(0, 1, 0, b't').unwrap();
    map.set_code(0, 2, 1, b'?').unwrap();
    let rgba = layer_rgba(&map, 0, Some(&set));
    assert_eq!(rgba.len(), 3 * 2 * 4);
    let grass = set
        .by_code('.')
        .unwrap()
        .colour
        .map(|c| (c * 255.0).round() as u8);
    assert_eq!(rgba[0..3], grass);
    assert_eq!(rgba[3], 255);
    assert_ne!(rgba[4..7], grass, "tilled");
    assert_eq!(rgba[20..24], MISSING, "a code not in the tileset");
    // An empty layer is see-through.
    assert!(layer_rgba(&map, 1, Some(&set)).iter().all(|&b| b == 0));
    // No tileset: every cell missing.
    assert_eq!(layer_rgba(&map, 0, None)[0..4], MISSING);
}

#[test]
fn layers_under_come_first_and_over_last_hidden_ones_left_out() {
    let (id, set) = tileset();
    let sheets = sheets_with(id, set);
    let map = Arc::new(TileMap::new([4, 4], Some(id), Some(b'.')));
    let maps = [ViewTileMap {
        entity: entity(),
        transform: Mat4::from_translation(Vec3::new(2.0, 3.0, 0.0)),
        map: Arc::clone(&map),
    }];
    let pictures = TilePictures::default();
    let (pics, batches) = pictures.draw(&maps, &sheets, [1.0, 1.0], &HashSet::new());
    assert_eq!(pics.len(), 3);
    assert_eq!(batches.len(), 3);
    let flags: Vec<u32> = batches.iter().map(|b| b.sprites[0].flags).collect();
    assert_eq!(
        flags,
        [UNDER, UNDER, OVER],
        "Ground, Decoration, then Above"
    );
    // Its top-left corner at the entity; a cell a metre across, rows
    // down the world's -Y.
    let s = &batches[0].sprites[0];
    assert_eq!(s.corner, [2.0, 3.0, 0.0]);
    assert_eq!(s.axis_x, [1.0, 0.0]);
    assert_eq!(s.axis_y, [0.0, -1.0]);
    assert_eq!(s.rect, [0.0, 0.0, 4.0, 4.0]);
    // Hidden in the editor: not drawn.
    let hidden = HashSet::from([map.layers()[0].id]);
    let (pics, batches) = pictures.draw(&maps, &sheets, [1.0, 1.0], &hidden);
    assert_eq!((pics.len(), batches.len()), (2, 2));
}

#[test]
fn a_picture_is_made_again_only_when_its_layer_changes() {
    let (id, set) = tileset();
    let sheets = sheets_with(id, set);
    let mut map = TileMap::new([4, 4], Some(id), Some(b'.'));
    let view = |map: &TileMap| {
        [ViewTileMap {
            entity: entity(),
            transform: Mat4::IDENTITY,
            map: Arc::new(map.clone()),
        }]
    };
    let pictures = TilePictures::default();
    let (first, _) = pictures.draw(&view(&map), &sheets, [1.0, 1.0], &HashSet::new());
    let (again, _) = pictures.draw(&view(&map), &sheets, [1.0, 1.0], &HashSet::new());
    for (a, b) in first.iter().zip(&again) {
        assert!(Arc::ptr_eq(a, b), "kept");
    }
    map.set_code(1, 0, 0, b't').unwrap();
    let (changed, _) = pictures.draw(&view(&map), &sheets, [1.0, 1.0], &HashSet::new());
    assert!(Arc::ptr_eq(&first[0], &changed[0]), "Ground unchanged");
    assert!(
        !Arc::ptr_eq(&first[1], &changed[1]),
        "Decoration made again"
    );
    assert_ne!(first[1].key, changed[1].key, "so it's uploaded again");
    assert_eq!(first[1].id, changed[1].id, "under the same ID");
    // A map gone: its pictures are let go.
    let _ = pictures.draw(&[], &sheets, [1.0, 1.0], &HashSet::new());
    assert!(pictures.made.lock().unwrap().is_empty());
}

#[test]
fn the_cell_at_a_point_and_its_corners() {
    let map = TileMap::new([10, 5], None, None);
    let at = Mat4::from_translation(Vec3::new(4.0, 2.0, 0.0));
    let cell = [0.5, 0.5];
    assert_eq!(
        cell_at(&map, at, cell, Vec3::new(4.1, 1.9, 0.0)),
        Some([0, 0])
    );
    assert_eq!(
        cell_at(&map, at, cell, Vec3::new(5.3, 0.6, 0.0)),
        Some([2, 2])
    );
    // Off the map, left, above and past its end.
    assert_eq!(cell_at(&map, at, cell, Vec3::new(3.9, 1.9, 0.0)), None);
    assert_eq!(cell_at(&map, at, cell, Vec3::new(4.1, 2.1, 0.0)), None);
    assert_eq!(cell_at(&map, at, cell, Vec3::new(9.1, 1.9, 0.0)), None);
    assert_eq!(cell_at(&map, at, cell, Vec3::new(4.1, -0.6, 0.0)), None);
    assert_eq!(cell_at(&map, at, cell, Vec3::NAN), None);
    let [a, b] = cell_corners(at, cell, [2, 2]);
    assert_eq!(a, Vec3::new(5.0, 1.0, 0.0));
    assert_eq!(b, Vec3::new(5.5, 0.5, 0.0));
}

#[test]
fn the_overlay_menu_asks_for_values_and_chunks_and_finds_the_cell() {
    use egui_kittest::kittest::Queryable;
    let mut map = TileMap::new([8, 8], None, Some(b'.'));
    map.add_value("growth", tiles::ValueKind::Number).unwrap();
    map.set_value(0, 1, 1, tiles::CellValue::Number(5)).unwrap();
    let overlay = TileOverlay {
        map: Some(ViewTileMap {
            entity: entity(),
            transform: Mat4::IDENTITY,
            map: Arc::new(map),
        }),
        values: vec![0],
        ..TileOverlay::default()
    };
    // Looking straight down at the map, the pointer on cell (1, 1).
    let mut camera = crate::camera::Camera::default().as_kind(crate::camera::ViewKind::TwoD);
    camera.position.x = 4.0;
    camera.position.y = -4.0;
    camera.flat_height = 10.0;
    let mut h = egui_kittest::Harness::builder()
        .with_size(egui::vec2(400.0, 400.0))
        .build_ui_state(
            move |ui, o: &mut (TileOverlay, Option<egui::Rect>)| {
                let rect = ui.max_rect();
                let mouse = camera.project(
                    Vec3::new(1.5, -1.5, 0.0),
                    glam::Vec2::new(rect.width(), rect.height()),
                );
                o.1 = draw_overlay(
                    ui,
                    rect,
                    &camera,
                    &mut o.0,
                    [1.0, 1.0],
                    mouse,
                    crate::theme::Accent::default(),
                );
                // What the workbench does each frame.
                o.0.asked.retain(|_| false);
            },
            (overlay, None),
        );
    h.run();
    assert_eq!(h.state().0.hovered, Some([1, 1]));
    assert!(h.state().1.is_some(), "the menu's place is given back");
    assert!(h.query_by_label("Values: growth ▾").is_some());
    h.get_by_label("Chunks").click();
    h.run_steps(1);
    assert!(h.state().0.chunks_asked);
}

#[test]
fn no_map_selected_no_overlay() {
    let mut h = egui_kittest::Harness::builder()
        .with_size(egui::vec2(200.0, 200.0))
        .build_ui_state(
            |ui, o: &mut (TileOverlay, bool)| {
                let rect = ui.max_rect();
                o.1 = draw_overlay(
                    ui,
                    rect,
                    &crate::camera::Camera::default(),
                    &mut o.0,
                    [1.0, 1.0],
                    Some(glam::Vec2::ONE),
                    crate::theme::Accent::default(),
                )
                .is_some();
            },
            (TileOverlay::default(), true),
        );
    h.run();
    assert!(!h.state().1);
    assert_eq!(h.state().0.hovered, None);
}
