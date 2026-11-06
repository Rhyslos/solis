//! A screen laid out on a real screen ([decision 0083](../../../docs/decisions/0083-game-ui-on-any-screen.md)):
//! each element placed in its parent by its anchor and offset, sizes
//! scaled by the screen's [`ScaleRule`], and turned into [`Shape`]s in
//! drawing order, in the real screen's pixels.

use crate::screen::{Anchor, Colour, Edge, Element, ElementKind, Fill, MAX_DEPTH, MAX_ELEMENTS};
use crate::screen::{ScaleRule, Screen};

/// What a shape is, for the shader.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeKind {
    /// A box (its corners rounded by `round`).
    Box,
    /// An ellipse filling its box.
    Ellipse,
    /// A line from the box's `from` to `to`, `width` thick.
    Line,
    /// A letter: its box drawn from the letters' atlas, the part from
    /// `from` to `to` (atlas pixels), `round` its size over
    /// [`crate::text::SDF_SIZE`].
    Letter,
}

/// Which way a fill's colours run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stretch {
    /// The first colour at the left, the second at the right.
    Across,
    /// The first at the top, the second at the bottom.
    Down,
}

/// One shape to draw, in the real screen's pixels (x right, y down).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shape {
    /// What it is.
    pub kind: ShapeKind,
    /// Its box: left, top, right, bottom. For a line, its two ends are
    /// `from` and `to`.
    pub rect: [f32; 4],
    /// A line's ends.
    pub from: [f32; 2],
    /// A line's other end.
    pub to: [f32; 2],
    /// How round a box's corners are; how thick a line is.
    pub round: f32,
    /// Its colours (both the same for one colour), with its opacity
    /// worked in.
    pub colours: [Colour; 2],
    /// Which way the colours run.
    pub stretch: Stretch,
    /// Its border's thickness (0 for none) and colour.
    pub border: (f32, Colour),
    /// Whether its edges are hard, for pixel art (no smoothing).
    pub crisp: bool,
}

/// How a screen designed at `design` is scaled onto a screen `screen`
/// pixels big. `pixel_step` is the whole number the game's picture is
/// scaled by (2D games; none in 3D), for [`ScaleRule::WholeSteps`].
pub fn scale_of(
    rule: ScaleRule,
    design: [f32; 2],
    screen: [f32; 2],
    pixel_step: Option<u32>,
) -> f32 {
    let s = match rule {
        ScaleRule::Grow if design[1] > 0.0 => screen[1] / design[1],
        ScaleRule::Grow | ScaleRule::Keep => 1.0,
        ScaleRule::WholeSteps => match pixel_step {
            Some(step) => step.max(1) as f32,
            // Without a game picture to follow: the biggest whole step the
            // screen holds (at least 1).
            None if design[0] > 0.0 && design[1] > 0.0 => (screen[0] / design[0])
                .min(screen[1] / design[1])
                .floor()
                .max(1.0),
            None => 1.0,
        },
    };
    if s.is_finite() && s > 0.0 {
        s.clamp(MIN_SCALE, MAX_SCALE)
    } else {
        1.0
    }
}

/// The smallest and biggest a screen is ever scaled.
const MIN_SCALE: f32 = 1.0 / 256.0;
const MAX_SCALE: f32 = 1024.0;
/// The biggest a design size or offset is taken to be, in design pixels.
const MAX_DESIGN: f32 = 1.0e5;

/// One side of an element's box in its parent's (from `lo` to `hi`):
/// where it starts and ends.
fn side(edge: Edge, lo: f32, hi: f32, offset: f32, size: f32) -> (f32, f32) {
    match edge {
        Edge::Start => (lo + offset, lo + offset + size),
        Edge::End => (hi - offset - size, hi - offset),
        Edge::Middle => {
            let mid = (lo + hi) * 0.5 + offset;
            (mid - size * 0.5, mid + size * 0.5)
        }
        Edge::Stretch => {
            let (a, b) = (lo + offset, hi - size);
            if b < a {
                ((a + b) * 0.5, (a + b) * 0.5)
            } else {
                (a, b)
            }
        }
    }
}

/// `v` when finite (kept within [`MAX_DESIGN`]), else 0.
fn finite(v: f32) -> f32 {
    if v.is_finite() {
        v.clamp(-MAX_DESIGN, MAX_DESIGN)
    } else {
        0.0
    }
}

/// Where `element` lies in a parent box `parent` (left, top, right,
/// bottom), every design size times `scale`.
pub fn place(element: &Element, parent: [f32; 4], scale: f32) -> [f32; 4] {
    let at = element.offset.map(|v| finite(v) * scale);
    let size = element.size.map(|v| finite(v).max(0.0) * scale);
    let Anchor { across, down } = element.anchor;
    let (l, r) = side(across, parent[0], parent[2], at[0], size[0]);
    let (t, b) = side(down, parent[1], parent[3], at[1], size[1]);
    [l, t, r, b]
}

