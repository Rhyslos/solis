//! The game's fast storage while it runs
//! ([decision 0024](../../../docs/decisions/0024-game-storage-is-hecs.md)).
//!
//! Layer 1 (engine). The area (`area::Area`) is the document the editor
//! edits: easy to change, undo and save. A [`World`] is built from it when
//! Play is pressed: the same entities in `hecs`, with the built-in
//! components as Rust types and anything else kept as plain values, laid
//! out for a game to run over quickly. Stepping it moves its clock on in
//! fixed ticks and works out where everything is in the world; game code,
//! physics, projectiles and sprites move things on each tick.
//!
//! The world never changes the area: stopping throws it away.

pub mod animators;
mod building;
mod checksum;
mod clock;
pub mod flat_physics;
mod game_set;
mod physics_3d;
mod placing;
pub mod projectiles;
mod screens_and_sounds;
pub mod sprites;
pub mod tile_maps;

pub use animators::Animators;
pub use flat_physics::{FlatBroke, RESTART_WAIT, SolidSets};
pub use projectiles::{MAX_PROJECTILES, Projectile, Projectiles, Shot};
pub use sprites::{ClipSets, SpriteCue};
pub use tile_maps::MapFiles;

use area::builtin::{
    Animator, AudioSource, Camera, CharacterBody, Collider, Environment, Light, Listener, Model,
    ReverbZone, RigidBody, Sprite, Transform,
};
use glam::Mat4;
use reflect::{ComponentData, EntityId};
use std::collections::{BTreeMap, HashMap};

pub use hecs;
pub use physics;

/// How long one tick is, in seconds (60 a second).
pub const TICK: f64 = 1.0 / 60.0;
/// The most ticks one step runs: after a long stall (a breakpoint, a
/// window dragged) the game skips ahead instead of racing to catch up.
pub const MAX_TICKS_PER_STEP: u64 = 8;

/// The fastest the game runs ([`World::set_speed`]): four times.
pub const MAX_SPEED: f64 = 4.0;

/// An entity's ID in the area, on its world entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Id(pub EntityId);

/// An entity's name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Name(pub String);

/// The entity it belongs to, by area ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Parent(pub EntityId);

/// Where an entity is in the area's tree order (its place in `order`), so
/// a query can give entities in that order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Place(usize);

/// Where an entity is in the world: its transform after its parents'.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Global(pub Mat4);

/// Where an entity was in the world when the last tick started (none
/// until a tick has run since it joined), so drawing can show it part way
/// between that tick and the next ([`World::placed_between`]).
#[derive(Debug, Clone, Copy, PartialEq)]
struct Previous(Option<Mat4>);

/// Part way from `before` to `now` (`t` from 0 to 1), for drawing between
/// ticks. Each number moves in a straight line: in a sixtieth of a second
/// things turn too little for that to bend them visibly, and it costs
/// almost nothing.
pub fn between(before: Mat4, now: Mat4, t: f32) -> Mat4 {
    if before == now || t >= 1.0 {
        return now;
    }
    let t = t.max(0.0);
    Mat4::from_cols(
        before.x_axis.lerp(now.x_axis, t),
        before.y_axis.lerp(now.y_axis, t),
        before.z_axis.lerp(now.z_axis, t),
        before.w_axis.lerp(now.w_axis, t),
    )
}

/// Components with no Rust type of their own (game code's, or unknown
/// ones), as plain values by name.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Values(pub BTreeMap<String, ComponentData>);

/// The game's clock.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Clock {
    /// Seconds of game time run, in whole ticks.
    pub time: f64,
    /// Ticks run.
    pub ticks: u64,
    /// Time waiting for the next tick.
    carry: f64,
}

