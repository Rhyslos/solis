//! 2D physics (decision 0071): Rapier2D in its strict mode, beside the 3D
//! world and walled into this crate the same way. 2D lies on the ground
//! (decision 0062): a place here is Solis's X and Y, and Z (the layer) is
//! left alone.
//!
//! What it holds:
//! - **solid things:** an entity's flat collider (a circle or a box) that
//!   never moves on its own: a house, a shipping bin;
//! - **characters:** an entity's flat collider moved by game code through
//!   [`Flat::move_characters`], which slides it along whatever's in its way;
//! - **walls:** each tile map's solid cells, merged into rectangles chunk by
//!   chunk (`tiles::walls`), replaced chunk by chunk as cells change.
//!
//! The mover is Solis's own, not Rapier's character controller (which is
//! made for side views, with a floor and slopes): a shape cast to what it
//! meets, a small skin back out from it, then a slide along it, a few
//! times over. Kept a skin away, a character sliding along a row of walls never
//! touches the corners where they meet, so it never snags on a seam.
//!
//! Everything is kept in entity order, so the same changes always make the
//! same handles, and a tick's moves are made one after another in the
//! order asked: the same on every PC.

use glam::Vec2;
use rapier2d::parry::query::ShapeCastOptions;
use rapier2d::prelude as rp;
use reflect::EntityId;
use std::collections::BTreeMap;
use std::sync::Arc;

/// How far a character stays from what it slides along, in metres (a
/// tenth of a pixel at 16 pixels a metre).
pub const SKIN: f32 = 0.006;

/// The furthest a character moves in one tick, in metres: a wanted move
/// further is cut short (a mistake in game code, not a teleport).
pub const MAX_FLAT_MOVE: f32 = 4.0;

/// The most rectangles one chunk of one map's walls makes (a chunk is
/// 32 × 32 cells, so a checkerboard is the worst).
pub const MAX_CHUNK_WALLS: usize = 512;

/// How many times a move slides along what it meets before stopping.
const SLIDES: usize = 4;

/// A flat collider's shape (in metres, on the ground).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FlatShape {
    /// A circle of this radius: a character's footprint, which slides
    /// around corners.
    Circle(f32),
    /// A box this size (full width and height).
    Box(Vec2),
}

/// A flat collider (the `Collider2D` component's), placed from its
/// entity's place.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlatCollider {
    /// Its shape.
    pub shape: FlatShape,
    /// Where its middle is, from the entity's place (at the feet, say).
    pub center: Vec2,
}

impl FlatCollider {
    /// Rapier's shape for it; sizes that aren't sensible are made so.
    fn rapier(&self) -> rp::SharedShape {
        let size = |v: f32| if v.is_finite() { v.clamp(0.001, 1000.0) } else { 0.5 };
        match self.shape {
            FlatShape::Circle(r) => rp::SharedShape::ball(size(r)),
            FlatShape::Box(s) => rp::SharedShape::cuboid(size(s.x) * 0.5, size(s.y) * 0.5),
        }
    }

    fn center(&self) -> Vec2 {
        if self.center.is_finite() {
            self.center.clamp(Vec2::splat(-1000.0), Vec2::splat(1000.0))
        } else {
            Vec2::ZERO
        }
    }
}

/// One entity's part in 2D physics.
#[derive(Debug, Clone, Copy)]
struct Entry {
    collider: rp::ColliderHandle,
    spec: FlatCollider,
    /// Moved by game code, sliding (otherwise it stays where it's put).
    character: bool,
}

/// Everything 2D physics knows. Shared (`Arc`) with game code while a
/// tick's phases read it, changed only between them.
#[derive(Clone)]
pub struct FlatState {
    params: rp::IntegrationParameters,
    islands: rp::IslandManager,
    broad_phase: rp::BroadPhaseBvh,
    narrow_phase: rp::NarrowPhase,
    bodies: rp::RigidBodySet,
    colliders: rp::ColliderSet,
    impulse_joints: rp::ImpulseJointSet,
    multibody_joints: rp::MultibodyJointSet,
    soft_bodies: rp::SoftBodySet,
    ccd: rp::CCDSolver,
    by_entity: BTreeMap<EntityId, Entry>,
    /// Each map's walls, by the map's entity and chunk.
    walls: BTreeMap<(EntityId, [u32; 2]), Vec<rp::ColliderHandle>>,
}

impl std::fmt::Debug for FlatState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FlatState")
            .field("entities", &self.by_entity.len())
            .field("colliders", &self.colliders.len())
            .finish_non_exhaustive()
    }
}

