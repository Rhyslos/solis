//! The game's physics ([decision 0036](../../../docs/decisions/0036-physics-is-rapier-strict-on-every-pc.md)):
//! Rapier in its strict mode, so the same game gives exactly the same
//! result on every PC. Bodies and colliders belong to entities, by their
//! area IDs; places and directions are Solis's (`glam`, Z up), turned into
//! Rapier's at this crate's edge, so games never meet Rapier itself.
//!
//! The state ([`State`]) is shared (`Arc`) with game code while a tick's
//! phases read it on every core, and changed only between phases, when
//! no one else holds it: [`Physics::step`] and the changes queued before
//! it. Everything is kept in entity order, never a hash map's, so the
//! same changes always make the same handles.

use glam::{Quat, Vec3};
use rapier3d::prelude as rp;
use reflect::EntityId;
use std::collections::BTreeMap;
use std::sync::Arc;

/// The most bodies (and the most entities with colliders) at once; more
/// aren't made, and [`Physics::refused`] counts them.
pub const MAX_BODIES: usize = 100_000;

/// Gravity, metres a second each second, down Solis's Z.
pub const GRAVITY: f32 = -9.81;

/// How a body moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BodyKind {
    /// Moved by physics: falls, is pushed, bounces.
    #[default]
    Dynamic,
    /// Moved by game code (`moveTo`), pushing dynamic bodies out of its
    /// way but never pushed itself: platforms, doors, characters.
    Kinematic,
    /// Never moves: floors, walls.
    Fixed,
}

/// A body's settings (the `RigidBody` component's).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Body {
    /// How it moves.
    pub kind: BodyKind,
    /// Its mass in kilograms; 0 works it out from its colliders' size.
    pub mass: f32,
    /// How quickly it slows down moving (0 never, 1 a lot).
    pub linear_damping: f32,
    /// How quickly it slows down turning.
    pub angular_damping: f32,
    /// How much gravity pulls it (1 normal, 0 floats).
    pub gravity: f32,
    /// Never tips over or spins (characters, upright things).
    pub upright: bool,
    /// Checked between ticks too, so it can't pass through thin walls
    /// when fast (costs more).
    pub fast: bool,
}

impl Default for Body {
    fn default() -> Self {
        Self {
            kind: BodyKind::Dynamic,
            mass: 0.0,
            linear_damping: 0.0,
            angular_damping: 0.05,
            gravity: 1.0,
            upright: false,
            fast: false,
        }
    }
}

/// A collider's shape, in the entity's own space before its scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Shape {
    /// A box this size (full widths, metres).
    Box(Vec3),
    /// A ball of this radius.
    Sphere(f32),
    /// A capsule standing along Z: this radius, and this height overall.
    Capsule {
        /// Its radius.
        radius: f32,
        /// Its height, end to end.
        height: f32,
    },
}

/// A collider's settings (the `Collider` component's).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColliderSpec {
    /// Its shape.
    pub shape: Shape,
    /// Where its middle is, from the entity's origin.
    pub center: Vec3,
    /// How much it grips (0 ice, 1 rubber).
    pub friction: f32,
    /// How much it bounces (0 not at all, 1 fully).
    pub bounciness: f32,
}

impl Default for ColliderSpec {
    fn default() -> Self {
        Self {
            shape: Shape::Box(Vec3::ONE),
            center: Vec3::ZERO,
            friction: 0.5,
            bounciness: 0.0,
        }
    }
}

/// What a ray hit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hit {
    /// The entity whose collider it hit.
    pub entity: EntityId,
    /// How far along the ray, in metres.
    pub distance: f32,
    /// Where.
    pub point: Vec3,
    /// Which way the surface faces there.
    pub normal: Vec3,
}

/// One entity's part in physics.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Entry {
    body: rp::RigidBodyHandle,
    collider: Option<rp::ColliderHandle>,
}

/// Everything physics knows: bodies, colliders and Rapier's own state.
/// Shared with game code during a tick's phases (read only), saved with
/// a game.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct State {
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
}

impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("State")
            .field("bodies", &self.bodies.len())
            .field("colliders", &self.colliders.len())
            .finish_non_exhaustive()
    }
}

