//! Moving, turning and scaling the selection with the gizmo: what moves,
//! where the gizmo sits, snapping, and each entity's new transform for a
//! [`Change`] ([decision 0025](../../../docs/decisions/0025-picking-and-gizmos.md),
//! following Aventail's rules for moving several objects).
//!
//! A drag always works from where everything was when it started, so each
//! frame sets the whole change again; the edits merge into one undo step.

use crate::document::Selection;
use crate::gizmo::{Change, snap};
use area::Area;
use area::builtin::Transform;
use glam::{Mat3, Mat4, Quat, Vec3};
use reflect::EntityId;
use serde::{Deserialize, Serialize};

/// Grid steps for moving, in metres, as the snapping corner offers them.
pub const GRID_STEPS: [f32; 4] = [1.0, 0.5, 0.25, 0.1];
/// Angle steps for turning, in degrees.
pub const ANGLE_STEPS: [f32; 4] = [5.0, 15.0, 45.0, 90.0];
/// The scale step: 10%.
pub const SCALE_STEP: f32 = 0.1;
/// The smallest scale a drag makes, so nothing is squashed flat (which
/// can't be undone by scaling back up).
const MIN_SCALE: f32 = 1e-3;

/// Snapping and axis settings, shown in the viewport's bottom-right
/// corner and saved per computer.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Snapping {
    /// Moves land on the grid.
    pub grid: bool,
    /// The grid step, in metres.
    pub grid_step: f32,
    /// Turns go in steps.
    pub angle: bool,
    /// The angle step, in degrees.
    pub angle_step: f32,
    /// Scaling goes in 10% steps.
    pub scale: bool,
    /// The gizmo follows the selected entity's own axes, not the world's
    /// (L).
    pub object_axes: bool,
    /// Axes that drags leave alone (X, Y, Z).
    pub locked: [bool; 3],
}

impl Default for Snapping {
    fn default() -> Self {
        Self {
            grid: false,
            grid_step: 1.0,
            angle: false,
            angle_step: 15.0,
            scale: false,
            object_axes: false,
            locked: [false; 3],
        }
    }
}

impl Snapping {
    /// The settings with anything a damaged save could hold put back in
    /// range: steps must be ones the corner offers.
    #[must_use]
    pub fn sanitized(self) -> Self {
        let d = Self::default();
        Self {
            grid_step: if GRID_STEPS.contains(&self.grid_step) {
                self.grid_step
            } else {
                d.grid_step
            },
            angle_step: if ANGLE_STEPS.contains(&self.angle_step) {
                self.angle_step
            } else {
                d.angle_step
            },
            ..self
        }
    }
}

/// One entity being moved, as it was when the drag started.
#[derive(Debug, Clone, PartialEq)]
pub struct Target {
    /// The entity.
    pub id: EntityId,
    /// Its transform, relative to its parent.
    pub start: Transform,
    /// Where it was in the world.
    pub start_world: Mat4,
    /// Its parent's world matrix (identity at the top level).
    pub parent_world: Mat4,
}

/// What a drag moves, and where the gizmo sits.
#[derive(Debug, Clone, PartialEq)]
pub struct Targets {
    /// The entities that move: selected ones with a transform, leaving out
    /// any inside another selected one (it moves with its parent).
    pub entities: Vec<Target>,
    /// Where the gizmo sits: the middle of their positions.
    pub pivot: Vec3,
    /// The gizmo's axes, unit length.
    pub axes: [Vec3; 3],
}

/// The world axes.
pub const WORLD_AXES: [Vec3; 3] = [Vec3::X, Vec3::Y, Vec3::Z];

/// What moving the selection would move, or `None` when nothing selected
/// has a transform (or it's all locked away).
pub fn targets(area: &Area, selection: &Selection, object_axes: bool) -> Option<Targets> {
    targets_placed(area, &|id| area.world_matrix(id), selection, object_axes)
}

/// [`targets`], with where each entity is in the world from `world` (the
/// document's matrices, worked out once a frame).
pub fn targets_placed(
    area: &Area,
    world: &dyn Fn(EntityId) -> Mat4,
    selection: &Selection,
    object_axes: bool,
) -> Option<Targets> {
    let selected: Vec<EntityId> = selection.iter().collect();
    let inside_another = |id: EntityId| {
        let mut parent = area.get(id).and_then(|e| e.parent);
        // A damaged area's loop of parents ends the walk.
        let mut steps = 0;
        while let Some(p) = parent {
            if selection.contains(p) {
                return true;
            }
            steps += 1;
            if steps > area.len() {
                break;
            }
            parent = area.get(p).and_then(|e| e.parent);
        }
        false
    };
    let entities: Vec<Target> = selected
        .into_iter()
        .filter(|&id| !inside_another(id))
        .filter_map(|id| {
            let entity = area.get(id)?;
            let start = entity.get::<Transform>()?;
            let parent_world = entity.parent.map_or(Mat4::IDENTITY, world);
            Some(Target {
                id,
                start,
                start_world: parent_world * start.matrix(),
                parent_world,
            })
        })
        .collect();
    if entities.is_empty() {
        return None;
    }
    let pivot = entities
        .iter()
        .map(|t| t.start_world.w_axis.truncate())
        .sum::<Vec3>()
        / entities.len() as f32;
    let axes = if object_axes {
        let primary = selection
            .primary()
            .and_then(|p| entities.iter().find(|t| t.id == p))
            .unwrap_or(&entities[0]);
        own_axes(primary.start_world)
    } else {
        WORLD_AXES
    };
    Some(Targets {
        entities,
        pivot,
        axes,
    })
}

