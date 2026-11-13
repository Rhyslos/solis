//! One font file, read with `skrifa` (decision 0107): its letters, their
//! advances, its kerning and the letters' outlines, drawn into coverage by
//! `ab_glyph_rasterizer`. The rest of `ui` sees only this module's types.
//!
//! Sizes are in pixels from the top of the font's ascent to the bottom of
//! its descent, as decision 0084's first font reader had them, so text
//! keeps its size.

use ab_glyph_rasterizer::{Point, Rasterizer, point};
use skrifa::instance::{LocationRef, Size};
use skrifa::outline::OutlinePen as _;
use skrifa::raw::TableProvider as _;
use skrifa::raw::tables::kern::SubtableKind;
use skrifa::{FontRef, MetadataProvider as _};
use std::collections::HashMap;
use std::sync::Arc;

pub(crate) use skrifa::GlyphId;

/// The characters read when the font is: every one below this (Latin,
/// Greek, Cyrillic, Hebrew, Arabic and their punctuation), looked up by
/// number, the quickest way.
const COMMON: u32 = 0x800;

/// The most characters, or pairs of letters, a font remembers; past it
/// it starts again (text of every letter there is can't grow it forever).
pub(crate) const MAX_REMEMBERED: usize = 1 << 16;

/// A font file, and the numbers every size of it is worked out from, in
/// the font's own units.
///
/// What measuring needs (each character's letter and advance, each
/// pair's kerning) is read from the file the first time it's asked for
/// and remembered: reading the file's tables again for every word made a
/// screen full of text cost thousands of reads a frame (the second
/// review pass).
#[derive(Debug, Clone)]
pub(crate) struct Font {
    bytes: Arc<Vec<u8>>,
    ascent: f32,
    /// Below the baseline: negative.
    descent: f32,
    line_gap: f32,
    /// Whether it has a `kern` table at all (many fonts kern elsewhere).
    kerns: bool,
    /// The letter and advance of each character below [`COMMON`].
    common: Vec<(GlyphId, f32)>,
    /// Each other character's letter and how far the pen moves after it.
    letters: HashMap<char, (GlyphId, f32)>,
    /// Each pair of letters' kerning.
    pairs: HashMap<(GlyphId, GlyphId), f32>,
}

impl Font {
    /// The file's first font, when it is one with a height.
    pub(crate) fn read(bytes: Vec<u8>) -> Option<Self> {
        let face = FontRef::from_index(&bytes, 0).ok()?;
        let metrics = face.metrics(Size::unscaled(), LocationRef::default());
        let (ascent, descent, line_gap) = (metrics.ascent, metrics.descent, metrics.leading);
        let height = ascent - descent;
        if !(height.is_finite() && height > 0.0 && line_gap.is_finite()) {
            return None;
        }
        let kerns = face.kern().is_ok();
        let charmap = face.charmap();
        let advances = face.glyph_metrics(Size::unscaled(), LocationRef::default());
        let common = (0..COMMON)
            .map(|c| {
                let id = char::from_u32(c)
                    .and_then(|c| charmap.map(c))
                    .unwrap_or(GlyphId::NOTDEF);
                (id, advances.advance_width(id).unwrap_or(0.0))
            })
            .collect();
        Some(Self {
            bytes: Arc::new(bytes),
            ascent,
            descent,
            line_gap,
            kerns,
            common,
            letters: HashMap::new(),
            pairs: HashMap::new(),
        })
    }

    /// The font at `size` pixels: its numbers for laying text out.
    pub(crate) fn at(&self, size: f32) -> Sized {
        let scale = size / (self.ascent - self.descent);
        Sized {
            size,
            scale,
            ascent: self.ascent * scale,
            line_gap: self.line_gap * scale,
        }
    }

    /// The letter for `c` (the font's "missing" letter when it has none),
    /// and how far the pen moves after it, in font units.
    pub(crate) fn letter(&mut self, c: char) -> (GlyphId, f32) {
        if let Some(&found) = self.common.get(c as usize) {
            return found;
        }
        if let Some(&found) = self.letters.get(&c) {
            return found;
        }
        let found = FontRef::from_index(&self.bytes, 0)
            .map(|face| {
                let id = face.charmap().map(c).unwrap_or(GlyphId::NOTDEF);
                let advance = face
                    .glyph_metrics(Size::unscaled(), LocationRef::default())
                    .advance_width(id)
                    .unwrap_or(0.0);
                (id, advance)
            })
            .unwrap_or((GlyphId::NOTDEF, 0.0));
        if self.letters.len() >= MAX_REMEMBERED {
            self.letters.clear();
        }
        self.letters.insert(c, found);
        found
    }

    /// How much closer (negative) or further `second` goes after `first`,
    /// in font units, from the font's `kern` table (its horizontal pairs
    /// and classes).
    pub(crate) fn kern(&mut self, first: GlyphId, second: GlyphId) -> f32 {
        if !self.kerns {
            return 0.0;
        }
        if let Some(&found) = self.pairs.get(&(first, second)) {
            return found;
        }
        let found = FontRef::from_index(&self.bytes, 0)
            .ok()
            .and_then(|face| face.kern().ok())
            .and_then(|kern| {
                kern.subtables()
                    .filter_map(Result::ok)
                    .filter(|t| t.is_horizontal() && !t.is_variable())
                    .find_map(|t| match t.kind().ok()? {
                        SubtableKind::Format0(t) => t.kerning(first, second),
                        SubtableKind::Format2(t) => t.kerning(first, second),
                        SubtableKind::Format3(t) => t.kerning(first, second),
                        SubtableKind::Format1(_) => None,
                    })
            })
            .map_or(0.0, |k| k as f32);
        if self.pairs.len() >= MAX_REMEMBERED {
            self.pairs.clear();
        }
        self.pairs.insert((first, second), found);
        found
    }

