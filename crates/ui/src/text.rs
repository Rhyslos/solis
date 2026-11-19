//! Text for game UI (the game UI plan's M1b; decision 0084): fonts read
//! with `skrifa` (`font.rs`, decision 0107), each letter turned once into
//! a distance image (how far every pixel is from the letter's edge) kept
//! in one atlas, so a letter is sharp at any size from one picture of it,
//! and its edge, an outline or a glow are worked out in the shader.
//! Pixel-art screens draw the same letters with hard edges.
//!
//! Text is laid out as paragraphs (`paragraph.rs`, stage 1's text
//! layout): shaped, wrapped and cut, in the project's own fonts with
//! fallbacks ([`FontFiles`]), and kept while it stays the same, so text
//! that doesn't change costs nothing each frame.

use crate::font::{Font, GlyphId};
pub use crate::font_files::FontFiles;
pub(crate) use crate::font_files::Slot;
use crate::paragraph::{Builder, Fit, Paragraph, TextStyle};
use crate::screen::Edge;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

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
/// The most fonts of its own a project's screens use.
pub const MAX_PROJECT_FONTS: usize = 32;
/// The most paragraphs kept laid out; past it, those used longest ago go.
pub const MAX_PARAGRAPHS: usize = 8192;

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

/// A paragraph kept, with what it was laid out from.
#[derive(Debug, Clone)]
struct Kept {
    text: Box<str>,
    style: TextStyle,
    fit: Fit,
    paragraph: Arc<Paragraph>,
    /// When it was last asked for, by [`Fonts::paragraph`]'s count.
    used: u64,
}

