//! Extruding and insetting by moving the mouse (Tools 1a, B4; decision
//! 0111; the approved mock-up `docs/design/tools-1/04-brush-extrude.png`):
//! **E** (or Extrude on the ribbon) with faces picked, then the mouse
//! pulls the new part out along the faces' way and a click keeps it;
//! Inset on the ribbon works the same way, the mouse setting how wide
//! the border is. Ctrl turns snapping off; Esc or a right-click gives up
//! and leaves nothing.
//!
//! The operation is made once, as it starts, at no distance ([`Pull`]
//! keeps that mesh and, for each point that moves, where it starts and
//! which way it goes): every frame after only moves those points, so only
//! the brush's own preview is drawn anew, never the area's. Extruding and
//! insetting move each point in a straight line as the distance grows,
//! so this is exact.
//!
//! Touching faces go out together as one piece, straight along their
//! average facing (`mesh::edit::ExtrudeShape`'s Straight); pieces that
//! don't touch each go along their own. The mouse is read along the
//! line through the pick's middle in the way of all the picked faces
//! together, or, when the camera looks along that line, up the screen.

use crate::camera::Camera;
use glam::{Mat3, Mat4, Vec2, Vec3};
use mesh::edit::{ExtrudeShape, Grouping, extrude_edges, extrude_shaped, inset_polygons};
use mesh::{EdgeKey, FaceId, Mesh, VertexId};
use std::sync::Arc;

/// What a pull does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PullKind {
    /// Pulls the picked faces out (E).
    Extrude,
    /// Grows a face from each picked edge on an open side.
    ExtrudeEdges,
    /// Makes a smaller face inside each picked face, with a border round
    /// it.
    Inset,
}

impl PullKind {
    /// What the top of the view and the status bar say it is doing.
    pub(crate) fn doing(self) -> &'static str {
        match self {
            Self::Extrude | Self::ExtrudeEdges => "Extruding",
            Self::Inset => "Insetting",
        }
    }

    /// Its undo step's name.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Extrude | Self::ExtrudeEdges => "Extrude",
            Self::Inset => "Inset",
        }
    }

    /// The keys at the foot of the view while it goes on.
    pub(crate) fn keys(self) -> [(&'static str, &'static str); 5] {
        let moves = match self {
            Self::Extrude | Self::ExtrudeEdges => "pull out",
            Self::Inset => "border width",
        };
        [
            ("Move", moves),
            ("Ctrl", "no snapping"),
            ("Click", "keep"),
            ("Esc", "or"),
            ("Right-click", "give up"),
        ]
    }
}

/// How the mouse is read, chosen when it is first seen.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Gauge {
    /// Along the line through `origin` going `way` (world, unit long).
    Line { origin: Vec3, way: Vec3 },
    /// Up the screen, `per_point` metres a point.
    Up { per_point: f32 },
    /// Away from where the pointer began, any way, `per_point` metres a
    /// point.
    Away { per_point: f32 },
}

/// The mouse this frame, as a pull needs it.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct PullInput {
    /// Where the pointer is, from the view's top left, in points.
    pub(crate) mouse: Option<Vec2>,
    /// The left button went down on the view.
    pub(crate) pressed: bool,
    pub(crate) right_pressed: bool,
    pub(crate) ctrl: bool,
    /// The pointer is on the view itself (not its pills).
    pub(crate) on_view: bool,
}

/// What came of a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PullOutcome {
    /// Still going; whether the brush changed.
    Going(bool),
    /// A click: keep it.
    Keep,
    /// A right-click: give up.
    GiveUp,
}

/// An extrude or inset under way, kept with the brush being edited.
#[derive(Debug, Clone)]
pub(crate) struct Pull {
    pub(crate) kind: PullKind,
    /// What's pulled, in words: "1 face", "3 edges".
    what: String,
    /// The brush with the operation made at no distance.
    start: Arc<Mesh>,
    /// Each point that moves: where it starts and how far it goes for one
    /// unit of distance, in the brush's own space.
    moving: Vec<(VertexId, Vec3, Vec3)>,
    /// The way the mouse pulls, in the world (unit long), through the
    /// middle of what's picked.
    origin: Vec3,
    way: Vec3,
    /// Metres in the world for one unit of the brush's own space.
    scale: f32,
    /// How the mouse is read, and where it began, once it is seen.
    gauge: Option<(Gauge, Vec2)>,
    /// How far, in metres: snapped to the grid unless `free`.
    pub(crate) metres: f32,
    /// Ctrl is held: no snapping.
    pub(crate) free: bool,
    /// The brush as it would be now.
    now: Arc<Mesh>,
}

