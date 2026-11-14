//! What is drawn over the view while a brush is drawn out (the approved
//! mock-up `02-brush-draw.png`): the footprint on the grid with its
//! corners, the shape's ghost (its edges in the accent, its faces seen
//! through), each size in metres beside its edge, the height's arrow,
//! the steps at the top of the view and the keys at its foot.
//!
//! Only the brush's own few lines and faces are drawn, by the painter,
//! each frame: never through the area's brushes, which stay as they are.

use super::draw::{Draw, Plane, Step, footprint_words};
use crate::camera::Camera;
use crate::icons::{self, Icon};
use crate::tokens::{self, size, text};
use crate::widgets;
use egui::{Align2, Color32, CornerRadius, Pos2, Rect, Stroke, Ui, pos2, vec2};
use glam::{Mat4, Vec2, Vec3};

/// Draws `draw` over the view at `view`, seen by `camera`. Returns where
/// its steps and keys cover the view (presses there aren't the view's).
pub(crate) fn show(ui: &mut Ui, view: Rect, camera: &Camera, draw: &Draw, grid: f32) -> Vec<Rect> {
    let size = Vec2::new(view.width(), view.height());
    let project = camera.projector(size);
    let on_screen = |p: Vec3| project(p).map(|s| view.min + vec2(s.x, s.y));
    let painter = ui.painter_at(view);
    let accent = crate::interface_look::accent_fill();
    match draw.step {
        Step::Waiting { .. } => {}
        Step::Footprint {
            plane, from, to, ..
        } => {
            if matches!(plane, Plane::View(_)) {
                ghost(&painter, draw, grid, &on_screen, accent);
            }
            footprint(&painter, plane, (from, to), &on_screen, accent);
            sizes(ui, plane, (from, to), &on_screen, accent);
        }
        Step::Height {
            plane,
            from,
            to,
            height,
            ..
        } => {
            ghost(&painter, draw, grid, &on_screen, accent);
            footprint(&painter, plane, (from, to), &on_screen, accent);
            sizes(ui, plane, (from, to), &on_screen, accent);
            let foot = plane.world((from + to) / 2.0);
            arrow(ui, (foot, foot + Vec3::Z * height), &on_screen, height);
        }
    }
    vec![steps(ui, view, draw), keys(ui, view, draw)]
}

/// The shape as it would be made now: its faces seen through, its edges
/// in the accent.
fn ghost(
    painter: &egui::Painter,
    draw: &Draw,
    grid: f32,
    on_screen: &impl Fn(Vec3) -> Option<Pos2>,
    accent: Color32,
) {
    let Some(planned) = draw.planned_now(grid) else {
        return;
    };
    let shape = planned.shape.build(&planned.settings);
    let place = Mat4::from_rotation_translation(planned.rotation, planned.position);
    let (points, triangles) = mesh::solid_triangles(&shape);
    let seen: Vec<Option<Pos2>> = points
        .iter()
        .map(|p| on_screen(place.transform_point3(*p)))
        .collect();
    let fill = accent.gamma_multiply(tokens::share::SELECTED);
    let mut faces = egui::Mesh::default();
    for [a, b, c] in triangles {
        let corners = [a, b, c].map(|i| seen.get(i as usize).copied().flatten());
        if let [Some(a), Some(b), Some(c)] = corners {
            let first = faces.vertices.len() as u32;
            for p in [a, b, c] {
                faces.colored_vertex(p, fill);
            }
            faces.add_triangle(first, first + 1, first + 2);
        }
    }
    painter.add(egui::Shape::mesh(faces));
    let stroke = Stroke::new(size::LINE * 1.5, accent);
    for edge in shape.edges() {
        let (a, b) = edge.ends();
        let ends = [a, b].map(|v| {
            shape
                .position(v)
                .and_then(|p| on_screen(place.transform_point3(p)))
        });
        if let [Some(a), Some(b)] = ends {
            painter.line_segment([a, b], stroke);
        }
    }
}

/// The footprint's corners on screen, in order round it.
fn corners(
    plane: Plane,
    (from, to): (Vec2, Vec2),
    on_screen: &impl Fn(Vec3) -> Option<Pos2>,
) -> Option<[Pos2; 4]> {
    let at = |x: f32, y: f32| on_screen(plane.world(Vec2::new(x, y)));
    Some([
        at(from.x, from.y)?,
        at(to.x, from.y)?,
        at(to.x, to.y)?,
        at(from.x, to.y)?,
    ])
}

