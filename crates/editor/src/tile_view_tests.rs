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
        "farm.tileset.lst".into(),
        crate::sprite_sheets::TilesetState::Read(Arc::new(set), Vec::new()),
    );
    sheets.paths.insert(id, "farm.tileset.lst".into());
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
    let (pics, batches) = pictures.draw(
        &maps,
        &sheets,
        [1.0, 1.0],
        &HashSet::new(),
        &|_| None,
        0,
        None,
        Viewer::View(0),
    );
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
    let (pics, batches) = pictures.draw(
        &maps,
        &sheets,
        [1.0, 1.0],
        &hidden,
        &|_| None,
        0,
        None,
        Viewer::View(0),
    );
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
    let (first, _) = pictures.draw(
        &view(&map),
        &sheets,
        [1.0, 1.0],
        &HashSet::new(),
        &|_| None,
        0,
        None,
        Viewer::View(0),
    );
    let (again, _) = pictures.draw(
        &view(&map),
        &sheets,
        [1.0, 1.0],
        &HashSet::new(),
        &|_| None,
        0,
        None,
        Viewer::View(0),
    );
    for (a, b) in first.iter().zip(&again) {
        assert!(Arc::ptr_eq(a, b), "kept");
    }
    map.set_code(1, 0, 0, b't').unwrap();
    let (changed, _) = pictures.draw(
        &view(&map),
        &sheets,
        [1.0, 1.0],
        &HashSet::new(),
        &|_| None,
        0,
        None,
        Viewer::View(0),
    );
    assert!(Arc::ptr_eq(&first[0], &changed[0]), "Ground unchanged");
    assert!(
        !Arc::ptr_eq(&first[1], &changed[1]),
        "Decoration made again"
    );
    assert_ne!(first[1].key, changed[1].key, "so it's uploaded again");
    assert_eq!(first[1].id, changed[1].id, "under the same ID");
    // A map gone: its pictures are let go.
    let _ = pictures.draw(
        &[],
        &sheets,
        [1.0, 1.0],
        &HashSet::new(),
        &|_| None,
        0,
        None,
        Viewer::View(0),
    );
    assert!(pictures.made.lock().unwrap().is_empty());
}

/// The review, BUG-10: a layer's picture knew what it was made from by
/// where the layer's cells and the tileset were in memory. A map or a
/// tileset read again is another one, its layers at the same revisions,
/// and can be put right where the last one was once that is let go: the
/// picture was then kept, and showed the old one.
#[test]
fn another_map_or_tileset_just_like_the_last_is_never_taken_for_it() {
    let (id, set) = tileset();
    let pictures = TilePictures::default();
    let draw = |map: &Arc<TileMap>, sheets: &SpriteSheets| {
        let maps = [ViewTileMap {
            entity: entity(),
            transform: Mat4::IDENTITY,
            map: Arc::clone(map),
        }];
        pictures
            .draw(
                &maps,
                sheets,
                [1.0, 1.0],
                &HashSet::new(),
                &|_| None,
                0,
                None,
                Viewer::View(0),
            )
            .0
    };
    let sheets = sheets_with(id, set.clone());
    let map = Arc::new(TileMap::new([4, 4], Some(id), Some(b'.')));
    let first = draw(&map, &sheets);
    let kept = draw(&map, &sheets);
    assert!(
        Arc::ptr_eq(&first[0], &kept[0]),
        "kept while nothing changes"
    );
    // The map let go, then another made just like it (its file read
    // again): its layers are at the same revisions, and it's drawn afresh.
    let revisions = (0..3).map(|i| map.layer_revision(i)).collect::<Vec<_>>();
    drop(map);
    let map = Arc::new(TileMap::new([4, 4], Some(id), Some(b'.')));
    assert_eq!(
        (0..3).map(|i| map.layer_revision(i)).collect::<Vec<_>>(),
        revisions
    );
    let again = draw(&map, &sheets);
    for (a, b) in first.iter().zip(&again) {
        assert_ne!(a.key, b.key, "made again, and uploaded again");
        assert_eq!(a.id, b.id, "under the same ID");
    }
    // The tileset let go, then another just like it: the same.
    drop(sheets);
    let sheets = sheets_with(id, set);
    let with_new_set = draw(&map, &sheets);
    for (a, b) in again.iter().zip(&with_new_set) {
        assert_ne!(a.key, b.key);
    }
    let kept = draw(&map, &sheets);
    assert!(Arc::ptr_eq(&with_new_set[0], &kept[0]), "and kept again");
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
            move |ui, o: &mut TileOverlay| {
                let rect = ui.max_rect();
                let mouse = camera.project(
                    Vec3::new(1.5, -1.5, 0.0),
                    glam::Vec2::new(rect.width(), rect.height()),
                );
                draw_overlay(
                    ui,
                    rect,
                    &camera,
                    o,
                    [1.0, 1.0],
                    mouse,
                    crate::theme::Accent::default(),
                );
            },
            overlay,
        );
    h.run();
    assert_eq!(h.state().hovered, Some([1, 1]));
    // What is shown is chosen on the ribbon now: nothing over the view
    // to click (round 3).
    assert!(h.query_by_label_contains("Values").is_none());
    assert!(h.query_by_label("Chunks").is_none());
}