/// Lays `screen` out on a screen `size` pixels big (`pixel_step`: as
/// [`scale_of`]), into shapes in drawing order. Hidden elements, and
/// anything past [`MAX_ELEMENTS`] or [`MAX_DEPTH`], are left out.
pub fn layout(screen: &Screen, size: [u32; 2], pixel_step: Option<u32>) -> Vec<Shape> {
    layout_with(screen, size, pixel_step, None)
}

/// [`layout`], with text: its letters from `fonts` (their distance images
/// made into its atlas the first time each is used). Without fonts, text
/// is left out.
pub fn layout_with(
    screen: &Screen,
    size: [u32; 2],
    pixel_step: Option<u32>,
    fonts: Option<&mut crate::text::Fonts>,
) -> Vec<Shape> {
    let screen_px = [size[0] as f32, size[1] as f32];
    let scale = scale_of(screen.scale, screen.design, screen_px, pixel_step);
    let crisp = screen.scale == ScaleRule::WholeSteps;
    let mut out = Vec::new();
    let mut count = 0;
    let whole = [0.0, 0.0, screen_px[0], screen_px[1]];
    let mut walking = Walk {
        scale,
        crisp,
        count: &mut count,
        out: &mut out,
        fonts,
    };
    for e in &screen.elements {
        walking.walk(e, whole, 1.0, 0);
    }
    // A view written in Lust: laid out in design pixels as wide as the
    // screen's shape, then drawn like elements.
    if let Some(view) = &screen.view {
        let design = [screen_px[0] / scale, screen_px[1] / scale];
        let (elements, _) = match walking.fonts.as_deref_mut() {
            Some(fonts) => crate::view::lay_out_with(view, design, fonts, screen.looks),
            None => {
                crate::view::lay_out_with(view, design, &mut crate::view::NoFonts, screen.looks)
            }
        };
        for e in &elements {
            walking.walk(e, whole, 1.0, 0);
        }
    }
    out
}

/// Where a 2D game's picture of `base` pixels goes on a `screen` (both in
/// pixels), and how it's scaled: what the renderer's pixel stage draws, and
/// so where game UI is laid out ([`layout_at`]) and clicked.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PictureArea {
    /// Screen pixels per picture pixel.
    pub scale: f32,
    /// Whether that's a whole number.
    pub whole: bool,
    /// The picture's top-left corner on the screen, in whole pixels.
    pub origin: [u32; 2],
    /// The picture's size on the screen, in whole pixels.
    pub size: [u32; 2],
}

/// [`PictureArea`] for a picture of `base` pixels on a `screen`:
/// `whole_steps` keeps each picture pixel a whole number of screen pixels
/// (with bars round it), else it fills as much as it can. A screen smaller
/// than the picture shows it shrunk to fit.
pub fn picture_area(base: [u32; 2], screen: [u32; 2], whole_steps: bool) -> PictureArea {
    let base = base.map(|v| v.max(1));
    let screen = screen.map(|v| v.max(1));
    let fits = (screen[0] as f32 / base[0] as f32).min(screen[1] as f32 / base[1] as f32);
    let steps = (screen[0] / base[0]).min(screen[1] / base[1]);
    let whole = steps >= 1 && (whole_steps || fits == steps as f32);
    let scale = if whole { steps as f32 } else { fits };
    let size = if whole {
        [base[0] * steps, base[1] * steps]
    } else {
        [
            ((base[0] as f32 * scale).round() as u32).clamp(1, screen[0]),
            ((base[1] as f32 * scale).round() as u32).clamp(1, screen[1]),
        ]
    };
    let origin = [(screen[0] - size[0]) / 2, (screen[1] - size[1]) / 2];
    PictureArea {
        scale,
        whole,
        origin,
        size,
    }
}

/// The clickable piece of `screen`'s view (decision 0088) at `at`, in a
/// screen `size` pixels big (`at` in its pixels, from its top left), laid
/// out as [`layout_with`] lays it out (with the same `pixel_step`).
pub fn clickable_at(
    screen: &Screen,
    size: [u32; 2],
    pixel_step: Option<u32>,
    at: [f32; 2],
    measure: &mut dyn crate::view::Measure,
) -> Option<usize> {
    let view = screen.view.as_ref()?;
    if !(at[0].is_finite() && at[1].is_finite()) {
        return None;
    }
    let screen_px = [size[0] as f32, size[1] as f32];
    let scale = scale_of(screen.scale, screen.design, screen_px, pixel_step);
    let design = [screen_px[0] / scale, screen_px[1] / scale];
    let (_, clickable) = crate::view::lay_out_with(view, design, measure, screen.looks);
    crate::view::hit(&clickable, [at[0] / scale, at[1] / scale])
}

/// What laying a screen out carries as it goes.
struct Walk<'a> {
    scale: f32,
    crisp: bool,
    count: &'a mut usize,
    out: &'a mut Vec<Shape>,
    fonts: Option<&'a mut crate::text::Fonts>,
}

