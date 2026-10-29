//! The painter's shapes: lines without gaps, rectangles clipped to the
//! map, fills that keep to one field, and stamps flipped, turned and put
//! down.

use super::*;
use proptest::prelude::*;

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

fn rows_of(s: &Stamp) -> Vec<String> {
    s.cells
        .chunks(s.size[0] as usize)
        .map(|r| {
            r.iter()
                .map(|c| if c.code == 0 { ' ' } else { char::from(c.code) })
                .collect()
        })
        .collect()
}

#[test]
fn a_line_has_both_ends_and_no_gaps() {
    assert_eq!(line_cells([2, 3], [2, 3]), [[2, 3]]);
    assert_eq!(line_cells([0, 0], [3, 0]), [[0, 0], [1, 0], [2, 0], [3, 0]]);
    assert_eq!(line_cells([0, 0], [-2, -2]), [[0, 0], [-1, -1], [-2, -2]]);
    let shallow = line_cells([0, 0], [6, 2]);
    assert_eq!(shallow.len(), 7, "one cell per column of a shallow line");
}

#[test]
fn a_rectangle_is_clipped_to_the_map() {
    assert_eq!(
        rect_within([5, 1], [2, 4], [10, 10]),
        Some([[2, 1], [5, 4]])
    );
    assert_eq!(
        rect_within([-3, -3], [20, 2], [10, 10]),
        Some([[0, 0], [9, 2]])
    );
    assert_eq!(rect_within([-3, -3], [-1, 5], [10, 10]), None);
    assert_eq!(rect_within([10, 0], [12, 0], [10, 10]), None);
}

#[test]
fn a_fill_keeps_to_one_field() {
    let map = map_of(&["..tt.", "..t..", "ttt..", "....."]);
    let mut field = flood_fill(&map, 0, [2, 0]);
    field.sort_unstable();
    // The tilled cells joined by their sides, not the grass round them.
    assert_eq!(field, [2, 3, 7, 10, 11, 12]);
    // The grass on the left is cut off from the right by the tilled path.
    assert_eq!(flood_fill(&map, 0, [0, 0]).len(), 4);
    assert_eq!(flood_fill(&map, 0, [4, 0]).len(), 10);
    // Empty cells fill like a terrain; outside or no such layer: nothing.
    let empty = TileMap::new([4, 4], None, None);
    assert_eq!(flood_fill(&empty, 0, [1, 1]).len(), 16);
    assert!(flood_fill(&map, 0, [5, 0]).is_empty());
    assert!(flood_fill(&map, 3, [0, 0]).is_empty());
}

#[test]
fn a_stamp_is_copied_flipped_and_turned() {
    let mut map = map_of(&["ab.", "cd.", "..."]);
    map.set_exact(0, 1, 0, Some(id(5))).unwrap();
    let mut s = Stamp::copy(&map, 0, [1, 1], [0, 0]).unwrap();
    assert_eq!(rows_of(&s), ["ab", "cd"]);
    assert_eq!(s.cells[1].exact, Some(id(5)));
    s.flip_x();
    assert_eq!(rows_of(&s), ["ba", "dc"]);
    assert_eq!(
        s.cells[0].exact,
        Some(id(5)),
        "exact tiles move with their cell"
    );
    s.flip_y();
    assert_eq!(rows_of(&s), ["dc", "ba"]);
    let mut wide = Stamp::copy(&map, 0, [0, 0], [2, 1]).unwrap();
    assert_eq!(rows_of(&wide), ["ab.", "cd."]);
    wide.rotate();
    assert_eq!(wide.size, [2, 3]);
    assert_eq!(rows_of(&wide), ["ca", "db", ".."]);
    // An area of empty cells, or outside the map, copies nothing.
    let empty = TileMap::new([4, 4], None, None);
    assert!(Stamp::copy(&empty, 0, [0, 0], [3, 3]).is_none());
    assert!(Stamp::copy(&map, 0, [5, 5], [9, 9]).is_none());
}

#[test]
fn a_stamp_sits_round_the_pointer_and_skips_empty_cells() {
    let s = Stamp {
        size: [3, 2],
        cells: [b'a', 0, b'c', b'd', b'e', b'f']
            .map(|code| StampCell { code, exact: None })
            .to_vec(),
    };
    assert_eq!(s.handle(), [1, 0]);
    let placed: Vec<[u32; 2]> = s.placed([1, 0], [10, 10]).map(|(c, _)| c).collect();
    assert_eq!(placed, [[0, 0], [2, 0], [0, 1], [1, 1], [2, 1]]);
    // At the map's corner, what's outside is left off.
    let placed: Vec<[u32; 2]> = s.placed([0, 9], [10, 10]).map(|(c, _)| c).collect();
    assert_eq!(placed, [[1, 9]]);
    assert_eq!(Stamp::one(b't', None).placed([3, 3], [4, 4]).count(), 1);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Every step of a line is to a neighbouring cell, and it ends where
    /// it was asked to.
    #[test]
    fn lines_have_no_gaps(ax in -50i64..50, ay in -50i64..50, bx in -50i64..50, by in -50i64..50) {
        let line = line_cells([ax, ay], [bx, by]);
        prop_assert_eq!(line[0], [ax, ay]);
        prop_assert_eq!(*line.last().unwrap(), [bx, by]);
        prop_assert_eq!(line.len() as i64, (bx - ax).abs().max((by - ay).abs()) + 1);
        for pair in line.windows(2) {
            prop_assert!((pair[1][0] - pair[0][0]).abs() <= 1);
            prop_assert!((pair[1][1] - pair[0][1]).abs() <= 1);
        }
    }

    /// Four turns, or two flips each way, give the stamp back.
    #[test]
    fn turns_and_flips_come_back(w in 1u32..6, h in 1u32..6, seed in 0u64..1000) {
        let cells = (0..w * h)
            .map(|i| StampCell { code: 33 + ((seed + u64::from(i) * 7) % 90) as u8, exact: None })
            .collect();
        let s = Stamp { size: [w, h], cells };
        let mut t = s.clone();
        for _ in 0..4 {
            t.rotate();
        }
        prop_assert_eq!(&t, &s);
        t.flip_x();
        t.flip_x();
        t.flip_y();
        t.flip_y();
        prop_assert_eq!(&t, &s);
    }
}
