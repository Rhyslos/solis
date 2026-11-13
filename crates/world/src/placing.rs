//! Where everything is: each entity's place in the world worked out from
//! its parents', set and changed while playing, settled after physics, and
//! read, now or between two ticks for drawing.

use crate::{Global, Id, Place, Previous, World, between};
use area::builtin::{Camera, Transform};
use glam::Mat4;
use reflect::EntityId;

impl World {
    /// Works out where every entity is in the world, parents first.
    pub(crate) fn place_everything(&mut self) {
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
        if !self.following && (self.physics.state().contains(id) || self.flat.state().contains(id))
        {
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

    pub(crate) fn parent_global_at(&self, at: usize) -> Mat4 {
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

    /// A body under something that moved goes with it at the next
    /// physics step (a lift's floor under the lift): colliders, kinematic
    /// bodies and characters. Dynamic bodies don't: physics moves those.
    ///
    /// A dynamic body goes too when game code or an edit moved what it's
    /// under (a crate in a group that's moved), but not when physics
    /// moved it (a crate under a tumbling body is physics' to move).
    pub(crate) fn follow_parent(&mut self, entity: hecs::Entity) {
        if self.following {
            return;
        }
        if let Ok(id) = self.ecs.get::<&Id>(entity).map(|i| i.0)
            && (self.physics.state().contains(id) || self.flat.state().contains(id))
            && !(self.after_physics && self.physics.state().is_dynamic(id))
        {
            self.physics_moved.push(id);
        }
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

    /// Where the entity is drawn, `t` of the way between ticks (see
    /// [`Self::placed_between`]): where sounds on it are heard from.
    pub fn global_between(&self, id: EntityId, t: f32) -> Option<Mat4> {
        let entity = self.entity(id)?;
        let now = self.ecs.get::<&Global>(entity).ok()?.0;
        let before = self.ecs.get::<&Previous>(entity).ok().and_then(|p| p.0);
        Some(before.map_or(now, |before| between(before, now, t)))
    }
}