#[test]
fn no_map_selected_no_overlay() {
    let mut h = egui_kittest::Harness::builder()
        .with_size(egui::vec2(200.0, 200.0))
        .build_ui_state(
            |ui, o: &mut TileOverlay| {
                let rect = ui.max_rect();
                draw_overlay(
                    ui,
                    rect,
                    &crate::camera::Camera::default(),
                    o,
                    [1.0, 1.0],
                    Some(glam::Vec2::ONE),
                    crate::theme::Accent::default(),
                );
            },
            TileOverlay::default(),
        );
    h.run();
    assert_eq!(h.state().hovered, None);
}

/// Sheets whose tileset (`id`) has art on a 2 × 1 picture of 4-pixel
/// tiles: grass's one piece animated over both columns, every 100 ms.
fn sheets_with_art() -> (SpriteSheets, AssetId) {
    let (id, mut set) = tileset();
    let png = AssetId::random();
    set.picture = Some(png);
    set.tile = [4, 4];
    set.pieces.push(tiles::Piece {
        id: Id::random(),
        terrain: set.terrains[0].id,
        slot: 0,
        at: [0, 0],
        weight: 1.0,
        animation: tiles::Animation::Columns { count: 2, ms: 100 },
    });
    let mut sheets = sheets_with(id, set);
    sheets.tile_art.insert(
        "tiles.png".into(),
        crate::sprite_sheets::ArtState::Read(Arc::new(assets::tile_art::TileArt {
            size: [8, 4],
            frames: vec![vec![9; 8 * 4 * 4], vec![7; 8 * 4 * 4]],
            durations: vec![100, 100],
        })),
    );
    sheets.paths.insert(png, "tiles.png".into());
    (sheets, id)
}

#[test]
fn a_picture_s_atlas_stacks_its_frames_over_a_white_patch() {
    let art = assets::tile_art::TileArt {
        size: [2, 1],
        frames: vec![vec![1; 8], vec![2; 8]],
        durations: vec![100, 100],
    };
    let ([w, h], rgba) = atlas_rgba(&art);
    assert_eq!(
        (w, h),
        (4, 1 + 1 + 4),
        "as wide as the patch, the frames then the patch"
    );
    assert_eq!(rgba[0], 1);
    assert_eq!(
        rgba[(w * 4) as usize],
        2,
        "the second frame below the first"
    );
    assert_eq!(
        rgba[(2 * w * 4) as usize..(2 * w * 4 + 4) as usize],
        [255; 4]
    );
}

