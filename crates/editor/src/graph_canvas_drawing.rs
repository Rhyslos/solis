//! Drawing the graph canvas (`graph_canvas.rs`): the grid, the arrows
//! with their labels (each clickable), the boxes, and what's being
//! dragged.

use super::{
    Drag, Frame, GraphBox, GraphCanvas, GraphEvent, GraphTarget, GraphView, GraphWire, LIVE, ROUND,
    SIDE_BY_SIDE, distance_to_segment,
};
use crate::theme;
use egui::{Color32, Pos2, Rect, Sense, Stroke, Ui, Vec2, pos2, vec2};

/// Where an arrow between two boxes starts and ends: from edge to edge
/// along the line between their middles, moved aside when there's one
/// going the other way.
fn wire_ends(from: Rect, to: Rect, aside: bool) -> (Pos2, Pos2) {
    let (a, b) = (from.center(), to.center());
    let dir = (b - a).normalized();
    let shift = if aside {
        vec2(-dir.y, dir.x) * SIDE_BY_SIDE
    } else {
        Vec2::ZERO
    };
    let edge = |r: Rect, toward: Vec2| -> Pos2 {
        // Where the ray from the middle leaves the box.
        let half = r.size() / 2.0;
        let t = (half.x / toward.x.abs().max(1e-6)).min(half.y / toward.y.abs().max(1e-6));
        r.center() + toward * t
    };
    if !dir.is_finite() || dir == Vec2::ZERO {
        return (a, b);
    }
    (edge(from, dir) + shift, edge(to, -dir) + shift)
}

/// The arrow's head at `tip`, pointing along `dir`.
pub(super) fn arrow_head(painter: &egui::Painter, tip: Pos2, dir: Vec2, color: Color32) {
    let back = -dir * 9.0;
    let side = vec2(-dir.y, dir.x) * 5.0;
    painter.add(egui::Shape::convex_polygon(
        vec![tip, tip + back + side, tip + back - side],
        color,
        Stroke::NONE,
    ));
}

/// A quiet grid, so panning shows.
pub(super) fn draw_grid(painter: &egui::Painter, full: Rect, view: GraphView) {
    let step = 40.0 * view.zoom;
    if step <= 8.0 {
        return;
    }
    let start = full.min
        + vec2(
            view.offset.x.rem_euclid(step),
            view.offset.y.rem_euclid(step),
        );
    let grid = crate::tokens::row_hover();
    let mut x = start.x;
    while x < full.right() {
        painter.vline(x, full.y_range(), Stroke::new(1.0, grid));
        x += step;
    }
    let mut y = start.y;
    while y < full.bottom() {
        painter.hline(full.x_range(), y, Stroke::new(1.0, grid));
        y += step;
    }
}

/// Draws the arrows, each clickable by its label, and gives back the one
/// under the pointer (or right-clicked), if any.
pub(super) fn draw_wires(
    ui: &Ui,
    painter: &egui::Painter,
    frame: &Frame,
    boxes: &[GraphBox],
    wires: &[GraphWire],
    events: &mut Vec<GraphEvent>,
) -> Option<u64> {
    let find = |id: u64| boxes.iter().find(|b| b.id == id).map(|b| frame.rect_of(b));
    let mut wire_hit: Option<u64> = None;
    let font = egui::FontId::proportional(11.0 * frame.view.zoom.clamp(0.7, 1.4));
    for w in wires {
        let (Some(a), Some(b)) = (find(w.from), find(w.to)) else {
            continue;
        };
        let back = wires.iter().any(|o| o.from == w.to && o.to == w.from);
        let (start, end) = wire_ends(a, b, back);
        let color = if w.picked {
            frame.accent
        } else {
            Color32::from_rgb(0x9a, 0x9a, 0x9a)
        };
        painter.line_segment(
            [start, end],
            Stroke::new(if w.picked { 2.5 } else { 1.5 }, color),
        );
        let dir = (end - start).normalized();
        if dir.is_finite() {
            arrow_head(painter, end, dir, color);
        }
        let middle = start + (end - start) / 2.0;
        if !w.label.is_empty() {
            let (offset, align) = label_side(dir, back);
            painter.text(
                middle + offset,
                align,
                &w.label,
                font.clone(),
                if w.picked {
                    frame.accent
                } else {
                    theme::text_weak()
                },
            );
        }
        if let Some(p) = frame.pointer
            && frame.hovered
            && distance_to_segment(p, start, end) < 6.0
        {
            wire_hit = Some(w.id);
        }
        // Each arrow can be found and clicked by its label (screen readers,
        // tests).
        let hit = Rect::from_center_size(middle, vec2(18.0, 18.0));
        let r = ui.interact(hit, ui.id().with(("wire", w.id)), Sense::click());
        let label = format!("Transition {}", w.label);
        r.widget_info(|| {
            egui::WidgetInfo::selected(egui::WidgetType::Button, true, w.picked, &label)
        });
        if r.clicked() {
            events.push(GraphEvent::Pick(GraphTarget::Wire(w.id), frame.add));
        }
        if r.secondary_clicked() {
            wire_hit = Some(w.id);
        }
    }
    wire_hit
}

