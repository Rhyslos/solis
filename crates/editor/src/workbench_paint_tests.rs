//! The tile painter in the whole editor: each tool's stroke saved as one
//! undo step, the right button emptying, Alt picking a terrain, the
//! eyedropper's copy flipped and painted, exact tiles, Esc, the keys, the
//! tool bar, and painting off while playing.

use super::*;
use crate::test_support::*;
use crate::tile_paint::{Button, PaintInput};
use crate::workbench_tiles::tests::{saved_maps, the_map, with_map};
use egui_kittest::kittest::Queryable;
use reflect::Id;

/// The farm map placed and selected, painting with `tool`.
fn painting(tool: PaintTool) -> Workbench {
    let (mut w, map, _) = with_map();
    w.place_tile_map(map, None).unwrap();
    w.follow_map_selection();
    w.set_tile_overlay();
    w.viewport.tile_overlay.paint.tool = Some(tool);
    w.outbox.clear();
    w
}

/// One frame with the pointer on `at`, the left (or right) button down
/// (`pressed` on its first frame), and Ctrl or Alt held.
fn frame(w: &mut Workbench, at: [i64; 2], down: bool, right: bool, pressed: bool, keys: [bool; 2]) {
    w.set_tile_overlay();
    let button = Button { pressed, down };
    let overlay = &mut w.viewport.tile_overlay;
    overlay.pointer = Some(at);
    overlay.input = PaintInput {
        on_view: true,
        left: if right { Button::default() } else { button },
        right: if right { button } else { Button::default() },
        ctrl: keys[0],
        alt: keys[1],
    };
    w.paint_frame();
}

/// A left-drag through `cells`, let go on the last.
fn drag(w: &mut Workbench, cells: &[[i64; 2]], right: bool) {
    for (i, &at) in cells.iter().enumerate() {
        frame(w, at, true, right, i == 0, [false; 2]);
    }
    frame(w, *cells.last().unwrap(), false, right, false, [false; 2]);
}

fn row(w: &Workbench, y: u32, xs: std::ops::Range<u32>) -> String {
    let map = the_map(w);
    xs.map(|x| match map.code(0, x, y) {
        Some(0) | None => ' ',
        Some(c) => char::from(c),
    })
    .collect()
}

#[test]
fn a_brush_stroke_paints_every_cell_dragged_over_as_one_undo_step() {
    let mut w = painting(PaintTool::Brush);
    w.viewport.tile_overlay.paint.code = Some(b'~');
    frame(&mut w, [10, 1], true, false, true, [false; 2]);
    // Shown at once, saved only at the end.
    assert_eq!(row(&w, 1, 10..11), "~");
    assert!(saved_maps(&w.outbox).is_empty());
    // A fast drag: no gaps between.
    frame(&mut w, [16, 1], true, false, false, [false; 2]);
    frame(&mut w, [16, 1], false, false, false, [false; 2]);
    assert_eq!(row(&w, 1, 9..18), ".~~~~~~~.");
    assert_eq!(saved_maps(&w.outbox).len(), 1);
    w.map_undo(false, true);
    assert_eq!(row(&w, 1, 9..18), ".........");
    w.map_undo(true, false);
    assert_eq!(row(&w, 1, 9..18), ".~~~~~~~.");
}

#[test]
fn the_right_button_empties_and_a_stroke_changing_nothing_leaves_no_step() {
    let mut w = painting(PaintTool::Brush);
    drag(&mut w, &[[4, 4], [6, 4]], true);
    assert_eq!(row(&w, 4, 3..8), ".   t");
    w.outbox.clear();
    // Grass painted on grass: nothing to undo or save.
    drag(&mut w, &[[20, 20]], false);
    assert!(saved_maps(&w.outbox).is_empty());
}