/// The running game's entities.
pub struct World {
    ecs: hecs::World,
    by_id: reflect::IdMap<hecs::Entity>,
    /// Where each entity is in `order`.
    place: reflect::IdMap<usize>,
    /// Every entity, parents before their children (the area's tree
    /// order), with where its parent is in this list.
    order: Vec<(hecs::Entity, Option<usize>)>,
    clock: Clock,
    /// Where in `order` each entity's subtree ends (one past its last
    /// descendant): `order` is depth-first, so a subtree is a run.
    ends: Vec<usize>,
    /// Entities with children whose transform was set since places were
    /// last worked out, by their place in `order` (see
    /// [`World::settle`]).
    unsettled: Vec<usize>,
    /// Each name's first entity in `order`, for [`World::find`].
    names: HashMap<String, usize>,
    /// Goes up whenever the entities, their order or their names change.
    revision: u64,
    /// The checksum of what only changes with the revision (IDs, parents,
    /// names), worked out when first asked for after it changes.
    structure: std::sync::OnceLock<u64>,
    /// Entities the game moved ([`World::set_transform`]), with the
    /// area's Transform for them when it first did: while the area still
    /// has that one, the game's place is kept, even when the world is
    /// built again.
    game_moved: reflect::IdMap<Transform>,
    /// Projectiles in flight: not entities, and kept when the world is
    /// built again.
    projectiles: Projectiles,
    /// The seed the game's random numbers are made from.
    seed: u64,
    /// How fast the game runs (`World:setGameSpeed`; 1 as it starts, 0
    /// paused): game time, physics, projectiles and sprites go at this
    /// share of each tick, while game code still runs every tick.
    speed: f64,
    /// Hours game code moved the day on by (`World:setTimeOfDay`,
    /// decision 0092); 0 as it starts.
    day_shift: f64,
    /// Bodies and colliders (decision 0036): every entity with a
    /// `RigidBody`, a `Collider` or a `CharacterBody`.
    physics: physics::Physics,
    /// Entities with physics game code moved since the last physics step,
    /// whose bodies follow them at the next.
    physics_moved: Vec<EntityId>,
    /// Pushes and velocities game code asked for, in order, made at the
    /// next physics step (so nothing changes physics while game code
    /// reads it on other cores).
    physics_asked: Vec<PhysicsAsk>,
    /// Whether the world is following physics (its own moves then aren't
    /// sent back to physics).
    following: bool,
    /// Whether what's settling was moved by physics (dynamic bodies under
    /// it then stay physics' to move).
    after_physics: bool,
    /// See [`World::changes`]: the last number given, and whether
    /// anything changed since.
    stamp: u64,
    touched: bool,
    /// The clips sprites play (Phase 9, D3), by sheet or clips file.
    clip_sets: std::sync::Arc<ClipSets>,
    /// The animators entities run (Phase 9, D3b), by animator file.
    animators: std::sync::Arc<Animators>,
    /// Parameter values a loaded save gave entities whose animators
    /// haven't started running yet.
    saved_values: HashMap<EntityId, Vec<(String, animation::ParamValue)>>,
    /// The project's tile map files (Phase 9, D4), by asset ID.
    map_files: std::sync::Arc<MapFiles>,
    /// Maps a loaded save gave entities whose maps haven't started yet.
    saved_maps: HashMap<EntityId, tiles::TileMap>,
    /// Goes up whenever a map at play changes (`World::maps_revision`).
    maps_revision: u64,
    /// 2D physics (decision 0071): every entity with a `Collider2D`, and
    /// each tile map's walls.
    flat: physics::flat::Flat,
    /// The project's 2D settings: `Collider2D`'s pixels and tile maps'
    /// cell size.
    pixels: area::pixels::PixelSettings,
    /// Which terrains of each tileset are solid.
    solid_sets: std::sync::Arc<SolidSets>,
    /// What each map's walls were last built from.
    walls_built: HashMap<EntityId, flat_physics::WallsBuilt>,
    /// How 2D physics is kept going after Rapier fails in it.
    flat_recovery: flat_physics::FlatRecovery,
}

/// A change to physics game code asked for, made at the next step.
#[derive(Debug, Clone, Copy, PartialEq)]
enum PhysicsAsk {
    /// A push (kilograms times metres a second).
    Push(EntityId, glam::Vec3),
    /// A new velocity (metres a second).
    Velocity(EntityId, glam::Vec3),
    /// A character's move (metres, this tick).
    Character(EntityId, glam::Vec3),
    /// A 2D character's move on the ground (metres, this tick).
    Flat(EntityId, glam::Vec2),
}

/// The next of a count shared by every world, so numbers never repeat.
fn next_number() -> u64 {
    static NUMBERS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    NUMBERS.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

impl std::fmt::Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("World")
            .field("entities", &self.order.len())
            .field("clock", &self.clock)
            .finish_non_exhaustive()
    }
}

impl World {
    /// A new revision: unique across worlds, so a world built again never
    /// repeats one.
    fn bump_revision(&mut self) {
        self.revision = next_number();
        self.structure = std::sync::OnceLock::new();
        self.touched = true;
    }

    /// A number that changes whenever anything that can be seen does:
    /// where entities are, which there are, the clock or the projectiles
    /// (never repeating, even in a world built again). What's drawn of
    /// the world is worked out again only when it changes.
    pub fn changes(&mut self) -> u64 {
        if self.touched {
            self.touched = false;
            self.stamp = next_number();
        }
        self.stamp
    }

    /// The values of one entity's component called `name` kept as plain
    /// values (as [`Self::values_named`] gives them for every entity).
    pub fn values_named_for(&self, id: EntityId, name: &str) -> Option<ComponentData> {
        let entity = *self.by_id.get(&id)?;
        let values = self.ecs.get::<&Values>(entity).ok()?;
        values.0.get(name).cloned()
    }