/// Where an arrow's label sits from its middle, and how it's aligned
/// there: on the arrow's own side, so two arrows each way keep their
/// labels apart (the label sits beyond the line, away from the other
/// arrow). `dir` is the arrow's direction; `back` whether one goes the
/// other way.
fn label_side(dir: Vec2, back: bool) -> (Vec2, egui::Align2) {
    let perp = if dir.is_finite() {
        vec2(-dir.y, dir.x)
    } else {
        vec2(0.0, -1.0)
    };
    let side = if back {
        perp
    } else if perp.y.abs() >= perp.x.abs() {
        // Lying down: above it.
        perp * -perp.y.signum()
    } else {
        // Standing up: to its right.
        perp * perp.x.signum()
    };
    let align = if side.y > 0.3 {
        egui::Align2::CENTER_TOP
    } else if side.y < -0.3 {
        egui::Align2::CENTER_BOTTOM
    } else if side.x > 0.0 {
        egui::Align2::LEFT_CENTER
    } else {
        egui::Align2::RIGHT_CENTER
    };
    (side * 5.0, align)
}

/// A box's name and quieter line: names in the heading font, where the
/// editor's fonts are loaded.
pub(super) fn box_fonts(ui: &Ui, zoom: f32) -> (egui::FontId, egui::FontId) {
    let semibold = egui::FontFamily::Name(theme::SEMIBOLD.into());
    let family = if ui.fonts(|f| f.families().contains(&semibold)) {
        semibold
    } else {
        egui::FontFamily::Proportional
    };
    (
        egui::FontId::new(13.0 * zoom, family),
        egui::FontId::proportional(11.0 * zoom),
    )
}

/// Draws box `b` at `rect`: its glow when live, its border, name, line
/// and chip, and "Start" before the box the graph starts at.
pub(super) fn draw_box(
    painter: &egui::Painter,
    frame: &Frame,
    b: &GraphBox,
    rect: Rect,
    (title_font, small): &(egui::FontId, egui::FontId),
) {
    let zoom = frame.view.zoom;
    if b.live {
        for (grow, alpha) in [(6.0, 40), (3.0, 90)] {
            painter.rect_stroke(
                rect.expand(grow),
                ROUND + grow,
                Stroke::new(3.0, LIVE.gamma_multiply(f32::from(alpha as u8) / 255.0)),
                egui::StrokeKind::Outside,
            );
        }
    }
    painter.rect_filled(rect, ROUND, crate::tokens::field_border());
    let border = if b.live {
        Stroke::new(2.0, LIVE)
    } else if b.picked {
        Stroke::new(2.0, frame.accent)
    } else {
        Stroke::new(1.0, Color32::from_rgb(0x4a, 0x4a, 0x4a))
    };
    painter.rect_stroke(rect, ROUND, border, egui::StrokeKind::Inside);
    let pad = 10.0 * zoom;
    let clip = painter.with_clip_rect(rect.shrink(2.0).intersect(frame.full));
    clip.text(
        rect.min + vec2(pad, 7.0 * zoom),
        egui::Align2::LEFT_TOP,
        &b.title,
        title_font.clone(),
        theme::text_strong(),
    );
    clip.text(
        rect.min + vec2(pad, 28.0 * zoom),
        egui::Align2::LEFT_TOP,
        &b.line,
        small.clone(),
        theme::text_weak(),
    );
    if let Some(chip) = &b.chip {
        let galley = clip.layout_no_wrap(chip.clone(), small.clone(), theme::WARNING);
        let at = rect.min + vec2(pad, rect.height() - 22.0 * zoom);
        let chip_rect = Rect::from_min_size(at, galley.size() + vec2(12.0, 2.0) * zoom);
        clip.rect_filled(chip_rect, 8.0 * zoom, Color32::from_rgb(0x3a, 0x30, 0x20));
        clip.galley(at + vec2(6.0, 1.0) * zoom, galley, theme::WARNING);
    }
    if b.start {
        let at = pos2(rect.left() - 12.0, rect.center().y);
        painter.text(
            at - vec2(14.0, 0.0),
            egui::Align2::RIGHT_CENTER,
            "Start",
            small.clone(),
            theme::COLLIDER,
        );
        arrow_head(
            painter,
            at + vec2(8.0, 0.0),
            vec2(1.0, 0.0),
            theme::COLLIDER,
        );
        painter.line_segment(
            [at - vec2(10.0, 0.0), at],
            Stroke::new(1.5, theme::COLLIDER),
        );
    }
}

/// What's being dragged, drawn: an arrow being drawn to the pointer, or
/// a picking box.
pub(super) fn draw_drag(
    painter: &egui::Painter,
    frame: &Frame,
    canvas: &GraphCanvas,
    boxes: &[GraphBox],
) {
    let Some(p) = frame.pointer else {
        return;
    };
    let accent = frame.accent;
    // An arrow being drawn.
    if let Drag::Joining(from) | Drag::Aiming(from) = &canvas.drag
        && let Some(a) = boxes
            .iter()
            .find(|b| b.id == *from)
            .map(|b| frame.rect_of(b))
    {
        let start = a.center();
        painter.line_segment([start, p], Stroke::new(2.0, accent));
        let dir = (p - start).normalized();
        if dir.is_finite() {
            arrow_head(painter, p, dir, accent);
        }
    }
    // A picking box.
    if let Drag::Picking(from, _) = &canvas.drag {
        let r = Rect::from_two_pos(*from, p);
        painter.rect_filled(r, 0.0, accent.gamma_multiply(0.12));
        painter.rect_stroke(r, 0.0, Stroke::new(1.0, accent), egui::StrokeKind::Inside);
    }
}