/// The footprint: filled faintly, outlined, a square at each corner.
fn footprint(
    painter: &egui::Painter,
    plane: Plane,
    sides: (Vec2, Vec2),
    on_screen: &impl Fn(Vec3) -> Option<Pos2>,
    accent: Color32,
) {
    let Some(corners) = corners(plane, sides, on_screen) else {
        return;
    };
    painter.add(egui::Shape::convex_polygon(
        corners.to_vec(),
        accent.gamma_multiply(tokens::share::SELECTED),
        Stroke::new(size::LINE * 1.5, accent),
    ));
    for corner in corners {
        painter.rect_filled(
            Rect::from_center_size(corner, vec2(size::BRUSH_CORNER, size::BRUSH_CORNER)),
            CornerRadius::ZERO,
            accent,
        );
    }
}

/// Each side's length beside it, in metres: beside whichever of its two
/// edges is lower on screen (the nearer one, mostly).
fn sizes(
    ui: &Ui,
    plane: Plane,
    (from, to): (Vec2, Vec2),
    on_screen: &impl Fn(Vec3) -> Option<Pos2>,
    accent: Color32,
) {
    let Some(c) = corners(plane, (from, to), on_screen) else {
        return;
    };
    let middle = c.iter().fold(Pos2::ZERO, |m, p| m + p.to_vec2() / 4.0);
    let size = (to - from).abs();
    // Corners 0-1 and 3-2 run across (x); 1-2 and 0-3 along (y).
    for (length, edges) in [(size.x, [(0, 1), (3, 2)]), (size.y, [(1, 2), (0, 3)])] {
        let mid = |(a, b): (usize, usize)| c[a] + (c[b] - c[a]) / 2.0;
        let lower = edges
            .into_iter()
            .map(mid)
            .max_by(|a, b| a.y.total_cmp(&b.y))
            .unwrap_or(middle);
        let away = (lower - middle).normalized();
        let at = lower + away * size::BRUSH_LABEL_OFFSET;
        label(ui, at, &format!("{length:.2} m"), accent);
    }
}

/// A size in a small dark box with the accent round it, centred on `at`.
fn label(ui: &Ui, at: Pos2, words: &str, edge: Color32) {
    let painter = ui.painter();
    let galley = painter.layout_no_wrap(
        words.to_owned(),
        text::mono_small(),
        tokens::text_emphasis(),
    );
    let rect = Rect::from_center_size(
        at,
        galley.size() + vec2(2.0 * size::FIELD_PAD, size::FIELD_PAD),
    );
    painter.rect(
        rect,
        CornerRadius::same(size::radius::CONTROL),
        tokens::label_box_fill(),
        Stroke::new(widgets::hairline(ui), edge),
        egui::StrokeKind::Inside,
    );
    painter.galley(
        rect.center() - galley.size() / 2.0,
        galley,
        tokens::text_emphasis(),
    );
}

/// The height: an arrow from the footprint's middle up to the top, a dot
/// at its foot, and the height beside its head.
fn arrow(
    ui: &Ui,
    (foot, top): (Vec3, Vec3),
    on_screen: &impl Fn(Vec3) -> Option<Pos2>,
    height: f32,
) {
    let (Some(a), Some(b)) = (on_screen(foot), on_screen(top)) else {
        return;
    };
    let painter = ui.painter();
    let ink = tokens::text_emphasis();
    let [head, dot] = size::BRUSH_ARROW;
    painter.line_segment([a, b], Stroke::new(size::LINE * 1.5, ink));
    painter.circle_filled(a, dot, ink);
    let up = if a.distance(b) > 1e-3 {
        (b - a).normalized()
    } else {
        vec2(0.0, -1.0)
    };
    let side = vec2(-up.y, up.x) * head / 2.0;
    painter.add(egui::Shape::convex_polygon(
        vec![b + up * head, b + side, b - side],
        ink,
        Stroke::NONE,
    ));
    let at = b + vec2(size::BRUSH_LABEL_OFFSET * 2.0, size::BRUSH_LABEL_OFFSET);
    label(ui, at, &format!("{height:.2} m"), tokens::strong_line());
}

/// How far a step is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Progress {
    Done,
    Now,
    Later,
}

