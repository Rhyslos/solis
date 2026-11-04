//! Walls from a tile map's solid cells (decision 0071): every cell whose
//! terrain is solid, on any layer, merged into as few rectangles as a
//! greedy sweep finds, chunk by chunk. Never one shape per tile: a
//! character sliding along a fence meets one long edge, not a seam at
//! every tile, and a changed cell rebuilds only its own chunk's walls.

use crate::{CHUNK, TileMap, Tileset};

/// Which codes are solid, by code (a map cell's byte).
pub type Solid = [bool; 256];

/// The solid codes of `tileset`'s terrains.
pub fn solid_codes(tileset: &Tileset) -> Solid {
    let mut solid = [false; 256];
    for t in &tileset.terrains {
        if t.solid && t.code.is_ascii() {
            solid[t.code as usize] = true;
        }
    }
    solid
}

/// A rectangle of cells: its top-left cell and its size, in cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    /// Its left column.
    pub x: u32,
    /// Its top row.
    pub y: u32,
    /// How many columns wide.
    pub w: u32,
    /// How many rows tall.
    pub h: u32,
}

/// Whether the cell is solid on any layer.
fn solid_at(map: &TileMap, solid: &Solid, x: u32, y: u32) -> bool {
    (0..map.layers().len()).any(|l| map.code(l, x, y).is_some_and(|c| solid[c as usize]))
}

/// The solid cells of chunk `chunk` (column and row, in chunks) merged
/// into rectangles that never leave the chunk, in the order found (rows
/// from the top, each from the left), so the same map always gives the
/// same walls.
pub fn chunk_walls(map: &TileMap, solid: &Solid, chunk: [u32; 2]) -> Vec<Rect> {
    let [w, h] = map.size();
    let x0 = chunk[0].saturating_mul(CHUNK);
    let y0 = chunk[1].saturating_mul(CHUNK);
    if x0 >= w || y0 >= h || !solid.iter().any(|s| *s) {
        return Vec::new();
    }
    let cw = (w - x0).min(CHUNK) as usize;
    let ch = (h - y0).min(CHUNK) as usize;
    // Solid and not yet in a rectangle.
    let mut open = vec![false; cw * ch];
    for y in 0..ch {
        for x in 0..cw {
            open[y * cw + x] = solid_at(map, solid, x0 + x as u32, y0 + y as u32);
        }
    }
    let mut out = Vec::new();
    for y in 0..ch {
        for x in 0..cw {
            if !open[y * cw + x] {
                continue;
            }
            // As wide as the row allows, then as tall as every row below
            // stays solid across that width.
            let mut rw = 1;
            while x + rw < cw && open[y * cw + x + rw] {
                rw += 1;
            }
            let mut rh = 1;
            while y + rh < ch && (x..x + rw).all(|i| open[(y + rh) * cw + i]) {
                rh += 1;
            }
            for yy in y..y + rh {
                for xx in x..x + rw {
                    open[yy * cw + xx] = false;
                }
            }
            out.push(Rect {
                x: x0 + x as u32,
                y: y0 + y as u32,
                w: rw as u32,
                h: rh as u32,
            });
        }
    }
    out
}

#[cfg(test)]
#[path = "walls_tests.rs"]
mod tests;
