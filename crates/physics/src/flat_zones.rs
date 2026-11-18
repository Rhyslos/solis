//! What overlaps a trigger zone in 2D (stage 1, triggers): each zone, a
//! box, a circle or a freeform outline's convex pieces, placed and turned
//! on the ground, asks the query pipeline which characters meet it. Only
//! characters count: the rest of 2D physics (houses, bins, walls) never
//! moves, so it never walks in or out.
//!
//! A zone's shape is made once ([`FlatZone`]), when the zone or its
//! entity's scale changes, never each tick: a freeform outline is cut into
//! convex pieces then. The answers are each zone's entities, sorted,
//! worked out for every zone at once on every core, so they're the same on
//! any number of cores.

use super::{Flat, FlatState};
use glam::Vec2;
use rapier2d::prelude as rp;
use rayon::prelude::*;
use reflect::EntityId;

/// The furthest from the middle a zone's size or point may reach, in
/// metres.
const MAX_REACH: f32 = 100_000.0;

/// A trigger zone's shape on the ground, made ready to ask physics with:
/// a box, a circle, or the convex pieces of a freeform outline.
#[derive(Clone)]
pub struct FlatZone {
    pieces: Vec<rp::SharedShape>,
}

impl std::fmt::Debug for FlatZone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FlatZone")
            .field("pieces", &self.pieces.len())
            .finish()
    }
}

/// A length a zone can use: at least a millimetre, within reach.
fn length(v: f32) -> f32 {
    if v.is_finite() {
        v.abs().clamp(0.001, MAX_REACH)
    } else {
        0.001
    }
}

impl FlatZone {
    /// A box this size (full width and height).
    pub fn rectangle(size: Vec2) -> Self {
        Self {
            pieces: vec![rp::SharedShape::cuboid(
                length(size.x) * 0.5,
                length(size.y) * 0.5,
            )],
        }
    }

    /// A circle of this radius.
    pub fn circle(radius: f32) -> Self {
        Self {
            pieces: vec![rp::SharedShape::ball(length(radius))],
        }
    }

    /// Convex pieces (each counter-clockwise, as `mesh::outline` cuts
    /// them), together the zone's shape. A piece that isn't usable (fewer
    /// than three points, or none that are numbers within reach) is left
    /// out.
    pub fn pieces(pieces: &[Vec<Vec2>]) -> Self {
        Self {
            pieces: pieces
                .iter()
                .filter(|piece| {
                    piece
                        .iter()
                        .all(|p| p.is_finite() && p.abs().max_element() <= MAX_REACH)
                })
                .filter_map(|piece| rp::SharedShape::convex_hull(piece))
                .collect(),
        }
    }

    /// How many convex pieces it is made of (0: it overlaps nothing).
    pub fn piece_count(&self) -> usize {
        self.pieces.len()
    }
}

/// One zone asking what overlaps it: its entity, where its middle is on
/// the ground, how far it's turned (radians, anticlockwise) and its
/// shape.
#[derive(Debug, Clone, Copy)]
pub struct FlatZoneQuery<'a> {
    /// The zone's entity (never counted as inside itself).
    pub zone: EntityId,
    /// Where its middle is.
    pub place: Vec2,
    /// How far it's turned, radians anticlockwise.
    pub angle: f32,
    /// Its shape.
    pub shape: &'a FlatZone,
}

impl FlatState {
    /// For each zone, in the order given, the characters whose colliders
    /// overlap it, by entity, sorted and each once.
    fn overlapping(&self, zones: &[FlatZoneQuery<'_>]) -> Vec<Vec<EntityId>> {
        zones
            .par_iter()
            .map(|zone| self.overlapping_one(zone))
            .collect()
    }

    fn overlapping_one(&self, zone: &FlatZoneQuery<'_>) -> Vec<EntityId> {
        if !zone.place.is_finite() || !zone.angle.is_finite() || self.by_entity.is_empty() {
            return Vec::new();
        }
        let character = |handle: rp::ColliderHandle, c: &rp::Collider| {
            reflect::Id::from_raw(c.user_data as u64)
                .map(EntityId)
                .and_then(|id| self.by_entity.get(&id))
                .is_some_and(|e| e.character && e.collider == handle)
        };
        let filter = rp::QueryFilter::default()
            .exclude_sensors()
            .predicate(&character);
        let queries = self.broad_phase.as_query_pipeline(
            self.narrow_phase.query_dispatcher(),
            &self.bodies,
            &self.colliders,
            filter,
        );
        let pose = rp::Pose::new(zone.place, zone.angle);
        let mut ids: Vec<EntityId> = Vec::new();
        for piece in &zone.shape.pieces {
            // Rapier's maths failing on something out of range is nothing
            // found, never a crash.
            let found = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                queries
                    .intersect_shape(pose, &**piece)
                    .filter_map(|(_, c)| reflect::Id::from_raw(c.user_data as u64))
                    .map(EntityId)
                    .collect::<Vec<_>>()
            }))
            .unwrap_or_default();
            ids.extend(found);
        }
        ids.retain(|id| *id != zone.zone);
        ids.sort_unstable();
        ids.dedup();
        ids
    }
}

impl Flat {
    /// For each zone, in the order given, the characters whose colliders
    /// overlap it, sorted. Rapier first catches up with anything added or
    /// moved since its lists of what's near what were made, so a character
    /// put somewhere (or just joined) is found where it is.
    pub fn overlapping(&mut self, zones: &[FlatZoneQuery<'_>]) -> Vec<Vec<EntityId>> {
        self.catch_up(None);
        self.state.overlapping(zones)
    }
}