/// The steps across the top of the view: the shape, then 1 Footprint
/// (its sides once it has them) and, on the ground, 2 Height.
fn steps(ui: &mut Ui, view: Rect, draw: &Draw) -> Rect {
    let footprint = match draw.step {
        Step::Waiting { .. } => ("1 Footprint".to_owned(), Progress::Now),
        Step::Footprint { from, to, .. } => (
            format!("1 Footprint \u{b7} {}", footprint_words(from, to)),
            Progress::Now,
        ),
        Step::Height { from, to, .. } => (
            format!("1 Footprint \u{b7} {}", footprint_words(from, to)),
            Progress::Done,
        ),
    };
    let mut pills: Vec<(Option<Icon>, String, Progress)> = vec![
        (
            Some(super::icon(draw.shape)),
            draw.shape.name().to_owned(),
            Progress::Later,
        ),
        (None, footprint.0, footprint.1),
    ];
    if !draw.in_plane() {
        let height = if matches!(draw.step, Step::Height { .. }) {
            Progress::Now
        } else {
            Progress::Later
        };
        pills.push((None, "2 Height".to_owned(), height));
    }
    let font = text::ribbon_small();
    let widths: Vec<f32> = pills
        .iter()
        .map(|(icon, words, progress)| {
            let marks = usize::from(icon.is_some()) + usize::from(*progress == Progress::Done);
            widgets::width_of(ui, words, font.clone())
                + 2.0 * size::PILL_PAD
                + marks as f32 * (size::ICON_SMALL + size::PILL_ICON_GAP)
        })
        .collect();
    let whole = widths.iter().sum::<f32>() + size::PILL_GAP * (pills.len() - 1) as f32;
    let top = view.top() + size::PILL_MARGIN + size::PILL_H + size::BRUSH_STEPS_DROP;
    let mut left = view.center().x - whole / 2.0;
    let covered = Rect::from_min_size(pos2(left, top), vec2(whole, size::PILL_H));
    for ((icon, words, progress), width) in pills.into_iter().zip(widths) {
        let rect = Rect::from_min_size(pos2(left, top), vec2(width, size::PILL_H));
        step_pill(ui, rect, icon, &words, progress);
        left += width + size::PILL_GAP;
    }
    covered
}

/// One step's pill: the step in hand outlined in the accent; a done one
/// ticked.
fn step_pill(ui: &Ui, rect: Rect, icon: Option<Icon>, words: &str, progress: Progress) {
    let painter = ui.painter();
    let radius = size::radius::CONTROL;
    painter.rect_filled(rect, CornerRadius::same(radius), tokens::pill_fill());
    let (edge, ink) = match progress {
        Progress::Now => (
            crate::interface_look::accent_fill(),
            tokens::text_emphasis(),
        ),
        Progress::Done => (tokens::pill_border(), tokens::text_label()),
        Progress::Later => (tokens::pill_border(), tokens::text_primary()),
    };
    widgets::border(ui, rect, radius, edge);
    let mut left = rect.left() + size::PILL_PAD;
    let mark = icon.or((progress == Progress::Done).then_some(Icon::Check));
    if let Some(mark) = mark {
        let at = Rect::from_min_size(
            pos2(left, rect.center().y - size::ICON_SMALL / 2.0),
            vec2(size::ICON_SMALL, size::ICON_SMALL),
        );
        icons::icon(painter, at, mark, ink);
        left = at.right() + size::PILL_ICON_GAP;
    }
    widgets::words(
        ui,
        rect.with_min_x(left),
        Align2::LEFT_CENTER,
        words,
        text::ribbon_small(),
        ink,
    );
}

/// The keys at the foot of the view, for the step in hand.
fn keys(ui: &mut Ui, view: Rect, draw: &Draw) -> Rect {
    let keys: &[(&str, &str)] = match draw.step {
        Step::Height { .. } => &[
            ("Move up", "height"),
            ("Ctrl", "no snapping"),
            ("Click", "keep"),
            ("Esc", "give up"),
        ],
        _ if draw.in_plane() => &[
            ("Drag", "footprint"),
            ("Ctrl", "no snapping"),
            ("Let go", "make it"),
            ("Esc", "give up"),
        ],
        _ => &[
            ("Drag", "footprint"),
            ("Ctrl", "no snapping"),
            ("Let go", "height"),
            ("Esc", "give up"),
        ],
    };
    // Over the view's bottom pills (the locks, the space, the camera's
    // speed), as the mock-up draws it.
    let above = view.with_max_y(view.bottom() - size::PILL_H - size::PILL_MARGIN);
    widgets::pill::key_hints(ui, above, keys)
}
