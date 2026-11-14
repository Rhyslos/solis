//! Moving, turning and stretching a brush's picked parts with the gizmo,
//! and stretching the whole brush to a size typed on its card (Tools 1a,
//! B3).
//!
//! A drag works from the brush as it was when the drag began: each frame
//! the change so far is made again on that copy, in the brush's own
//! space, so only the brush being dragged is drawn anew. Nothing reaches
//! the area until the drag is let go: then the brush is checked (only
//! the faces touching what moved, [`mesh::check_moved`]) and either
//! becomes one undo step or snaps back, and the status bar says why.

use crate::gizmo::Change;
use crate::transforming::{Targets, clamp_scale};
use glam::{Mat3, Mat4, Quat, Vec3};
use mesh::{Broken, Mesh, VertexId};
use std::sync::Arc;

/// The smallest size a brush can be typed to, along each axis, in metres
/// (a shape's smallest, `mesh::ShapeSettings::clamped`).
pub(crate) const SMALLEST: f32 = 0.01;
/// The largest, in metres.
pub(crate) const LARGEST: f32 = 10_000.0;

/// A gizmo drag of a brush's parts, under way.
#[derive(Debug, Clone)]
pub(crate) struct Drag {
    /// The brush as it was when the drag began.
    start: Arc<Mesh>,
    /// As it would be now.
    now: Arc<Mesh>,
    /// The points that move.
    points: Vec<VertexId>,
    /// Where the gizmo sat and its axes, for snapping (no entities move).
    pub(crate) targets: Targets,
    /// Where the brush is in the world.
    world: Mat4,
    /// The change so far, in the brush's own space.
    local: Mat4,
    /// What the drag is doing, for its undo step's name.
    verb: &'static str,
}

impl Drag {
    /// A drag of `points` of `mesh` (placed in the world by `world`), the
    /// gizmo at `pivot` along `axes`.
    pub(crate) fn begin(
        mesh: Arc<Mesh>,
        points: Vec<VertexId>,
        (pivot, axes): (Vec3, [Vec3; 3]),
        world: Mat4,
    ) -> Self {
        Self {
            now: Arc::clone(&mesh),
            start: mesh,
            points,
            targets: Targets {
                entities: Vec::new(),
                pivot,
                axes,
            },
            world,
            local: Mat4::IDENTITY,
            verb: "Move",
        }
    }

    /// The brush as dragged so far.
    pub(crate) fn now(&self) -> &Mesh {
        &self.now
    }

    /// The brush as dragged so far, shared.
    pub(crate) fn now_shared(&self) -> Arc<Mesh> {
        Arc::clone(&self.now)
    }

    /// Sets the change so far (snapped already): the picked points of the
    /// brush as it was, moved by it.
    pub(crate) fn set(&mut self, change: Change) {
        let in_world = world_change(change, self.targets.pivot, self.targets.axes);
        let own = self.world.inverse();
        let local = own * in_world * self.world;
        if !local.is_finite() {
            return;
        }
        self.verb = match change {
            Change::Translate(_) => "Move",
            Change::Rotate { .. } => "Rotate",
            Change::Scale(_) => "Scale",
        };
        self.local = local;
        let mut now = (*self.start).clone();
        mesh::edit::transform_points(&mut now, &self.points, local);
        self.now = Arc::new(now);
    }

    /// The drag let go: the brush as it ends, or `None` if nothing moved;
    /// `Err` with why if it would break the brush.
    pub(crate) fn finish(&self) -> Result<Option<Arc<Mesh>>, Broken> {
        if Arc::ptr_eq(&self.now, &self.start) || *self.now == *self.start {
            return Ok(None);
        }
        mesh::check_moved(
            &self.start,
            &self.now,
            &self.points,
            Mat3::from_mat4(self.local),
        )?;
        Ok(Some(Arc::clone(&self.now)))
    }

    /// Its undo step's name: "Move 1 face", "Rotate 4 points".
    pub(crate) fn label(&self, what: &str) -> String {
        format!("{} {what}", self.verb)
    }
}

/// `change` (from the gizmo, snapped) as a matrix in the world: a move,
/// a turn about `pivot`, or a stretch about it along `axes`.
pub(crate) fn world_change(change: Change, pivot: Vec3, axes: [Vec3; 3]) -> Mat4 {
    let about = |m: Mat4| Mat4::from_translation(pivot) * m * Mat4::from_translation(-pivot);
    match change {
        Change::Translate(v) => Mat4::from_translation(v),
        Change::Rotate { axis, angle } => about(Mat4::from_quat(Quat::from_axis_angle(
            axis.normalize_or(Vec3::Z),
            angle,
        ))),
        Change::Scale(f) => {
            let f = Vec3::new(clamp_scale(f.x), clamp_scale(f.y), clamp_scale(f.z));
            let axes = Mat3::from_cols(axes[0], axes[1], axes[2]);
            about(Mat4::from_mat3(
                axes * Mat3::from_diagonal(f) * axes.transpose(),
            ))
        }
    }
}

/// Where the gizmo sits for `points` of `mesh` placed by `world`: the
/// middle of the box around them, in the world.
pub(crate) fn pivot_of(mesh: &Mesh, points: &[VertexId], world: Mat4) -> Option<Vec3> {
    let placed = points
        .iter()
        .filter_map(|v| mesh.position(*v))
        .map(|p| world.transform_point3(p));
    let bounds = mesh::Bounds::around(placed)?;
    Some(bounds.centre())
}

/// The size of the box around `mesh`, in metres in its own space.
pub(crate) fn size_of(mesh: &Mesh) -> Vec3 {
    mesh.bounds().map_or(Vec3::ZERO, |b| b.size())
}

/// `mesh` stretched about its middle to `size` (each side kept between
/// [`SMALLEST`] and [`LARGEST`]; a side with no length, which can't be
/// stretched, stays as it is), and the stretch. `None` when nothing
/// would change or the size isn't a number; `Err` if it would break the
/// brush (one face flattened against another, say).
pub(crate) fn stretched(mesh: &Mesh, size: Vec3) -> Result<Option<(Mesh, Mat4)>, Broken> {
    let Some(bounds) = mesh.bounds() else {
        return Ok(None);
    };
    if !size.is_finite() {
        return Ok(None);
    }
    let was = bounds.size();
    let wanted = size.clamp(Vec3::splat(SMALLEST), Vec3::splat(LARGEST));
    let factor = Vec3::select(was.cmpgt(Vec3::splat(1e-6)), wanted / was, Vec3::ONE);
    if (factor - Vec3::ONE).abs().max_element() < 1e-6 {
        return Ok(None);
    }
    let middle = bounds.centre();
    let stretch =
        Mat4::from_translation(middle) * Mat4::from_scale(factor) * Mat4::from_translation(-middle);
    let points: Vec<VertexId> = mesh.vertices().map(|(v, _)| v).collect();
    let mut after = mesh.clone();
    mesh::edit::transform_points(&mut after, &points, stretch);
    mesh::check_moved(mesh, &after, &points, Mat3::from_mat4(stretch))?;
    Ok(Some((after, stretch)))
}

/// What the status bar says when a change is refused: "Put back: it
/// would fold a face over itself".
pub(crate) fn refused_words(broken: Broken) -> String {
    format!("Put back: {broken}")
}

#[cfg(test)]
#[path = "reshape_tests.rs"]
mod tests;
