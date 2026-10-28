//! A sprite sheet (Phase 9, D2): an Aseprite file's frames packed into one
//! picture, an *atlas*, ready to upload, with where each frame is.
//!
//! Each frame is trimmed to what's drawn in it (its see-through border is
//! left out, and where it was is kept as an offset), frames that are the
//! same are packed once, and each is packed with a one-pixel border
//! copied from its own edge (*extruded*), plus a pixel of space, so a
//! frame never shows a sliver of its neighbour.

use crate::aseprite::{AseLayer, AseSlice, AseTag, Aseprite};

/// The widest and tallest atlas, in pixels: every graphics card Solis
/// runs on holds a texture this size.
pub const MAX_ATLAS_SIDE: u32 = 4096;

/// One frame of a sheet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetFrame {
    /// Where its trimmed picture is in the atlas: left, top, width and
    /// height, in pixels (width 0 for a frame with nothing drawn).
    pub rect: [u32; 4],
    /// Where that picture's top-left corner is on the canvas.
    pub offset: [u32; 2],
    /// How long it shows, in milliseconds.
    pub duration_ms: u32,
    /// Events on it (user data on its cels).
    pub events: Vec<String>,
}

/// An Aseprite file's frames packed into one atlas, with everything else
/// it holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpriteSheet {
    /// The canvas's width and height: every frame's size, in pixels.
    pub size: [u32; 2],
    /// The atlas's width and height, in pixels.
    pub atlas_size: [u32; 2],
    /// The atlas, straight RGBA, rows top to bottom.
    pub atlas: Vec<u8>,
    /// Its frames, in order.
    pub frames: Vec<SheetFrame>,
    /// Its tags (clips).
    pub tags: Vec<AseTag>,
    /// Its slices.
    pub slices: Vec<AseSlice>,
    /// Its layers.
    pub layers: Vec<AseLayer>,
    /// Its palette.
    pub palette: Vec<[u8; 4]>,
    /// What was put right or skipped while reading it.
    pub notes: Vec<String>,
}

/// Why a sheet couldn't be packed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetError(pub String);

impl std::fmt::Display for SheetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for SheetError {}

/// A frame's drawn part: where it is on the canvas and its pixels.
struct Trimmed {
    offset: [u32; 2],
    size: [u32; 2],
    rgba: Vec<u8>,
}

/// The part of a `size` frame that isn't fully see-through.
fn trim(size: [u32; 2], rgba: &[u8]) -> Trimmed {
    let [w, h] = size;
    let opaque = |x: u32, y: u32| rgba[((y * w + x) * 4 + 3) as usize] != 0;
    let (mut left, mut top, mut right, mut bottom) = (w, h, 0, 0);
    for y in 0..h {
        for x in 0..w {
            if opaque(x, y) {
                left = left.min(x);
                right = right.max(x + 1);
                top = top.min(y);
                bottom = bottom.max(y + 1);
            }
        }
    }
    if right <= left || bottom <= top {
        return Trimmed {
            offset: [0, 0],
            size: [0, 0],
            rgba: Vec::new(),
        };
    }
    let (tw, th) = (right - left, bottom - top);
    let mut out = Vec::with_capacity((tw * th * 4) as usize);
    for y in top..bottom {
        let start = ((y * w + left) * 4) as usize;
        out.extend_from_slice(&rgba[start..start + (tw * 4) as usize]);
    }
    Trimmed {
        offset: [left, top],
        size: [tw, th],
        rgba: out,
    }
}

