//! Autotiling: every kind's slots from neighbours, variations the same
//! every time, missing pieces noticed, animation frames, templates, and the
//! picture's grid and kind guessed.

use super::*;
use crate::tileset::tests::farm_art;
use crate::tileset::{Piece, Tileset};
use reflect::Id;

fn id(n: u64) -> Id {
    Id::from_raw(n).unwrap()
}

/// A map from rows of codes (space for empty), one layer.
fn map_of(rows: &[&str]) -> TileMap {
    let (w, h) = (rows[0].len() as u32, rows.len() as u32);
    let mut map = TileMap::new([w, h], None, None);
    for (y, row) in rows.iter().enumerate() {
        for (x, c) in row.bytes().enumerate() {
            if c != b' ' {
                map.set_code(0, x as u32, y as u32, c).unwrap();
            }
        }
    }
    map
}

fn at(tiles: &[Placed], x: i32, y: i32) -> Vec<Art> {
    tiles
        .iter()
        .filter(|p| p.x == x && p.y == y)
        .map(|p| p.art)
        .collect()
}

fn piece_slot(set: &Tileset, art: Art) -> Option<(Id, u32)> {
    match art {
        Art::Piece(i) => Some((set.pieces[i].terrain, set.pieces[i].slot)),
        _ => None,
    }
}

#[test]
fn there_are_47_blob_slots_and_every_mask_has_one() {
    assert_eq!(BLOB_MASKS.len(), 47);
    assert_eq!(BLOB_MASKS[0], 0);
    assert_eq!(BLOB_MASKS[46], 255);
    for m in 0..=255u8 {
        let slot = blob_slot(m);
        assert!(slot < 47);
        assert_eq!(BLOB_MASKS[slot as usize], blob_reduce(m), "{m:08b}");
    }
    // A corner alone counts for nothing.
    assert_eq!(blob_slot(NE), blob_slot(0));
    assert_ne!(blob_slot(N | E | NE), blob_slot(N | E));
}

#[test]
fn plain_cells_show_their_piece_and_variations_stay_put() {
    let set = farm_art();
    let map = map_of(&["....", "...."]);
    let tiles = chunk_tiles(&map, 0, &set, [0, 0]);
    assert_eq!(tiles.len(), 8);
    assert!(
        tiles
            .iter()
            .all(|p| p.pass == 0 && p.x % 2 == 0 && p.y % 2 == 0)
    );
    // The same every time, and both variations used across a field (the
    // heavier one more often).
    let again = chunk_tiles(&map, 0, &set, [0, 0]);
    assert_eq!(tiles, again);
    let big = TileMap::new([32, 32], None, Some(b'.'));
    let tiles = chunk_tiles(&big, 0, &set, [0, 0]);
    let first = tiles.iter().filter(|p| p.art == Art::Piece(0)).count();
    let second = tiles.iter().filter(|p| p.art == Art::Piece(1)).count();
    assert_eq!(first + second, 1024);
    assert!(first > second * 2 && second > 100, "{first} and {second}");
}

#[test]
fn a_16_tile_terrain_sits_where_four_cells_meet() {
    let set = farm_art();
    let tilled = id(2);
    // One tilled cell in grass: four pieces round it, each with one corner.
    let map = map_of(&["...", ".t.", "..."]);
    let tiles = chunk_tiles(&map, 0, &set, [0, 0]);
    let duals: Vec<&Placed> = tiles.iter().filter(|p| p.pass == 2).collect();
    assert_eq!(duals.len(), 4);
    // Vertex (1, 1) is the cell's top left: the cell is its bottom right.
    let piece = at(&tiles, 1, 1)
        .into_iter()
        .find(|a| matches!(a, Art::Piece(_)))
        .unwrap();
    let _ = piece;
    let slots: Vec<u32> = [(1, 1), (3, 1), (1, 3), (3, 3)]
        .iter()
        .map(|&(x, y)| {
            tiles
                .iter()
                .find(|p| p.pass == 2 && p.x == x && p.y == y)
                .and_then(|p| piece_slot(&set, p.art))
                .map(|(t, s)| {
                    assert_eq!(t, tilled);
                    s
                })
                .unwrap()
        })
        .collect();
    assert_eq!(slots, [1, 2, 4, 8]);
    // Grass is still drawn under it, in the first pass.
    assert_eq!(tiles.iter().filter(|p| p.pass == 0).count(), 8);
    // A checkerboard needs slots 6 and 9, which have no piece: missing.
    let map = map_of(&["t.", ".t"]);
    let tiles = chunk_tiles(&map, 0, &set, [0, 0]);
    let middle = tiles
        .iter()
        .find(|p| p.pass == 2 && p.x == 1 && p.y == 1)
        .unwrap();
    assert_eq!(middle.art, Art::Missing);
}

