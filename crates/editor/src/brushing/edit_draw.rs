//! What is drawn over the view while a brush's insides are edited (Tools
//! 1a, B3; the approved mock-up `03-brush-edit-faces.png`): the brush's
//! edges, a dot on each point (points mode) or at each face's middle
//! (faces mode), what's picked in the accent and what's under the
//! pointer lit; and, on the view in use, the text at its top and the
//! keys at its foot, over the tool bar. While an extrude or inset is
//! under way (B4, the mock-up `04-brush-extrude.png`): the new faces in
//! the accent, the distance in metres beside them, and its own words and
//! keys.
//!
//! Only the sides facing the camera are drawn, worked out from the
//! brush's own few faces each frame by the painter: never through the
//! area's other brushes.

use super::edit::{Editing, Mode, Part};
use super::ghost::{Progress, step_pill};
use super::pull::{Pull, PullKind};
use crate::camera::Camera;
use crate::icons::Icon;
use crate::tokens::{self, size, text};
use crate::widgets;
use egui::{Color32, Pos2, Rect, Shape, Ui, pos2, vec2};
use glam::{Mat3, Mat4, Vec2, Vec3};
use mesh::{FaceId, Mesh};

/// The keys at the foot of the view while editing: only what works now.
pub(crate) const KEYS: [(&str, &str); 5] = [
    ("Click", "pick"),
    ("Ctrl + Click", "add"),
    ("Drag", "box pick"),
    ("E", "extrude"),
    ("Tab", "done"),
];

/// How big a point's or a face's dot is, in points.
const DOT: f32 = 3.0;
/// How much bigger a picked or lit one is.
const DOT_LIT: f32 = 4.5;
/// How see-through a picked face's fill is.
const FACE_FILL: f32 = 0.35;

/// Draws `edit`'s brush (placed by `world`) over the view at `view`,
/// seen by `camera`: its edges, dots and what's picked.
pub(crate) fn show(ui: &mut Ui, view: Rect, camera: &Camera, (edit, world): (&Editing, Mat4)) {
    if let Some(mesh) = edit.mesh() {
        if let Some(pull) = &edit.pull {
            pulled(ui, view, camera, (pull, mesh, world));
        }
        parts(ui, view, camera, (edit, mesh, world));
    }
}

/// The view in use's words while `edit` goes on: the text at its top,
/// and the keys with their foot at `strip` (over the tool bar, or the
/// bottom pills). Returns where they cover the view (presses there
/// aren't the view's). While extruding or insetting, its own words and
/// keys (B4).
pub(crate) fn words(ui: &mut Ui, view: Rect, edit: &Editing, strip: f32) -> Vec<Rect> {
    let Some(pull) = &edit.pull else {
        return vec![
            top(ui, view, None, &edit.top_words()),
            keys(ui, view, strip, &KEYS),
        ];
    };
    let icon = match pull.kind {
        PullKind::Extrude | PullKind::ExtrudeEdges => Icon::ArrowUp,
        PullKind::Inset => Icon::FourSquares,
    };
    vec![
        top(ui, view, Some(icon), &edit.top_words()),
        keys(ui, view, strip, &pull.kind.keys()),
    ]
}

