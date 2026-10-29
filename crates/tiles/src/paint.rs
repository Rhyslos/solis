//! What the tile painter's tools touch (Phase 9, D6), worked out apart
//! from the editor so it's tested on its own: the cells of a line and a
//! rectangle, the cells a fill reaches, and a stamp (an area copied with
//! the eyedropper) flipped, turned and put down.
//!
//! The editor makes the changes through [`TileMap::set_code`] and
//! [`TileMap::set_exact`], one stroke at a time, so each stroke is one
//! undo step.

use crate::map::TileMap;
use reflect::Id;

/// The most cells one fill reaches: every cell of the biggest map.
pub const MAX_FILL: usize = (crate::map::MAX_SIDE * crate::map::MAX_SIDE) as usize;

/// The cells of a straight line from `a` to `b`, both ends included, each
/// touching the last by a side or a corner (Bresenham's line). A brush
/// dragged fast between two frames paints this, so it leaves no gaps.
pub fn line_cells(a: [i64; 2], b: [i64; 2]) -> Vec<[i64; 2]> {
    let (dx, dy) = ((b[0] - a[0]).abs(), -(b[1] - a[1]).abs());
    let (sx, sy) = ((b[0] - a[0]).signum(), (b[1] - a[1]).signum());
    let mut out = Vec::with_capacity((dx.max(-dy) + 1) as usize);
    let (mut x, mut y, mut err) = (a[0], a[1], dx + dy);
    loop {
        out.push([x, y]);
        if x == b[0] && y == b[1] {
            return out;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

/// The corners of the rectangle with corners `a` and `b` (any two
/// opposite ones), clipped to a map of `size`: its top-left and
/// bottom-right cells, or `None` if it's wholly outside.
pub fn rect_within(a: [i64; 2], b: [i64; 2], size: [u32; 2]) -> Option<[[u32; 2]; 2]> {
    let lo = [a[0].min(b[0]).max(0), a[1].min(b[1]).max(0)];
    let hi = [
        a[0].max(b[0]).min(i64::from(size[0]) - 1),
        a[1].max(b[1]).min(i64::from(size[1]) - 1),
    ];
    (lo[0] <= hi[0] && lo[1] <= hi[1])
        .then(|| [[lo[0] as u32, lo[1] as u32], [hi[0] as u32, hi[1] as u32]])
}

/// The cells a fill from `start` reaches on `layer`: every cell joined to
/// it by its sides with the same terrain (or the same emptiness), so a
/// fill keeps to one field and stops at its edges. As cell indices
/// (row by row), in the order reached. Empty if `start` is outside.
pub fn flood_fill(map: &TileMap, layer: usize, start: [u32; 2]) -> Vec<u32> {
    let [w, h] = map.size();
    let Some(cells) = map.layer_cells(layer) else {
        return Vec::new();
    };
    if start[0] >= w || start[1] >= h {
        return Vec::new();
    }
    let at = (start[1] * w + start[0]) as usize;
    let code = cells[at];
    let mut seen = vec![false; cells.len()];
    let mut out = Vec::new();
    let mut todo = vec![at as u32];
    seen[at] = true;
    while let Some(i) = todo.pop() {
        out.push(i);
        let (x, y) = (i % w, i / w);
        let mut visit = |j: u32| {
            let j = j as usize;
            if !seen[j] && cells[j] == code {
                seen[j] = true;
                todo.push(j as u32);
            }
        };
        if x > 0 {
            visit(i - 1);
        }
        if x + 1 < w {
            visit(i + 1);
        }
        if y > 0 {
            visit(i - w);
        }
        if y + 1 < h {
            visit(i + w);
        }
    }
    out
}

/// One cell of a stamp: its terrain's code (0 for empty, which a stamp
/// leaves as it is when put down) and its exact tile, if it has one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StampCell {
    /// The terrain's code, or 0 for a cell the stamp doesn't change.
    pub code: u8,
    /// The piece it shows as it is, if it was painted with one.
    pub exact: Option<Id>,
}

/// An area of a layer copied with the eyedropper, to paint with: flipped
/// and turned, then put down where the pointer is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stamp {
    /// Columns and rows.
    pub size: [u32; 2],
    /// Its cells, row by row.
    pub cells: Vec<StampCell>,
}

impl Stamp {
    /// The area of `layer` between cells `a` and `b` (any two opposite
    /// corners, clipped to the map), or `None` if it's wholly outside or
    /// every cell of it is empty.
    pub fn copy(map: &TileMap, layer: usize, a: [i64; 2], b: [i64; 2]) -> Option<Stamp> {
        let [[x0, y0], [x1, y1]] = rect_within(a, b, map.size())?;
        let mut cells = Vec::with_capacity(((x1 - x0 + 1) * (y1 - y0 + 1)) as usize);
        for y in y0..=y1 {
            for x in x0..=x1 {
                let code = map.code(layer, x, y)?;
                let exact = map.exact(layer, x, y);
                cells.push(StampCell { code, exact });
            }
        }
        cells.iter().any(|c| c.code != 0).then_some(Stamp {
            size: [x1 - x0 + 1, y1 - y0 + 1],
            cells,
        })
    }

    /// One cell, to paint a terrain (or an exact tile) a cell at a time.
    pub fn one(code: u8, exact: Option<Id>) -> Stamp {
        Stamp {
            size: [1, 1],
            cells: vec![StampCell { code, exact }],
        }
    }

    /// Left and right swapped. Exact tiles move with their cells; their
    /// pieces aren't mirrored (pick the mirrored piece if the tileset has
    /// one).
    pub fn flip_x(&mut self) {
        let [w, _] = self.size;
        for row in self.cells.chunks_mut(w as usize) {
            row.reverse();
        }
    }

    /// Top and bottom swapped.
    pub fn flip_y(&mut self) {
        let [w, h] = self.size;
        let (w, h) = (w as usize, h as usize);
        for y in 0..h / 2 {
            for x in 0..w {
                self.cells.swap(y * w + x, (h - 1 - y) * w + x);
            }
        }
    }

    /// Turned a quarter clockwise: its columns become rows.
    pub fn rotate(&mut self) {
        let [w, h] = self.size;
        let (w, h) = (w as usize, h as usize);
        let mut turned = vec![StampCell::default(); w * h];
        for y in 0..h {
            for x in 0..w {
                // The cell at (x, y) goes to column h - 1 - y, row x.
                turned[x * h + (h - 1 - y)] = self.cells[y * w + x];
            }
        }
        self.cells = turned;
        self.size = [h as u32, w as u32];
    }

    /// The cell under the pointer when it's put down: its middle (its
    /// left and top middle when even), so it sits round the pointer.
    pub fn handle(&self) -> [i64; 2] {
        [
            i64::from(self.size[0].saturating_sub(1) / 2),
            i64::from(self.size[1].saturating_sub(1) / 2),
        ]
    }

    /// Each cell it changes put down with its handle at `at`, inside a
    /// map of `size`: the cell and what goes there.
    pub fn placed(
        &self,
        at: [i64; 2],
        size: [u32; 2],
    ) -> impl Iterator<Item = ([u32; 2], StampCell)> + '_ {
        let [w, _] = self.size;
        let [hx, hy] = self.handle();
        self.cells.iter().enumerate().filter_map(move |(i, &c)| {
            if c.code == 0 {
                return None;
            }
            let x = at[0] - hx + (i as u32 % w) as i64;
            let y = at[1] - hy + (i as u32 / w) as i64;
            (x >= 0 && y >= 0 && x < i64::from(size[0]) && y < i64::from(size[1]))
                .then_some(([x as u32, y as u32], c))
        })
    }
}

#[cfg(test)]
#[path = "paint_tests.rs"]
mod tests;
