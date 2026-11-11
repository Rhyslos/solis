//! A dashed outline round a rounded rectangle: the design's mark for
//! "add something here" (a "+ Component" button, a "+" beside a key), and
//! for a keycap that is waiting for a key. egui can dash a line but not
//! a rounded rectangle, so its outline is walked here.

use super::hairline;
use egui::{Color32, Pos2, Rect, Shape, Stroke, Ui, pos2};
use std::f32::consts::FRAC_PI_2;

/// A dash's length and the gap after it, in points, measured on the
/// design's own picture of a keycap waiting for a key.
pub const DASH: [f32; 2] = [3.0, 2.0];

/// The corners of `rect`'s outline with corners rounded by `radius`,
/// clockwise from the top left, the line's middle half a line in from the
/// edge so the whole line is inside `rect`.
pub(crate) fn outline(rect: Rect, radius: f32, line: f32) -> Vec<Pos2> {
    let rect = rect.shrink(line / 2.0);
    let radius = (radius - line / 2.0)
        .min(rect.width() / 2.0)
        .min(rect.height() / 2.0)
        .max(0.0);
    // Each corner: its circle's middle, and the angle its arc starts at.
    let corners = [
        (pos2(rect.left() + radius, rect.top() + radius), 2.0),
        (pos2(rect.right() - radius, rect.top() + radius), 3.0),
        (pos2(rect.right() - radius, rect.bottom() - radius), 0.0),
        (pos2(rect.left() + radius, rect.bottom() - radius), 1.0),
    ];
    const PIECES: usize = 6;
    let mut points = Vec::with_capacity(4 * (PIECES + 1) + 1);
    for (middle, quarter) in corners {
        for piece in 0..=PIECES {
            let angle = (quarter + piece as f32 / PIECES as f32) * FRAC_PI_2;
            points.push(middle + egui::vec2(angle.cos(), angle.sin()) * radius);
        }
    }
    points.extend(points.first().copied());
    points
}

/// Draws a 1 px dashed outline just inside `rect`.
pub fn rounded(ui: &Ui, rect: Rect, radius: u8, colour: Color32) {
    let line = hairline(ui);
    let points = outline(rect, f32::from(radius), line);
    ui.painter().extend(Shape::dashed_line(
        &points,
        Stroke::new(line, colour),
        DASH[0],
        DASH[1],
    ));
}