#[test]
fn an_exact_tile_shows_its_piece_over_the_rest_of_the_layer() {
    let set = farm_art();
    let mut map = map_of(&["...", ".t.", "..."]);
    let fence = set.pieces[set.pieces.len() - 1].id;
    map.set_exact(0, 0, 0, Some(fence)).unwrap();
    map.set_exact(0, 2, 2, Some(id(999))).unwrap();
    let tiles = chunk_tiles(&map, 0, &set, [0, 0]);
    let top = 1 + set.terrains.len() as u16;
    let exact: Vec<&Placed> = tiles.iter().filter(|p| p.pass == top).collect();
    assert_eq!(exact.len(), 2);
    assert_eq!(at(&tiles, 0, 0), [Art::Piece(set.pieces.len() - 1)]);
    // A piece the tileset hasn't any more is missing; the cell's own art
    // isn't drawn under either.
    assert_eq!(at(&tiles, 4, 4), [Art::Missing]);
    assert_eq!(tiles.iter().filter(|p| p.pass == 0).count(), 6);
    // The tilled cell's neighbours still count it: four corners round it.
    assert_eq!(tiles.iter().filter(|p| p.pass == 2).count(), 4);
    // Another chunk's exact tiles stay out of this one.
    let mut big = TileMap::new([64, 32], None, Some(b'.'));
    big.set_exact(0, 40, 3, Some(fence)).unwrap();
    assert!(
        !chunk_tiles(&big, 0, &set, [0, 0])
            .iter()
            .any(|p| p.pass == top)
    );
    assert_eq!(
        chunk_tiles(&big, 0, &set, [1, 0])
            .iter()
            .filter(|p| p.pass == top)
            .count(),
        1
    );
}

#[test]
fn a_16_tile_terrain_s_corners_belong_to_one_chunk_each() {
    let set = farm_art();
    let map = TileMap::new([40, 40], None, Some(b't'));
    let mut all = Vec::new();
    for cy in 0..2 {
        for cx in 0..2 {
            all.extend(chunk_tiles(&map, 0, &set, [cx, cy]));
        }
    }
    // 41 × 41 corners for 40 × 40 cells, each once.
    let duals: std::collections::HashSet<(i32, i32)> = all
        .iter()
        .filter(|p| p.pass == 2)
        .map(|p| (p.x, p.y))
        .collect();
    assert_eq!(duals.len(), 41 * 41);
    assert_eq!(all.iter().filter(|p| p.pass == 2).count(), 41 * 41);
    // The map's own edge is drawn too: no half-tile gap.
    assert!(duals.contains(&(-1, -1)) && duals.contains(&(79, 79)));
}

#[test]
fn a_47_tile_terrain_reads_its_eight_neighbours() {
    let set = farm_art();
    let map = map_of(&["~~~", "~~~", "~~~"]);
    let tiles = chunk_tiles(&map, 0, &set, [0, 0]);
    // The middle is surrounded: slot 46, which has a piece; past the
    // map's edge counts as water, so every cell is.
    assert!(
        tiles
            .iter()
            .all(|p| piece_slot(&set, p.art).is_some_and(|(_, s)| s == 46))
    );
    let map = map_of(&["...", ".~.", "..."]);
    let tiles = chunk_tiles(&map, 0, &set, [0, 0]);
    assert_eq!(at(&tiles, 2, 2), [Art::Missing], "slot 0 has no piece");
}

#[test]
fn edges_join_their_sides() {
    let set = farm_art();
    let fence = id(4);
    let map = map_of(&["     ", " ### ", "  #  ", "     "]);
    let tiles = chunk_tiles(&map, 0, &set, [0, 0]);
    let slot = |x: i32, y: i32| piece_slot(&set, at(&tiles, x, y)[0]).unwrap();
    assert_eq!(slot(2, 2), (fence, 2), "east only");
    assert_eq!(slot(4, 2), (fence, 2 | 4 | 8), "east, south, west");
    assert_eq!(slot(6, 2), (fence, 8), "west only");
    assert_eq!(slot(4, 4), (fence, 1), "north only");
}

#[test]
fn rules_are_tried_in_order() {
    let set = farm_art();
    let rocks = id(5);
    // The first rule: grass above, rocks either side.
    let map = map_of(&[" . ", "rrr", "   "]);
    let tiles = chunk_tiles(&map, 0, &set, [0, 0]);
    assert_eq!(piece_slot(&set, at(&tiles, 2, 2)[0]), Some((rocks, 1)));
    // The ends don't match it: the second rule (any rock) does.
    assert_eq!(piece_slot(&set, at(&tiles, 0, 2)[0]), Some((rocks, 0)));
    // No rule at all: missing, never another piece.
    let mut none = set.clone();
    none.rules.clear();
    let tiles = chunk_tiles(&map, 0, &none, [0, 0]);
    assert_eq!(at(&tiles, 2, 2), [Art::Missing]);
}

