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
use skrifa::raw::tables::kern::{Kern, SubtableKind};
use skrifa::{FontRef, MetadataProvider as _};
use std::sync::Arc;

pub(crate) use skrifa::GlyphId;

/// A font file, and the numbers every size of it is worked out from, in
/// the font's own units.
#[derive(Debug, Clone)]
pub(crate) struct Font {
    bytes: Arc<Vec<u8>>,
    ascent: f32,
    /// Below the baseline: negative.
    descent: f32,
    line_gap: f32,
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
        Some(Self {
            bytes: Arc::new(bytes),
            ascent,
            descent,
            line_gap,
        })
    }

    /// The font at `size` pixels, ready to lay text out.
    pub(crate) fn at(&self, size: f32) -> Option<Sized<'_>> {
        let face = FontRef::from_index(&self.bytes, 0).ok()?;
        Some(Sized {
            charmap: face.charmap(),
            advances: face.glyph_metrics(Size::unscaled(), LocationRef::default()),
            kern: face.kern().ok(),
            outlines: face.outline_glyphs(),
            size,
            scale: size / (self.ascent - self.descent),
            font: self,
        })
    }
}

/// A font at one size: letters, advances and kerning in pixels.
pub(crate) struct Sized<'a> {
    charmap: skrifa::charmap::Charmap<'a>,
    advances: skrifa::metrics::GlyphMetrics<'a>,
    kern: Option<Kern<'a>>,
    outlines: skrifa::outline::OutlineGlyphCollection<'a>,
    /// From the top of the ascent to the bottom of the descent, in pixels.
    size: f32,
    /// Pixels per font unit.
    scale: f32,
    font: &'a Font,
}

impl Sized<'_> {
    /// The letter for `c`; the font's "missing" letter when it has none.
    pub(crate) fn glyph(&self, c: char) -> GlyphId {
        self.charmap.map(c).unwrap_or(GlyphId::NOTDEF)
    }

    /// How far the pen moves after `id`.
    pub(crate) fn advance(&self, id: GlyphId) -> f32 {
        self.advances.advance_width(id).unwrap_or(0.0) * self.scale
    }

    /// How much closer (negative) or further `second` goes after `first`,
    /// from the font's `kern` table (its horizontal pairs and classes).
    pub(crate) fn kern(&self, first: GlyphId, second: GlyphId) -> f32 {
        let Some(kern) = &self.kern else {
            return 0.0;
        };
        kern.subtables()
            .filter_map(Result::ok)
            .filter(|t| t.is_horizontal() && !t.is_variable())
            .find_map(|t| match t.kind().ok()? {
                SubtableKind::Format0(t) => t.kerning(first, second),
                SubtableKind::Format2(t) => t.kerning(first, second),
                SubtableKind::Format3(t) => t.kerning(first, second),
                SubtableKind::Format1(_) => None,
            })
            .map_or(0.0, |k| k as f32 * self.scale)
    }

    /// From the baseline up to the top of the line.
    pub(crate) fn ascent(&self) -> f32 {
        self.font.ascent * self.scale
    }

    /// From one line's baseline to the next's.
    pub(crate) fn line(&self) -> f32 {
        self.size + self.font.line_gap * self.scale
    }

    /// Letter `id`'s outline, ready to draw; none for one with no outline
    /// (a space) or one the font can't give.
    pub(crate) fn outline(&self, id: GlyphId) -> Option<Outline> {
        let glyph = self.outlines.get(id)?;
        let mut pen = Pen::default();
        glyph
            .draw(
                skrifa::outline::DrawSettings::unhinted(Size::unscaled(), LocationRef::default()),
                &mut pen,
            )
            .ok()?;
        pen.close();
        Outline::new(pen.curves, self.scale)
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