impl Default for State {
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
        }
    }
}

/// Solis's vector as Rapier's.
fn to_rp(v: Vec3) -> rp::Vector {
    rp::Vector::new(v.x, v.y, v.z)
}

/// Rapier's vector as Solis's.
fn from_rp(v: rp::Vector) -> Vec3 {
    Vec3::new(v.x, v.y, v.z)
}

/// A place and facing as Rapier's pose.
fn pose(position: Vec3, rotation: Quat) -> rp::Pose {
    let r = rotation.normalize();
    rp::Pose::from_parts(
        to_rp(position),
        rp::Rotation::from_xyzw(r.x, r.y, r.z, r.w),
    )
}

/// Rapier's pose as a place and facing.
fn from_pose(p: &rp::Pose) -> (Vec3, Quat) {
    let r = p.rotation;
    (
        from_rp(p.translation),
        Quat::from_xyzw(r.x, r.y, r.z, r.w),
    )
}

/// Whether every number is sensible: Rapier is never given NaN.
fn finite(position: Vec3, rotation: Quat) -> bool {
    position.is_finite() && rotation.is_finite() && rotation.length_squared() > 1e-12
}

/// A length that's usable: positive and finite, at least a millimetre.
fn length(value: f32) -> f32 {
    if value.is_finite() {
        value.abs().max(0.001)
    } else {
        0.001
    }
}

impl State {
    /// The entity a collider belongs to.
    fn entity_of(&self, collider: rp::ColliderHandle) -> Option<EntityId> {
        let data = self.colliders.get(collider)?.user_data;
        reflect::Id::from_raw(data as u64).map(EntityId)
    }

    /// How many entities take part.
    pub fn len(&self) -> usize {
        self.by_entity.len()
    }

    /// Whether none do.
    pub fn is_empty(&self) -> bool {
        self.by_entity.is_empty()
    }

    /// Whether `id` takes part.
    pub fn contains(&self, id: EntityId) -> bool {
        self.by_entity.contains_key(&id)
    }

    /// How fast `id`'s body moves, metres a second.
    pub fn velocity(&self, id: EntityId) -> Option<Vec3> {
        let entry = self.by_entity.get(&id)?;
        self.bodies.get(entry.body).map(|b| from_rp(b.linvel()))
    }

    /// Where `id`'s body is and which way it faces.
    pub fn pose(&self, id: EntityId) -> Option<(Vec3, Quat)> {
        let entry = self.by_entity.get(&id)?;
        self.bodies.get(entry.body).map(|b| from_pose(b.position()))
    }

    /// The first collider a ray from `from` heading `direction` meets
    /// within `reach` metres. Colliders the ray starts inside are passed
    /// through (a character casting from its own middle doesn't hit
    /// itself).
    pub fn cast_ray(&self, from: Vec3, direction: Vec3, reach: f32) -> Option<Hit> {
        let direction = direction.normalize_or_zero();
        if !(from.is_finite() && reach.is_finite()) || reach <= 0.0 || direction == Vec3::ZERO {
            return None;
        }
        let start = to_rp(from);
        let inside = |_: rp::ColliderHandle, c: &rp::Collider| {
            !c.shape().contains_point(c.position(), start)
        };
        let filter = rp::QueryFilter::default().predicate(&inside);
        let queries = self.broad_phase.as_query_pipeline(
            self.narrow_phase.query_dispatcher(),
            &self.bodies,
            &self.colliders,
            filter,
        );
        let ray = rp::Ray::new(start, to_rp(direction));
        let (handle, hit) = queries.cast_ray_and_get_normal(&ray, reach.min(1.0e7), true)?;
        Some(Hit {
            entity: self.entity_of(handle)?,
            distance: hit.time_of_impact,
            point: from + direction * hit.time_of_impact,
            normal: from_rp(hit.normal),
        })
    }