#[test]
fn with_art_a_map_is_drawn_tile_by_tile_and_its_tiles_animate() {
    let (sheets, id) = sheets_with_art();
    let map = Arc::new(TileMap::new([3, 2], Some(id), Some(b'.')));
    let maps = [ViewTileMap {
        entity: entity(),
        transform: Mat4::IDENTITY,
        map,
    }];
    let pictures = TilePictures::default();
    let (pics, batches) = pictures.draw(
        &maps,
        &sheets,
        [1.0, 1.0],
        &HashSet::new(),
        &|_| None,
        0,
        None,
        Viewer::View(0),
    );
    assert_eq!(pics.len(), 1, "one atlas, no layer pictures");
    assert_eq!(batches.len(), 1, "the empty layers draw nothing");
    let sprites = &batches[0].sprites;
    assert_eq!(sprites.len(), 6);
    assert_eq!(sprites[0].rect, [0.0, 0.0, 4.0, 4.0]);
    assert_eq!(
        sprites[0].axis_x,
        [0.25, 0.0],
        "a texel a quarter of a cell"
    );
    // 150 ms later: the second column.
    let (_, batches) = pictures.draw(
        &maps,
        &sheets,
        [1.0, 1.0],
        &HashSet::new(),
        &|_| None,
        150,
        None,
        Viewer::View(0),
    );
    assert_eq!(batches[0].sprites[0].rect, [4.0, 0.0, 4.0, 4.0]);
}

/// BUG-10, for art: a picture's atlas went by where the picture was in
/// memory, and so did the ID the graphics card keeps it under. A picture
/// read again (painted on since) could be put where the last one was:
/// the old atlas was kept, and the card never got the new one.
#[test]
fn a_picture_read_again_gets_an_atlas_of_its_own_and_its_tiles_are_made_again() {
    let (mut sheets, id) = sheets_with_art();
    let map = Arc::new(TileMap::new([3, 2], Some(id), Some(b'.')));
    let maps = [ViewTileMap {
        entity: entity(),
        transform: Mat4::IDENTITY,
        map,
    }];
    let pictures = TilePictures::default();
    let draw = |sheets: &SpriteSheets| {
        pictures.draw(
            &maps,
            sheets,
            [1.0, 1.0],
            &HashSet::new(),
            &|_| None,
            0,
            None,
            Viewer::View(0),
        )
    };
    let (first, first_batches) = draw(&sheets);
    let (kept, kept_batches) = draw(&sheets);
    assert!(Arc::ptr_eq(&first[0], &kept[0]), "one atlas, kept");
    assert!(Arc::ptr_eq(
        &first_batches[0].sprites,
        &kept_batches[0].sprites
    ));
    // The picture let go, then read again: the same size, painted on.
    let art = |fill: u8| {
        crate::sprite_sheets::ArtState::Read(Arc::new(assets::tile_art::TileArt {
            size: [8, 4],
            frames: vec![vec![fill; 8 * 4 * 4], vec![7; 8 * 4 * 4]],
            durations: vec![100, 100],
        }))
    };
    sheets.tile_art.remove("tiles.png");
    sheets.tile_art.insert("tiles.png".into(), art(200));
    let (again, again_batches) = draw(&sheets);
    assert_eq!(again.len(), 1);
    assert_eq!(again[0].rgba[0], 200, "the atlas of the picture as it is");
    assert_ne!(again[0].id, first[0].id, "under an ID of its own");
    assert_ne!(again[0].key, first[0].key);
    assert_eq!(
        again_batches[0].sheet, again[0].id,
        "the tiles drawn from it"
    );
    assert!(!Arc::ptr_eq(
        &first_batches[0].sprites,
        &again_batches[0].sprites
    ));
    assert_eq!(
        pictures.atlases.lock().unwrap().len(),
        1,
        "the old one let go"
    );
}

#[test]
fn only_chunks_in_view_are_drawn_and_far_away_maps_are_coloured() {
    let (sheets, id) = sheets_with_art();
    let map = Arc::new(TileMap::new([600, 600], Some(id), Some(b'.')));
    let maps = [ViewTileMap {
        entity: entity(),
        transform: Mat4::IDENTITY,
        map,
    }];
    let pictures = TilePictures::default();
    // A view of the map's first few cells: the chunk there and those
    // beside it.
    let near = |_| Some([Vec2::new(0.0, -10.0), Vec2::new(10.0, 0.0)]);
    let (_, batches) = pictures.draw(
        &maps,
        &sheets,
        [1.0, 1.0],
        &HashSet::new(),
        &near,
        0,
        None,
        Viewer::View(0),
    );
    let drawn: usize = batches.iter().map(|b| b.sprites.len()).sum();
    assert_eq!(drawn, 4 * 1024, "2 × 2 chunks");
    // Above the horizon (everything may show): 361 chunks is too many to
    // draw tile by tile, so each layer is one coloured quad.
    let (pics, batches) = pictures.draw(
        &maps,
        &sheets,
        [1.0, 1.0],
        &HashSet::new(),
        &|_| None,
        0,
        None,
        Viewer::View(0),
    );
    assert!(pics.iter().all(|p| p.size == [600, 600]));
    assert_eq!(batches.iter().map(|b| b.sprites.len()).sum::<usize>(), 3);
}

