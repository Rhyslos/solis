//! What overlaps a trigger zone in 3D (stage 1, triggers): each zone, a
//! box or a ball placed and turned in the world, asks which colliders of
//! moving things meet it. Characters and dynamic or kinematic bodies
//! count; still things (fixed bodies, colliders with no body, brushes)
//! don't, since they never walk in or out.
//!
//! The question goes to a tree of boxes (parry's BVH) made for the
//! purpose from where the moving things are now, then each candidate is
//! tested exactly. Rapier's own lists of what's near what are brought up
//! to date only by a physics step, so right after physics is built again
//! (an edit while playing, a save, a load) they know nothing: asked then,
//! every zone would see everyone leave and come back. The answers are each
//! zone's entities, sorted, worked out for every zone at once on every
//! core (each zone only reads), so they're the same on any number of
//! cores.

use crate::{State, finite, length, pose};
use glam::{Quat, Vec3};
use rapier3d::parry::bounding_volume::{Aabb, BoundingVolume};
use rapier3d::parry::partitioning::{Bvh, BvhBuildStrategy};
use rapier3d::parry::query::intersection_test;
use rapier3d::prelude as rp;
use rayon::prelude::*;
use reflect::EntityId;

/// A trigger zone's shape in 3D, in metres, already grown with its
/// entity's scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ZoneShape {
    /// A box this size (full width, depth and height).
    Box(Vec3),
    /// A ball of this radius.
    Ball(f32),
}

impl ZoneShape {
    /// Rapier's shape for it; sizes that aren't sensible are made so.
    fn rapier(self) -> rp::SharedShape {
        match self {
            Self::Box(size) => rp::SharedShape::cuboid(
                length(size.x) * 0.5,
                length(size.y) * 0.5,
                length(size.z) * 0.5,
            ),
            Self::Ball(radius) => rp::SharedShape::ball(length(radius)),
        }
    }
}

/// One zone asking what overlaps it: its entity (never counted as inside
/// itself), where it is, which way it's turned, and its shape.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZoneQuery {
    /// The zone's entity.
    pub zone: EntityId,
    /// Its middle, in the world.
    pub place: Vec3,
    /// Which way it's turned.
    pub rotation: Quat,
    /// Its shape.
    pub shape: ZoneShape,
}

/// A moving thing's collider, where it is now.
struct Mover<'a> {
    id: EntityId,
    pose: rp::Pose,
    shape: &'a dyn rp::Shape,
}

impl State {
    /// For each zone, in the order given, the moving things whose
    /// colliders overlap it, by entity, sorted and each once. A zone whose
    /// place isn't sensible overlaps nothing.
    pub fn overlapping(&self, zones: &[ZoneQuery]) -> Vec<Vec<EntityId>> {
        let movers = self.movers();
        if movers.is_empty() {
            return vec![Vec::new(); zones.len()];
        }
        let boxes: Vec<Aabb> = movers
            .iter()
            .map(|m| m.shape.compute_aabb(&m.pose))
            .collect();
        let tree = Bvh::from_leaves(BvhBuildStrategy::Binned, &boxes);
        zones
            .par_iter()
            .map(|zone| overlapping_one(&movers, &tree, zone))
            .collect()
    }

    /// Every character's and dynamic or kinematic body's collider, in
    /// entity order, placed where its body is now (a collider's own
    /// place catches up with its body only at the next step).
    fn movers(&self) -> Vec<Mover<'_>> {
        self.by_entity
            .iter()
            .filter_map(|(id, entry)| {
                let body = self.bodies.get(entry.body)?;
                if body.is_fixed() {
                    return None;
                }
                let collider = self.colliders.get(entry.collider?)?;
                if collider.is_sensor() {
                    return None;
                }
                let offset = collider.position_wrt_parent().copied().unwrap_or_default();
                Some(Mover {
                    id: *id,
                    pose: *body.position() * offset,
                    shape: collider.shape(),
                })
            })
            .collect()
    }
}

/// The movers overlapping one zone, sorted.
fn overlapping_one(movers: &[Mover<'_>], tree: &Bvh, zone: &ZoneQuery) -> Vec<EntityId> {
    if !finite(zone.place, zone.rotation) {
        return Vec::new();
    }
    let shape = zone.shape.rapier();
    let at = pose(zone.place, zone.rotation);
    // Rapier's maths failing on something out of range is nothing found,
    // never a crash.
    let mut ids = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let reach = shape.compute_aabb(&at).loosened(1.0e-4);
        tree.intersect_aabb(&reach)
            .filter_map(|leaf| movers.get(leaf as usize))
            .filter(|m| m.id != zone.zone)
            .filter(|m| {
                intersection_test(&at, &*shape, &m.pose, m.shape).is_ok_and(|hit| hit.intersecting)
            })
            .map(|m| m.id)
            .collect::<Vec<_>>()
    }))
    .unwrap_or_default();
    ids.sort_unstable();
    ids.dedup();
    ids
}

#[cfg(test)]
#[path = "zones_tests.rs"]
mod tests;
