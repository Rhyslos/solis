//! Drawing trigger zones over a view (the approved mock-up): a teal
//! outline, solid with a faint fill and handles when the zone is picked,
//! dashed when it isn't, with a small label of its name and size above
//! it. In a 2D view a zone is its footprint on the ground; in a 3D view a
//! box's edges or a sphere's rings. A freeform zone's points being drawn
//! or edited are drawn instead of its outline: each point, its edges and,
//! while drawing, a faint line to the pointer.

use super::points::{PointsEdit, Stage};
use super::{ViewZone, ZoneTool, footprint, label, lines_3d, sizing};
use crate::camera::Camera;
use crate::theme;
use egui::{Color32, Pos2, Rect, Shape, Stroke, Ui};
use glam::{Vec2, Vec3};
use level::builtin::ZoneForm;

/// The outline's width, in points.
const LINE: f32 = 1.5;
/// A dash and the gap after it, in points.
const DASH: (f32, f32) = (5.0, 4.0);
/// A handle's side, in points.
const HANDLE: f32 = 7.0;
/// A point's radius while drawing or editing, in points.
const POINT: f32 = 4.0;

/// Draws every zone over the view at `rect` (`flat`: a 2D view).
pub(crate) fn show(
    ui: &Ui,
    rect: Rect,
    camera: &Camera,
    zones: &[ViewZone],
    tool: &ZoneTool,
    flat: bool,
) {
    let painter = ui.painter_at(rect);
    let size = Vec2::new(rect.width(), rect.height());
    let at = |p: Vec2| rect.min + egui::vec2(p.x, p.y);
    let project = |p: Vec3| camera.project(p, size).map(at);
    for zone in zones {
        let editing = tool.points.as_ref().filter(|e| e.entity == zone.entity);
        if let Some(edit) = editing {
            draw_points(&painter, edit, &project);
            continue;
        }
        let picked = tool.picked.contains(&zone.entity);
        let screen = outline_on_screen(zone, flat, &project);
        if screen.is_empty() {
            continue;
        }
        let colour = theme::TRIGGER_ZONE;
        let stroke = Stroke::new(LINE, colour);
        for line in &screen {
            if picked {
                painter.add(Shape::line(line.clone(), stroke));
            } else {
                painter.extend(Shape::dashed_line(line, stroke, DASH.0, DASH.1));
            }
        }
        if picked && flat && zone.zone.form != ZoneForm::Freeform {
            // A box and a circle are convex: a plain fill draws them.
            painter.add(Shape::convex_polygon(
                screen[0].clone(),
                colour.gamma_multiply(0.12),
                Stroke::NONE,
            ));
        }
        draw_label(&painter, &screen, &label(&zone.name, &zone.zone, flat));
        if tool.alone == Some(zone.entity) {
            draw_handles(&painter, zone, tool, flat, &project);
        }
    }
}

/// A zone's outline on the screen, as lines of points: in a 2D view one
/// closed ring (its footprint); in a 3D view a line for each edge.
fn outline_on_screen(
    zone: &ViewZone,
    flat: bool,
    project: &impl Fn(Vec3) -> Option<Pos2>,
) -> Vec<Vec<Pos2>> {
    let world = zone.transform;
    if flat {
        let ring: Option<Vec<Pos2>> = footprint(&zone.zone)
            .iter()
            .map(|p| project(world.transform_point3(p.extend(0.0))))
            .collect();
        return match ring {
            Some(mut ring) if ring.len() >= 3 => {
                ring.push(ring[0]);
                vec![ring]
            }
            _ => Vec::new(),
        };
    }
    lines_3d(&zone.zone)
        .into_iter()
        .filter_map(|(a, b)| {
            Some(vec![
                project(world.transform_point3(a))?,
                project(world.transform_point3(b))?,
            ])
        })
        .collect()
}

