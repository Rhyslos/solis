//! What overlaps a trigger zone in 3D (stage 1, triggers): each zone, a
//! box or a ball placed and turned in the world, asks the query pipeline
//! which colliders of moving things meet it. Characters and dynamic or
//! kinematic bodies count; still things (fixed bodies, colliders with no
//! body, brushes) don't, since they never walk in or out.
//!
//! The answers are each zone's entities, sorted, worked out for every zone
//! at once on every core (each zone only reads physics), so they're the
//! same on any number of cores.

use crate::{State, finite, length, pose};
use glam::{Quat, Vec3};
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

impl State {
    /// For each zone, in the order given, the moving things whose
    /// colliders overlap it, by entity, sorted and each once. A zone whose
    /// place isn't sensible overlaps nothing.
    pub fn overlapping(&self, zones: &[ZoneQuery]) -> Vec<Vec<EntityId>> {
        zones
            .par_iter()
            .map(|zone| self.overlapping_one(zone))
            .collect()
    }

    fn overlapping_one(&self, zone: &ZoneQuery) -> Vec<EntityId> {
        if !finite(zone.place, zone.rotation) || self.by_entity.is_empty() {
            return Vec::new();
        }
        let shape = zone.shape.rapier();
        let filter = rp::QueryFilter::exclude_fixed().exclude_sensors();
        let queries = self.broad_phase.as_query_pipeline(
            self.narrow_phase.query_dispatcher(),
            &self.bodies,
            &self.colliders,
            filter,
        );
        // Rapier's maths failing on something out of range is nothing
        // found, never a crash.
        let found = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            queries
                .intersect_shape(pose(zone.place, zone.rotation), &*shape)
                .map(|(handle, _)| handle)
                .collect::<Vec<_>>()
        }))
        .unwrap_or_default();
        let mut ids: Vec<EntityId> = found
            .into_iter()
            .filter_map(|handle| self.entity_of(handle))
            .filter(|id| *id != zone.zone)
            .collect();
        ids.sort_unstable();
        ids.dedup();
        ids
    }
}

#[cfg(test)]
#[path = "zones_tests.rs"]
mod tests;