/// What an extrude or inset under way adds (B4): the new faces seen
/// through in the accent, and how far, in metres, beside the part
/// lowest on screen with a line from where it began.
fn pulled(ui: &Ui, view: Rect, camera: &Camera, (pull, mesh, world): (&Pull, &Mesh, Mat4)) {
    let size = Vec2::new(view.width(), view.height());
    let project = camera.projector(size);
    let at = |p: Vec3| project(world.transform_point3(p)).map(|s| view.min + vec2(s.x, s.y));
    let accent = crate::interface_look::accent_fill();
    let screen: Vec<Option<Pos2>> = mesh.point_slots().map(|p| p.and_then(at)).collect();
    let moving: std::collections::HashSet<mesh::VertexId> =
        pull.moving_points().into_iter().collect();
    let topology = mesh.topology();
    let fill = accent.gamma_multiply(tokens::share::SELECTED);
    let mut batch = Batch::default();
    let mut new_faces = std::collections::BTreeSet::new();
    for v in &moving {
        new_faces.extend(topology.vertex_faces(*v).iter().copied());
    }
    for face in new_faces {
        batch.face(mesh, face, &screen, fill);
    }
    ui.painter_at(view).add(Shape::mesh(batch.0));
    // Beside the moving part furthest out from the rest, square to its
    // own line, so the measure stands clear of the brush as the mock-up
    // draws it (below a side pulled sideways, beside a top pulled up).
    let lines: Vec<(Pos2, Pos2)> = pull
        .moved()
        .filter_map(|(from, to)| Some((at(from)?, at(to)?)))
        .collect();
    let centre = lines
        .iter()
        .fold(egui::Vec2::ZERO, |sum, (_, to)| sum + to.to_vec2())
        / lines.len().max(1) as f32;
    let outward = |(from, to): &(Pos2, Pos2)| {
        let along = *to - *from;
        let square = if along.length() > 1e-3 {
            along.normalized().rot90()
        } else {
            vec2(0.0, 1.0)
        };
        let middle = (from.to_vec2() + to.to_vec2()) / 2.0 - centre;
        let away = if middle.dot(square) < 0.0 {
            -square
        } else {
            square
        };
        (middle.dot(away), away)
    };
    let furthest = lines
        .iter()
        .max_by(|a, b| outward(a).0.total_cmp(&outward(b).0));
    if let Some(line) = furthest {
        let words = format!("{:.2} m", pull.metres.abs());
        measure(ui, *line, outward(line).1, &words, accent);
    }
}

/// A measuring line from `from` to `to`, set a little `away` from them
/// (a unit direction on screen), with a tick at each end and `words` in
/// a box beyond its middle.
fn measure(ui: &Ui, (from, to): (Pos2, Pos2), away: egui::Vec2, words: &str, accent: Color32) {
    let down = away * size::BRUSH_LABEL_OFFSET;
    let (a, b) = (from + down, to + down);
    let ink = tokens::text_emphasis();
    let stroke = egui::Stroke::new(size::LINE * 1.5, ink);
    let painter = ui.painter();
    painter.line_segment([a, b], stroke);
    let along = if a.distance(b) > 1e-3 {
        (b - a).normalized()
    } else {
        vec2(1.0, 0.0)
    };
    let tick = along.rot90() * size::BRUSH_ARROW[0] / 2.0;
    for end in [a, b] {
        painter.line_segment([end - tick, end + tick], stroke);
    }
    // The label's box clear of the line, whichever way it stands off.
    let width = widgets::width_of(ui, words, text::mono_small()) + 2.0 * size::FIELD_PAD;
    let height = widgets::line_height(ui, &text::mono_small()) + size::FIELD_PAD;
    let clear = away.x.abs() * width / 2.0 + away.y.abs() * height / 2.0;
    let middle = a + (b - a) / 2.0 + away * (size::BRUSH_LABEL_OFFSET + clear);
    super::ghost::label(ui, middle, words, accent);
}

/// Which faces of `mesh` face the camera, by face slot.
fn facing(mesh: &Mesh, camera: &Camera, world: Mat4) -> Vec<bool> {
    let normals = Mat3::from_mat4(world).inverse().transpose();
    let flat = camera.is_flat();
    let eye = camera.position;
    let forward = camera.forward();
    let (starts, _) = mesh.face_layout();
    let mut front = vec![false; starts.len().saturating_sub(1)];
    for (face, corners) in mesh.faces() {
        let normal = normals * mesh.face_normal(face);
        front[face.get() as usize] = if flat {
            normal.dot(forward) < 0.0
        } else {
            normal.dot(eye - world.transform_point3(middle_of(mesh, corners))) > 0.0
        };
    }
    front
}

/// The average of a face's corners.
fn middle_of(mesh: &Mesh, corners: &[mesh::VertexId]) -> Vec3 {
    corners
        .iter()
        .filter_map(|v| mesh.position(*v))
        .sum::<Vec3>()
        / corners.len().max(1) as f32
}

/// Everything drawn for the brush, gathered into one mesh of plain
/// triangles: a big brush has thousands of edges and dots, and one shape
/// costs the painter far less than thousands.
#[derive(Default)]
struct Batch(egui::Mesh);

