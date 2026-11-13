//! Text for game UI (the game UI plan's M1b; decision 0084): fonts read
//! with `skrifa` (`font.rs`, decision 0107), each letter turned once into
//! a distance image (how far every pixel is from the letter's edge) kept
//! in one atlas, so a letter is sharp at any size from one picture of it,
//! and its edge, an outline or a glow are worked out in the shader.
//! Pixel-art screens draw the same letters with hard edges.
//!
//! Layout is simple and exact: left to right, a line per `\n`, each
//! letter's advance and the font's kerning; long lines aren't wrapped yet.

use crate::font::{Font, GlyphId};
use std::collections::HashMap;

/// The size letters are drawn at for their distance images, in pixels.
pub const SDF_SIZE: f32 = 48.0;
/// How far, in those pixels, a distance image reaches past a letter's
/// edge: as far as an outline or glow can go.
pub const SPREAD: f32 = 6.0;
/// The atlas's side, in pixels (one channel).
pub const ATLAS_SIDE: u32 = 1024;
/// The most letters a text holds (more are left out).
pub const MAX_LETTERS: usize = 4096;
/// The biggest a font file may be.
pub const MAX_FONT_BYTES: usize = 32 << 20;

/// A letter's distance image in the atlas.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Letter {
    /// Its place in the atlas: left, top, right, bottom, in pixels.
    pub atlas: [u16; 4],
    /// Where its image's top left corner is from the pen (on the
    /// baseline), at [`SDF_SIZE`].
    pub offset: [f32; 2],
}

/// One letter placed: its box on the screen and its place in the atlas.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placed {
    /// Left, top, right, bottom, in screen pixels.
    pub rect: [f32; 4],
    /// Its place in the atlas, in pixels.
    pub atlas: [u16; 4],
}

/// The atlas: one channel, a byte a pixel (128 is the edge).
#[derive(Debug, Clone)]
pub struct Atlas {
    /// Its pixels, row by row.
    pub pixels: Vec<u8>,
    /// Different for every atlas and after every change (a letter added,
    /// room made), so the renderer uploads it again only then, and never
    /// mistakes another set of fonts' atlas for this one.
    pub revision: u64,
    /// The shelf being filled: its top, its height, and how far along.
    shelf: [u32; 3],
}

impl Default for Atlas {
    fn default() -> Self {
        Self {
            pixels: vec![0; (ATLAS_SIDE * ATLAS_SIDE) as usize],
            revision: crate::next_revision(),
            shelf: [0, 0, 0],
        }
    }
}

impl Atlas {
    /// Room for an image `w` × `h`, on the current shelf or a new one
    /// below; none when the atlas is full.
    fn room(&mut self, w: u32, h: u32) -> Option<[u32; 2]> {
        if w > ATLAS_SIDE || h > ATLAS_SIDE {
            return None;
        }
        let [top, height, along] = self.shelf;
        if along + w <= ATLAS_SIDE && top + h <= ATLAS_SIDE {
            self.shelf = [top, height.max(h), along + w + 1];
            return Some([along, top]);
        }
        let next = top + height + 1;
        if next + h > ATLAS_SIDE {
            return None;
        }
        self.shelf = [next, h, w + 1];
        Some([0, next])
    }
}

/// The fonts a game's UI uses, and their letters' distance images.
///
/// Call [`Self::begin_frame`] once a frame, before laying out any text:
/// the atlas changes only then, so every letter placed during a frame
/// keeps its place in the atlas until the frame is drawn.
#[derive(Debug, Clone)]
pub struct Fonts {
    fonts: Vec<Font>,
    /// Each letter made (none for one with no outline, a space), and the
    /// frame it was last placed in.
    letters: HashMap<(usize, GlyphId), Option<(Letter, u64)>>,
    /// The atlas the letters are in.
    pub atlas: Atlas,
    /// Whether a letter didn't fit (the atlas is full): said once.
    pub full: bool,
    /// Whether a letter didn't fit this frame: the next frame makes room,
    /// letting go of letters the frame didn't show (the stability review:
    /// a full atlas used to lose new letters for good).
    start_again: bool,
    /// The frame being laid out, counted by [`Self::begin_frame`].
    frame: u64,
}

/// What making a letter came to.
enum Made {
    /// Its distance image, in the atlas.
    Letter(Letter),
    /// It has no outline (a space): nothing to draw.
    Blank,
    /// The atlas has no room for it this frame.
    NoRoom,
}