#[test]
fn missing_tiles_are_pink_and_terrains_without_art_their_colour() {
    let (sheets, id) = sheets_with_art();
    let mut map = TileMap::new([2, 1], Some(id), Some(b'.'));
    map.set_code(0, 1, 0, b't').unwrap();
    // tilled has no art yet: its colour; a code no terrain has: pink.
    map.set_code(1, 0, 0, b'?').unwrap();
    let maps = [ViewTileMap {
        entity: entity(),
        transform: Mat4::IDENTITY,
        map: Arc::new(map),
    }];
    let (_, batches) = TilePictures::default().draw(
        &maps,
        &sheets,
        [1.0, 1.0],
        &HashSet::new(),
        &|_| None,
        0,
        None,
        Viewer::View(0),
    );
    let all: Vec<&ViewSprite> = batches.iter().flat_map(|b| b.sprites.iter()).collect();
    let white = |s: &&&ViewSprite| s.rect[1] == 8.0;
    assert_eq!(
        all.iter().filter(white).count(),
        2,
        "the colour and the pink, from the white patch"
    );
    assert!(all.iter().any(|s| s.tint[..3] == [1.0, 0.0, 1.0]));
}

#[test]
fn the_ground_in_view_is_where_the_corners_land() {
    let mut camera = crate::camera::Camera::default().as_kind(crate::camera::ViewKind::TwoD);
    camera.position.x = 4.0;
    camera.position.y = -4.0;
    camera.flat_height = 10.0;
    let vp = camera.view_projection(1.0);
    let [lo, hi] = ground_in_view(vp, 0.0).unwrap();
    assert!(
        (lo.x - -1.0).abs() < 0.01 && (hi.x - 9.0).abs() < 0.01,
        "{lo} {hi}"
    );
    assert!(
        (lo.y - -9.0).abs() < 0.01 && (hi.y - 1.0).abs() < 0.01,
        "{lo} {hi}"
    );
    // A perspective view looking at the horizon sees past it.
    let level = crate::camera::Camera {
        tilt: 0.0,
        ..crate::camera::Camera::default()
    };
    assert!(ground_in_view(level.view_projection(1.0), 0.0).is_none());
}

#[test]
fn a_layer_s_tiles_are_kept_while_nothing_they_show_changes() {
    // The phase review: every frame copied, sorted and made every tile of
    // every layer in view again.
    let (id, mut set) = tileset();
    let png = AssetId::random();
    set.picture = Some(png);
    set.tile = [4, 4];
    set.pieces.push(tiles::Piece {
        id: Id::random(),
        terrain: set.terrains[0].id,
        slot: 0,
        at: [0, 0],
        weight: 1.0,
        animation: tiles::Animation::None,
    });
    let mut sheets = sheets_with(id, set);
    sheets.tile_art.insert(
        "tiles.png".into(),
        crate::sprite_sheets::ArtState::Read(Arc::new(assets::tile_art::TileArt {
            size: [8, 4],
            frames: vec![vec![9; 8 * 4 * 4]],
            durations: vec![100],
        })),
    );
    sheets.paths.insert(png, "tiles.png".into());
    let who = entity();
    let draw = |pictures: &TilePictures, map: &TileMap, ms| {
        let maps = [ViewTileMap {
            entity: who,
            transform: Mat4::IDENTITY,
            map: Arc::new(map.clone()),
        }];
        pictures
            .draw(
                &maps,
                &sheets,
                [1.0, 1.0],
                &HashSet::new(),
                &|_| None,
                ms,
                None,
                Viewer::View(0),
            )
            .1
    };
    let mut map = TileMap::new([3, 2], Some(id), Some(b'.'));
    let pictures = TilePictures::default();
    let first = draw(&pictures, &map, 0);
    let again = draw(&pictures, &map, 150);
    assert!(
        Arc::ptr_eq(&first[0].sprites, &again[0].sprites),
        "the same tiles, kept (nothing moves in this tileset)"
    );
    map.set_code(0, 1, 1, b'?').unwrap();
    let changed = draw(&pictures, &map, 150);
    assert!(!Arc::ptr_eq(&first[0].sprites, &changed[0].sprites));
}

