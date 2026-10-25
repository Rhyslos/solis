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

pub mod projectiles;

pub use projectiles::{MAX_PROJECTILES, Projectile, Projectiles, Shot};

use area::Area;
use area::builtin::{
    Camera, CharacterBody, Collider, Environment, Light, Model, RigidBody, Transform,
};
use glam::Mat4;
use reflect::{Component, ComponentData, EntityId};
use std::collections::{BTreeMap, HashMap};

pub use hecs;
pub use physics;

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
    /// The world for `area`: every entity, with its components, and where
    /// each is in the world worked out. The clock starts at zero.
    pub fn from_area(area: &Area) -> Self {
        let mut world = Self::from_area_without_physics(area);
        // Physics after places: bodies start where their entities are.
        world.join_all();
        world
    }

    /// As [`Self::from_area`], with physics left empty: for a world whose
    /// physics is built next anyway ([`Self::rebuild_physics`]: a rebuild,
    /// a save loaded), or one thrown away, so it isn't built twice.
    pub fn from_area_without_physics(area: &Area) -> Self {
        let tree = area.depth_first();
        let mut ecs = hecs::World::new();
        let mut by_id = reflect::IdMap::with_capacity_and_hasher(tree.len(), Default::default());
        let mut place: reflect::IdMap<usize> =
            reflect::IdMap::with_capacity_and_hasher(tree.len(), Default::default());
        let mut order = Vec::with_capacity(tree.len());
        for id in tree {
            let Some(data) = area.get(id) else {
                continue;
            };
            let mut builder = components_of(id, data);
            // A parent that comes later in the order is part of a loop
            // in a damaged area: the entity is treated as standing alone.
            let parent = data.parent.and_then(|p| place.get(&p).copied());
            builder.add(Place(order.len()));
            let entity = ecs.spawn(builder.build());
            by_id.insert(id, entity);
            place.insert(id, order.len());
            order.push((entity, parent));
        }
        let mut world = Self {
            ecs,
            by_id,
            place,
            order,
            clock: Clock::default(),
            ends: Vec::new(),
            unsettled: Vec::new(),
            names: HashMap::new(),
            revision: 0,
            structure: std::sync::OnceLock::new(),
            game_moved: reflect::IdMap::default(),
            projectiles: Projectiles::default(),
            seed: 0,
            physics: physics::Physics::new(),
            physics_moved: Vec::new(),
            physics_asked: Vec::new(),
            following: false,
            after_physics: false,
            stamp: next_number(),
            touched: false,
        };
        world.index();
        world.place_everything();
        world
    }

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

    /// Works out where each subtree ends, and each name's first entity.
    fn index(&mut self) {
        let mut ends: Vec<usize> = (1..=self.order.len()).collect();
        // Children come after their parents: walking back, each child's
        // end is known before its parent's is needed.
        for at in (0..self.order.len()).rev() {
            if let Some(parent) = self.order[at].1
                && parent < at
            {
                ends[parent] = ends[parent].max(ends[at]);
            }
        }
        self.ends = ends;
        self.bump_revision();
        self.names.clear();
        for (at, &(entity, _)) in self.order.iter().enumerate() {
            if let Ok(name) = self.ecs.get::<&Name>(entity) {
                self.names.entry(name.0.clone()).or_insert(at);
            }
        }
    }

    /// The world for `area` again (it changed in the editor while
    /// playing), keeping the clock.
    pub fn rebuild(&mut self, area: &Area) {
        self.touched = true;
        let clock = self.clock;
        // Where the game put things stays, unless the area moved them
        // since (an edit, an undo).
        let kept: Vec<(EntityId, Transform, Transform)> = self
            .game_moved
            .iter()
            .filter_map(|(&id, &from)| {
                let now = area.get(id)?.get::<Transform>().unwrap_or_default();
                let at = self.get::<Transform>(id)?;
                (now == from).then_some((id, from, at))
            })
            .collect();
        let projectiles = std::mem::take(&mut self.projectiles);
        let seed = self.seed;
        // How each body was moving carries on: speed, spin, sleep, and a
        // character's finding.
        let states = self.physics.state().body_states();
        let asked = std::mem::take(&mut self.physics_asked);
        *self = Self::from_area_without_physics(area);
        self.clock = clock;
        self.projectiles = projectiles;
        self.seed = seed;
        self.physics_asked = asked;
        for (id, from, at) in kept {
            self.set_transform(id, at);
            self.game_moved.insert(id, from);
        }
        self.settle();
        // Bodies where the game had them, moving as they were.
        self.rebuild_physics(&states, false);
    }

    /// Adds `entities` (a new subtree game code spawned, just added to the
    /// area at the end of its tree) after everything else, without
    /// building the rest again: its root is the one whose parent isn't
    /// among them, standing at the top level. Entities already in the
    /// world are left out. Returns the IDs added, parents first.
    pub fn add_subtree(&mut self, entities: &[(EntityId, area::EntityData)]) -> Vec<EntityId> {
        self.touched = true;
        let ids: std::collections::HashSet<EntityId> = entities.iter().map(|(id, _)| *id).collect();
        let mut children: HashMap<Option<EntityId>, Vec<(&str, EntityId, &area::EntityData)>> =
            HashMap::new();
        for (id, data) in entities {
            let parent = data.parent.filter(|p| ids.contains(p));
            children
                .entry(parent)
                .or_default()
                .push((data.order.as_str(), *id, data));
        }
        for kids in children.values_mut() {
            kids.sort_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)));
        }
        // Depth first from the top of the subtree, as the area's tree
        // order has it.
        let mut stack: Vec<(EntityId, &area::EntityData)> = children
            .get(&None)
            .map(|kids| kids.iter().rev().map(|&(_, id, data)| (id, data)).collect())
            .unwrap_or_default();
        let mut added = Vec::new();
        let mut seen = std::collections::HashSet::new();
        while let Some((id, data)) = stack.pop() {
            if !seen.insert(id) || self.by_id.contains_key(&id) {
                continue;
            }
            let at = self.order.len();
            let parent = data.parent.and_then(|p| self.place.get(&p).copied());
            let entity = self
                .ecs
                .spawn(components_of(id, data).add(Place(at)).build());
            self.by_id.insert(id, entity);
            self.place.insert(id, at);
            self.order.push((entity, parent));
            self.ends.push(at + 1);
            // Its parents' runs now reach past it (only parents inside the
            // subtree: its root stands at the top).
            let mut up = parent;
            while let Some(p) = up {
                if p < self.ends.len() {
                    self.ends[p] = self.ends[p].max(at + 1);
                }
                up = self.order.get(p).and_then(|(_, pp)| *pp);
            }
            self.names.entry(data.name.clone()).or_insert(at);
            let local = data
                .get::<Transform>()
                .map_or(Mat4::IDENTITY, |t| t.matrix());
            let global = self.parent_global_at(at) * local;
            if let Ok(mut g) = self.ecs.get::<&mut Global>(entity) {
                g.0 = global;
            }
            added.push(id);
            if let Some(kids) = children.get(&Some(id)) {
                stack.extend(kids.iter().rev().map(|&(_, kid, data)| (kid, data)));
            }
        }
        if !added.is_empty() {
            self.bump_revision();
        }
        for &id in &added {
            self.physics_join(id);
        }
        added
    }

    /// The IDs of `id` and everything under it, parents first (none if
    /// it isn't in the world).
    pub fn subtree(&self, id: EntityId) -> Vec<EntityId> {
        let Some(&at) = self.place.get(&id) else {
            return Vec::new();
        };
        let end = self.ends.get(at).copied().unwrap_or(at + 1);
        self.order[at..end]
            .iter()
            .filter_map(|(entity, _)| self.ecs.get::<&Id>(*entity).ok().map(|i| i.0))
            .collect()
    }

    /// Takes each of `ids` and everything under it out of the world (game
    /// code despawned them), without building the rest again.
    pub fn remove_subtrees(&mut self, ids: &[EntityId]) {
        self.touched = true;
        let mut gone = vec![false; self.order.len()];
        let mut any = false;
        for id in ids {
            let Some(&at) = self.place.get(id) else {
                continue;
            };
            let end = self.ends.get(at).copied().unwrap_or(at + 1);
            for slot in gone.iter_mut().take(end).skip(at) {
                *slot = true;
            }
            any = true;
        }
        if !any {
            return;
        }
        // Where each kept entity moves to, and the order without the rest.
        let mut moved_to = vec![usize::MAX; self.order.len()];
        let old = std::mem::take(&mut self.order);
        for (at, (entity, parent)) in old.into_iter().enumerate() {
            if gone[at] {
                if let Ok(id) = self.ecs.get::<&Id>(entity).map(|i| i.0) {
                    self.by_id.remove(&id);
                    self.place.remove(&id);
                    self.game_moved.remove(&id);
                    self.physics.remove(id);
                }
                let _ = self.ecs.despawn(entity);
                continue;
            }
            let now = self.order.len();
            moved_to[at] = now;
            // A parent always comes first, so it has moved already.
            let parent = parent.and_then(|p| moved_to.get(p).copied().filter(|&n| n != usize::MAX));
            self.order.push((entity, parent));
            if now != at {
                if let Ok(id) = self.ecs.get::<&Id>(entity).map(|i| i.0) {
                    self.place.insert(id, now);
                }
                if let Ok(mut place) = self.ecs.get::<&mut Place>(entity) {
                    place.0 = now;
                }
            }
        }
        self.unsettled.clear();
        self.index();
    }

    /// Takes in changes to the components and names of entities in `area`
    /// (as the editor makes them while playing), without building the
    /// rest again. Each comes with whether its Transform changed: if not,
    /// it stays where the running game put it. Anything else (an entity
    /// added, deleted or moved to another parent) builds the whole world
    /// again. Keeps the clock.
    pub fn update(&mut self, area: &Area, changed: impl IntoIterator<Item = (EntityId, bool)>) {
        self.touched = true;
        let mut rejoin = Vec::new();
        for (id, moved) in changed {
            let (Some(data), Some(&entity), Some(&at)) =
                (area.get(id), self.by_id.get(&id), self.place.get(&id))
            else {
                self.rebuild(area);
                return;
            };
            let parent_now = self.ecs.get::<&Parent>(entity).ok().map(|p| p.0);
            if parent_now != data.parent {
                self.rebuild(area);
                return;
            }
            let kept = (!moved)
                .then(|| self.ecs.get::<&Transform>(entity).ok().map(|t| *t))
                .flatten();
            if moved {
                self.game_moved.remove(&id);
            }
            let before = self.physics_parts(entity);
            let _ = self.ecs.despawn(entity);
            let fresh = self
                .ecs
                .spawn(components_of(id, data).add(Place(at)).build());
            if let Some(transform) = kept {
                let _ = self.ecs.insert_one(fresh, transform);
            }
            self.by_id.insert(id, fresh);
            self.order[at].0 = fresh;
            // Only what physics knows of it changed: its body is built
            // again (an edit to another component leaves it alone).
            if self.physics_parts(fresh) != before {
                rejoin.push(id);
            }
        }
        // A name may have changed.
        self.index();
        self.place_everything();
        // Their bodies and colliders as they are now, moving as they were;
        // and bodies under them go where they now are.
        for id in rejoin {
            let state = self.physics.state().body_state(id);
            self.physics_join(id);
            if let Some(state) = state {
                self.physics.set_body_state(id, state, false);
            }
            for under in self.subtree(id).into_iter().skip(1) {
                if let Some(&entity) = self.by_id.get(&under) {
                    self.follow_parent(entity);
                }
            }
        }
    }

    /// Moves the clock on by `seconds` (ignored if not a sensible number),
    /// running as many whole ticks as fit, at most
    /// [`MAX_TICKS_PER_STEP`]. Returns how many ran. ([`Self::due`] and
    /// [`Self::begin_tick`] do the same a tick at a time.)
    pub fn step(&mut self, seconds: f64) -> u64 {
        let due = self.due(seconds);
        for _ in 0..due {
            self.begin_tick();
        }
        due
    }

    /// How many whole ticks `seconds` more (ignored if not a sensible
    /// number) make due, at most [`MAX_TICKS_PER_STEP`]; the rest waits
    /// for the next frame (or, behind by more than that, is dropped). The
    /// clock itself moves on as each tick begins ([`Self::begin_tick`]),
    /// so a frame that stops early (a pause, a load) never counts ticks
    /// that didn't run.
    pub fn due(&mut self, seconds: f64) -> u64 {
        if !seconds.is_finite() || seconds <= 0.0 {
            return 0;
        }
        self.clock.carry += seconds;
        let mut due = 0;
        while self.clock.carry >= TICK && due < MAX_TICKS_PER_STEP {
            self.clock.carry -= TICK;
            due += 1;
        }
        // Behind by more than the most ticks: the rest is dropped.
        if self.clock.carry >= TICK {
            self.clock.carry = 0.0;
        }
        due
    }

    /// A tick begins: the clock moves on by one.
    pub fn begin_tick(&mut self) {
        self.clock.ticks = self.clock.ticks.saturating_add(1);
        self.clock.time += TICK;
        self.touched = true;
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

    /// Works out where every entity is in the world, parents first.
    fn place_everything(&mut self) {
        self.unsettled.clear();
        let mut globals: Vec<Mat4> = Vec::with_capacity(self.order.len());
        for &(entity, parent) in &self.order {
            let local = self
                .ecs
                .get::<&Transform>(entity)
                .map_or(Mat4::IDENTITY, |t| t.matrix());
            // Always times the parent's (the identity for one standing
            // alone), as everywhere else a place is worked out: skipping it
            // left a -0.0 where every other way makes 0.0, and a world
            // built here (a loaded game) didn't match the same world built
            // move by move (found by the hardening milestone's invariant
            // tests, decision 0042).
            let parent = parent
                .and_then(|p| globals.get(p))
                .copied()
                .unwrap_or(Mat4::IDENTITY);
            let global = parent * local;
            globals.push(global);
            if let Ok(mut g) = self.ecs.get::<&mut Global>(entity) {
                g.0 = global;
            }
        }
    }

    /// Sets `id`'s transform (game code moving or turning it). Its own
    /// place in the world follows at once; the places of entities under it
    /// follow at [`Self::settle`]. Returns whether it's in the world.
    pub fn set_transform(&mut self, id: EntityId, transform: Transform) -> bool {
        self.change_transform(id, |_, _| Some(transform))
    }

    /// Changes `id`'s transform to what `change` makes of where its parent
    /// is in the world and its transform now (nothing, for `None`), with
    /// one lookup. Otherwise as [`Self::set_transform`].
    pub fn change_transform(
        &mut self,
        id: EntityId,
        change: impl FnOnce(Mat4, Transform) -> Option<Transform>,
    ) -> bool {
        self.touched = true;
        let (Some(&entity), Some(&at)) = (self.by_id.get(&id), self.place.get(&id)) else {
            return false;
        };
        let parent = self.parent_global_at(at);
        let now = self
            .ecs
            .get::<&Transform>(entity)
            .map(|t| *t)
            .unwrap_or_default();
        let Some(transform) = change(parent, now) else {
            return true;
        };
        self.game_moved.entry(id).or_insert(now);
        if let Ok(mut t) = self.ecs.get::<&mut Transform>(entity) {
            *t = transform;
        } else {
            let _ = self.ecs.insert_one(entity, transform);
        }
        if let Ok(mut g) = self.ecs.get::<&mut Global>(entity) {
            g.0 = parent * transform.matrix();
        }
        // Only what's under it needs working out again.
        if self.ends.get(at).is_some_and(|&end| end > at + 1) {
            self.unsettled.push(at);
        }
        // Its body follows at the next physics step.
        if !self.following && self.physics.state().contains(id) {
            self.physics_moved.push(id);
        }
        true
    }

    /// Where `id`'s parent is in the world (identity for an entity
    /// standing alone, or not in the world): what its transform is
    /// measured from.
    pub fn parent_global(&self, id: EntityId) -> Mat4 {
        self.place
            .get(&id)
            .map_or(Mat4::IDENTITY, |&at| self.parent_global_at(at))
    }

    fn parent_global_at(&self, at: usize) -> Mat4 {
        self.order
            .get(at)
            .and_then(|(_, parent)| *parent)
            .and_then(|p| self.order.get(p))
            .and_then(|(e, _)| self.ecs.get::<&Global>(*e).ok().map(|g| g.0))
            .unwrap_or(Mat4::IDENTITY)
    }

    /// Works out every entity's place again if a transform was set since
    /// last time (entities under one that moved follow it).
    pub fn settle(&mut self) {
        self.touched |= !self.unsettled.is_empty();
        let mut moved = std::mem::take(&mut self.unsettled);
        moved.sort_unstable();
        let mut done = 0;
        for at in moved {
            // Inside a subtree worked out already.
            if at < done {
                continue;
            }
            let end = self.ends.get(at).copied().unwrap_or(at + 1);
            for under in at + 1..end {
                let (entity, _) = self.order[under];
                let local = self
                    .ecs
                    .get::<&Transform>(entity)
                    .map_or(Mat4::IDENTITY, |t| t.matrix());
                let global = self.parent_global_at(under) * local;
                if let Ok(mut g) = self.ecs.get::<&mut Global>(entity) {
                    g.0 = global;
                }
                self.follow_parent(entity);
            }
            done = end;
        }
    }

    /// What physics knows of an entity: where it's placed and its physics
    /// components.
    fn physics_parts(
        &self,
        entity: hecs::Entity,
    ) -> (
        Option<Transform>,
        Option<RigidBody>,
        Option<Collider>,
        Option<CharacterBody>,
    ) {
        (
            self.ecs.get::<&Transform>(entity).ok().map(|c| *c),
            self.ecs.get::<&RigidBody>(entity).ok().map(|c| *c),
            self.ecs.get::<&Collider>(entity).ok().map(|c| *c),
            self.ecs.get::<&CharacterBody>(entity).ok().map(|c| *c),
        )
    }

    /// A body under something that moved goes with it at the next
    /// physics step (a lift's floor under the lift): colliders, kinematic
    /// bodies and characters. Dynamic bodies don't: physics moves those.
    ///
    /// A dynamic body goes too when game code or an edit moved what it's
    /// under (a crate in a group that's moved), but not when physics
    /// moved it (a crate under a tumbling body is physics' to move).
    fn follow_parent(&mut self, entity: hecs::Entity) {
        if self.following {
            return;
        }
        if let Ok(id) = self.ecs.get::<&Id>(entity).map(|i| i.0)
            && self.physics.state().contains(id)
            && !(self.after_physics && self.physics.state().is_dynamic(id))
        {
            self.physics_moved.push(id);
        }
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

    /// Where every entity is in the world, in the area's tree order (as
    /// [`Self::entities`]), into `out`.
    pub fn globals(&self, out: &mut Vec<Mat4>) {
        out.clear();
        out.resize(self.order.len(), Mat4::IDENTITY);
        for (place, global) in self.ecs.query::<(&Place, &Global)>().iter() {
            if let Some(slot) = out.get_mut(place.0) {
                *slot = global.0;
            }
        }
    }

    /// The first entity named `name`, in the area's tree order.
    pub fn find(&self, name: &str) -> Option<EntityId> {
        let &at = self.names.get(name)?;
        let (entity, _) = self.order.get(at)?;
        self.ecs.get::<&Id>(*entity).ok().map(|id| id.0)
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
        self.placed_between(1.0)
    }

    /// As [`Self::placed`], but each where it's drawn a share `t` (from 0
    /// to 1) of the way from where it was when the last tick started to
    /// where it is now: so a screen faster than the ticks shows smooth
    /// movement. [`Self::progress`] says how far the clock is into the
    /// next tick.
    pub fn placed_between<T: hecs::Component + Clone>(&self, t: f32) -> Vec<(EntityId, T, Mat4)> {
        // Only the entities that have a `T`, then put in tree order.
        let mut found: Vec<(usize, EntityId, T, Mat4)> = self
            .ecs
            .query::<(&Place, &Id, &T, Option<&Global>, Option<&Previous>)>()
            .iter()
            .map(|(place, id, value, global, previous)| {
                let now = global.map_or(Mat4::IDENTITY, |g| g.0);
                let drawn = match previous.and_then(|p| p.0) {
                    Some(before) => between(before, now, t),
                    None => now,
                };
                (place.0, id.0, value.clone(), drawn)
            })
            .collect();
        // Usually already in order (entities are made in tree order), so
        // only sorted when they aren't.
        if !found.is_sorted_by_key(|f| f.0) {
            found.sort_unstable_by_key(|f| f.0);
        }
        found
            .into_iter()
            .map(|(_, id, value, global)| (id, value, global))
            .collect()
    }

    /// The first entity with a `T` in the area's tree order, with it and
    /// where it is.
    pub fn first<T: hecs::Component + Clone>(&self) -> Option<(EntityId, T, Mat4)> {
        self.ecs
            .query::<(&Place, &Id, &T, Option<&Global>)>()
            .iter()
            .min_by_key(|(place, ..)| place.0)
            .map(|(_, id, value, global)| {
                (id.0, value.clone(), global.map_or(Mat4::IDENTITY, |g| g.0))
            })
    }

    /// The camera the game looks through: the first in the area's tree
    /// order.
    pub fn camera(&self) -> Option<(EntityId, Camera, Mat4)> {
        self.first::<Camera>()
    }

    /// As [`Self::camera`], drawn `t` of the way between ticks (see
    /// [`Self::placed_between`]).
    pub fn camera_between(&self, t: f32) -> Option<(EntityId, Camera, Mat4)> {
        let (id, camera, now) = self.first::<Camera>()?;
        let drawn = self
            .entity(id)
            .and_then(|e| self.ecs.get::<&Previous>(e).ok().and_then(|p| p.0))
            .map_or(now, |before| between(before, now, t));
        Some((id, camera, drawn))
    }

    /// Remembers where every entity is, as the start of a tick: what
    /// drawing between ticks moves from. The editor calls it before each
    /// tick.
    pub fn remember(&mut self) {
        for (global, previous) in self.ecs.query_mut::<(&Global, &mut Previous)>() {
            previous.0 = Some(global.0);
        }
    }

    /// How far the clock is into the next tick, from 0 to 1: how far
    /// drawing should be between the last tick and the next.
    pub fn progress(&self) -> f32 {
        ((self.clock.carry / TICK) as f32).clamp(0.0, 1.0)
    }

    /// A checksum of the game's world (Phase 6, determinism): every
    /// entity in order with its name, place and game components' values,
    /// the projectiles, physics and the seed. Two runs of a game that
    /// agree on it at a tick are in the same state (game code's running
    /// values are checked apart, by `script::Game::checksum`).
    pub fn checksum(&self) -> u64 {
        let mut sum = reflect::Checksum::new();
        // Not the clock: during a frame of several ticks it has moved on
        // by all of them before the first runs, and the tick's number is
        // known wherever checksums are compared.
        sum.u64(self.seed);
        // Which entities, in what tree, with what names and component
        // values: these change only with the area (which starts a new
        // revision), so they're worked out once for it.
        sum.u64(*self.structure.get_or_init(|| {
            let mut structure = reflect::Checksum::new();
            structure.u64(self.order.len() as u64);
            for &(entity, parent) in &self.order {
                let Ok(e) = self.ecs.entity(entity) else {
                    continue;
                };
                if let Some(id) = e.get::<&Id>() {
                    structure.entity(id.0);
                }
                structure.u64(parent.map_or(u64::MAX, |p| p as u64));
                if let Some(name) = e.get::<&Name>() {
                    structure.bytes(name.0.as_bytes());
                }
                if let Some(values) = e.get::<&Values>() {
                    for (name, data) in &values.0 {
                        structure.bytes(name.as_bytes());
                        structure.data(data);
                    }
                }
            }
            structure.finish()
        }));
        for &(entity, _) in &self.order {
            let Ok(global) = self.ecs.get::<&Global>(entity) else {
                continue;
            };
            // Where it is, turned and scaled: the matrix's top three rows
            // (the bottom is always 0 0 0 1).
            let m = global.0;
            for row in [m.row(0), m.row(1), m.row(2)] {
                for v in row.to_array() {
                    sum.f32(v);
                }
            }
        }
        sum.u64(self.physics.state().checksum());
        let flying = self.projectiles.flying();
        sum.u64(flying.len() as u64);
        for p in flying {
            for v in p
                .position
                .to_array()
                .into_iter()
                .chain(p.velocity.to_array())
            {
                sum.f32(v);
            }
            sum.f32(p.left);
            for v in p.colour {
                sum.f32(v);
            }
        }
        sum.finish()
    }

    /// The game's physics (decision 0036), to read: game code reads its
    /// state on every core during a tick.
    pub fn physics(&self) -> &physics::Physics {
        &self.physics
    }

    /// Makes `id`'s part in physics match its components and where it is
    /// now: a body and collider if it has them, nothing if not.
    fn physics_join(&mut self, id: EntityId) {
        let Some(&entity) = self.by_id.get(&id) else {
            self.physics.remove(id);
            return;
        };
        let global = self
            .ecs
            .get::<&Global>(entity)
            .map_or(Mat4::IDENTITY, |g| g.0);
        if let Ok(c) = self.ecs.get::<&CharacterBody>(entity).map(|c| *c) {
            let (_, rotation, position) = global.to_scale_rotation_translation();
            let spec = physics::CharacterSpec {
                height: c.height,
                radius: c.radius,
                step: c.step,
                slope: c.slope,
                snap: c.snap,
                mass: c.mass,
            };
            self.physics.add_character(id, (position, rotation), spec);
            self.touched = true;
            return;
        }
        let body = self.ecs.get::<&RigidBody>(entity).ok().map(|b| *b);
        let collider = self.ecs.get::<&Collider>(entity).ok().map(|c| *c);
        if body.is_none() && collider.is_none() {
            self.physics.remove(id);
            return;
        }
        let (scale, rotation, position) = global.to_scale_rotation_translation();
        let body = body.map(|b| physics::Body {
            kind: match b.kind {
                area::builtin::BodyKind::Dynamic => physics::BodyKind::Dynamic,
                area::builtin::BodyKind::Kinematic => physics::BodyKind::Kinematic,
                area::builtin::BodyKind::Fixed => physics::BodyKind::Fixed,
            },
            mass: b.mass,
            linear_damping: b.linear_damping,
            angular_damping: b.angular_damping,
            gravity: b.gravity,
            upright: b.upright,
            fast: b.fast,
        });
        let collider = collider.map(|c| physics::ColliderSpec {
            shape: match c.shape {
                area::builtin::ColliderShape::Box => physics::Shape::Box(c.size),
                area::builtin::ColliderShape::Sphere => physics::Shape::Sphere(c.radius),
                area::builtin::ColliderShape::Capsule => physics::Shape::Capsule {
                    radius: c.radius,
                    height: c.height,
                },
            },
            center: c.center,
            friction: c.friction,
            bounciness: c.bounciness,
        });
        self.physics
            .add(id, (position, rotation), scale, body, collider);
        self.touched = true;
    }

    /// Gives `id`'s body a push (kilograms times metres a second), at the
    /// next physics step.
    pub fn push(&mut self, id: EntityId, impulse: glam::Vec3) {
        self.physics_asked.push(PhysicsAsk::Push(id, impulse));
    }

    /// Sets how fast `id`'s body moves (metres a second), at the next
    /// physics step.
    pub fn set_velocity(&mut self, id: EntityId, velocity: glam::Vec3) {
        self.physics_asked.push(PhysicsAsk::Velocity(id, velocity));
    }

    /// Moves the character `id` (a `CharacterBody`) by `wanted` metres,
    /// as far as it can, at the next physics step (see
    /// `physics::Physics::move_character`).
    pub fn move_character(&mut self, id: EntityId, wanted: glam::Vec3) {
        self.physics_asked.push(PhysicsAsk::Character(id, wanted));
    }

    /// Bodies not made because there were already the most
    /// (`physics::MAX_BODIES`).
    pub fn physics_refused(&self) -> u64 {
        self.physics.refused()
    }

    /// Moves physics on by one tick: bodies follow what game code moved,
    /// its pushes and velocities are made, physics steps, and the
    /// entities physics moved follow their bodies (those under them too).
    /// The editor calls it once per tick, after game code's changes land.
    pub fn step_physics(&mut self) {
        if self.physics.state().is_empty() {
            self.physics_moved.clear();
            self.physics_asked.clear();
            return;
        }
        self.touched = true;
        // What game code moved, in the order moved, each once.
        let mut moved = std::mem::take(&mut self.physics_moved);
        let mut seen = reflect::IdSet::default();
        moved.retain(|id| seen.insert(*id));
        for id in moved {
            if let Some(global) = self.global(id) {
                let (_, rotation, position) = global.to_scale_rotation_translation();
                self.physics.set_pose(id, position, rotation);
            }
        }
        // Pushes and velocities in the order asked, then characters' moves
        // in the order asked (worked out on every core at once).
        let mut characters = Vec::new();
        for ask in std::mem::take(&mut self.physics_asked) {
            match ask {
                PhysicsAsk::Push(id, impulse) => self.physics.push(id, impulse),
                PhysicsAsk::Velocity(id, velocity) => self.physics.set_velocity(id, velocity),
                PhysicsAsk::Character(id, wanted) => characters.push((id, wanted)),
            }
        }
        if !characters.is_empty() {
            self.physics.move_characters(&characters, TICK as f32);
        }
        self.physics.step(TICK as f32);
        // The entities follow their bodies, without sending it back.
        self.following = true;
        for (id, position, rotation) in self.physics.moved() {
            self.change_transform(id, |parent, mut t| {
                if parent == Mat4::IDENTITY {
                    t.position = position;
                    t.rotation = rotation;
                } else {
                    t.position = parent.inverse().transform_point3(position);
                    let turn = parent.to_scale_rotation_translation().1;
                    t.rotation = (turn.inverse() * rotation).normalize();
                }
                (t.position.is_finite() && t.rotation.is_finite()).then_some(t)
            });
        }
        self.following = false;
        self.after_physics = true;
        self.settle();
        self.after_physics = false;
    }

    /// Sets the clock to `ticks` run, at game time `time` (a save game
    /// loaded, decision 0038), with nothing waiting for the next tick.
    pub fn set_clock(&mut self, ticks: u64, time: f64) {
        self.clock = Clock {
            ticks,
            time: if time.is_finite() { time } else { 0.0 },
            carry: 0.0,
        };
        self.touched = true;
    }

    /// Every body's place and motion (`physics::BodyState`), in entity
    /// order: what a save game keeps of physics (decision 0038).
    pub fn physics_states(&self) -> Vec<(EntityId, physics::BodyState)> {
        self.physics.state().body_states()
    }

    /// Builds physics again from scratch, every entity joining in ID
    /// order, then gives the bodies in `states` their motion, and their
    /// places too `with_place`. Nothing of Rapier's own (contacts,
    /// islands) is kept, so a save game, which builds the running game's
    /// physics again this way as it saves and the loaded game's as it
    /// loads, carries on exactly alike both ways, and nothing in a file
    /// can reach Rapier's insides.
    pub fn rebuild_physics(&mut self, states: &[(EntityId, physics::BodyState)], with_place: bool) {
        self.join_all();
        for (id, state) in states {
            self.physics.set_body_state(*id, *state, with_place);
        }
    }

    /// Empties physics and joins every entity again, in ID order.
    fn join_all(&mut self) {
        self.physics.clear();
        let mut ids: Vec<EntityId> = self
            .order
            .iter()
            .filter_map(|(e, _)| self.ecs.get::<&Id>(*e).ok().map(|i| i.0))
            .collect();
        ids.sort_unstable();
        for id in ids {
            self.physics_join(id);
        }
        self.physics_moved.clear();
        self.touched = true;
    }

    /// Whether physics broke (Rapier panicked on a damaged save) and was
    /// emptied since last asked.
    pub fn take_physics_broke(&mut self) -> bool {
        self.physics.take_broke()
    }

    /// The seed the game's random numbers are made from.
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Starts the game's random numbers again from `seed` (Play chooses
    /// one; game code's `setSeed` changes it).
    pub fn set_seed(&mut self, seed: u64) {
        self.seed = seed;
        self.touched = true;
    }

    /// The sky, fog and exposure: the first environment, if any.
    pub fn environment(&self) -> Option<Environment> {
        self.first::<Environment>().map(|(_, e, _)| e)
    }
}

/// An entity's components as the world keeps them: built-in ones as
/// their Rust types, anything else as plain values.
fn components_of(id: EntityId, data: &area::EntityData) -> hecs::EntityBuilder {
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
            RigidBody::NAME => {
                builder.add(RigidBody::from_data(component));
            }
            Collider::NAME => {
                builder.add(Collider::from_data(component));
            }
            CharacterBody::NAME => {
                builder.add(CharacterBody::from_data(component));
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
    if let Some(p) = data.parent {
        builder.add(Parent(p));
    }
    builder.add(Global(Mat4::IDENTITY)).add(Previous(None));
    builder
}

#[cfg(test)]
mod tests;
