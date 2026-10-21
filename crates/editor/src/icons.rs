//! The tool icons, drawn in code (no image files), so they stay sharp at any
//! size and take the colour they are given.

use crate::viewport::Tool;
use egui::{Color32, Painter, Pos2, Rect, Shape, Stroke, pos2, vec2};
use std::f32::consts::TAU;

/// Draws `tool`'s icon in `rect` with `color`.
pub fn tool_icon(painter: &Painter, rect: Rect, tool: Tool, color: Color32) {
    let side = rect.width().min(rect.height());
    let square = Rect::from_center_size(rect.center(), vec2(side, side));
    // Points given from 0 to 1 across the icon's square.
    let at = |x: f32, y: f32| pos2(square.left() + x * side, square.top() + y * side);
    let stroke = Stroke::new((side / 11.0).max(1.2), color);
    match tool {
        Tool::Select => select(painter, &at, stroke),
        Tool::Move => four_arrows(painter, &at, stroke, 0.45),
        Tool::Rotate => rotate(painter, square, stroke),
        Tool::Scale => scale(painter, &at, stroke),
        Tool::Transform => {
            painter.circle_stroke(square.center(), side * 0.3, stroke);
            four_arrows(painter, &at, stroke, 0.47);
        }
    }
}

/// A mouse pointer.
fn select(painter: &Painter, at: &dyn Fn(f32, f32) -> Pos2, stroke: Stroke) {
    let outline = vec![
        at(0.28, 0.12),
        at(0.78, 0.55),
        at(0.55, 0.58),
        at(0.68, 0.86),
        at(0.58, 0.9),
        at(0.45, 0.62),
        at(0.28, 0.78),
    ];
    painter.add(Shape::closed_line(outline, stroke));
}

/// A cross with an arrowhead on each end, reaching `reach` from the centre.
fn four_arrows(painter: &Painter, at: &dyn Fn(f32, f32) -> Pos2, stroke: Stroke, reach: f32) {
    let (lo, hi) = (0.5 - reach, 0.5 + reach);
    let head = 0.12;
    painter.line_segment([at(0.5, lo), at(0.5, hi)], stroke);
    painter.line_segment([at(lo, 0.5), at(hi, 0.5)], stroke);
    for (tip, side_a, side_b) in [
        (
            at(0.5, lo),
            at(0.5 - head, lo + head),
            at(0.5 + head, lo + head),
        ),
        (
            at(0.5, hi),
            at(0.5 - head, hi - head),
            at(0.5 + head, hi - head),
        ),
        (
            at(lo, 0.5),
            at(lo + head, 0.5 - head),
            at(lo + head, 0.5 + head),
        ),
        (
            at(hi, 0.5),
            at(hi - head, 0.5 - head),
            at(hi - head, 0.5 + head),
        ),
    ] {
        painter.add(Shape::line(vec![side_a, tip, side_b], stroke));
    }
}

/// Three quarters of a circle with an arrowhead.
fn rotate(painter: &Painter, square: Rect, stroke: Stroke) {
    let centre = square.center();
    let radius = square.width() * 0.32;
    let steps = 24;
    let start = -0.15 * TAU;
    let sweep = 0.75 * TAU;
    let arc: Vec<Pos2> = (0..=steps)
        .map(|i| {
            let angle = start + sweep * i as f32 / steps as f32;
            centre + vec2(angle.cos(), angle.sin()) * radius
        })
        .collect();
    let end = arc.last().copied().unwrap_or(centre);
    painter.add(Shape::line(arc, stroke));
    let head = square.width() * 0.13;
    painter.add(Shape::line(
        vec![end + vec2(-head, 0.0), end, end + vec2(0.0, head)],
        stroke,
    ));
}

/// Two corners and a diagonal: growing outwards.
fn scale(painter: &Painter, at: &dyn Fn(f32, f32) -> Pos2, stroke: Stroke) {
    painter.add(Shape::line(
        vec![at(0.15, 0.55), at(0.15, 0.85), at(0.45, 0.85)],
        stroke,
    ));
    painter.add(Shape::line(
        vec![at(0.55, 0.15), at(0.85, 0.15), at(0.85, 0.45)],
        stroke,
    ));
    painter.line_segment([at(0.17, 0.83), at(0.83, 0.17)], stroke);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tool_icon_draws_something() {
        let ctx = egui::Context::default();
        let output = ctx.run_ui(egui::RawInput::default(), |ui| {
            let painter = ui.ctx().layer_painter(egui::LayerId::background());
            let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(16.0, 16.0));
            for tool in Tool::ALL {
                tool_icon(&painter, rect, tool, Color32::WHITE);
            }
        });
        let shapes = output.shapes.len();
        // The font atlas upload is not needed here; egui insists it is
        // handled or cleared.
        let mut textures = output.textures_delta;
        textures.clear();
        assert!(shapes >= Tool::ALL.len());
    }
}