#[test]
fn terrains_without_art_or_unknown_codes_are_coloured_or_missing() {
    let mut set = farm_art();
    let map = map_of(&[".?"]);
    let tiles = chunk_tiles(&map, 0, &set, [0, 0]);
    assert_eq!(at(&tiles, 2, 0), [Art::Missing], "not in the tileset");
    set.pieces.retain(|p| p.terrain != id(1));
    let tiles = chunk_tiles(&map, 0, &set, [0, 0]);
    assert_eq!(at(&tiles, 0, 0), [Art::Colour(0)], "grass has no art now");
    set.picture = None;
    let map = map_of(&["t~"]);
    let tiles = chunk_tiles(&map, 0, &set, [0, 0]);
    assert_eq!(tiles.len(), 2, "no picture: colours, no corners");
    assert!(tiles.iter().all(|p| matches!(p.art, Art::Colour(_))));
}

#[test]
fn missing_slots_are_what_needs_drawing() {
    let set = farm_art();
    let t = |n| set.terrain(id(n)).unwrap();
    assert_eq!(missing_slots(&set, t(1)), Vec::<u32>::new());
    assert_eq!(missing_slots(&set, t(2)), [6, 9]);
    assert_eq!(missing_slots(&set, t(3)).len(), 46);
    assert!(missing_slots(&set, t(4)).is_empty());
    assert!(missing_slots(&set, t(5)).is_empty());
    assert_eq!(needed_slots(&set, t(5)), [0, 1]);
}

#[test]
fn every_weight_zero_picks_evenly_and_one_piece_is_always_picked() {
    let mut set = farm_art();
    for p in &mut set.pieces {
        p.weight = 0.0;
    }
    let grass = set.terrain(id(1)).unwrap().clone();
    let picks: std::collections::HashSet<Option<usize>> =
        (0..50).map(|x| pick(&set, &grass, 0, x, 0)).collect();
    assert_eq!(picks.len(), 2);
    let tilled = set.terrain(id(2)).unwrap().clone();
    assert!(pick(&set, &tilled, 6, 0, 0).is_none());
    assert!(pick(&set, &tilled, 5, -7, 1_000_000).is_some());
}

#[test]
fn animation_frames_follow_the_time() {
    let cols = Animation::Columns { count: 4, ms: 150 };
    assert_eq!(frame_at(cols, 0, &[]), 0);
    assert_eq!(frame_at(cols, 149, &[]), 0);
    assert_eq!(frame_at(cols, 150, &[]), 1);
    assert_eq!(frame_at(cols, 600, &[]), 0, "round again");
    assert_eq!(frame_at(Animation::Frames, 250, &[100, 200]), 1);
    assert_eq!(frame_at(Animation::Frames, 300, &[100, 200]), 0);
    assert_eq!(frame_at(Animation::Frames, 99, &[100]), 0, "one frame");
    assert_eq!(frame_at(Animation::None, 999, &[1, 1]), 0);
}

#[test]
fn templates_place_every_slot_and_show_its_neighbours() {
    for kind in TileKind::ALL {
        let [cols, rows] = template_size(kind);
        let mut seen = std::collections::HashSet::new();
        for slot in 0..slot_count(kind) {
            let [x, y] = template_place(kind, slot);
            assert!(x < cols && y < rows, "{kind:?} {slot}");
            assert!(seen.insert((x, y)));
        }
        let ([w, h], rgba) = template_rgba(kind, [8, 8], [1, 2, 3], [4, 5, 6]);
        assert_eq!((w, h), (cols * 8, rows * 8));
        assert_eq!(rgba.len(), (w * h * 4) as usize);
    }
    // 16-tile slot 1 is the bottom right corner only.
    let p = slot_picture(TileKind::Dual16, 1);
    assert!(p[8] && !p[0] && !p[2] && !p[6]);
    // Edges' slot 5 is north and south.
    let p = slot_picture(TileKind::Edges, 5);
    assert!(p[1] && p[7] && !p[3] && !p[5]);
    let ([w, _], rgba) = template_rgba(TileKind::Edges, [9, 9], [200, 0, 0], [0, 200, 0]);
    // Slot 0 (alone): its middle on, its top middle off.
    let px = |x: u32, y: u32| &rgba[((y * w + x) * 4) as usize..((y * w + x) * 4 + 3) as usize];
    assert_eq!(px(4, 4), [200, 0, 0]);
    assert_eq!(px(4, 1), [0, 200, 0]);
}

