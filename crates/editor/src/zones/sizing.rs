//! Sizing a zone with its handles (the mock-up's "drag the handles to
//! size it"): small squares on the picked zone's outline. In a 2D view a
//! box has eight (corners and sides) and a circle four; in a 3D view a
//! box has one on each face and a sphere six. Dragging a box's handle
//! moves that side (or corner) and keeps the far one where it is, so the
//! zone's middle moves too; a round zone's handle sets its radius. A
//! freeform zone is sized by its points instead.

use super::ViewZone;
use crate::camera::Camera;
use crate::screen::Ray;
use glam::{Mat4, Vec2, Vec3};
use level::builtin::{TriggerZone, ZoneForm};
use reflect::EntityId;

/// How close (points) the pointer must be to a handle to take it.
pub(crate) const HANDLE_REACH: f32 = 7.0;

/// One handle: where it is in the zone's own space, and which way it
/// sizes (-1, 0 or 1 along each axis).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Handle {
    /// Where it is, in the zone's own space.
    pub(crate) at: Vec3,
    /// Which way it sizes.
    pub(crate) way: Vec3,
}

/// The ways a 2D view's box handles size: corners, then sides.
const FLAT_BOX: [(f32, f32); 8] = [
    (-1.0, -1.0),
    (1.0, -1.0),
    (1.0, 1.0),
    (-1.0, 1.0),
    (0.0, -1.0),
    (1.0, 0.0),
    (0.0, 1.0),
    (-1.0, 0.0),
];

/// The six ways along the axes.
const AXES: [Vec3; 6] = [
    Vec3::X,
    Vec3::NEG_X,
    Vec3::Y,
    Vec3::NEG_Y,
    Vec3::Z,
    Vec3::NEG_Z,
];

/// A zone's handles (`flat`: in a 2D view). None for a freeform zone.
pub(crate) fn handles(zone: &TriggerZone, flat: bool) -> Vec<Handle> {
    let half = zone.size * 0.5;
    match (zone.form, flat) {
        (ZoneForm::Box, true) => FLAT_BOX
            .iter()
            .map(|&(x, y)| {
                let way = Vec3::new(x, y, 0.0);
                Handle {
                    at: way * half,
                    way,
                }
            })
            .collect(),
        (ZoneForm::Box, false) => AXES
            .iter()
            .map(|&way| Handle {
                at: way * half,
                way,
            })
            .collect(),
        (ZoneForm::Round, _) => AXES
            .iter()
            .take(if flat { 4 } else { 6 })
            .map(|&way| Handle {
                at: way * zone.radius,
                way,
            })
            .collect(),
        (ZoneForm::Freeform, _) => Vec::new(),
    }
}

/// The handle of `zone` within reach of `mouse` (points from the view's
/// corner), nearest first.
pub(crate) fn handle_at(
    zone: &ViewZone,
    flat: bool,
    camera: &Camera,
    size: Vec2,
    mouse: Vec2,
) -> Option<usize> {
    handles(&zone.zone, flat)
        .iter()
        .enumerate()
        .filter_map(|(i, h)| {
            let p = camera.project(zone.transform.transform_point3(h.at), size)?;
            let d = p.distance(mouse);
            (d <= HANDLE_REACH).then_some((d, i))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, i)| i)
}

/// A handle's drag under way.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SizeDrag {
    /// The zone's entity.
    pub(crate) entity: EntityId,
    /// Its settings as the drag began.
    start: TriggerZone,
    /// Where it was in the world as the drag began.
    world: Mat4,
    /// The handle dragged.
    pub(crate) handle: Handle,
    /// Its number, to light it.
    pub(crate) number: usize,
    /// In a 2D view: the pointer is followed on the ground.
    flat: bool,
}

impl SizeDrag {
    /// Starts dragging handle `number` of `zone`.
    pub(crate) fn new(zone: &ViewZone, number: usize, flat: bool) -> Option<Self> {
        let handle = *handles(&zone.zone, flat).get(number)?;
        Some(Self {
            entity: zone.entity,
            start: zone.zone.clone(),
            world: zone.transform,
            handle,
            number,
            flat,
        })
    }

    /// The zone sized for the pointer along `ray`, and where its middle
    /// is in the world then; none when the pointer can't be followed (a
    /// ray along the ground, a zone squashed flat by its scale).
    pub(crate) fn follow(&self, ray: &Ray) -> Option<(TriggerZone, Vec3)> {
        let inverse = self.world.inverse();
        if !inverse.is_finite() {
            return None;
        }
        let local = inverse.transform_point3(self.pointer_in_world(ray)?);
        if !local.is_finite() {
            return None;
        }
        let mut zone = self.start.clone();
        let mut middle = Vec3::ZERO;
        let way = self.handle.way;
        match zone.form {
            ZoneForm::Box => {
                let half = self.start.size * 0.5;
                for axis in 0..3 {
                    if way[axis] == 0.0 {
                        continue;
                    }
                    let far = -way[axis] * half[axis];
                    let size = (way[axis] * (local[axis] - far))
                        .clamp(TriggerZone::MIN_SIZE_M, TriggerZone::MAX_SIZE_M);
                    zone.size[axis] = size;
                    middle[axis] = far + way[axis] * size * 0.5;
                }
            }
            ZoneForm::Round => {
                let reach = if self.flat {
                    local.truncate().length()
                } else {
                    local.dot(way).abs()
                };
                zone.radius = reach.clamp(TriggerZone::MIN_SIZE_M, TriggerZone::MAX_SIZE_M);
            }
            ZoneForm::Freeform => return None,
        }
        Some((zone, self.world.transform_point3(middle)))
    }

    /// Where the pointer is, in the world, as the handle follows it: on
    /// the zone's own ground in a 2D view, else the nearest point of the
    /// line through the handle along its way.
    fn pointer_in_world(&self, ray: &Ray) -> Option<Vec3> {
        let start = self.world.transform_point3(self.handle.at);
        if self.flat {
            return ray.hit_ground(start.z).or_else(|| {
                // A 2D view looks straight down: where the ray is level
                // with the zone.
                (ray.direction.z.abs() < 1e-6).then(|| ray.origin)
            });
        }
        let along = self
            .world
            .transform_vector3(self.handle.way)
            .normalize_or_zero();
        if along == Vec3::ZERO {
            return None;
        }
        // The nearest points of the handle's line and the pointer's ray.
        let between = start - ray.origin;
        let a = along.dot(ray.direction);
        let denominator = 1.0 - a * a;
        if denominator.abs() < 1e-6 {
            return None;
        }
        let t = (a * between.dot(ray.direction) - between.dot(along)) / denominator;
        Some(start + along * t)
    }
}