impl Batch {
    /// A line from `a` to `b`, `width` points wide.
    fn line(&mut self, a: Pos2, b: Pos2, width: f32, colour: Color32) {
        let along = b - a;
        if along.length_sq() <= 0.0 {
            return;
        }
        let side = along.normalized().rot90() * (width / 2.0);
        self.quad([a - side, a + side, b + side, b - side], colour);
    }

    /// A square dot `size` points across at `at`.
    fn dot(&mut self, at: Pos2, size: f32, colour: Color32) {
        let r = Rect::from_center_size(at, vec2(size, size));
        self.quad(
            [
                r.left_top(),
                r.right_top(),
                r.right_bottom(),
                r.left_bottom(),
            ],
            colour,
        );
    }

    fn quad(&mut self, corners: [Pos2; 4], colour: Color32) {
        let first = self.0.vertices.len() as u32;
        for p in corners {
            self.0.colored_vertex(p, colour);
        }
        self.0.add_triangle(first, first + 1, first + 2);
        self.0.add_triangle(first, first + 2, first + 3);
    }

    /// A face filled with `colour`, cut into triangles as it is drawn.
    fn face(&mut self, mesh: &Mesh, face: FaceId, screen: &[Option<Pos2>], colour: Color32) {
        let Some(corners) = mesh.face(face) else {
            return;
        };
        let Some(points) = corners
            .iter()
            .map(|v| screen.get(v.get() as usize).copied().flatten())
            .collect::<Option<Vec<Pos2>>>()
        else {
            return;
        };
        let first = self.0.vertices.len() as u32;
        for p in points {
            self.0.colored_vertex(p, colour);
        }
        mesh::face_triangles(mesh, corners, |[a, b, c]| {
            self.0
                .add_triangle(first + a as u32, first + b as u32, first + c as u32);
        });
    }
}

/// The brush's edges, dots, picked parts and the part under the pointer.
fn parts(ui: &Ui, view: Rect, camera: &Camera, (edit, mesh, world): (&Editing, &Mesh, Mat4)) {
    let size = Vec2::new(view.width(), view.height());
    let project = camera.projector(size);
    let at = |p: Vec3| project(world.transform_point3(p)).map(|s| view.min + vec2(s.x, s.y));
    // Each point put on screen once.
    let screen: Vec<Option<Pos2>> = mesh.point_slots().map(|p| p.and_then(at)).collect();
    let point = |v: mesh::VertexId| screen.get(v.get() as usize).copied().flatten();
    let front = facing(mesh, camera, world);
    let faces_front = |f: &FaceId| front.get(f.get() as usize).copied().unwrap_or(false);
    let accent = crate::interface_look::accent_fill();
    let lit = tokens::mix(accent, Color32::WHITE, 0.55);
    let line = tokens::mix(tokens::text_emphasis(), tokens::strong_line(), 0.8);
    let mut batch = Batch::default();
    // Picked faces first, under the lines.
    if edit.mode == Mode::Faces {
        for part in edit.picked.iter().chain(edit.hovered.iter()) {
            if let Part::Face(face) = *part {
                let hovered = edit.hovered == Some(*part) && !edit.picked.contains(part);
                let fill = if hovered { lit } else { accent };
                batch.face(mesh, face, &screen, fill.gamma_multiply(FACE_FILL));
            }
        }
    }
    let topology = mesh.topology();
    for edge in mesh.edges() {
        if !topology.edge_faces(*edge).any(|f| faces_front(&f)) {
            continue;
        }
        let (a, b) = edge.ends();
        if let (Some(p), Some(q)) = (point(a), point(b)) {
            batch.line(p, q, LINE, line);
        }
    }
    // The picked edges, and the rim of the picked faces (sides shared by
    // two picked faces are inside it), in the accent.
    let mut sides: std::collections::HashMap<mesh::EdgeKey, u32> = Default::default();
    for part in &edit.picked {
        match *part {
            Part::Face(face) => {
                for edge in topology.face_edges(face) {
                    *sides.entry(*edge).or_default() += 1;
                }
            }
            other => outline_part(&mut batch, mesh, other, &point, accent),
        }
    }
    for (edge, _) in sides.into_iter().filter(|(_, n)| *n == 1) {
        outline_part(&mut batch, mesh, Part::Edge(edge), &point, accent);
    }
    if let Some(part) = edit.hovered {
        outline_part(&mut batch, mesh, part, &point, lit);
    }
    dots(
        &mut batch,
        (edit, mesh),
        (&faces_front, &point),
        (accent, lit, line),
    );
    ui.painter_at(view).add(Shape::mesh(batch.0));
}