impl Fonts {
    /// Fonts from their files' bytes, in order (the first is the
    /// standard). A file that isn't a font, or is too big, is left out;
    /// the reasons come back.
    pub fn new(files: Vec<Vec<u8>>) -> (Self, Vec<String>) {
        let mut fonts = Vec::new();
        let mut notes = Vec::new();
        for (i, bytes) in files.into_iter().enumerate() {
            if bytes.len() > MAX_FONT_BYTES {
                notes.push(format!(
                    "font {} is bigger than {} MB",
                    i + 1,
                    MAX_FONT_BYTES >> 20
                ));
                continue;
            }
            match Font::read(bytes) {
                Some(font) => fonts.push(font),
                None => notes.push(format!("font {} isn't a font Solis can read", i + 1)),
            }
        }
        (
            Self {
                fonts,
                letters: HashMap::new(),
                atlas: Atlas::default(),
                full: false,
                start_again: false,
                frame: 0,
            },
            notes,
        )
    }

    /// How many fonts there are.
    pub fn len(&self) -> usize {
        self.fonts.len()
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.fonts.is_empty()
    }

    /// Starts a frame: when a letter didn't fit during the last one, the
    /// atlas makes room by letting go of every letter that frame didn't
    /// place, moving the rest together (copied, not made again). When the
    /// last frame placed every letter in the atlas, it needs more than fit:
    /// the atlas stays as it is, and the letters that don't fit are left
    /// out (nothing is made again every frame).
    pub fn begin_frame(&mut self) {
        if std::mem::take(&mut self.start_again) {
            self.make_room();
        }
        self.frame += 1;
    }

    /// Lets go of the letters the frame just ended didn't place, and packs
    /// the rest into a new atlas, in their old order.
    fn make_room(&mut self) {
        let shown = self.frame;
        let mut kept: Vec<((usize, GlyphId), Letter)> = self
            .letters
            .iter()
            .filter_map(|(&key, made)| {
                made.filter(|(_, used)| *used == shown)
                    .map(|(l, _)| (key, l))
            })
            .collect();
        let in_atlas = self.letters.values().filter(|made| made.is_some()).count();
        if kept.len() == in_atlas {
            return;
        }
        // Top to bottom, left to right: the order they were packed in.
        kept.sort_by_key(|(_, l)| [l.atlas[1], l.atlas[0]]);
        let old = std::mem::take(&mut self.atlas);
        self.letters.retain(|_, made| made.is_none());
        for (key, letter) in kept {
            let [l, t, r, b] = letter.atlas.map(u32::from);
            let (w, h) = (r - l, b - t);
            let Some([x, y]) = self.atlas.room(w, h) else {
                continue;
            };
            for row in 0..h {
                let from = ((t + row) * ATLAS_SIDE + l) as usize;
                let to = ((y + row) * ATLAS_SIDE + x) as usize;
                self.atlas.pixels[to..to + w as usize]
                    .copy_from_slice(&old.pixels[from..from + w as usize]);
            }
            let moved = Letter {
                atlas: [x as u16, y as u16, (x + w) as u16, (y + h) as u16],
                ..letter
            };
            self.letters.insert(key, Some((moved, shown)));
        }
    }

    /// The letter `id` of font `font`, its distance image made the first
    /// time it's asked for. None for a letter with no outline (a space)
    /// or when the atlas has no room for it this frame.
    fn letter(&mut self, font: usize, id: GlyphId) -> Option<Letter> {
        let frame = self.frame;
        if let Some(found) = self.letters.get_mut(&(font, id)) {
            return found.as_mut().map(|(letter, used)| {
                *used = frame;
                *letter
            });
        }
        match self.make(font, id) {
            Made::Letter(letter) => {
                self.letters.insert((font, id), Some((letter, frame)));
                Some(letter)
            }
            Made::Blank => {
                self.letters.insert((font, id), None);
                None
            }
            // Not remembered: tried again once the atlas has made room.
            Made::NoRoom => None,
        }
    }

    fn make(&mut self, font: usize, id: GlyphId) -> Made {
        let Some(outlined) = self
            .fonts
            .get(font)
            .and_then(|f| f.at(SDF_SIZE))
            .and_then(|f| f.outline(id))
        else {
            return Made::Blank;
        };
        let [left, top, right, bottom] = outlined.bounds();
        let pad = SPREAD.ceil() as i32 + 1;
        let (bw, bh) = ((right - left).ceil() as i32, (bottom - top).ceil() as i32);
        if bw <= 0 || bh <= 0 || bw > 512 || bh > 512 {
            return Made::Blank;
        }
        let (w, h) = (bw + 2 * pad, bh + 2 * pad);
        // Room first: a full atlas costs no distance image.
        let Some([ax, ay]) = self.atlas.room(w as u32, h as u32) else {
            self.full = true;
            self.start_again = true;
            return Made::NoRoom;
        };
        let mut cover = vec![0.0_f32; (w * h) as usize];
        outlined.draw(|x, y, c| {
            let (x, y) = (x as i32 + pad, y as i32 + pad);
            if (0..w).contains(&x) && (0..h).contains(&y) {
                cover[(y * w + x) as usize] = c;
            }
        });
        let sdf = distance_image(&cover, w as usize, h as usize);
        for y in 0..h as u32 {
            let row = ((ay + y) * ATLAS_SIDE + ax) as usize;
            let from = (y * w as u32) as usize;
            self.atlas.pixels[row..row + w as usize].copy_from_slice(&sdf[from..from + w as usize]);
        }
        self.atlas.revision = crate::next_revision();
        Made::Letter(Letter {
            atlas: [
                ax as u16,
                ay as u16,
                (ax + w as u32) as u16,
                (ay + h as u32) as u16,
            ],
            offset: [left - pad as f32, top - pad as f32],
        })
    }