impl Pull {
    /// Extruding `faces` of `mesh` (placed by `world`); `None` when none
    /// of them is in it.
    pub(crate) fn extrude(
        mesh: &Mesh,
        faces: &[FaceId],
        world: Mat4,
        what: String,
    ) -> Option<Self> {
        Self::prepared(PullKind::Extrude, mesh, world, what, |m, d| {
            extrude_shaped(m, faces, d, Grouping::Together, ExtrudeShape::default());
        })
    }

    /// Extruding `edges` (those on an open side).
    pub(crate) fn extrude_edges(
        mesh: &Mesh,
        edges: &[EdgeKey],
        world: Mat4,
        what: String,
    ) -> Option<Self> {
        Self::prepared(PullKind::ExtrudeEdges, mesh, world, what, |m, d| {
            extrude_edges(m, edges, d, Grouping::Together);
        })
    }

    /// Insetting `faces`.
    pub(crate) fn inset(mesh: &Mesh, faces: &[FaceId], world: Mat4, what: String) -> Option<Self> {
        Self::prepared(PullKind::Inset, mesh, world, what, |m, d| {
            inset_polygons(m, faces, d, Grouping::Together);
        })
    }

    /// The operation `op` made at no distance and at one unit, which give
    /// where each moving point starts and which way it goes.
    fn prepared(
        kind: PullKind,
        mesh: &Mesh,
        world: Mat4,
        what: String,
        op: impl Fn(&mut Mesh, f32),
    ) -> Option<Self> {
        let mut start = mesh.clone();
        op(&mut start, 0.0);
        let mut one = mesh.clone();
        op(&mut one, 1.0);
        let moving: Vec<(VertexId, Vec3, Vec3)> = start
            .vertices()
            .filter_map(|(v, at)| {
                let per = one.position(v)? - at;
                (per.length_squared() > 0.0 && per.is_finite()).then_some((v, at, per))
            })
            .collect();
        if moving.is_empty() {
            return None;
        }
        let count = moving.len() as f32;
        let middle = moving.iter().map(|(_, at, _)| *at).sum::<Vec3>() / count;
        let sum: Vec3 = moving.iter().map(|(_, _, per)| *per).sum();
        let local_way = sum
            .try_normalize()
            .or_else(|| moving[0].2.try_normalize())
            .unwrap_or(Vec3::Z);
        let turned = world.transform_vector3(local_way);
        let scale = match kind {
            PullKind::Extrude => turned.length(),
            PullKind::ExtrudeEdges | PullKind::Inset => {
                let m = Mat3::from_mat4(world);
                (m.x_axis.length() + m.y_axis.length() + m.z_axis.length()) / 3.0
            }
        };
        if !(scale.is_finite() && scale > 1e-6) {
            return None;
        }
        let start = Arc::new(start);
        Some(Self {
            kind,
            what,
            now: Arc::clone(&start),
            start,
            moving,
            origin: world.transform_point3(middle),
            way: turned.normalize_or(Vec3::Z),
            scale,
            gauge: None,
            metres: 0.0,
            free: false,
        })
    }

    /// The brush as it would be now.
    pub(crate) fn now(&self) -> &Mesh {
        &self.now
    }

    /// The brush as it would be now, shared.
    pub(crate) fn now_shared(&self) -> Arc<Mesh> {
        Arc::clone(&self.now)
    }

    /// The points that move: what the whole check looks around.
    pub(crate) fn moving_points(&self) -> Vec<VertexId> {
        self.moving.iter().map(|(v, _, _)| *v).collect()
    }

