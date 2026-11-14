//! Marching cubes' table: for each of the 256 ways a cube's eight corners
//! can be in the ground or out of it, which triangles (as the cube edges
//! their points lie on) cut the inside from the outside.
//!
//! The table is worked out here rather than typed in. On each of the
//! cube's six faces, the edges the surface crosses are joined in pairs,
//! always keeping corners in the ground apart when only their diagonal
//! touches; joined up face to face, the pairs make closed loops round
//! the cube, and each loop is cut into a fan of triangles. A face's
//! pairing depends on that face alone, so the cube beside it joins the
//! same points the same way and no crack can open between them.

use std::sync::OnceLock;

/// A corner's place in the cube: x is its first bit, y its second, z its
/// third.
pub(crate) type Corner = u8;

/// The cube's twelve edges, each by its two corners (the lower first).
pub(crate) const EDGES: [(Corner, Corner); 12] = [
    (0, 1),
    (2, 3),
    (4, 5),
    (6, 7),
    (0, 2),
    (1, 3),
    (4, 6),
    (5, 7),
    (0, 4),
    (1, 5),
    (2, 6),
    (3, 7),
];

/// Each face's corners, counter-clockwise seen from outside the cube.
const FACES: [[Corner; 4]; 6] = [
    [0, 4, 6, 2],
    [1, 3, 7, 5],
    [0, 1, 5, 4],
    [2, 6, 7, 3],
    [0, 2, 3, 1],
    [4, 5, 7, 6],
];

fn edge_between(a: Corner, b: Corner) -> usize {
    let (lo, hi) = (a.min(b), a.max(b));
    EDGES
        .iter()
        .position(|e| *e == (lo, hi))
        .unwrap_or_default()
}

/// The table: for each corner mask (bit `i` set when corner `i` is in
/// the ground), its triangles as three edges each.
pub(crate) fn table() -> &'static [Vec<[u8; 3]>] {
    static TABLE: OnceLock<Vec<Vec<[u8; 3]>>> = OnceLock::new();
    TABLE.get_or_init(|| (0..256).map(|mask| triangles(mask as u8)).collect())
}

fn triangles(mask: u8) -> Vec<[u8; 3]> {
    let solid = |c: Corner| mask & (1 << c) != 0;
    // Where the loop goes from each crossed edge.
    let mut next = [None::<usize>; 12];
    for face in FACES {
        let crossing = |k: usize| {
            let (a, b) = (face[k], face[(k + 1) % 4]);
            (solid(a) != solid(b)).then_some((edge_between(a, b), solid(b)))
        };
        for start in 0..4 {
            // A crossing into the ground joins the next one out of it.
            let Some((entering, true)) = crossing(start) else {
                continue;
            };
            for step in 1..4 {
                if let Some((leaving, false)) = crossing((start + step) % 4) {
                    next[entering] = Some(leaving);
                    break;
                }
            }
        }
    }
    let mut seen = [false; 12];
    let mut out = Vec::new();
    for first in 0..12 {
        if seen[first] || next[first].is_none() {
            continue;
        }
        let mut ring = Vec::new();
        let mut at = first;
        while !seen[at] {
            seen[at] = true;
            ring.push(at as u8);
            match next[at] {
                Some(n) => at = n,
                None => break,
            }
        }
        for k in 1..ring.len().saturating_sub(1) {
            out.push([ring[0], ring[k], ring[k + 1]]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_and_full_cubes_have_no_triangles() {
        assert!(table()[0].is_empty());
        assert!(table()[255].is_empty());
    }

    #[test]
    fn one_corner_in_is_one_triangle() {
        for corner in 0..8 {
            assert_eq!(table()[1 << corner].len(), 1);
        }
    }

    #[test]
    fn every_crossed_edge_is_used_and_no_other() {
        for mask in 0..=255u8 {
            let solid = |c: Corner| mask & (1 << c) != 0;
            let mut used = [false; 12];
            for t in &table()[usize::from(mask)] {
                for e in t {
                    used[usize::from(*e)] = true;
                }
            }
            for (e, (a, b)) in EDGES.iter().enumerate() {
                assert_eq!(used[e], solid(*a) != solid(*b), "mask {mask} edge {e}");
            }
        }
    }

    #[test]
    fn a_cube_and_its_opposite_cut_the_same_edges() {
        for mask in 0..=255u8 {
            let count = |m: u8| table()[usize::from(m)].len();
            assert_eq!(count(mask) == 0, count(!mask) == 0);
        }
    }
}