/// Packs `file`'s frames into a sheet.
pub fn build_sheet(file: Aseprite) -> Result<SpriteSheet, SheetError> {
    let trimmed: Vec<Trimmed> = file
        .frames
        .iter()
        .map(|f| trim(file.size, &f.rgba))
        .collect();
    // Frames that look the same are packed once.
    let mut unique: Vec<usize> = Vec::new();
    let mut same_as: Vec<usize> = Vec::with_capacity(trimmed.len());
    for (i, t) in trimmed.iter().enumerate() {
        let found = unique
            .iter()
            .position(|&u| trimmed[u].size == t.size && trimmed[u].rgba == t.rgba);
        match found {
            Some(k) => same_as.push(k),
            None => {
                same_as.push(unique.len());
                unique.push(i);
            }
        }
    }
    // Shelves, tallest first: each picture takes its size plus its
    // extruded border and a pixel of space.
    let cell = |t: &Trimmed| [t.size[0] + 3, t.size[1] + 3];
    let area: u64 = unique
        .iter()
        .map(|&u| {
            let [w, h] = cell(&trimmed[u]);
            u64::from(w) * u64::from(h)
        })
        .sum();
    let widest = unique
        .iter()
        .map(|&u| cell(&trimmed[u])[0])
        .max()
        .unwrap_or(1);
    let mut width = ((area as f64).sqrt().ceil() as u32)
        .max(widest)
        .next_power_of_two()
        .max(16);
    let mut order: Vec<usize> = (0..unique.len()).collect();
    order.sort_by_key(|&k| std::cmp::Reverse(trimmed[unique[k]].size[1]));
    let places = loop {
        if width > MAX_ATLAS_SIDE {
            return Err(SheetError(format!(
                "its frames don't fit in a {MAX_ATLAS_SIDE} × {MAX_ATLAS_SIDE} picture; split it \
                 into smaller files"
            )));
        }
        let mut places = vec![[0u32; 2]; unique.len()];
        let (mut x, mut y, mut shelf) = (0u32, 0u32, 0u32);
        for &k in &order {
            let [w, h] = cell(&trimmed[unique[k]]);
            if x + w > width {
                x = 0;
                y += shelf;
                shelf = 0;
            }
            places[k] = [x, y];
            x += w;
            shelf = shelf.max(h);
        }
        let height = y + shelf;
        if height <= MAX_ATLAS_SIDE {
            break (places, height.max(1));
        }
        width *= 2;
    };
    let (places, height) = places;
    let mut atlas = vec![0u8; width as usize * height as usize * 4];
    let mut rects = vec![[0u32; 4]; unique.len()];
    for (k, &u) in unique.iter().enumerate() {
        let t = &trimmed[u];
        let [w, h] = t.size;
        if w == 0 {
            continue;
        }
        // The picture sits one pixel in from its cell's corner.
        let [cx, cy] = places[k];
        let (left, top) = (cx + 1, cy + 1);
        rects[k] = [left, top, w, h];
        // Every pixel of the cell's border copies the nearest edge pixel.
        for y in 0..h + 2 {
            for x in 0..w + 2 {
                let sx = x.saturating_sub(1).min(w - 1);
                let sy = y.saturating_sub(1).min(h - 1);
                let s = ((sy * w + sx) * 4) as usize;
                let d = (((cy + y) * width + cx + x) * 4) as usize;
                atlas[d..d + 4].copy_from_slice(&t.rgba[s..s + 4]);
            }
        }
    }
    let frames = file
        .frames
        .iter()
        .zip(&trimmed)
        .zip(&same_as)
        .map(|((f, t), &k)| SheetFrame {
            rect: rects[k],
            offset: t.offset,
            duration_ms: f.duration_ms,
            events: f.events.clone(),
        })
        .collect();
    Ok(SpriteSheet {
        size: file.size,
        atlas_size: [width, height],
        atlas,
        frames,
        tags: file.tags,
        slices: file.slices,
        layers: file.layers,
        palette: file.palette,
        notes: file.notes,
    })
}

impl SpriteSheet {
    /// Frame `i` as its own straight-RGBA picture, the canvas's size (for
    /// thumbnails and Properties).
    pub fn frame_rgba(&self, i: usize) -> Option<Vec<u8>> {
        let f = self.frames.get(i)?;
        let [w, h] = self.size;
        let mut out = vec![0u8; (w * h * 4) as usize];
        let [ax, ay, fw, fh] = f.rect;
        for y in 0..fh {
            for x in 0..fw {
                let s = (((ay + y) * self.atlas_size[0] + ax + x) * 4) as usize;
                let (dx, dy) = (f.offset[0] + x, f.offset[1] + y);
                if dx < w && dy < h {
                    let d = ((dy * w + dx) * 4) as usize;
                    out[d..d + 4].copy_from_slice(&self.atlas[s..s + 4]);
                }
            }
        }
        Some(out)
    }

    /// The slice named `name`, if the sheet has one.
    pub fn slice(&self, name: &str) -> Option<&AseSlice> {
        self.slices.iter().find(|s| s.name == name)
    }

    /// Where the sprite stands on frame `frame`, in the canvas's pixels
    /// from its top-left corner: the pivot of the slice named `pivot` (or
    /// its middle, without one), or the bottom middle of the canvas.
    pub fn pivot(&self, pivot: &str, frame: u32) -> [f32; 2] {
        let from_slice = (!pivot.is_empty())
            .then(|| self.slice(pivot))
            .flatten()
            .and_then(|s| s.key_at(frame))
            .map(|k| match k.pivot {
                Some([px, py]) => [(k.rect[0] + px) as f32, (k.rect[1] + py) as f32],
                None => [
                    k.rect[0] as f32 + k.rect[2] as f32 / 2.0,
                    k.rect[1] as f32 + k.rect[3] as f32 / 2.0,
                ],
            });
        from_slice.unwrap_or([self.size[0] as f32 / 2.0, self.size[1] as f32])
    }