/// A picture of `cols` × `rows` tiles of `t` pixels, each tile a flat
/// colour of its own, `blank` tiles see-through.
fn picture(cols: u32, rows: u32, t: u32, blank: &[[u32; 2]]) -> ([u32; 2], Vec<u8>) {
    let (w, h) = (cols * t, rows * t);
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let (cx, cy) = (x / t, y / t);
            if blank.contains(&[cx, cy]) {
                continue;
            }
            let i = ((y * w + x) * 4) as usize;
            rgba[i..i + 4].copy_from_slice(&[
                (cx * 37 % 255) as u8,
                (cy * 71 % 255) as u8,
                ((cx + cy) * 13 % 255) as u8,
                255,
            ]);
        }
    }
    ([w, h], rgba)
}

#[test]
fn the_grid_is_found_from_where_the_art_changes() {
    let (size, rgba) = picture(8, 4, 16, &[]);
    assert_eq!(detect_tile(size, &rgba, [16, 16]), [16, 16]);
    assert_eq!(
        detect_tile(size, &rgba, [8, 8]),
        [16, 16],
        "the art says 16"
    );
    let (size, rgba) = picture(4, 4, 32, &[]);
    assert_eq!(detect_tile(size, &rgba, [16, 16]), [32, 32]);
    // A flat picture says nothing: the project's size.
    let flat = vec![255u8; 64 * 64 * 4];
    assert_eq!(detect_tile([64, 64], &flat, [16, 16]), [16, 16]);
    assert_eq!(detect_tile([0, 0], &[], [16, 16]), [16, 16]);
    assert_eq!(
        detect_tile([10, 10], &[0; 4], [16, 16]),
        [16, 16],
        "too short"
    );
}

#[test]
fn the_kind_is_guessed_and_a_template_drawn_picture_is_marked_up() {
    let (size, rgba) = picture(4, 4, 16, &[[0, 0]]);
    let fill = |at: [u32; 2]| tile_fill(size, &rgba, [16, 16], at);
    assert_eq!(
        guess_kind(fill, [0, 0], [4, 4]),
        TileKind::Dual16,
        "15 solid tiles"
    );
    assert_eq!(
        guess_kind(|_| (true, false), [0, 0], [4, 4]),
        TileKind::Edges
    );
    assert_eq!(
        guess_kind(|_| (true, true), [0, 0], [8, 6]),
        TileKind::Blob47
    );
    assert_eq!(
        guess_kind(|_| (true, true), [0, 0], [1, 1]),
        TileKind::Plain
    );
    assert_eq!(
        guess_kind(|_| (true, true), [3, 3], [4, 4]),
        TileKind::Plain,
        "past the edge"
    );
    let shown = |at: [u32; 2]| tile_fill(size, &rgba, [16, 16], at).0;
    let marks = mark_up(TileKind::Dual16, [0, 0], shown);
    assert_eq!(marks.len(), 15, "slot 0 left out: its tile is blank");
    assert_eq!(marks[0], (1, [1, 0]));
    assert!(mark_up(TileKind::Rules, [0, 0], |_| true).is_empty());
    // Past the picture's edge: not filled.
    assert_eq!(tile_fill(size, &rgba, [16, 16], [9, 9]), (false, false));
    let _ = Piece::clone;
}

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
fn scale_check_autotiling_a_screen_of_chunks() {
    // A full 2D screen's worth: 8 × 8 chunks (65,536 cells) of a
    // 1,000 × 1,000 farm, tilled patches (16-tile) among grass, worked out
    // from scratch, as when a map first comes into view.
    const TARGET_MS: f64 = 16.0;
    let set = farm_art();
    let mut map = TileMap::new([1000, 1000], None, Some(b'.'));
    for y in 0..256 {
        for x in 0..256 {
            if (x / 5 + y / 7) % 3 == 0 {
                map.set_code(0, x, y, b't').unwrap();
            }
        }
    }
    let start = std::time::Instant::now();
    let mut tiles = 0;
    for cy in 0..8 {
        for cx in 0..8 {
            tiles += chunk_tiles(&map, 0, &set, [cx, cy]).len();
        }
    }
    let ms = start.elapsed().as_secs_f64() * 1000.0;
    assert!(tiles > 65_536);
    let verdict = if ms <= TARGET_MS { "ok" } else { "SLOW" };
    println!(
        "Scale check: autotiling 8 × 8 chunks of tiles {ms:.2} ms (target {TARGET_MS} ms) {verdict}"
    );
    assert!(ms <= TARGET_MS, "{ms:.2} ms");
}
