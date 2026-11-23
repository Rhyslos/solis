//! A picture cut into numbered cells (Fixes 3): a grid's cells, numbered
//! from 0 across and then down as Aseprite numbers frames, then boxes
//! drawn by hand, which move a grid cell or add one past the grid's.
//! The cells become a sheet's frames, so clips play them as they play an
//! Aseprite file's.

use crate::aseprite::{AseFrame, Aseprite};
use crate::sprite_sheet::{SheetError, SpriteSheet, build_sheet};

/// The most cells a picture is cut into.
pub const MAX_CELLS: u32 = 65_536;

/// The most pixels all of a cut picture's frames hold together, before
/// they're trimmed and packed (a quarter of a gigabyte).
pub const MAX_PIXELS: u64 = 1 << 26;

/// How a picture is cut.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Cutting {
    /// A cell's width and height, in pixels.
    pub cell: [u32; 2],
    /// The gap between cells, across and down.
    pub between: [u32; 2],
    /// The margin round the picture's edge, across and down.
    pub edge: [u32; 2],
    /// Boxes drawn by hand (left, top, width, height), by cell number: a
    /// grid cell's number moves that cell, a higher one adds a cell.
    pub drawn: Vec<(u32, [u32; 4])>,
}

impl Cutting {
    /// How many grid cells fit across and down a picture of `size`.
    pub fn across_down(&self, size: [u32; 2]) -> [u32; 2] {
        std::array::from_fn(|i| {
            let (cell, between) = (self.cell[i], self.between[i]);
            let room = size[i].saturating_sub(2 * self.edge[i]);
            if cell == 0 || room < cell {
                0
            } else {
                (room + between) / (cell + between)
            }
        })
    }

    /// Grid cell `n`'s box, for a grid `across` cells wide.
    pub fn grid_box(&self, n: u32, across: u32) -> [u32; 4] {
        let (x, y) = (n % across.max(1), n / across.max(1));
        [
            self.edge[0] + x * (self.cell[0] + self.between[0]),
            self.edge[1] + y * (self.cell[1] + self.between[1]),
            self.cell[0],
            self.cell[1],
        ]
    }

    /// Every cell's box in a picture of `size`, by number (`None` for a
    /// number no cell has), and what was left out, in plain words.
    pub fn boxes(&self, size: [u32; 2]) -> (Vec<Option<[u32; 4]>>, Vec<String>) {
        let mut notes = Vec::new();
        let [across, down] = self.across_down(size);
        let grid = (across * down).min(MAX_CELLS);
        if grid == 0 && self.cell != [0, 0] {
            notes.push(format!(
                "a {} × {} cell doesn't fit in the {} × {} picture, so the grid gives no cells",
                self.cell[0], self.cell[1], size[0], size[1]
            ));
        }
        let mut boxes: Vec<Option<[u32; 4]>> =
            (0..grid).map(|n| Some(self.grid_box(n, across))).collect();
        for &(n, rect) in &self.drawn {
            let [x, y, w, h] = rect;
            if n >= MAX_CELLS {
                notes.push(format!(
                    "cell {n} is past the last a picture can have; left out"
                ));
            } else if w == 0 || h == 0 {
                notes.push(format!("cell {n} has no width or height; left out"));
            } else if u64::from(x) + u64::from(w) > u64::from(size[0])
                || u64::from(y) + u64::from(h) > u64::from(size[1])
            {
                notes.push(format!(
                    "cell {n} reaches past the picture's edge; left out"
                ));
            } else {
                if boxes.len() <= n as usize {
                    boxes.resize(n as usize + 1, None);
                }
                boxes[n as usize] = Some(rect);
            }
        }
        (boxes, notes)
    }
}

/// The cells of `picture` (a plain picture's sheet, its first frame used)
/// as a sheet's frames, numbered as cut. Every frame is the biggest
/// cell's size, each cell drawn in it with its bottom middle at the
/// frame's, so a sprite stands on the same spot whatever the cell; a
/// number no cell has is an empty frame. Also gives what was left out.
pub fn cut_sheet(
    picture: &SpriteSheet,
    cutting: &Cutting,
) -> Result<(SpriteSheet, Vec<String>), SheetError> {
    let size = picture.size;
    let rgba = picture
        .frame_rgba(0)
        .ok_or_else(|| SheetError("the picture has no pixels".into()))?;
    let (boxes, notes) = cutting.boxes(size);
    let frame = boxes
        .iter()
        .flatten()
        .fold([1, 1], |[w, h], b| [w.max(b[2]), h.max(b[3])]);
    if boxes.len() as u64 * u64::from(frame[0]) * u64::from(frame[1]) > MAX_PIXELS {
        return Err(SheetError(format!(
            "{} cells of up to {} × {} pixels are too many to show; cut it into fewer or smaller cells",
            boxes.len(),
            frame[0],
            frame[1]
        )));
    }
    let frames = boxes
        .iter()
        .map(|b| AseFrame {
            duration_ms: 100,
            rgba: b.map_or_else(
                || vec![0; (frame[0] * frame[1] * 4) as usize],
                |b| placed(&rgba, size, b, frame),
            ),
            events: Vec::new(),
        })
        .collect();
    let sheet = build_sheet(Aseprite {
        size: frame,
        frames,
        layers: Vec::new(),
        tags: Vec::new(),
        slices: Vec::new(),
        palette: Vec::new(),
        notes: Vec::new(),
    })?;
    Ok((sheet, notes))
}

/// The pixels of `rect` in a picture of `size`, in a frame of `frame`
/// with their bottom middle at its bottom middle.
fn placed(rgba: &[u8], size: [u32; 2], rect: [u32; 4], frame: [u32; 2]) -> Vec<u8> {
    let [x0, y0, w, h] = rect;
    let (dx, dy) = ((frame[0] - w) / 2, frame[1] - h);
    let mut out = vec![0; (frame[0] * frame[1] * 4) as usize];
    for y in 0..h {
        let from = (((y0 + y) * size[0] + x0) * 4) as usize;
        let to = (((dy + y) * frame[0] + dx) * 4) as usize;
        let len = (w * 4) as usize;
        out[to..to + len].copy_from_slice(&rgba[from..from + len]);
    }
    out
}

#[cfg(test)]
#[path = "cells_tests.rs"]
mod tests;