impl Default for FlatState {
    fn default() -> Self {
        Self {
            params: rp::IntegrationParameters::default(),
            islands: rp::IslandManager::new(),
            broad_phase: rp::BroadPhaseBvh::new(),
            narrow_phase: rp::NarrowPhase::new(),
            bodies: rp::RigidBodySet::new(),
            colliders: rp::ColliderSet::new(),
            impulse_joints: rp::ImpulseJointSet::new(),
            multibody_joints: rp::MultibodyJointSet::new(),
            soft_bodies: rp::SoftBodySet::default(),
            ccd: rp::CCDSolver::new(),
            by_entity: BTreeMap::new(),
            walls: BTreeMap::new(),
        }
    }
}

fn to_rp(v: Vec2) -> rp::Vector {
    rp::Vector::new(v.x, v.y)
}

fn from_rp(v: rp::Vector) -> Vec2 {
    Vec2::new(v.x, v.y)
}

impl FlatState {
    /// Nothing in 2D physics: no entities and no walls.
    pub fn is_empty(&self) -> bool {
        self.by_entity.is_empty() && self.walls.is_empty()
    }

    /// Whether `id` is in 2D physics.
    pub fn contains(&self, id: EntityId) -> bool {
        self.by_entity.contains_key(&id)
    }

    /// Whether `id` is a character (moved by game code, sliding).
    pub fn is_character(&self, id: EntityId) -> bool {
        self.by_entity.get(&id).is_some_and(|e| e.character)
    }

    /// Where `id` is (its entity's place, not its collider's middle).
    pub fn place(&self, id: EntityId) -> Option<Vec2> {
        let entry = self.by_entity.get(&id)?;
        let c = self.colliders.get(entry.collider)?;
        Some(from_rp(c.translation()) - entry.spec.center())
    }

    /// How many wall rectangles every map makes together.
    pub fn wall_count(&self) -> usize {
        self.walls.values().map(Vec::len).sum()
    }

    /// Every wall rectangle, as its lowest and highest corner, in map and
    /// chunk order (for drawing, and tests).
    pub fn walls(&self) -> Vec<(Vec2, Vec2)> {
        self.walls
            .values()
            .flatten()
            .filter_map(|h| {
                let c = self.colliders.get(*h)?;
                let half = c.shape().as_cuboid()?.half_extents;
                let mid = from_rp(c.translation());
                Some((mid - from_rp(half), mid + from_rp(half)))
            })
            .collect()
    }

    /// A checksum of 2D physics (decision 0037's determinism): every
    /// entity in order with where it is, and how many walls there are.
    pub fn checksum(&self) -> u64 {
        let mut sum = reflect::Checksum::new();
        sum.u64(self.by_entity.len() as u64);
        for (id, entry) in &self.by_entity {
            sum.entity(*id);
            if let Some(c) = self.colliders.get(entry.collider) {
                let t = c.translation();
                sum.f32(t.x);
                sum.f32(t.y);
            }
            sum.u64(u64::from(entry.character));
        }
        sum.u64(self.wall_count() as u64);
        sum
            .finish()
    }

    /// Where the collider `skip`, at `start`, ends moving by `wanted`:
    /// stopping a skin short of what it meets and sliding along it.
    fn slide(&self, skip: rp::ColliderHandle, start: Vec2, wanted: Vec2) -> Vec2 {
        let Some(c) = self.colliders.get(skip) else {
            return start;
        };
        let shape = c.shared_shape().clone();
        let options = ShapeCastOptions {
            max_time_of_impact: 1.0,
            target_distance: 0.0,
            stop_at_penetration: false,
            compute_impact_geometry_on_penetration: true,
        };
        // What's been met and isn't in the way of what's left of the move
        // (slid along already): passed by in the casts after.
        let mut passed: Vec<rp::ColliderHandle> = Vec::new();
        let mut at = start;
        let mut left = wanted;
        for _ in 0..SLIDES * 2 {
            if left.length_squared() < 1.0e-12 {
                break;
            }
            let keep = |h: rp::ColliderHandle, _: &rp::Collider| !passed.contains(&h);
            let filter = rp::QueryFilter::default()
                .exclude_collider(skip)
                .exclude_sensors()
                .predicate(&keep);
            let queries = self.broad_phase.as_query_pipeline(
                self.narrow_phase.query_dispatcher(),
                &self.bodies,
                &self.colliders,
                filter,
            );
            let pose = rp::Pose::from_translation(to_rp(at));
            // Rapier's maths failing on something out of range stops the
            // move, never crashes.
            let Ok(hit) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                queries.cast_shape(&pose, to_rp(left), &*shape, options)
            })) else {
                break;
            };
            let Some((handle, hit)) = hit else {
                at += left;
                break;
            };
            let t = if hit.time_of_impact.is_finite() {
                hit.time_of_impact.clamp(0.0, 1.0)
            } else {
                0.0
            };
            // The hit thing's surface faces the mover: keep only the part
            // of what's left along it.
            let normal = from_rp(hit.normal1).normalize_or_zero();
            let rest = left * (1.0 - t);
            let into = rest.dot(normal);
            if into >= -1.0e-7 {
                // Already a skin away and moving along it or away: it isn't
                // in the way.
                passed.push(handle);
                continue;
            }
            // To where it touches, then a skin back out along the surface's
            // normal: sliding on, it passes the corners where two walls
            // meet a skin away, so a seam never catches it.
            at += left * t + normal * SKIN;
            left = rest - normal * into;
            passed.push(handle);
        }
        if at.is_finite() { at } else { start }
    }
}