/// The label above the outline's top, in the middle of it: the zone's
/// name and size on a small dark chip.
fn draw_label(painter: &egui::Painter, screen: &[Vec<Pos2>], words: &str) {
    let points = screen.iter().flatten();
    let (mut left, mut right, mut top) = (f32::MAX, f32::MIN, f32::MAX);
    for p in points {
        left = left.min(p.x);
        right = right.max(p.x);
        top = top.min(p.y);
    }
    if !(left.is_finite() && right.is_finite() && top.is_finite()) {
        return;
    }
    let galley = painter.layout_no_wrap(
        words.to_owned(),
        egui::FontId::proportional(11.5),
        theme::text_strong(),
    );
    let pad = egui::vec2(7.0, 3.0);
    let chip = Rect::from_center_size(
        Pos2::new(
            (left + right) * 0.5,
            top - 8.0 - galley.size().y * 0.5 - pad.y,
        ),
        galley.size() + pad * 2.0,
    );
    painter.rect(
        chip,
        4.0,
        theme::header(),
        Stroke::new(1.0, theme::button_border()),
        egui::StrokeKind::Inside,
    );
    painter.galley(chip.min + pad, galley, theme::text_strong());
}

/// The picked zone's handles: small squares, lit while pointed at or
/// dragged.
fn draw_handles(
    painter: &egui::Painter,
    zone: &ViewZone,
    tool: &ZoneTool,
    flat: bool,
    project: &impl Fn(Vec3) -> Option<Pos2>,
) {
    let lit = tool.drag.as_ref().map(|d| d.number).or(tool
        .hovered
        .filter(|(e, _)| *e == zone.entity)
        .map(|(_, n)| n));
    for (i, handle) in sizing::handles(&zone.zone, flat).iter().enumerate() {
        let Some(at) = project(zone.transform.transform_point3(handle.at)) else {
            continue;
        };
        let square = Rect::from_center_size(at, egui::vec2(HANDLE, HANDLE));
        let fill = if lit == Some(i) {
            theme::text_strong()
        } else {
            theme::TRIGGER_ZONE
        };
        painter.rect(
            square,
            1.0,
            fill,
            Stroke::new(1.0, theme::header()),
            egui::StrokeKind::Outside,
        );
    }
}

/// A freeform zone's points while drawn or edited: its edges, each point,
/// the picked one lit, and while drawing a faint line to the pointer (and
/// a fainter one from there back to the first point).
fn draw_points(
    painter: &egui::Painter,
    edit: &PointsEdit,
    project: &impl Fn(Vec3) -> Option<Pos2>,
) {
    let colour = theme::TRIGGER_ZONE;
    let on_screen = |p: Vec2| project(edit.world.transform_point3(p.extend(0.0)));
    let screen: Vec<Option<Pos2>> = edit.points.iter().map(|p| on_screen(*p)).collect();
    let n = screen.len();
    let closed = edit.stage == Stage::Editing;
    let edges = if closed { n } else { n.saturating_sub(1) };
    for i in 0..edges {
        if let (Some(a), Some(b)) = (screen[i], screen[(i + 1) % n]) {
            painter.line_segment([a, b], Stroke::new(LINE, colour));
        }
    }
    if edit.stage == Stage::Drawing
        && let (Some(last), Some(pointer)) = (
            screen.last().copied().flatten(),
            edit.pointer.and_then(on_screen),
        )
    {
        painter.line_segment(
            [last, pointer],
            Stroke::new(1.0, colour.gamma_multiply(0.5)),
        );
        if let Some(first) = screen.first().copied().flatten() {
            painter.line_segment(
                [pointer, first],
                Stroke::new(1.0, colour.gamma_multiply(0.2)),
            );
        }
    }
    for (i, p) in screen.iter().enumerate() {
        let Some(p) = *p else { continue };
        let picked = edit.picked == Some(i);
        let fill: Color32 = if picked { theme::text_strong() } else { colour };
        let radius = if i == 0 && !closed {
            POINT + 1.5
        } else {
            POINT
        };
        painter.circle(p, radius, fill, Stroke::new(1.0, theme::header()));
    }
}