    /// Bytes the atlas takes.
    pub fn atlas_bytes(&self) -> usize {
        self.atlas.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aseprite::{read_aseprite, tests as ase};

    fn walker() -> SpriteSheet {
        build_sheet(read_aseprite(&ase::write_test_file(&ase::walker())).unwrap()).unwrap()
    }

    #[test]
    fn frames_are_trimmed_packed_and_drawn_back_the_same() {
        let file = read_aseprite(&ase::write_test_file(&ase::walker())).unwrap();
        let sheet = build_sheet(file.clone()).unwrap();
        assert_eq!(sheet.frames.len(), 4);
        for (i, f) in file.frames.iter().enumerate() {
            assert_eq!(sheet.frame_rgba(i).unwrap(), f.rgba, "frame {i}");
        }
        // Trimmed to the drawn part: the hat's top to the body's bottom.
        assert_eq!(sheet.frames[0].offset, [4, 4]);
        assert_eq!(&sheet.frames[0].rect[2..], &[8, 20]);
        assert!(sheet.atlas_size[0].is_power_of_two());
        assert_eq!(sheet.frames[2].events, ["footstep"]);
    }

    #[test]
    fn frames_that_look_the_same_are_packed_once() {
        let mut f = ase::walker();
        f.frames[3].1[0] = f.frames[1].1[0].clone();
        let sheet = build_sheet(read_aseprite(&ase::write_test_file(&f)).unwrap()).unwrap();
        assert_eq!(sheet.frames[3].rect, sheet.frames[1].rect);
        assert_ne!(sheet.frames[2].rect, sheet.frames[1].rect);
    }

    #[test]
    fn each_frame_s_border_copies_its_edge_and_frames_never_touch() {
        let sheet = walker();
        let [aw, _] = sheet.atlas_size;
        let at = |x: u32, y: u32| {
            let i = ((y * aw + x) * 4) as usize;
            [
                sheet.atlas[i],
                sheet.atlas[i + 1],
                sheet.atlas[i + 2],
                sheet.atlas[i + 3],
            ]
        };
        for f in &sheet.frames {
            let [x, y, w, h] = f.rect;
            // The border round the picture copies its edge.
            assert_eq!(at(x - 1, y), at(x, y));
            assert_eq!(at(x + w, y + h - 1), at(x + w - 1, y + h - 1));
            assert_eq!(at(x - 1, y + h), at(x, y + h - 1));
        }
        for (i, a) in sheet.frames.iter().enumerate() {
            for b in &sheet.frames[i + 1..] {
                if a.rect == b.rect {
                    continue;
                }
                let apart = a.rect[0] + a.rect[2] + 2 <= b.rect[0]
                    || b.rect[0] + b.rect[2] + 2 <= a.rect[0]
                    || a.rect[1] + a.rect[3] + 2 <= b.rect[1]
                    || b.rect[1] + b.rect[3] + 2 <= a.rect[1];
                assert!(apart, "{:?} {:?}", a.rect, b.rect);
            }
        }
    }

    #[test]
    fn the_pivot_is_the_slice_s_or_the_bottom_middle() {
        let sheet = walker();
        assert_eq!(sheet.pivot("feet", 0), [8.0, 23.0]);
        assert_eq!(
            sheet.pivot("hand", 3),
            [14.0, 13.0],
            "the middle, past its second key"
        );
        assert_eq!(sheet.pivot("", 0), [8.0, 24.0]);
        assert_eq!(sheet.pivot("nowhere", 0), [8.0, 24.0]);
    }

    #[test]
    fn an_empty_frame_packs_to_nothing() {
        let mut f = ase::walker();
        f.frames[1].1.clear();
        let sheet = build_sheet(read_aseprite(&ase::write_test_file(&f)).unwrap()).unwrap();
        assert_eq!(sheet.frames[1].rect[2], 0);
        assert!(sheet.frame_rgba(1).unwrap().iter().all(|&b| b == 0));
    }

    #[test]
    fn frames_too_many_to_fit_are_refused() {
        let file = Aseprite {
            size: [2048, 2048],
            frames: (0..5)
                .map(|i| crate::aseprite::AseFrame {
                    duration_ms: 100,
                    rgba: vec![i as u8 + 1; 2048 * 2048 * 4],
                    events: Vec::new(),
                })
                .collect(),
            layers: Vec::new(),
            tags: Vec::new(),
            slices: Vec::new(),
            palette: Vec::new(),
            notes: Vec::new(),
        };
        assert!(build_sheet(file).unwrap_err().0.contains("don't fit"));
    }
}