/// 2D physics: its [`FlatState`], and Rapier's workspace.
#[derive(Default)]
pub struct Flat {
    state: Arc<FlatState>,
    pipeline: rp::PhysicsPipeline,
    /// Something was added, moved or taken out since Rapier last caught up
    /// (its lists of what's near what).
    stale: bool,
    /// Characters moved since [`Self::moved`] last said.
    moved: Vec<EntityId>,
    /// Entities not added because [`crate::MAX_BODIES`] were there.
    refused: u64,
}

impl std::fmt::Debug for Flat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Flat")
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}

impl Clone for Flat {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
            pipeline: rp::PhysicsPipeline::new(),
            stale: true,
            moved: self.moved.clone(),
            refused: self.refused,
        }
    }
}

impl Flat {
    /// Nothing yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// The state, to read.
    pub fn state(&self) -> &Arc<FlatState> {
        &self.state
    }

    fn state_mut(&mut self) -> &mut FlatState {
        self.stale = true;
        Arc::make_mut(&mut self.state)
    }

    /// Entities not added because there were already [`crate::MAX_BODIES`].
    pub fn refused(&self) -> u64 {
        self.refused
    }

    /// Empties 2D physics, walls too.
    pub fn clear(&mut self) {
        self.state = Arc::new(FlatState::default());
        self.pipeline = rp::PhysicsPipeline::new();
        self.moved.clear();
        self.stale = false;
    }

    /// Puts `id` in 2D physics at `place` with `collider` (again, if it
    /// was in already): a character if `character`, else solid and still.
    /// A place that isn't finite isn't used.
    pub fn add(&mut self, id: EntityId, place: Vec2, collider: FlatCollider, character: bool) {
        if !place.is_finite() {
            return;
        }
        let known = self.state.by_entity.contains_key(&id);
        if !known && self.state.by_entity.len() >= crate::MAX_BODIES {
            self.refused += 1;
            return;
        }
        self.remove(id);
        let state = self.state_mut();
        let made = rp::ColliderBuilder::new(collider.rapier())
            .translation(to_rp(place + collider.center()))
            .user_data(u128::from(id.0.raw()))
            .build();
        let handle = state.colliders.insert(made);
        state.by_entity.insert(
            id,
            Entry {
                collider: handle,
                spec: collider,
                character,
            },
        );
    }

    /// Takes `id` out of 2D physics.
    pub fn remove(&mut self, id: EntityId) {
        if !self.state.by_entity.contains_key(&id) {
            return;
        }
        let state = self.state_mut();
        if let Some(entry) = state.by_entity.remove(&id) {
            state.colliders.remove(
                entry.collider,
                &mut state.islands,
                &mut state.bodies,
                &mut state.soft_bodies,
                false,
            );
        }
    }

    /// Puts `id` at `place` (game code or an edit moved it), without
    /// sliding.
    pub fn set_place(&mut self, id: EntityId, place: Vec2) {
        if !place.is_finite() || !self.state.by_entity.contains_key(&id) {
            return;
        }
        let state = self.state_mut();
        let Some(entry) = state.by_entity.get(&id).copied() else {
            return;
        };
        if let Some(c) = state.colliders.get_mut(entry.collider) {
            c.set_translation(to_rp(place + entry.spec.center()));
        }
    }