    /// A checksum of physics (Phase 6, determinism): every body in entity
    /// order with where it is, how it moves, and whether it sleeps.
    pub fn checksum(&self) -> u64 {
        let mut sum = reflect::Checksum::new();
        sum.u64(self.by_entity.len() as u64);
        for (id, entry) in &self.by_entity {
            sum.entity(*id);
            if let Some(body) = self.bodies.get(entry.body) {
                let (p, r) = from_pose(body.position());
                for v in p.to_array().into_iter().chain(r.to_array()) {
                    sum.f32(v);
                }
                for v in from_rp(body.linvel()).to_array() {
                    sum.f32(v);
                }
                let w = body.angvel();
                for v in [w.x, w.y, w.z] {
                    sum.f32(v);
                }
                sum.u64(u64::from(body.is_sleeping()));
            }
        }
        sum.finish()
    }
}

/// The game's physics: its [`State`], and Rapier's workspace for stepping
/// it.
#[derive(Default)]
pub struct Physics {
    state: Arc<State>,
    pipeline: rp::PhysicsPipeline,
    /// Bodies not made because [`MAX_BODIES`] were already there.
    refused: u64,
}

impl std::fmt::Debug for Physics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Physics")
            .field("state", &self.state)
            .field("refused", &self.refused)
            .finish_non_exhaustive()
    }
}

impl Clone for Physics {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
            pipeline: rp::PhysicsPipeline::new(),
            refused: self.refused,
        }
    }
}

impl Physics {
    /// No bodies yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// The state, to read (shared: game code reads it on every core).
    pub fn state(&self) -> &Arc<State> {
        &self.state
    }

    /// The state to change: shared with no one between phases, so
    /// usually without a copy.
    fn state_mut(&mut self) -> &mut State {
        Arc::make_mut(&mut self.state)
    }

    /// Bodies not made because there were already [`MAX_BODIES`].
    pub fn refused(&self) -> u64 {
        self.refused
    }

    /// Makes `id` part of physics, at `position` facing `rotation`,
    /// `scale` times the size its collider says: with a body if it has
    /// one (otherwise it never moves), and a collider if it has one.
    /// Replaces what it had. Returns whether it was made (not past
    /// [`MAX_BODIES`], nor with numbers that aren't sensible).
    pub fn add(
        &mut self,
        id: EntityId,
        (position, rotation): (Vec3, Quat),
        scale: Vec3,
        body: Option<Body>,
        collider: Option<ColliderSpec>,
    ) -> bool {
        self.remove(id);
        if !finite(position, rotation) || !scale.is_finite() {
            return false;
        }
        if self.state.by_entity.len() >= MAX_BODIES {
            self.refused += 1;
            return false;
        }
        let state = self.state_mut();
        let spec = body.unwrap_or(Body {
            kind: BodyKind::Fixed,
            ..Body::default()
        });
        let builder = match spec.kind {
            BodyKind::Dynamic => rp::RigidBodyBuilder::dynamic(),
            BodyKind::Kinematic => rp::RigidBodyBuilder::kinematic_position_based(),
            BodyKind::Fixed => rp::RigidBodyBuilder::fixed(),
        };
        let mut builder = builder
            .pose(pose(position, rotation))
            .linear_damping(sensible(spec.linear_damping, 0.0, 100.0))
            .angular_damping(sensible(spec.angular_damping, 0.0, 100.0))
            .gravity_scale(sensible(spec.gravity, -100.0, 100.0))
            .ccd_enabled(spec.fast)
            .user_data(u128::from(id.0.raw()));
        if spec.upright {
            builder = builder.lock_rotations();
        }
        let handle = state.bodies.insert(builder);
        let collider = collider.map(|c| {
            let s = scale.abs();
            let shape = match c.shape {
                Shape::Box(size) => {
                    let half = size * s * 0.5;
                    rp::ColliderBuilder::cuboid(length(half.x), length(half.y), length(half.z))
                }
                Shape::Sphere(r) => rp::ColliderBuilder::ball(length(r * s.max_element())),
                Shape::Capsule { radius, height } => {
                    let radius = length(radius * s.x.max(s.y));
                    let half = (length(height * s.z) * 0.5 - radius).max(0.0);
                    rp::ColliderBuilder::capsule_z(half, radius)
                }
            };
            let center = if c.center.is_finite() {
                c.center * s
            } else {
                Vec3::ZERO
            };
            let mut builder = shape
                .translation(to_rp(center))
                .friction(sensible(c.friction, 0.0, 10.0))
                .restitution(sensible(c.bounciness, 0.0, 1.0))
                .user_data(u128::from(id.0.raw()));
            if spec.mass > 0.0 && spec.mass.is_finite() {
                builder = builder.mass(spec.mass.min(1.0e9));
            }
            state
                .colliders
                .insert_with_parent(builder, handle, &mut state.bodies)
        });
        state.by_entity.insert(
            id,
            Entry {
                body: handle,
                collider,
            },
        );
        true
    }