    /// How many characters and pairs it remembers.
    #[cfg(test)]
    pub(crate) fn remembered(&self) -> [usize; 2] {
        [self.letters.len(), self.pairs.len()]
    }

    /// Letter `id`'s outline at `size` pixels, ready to draw; none for one
    /// with no outline (a space) or one the font can't give.
    pub(crate) fn outline(&self, id: GlyphId, size: f32) -> Option<Outline> {
        let face = FontRef::from_index(&self.bytes, 0).ok()?;
        let glyph = face.outline_glyphs().get(id)?;
        let mut pen = Pen::default();
        glyph
            .draw(
                skrifa::outline::DrawSettings::unhinted(Size::unscaled(), LocationRef::default()),
                &mut pen,
            )
            .ok()?;
        pen.close();
        Outline::new(pen.curves, self.at(size).scale)
    }
}

/// A font at one size: its numbers in pixels.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Sized {
    /// From the top of the ascent to the bottom of the descent, in pixels.
    size: f32,
    /// Pixels per font unit.
    scale: f32,
    ascent: f32,
    line_gap: f32,
}

impl Sized {
    /// `units` of the font, in pixels.
    pub(crate) fn px(&self, units: f32) -> f32 {
        units * self.scale
    }

    /// From the baseline up to the top of the line.
    pub(crate) fn ascent(&self) -> f32 {
        self.ascent
    }

    /// From one line's baseline to the next's.
    pub(crate) fn line(&self) -> f32 {
        self.size + self.line_gap
    }
}

/// One piece of an outline, in font units (y up).
#[derive(Debug, Clone, Copy)]
enum Curve {
    Line([f32; 2], [f32; 2]),
    Quad([f32; 2], [f32; 2], [f32; 2]),
    Cubic([f32; 2], [f32; 2], [f32; 2], [f32; 2]),
}

/// Collects an outline's pieces, closing each shape back to where it
/// started (the coverage is only right for closed shapes).
#[derive(Default)]
struct Pen {
    curves: Vec<Curve>,
    start: [f32; 2],
    at: [f32; 2],
}

impl skrifa::outline::OutlinePen for Pen {
    fn move_to(&mut self, x: f32, y: f32) {
        self.close();
        self.start = [x, y];
        self.at = [x, y];
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.curves.push(Curve::Line(self.at, [x, y]));
        self.at = [x, y];
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        self.curves.push(Curve::Quad(self.at, [cx0, cy0], [x, y]));
        self.at = [x, y];
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.curves
            .push(Curve::Cubic(self.at, [cx0, cy0], [cx1, cy1], [x, y]));
        self.at = [x, y];
    }

    fn close(&mut self) {
        if self.at != self.start {
            self.curves.push(Curve::Line(self.at, self.start));
            self.at = self.start;
        }
    }
}

/// A letter's outline at one size, and the whole pixels it covers.
pub(crate) struct Outline {
    curves: Vec<Curve>,
    scale: f32,
    /// Left, top, right, bottom in pixels from the pen (y down), out to
    /// whole pixels.
    bounds: [f32; 4],
}

impl Outline {
    fn new(curves: Vec<Curve>, scale: f32) -> Option<Self> {
        let points = curves.iter().flat_map(|c| match *c {
            Curve::Line(a, b) => vec![a, b],
            Curve::Quad(a, b, c) => vec![a, b, c],
            Curve::Cubic(a, b, c, d) => vec![a, b, c, d],
        });
        let (mut min, mut max) = ([f32::MAX; 2], [f32::MIN; 2]);
        for [x, y] in points {
            min = [min[0].min(x), min[1].min(y)];
            max = [max[0].max(x), max[1].max(y)];
        }
        let finite = min.iter().chain(&max).all(|v| v.is_finite());
        if !finite || min[0] >= max[0] || min[1] >= max[1] {
            return None;
        }
        Some(Self {
            curves,
            scale,
            bounds: [
                (min[0] * scale).floor(),
                (-max[1] * scale).floor(),
                (max[0] * scale).ceil(),
                (-min[1] * scale).ceil(),
            ],
        })
    }

    /// Left, top, right, bottom of the pixels it covers, from the pen.
    pub(crate) fn bounds(&self) -> [f32; 4] {
        self.bounds
    }

    /// Calls `cover` with each pixel inside [`Self::bounds`] (from its top
    /// left) and how much of it the letter covers, 0 to 1.
    pub(crate) fn draw(&self, cover: impl FnMut(u32, u32, f32)) {
        let [left, top, right, bottom] = self.bounds;
        let to_pixels =
            |[x, y]: [f32; 2]| -> Point { point(x * self.scale - left, -y * self.scale - top) };
        let mut raster = Rasterizer::new((right - left) as usize, (bottom - top) as usize);
        for curve in &self.curves {
            match *curve {
                Curve::Line(a, b) => raster.draw_line(to_pixels(a), to_pixels(b)),
                Curve::Quad(a, b, c) => raster.draw_quad(to_pixels(a), to_pixels(b), to_pixels(c)),
                Curve::Cubic(a, b, c, d) => {
                    raster.draw_cubic(to_pixels(a), to_pixels(b), to_pixels(c), to_pixels(d));
                }
            }
        }
        raster.for_each_pixel_2d(cover);
    }
}