/// [`layout`] in part of a screen: the game's picture, `size` pixels big
/// with its top left corner at `origin` (a 2D game's picture with bars
/// round it), every shape moved there.
pub fn layout_at(
    screen: &Screen,
    origin: [u32; 2],
    size: [u32; 2],
    pixel_step: Option<u32>,
    fonts: Option<&mut crate::text::Fonts>,
) -> Vec<Shape> {
    let [x, y] = origin.map(|v| v as f32);
    let mut shapes = layout_with(screen, size, pixel_step, fonts);
    for s in &mut shapes {
        s.rect = [s.rect[0] + x, s.rect[1] + y, s.rect[2] + x, s.rect[3] + y];
        if s.kind != ShapeKind::Letter {
            s.from = [s.from[0] + x, s.from[1] + y];
            s.to = [s.to[0] + x, s.to[1] + y];
        }
    }
    shapes
}

impl Walk<'_> {
    fn walk(&mut self, e: &Element, parent: [f32; 4], opacity: f32, depth: usize) {
        let (scale, crisp) = (self.scale, self.crisp);
        if !e.shown || depth >= MAX_DEPTH || *self.count >= MAX_ELEMENTS {
            return;
        }
        *self.count += 1;
        let mut rect = place(e, parent, scale);
        if crisp {
            // On whole screen pixels of the game's own pixels' grid.
            rect = rect.map(|v| (v / scale).round() * scale);
        }
        let opacity = opacity * finite(e.opacity).clamp(0.0, 1.0);
        let (colours, stretch) = match e.fill {
            Fill::Solid(c) => ([c, c], Stretch::Across),
            Fill::Across(a, b) => ([a, b], Stretch::Across),
            Fill::Down(a, b) => ([a, b], Stretch::Down),
        };
        let faded = |c: Colour| {
            let c = c.sanitized();
            c.with_alpha(c.0[3] * opacity)
        };
        let colours = colours.map(faded);
        let border = e.border.map_or((0.0, Colour::NONE), |(w, c)| {
            (finite(w).max(0.0) * scale, faded(c))
        });
        let base = Shape {
            kind: ShapeKind::Box,
            rect,
            from: [rect[0], rect[1]],
            to: [rect[2], rect[3]],
            round: 0.0,
            colours,
            stretch,
            border,
            crisp,
        };
        match &e.kind {
            ElementKind::Group => {}
            ElementKind::Text {
                text,
                size,
                font,
                align,
            } => {
                if let Some(fonts) = self.fonts.as_deref_mut() {
                    let px = finite(*size).max(0.0) * scale;
                    let [w, h] = fonts.measure(*font, text, px);
                    let left = match align {
                        Edge::Middle => (rect[0] + rect[2] - w) * 0.5,
                        Edge::End => rect[2] - w,
                        Edge::Start | Edge::Stretch => rect[0],
                    };
                    let top = (rect[1] + rect[3] - h) * 0.5;
                    let step = crisp.then_some(scale);
                    for letter in fonts.place(*font, text, px, [left, top], step) {
                        let [l, t, r, b] = letter.atlas.map(f32::from);
                        self.out.push(Shape {
                            kind: ShapeKind::Letter,
                            rect: letter.rect,
                            from: [l, t],
                            to: [r, b],
                            round: px / crate::text::SDF_SIZE,
                            colours: [colours[0], colours[0]],
                            border: (0.0, Colour::NONE),
                            ..base
                        });
                    }
                }
            }
            ElementKind::Box { round } => self.out.push(Shape {
                round: finite(*round).max(0.0) * scale,
                ..base
            }),
            ElementKind::Ellipse => self.out.push(Shape {
                kind: ShapeKind::Ellipse,
                ..base
            }),
            ElementKind::Line { width, rising } => self.out.push(Shape {
                kind: ShapeKind::Line,
                round: finite(*width).max(0.0) * scale,
                border: (0.0, Colour::NONE),
                from: if *rising {
                    [rect[0], rect[3]]
                } else {
                    base.from
                },
                to: if *rising { [rect[2], rect[1]] } else { base.to },
                ..base
            }),
            ElementKind::Bar {
                round,
                value,
                background,
            } => {
                let round = finite(*round).max(0.0) * scale;
                let back = faded(*background);
                self.out.push(Shape {
                    round,
                    colours: [back, back],
                    ..base
                });
                let value = finite(*value).clamp(0.0, 1.0);
                // The full part inside the border.
                let inset = base.border.0;
                let inner = [
                    rect[0] + inset,
                    rect[1] + inset,
                    rect[2] - inset,
                    rect[3] - inset,
                ];
                let mut right = inner[0] + (inner[2] - inner[0]).max(0.0) * value;
                if crisp {
                    right = (right / scale).round() * scale;
                }
                if right > inner[0] {
                    self.out.push(Shape {
                        rect: [inner[0], inner[1], right, inner[3]],
                        round: (round - inset).max(0.0),
                        border: (0.0, Colour::NONE),
                        ..base
                    });
                }
            }
        }
        for c in &e.children {
            self.walk(c, rect, opacity, depth + 1);
        }
    }
}

#[cfg(test)]
#[path = "layout_tests.rs"]
mod tests;