/// Sheets whose tileset has a picture with one piece that doesn't move,
/// for tests of what's kept.
fn sheets_with_still_art() -> (SpriteSheets, AssetId) {
    let (id, mut set) = tileset();
    let png = AssetId::random();
    set.picture = Some(png);
    set.tile = [4, 4];
    set.pieces.push(tiles::Piece {
        id: Id::random(),
        terrain: set.terrains[0].id,
        slot: 0,
        at: [0, 0],
        weight: 1.0,
        animation: tiles::Animation::None,
    });
    let mut sheets = sheets_with(id, set);
    sheets.tile_art.insert(
        "tiles.png".into(),
        crate::sprite_sheets::ArtState::Read(Arc::new(assets::tile_art::TileArt {
            size: [8, 4],
            frames: vec![vec![9; 8 * 4 * 4]],
            durations: vec![100],
        })),
    );
    sheets.paths.insert(png, "tiles.png".into());
    (sheets, id)
}

/// The ground a view sees: `x` metres along the map's top edge, ten
/// metres square.
fn ground_at(x: f32) -> impl Fn(f32) -> Option<[Vec2; 2]> {
    move |_| Some([Vec2::new(x, -10.0), Vec2::new(x + 10.0, 0.0)])
}

/// Each chunk's tiles as kept now.
fn chunk_tiles_kept(pictures: &TilePictures) -> HashMap<ChunkId, Arc<Vec<Placed>>> {
    lock(&pictures.chunks)
        .iter()
        .map(|((id, _), tiles)| (*id, Arc::clone(tiles)))
        .collect()
}

/// BUG-19: every view of the viewport and the Game panel draw with one
/// cache, one after another each frame, and each draw kept only what it
/// had drawn itself. Two views of one map threw out each other's tiles,
/// so every frame each view's chunks were autotiled, sorted and made into
/// sprites again.
#[test]
fn views_drawn_one_after_another_keep_each_other_s_tiles() {
    let (sheets, id) = sheets_with_still_art();
    let maps = [ViewTileMap {
        entity: entity(),
        transform: Mat4::IDENTITY,
        map: Arc::new(TileMap::new([320, 320], Some(id), Some(b'.'))),
    }];
    let pictures = TilePictures::default();
    let (left, right) = (ground_at(0.0), ground_at(200.0));
    let draw = |ground: &dyn Fn(f32) -> Option<[Vec2; 2]>, viewer| {
        let (_, batches) = pictures.draw(
            &maps,
            &sheets,
            [1.0, 1.0],
            &HashSet::new(),
            ground,
            0,
            None,
            viewer,
        );
        Arc::clone(&batches[0].sprites)
    };
    // One frame: two views and the Game panel, each seeing its own part.
    let first = [
        draw(&left, Viewer::View(0)),
        draw(&right, Viewer::View(1)),
        draw(&|_| None, Viewer::Game),
    ];
    let kept = chunk_tiles_kept(&pictures);
    assert_eq!(
        kept.len(),
        3 * 100,
        "every chunk of each layer: the Game panel sees them all"
    );
    // The next frames: nothing changed, so nothing is made again.
    for _ in 0..3 {
        let again = [
            draw(&left, Viewer::View(0)),
            draw(&right, Viewer::View(1)),
            draw(&|_| None, Viewer::Game),
        ];
        for (view, (a, b)) in first.iter().zip(&again).enumerate() {
            assert!(Arc::ptr_eq(a, b), "view {view}'s sprites kept");
        }
        let now = chunk_tiles_kept(&pictures);
        assert_eq!(now.len(), kept.len());
        for (chunk, tiles) in &kept {
            assert!(Arc::ptr_eq(tiles, &now[chunk]), "{chunk:?} autotiled again");
        }
    }
}