#[test]
fn a_rectangle_and_a_line_show_first_and_paint_when_let_go() {
    let mut w = painting(PaintTool::Rectangle);
    w.viewport.tile_overlay.paint.code = Some(b'~');
    frame(&mut w, [-3, 30], true, false, true, [false; 2]);
    frame(&mut w, [1, 31], true, false, false, [false; 2]);
    assert_eq!(w.viewport.tile_overlay.preview, [[[-3, 30], [1, 31]]]);
    assert_eq!(row(&w, 30, 0..3), "...", "nothing painted yet");
    frame(&mut w, [1, 31], false, false, false, [false; 2]);
    // Clipped to the map.
    assert_eq!(row(&w, 30, 0..3), "~~.");
    assert_eq!(row(&w, 31, 0..3), "~~.");
    assert!(w.viewport.tile_overlay.preview.is_empty());
    w.viewport.tile_overlay.paint.tool = Some(PaintTool::Line);
    frame(&mut w, [20, 10], true, false, true, [false; 2]);
    frame(&mut w, [23, 13], true, false, false, [false; 2]);
    assert_eq!(w.viewport.tile_overlay.preview.len(), 4);
    frame(&mut w, [23, 13], false, false, false, [false; 2]);
    let map = the_map(&w);
    for i in 0..4 {
        assert_eq!(map.code(0, 20 + i, 10 + i), Some(b'~'));
    }
    assert_eq!(map.code(0, 21, 10), Some(b'.'));
    assert_eq!(saved_maps(&w.outbox).len(), 2, "one step each");
}

#[test]
fn a_fill_keeps_to_the_patch_clicked() {
    let mut w = painting(PaintTool::Fill);
    w.viewport.tile_overlay.paint.code = Some(b'~');
    // The tilled patch: 6 × 4 cells.
    frame(&mut w, [5, 5], true, false, true, [false; 2]);
    let map = the_map(&w);
    assert_eq!(map.counts(0)[usize::from(b'~')], 24);
    assert_eq!(map.code(0, 3, 5), Some(b'.'));
    assert_eq!(saved_maps(&w.outbox).len(), 1, "done at once");
    // Outside the map: nothing.
    frame(&mut w, [5, 5], false, false, false, [false; 2]);
    frame(&mut w, [-1, 5], true, false, true, [false; 2]);
    assert_eq!(saved_maps(&w.outbox).len(), 1);
}

#[test]
fn alt_picks_a_terrain_and_the_eyedropper_copies_cells_to_paint_with() {
    let mut w = painting(PaintTool::Brush);
    frame(&mut w, [5, 5], true, false, true, [false, true]);
    assert_eq!(w.viewport.tile_overlay.paint.code, Some(b't'));
    assert!(saved_maps(&w.outbox).is_empty(), "a pick paints nothing");
    frame(&mut w, [5, 5], false, false, false, [false; 2]);
    // Copy two cells: grass then tilled.
    w.viewport.tile_overlay.paint.tool = Some(PaintTool::Eyedropper);
    drag(&mut w, &[[3, 4], [4, 4]], false);
    let paint = &w.viewport.tile_overlay.paint;
    assert_eq!(
        paint.tool,
        Some(PaintTool::Brush),
        "the brush takes the copy"
    );
    assert_eq!(paint.stamp.as_ref().map(|s| s.size), Some([2, 1]));
    // Flipped (X), then painted: tilled then grass.
    w.viewport.tile_overlay.turn_asked = Some(Some(Turn::FlipX));
    drag(&mut w, &[[20, 20]], false);
    assert_eq!(row(&w, 20, 19..23), ".t..");
    // Esc puts the copy away, then the tool.
    assert!(w.paint_escape());
    assert!(w.viewport.tile_overlay.paint.stamp.is_none());
    assert!(w.paint_escape());
    assert_eq!(w.viewport.tile_overlay.paint.tool, None);
    assert!(!w.paint_escape());
}

#[test]
fn ctrl_or_exact_places_the_piece_chosen() {
    let mut w = painting(PaintTool::Brush);
    let mut set = (**w
        .viewport
        .sheets
        .tilesets
        .values()
        .next()
        .map(|s| match s {
            crate::sprite_sheets::TilesetState::Read(s, _) => s,
            _ => unreachable!(),
        })
        .unwrap())
    .clone();
    let water = set.by_name("water").unwrap().id;
    let piece = Id::random();
    set.pieces.push(tiles::Piece {
        id: piece,
        terrain: water,
        slot: 0,
        at: [0, 0],
        weight: 1.0,
        animation: tiles::Animation::None,
    });
    w.save_tileset("farm.soltileset", set);
    w.viewport.tile_overlay.paint.code = Some(b'~');
    frame(&mut w, [30, 30], true, false, true, [true, false]);
    frame(&mut w, [30, 30], false, false, false, [false; 2]);
    assert_eq!(the_map(&w).exact(0, 30, 30), Some(piece));
    // Painted over without Ctrl: autotiled again.
    drag(&mut w, &[[30, 30]], false);
    assert_eq!(the_map(&w).exact(0, 30, 30), None);
}

