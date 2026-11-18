//! What overlaps a trigger zone in 2D (stage 1, triggers): each zone, a
//! box, a circle or a freeform outline's convex pieces, placed and turned
//! on the ground, asks which characters meet it. Only characters count:
//! the rest of 2D physics (houses, bins, walls) never moves, so it never
//! walks in or out. As in 3D (`zones.rs`), the question goes to a tree of
//! boxes made for it from where the characters are now, so it's never a
//! step behind, then each candidate is tested exactly.
//!
//! A zone's shape is made once ([`FlatZone`]), when the zone or its
//! entity's scale changes, never each tick: a freeform outline is cut into
//! convex pieces then. The answers are each zone's entities, sorted,
//! worked out for every zone at once on every core, so they're the same on
//! any number of cores.

use super::{Flat, FlatState};
use glam::Vec2;
use rapier2d::parry::bounding_volume::{Aabb, BoundingVolume};
use rapier2d::parry::partitioning::{Bvh, BvhBuildStrategy};
use rapier2d::parry::query::intersection_test;
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
        let movers: Vec<(EntityId, &rp::Collider)> = self
            .by_entity
            .iter()
            .filter(|(_, e)| e.character)
            .filter_map(|(id, e)| Some((*id, self.colliders.get(e.collider)?)))
            .collect();
        if movers.is_empty() {
            return vec![Vec::new(); zones.len()];
        }
        let boxes: Vec<Aabb> = movers.iter().map(|(_, c)| c.compute_aabb()).collect();
        let tree = Bvh::from_leaves(BvhBuildStrategy::Binned, &boxes);
        zones
            .par_iter()
            .map(|zone| overlapping_one(&movers, &tree, zone))
            .collect()
    }
}

/// The characters overlapping one zone, sorted.
fn overlapping_one(
    movers: &[(EntityId, &rp::Collider)],
    tree: &Bvh,
    zone: &FlatZoneQuery<'_>,
) -> Vec<EntityId> {
    if !zone.place.is_finite() || !zone.angle.is_finite() {
        return Vec::new();
    }
    let at = rp::Pose::new(zone.place, zone.angle);
    let mut ids: Vec<EntityId> = Vec::new();
    for piece in &zone.shape.pieces {
        // Rapier's maths failing on something out of range is nothing
        // found, never a crash.
        let found = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let reach = piece.compute_aabb(&at).loosened(1.0e-4);
            tree.intersect_aabb(&reach)
                .filter_map(|leaf| movers.get(leaf as usize))
                .filter(|(id, _)| *id != zone.zone)
                .filter(|(_, c)| {
                    intersection_test(&at, &**piece, c.position(), c.shape())
                        .is_ok_and(|hit| hit.intersecting)
                })
                .map(|(id, _)| *id)
                .collect::<Vec<_>>()
        }))
        .unwrap_or_default();
        ids.extend(found);
    }
    ids.sort_unstable();
    ids.dedup();
    ids
}

impl Flat {
    /// For each zone, in the order given, the characters whose colliders
    /// overlap it where they are now, sorted.
    pub fn overlapping(&self, zones: &[FlatZoneQuery<'_>]) -> Vec<Vec<EntityId>> {
        self.state.overlapping(zones)
    }
}