/// What a view drew with is let go once it stops drawing (a view hidden,
/// the Game panel closed), and what no view draws any more is let go at
/// once.
#[test]
fn a_viewer_that_stops_drawing_lets_go_of_its_tiles() {
    let (sheets, id) = sheets_with_still_art();
    let maps = [ViewTileMap {
        entity: entity(),
        transform: Mat4::IDENTITY,
        map: Arc::new(TileMap::new([320, 320], Some(id), Some(b'.'))),
    }];
    let pictures = TilePictures::default();
    let (left, right) = (ground_at(0.0), ground_at(200.0));
    let draw = |ground: &dyn Fn(f32) -> Option<[Vec2; 2]>, viewer| {
        let _ = pictures.draw(
            &maps,
            &sheets,
            [1.0, 1.0],
            &HashSet::new(),
            ground,
            0,
            None,
            viewer,
        );
    };
    let columns = |p: &TilePictures| {
        let mut c: Vec<u32> = chunk_tiles_kept(p).keys().map(|k| k.2[0]).collect();
        c.sort_unstable();
        c.dedup();
        c
    };
    draw(&left, Viewer::View(0));
    draw(&right, Viewer::View(1));
    assert_eq!(columns(&pictures), [0, 1, 5, 6, 7], "both views' chunks");
    assert_eq!(lock(&pictures.layers).len(), 2 * 3, "each view's layers");
    // View 0 looks elsewhere: what it saw before goes, view 1's stays.
    draw(&ground_at(100.0), Viewer::View(0));
    assert_eq!(columns(&pictures), [2, 3, 4, 5, 6, 7]);
    // View 1 hidden: kept only while it might still draw again.
    for _ in 1..FORGET_AFTER {
        draw(&ground_at(100.0), Viewer::View(0));
    }
    assert_eq!(columns(&pictures), [2, 3, 4]);
    assert_eq!(lock(&pictures.layers).len(), 3);
    assert_eq!(lock(&pictures.viewers).used.len(), 1);
}

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
fn scale_check_a_tile_map_in_view_frame_after_frame() {
    // The phase review: a big map in view was rebuilt and sorted every
    // frame. Now a frame where nothing changed reuses each layer's tiles.
    const TARGET_MS: f64 = 1.0;
    let (id, mut set) = tileset();
    let png = AssetId::random();
    set.picture = Some(png);
    set.tile = [4, 4];
    set.pieces.push(tiles::Piece {
        id: Id::random(),
        terrain: set.terrains[0].id,
        slot: 0,
        at: [0, 0],
        weight: 1.0,
        animation: tiles::Animation::None,
    });
    let mut sheets = sheets_with(id, set);
    sheets.tile_art.insert(
        "tiles.png".into(),
        crate::sprite_sheets::ArtState::Read(Arc::new(assets::tile_art::TileArt {
            size: [8, 4],
            frames: vec![vec![9; 8 * 4 * 4]],
            durations: vec![100],
        })),
    );
    sheets.paths.insert(png, "tiles.png".into());
    // As many chunks as are drawn tile by tile, each full.
    let side = 32 * 8;
    let mut map = TileMap::new([side, side], Some(id), Some(b'.'));
    map.set_code(1, 3, 3, b'.').unwrap();
    let maps = [ViewTileMap {
        entity: entity(),
        transform: Mat4::IDENTITY,
        map: Arc::new(map),
    }];
    let pictures = TilePictures::default();
    let draw = || {
        pictures.draw(
            &maps,
            &sheets,
            [1.0, 1.0],
            &HashSet::new(),
            &|_| None,
            0,
            None,
            Viewer::View(0),
        )
    };
    let (_, first) = draw();
    let tiles: usize = first.iter().map(|b| b.sprites.len()).sum();
    assert!(tiles >= (side * side) as usize, "{tiles}");
    let start = std::time::Instant::now();
    for _ in 0..10 {
        let _ = draw();
    }
    let ms = start.elapsed().as_secs_f64() * 1000.0 / 10.0;
    let verdict = if ms <= TARGET_MS { "ok" } else { "SLOW" };
    println!(
        "Scale check: a {side} × {side} tile map in view, a frame with nothing changed {ms:.3} ms \
         (target {TARGET_MS} ms) {verdict}"
    );
    assert!(ms <= TARGET_MS, "{ms:.3} ms");
}

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
fn scale_check_two_views_and_the_game_panel_of_a_tile_map() {
    // BUG-19: views drawn one after another threw out each other's tiles,
    // so each view made its tiles again every frame. A millisecond a view.
    const TARGET_MS: f64 = 3.0;
    let (sheets, id) = sheets_with_still_art();
    // Three views side by side, each as many chunks as are drawn tile by
    // tile (with the one more each way), each full.
    let side = 32 * 48;
    let mut map = TileMap::new([side, side], Some(id), Some(b'.'));
    map.set_code(1, 3, 3, b'.').unwrap();
    let maps = [ViewTileMap {
        entity: entity(),
        transform: Mat4::IDENTITY,
        map: Arc::new(map),
    }];
    let pictures = TilePictures::default();
    let seen = 13.0 * 32.0;
    let views = [
        (0.0, Viewer::View(0)),
        (512.0, Viewer::View(1)),
        (1024.0, Viewer::Game),
    ];
    let frame = || {
        let mut tiles = 0;
        for &(x, viewer) in &views {
            let (_, batches) = pictures.draw(
                &maps,
                &sheets,
                [1.0, 1.0],
                &HashSet::new(),
                &|_| Some([Vec2::new(x, -seen), Vec2::new(x + seen, 0.0)]),
                0,
                None,
                viewer,
            );
            tiles += batches.iter().map(|b| b.sprites.len()).sum::<usize>();
        }
        tiles
    };
    let tiles = frame();
    assert!(tiles >= 3 * 200 * 1024, "{tiles}");
    let start = std::time::Instant::now();
    for _ in 0..10 {
        let _ = frame();
    }
    let ms = start.elapsed().as_secs_f64() * 1000.0 / 10.0;
    let verdict = if ms <= TARGET_MS { "ok" } else { "SLOW" };
    println!(
        "Scale check: two views and the Game panel of a {side} × {side} tile map, each seeing \
         its own part, a frame with nothing changed {ms:.3} ms (target {TARGET_MS} ms) {verdict}"
    );
    assert!(ms <= TARGET_MS, "{ms:.3} ms");
}