/// Where a point is on screen, if it is.
type OnScreen<'a> = dyn Fn(mesh::VertexId) -> Option<Pos2> + 'a;

/// How wide the brush's edges are drawn, and a picked one's.
const LINE: f32 = 1.2;
const LINE_PICKED: f32 = 2.0;

/// The lines of a picked or lit edge or face.
fn outline_part(
    batch: &mut Batch,
    mesh: &Mesh,
    part: Part,
    point: &dyn Fn(mesh::VertexId) -> Option<Pos2>,
    colour: Color32,
) {
    let mut side = |a, b| {
        if let (Some(p), Some(q)) = (point(a), point(b)) {
            batch.line(p, q, LINE_PICKED, colour);
        }
    };
    match part {
        Part::Edge(edge) => {
            let (a, b) = edge.ends();
            side(a, b);
        }
        Part::Face(face) => {
            let corners = mesh.face(face).unwrap_or(&[]);
            for i in 0..corners.len() {
                side(corners[i], corners[(i + 1) % corners.len()]);
            }
        }
        Part::Point(_) => {}
    }
}

/// A dot on each point (points mode) or at each face's middle (faces
/// mode) facing the camera; picked ones in the accent, the one under the
/// pointer lit.
fn dots(
    batch: &mut Batch,
    (edit, mesh): (&Editing, &Mesh),
    (front, point): (&dyn Fn(&FaceId) -> bool, &OnScreen<'_>),
    (accent, lit, plain): (Color32, Color32, Color32),
) {
    let look = |part: Part| {
        if edit.hovered == Some(part) {
            (lit, DOT_LIT)
        } else if edit.picked.contains(&part) {
            (accent, DOT_LIT)
        } else {
            (plain, DOT)
        }
    };
    match edit.mode {
        Mode::Points => {
            let topology = mesh.topology();
            for (v, _) in mesh.vertices() {
                if !topology.vertex_faces(v).iter().any(front) {
                    continue;
                }
                if let Some(s) = point(v) {
                    let (colour, size) = look(Part::Point(v));
                    batch.dot(s, size * 1.6, colour);
                }
            }
        }
        Mode::Faces => {
            for (face, corners) in mesh.faces() {
                if !front(&face) {
                    continue;
                }
                let middle: Option<Vec<Pos2>> = corners.iter().map(|v| point(*v)).collect();
                if let Some(middle) = middle {
                    let sum = middle
                        .iter()
                        .fold(Vec2::ZERO, |s, p| s + Vec2::new(p.x, p.y));
                    let s = sum / middle.len().max(1) as f32;
                    let (colour, size) = look(Part::Face(face));
                    batch.dot(Pos2::new(s.x, s.y), size * 1.6, colour);
                }
            }
        }
        Mode::Edges => {}
    }
}

/// The text at the top of the view, outlined in the accent as a step in
/// hand is: "Editing Box_03 · Faces · 1 picked".
fn top(ui: &mut Ui, view: Rect, icon: Option<Icon>, words: &str) -> Rect {
    let marks = if icon.is_some() {
        size::ICON_SMALL + size::PILL_ICON_GAP
    } else {
        0.0
    };
    let width = widgets::width_of(ui, words, text::ribbon_small()) + 2.0 * size::PILL_PAD + marks;
    let top = view.top() + size::PILL_MARGIN + size::PILL_H + size::BRUSH_STEPS_DROP;
    let rect = Rect::from_min_size(
        pos2(view.center().x - width / 2.0, top),
        vec2(width, size::PILL_H),
    );
    step_pill(ui, rect, icon, words, Progress::Now);
    rect
}

/// The keys at the foot of the view, their own foot at `strip`: over the
/// tool bar, or the bottom pills, as the mock-ups draw them.
fn keys(ui: &mut Ui, view: Rect, strip: f32, keys: &[(&str, &str)]) -> Rect {
    let above = view.with_max_y(strip + size::PILL_MARGIN);
    widgets::pill::key_hints(ui, above, keys)
}