#[test]
fn esc_gives_a_stroke_up_and_playing_stops_painting() {
    let mut w = painting(PaintTool::Brush);
    w.viewport.tile_overlay.paint.code = Some(b'~');
    frame(&mut w, [10, 10], true, false, true, [false; 2]);
    assert_eq!(the_map(&w).code(0, 10, 10), Some(b'~'));
    assert!(w.paint_escape());
    assert_eq!(the_map(&w).code(0, 10, 10), Some(b'.'));
    frame(&mut w, [10, 10], false, false, false, [false; 2]);
    assert!(saved_maps(&w.outbox).is_empty());
    w.play();
    w.set_tile_overlay();
    assert!(!w.viewport.tile_overlay.paint.painting());
    frame(&mut w, [10, 10], true, false, true, [false; 2]);
    assert_eq!(the_map(&w).code(0, 10, 10), Some(b'.'));
}

#[test]
fn the_tool_bar_and_keys_choose_the_painting_tools_while_a_map_is_selected() {
    let (w, _, _) = with_map();
    let mut h = harness(w);
    h.run();
    assert!(h.query_by_label("Brush").is_none(), "no map selected");
    let (mut w, map, _) = with_map();
    w.place_tile_map(map, None).unwrap();
    let mut h = harness(w);
    h.run();
    h.run();
    for label in ["Brush", "Rectangle", "Line", "Fill", "Eyedropper", "Exact"] {
        assert!(h.query_by_label(label).is_some(), "{label}");
    }
    h.get_by_label("Fill").click();
    h.run();
    assert_eq!(
        h.state().0.viewport.tile_overlay.paint.tool,
        Some(PaintTool::Fill)
    );
    for (key, tool) in [
        (egui::Key::B, Some(PaintTool::Brush)),
        (egui::Key::U, Some(PaintTool::Rectangle)),
        (egui::Key::N, Some(PaintTool::Line)),
        (egui::Key::I, Some(PaintTool::Eyedropper)),
        (egui::Key::G, Some(PaintTool::Fill)),
        (egui::Key::Q, None),
    ] {
        h.key_press(key);
        h.run();
        assert_eq!(
            h.state().0.viewport.tile_overlay.paint.tool,
            tool,
            "{key:?}"
        );
    }
    h.key_press(egui::Key::Tab);
    h.run();
    assert!(h.state().0.viewport.tile_overlay.paint.meaning);
    // Clicking a swatch in the Map tab paints with it (the brush taken).
    h.get_by_label("Paint with water").click();
    h.run();
    let paint = &h.state().0.viewport.tile_overlay.paint;
    assert_eq!(paint.code, Some(b'~'));
    assert_eq!(paint.tool, Some(PaintTool::Brush));
}

