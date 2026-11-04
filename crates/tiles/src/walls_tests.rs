//! Walls from solid cells: merged, kept inside their chunk, every solid
//! cell covered exactly once, any layer counting, and the same every time.

use super::*;
use crate::{Terrain, TileKind};
use proptest::prelude::*;

fn solid(codes: &[u8]) -> Solid {
    let mut s = [false; 256];
    for c in codes {
        s[*c as usize] = true;
    }
    s
}

/// How many rectangles cover each cell (0 for none).
fn coverage(map: &TileMap, rects: &[Rect]) -> Vec<u32> {
    let [w, h] = map.size();
    let mut seen = vec![0; (w * h) as usize];
    for r in rects {
        for y in r.y..r.y + r.h {
            for x in r.x..r.x + r.w {
                seen[(y * w + x) as usize] += 1;
            }
        }
    }
    seen
}

fn all_walls(map: &TileMap, s: &Solid) -> Vec<Rect> {
    let [cx, cy] = map.chunks();
    let mut out = Vec::new();
    for y in 0..cy {
        for x in 0..cx {
            out.extend(chunk_walls(map, s, [x, y]));
        }
    }
    out
}

#[test]
fn a_fence_and_a_pond_become_three_rectangles_not_seventy_cells() {
    let mut map = TileMap::new([30, 20], None, Some(b'.'));
    // A fence along the top and down the left, on the decoration layer.
    for x in 0..30 {
        map.set_code(1, x, 0, b'#').unwrap();
    }
    for y in 1..20 {
        map.set_code(1, 0, y, b'#').unwrap();
    }
    // A pond 6 × 4 on the ground.
    for y in 10..14 {
        for x in 12..18 {
            map.set_code(0, x, y, b'~').unwrap();
        }
    }
    let walls = chunk_walls(&map, &solid(b"#~"), [0, 0]);
    assert_eq!(
        walls,
        [
            Rect { x: 0, y: 0, w: 30, h: 1 },
            Rect { x: 0, y: 1, w: 1, h: 19 },
            Rect { x: 12, y: 10, w: 6, h: 4 },
        ]
    );
    // Nothing solid, nothing made.
    assert!(chunk_walls(&map, &solid(b""), [0, 0]).is_empty());
    assert!(chunk_walls(&map, &solid(b"x"), [0, 0]).is_empty());
}

#[test]
fn walls_stay_inside_their_chunk() {
    let mut map = TileMap::new([70, 3], None, Some(b'#'));
    let _ = map.take_dirty();
    let walls = all_walls(&map, &solid(b"#"));
    assert_eq!(
        walls,
        [
            Rect { x: 0, y: 0, w: 32, h: 3 },
            Rect { x: 32, y: 0, w: 32, h: 3 },
            Rect { x: 64, y: 0, w: 6, h: 3 },
        ]
    );
    assert!(chunk_walls(&map, &solid(b"#"), [3, 0]).is_empty(), "off the map");
    assert!(chunk_walls(&map, &solid(b"#"), [u32::MAX, u32::MAX]).is_empty());
}

#[test]
fn a_tileset_says_which_codes_are_solid() {
    let mut set = Tileset::default();
    for (name, code, solid) in [("grass", '.', false), ("water", '~', true), ("fence", '#', true)] {
        set.terrains.push(Terrain {
            id: reflect::Id::random(),
            name: name.into(),
            code,
            colour: [0.5; 3],
            kind: TileKind::Plain,
            solid,
        });
    }
    let s = solid_codes(&set);
    assert!(s[b'~' as usize] && s[b'#' as usize]);
    assert!(!s[b'.' as usize] && !s[0], "grass and empty cells aren't");
}

proptest! {
    /// Any map: every solid cell is in exactly one rectangle, nothing
    /// else is, no rectangle crosses a chunk's edge, and the same map
    /// gives the same walls.
    #[test]
    fn every_solid_cell_is_covered_once(
        w in 1u32..80, h in 1u32..80, seed in any::<u64>(), density in 0u32..100
    ) {
        let mut map = TileMap::new([w, h], None, Some(b'.'));
        let mut r = seed | 1;
        for y in 0..h {
            for x in 0..w {
                r ^= r << 13; r ^= r >> 7; r ^= r << 17;
                if (r % 100) < u64::from(density) {
                    map.set_code((r >> 40) as usize % 2, x, y, b'#').unwrap();
                }
            }
        }
        let s = solid(b"#");
        let walls = all_walls(&map, &s);
        let seen = coverage(&map, &walls);
        for y in 0..h {
            for x in 0..w {
                let want = u32::from(solid_at(&map, &s, x, y));
                prop_assert_eq!(seen[(y * w + x) as usize], want);
            }
        }
        for r in &walls {
            prop_assert_eq!(r.x / CHUNK, (r.x + r.w - 1) / CHUNK);
            prop_assert_eq!(r.y / CHUNK, (r.y + r.h - 1) / CHUNK);
        }
        prop_assert_eq!(walls, all_walls(&map, &s));
    }
}
