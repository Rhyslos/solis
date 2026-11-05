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
    let screen_px = [size[0] as f32, size[1] as f32];
    let scale = scale_of(screen.scale, screen.design, screen_px, pixel_step);
    let crisp = screen.scale == ScaleRule::WholeSteps;
    let mut out = Vec::new();
    let mut count = 0;
    let whole = [0.0, 0.0, screen_px[0], screen_px[1]];
    for e in &screen.elements {
        walk(e, whole, scale, 1.0, crisp, 0, &mut count, &mut out);
    }
    out
}

/// [`layout`] in part of a screen: the game's picture, `size` pixels big
/// with its top left corner at `origin` (a 2D game's picture with bars
/// round it), every shape moved there.
pub fn layout_at(
    screen: &Screen,
    origin: [u32; 2],
    size: [u32; 2],
    pixel_step: Option<u32>,
) -> Vec<Shape> {
    let [x, y] = origin.map(|v| v as f32);
    let mut shapes = layout(screen, size, pixel_step);
    for s in &mut shapes {
        s.rect = [s.rect[0] + x, s.rect[1] + y, s.rect[2] + x, s.rect[3] + y];
        s.from = [s.from[0] + x, s.from[1] + y];
        s.to = [s.to[0] + x, s.to[1] + y];
    }
    shapes
}

#[allow(clippy::too_many_arguments)]
fn walk(
    e: &Element,
    parent: [f32; 4],
    scale: f32,
    opacity: f32,
    crisp: bool,
    depth: usize,
    count: &mut usize,
    out: &mut Vec<Shape>,
) {
    if !e.shown || depth >= MAX_DEPTH || *count >= MAX_ELEMENTS {
        return;
    }
    *count += 1;
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
        ElementKind::Box { round } => out.push(Shape {
            round: finite(*round).max(0.0) * scale,
            ..base
        }),
        ElementKind::Ellipse => out.push(Shape {
            kind: ShapeKind::Ellipse,
            ..base
        }),
        ElementKind::Line { width } => out.push(Shape {
            kind: ShapeKind::Line,
            round: finite(*width).max(0.0) * scale,
            border: (0.0, Colour::NONE),
            ..base
        }),
        ElementKind::Bar {
            round,
            value,
            background,
        } => {
            let round = finite(*round).max(0.0) * scale;
            let back = faded(*background);
            out.push(Shape {
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
                out.push(Shape {
                    rect: [inner[0], inner[1], right, inner[3]],
                    round: (round - inset).max(0.0),
                    border: (0.0, Colour::NONE),
                    ..base
                });
            }
        }
    }
    for c in &e.children {
        walk(c, rect, scale, opacity, crisp, depth + 1, count, out);
    }
}

#[cfg(test)]
#[path = "layout_tests.rs"]
mod tests;
