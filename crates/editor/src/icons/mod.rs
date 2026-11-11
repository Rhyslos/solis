//! The editor's icons: small line drawings kept as data and drawn by
//! Solis's own reader, so they stay sharp at any size and interface
//! scale and take whatever colour they are given. There are no image
//! files and no drawing library.
//!
//! - [`Icon`] and the table of drawings are in `paths.rs`: the project
//!   lead's own icons, as the design's screens draw them, and the ones
//!   made for what the design lacks.
//! - `reader.rs` turns a drawing's path text into lines.
//! - [`icon`] paints one, in the design's line style: square line ends,
//!   sharp corners, and a line weight that follows the size drawn (the
//!   plan's inventory, part d.7).

mod paths;
pub(crate) mod reader;

pub use paths::{Drawing, Icon, Weight};

use egui::{Color32, Painter, Pos2, Rect, Shape, Stroke, pos2};
use reader::Outline;
use std::sync::OnceLock;

/// The side of the grid every icon is drawn on.
pub const GRID: f32 = 16.0;

/// The design's line weights, in grid units: the larger an icon is
/// drawn, the lighter its lines, so a line is between one and one and a
/// half pixels wide at the sizes the interface uses (12 to 26 px), and
/// only a little more on a tile's thumbnail.
mod weight {
    /// A ribbon's 18 px icons.
    pub(super) const RIBBON: f32 = 1.3;
    /// Icons of 12 to 15 px: tools, rows, tabs.
    pub(super) const SMALL: f32 = 1.5;
    /// Drawn at 20 px and up.
    pub(super) const LARGE: f32 = 1.0;
    /// Drawn at 24 px and up: a template's or a page's icon.
    pub(super) const LARGER: f32 = 0.9;
    /// Drawn at 40 px and up: a tile's thumbnail.
    pub(super) const THUMBNAIL: f32 = 0.8;
    /// Small marks that must carry.
    pub(super) const MARK: f32 = 1.6;
    /// The smallest chevrons.
    pub(super) const CHEVRON: f32 = 1.8;
    /// How see-through a faint line is.
    pub(super) const FAINT: f32 = 0.5;
}

impl Weight {
    /// How wide a line of this weight is, in grid units, in an icon drawn
    /// `side` points across.
    pub fn width(self, side: f32) -> f32 {
        let by_size = if side >= 40.0 {
            weight::THUMBNAIL
        } else if side >= 24.0 {
            weight::LARGER
        } else if side >= 20.0 {
            weight::LARGE
        } else if side >= 17.0 {
            weight::RIBBON
        } else {
            weight::SMALL
        };
        match self {
            Self::Line => by_size,
            Self::Mark if side < 20.0 => weight::MARK,
            Self::Chevron if side < 20.0 => weight::CHEVRON,
            Self::Mark | Self::Chevron => by_size,
        }
    }
}

/// An icon's drawing, read: each part as runs of straight lines on the
/// grid.
#[derive(Debug, Default)]
struct Read {
    lines: Vec<Outline>,
    fills: Vec<Outline>,
    dashed: Vec<Outline>,
    faint: Vec<Outline>,
}

/// Reads every path of `paths`. One that doesn't read draws nothing (a
/// test reads the whole table, so none is like that).
fn read_all(paths: &[&str]) -> Vec<Outline> {
    paths
        .iter()
        .flat_map(|path| reader::read(path).unwrap_or_default())
        .collect()
}

/// As [`read_all`], for paths that are stroked: each outline is cut
/// where the line turns back on itself. A sharp corner is drawn to a
/// point, and at a turn that sharp the point would shoot far past the
/// corner; cut there, the line simply ends, as the design's pictures
/// show such corners.
fn read_lines(paths: &[&str]) -> Vec<Outline> {
    read_all(paths).into_iter().flat_map(cut_at_cusps).collect()
}

/// The cosine of the sharpest turn drawn as a corner: beyond about 150
/// degrees the corner is cut (the limit the design's own drawings were
/// made under).
const SHARPEST_CORNER: f32 = -0.875;