#[test]
fn ctrl_z_while_painting_undoes_the_last_stroke() {
    let (mut w, map, _) = with_map();
    w.place_tile_map(map, None).unwrap();
    w.follow_map_selection();
    w.set_tile_overlay();
    w.viewport.tile_overlay.paint.tool = Some(PaintTool::Brush);
    w.viewport.tile_overlay.paint.code = Some(b'~');
    drag(&mut w, &[[12, 12]], false);
    assert_eq!(the_map(&w).code(0, 12, 12), Some(b'~'));
    let mut h = harness(w);
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    assert_eq!(the_map(&h.state().0).code(0, 12, 12), Some(b'.'));
}

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
fn scale_check_painting_a_1000_by_1000_map() {
    // The plan's budget: painting on a 1,000 × 1,000 map keeps the
    // editor's frame (16 ms): each frame of a brush stroke paints, shows
    // the map at once and draws again the chunks it touched, with art.
    const TARGET_MS: f64 = 16.0;
    // Letting go saves the whole map once: a stroke's end.
    const SAVE_MS: f64 = 250.0;
    let (mut w, map_id, set_id) = with_map();
    let png = reflect::AssetId::random();
    let mut set = crate::workbench_tiles::starter_tileset();
    set.terrains[1].kind = tiles::TileKind::Dual16;
    set.picture = Some(png);
    set.tile = [8, 8];
    let tilled = set.terrains[1].id;
    for slot in 0..16 {
        set.pieces.push(tiles::Piece {
            id: Id::random(),
            terrain: tilled,
            slot,
            at: [slot % 4, slot / 4],
            weight: 1.0,
            animation: tiles::Animation::None,
        });
    }
    let mut sheets = (*w.viewport.sheets).clone();
    sheets.tilesets.insert(
        "farm.soltileset".into(),
        crate::sprite_sheets::TilesetState::Read(Arc::new(set), Vec::new()),
    );
    sheets.tile_art.insert(
        "tiles.png".into(),
        crate::sprite_sheets::ArtState::Read(Arc::new(assets::tile_art::TileArt {
            size: [32, 32],
            frames: vec![vec![200; 32 * 32 * 4]],
            durations: vec![100],
        })),
    );
    sheets.paths.insert(png, "tiles.png".into());
    let mut big = tiles::TileMap::new([1000, 1000], Some(set_id), Some(b'.'));
    for i in 0..50_000u32 {
        big.set_code(0, (i * 37) % 1000, (i * 91) % 1000, b't')
            .unwrap();
    }
    sheets.tile_maps.insert(
        "farm.soltilemap".into(),
        crate::sprite_sheets::MapFileState::Read(Arc::new(big), Vec::new()),
    );
    w.set_sprite_sheets(sheets);
    w.place_tile_map(map_id, Some((glam::Vec3::ZERO, None)))
        .unwrap();
    w.follow_map_selection();
    let [tx, ty] = w.pixels.tile_size();
    w.viewport.camera = crate::camera::Camera::default().as_kind(crate::camera::ViewKind::TwoD);
    w.viewport.camera.position = glam::Vec3::new(500.0 * tx, -500.0 * ty, 10.0);
    w.set_tile_overlay();
    w.viewport.tile_overlay.paint.tool = Some(PaintTool::Brush);
    w.viewport.tile_overlay.paint.code = Some(b't');
    w.viewport.area = crate::area_view::AreaView::of(w.document.area(), &w.models);
    // Warm: the chunks in view drawn once, tile by tile from the art.
    let view = w.viewport.view([1280, 720], 1.0);
    let atlas = view
        .tile_pictures
        .iter()
        .find(|p| p.size[0] == 32)
        .expect("art");
    let tiles: usize = view
        .sprites
        .iter()
        .filter(|b| b.sheet == atlas.id)
        .map(|b| b.sprites.len())
        .sum();
    assert!(tiles > 1000, "{tiles} tiles");
    let frames = 120;
    let start = std::time::Instant::now();
    for f in 0..frames {
        let at = [480 + i64::from(f % 40), 490 + i64::from(f / 10)];
        frame(&mut w, at, true, false, f == 0, [false; 2]);
        let view = w.viewport.view([1280, 720], 1.0);
        assert!(!view.tile_pictures.is_empty());
    }
    let ms = start.elapsed().as_secs_f64() * 1000.0 / f64::from(frames);
    let start = std::time::Instant::now();
    frame(&mut w, [520, 502], false, false, false, [false; 2]);
    let save_ms = start.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(saved_maps(&w.outbox).len(), 1);
    let verdict = |ms: f64, target: f64| if ms <= target { "ok" } else { "SLOW" };
    println!(
        "Scale check: a frame of brushing on a 1,000 × 1,000 tile map with art {ms:.2} ms (target {TARGET_MS} ms) {}",
        verdict(ms, TARGET_MS)
    );
    println!(
        "Scale check: letting go of a stroke on a 1,000 × 1,000 tile map (one undo step, saved) {save_ms:.0} ms (target {SAVE_MS} ms) {}",
        verdict(save_ms, SAVE_MS)
    );
    assert!(ms <= TARGET_MS, "{ms:.2} ms a frame");
    assert!(save_ms <= SAVE_MS, "{save_ms:.0} ms to let go");
}