/// A selected 2,048 × 2,048 map seen whole, with a wall in every chunk
/// and a switch on in a cell of each, for the overlay's scale check.
fn big_overlay() -> TileOverlay {
    let side = 2048;
    let mut map = TileMap::new([side, side], None, Some(b'.'));
    map.add_value("watered", tiles::ValueKind::Switch).unwrap();
    for cy in 0..side / tiles::CHUNK {
        for cx in 0..side / tiles::CHUNK {
            let (x, y) = (cx * tiles::CHUNK + 3, cy * tiles::CHUNK + 5);
            map.set_code(1, x, y, b'#').unwrap();
            map.set_value(0, x, y, tiles::CellValue::Switch(true))
                .unwrap();
        }
    }
    let mut solid = [false; 256];
    solid[usize::from(b'#')] = true;
    TileOverlay {
        map: Some(ViewTileMap {
            entity: entity(),
            transform: Mat4::IDENTITY,
            map: Arc::new(map),
        }),
        values: vec![0],
        walls: true,
        solid: Some(solid),
        ..TileOverlay::default()
    }
}

/// Draws `overlay` as a 400-point view through `camera` sees it, as many
/// frames as asked; gives it back.
fn overlay_seen(overlay: TileOverlay, camera: crate::camera::Camera, frames: usize) -> TileOverlay {
    let mut h = egui_kittest::Harness::builder()
        .with_size(egui::vec2(400.0, 400.0))
        .build_ui_state(
            move |ui, o: &mut TileOverlay| {
                let rect = ui.max_rect();
                draw_overlay(
                    ui,
                    rect,
                    &camera,
                    o,
                    [1.0, 1.0],
                    None,
                    crate::theme::Accent::default(),
                );
            },
            overlay,
        );
    for _ in 0..frames {
        h.step();
    }
    h.into_state()
}