    /// Each moving point where it started and where it is now, in the
    /// brush's own space.
    pub(crate) fn moved(&self) -> impl Iterator<Item = (Vec3, Vec3)> + '_ {
        self.moving
            .iter()
            .filter_map(|(v, at, _)| Some((*at, self.now.position(*v)?)))
    }

    /// Takes this frame's mouse, with the camera's view (`size` points)
    /// and the grid's step in metres.
    pub(crate) fn update(
        &mut self,
        input: PullInput,
        (camera, size): (&Camera, Vec2),
        grid: f32,
    ) -> PullOutcome {
        if input.right_pressed {
            return PullOutcome::GiveUp;
        }
        if input.pressed && input.on_view {
            return PullOutcome::Keep;
        }
        let Some(mouse) = input.mouse else {
            return PullOutcome::Going(false);
        };
        let (gauge, from) = match self.gauge {
            Some(seen) => seen,
            None => {
                let seen = (self.gauge_for(mouse, camera, size), mouse);
                self.gauge = Some(seen);
                seen
            }
        };
        let Some(raw) = reading(gauge, (from, mouse), camera, size) else {
            return PullOutcome::Going(false);
        };
        let snapped = if input.ctrl || grid <= 0.0 {
            raw
        } else {
            (raw / grid).round() * grid
        };
        let metres = match self.kind {
            PullKind::Extrude => snapped,
            PullKind::ExtrudeEdges | PullKind::Inset => snapped.max(0.0),
        };
        let changed = metres != self.metres;
        self.free = input.ctrl;
        if changed {
            self.set(metres);
        }
        PullOutcome::Going(changed)
    }

    /// Moves the moving points `metres` along.
    pub(crate) fn set(&mut self, metres: f32) {
        if !metres.is_finite() {
            return;
        }
        self.metres = metres;
        let units = metres / self.scale;
        let mut now = (*self.start).clone();
        for (v, at, per) in &self.moving {
            now.set_position(*v, *at + *per * units);
        }
        self.now = Arc::new(now);
    }

    /// How the mouse will be read, it being first seen at `mouse`.
    fn gauge_for(&self, mouse: Vec2, camera: &Camera, size: Vec2) -> Gauge {
        let project = camera.projector(size);
        let across = match (project(self.origin), project(self.origin + camera.right())) {
            (Some(a), Some(b)) => a.distance(b),
            _ => 0.0,
        };
        let per_point = if across > 1e-3 { 1.0 / across } else { 0.01 };
        if self.kind != PullKind::Extrude {
            return Gauge::Away { per_point };
        }
        let ray = camera.ray_through(mouse, size);
        if ray.direction.dot(self.way).abs() > 0.97 {
            return Gauge::Up { per_point };
        }
        Gauge::Line {
            origin: self.origin,
            way: self.way,
        }
    }

    /// The text at the top of the view: "Extruding 1 face · 2.00 m".
    pub(crate) fn top_words(&self) -> String {
        format!(
            "{} {} \u{b7} {:.2} m",
            self.kind.doing(),
            self.what,
            self.metres
        )
    }

    /// What the status bar says: "Extruding 1 face: 2.00 m · Click to
    /// keep · Esc or right-click to give up".
    pub(crate) fn status_words(&self) -> String {
        format!(
            "{} {}: {:.2} m \u{b7} Click to keep \u{b7} Esc or right-click to give up",
            self.kind.doing(),
            self.what,
            self.metres
        )
    }

    /// Its undo step's name: "Extrude 1 face".
    pub(crate) fn label(&self) -> String {
        format!("{} {}", self.kind.name(), self.what)
    }
}

/// How far the pointer at `now` is from where it began (`from`), in
/// metres, as `gauge` reads it.
fn reading(gauge: Gauge, (from, now): (Vec2, Vec2), camera: &Camera, size: Vec2) -> Option<f32> {
    match gauge {
        Gauge::Line { origin, way } => {
            let along = |at: Vec2| along_line(origin, way, camera.ray_through(at, size));
            Some(along(now)? - along(from)?)
        }
        Gauge::Up { per_point } => Some((from.y - now.y) * per_point),
        Gauge::Away { per_point } => Some(from.distance(now) * per_point),
    }
}

/// How far along the line through `origin` going `way` its nearest point
/// to `ray` is; none when they run side by side.
fn along_line(origin: Vec3, way: Vec3, ray: crate::screen::Ray) -> Option<f32> {
    let b = way.dot(ray.direction);
    let apart = 1.0 - b * b;
    if apart < 1e-6 {
        return None;
    }
    let w = origin - ray.origin;
    let (d, e) = (way.dot(w), ray.direction.dot(w));
    let along = (b * e - d) / apart;
    along.is_finite().then_some(along)
}

#[cfg(test)]
#[path = "pull_tests.rs"]
mod tests;
