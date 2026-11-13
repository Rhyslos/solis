//! Building the world from an area, and adding, removing and updating
//! entities in it while the game plays (an edit while playing).

use crate::{
    Clock, Global, Id, Name, Parent, Place, Previous, Projectiles, Values, World, flat_physics,
    game_set, next_number,
};
use area::Area;
use area::builtin::{
    Animator, AudioSource, Camera, CharacterBody, Collider, Environment, Light, Listener, Model,
    ReverbZone, RigidBody, Sprite, Transform,
};
use glam::Mat4;
use reflect::{Component, EntityId};
use std::collections::HashMap;

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
        let tree = area::Tree::of(area);
        let count = tree.order().len();
        let mut ecs = hecs::World::new();
        let mut by_id = reflect::IdMap::with_capacity_and_hasher(count, Default::default());
        let mut place: reflect::IdMap<usize> =
            reflect::IdMap::with_capacity_and_hasher(count, Default::default());
        let mut order = Vec::with_capacity(count);
        for &id in tree.order() {
            let Some(data) = area.get(id) else {
                continue;
            };
            let mut builder = components_of(id, data);
            // An entity caught in a loop of parents (a damaged area; the
            // readers take loops out) stands alone, as the editor places
            // it (`area::Tree::world_matrices`).
            let parent = data
                .parent
                .filter(|_| !tree.in_loop(id))
                .and_then(|p| place.get(&p).copied());
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
            speed: 1.0,
            day_shift: 0.0,
            physics: physics::Physics::new(),
            physics_moved: Vec::new(),
            physics_asked: Vec::new(),
            following: false,
            after_physics: false,
            stamp: next_number(),
            touched: false,
            clip_sets: std::sync::Arc::default(),
            animators: std::sync::Arc::default(),
            saved_values: HashMap::new(),
            map_files: std::sync::Arc::default(),
            saved_maps: HashMap::new(),
            maps_revision: 0,
            flat: physics::flat::Flat::new(),
            pixels: area::pixels::PixelSettings::default(),
            solid_sets: std::sync::Arc::default(),
            walls_built: HashMap::new(),
            flat_recovery: flat_physics::FlatRecovery::default(),
        };
        world.index();
        world.place_everything();
        world
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
        let speed = self.speed;
        let day_shift = self.day_shift;
        // What the game made of each entity, put back on it once it's
        // made again (see `game_set`).
        let carried = self.carry_all();
        let clip_sets = std::sync::Arc::clone(&self.clip_sets);
        let animators = std::sync::Arc::clone(&self.animators);
        let saved_values = std::mem::take(&mut self.saved_values);
        let map_files = std::sync::Arc::clone(&self.map_files);
        let saved_maps = std::mem::take(&mut self.saved_maps);
        // How each body was moving carries on: speed, spin, sleep, and a
        // character's finding.
        let states = self.physics.state().body_states();
        let asked = std::mem::take(&mut self.physics_asked);
        let flat_recovery = self.flat_recovery;
        *self = Self::from_area_without_physics(area);
        self.flat_recovery = flat_recovery;
        self.clock = clock;
        self.projectiles = projectiles;
        self.seed = seed;
        self.speed = speed;
        self.day_shift = day_shift;
        self.clip_sets = clip_sets;
        self.animators = animators;
        self.saved_values = saved_values;
        self.map_files = map_files;
        self.saved_maps = saved_maps;
        for carried in carried {
            self.put_back(carried);
        }
        self.start_maps();
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

    /// Whether `inner` is `outer` or somewhere under it, without listing
    /// the subtree: the world keeps each subtree as one run of its order.
    pub fn is_inside(&self, inner: EntityId, outer: EntityId) -> bool {
        let (Some(&i), Some(&o)) = (self.place.get(&inner), self.place.get(&outer)) else {
            return false;
        };
        let end = self.ends.get(o).copied().unwrap_or(o + 1);
        (o..end).contains(&i)
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
                    // And from 2D physics: its collider, and a map's walls.
                    self.flat.remove(id);
                    if self.walls_built.remove(&id).is_some() {
                        self.flat.remove_walls(id);
                    }
                    self.touched = true;
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
            // What the game made of it, put back on it once it's made
            // again (see `game_set`).
            let carried = self.carry(entity);
            let _ = self.ecs.despawn(entity);
            let fresh = self
                .ecs
                .spawn(components_of(id, data).add(Place(at)).build());
            if let Some(transform) = kept {
                let _ = self.ecs.insert_one(fresh, transform);
            }
            self.by_id.insert(id, fresh);
            self.order[at].0 = fresh;
            if let Some(carried) = carried {
                self.put_back(carried);
            }
            // Only what physics knows of it changed: its body is built
            // again (an edit to another component leaves it alone).
            if self.physics_parts(fresh) != before {
                rejoin.push(id);
            }
        }
        // An entity given a TileMap, or another file's, starts its map.
        self.start_maps();
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
}

/// An entity's components as the world keeps them: built-in ones as
/// their Rust types, anything else as plain values.
fn components_of(id: EntityId, data: &area::EntityData) -> hecs::EntityBuilder {
    let mut builder = hecs::EntityBuilder::new();
    builder.add(Id(id)).add(Name(data.name.clone()));
    let mut values = Values::default();
    // What the area gives of the things game code sets (see `game_set`).
    let mut start = game_set::GameSet::default();
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
            area::builtin::Collider2D::NAME => {
                builder.add(area::builtin::Collider2D::from_data(component));
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
            AudioSource::NAME => {
                let source = AudioSource::from_data(component);
                start.read_source(&source);
                builder.add(source);
            }
            ReverbZone::NAME => {
                builder.add(ReverbZone::from_data(component));
            }
            area::builtin::Parallax::NAME => {
                builder.add(area::builtin::Parallax::from_data(component));
            }
            Listener::NAME => {
                builder.add(Listener::from_data(component));
            }
            Sprite::NAME => {
                let sprite = Sprite::from_data(component);
                start.read_sprite(&sprite);
                builder.add(sprite);
            }
            Animator::NAME => {
                let animator = Animator::from_data(component);
                start.read_animator(&animator);
                builder.add(animator);
            }
            area::builtin::TileMap::NAME => {
                builder.add(area::builtin::TileMap::from_data(component));
            }
            _ => {
                if name == area::builtin::Screen::NAME {
                    start.read_screen(component);
                }
                values.0.insert(name.clone(), component.clone());
            }
        }
    }
    if !values.0.is_empty() {
        builder.add(values);
    }
    if !start.is_empty() {
        builder.add(game_set::Start(start));
    }
    if let Some(p) = data.parent {
        builder.add(Parent(p));
    }
    builder.add(Global(Mat4::IDENTITY)).add(Previous(None));
    builder
}