    /// Makes chunk `chunk` of map `map`'s walls `rects` (each its lowest
    /// and highest corner, in metres), replacing the chunk's old ones; no
    /// rects takes them away. At most [`MAX_CHUNK_WALLS`] are made.
    pub fn set_walls(&mut self, map: EntityId, chunk: [u32; 2], rects: &[(Vec2, Vec2)]) {
        let had = self.state.walls.contains_key(&(map, chunk));
        if !had && rects.is_empty() {
            return;
        }
        let state = self.state_mut();
        for h in state.walls.remove(&(map, chunk)).unwrap_or_default() {
            state
                .colliders
                .remove(h, &mut state.islands, &mut state.bodies, &mut state.soft_bodies, false);
        }
        let made: Vec<rp::ColliderHandle> = rects
            .iter()
            .filter(|(lo, hi)| lo.is_finite() && hi.is_finite() && hi.x > lo.x && hi.y > lo.y)
            .take(MAX_CHUNK_WALLS)
            .map(|(lo, hi)| {
                let half = (*hi - *lo) * 0.5;
                let wall = rp::ColliderBuilder::cuboid(half.x, half.y)
                    .translation(to_rp(*lo + half))
                    .user_data(u128::from(map.0.raw()))
                    .build();
                state.colliders.insert(wall)
            })
            .collect();
        if !made.is_empty() {
            state.walls.insert((map, chunk), made);
        }
    }

    /// Takes away every wall of map `map`.
    pub fn remove_walls(&mut self, map: EntityId) {
        let chunks: Vec<[u32; 2]> = self
            .state
            .walls
            .range((map, [0, 0])..=(map, [u32::MAX, u32::MAX]))
            .map(|((_, c), _)| *c)
            .collect();
        for chunk in chunks {
            self.set_walls(map, chunk, &[]);
        }
    }

    /// The maps with walls.
    pub fn maps_with_walls(&self) -> Vec<EntityId> {
        let mut maps: Vec<EntityId> = self.state.walls.keys().map(|(m, _)| *m).collect();
        maps.dedup();
        maps
    }

    /// Rapier catches up with what changed (its lists of what's near
    /// what), so the next casts see it.
    fn catch_up(&mut self) {
        if !self.stale {
            return;
        }
        self.stale = false;
        let Self {
            state, pipeline, ..
        } = &mut *self;
        let state = Arc::make_mut(state);
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            pipeline.step(
                rp::Vector::ZERO,
                &state.params,
                &mut state.islands,
                &mut state.broad_phase,
                &mut state.narrow_phase,
                &mut state.bodies,
                &mut state.colliders,
                &mut state.impulse_joints,
                &mut state.multibody_joints,
                &mut state.soft_bodies,
                &mut state.ccd,
                &(),
                &(),
            );
        }));
        if caught.is_err() {
            // Never a crash: 2D physics empties, as 3D's does.
            self.clear();
        }
    }

    /// Moves each character by its wanted distance (metres, on the
    /// ground), in the order given, each as far as it can: it stops a
    /// skin short of what's in its way and slides along it. Others than
    /// characters, and wanted moves that aren't finite, are left alone.
    /// A move further than [`MAX_FLAT_MOVE`] is cut short.
    pub fn move_characters(&mut self, moves: &[(EntityId, Vec2)]) {
        for (id, wanted) in moves {
            let Some(entry) = self.state.by_entity.get(id).copied() else {
                continue;
            };
            if !entry.character || !wanted.is_finite() {
                continue;
            }
            self.catch_up();
            let Some(start) = self
                .state
                .colliders
                .get(entry.collider)
                .map(|c| from_rp(c.translation()))
            else {
                continue;
            };
            // Huge moves are cut short without overflowing.
            let biggest = wanted.abs().max_element();
            let wanted = if biggest > MAX_FLAT_MOVE {
                (wanted / biggest).clamp_length_max(1.0) * MAX_FLAT_MOVE
            } else {
                wanted.clamp_length_max(MAX_FLAT_MOVE)
            };
            let end = self.state.slide(entry.collider, start, wanted);
            if end != start {
                let state = self.state_mut();
                if let Some(c) = state.colliders.get_mut(entry.collider) {
                    c.set_translation(to_rp(end));
                }
                self.moved.push(*id);
            }
        }
        self.catch_up();
    }

    /// The characters moved since last asked, in entity order, with where
    /// each is now: for the world to follow.
    pub fn moved(&mut self) -> Vec<(EntityId, Vec2)> {
        let mut ids = std::mem::take(&mut self.moved);
        ids.sort_unstable();
        ids.dedup();
        ids.into_iter()
            .filter_map(|id| Some((id, self.state.place(id)?)))
            .collect()
    }
}

#[cfg(test)]
#[path = "flat_tests.rs"]
mod tests;