    /// Takes `id` out of physics, if it's in.
    pub fn remove(&mut self, id: EntityId) {
        if !self.state.by_entity.contains_key(&id) {
            return;
        }
        let state = self.state_mut();
        if let Some(entry) = state.by_entity.remove(&id) {
            state.bodies.remove(
                entry.body,
                &mut state.islands,
                &mut state.colliders,
                &mut state.impulse_joints,
                &mut state.multibody_joints,
                &mut state.soft_bodies,
                true,
            );
        }
    }

    /// Puts `id`'s body at `position` facing `rotation` (game code moved
    /// it): a kinematic one moves there over the next step, pushing what's
    /// in its way; others are put there at once.
    pub fn set_pose(&mut self, id: EntityId, position: Vec3, rotation: Quat) {
        if !finite(position, rotation) {
            return;
        }
        let Some(handle) = self.state.by_entity.get(&id).map(|e| e.body) else {
            return;
        };
        let state = self.state_mut();
        if let Some(body) = state.bodies.get_mut(handle) {
            let to = pose(position, rotation);
            if body.is_kinematic() {
                body.set_next_kinematic_position(to);
            } else {
                body.set_position(to, true);
            }
        }
    }

    /// Sets how fast `id`'s body moves.
    pub fn set_velocity(&mut self, id: EntityId, velocity: Vec3) {
        if !velocity.is_finite() {
            return;
        }
        let Some(handle) = self.state.by_entity.get(&id).map(|e| e.body) else {
            return;
        };
        if let Some(body) = self.state_mut().bodies.get_mut(handle) {
            body.set_linvel(to_rp(velocity), true);
        }
    }

    /// Gives `id`'s body a push: a change in its momentum (kilograms times
    /// metres a second), so a heavy body moves less than a light one.
    pub fn push(&mut self, id: EntityId, impulse: Vec3) {
        if !impulse.is_finite() {
            return;
        }
        let Some(handle) = self.state.by_entity.get(&id).map(|e| e.body) else {
            return;
        };
        if let Some(body) = self.state_mut().bodies.get_mut(handle) {
            body.apply_impulse(to_rp(impulse), true);
        }
    }

    /// Moves physics on by `dt` seconds (one tick).
    pub fn step(&mut self, dt: f32) {
        if self.state.by_entity.is_empty() || !(dt.is_finite() && dt > 0.0) {
            return;
        }
        let Self {
            state, pipeline, ..
        } = self;
        let state = Arc::make_mut(state);
        state.params.dt = dt;
        let gravity = rp::Vector::new(0.0, 0.0, GRAVITY);
        pipeline.step(
            gravity,
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
    }

    /// The bodies physics moves (dynamic ones, awake), in entity order,
    /// with where each is now: for the world to follow after a step.
    pub fn moved(&self) -> Vec<(EntityId, Vec3, Quat)> {
        self.state
            .by_entity
            .iter()
            .filter_map(|(id, entry)| {
                let body = self.state.bodies.get(entry.body)?;
                (body.is_dynamic() && !body.is_sleeping()).then(|| {
                    let (p, r) = from_pose(body.position());
                    (*id, p, r)
                })
            })
            .collect()
    }
}

/// `value` within `low` to `high`; `low` if it isn't a number.
fn sensible(value: f32, low: f32, high: f32) -> f32 {
    if value.is_finite() {
        value.clamp(low, high)
    } else {
        low.max(0.0).min(high)
    }
}

#[cfg(test)]
mod tests;
