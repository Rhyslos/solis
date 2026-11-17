//! The plain pictures that stand for a project's kind (boards 01 and 02):
//! a field of tiles for 2D, a block on a floor for 3D, both side by side
//! for both. A project's card shows its kind's until projects keep a
//! picture of their last view; New project's kind cards show them too.

use super::ProjectKind;
use crate::tokens::{self, picture as p};
use egui::{CornerRadius, Painter, Pos2, Rect, Shape, Stroke, pos2, vec2};

/// Draws the picture of `kind` filling `rect` (its backdrop included).
pub(crate) fn kind_picture(painter: &Painter, rect: Rect, kind: ProjectKind, radius: CornerRadius) {
    painter.rect_filled(rect, radius, tokens::viewport());
    let inner = rect.shrink2(vec2(rect.width() * 0.08, rect.height() * 0.12));
    match kind {
        ProjectKind::TwoD => field(painter, centred(inner, 1.6)),
        ProjectKind::ThreeD => {
            floor(painter, rect);
            block(painter, centred(inner, 1.0));
        }
        ProjectKind::Both => {
            let half = inner.width() / 2.0;
            let left = Rect::from_min_size(inner.min, vec2(half, inner.height()));
            let right = Rect::from_min_size(pos2(inner.left() + half, inner.top()), left.size());
            field(painter, centred(left.shrink(2.0), 1.0));
            block(painter, centred(right.shrink(2.0), 1.0));
        }
    }
}

/// The largest rect of `aspect` (width over height) centred in `room`.
fn centred(room: Rect, aspect: f32) -> Rect {
    let width = room.width().min(room.height() * aspect);
    Rect::from_center_size(room.center(), vec2(width, width / aspect))
}

/// A field of grass tiles with a row of soil along its foot.
fn field(painter: &Painter, rect: Rect) {
    let (across, down) = if rect.width() > rect.height() * 1.3 {
        (8, 5)
    } else {
        (5, 5)
    };
    let cell = vec2(rect.width() / across as f32, rect.height() / down as f32);
    for row in 0..down {
        for column in 0..across {
            let at = rect.min + vec2(cell.x * column as f32, cell.y * row as f32);
            let tile = Rect::from_min_size(at, cell);
            let fill = if row == down - 1 { p::SOIL } else { p::GRASS };
            painter.rect_filled(tile, 0.0, fill);
            painter.rect_stroke(
                tile,
                0.0,
                Stroke::new(1.0, p::GRASS_LINE),
                egui::StrokeKind::Inside,
            );
        }
    }
}

/// The floor under a block: the picture's lower third, a shade lighter.
fn floor(painter: &Painter, rect: Rect) {
    let top = rect.top() + rect.height() * 0.68;
    let ground = Rect::from_min_max(pos2(rect.left(), top), rect.max);
    painter.rect_filled(ground, 0.0, p::FLOOR);
}

/// A block seen from above one corner: its lit top and two sides.
fn block(painter: &Painter, rect: Rect) {
    let size = rect.width().min(rect.height());
    let c = rect.center();
    let half = size * 0.42;
    let rise = half * 0.55;
    let corner = |x: f32, y: f32| -> Pos2 { pos2(c.x + x, c.y + y) };
    let top = [
        corner(0.0, -rise * 2.0),
        corner(half, -rise),
        corner(0.0, 0.0),
        corner(-half, -rise),
    ];
    let left = [
        corner(-half, -rise),
        corner(0.0, 0.0),
        corner(0.0, rise * 2.0),
        corner(-half, rise),
    ];
    let right = [
        corner(0.0, 0.0),
        corner(half, -rise),
        corner(half, rise),
        corner(0.0, rise * 2.0),
    ];
    for (points, fill) in [
        (top, p::BLOCK_TOP),
        (left, p::BLOCK_LIT),
        (right, p::BLOCK_SHADE),
    ] {
        painter.add(Shape::convex_polygon(points.to_vec(), fill, Stroke::NONE));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_picture_keeps_its_shape_inside_its_room() {
        let room = Rect::from_min_size(pos2(10.0, 20.0), vec2(200.0, 50.0));
        let fitted = centred(room, 1.6);
        assert!((fitted.width() / fitted.height() - 1.6).abs() < 1e-4);
        assert!(room.contains_rect(fitted.shrink(0.01)));
        assert!((fitted.center() - room.center()).length() < 1e-3);
    }
}