    /// The values of an entity's component called `name` as the game has
    /// it now: a built-in kept as its Rust type (its Transform as the game
    /// moved it), or one kept as plain values. What screens read of
    /// Solis's own components (decision 0087).
    pub fn component_data(&self, id: EntityId, name: &str) -> Option<ComponentData> {
        use area::builtin::{Collider2D, TileMap};
        use reflect::Component;
        fn typed<T: Component + hecs::Component + Clone>(
            world: &World,
            id: EntityId,
        ) -> Option<ComponentData> {
            world.get::<T>(id).map(|c| c.to_data())
        }
        match name {
            Transform::NAME => typed::<Transform>(self, id),
            Light::NAME => typed::<Light>(self, id),
            RigidBody::NAME => typed::<RigidBody>(self, id),
            Collider::NAME => typed::<Collider>(self, id),
            Collider2D::NAME => typed::<Collider2D>(self, id),
            CharacterBody::NAME => typed::<CharacterBody>(self, id),
            Model::NAME => typed::<Model>(self, id),
            Environment::NAME => typed::<Environment>(self, id),
            Camera::NAME => typed::<Camera>(self, id),
            AudioSource::NAME => typed::<AudioSource>(self, id),
            ReverbZone::NAME => typed::<ReverbZone>(self, id),
            Listener::NAME => typed::<Listener>(self, id),
            Sprite::NAME => typed::<Sprite>(self, id),
            area::builtin::Parallax::NAME => typed::<area::builtin::Parallax>(self, id),
            Animator::NAME => typed::<Animator>(self, id),
            TileMap::NAME => typed::<TileMap>(self, id),
            _ => self.values_named_for(id, name),
        }
    }

    /// The projectiles in flight.
    pub fn projectiles(&self) -> &Projectiles {
        &self.projectiles
    }

    /// The projectiles, to fire more or move them on a tick
    /// ([`Projectiles::tick`]: the editor does it once per tick, before
    /// game code runs).
    pub fn projectiles_mut(&mut self) -> &mut Projectiles {
        self.touched = true;
        &mut self.projectiles
    }

    /// Changes whenever the entities, their order or their names change
    /// (not when they move), never repeating, even in a world built again:
    /// what [`Self::entities`] gives is the same while it is.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Every entity, with its name, in the area's tree order: entity
    /// number `i` here is number `i` in [`Self::globals`] (`None` only for
    /// an entity somehow without its ID).
    pub fn entities(&self) -> Vec<Option<(EntityId, String)>> {
        self.order
            .iter()
            .map(|&(entity, _)| {
                let e = self.ecs.entity(entity).ok()?;
                let id = e.get::<&Id>()?.0;
                let name = e.get::<&Name>().map(|n| n.0.clone()).unwrap_or_default();
                Some((id, name))
            })
            .collect()
    }

    /// The first entity named `name`, in the area's tree order.
    pub fn find(&self, name: &str) -> Option<EntityId> {
        let &at = self.names.get(name)?;
        let (entity, _) = self.order.get(at)?;
        self.ecs.get::<&Id>(*entity).ok().map(|id| id.0)
    }

    /// How many entities there are.
    pub fn len(&self) -> usize {
        self.order.len()
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }

    /// The storage itself, for systems that run over it.
    pub fn ecs(&self) -> &hecs::World {
        &self.ecs
    }

    /// The world entity for an area entity.
    pub fn entity(&self, id: EntityId) -> Option<hecs::Entity> {
        self.by_id.get(&id).copied()
    }

    /// A copy of the entity's `T` component, if it has one.
    pub fn get<T: hecs::Component + Clone>(&self, id: EntityId) -> Option<T> {
        let entity = self.entity(id)?;
        self.ecs.get::<&T>(entity).ok().map(|c| (*c).clone())
    }

    /// Every entity with a component called `name` kept as plain values
    /// (game code's, and built-ins with no Rust type in the world, such as
    /// a Screen), with its values, in the area's tree order.
    pub fn values_named(&self, name: &str) -> Vec<(EntityId, ComponentData)> {
        let mut found = Vec::new();
        for &(entity, _) in &self.order {
            let Ok(e) = self.ecs.entity(entity) else {
                continue;
            };
            if let (Some(id), Some(values)) = (e.get::<&Id>(), e.get::<&Values>())
                && let Some(data) = values.0.get(name)
            {
                found.push((id.0, data.clone()));
            }
        }
        found
    }

    /// The sky, fog and exposure: the first environment, if any.
    pub fn environment(&self) -> Option<Environment> {
        self.first::<Environment>().map(|(_, e, _)| e)
    }
}

#[cfg(test)]
mod tests;