/// Whether the line turns back on itself at `b`, coming from `a` and
/// leaving for `c`.
fn is_cusp(a: [f32; 2], b: [f32; 2], c: [f32; 2]) -> bool {
    let (d1, d2) = ([b[0] - a[0], b[1] - a[1]], [c[0] - b[0], c[1] - b[1]]);
    let lengths = (d1[0] * d1[0] + d1[1] * d1[1]).sqrt() * (d2[0] * d2[0] + d2[1] * d2[1]).sqrt();
    lengths > f32::EPSILON && (d1[0] * d2[0] + d1[1] * d2[1]) / lengths < SHARPEST_CORNER
}

/// `outline` in pieces that end wherever it turns back on itself.
fn cut_at_cusps(outline: Outline) -> Vec<Outline> {
    let mut points = outline.points;
    let mut closed = outline.closed;
    let count = points.len();
    if closed && count > 2 {
        // Started from a cusp, a closed outline is an open one that ends
        // where it began.
        let cusp = (0..count).find(|&i| {
            is_cusp(
                points[(i + count - 1) % count],
                points[i],
                points[(i + 1) % count],
            )
        });
        if let Some(at) = cusp {
            points.rotate_left(at);
            points.push(points[0]);
            closed = false;
        }
    }
    if closed {
        return vec![Outline { points, closed }];
    }
    let mut pieces = Vec::new();
    let mut piece = vec![points[0]];
    for i in 1..points.len() {
        piece.push(points[i]);
        let turns_back = i + 1 < points.len() && is_cusp(points[i - 1], points[i], points[i + 1]);
        if turns_back {
            pieces.push(std::mem::replace(&mut piece, vec![points[i]]));
        }
    }
    pieces.push(piece);
    pieces
        .into_iter()
        .map(|points| Outline {
            points,
            closed: false,
        })
        .collect()
}

/// `icon`'s drawing, read. The whole table is read once, the first time
/// an icon is drawn: it is small (well under a millisecond's work).
fn read_of(icon: Icon) -> Option<&'static Read> {
    static TABLE: OnceLock<Vec<Read>> = OnceLock::new();
    let table = TABLE.get_or_init(|| {
        Icon::ALL
            .iter()
            .map(|icon| {
                let drawing = icon.drawing();
                Read {
                    lines: read_lines(drawing.lines),
                    fills: read_all(drawing.fills),
                    dashed: read_all(drawing.dashed),
                    faint: read_lines(drawing.faint),
                }
            })
            .collect()
    });
    // `Icon::ALL` is in the enum's own order.
    table.get(icon as usize)
}

/// Draws `icon` in the middle of `rect`, as large as fits, in `color`.
pub fn icon(painter: &Painter, rect: Rect, icon: Icon, color: Color32) {
    let side = rect.width().min(rect.height());
    if !rect.is_finite() || side <= 0.0 {
        return;
    }
    let Some(read) = read_of(icon) else {
        return;
    };
    let unit = side / GRID;
    let corner = rect.center() - egui::vec2(side, side) / 2.0;
    let placed = |outline: &Outline| -> Vec<Pos2> {
        outline
            .points
            .iter()
            .map(|p| pos2(corner.x + p[0] * unit, corner.y + p[1] * unit))
            .collect()
    };
    let drawing = icon.drawing();
    let stroke = Stroke::new(drawing.weight.width(side) * unit, color);
    let stroked = |outline: &Outline, stroke: Stroke| {
        if outline.closed {
            Shape::closed_line(placed(outline), stroke)
        } else {
            Shape::line(placed(outline), stroke)
        }
    };
    for outline in &read.fills {
        painter.add(Shape::convex_polygon(placed(outline), color, Stroke::NONE));
    }
    for outline in &read.lines {
        painter.add(stroked(outline, stroke));
    }
    let faint = Stroke::new(stroke.width, color.gamma_multiply(weight::FAINT));
    for outline in &read.faint {
        painter.add(stroked(outline, faint));
    }
    for outline in &read.dashed {
        let mut points = placed(outline);
        if outline.closed {
            points.extend(points.first().copied());
        }
        let [on, off] = drawing.dash;
        painter.extend(Shape::dashed_line(&points, stroke, on * unit, off * unit));
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
