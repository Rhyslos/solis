//! The game's fast storage while it runs
//! ([decision 0024](../../../docs/decisions/0024-game-storage-is-hecs.md)).
//!
//! Layer 1 (engine). The area (`area::Area`) is the document the editor
//! edits: easy to change, undo and save. A [`World`] is built from it when
//! Play is pressed: the same entities in `hecs`, with the built-in
//! components as Rust types and anything else kept as plain values, laid
//! out for a game to run over quickly. Stepping it moves its clock on in
//! fixed ticks and works out where everything is in the world; game code
//! joins the ticks in Phase 4.
//!
//! The world never changes the area: stopping throws it away.

use area::Area;
use area::builtin::{Camera, Environment, Light, Model, Transform};
use glam::Mat4;
use reflect::{Component, ComponentData, EntityId};
use std::collections::{BTreeMap, HashMap};

pub use hecs;

/// How long one tick is, in seconds (60 a second).
pub const TICK: f64 = 1.0 / 60.0;
/// The most ticks one step runs: after a long stall (a breakpoint, a
/// window dragged) the game skips ahead instead of racing to catch up.
pub const MAX_TICKS_PER_STEP: u64 = 8;

/// An entity's ID in the area, on its world entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Id(pub EntityId);

/// An entity's name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Name(pub String);

/// The entity it belongs to, by area ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Parent(pub EntityId);

/// Where an entity is in the world: its transform after its parents'.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Global(pub Mat4);

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
    by_id: HashMap<EntityId, hecs::Entity>,
    /// Every entity, parents before their children (the area's tree
    /// order), with where its parent is in this list.
    order: Vec<(hecs::Entity, Option<usize>)>,
    clock: Clock,
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
    /// The world for `area`: every entity, with its components, and where
    /// each is in the world worked out. The clock starts at zero.
    pub fn from_area(area: &Area) -> Self {
        let tree = area.depth_first();
        let mut ecs = hecs::World::new();
        let mut by_id = HashMap::with_capacity(tree.len());
        let mut place: HashMap<EntityId, usize> = HashMap::with_capacity(tree.len());
        let mut order = Vec::with_capacity(tree.len());
        for id in tree {
            let Some(data) = area.get(id) else {
                continue;
            };
            let mut builder = hecs::EntityBuilder::new();
            builder.add(Id(id)).add(Name(data.name.clone()));
            let mut values = Values::default();
            for (name, component) in &data.components {
                match name.as_str() {
                    Transform::NAME => {
                        builder.add(Transform::from_data(component));
                    }
                    Light::NAME => {
                        builder.add(Light::from_data(component));
                    }
                    Model::NAME => {
                        builder.add(Model::from_data(component));
                    }
                    Environment::NAME => {
                        builder.add(Environment::from_data(component));
                    }
                    Camera::NAME => {
                        builder.add(Camera::from_data(component).sanitized());
                    }
                    _ => {
                        values.0.insert(name.clone(), component.clone());
                    }
                }
            }
            if !values.0.is_empty() {
                builder.add(values);
            }
            // A parent that comes later in the order is part of a loop
            // in a damaged area: the entity is treated as standing alone.
            let parent = data.parent.and_then(|p| place.get(&p).copied());
            if let Some(p) = data.parent {
                builder.add(Parent(p));
            }
            builder.add(Global(Mat4::IDENTITY));
            let entity = ecs.spawn(builder.build());
            by_id.insert(id, entity);
            place.insert(id, order.len());
            order.push((entity, parent));
        }
        let mut world = Self {
            ecs,
            by_id,
            order,
            clock: Clock::default(),
        };
        world.place_everything();
        world
    }

    /// The world for `area` again (it changed in the editor while
    /// playing), keeping the clock.
    pub fn rebuild(&mut self, area: &Area) {
        let clock = self.clock;
        *self = Self::from_area(area);
        self.clock = clock;
    }

    /// Moves the clock on by `seconds` (ignored if not a sensible number),
    /// running as many whole ticks as fit, at most
    /// [`MAX_TICKS_PER_STEP`]. Returns how many ran.
    pub fn step(&mut self, seconds: f64) -> u64 {
        if !seconds.is_finite() || seconds <= 0.0 {
            return 0;
        }
        self.clock.carry += seconds;
        let mut ran = 0;
        while self.clock.carry >= TICK && ran < MAX_TICKS_PER_STEP {
            self.clock.carry -= TICK;
            self.clock.ticks += 1;
            self.clock.time += TICK;
            ran += 1;
        }
        // Behind by more than the most ticks: the rest is dropped.
        if self.clock.carry >= TICK {
            self.clock.carry = 0.0;
        }
        if ran > 0 {
            self.place_everything();
        }
        ran
    }

    /// Works out where every entity is in the world, parents first.
    fn place_everything(&mut self) {
        let mut globals: Vec<Mat4> = Vec::with_capacity(self.order.len());
        for &(entity, parent) in &self.order {
            let local = self
                .ecs
                .get::<&Transform>(entity)
                .map_or(Mat4::IDENTITY, |t| t.matrix());
            let global = match parent.and_then(|p| globals.get(p)) {
                Some(parent) => *parent * local,
                None => local,
            };
            globals.push(global);
            if let Ok(mut g) = self.ecs.get::<&mut Global>(entity) {
                g.0 = global;
            }
        }
    }

    /// The game's clock.
    pub fn clock(&self) -> Clock {
        self.clock
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

    /// Where the entity is in the world.
    pub fn global(&self, id: EntityId) -> Option<Mat4> {
        self.get::<Global>(id).map(|g| g.0)
    }

    /// Every entity with a `T`, with it and where it is, in the area's
    /// tree order.
    pub fn placed<T: hecs::Component + Clone>(&self) -> Vec<(EntityId, T, Mat4)> {
        self.order
            .iter()
            .filter_map(|&(entity, _)| {
                let e = self.ecs.entity(entity).ok()?;
                let id = e.get::<&Id>()?.0;
                let value = (*e.get::<&T>()?).clone();
                let global = e.get::<&Global>().map_or(Mat4::IDENTITY, |g| g.0);
                Some((id, value, global))
            })
            .collect()
    }

    /// The camera the game looks through: the first in the area's tree
    /// order.
    pub fn camera(&self) -> Option<(EntityId, Camera, Mat4)> {
        self.placed::<Camera>().into_iter().next()
    }

    /// The sky, fog and exposure: the first environment, if any.
    pub fn environment(&self) -> Option<Environment> {
        self.placed::<Environment>()
            .into_iter()
            .next()
            .map(|(_, e, _)| e)
    }
}

#[cfg(test)]
mod tests;