/// BUG-22: with Walls on, every chunk of the map was looked through for
/// its walls every frame, on screen or not. Now only those in view, and
/// each again only when it changes.
#[test]
fn walls_are_found_only_in_chunks_in_view_and_kept() {
    let mut overlay = big_overlay();
    // Looking straight down at the corner of the map, 100 cells high.
    let mut camera = crate::camera::Camera::default().as_kind(crate::camera::ViewKind::TwoD);
    camera.position.x = 40.0;
    camera.position.y = -40.0;
    camera.flat_height = 100.0;
    overlay = overlay_seen(overlay, camera, 1);
    let first: Vec<([u32; 2], Arc<Vec<tiles::walls::Rect>>)> = overlay
        .walls_kept
        .chunks
        .iter()
        .map(|(c, (_, walls))| (*c, Arc::clone(walls)))
        .collect();
    // Cells 0 to about 90 across and down: chunks 0 to 2 each way.
    assert_eq!(first.len(), 9, "the chunks in view, of 4,096");
    assert!(
        first
            .iter()
            .all(|(c, walls)| c[0] < 3 && c[1] < 3 && walls.len() == 1)
    );
    overlay = overlay_seen(overlay, camera, 3);
    for (chunk, walls) in &first {
        assert!(
            Arc::ptr_eq(walls, &overlay.walls_kept.chunks[chunk].1),
            "{chunk:?} looked through again"
        );
    }
    // A cell made solid (on a copy of the map, as the editor changes
    // one): the walls in view are found again, the new one with them.
    let mut changed = (*overlay.map.as_ref().unwrap().map).clone();
    changed.set_code(0, 40, 40, b'#').unwrap();
    overlay.map.as_mut().unwrap().map = Arc::new(changed);
    overlay = overlay_seen(overlay, camera, 1);
    assert_eq!(overlay.walls_kept.chunks.len(), 9);
    assert_eq!(overlay.walls_kept.chunks[&[1, 1]].1.len(), 2);
}

#[test]
fn values_are_looked_at_only_while_few_enough_cells_are_in_view() {
    assert_eq!(valued_cells(None), None);
    let few = Some((0, 255, 0, 255));
    assert_eq!(valued_cells(few), few, "256 × 256: shown");
    assert_eq!(
        valued_cells(Some((0, 256, 0, 255))),
        None,
        "one column more"
    );
    assert_eq!(
        valued_cells(Some((0, 2047, 0, 2047))),
        None,
        "a big map whole"
    );
    assert_eq!(valued_cells(Some((5, 5, 9, 9))), Some((5, 5, 9, 9)));
}

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
fn scale_check_a_big_selected_map_seen_whole_with_its_walls_and_values() {
    // BUG-22: every frame, the overlay looked for walls in every chunk of
    // the map, on screen or not, and visited every cell in view for its
    // values even when there were far too many to show.
    const TARGET_MS: f64 = 4.0;
    let mut camera = crate::camera::Camera::default().as_kind(crate::camera::ViewKind::TwoD);
    camera.position.x = 1024.0;
    camera.position.y = -1024.0;
    camera.flat_height = 2100.0;
    let mut h = egui_kittest::Harness::builder()
        .with_size(egui::vec2(1200.0, 1200.0))
        .build_ui_state(
            move |ui, o: &mut TileOverlay| {
                let rect = ui.max_rect();
                draw_overlay(
                    ui,
                    rect,
                    &camera,
                    o,
                    [1.0, 1.0],
                    None,
                    crate::theme::Accent::default(),
                );
            },
            big_overlay(),
        );
    h.step();
    let frames = 5;
    let start = std::time::Instant::now();
    for _ in 0..frames {
        h.step();
    }
    let ms = start.elapsed().as_secs_f64() * 1000.0 / f64::from(frames);
    let verdict = if ms <= TARGET_MS { "ok" } else { "SLOW" };
    println!(
        "Scale check: a 2048 × 2048 tile map selected and seen whole, with its walls and \
         values shown, a frame {ms:.2} ms (target {TARGET_MS} ms) {verdict}"
    );
    assert!(ms <= TARGET_MS, "{ms:.2} ms");
}