/// A world matrix's own axes, unit length; the world's when it's squashed
/// flat.
fn own_axes(world: Mat4) -> [Vec3; 3] {
    let (_, rotation, _) = world.to_scale_rotation_translation();
    if rotation.is_finite() {
        [rotation * Vec3::X, rotation * Vec3::Y, rotation * Vec3::Z]
    } else {
        WORLD_AXES
    }
}

/// `change` with snapping applied (turned around by `flip`, Ctrl held):
/// moves land on the grid (world axes) or go in grid steps (object
/// axes), turns go in angle steps, scaling in 10% steps.
pub fn snapped(change: Change, targets: &Targets, snapping: &Snapping, flip: bool) -> Change {
    match change {
        Change::Translate(v) if snapping.grid != flip => {
            let step = snapping.grid_step;
            if targets.axes == WORLD_AXES {
                // The pivot lands on grid lines, along the axes that move.
                let to = targets.pivot + v;
                let mut snapped = to;
                for i in 0..3 {
                    if v[i].abs() > 1e-9 {
                        snapped[i] = snap(to[i], step);
                    }
                }
                Change::Translate(snapped - targets.pivot)
            } else {
                Change::Translate(
                    targets
                        .axes
                        .iter()
                        .map(|&axis| axis * snap(v.dot(axis), step))
                        .sum(),
                )
            }
        }
        Change::Rotate { axis, angle } if snapping.angle != flip => Change::Rotate {
            axis,
            angle: snap(angle, snapping.angle_step.to_radians()),
        },
        Change::Scale(f) if snapping.scale != flip => {
            Change::Scale(Vec3::new(snap_scale(f.x), snap_scale(f.y), snap_scale(f.z)))
        }
        other => other,
    }
}

/// A scale factor rounded to 10% steps, never 0.
fn snap_scale(f: f32) -> f32 {
    let s = snap(f, SCALE_STEP);
    if s.abs() < SCALE_STEP * 0.5 {
        SCALE_STEP.copysign(f)
    } else {
        s
    }
}

/// Each target's new transform (relative to its parent) for `change`,
/// made from where it was when the drag started.
pub fn moved(targets: &Targets, change: Change) -> Vec<(EntityId, Transform)> {
    targets
        .entities
        .iter()
        .map(|t| (t.id, move_one(t, targets, change)))
        .collect()
}

fn move_one(t: &Target, targets: &Targets, change: Change) -> Transform {
    let to_parent = t.parent_world.inverse();
    if !to_parent.is_finite() {
        // A parent squashed flat: nothing sensible to do.
        return t.start;
    }
    let pivot = targets.pivot;
    match change {
        Change::Translate(v) => {
            // Position only: rotation and scale stay exactly as they were.
            Transform {
                position: t.start.position + to_parent.transform_vector3(v),
                ..t.start
            }
        }
        Change::Rotate { axis, angle } => {
            let turn = Quat::from_axis_angle(axis.normalize_or(Vec3::Z), angle);
            let world_position = t.start_world.w_axis.truncate();
            let new_world_position = pivot + turn * (world_position - pivot);
            // The turn seen from inside the parent, applied on top of the
            // entity's own rotation; its scale stays as it was.
            let (_, parent_rotation, _) = t.parent_world.to_scale_rotation_translation();
            let local_turn = parent_rotation.inverse() * turn * parent_rotation;
            Transform {
                position: to_parent.transform_point3(new_world_position),
                rotation: (local_turn * t.start.rotation).normalize(),
                scale: t.start.scale,
            }
        }
        Change::Scale(f) => {
            let f = Vec3::new(clamp_scale(f.x), clamp_scale(f.y), clamp_scale(f.z));
            let axes = Mat3::from_cols(targets.axes[0], targets.axes[1], targets.axes[2]);
            let stretch = Mat4::from_translation(pivot)
                * Mat4::from_mat3(axes * Mat3::from_diagonal(f) * axes.transpose())
                * Mat4::from_translation(-pivot);
            let world = stretch * t.start_world;
            let own = own_axes(t.start_world);
            let aligned = (0..3).all(|i| own[i].dot(targets.axes[i]).abs() > 0.9999);
            let position = to_parent.transform_point3(world.w_axis.truncate());
            if aligned && t.parent_world == Mat4::IDENTITY {
                // Along its own axes: the scale multiplies exactly, with
                // no rounding creeping into the rotation.
                Transform {
                    position,
                    rotation: t.start.rotation,
                    scale: t.start.scale * f,
                }
            } else {
                // Along other axes a turned entity would need a skew; the
                // nearest shape without one is used (as Aventail does).
                let (scale, rotation, _) = (to_parent * world).to_scale_rotation_translation();
                Transform {
                    position,
                    rotation: rotation.normalize(),
                    scale,
                }
            }
        }
    }
}

fn clamp_scale(f: f32) -> f32 {
    if !f.is_finite() {
        1.0
    } else if f.abs() < MIN_SCALE {
        MIN_SCALE.copysign(if f == 0.0 { 1.0 } else { f })
    } else {
        f
    }
}

#[cfg(test)]
#[path = "transforming_tests.rs"]
mod tests;