    /// How wide `text` is at `size` pixels in font `font`, and how tall a
    /// line is.
    pub fn measure(&self, font: usize, text: &str, size: f32) -> [f32; 2] {
        let Some(scaled) = self
            .fonts
            .get(font.min(self.fonts.len().saturating_sub(1)))
            .and_then(|f| f.at(size.max(0.0)))
        else {
            return [0.0, 0.0];
        };
        let line = scaled.line();
        let mut widest = 0.0_f32;
        let mut lines = 0;
        for text_line in text.split('\n') {
            lines += 1;
            let mut x = 0.0;
            let mut last: Option<GlyphId> = None;
            for c in text_line.chars().take(MAX_LETTERS) {
                let id = scaled.glyph(c);
                if let Some(prev) = last {
                    x += scaled.kern(prev, id);
                }
                x += scaled.advance(id);
                last = Some(id);
            }
            widest = widest.max(x);
        }
        [widest, line * lines as f32]
    }

    /// Lays `text` out at `size` pixels in font `font` with its first
    /// line's top left corner at `at`: each letter's box on the screen and
    /// its place in the atlas. With `crisp`, each letter lands on whole
    /// multiples of `step` (pixel art).
    pub fn place(
        &mut self,
        font: usize,
        text: &str,
        size: f32,
        at: [f32; 2],
        crisp: Option<f32>,
    ) -> Vec<Placed> {
        let font = font.min(self.fonts.len().saturating_sub(1));
        let Some(f) = self.fonts.get(font).cloned() else {
            return Vec::new();
        };
        let size = if size.is_finite() {
            size.clamp(0.0, 4096.0)
        } else {
            0.0
        };
        let Some(scaled) = f.at(size) else {
            return Vec::new();
        };
        let k = size / SDF_SIZE;
        let line = scaled.line();
        let mut out = Vec::new();
        let mut y = at[1] + scaled.ascent();
        for text_line in text.split('\n') {
            let mut x = at[0];
            let mut last: Option<GlyphId> = None;
            for c in text_line.chars() {
                if out.len() >= MAX_LETTERS {
                    return out;
                }
                let id = scaled.glyph(c);
                if let Some(prev) = last {
                    x += scaled.kern(prev, id);
                }
                if let Some(letter) = self.letter(font, id) {
                    let [l, t, r, b] = letter.atlas.map(f32::from);
                    let mut left = x + letter.offset[0] * k;
                    let mut top = y + letter.offset[1] * k;
                    if let Some(step) = crisp.filter(|s| *s > 0.0) {
                        left = (left / step).round() * step;
                        top = (top / step).round() * step;
                    }
                    out.push(Placed {
                        rect: [left, top, left + (r - l) * k, top + (b - t) * k],
                        atlas: letter.atlas,
                    });
                }
                x += scaled.advance(id);
                last = Some(id);
            }
            y += line;
        }
        out
    }
}

/// A distance image from a letter's coverage (0 to 1 a pixel): 128 on
/// the edge, more inside and less outside, [`SPREAD`] pixels to the
/// ends. Each pixel looks at its neighbours within the spread for the
/// nearest of the other side.
fn distance_image(cover: &[f32], w: usize, h: usize) -> Vec<u8> {
    let inside = |x: i32, y: i32| -> bool {
        x >= 0
            && y >= 0
            && (x as usize) < w
            && (y as usize) < h
            && cover[y as usize * w + x as usize] >= 0.5
    };
    let reach = SPREAD.ceil() as i32;
    let mut out = vec![0_u8; w * h];
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let me = inside(x, y);
            let mut best = SPREAD * SPREAD;
            for dy in -reach..=reach {
                for dx in -reach..=reach {
                    let d = (dx * dx + dy * dy) as f32;
                    if d < best && inside(x + dx, y + dy) != me {
                        best = d;
                    }
                }
            }
            // The edge lies half a pixel between two pixels.
            let d = (best.sqrt() - 0.5).clamp(0.0, SPREAD);
            let signed = if me { d } else { -d };
            out[y as usize * w + x as usize] =
                (128.0 + signed / SPREAD * 127.0).round().clamp(0.0, 255.0) as u8;
        }
    }
    out
}

#[cfg(test)]
#[path = "text_tests.rs"]
mod tests;