/// The fonts a game's UI uses, and their letters' distance images.
///
/// Call [`Self::begin_frame`] once a frame, before laying out any text:
/// the atlas changes only then, so every letter placed during a frame
/// keeps its place in the atlas until the frame is drawn.
#[derive(Debug, Clone)]
pub struct Fonts {
    /// The game's own fonts first, then the project's.
    fonts: Vec<Slot>,
    /// How many of [`Self::fonts`] are the game's own.
    own: usize,
    /// The revision of the project's fonts in use.
    files: Option<u64>,
    /// Each letter made (none for one with no outline, a space), by its
    /// font's key, and the frame it was last placed in.
    letters: HashMap<(u64, GlyphId), Option<(Letter, u64)>>,
    /// Paragraphs laid out, by a hash of what they're laid out from.
    paragraphs: HashMap<u64, Vec<Kept>>,
    kept: usize,
    /// Counts paragraphs asked for, to know which were used longest ago.
    asked: u64,
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
    /// The game's own fonts from their files' bytes, in order (the first
    /// is the standard, the second its bold). A file that isn't a font, or
    /// is too big, is left out; the reasons come back.
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
                Some(font) => fonts.push(Slot::new("", crate::next_revision(), font)),
                None => notes.push(format!("font {} isn't a font Solis can read", i + 1)),
            }
        }
        let own = fonts.len();
        (
            Self {
                fonts,
                own,
                files: None,
                letters: HashMap::new(),
                paragraphs: HashMap::new(),
                kept: 0,
                asked: 0,
                atlas: Atlas::default(),
                full: false,
                start_again: false,
                frame: 0,
            },
            notes,
        )
    }

    /// How many fonts there are, the project's included.
    pub fn len(&self) -> usize {
        self.fonts.len()
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.fonts.is_empty()
    }

    /// Uses the project's fonts `files` (none: only the game's own) from
    /// now on. Nothing changes when they're the ones in use already.
    pub fn use_files(&mut self, files: Option<&FontFiles>) {
        let revision = files.map(|f| f.revision);
        if revision == self.files {
            return;
        }
        self.files = revision;
        self.fonts.truncate(self.own);
        for file in files.map_or(&[][..], |f| &f.files[..]) {
            if let Some(font) = Font::read(file.bytes.to_vec()) {
                self.fonts.push(Slot::new(&file.name, file.uid, font));
            }
        }
        self.paragraphs.clear();
        self.kept = 0;
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
        let mut kept: Vec<((u64, GlyphId), Letter)> = self
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

    /// The letter `id` of the font in place `slot`, its distance image
    /// made the first time it's asked for. None for a letter with no
    /// outline (a space) or when the atlas has no room for it this frame.
    fn letter(&mut self, slot: usize, id: GlyphId) -> Option<Letter> {
        let frame = self.frame;
        let key = (self.fonts.get(slot)?.uid, id);
        if let Some(found) = self.letters.get_mut(&key) {
            return found.as_mut().map(|(letter, used)| {
                *used = frame;
                *letter
            });
        }
        match self.make(slot, id) {
            Made::Letter(letter) => {
                self.letters.insert(key, Some((letter, frame)));
                Some(letter)
            }
            Made::Blank => {
                self.letters.insert(key, None);
                None
            }
            // Not remembered: tried again once the atlas has made room.
            Made::NoRoom => None,
        }
    }

    fn make(&mut self, slot: usize, id: GlyphId) -> Made {
        let Some(outlined) = self
            .fonts
            .get(slot)
            .and_then(|f| f.font.outline(id, SDF_SIZE))
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

    /// The fonts `style` asks for, in order: the project's it names that
    /// are in use, then the game's own [`TextStyle::font`].
    fn stack(&self, style: &TextStyle) -> Vec<usize> {
        let mut stack: Vec<usize> = style
            .fonts
            .iter()
            .filter_map(|name| {
                let name = name.trim().replace('\\', "/");
                (self.own..self.fonts.len())
                    .find(|&i| self.fonts[i].name.eq_ignore_ascii_case(&name))
            })
            .collect();
        if self.own > 0 {
            stack.push(style.font.min(self.own - 1));
        } else if self.fonts.is_empty() {
            return Vec::new();
        }
        if stack.is_empty() {
            stack.push(0);
        }
        stack.dedup();
        stack
    }

    /// `text` laid out in `style` to `fit` (in design pixels at the text's
    /// size): made the first time it's asked for, and kept while it's
    /// asked for again. With no fonts at all, an empty paragraph.
    pub fn paragraph(&mut self, text: &str, style: &TextStyle, fit: &Fit) -> Arc<Paragraph> {
        self.asked += 1;
        let hash = paragraph_hash(text, style, fit);
        let asked = self.asked;
        if let Some(found) = self.paragraphs.get_mut(&hash).and_then(|kept| {
            kept.iter_mut()
                .find(|k| &*k.text == text && k.style == *style && k.fit == *fit)
        }) {
            found.used = asked;
            return Arc::clone(&found.paragraph);
        }
        let stack = self.stack(style);
        let paragraph = if stack.is_empty() {
            Arc::new(Paragraph::default())
        } else {
            Arc::new(
                Builder {
                    slots: &mut self.fonts,
                    stack: &stack,
                    style,
                    fit: *fit,
                    max_letters: MAX_LETTERS,
                }
                .build(text),
            )
        };
        if self.kept >= MAX_PARAGRAPHS {
            self.forget_old_paragraphs();
        }
        self.kept += 1;
        self.paragraphs.entry(hash).or_default().push(Kept {
            text: text.into(),
            style: style.clone(),
            fit: *fit,
            paragraph: Arc::clone(&paragraph),
            used: asked,
        });
        paragraph
    }

    /// Lets go of the half of the paragraphs kept that were asked for
    /// longest ago.
    fn forget_old_paragraphs(&mut self) {
        let since = self.asked.saturating_sub(MAX_PARAGRAPHS as u64 / 2);
        let mut kept = 0;
        self.paragraphs.retain(|_, list| {
            list.retain(|k| k.used >= since);
            kept += list.len();
            !list.is_empty()
        });
        self.kept = kept;
    }

    /// How many paragraphs are kept.
    pub fn paragraphs_kept(&self) -> usize {
        self.kept
    }

    /// How wide `text` is at `size` pixels in the game's font `font`, and
    /// how tall its lines are, a line per `\n`.
    pub fn measure(&mut self, font: usize, text: &str, size: f32) -> [f32; 2] {
        self.paragraph(text, &TextStyle::new(size, font), &Fit::default())
            .size
    }

    /// Lays `text` out at `size` pixels in the game's font `font` with its
    /// first line's top left corner at `at`, a line per `\n`: each letter's
    /// box on the screen and its place in the atlas. With `crisp`, each
    /// letter lands on whole multiples of `step` (pixel art).
    pub fn place(
        &mut self,
        font: usize,
        text: &str,
        size: f32,
        at: [f32; 2],
        crisp: Option<f32>,
    ) -> Vec<Placed> {
        let paragraph = self.paragraph(text, &TextStyle::new(size, font), &Fit::default());
        self.place_paragraph(&paragraph, Edge::Start, at, 1.0, crisp)
    }

    /// `paragraph`'s letters on the screen: its top left corner at `at`
    /// (screen pixels), `scale` screen pixels to a design pixel, each line
    /// placed along the paragraph's width by `align`. With `crisp`, each
    /// letter lands on whole multiples of `step` (pixel art).
    pub fn place_paragraph(
        &mut self,
        paragraph: &Paragraph,
        align: Edge,
        at: [f32; 2],
        scale: f32,
        crisp: Option<f32>,
    ) -> Vec<Placed> {
        let scale = if scale.is_finite() {
            scale.clamp(0.0, 1.0e4)
        } else {
            0.0
        };
        let k = paragraph.text_size * scale / SDF_SIZE;
        let mut out = Vec::with_capacity(paragraph.letters());
        for line in &paragraph.lines {
            let room = paragraph.size[0] - line.width;
            let dx = match align {
                Edge::Middle => room * 0.5,
                Edge::End => room,
                Edge::Start | Edge::Stretch => 0.0,
            };
            for g in paragraph.glyphs_of(line) {
                let Some(letter) = self.letter(usize::from(g.slot), g.id) else {
                    continue;
                };
                let [l, t, r, b] = letter.atlas.map(f32::from);
                let pen = [
                    at[0] + (dx + g.at[0]) * scale,
                    at[1] + (line.baseline + g.at[1]) * scale,
                ];
                let mut left = pen[0] + letter.offset[0] * k;
                let mut top = pen[1] + letter.offset[1] * k;
                if let Some(step) = crisp.filter(|s| *s > 0.0) {
                    left = (left / step).round() * step;
                    top = (top / step).round() * step;
                }
                let rect = [left, top, left + (r - l) * k, top + (b - t) * k];
                if rect.iter().all(|v| v.is_finite()) {
                    out.push(Placed {
                        rect,
                        atlas: letter.atlas,
                    });
                }
            }
        }
        out
    }
}

/// What a paragraph is laid out from, as one number.
fn paragraph_hash(text: &str, style: &TextStyle, fit: &Fit) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut h);
    style.size.to_bits().hash(&mut h);
    style.font.hash(&mut h);
    style.fonts.hash(&mut h);
    style.line_height.to_bits().hash(&mut h);
    style.letter_spacing.to_bits().hash(&mut h);
    fit.width.map(f32::to_bits).hash(&mut h);
    fit.wrap.hash(&mut h);
    fit.lines.hash(&mut h);
    fit.cut.hash(&mut h);
    h.finish()
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
