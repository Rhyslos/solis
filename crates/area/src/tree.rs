//! An area's tree, worked out once: each entity's children in sibling
//! order, every entity in tree order, and where each one is in the world.
//! The editor keeps one until the tree changes, so the Area panel, the
//! viewport and "the first sun" don't each walk the area again every
//! frame.
//!
//! Also the two things that keep a loop of parents (a damaged file) from
//! doing harm. [`Area::mend_loops`] takes the loops out of an area as
//! it's read, so that no area in use has one. And [`Area::ancestors`] is
//! the one walk up through an entity's parents, which ends in a loop
//! instead of going round: an entity it finds in one stands alone, placed
//! by its own Transform, here and in the game's world alike.

use crate::builtin::Transform;
use crate::document::{Area, EntityData};
use glam::Mat4;
use reflect::EntityId;
use std::collections::HashMap;
use std::fmt;

/// An area's tree at one moment. It doesn't follow later changes: work it
/// out again when entities are added, deleted or given another parent.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Tree {
    children: HashMap<Option<EntityId>, Vec<EntityId>>,
    order: Vec<EntityId>,
    reachable: usize,
    place: HashMap<EntityId, usize>,
}

impl Tree {
    /// The tree of `area`, in one pass over it (and a sort of each
    /// entity's children).
    pub fn of(area: &Area) -> Self {
        let children = area.hierarchy();
        let mut order = Vec::with_capacity(area.len());
        let mut place = HashMap::with_capacity(area.len());
        let mut todo: Vec<EntityId> = children.get(&None).cloned().unwrap_or_default();
        todo.reverse();
        while let Some(id) = todo.pop() {
            if place.contains_key(&id) {
                continue;
            }
            place.insert(id, order.len());
            order.push(id);
            if let Some(kids) = children.get(&Some(id)) {
                todo.extend(kids.iter().rev());
            }
        }
        let reachable = order.len();
        // Entities caught in a loop of a damaged area: last, so nothing is
        // ever left out.
        for (id, _) in area.iter() {
            if let std::collections::hash_map::Entry::Vacant(free) = place.entry(id) {
                free.insert(order.len());
                order.push(id);
            }
        }
        Self {
            children,
            order,
            reachable,
            place,
        }
    }

    /// The children of `parent` (`None`: the top level), in sibling order.
    pub fn children(&self, parent: Option<EntityId>) -> &[EntityId] {
        self.children.get(&parent).map_or(&[], Vec::as_slice)
    }

    /// Every entity in tree order: each parent, then its children, as the
    /// Area panel lists them; entities caught in a loop come last.
    pub fn order(&self) -> &[EntityId] {
        &self.order
    }

    /// The entities a walk down from the top reaches, in tree order: all
    /// of them, unless a damaged area has a loop of parents.
    pub fn reachable(&self) -> &[EntityId] {
        &self.order[..self.reachable]
    }

    /// Whether `id` is caught in a loop of parents (a walk down from the
    /// top never reaches it).
    pub fn in_loop(&self, id: EntityId) -> bool {
        self.place.get(&id).is_some_and(|&at| at >= self.reachable)
    }

    /// Which of `ids` comes first in tree order: how "the first sun" or
    /// "the first camera" is chosen everywhere.
    pub fn first_of(&self, ids: impl IntoIterator<Item = EntityId>) -> Option<EntityId> {
        ids.into_iter()
            .filter_map(|id| Some((*self.place.get(&id)?, id)))
            .min()
            .map(|(_, id)| id)
    }

    /// Where every entity of `area` (the area this tree is of) is in the
    /// world, worked out parents first, so each is one multiplication.
    /// The same as [`Area::world_matrix`] for each: an entity caught in a
    /// loop of parents stands alone, at its own Transform.
    pub fn world_matrices(&self, area: &Area) -> HashMap<EntityId, Mat4> {
        let mut world: HashMap<EntityId, Mat4> = HashMap::with_capacity(self.order.len());
        for (at, &id) in self.order.iter().enumerate() {
            let Some(entity) = area.get(id) else {
                continue;
            };
            let local = entity
                .get::<Transform>()
                .map_or(Mat4::IDENTITY, |t| t.matrix());
            // Past the reachable ones there is no "parents first": each
            // of those is in a loop or under one.
            let parent = entity
                .parent
                .filter(|_| at < self.reachable)
                .and_then(|p| world.get(&p));
            world.insert(id, parent.map_or(local, |p| *p * local));
        }
        world
    }
}

/// The entities above one, nearest first: see [`Area::ancestors`].
#[derive(Debug, Clone)]
pub struct Ancestors<'a> {
    area: &'a Area,
    next: Option<EntityId>,
    /// One entity passed on the way: meeting it again means the walk is
    /// going round a loop. It moves up to where the walk is after 1, 2,
    /// 4, 8… steps, so any loop is found within a few turns of it, with
    /// nothing kept but this (Brent's way of finding a cycle).
    mark: EntityId,
    since_mark: usize,
    reach: usize,
    looped: bool,
}

impl Ancestors<'_> {
    /// Whether the walk ended because the parents lead round in a loop
    /// (known once it has ended).
    pub fn looped(&self) -> bool {
        self.looped
    }
}

impl<'a> Iterator for Ancestors<'a> {
    type Item = (EntityId, &'a EntityData);

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.next.take()?;
        if id == self.mark {
            self.looped = true;
            return None;
        }
        let entity = self.area.get(id)?;
        if self.since_mark == self.reach {
            self.mark = id;
            self.reach = self.reach.saturating_mul(2);
            self.since_mark = 0;
        }
        self.since_mark += 1;
        self.next = entity.parent;
        Some((id, entity))
    }
}

/// An entity [`Area::mend_loops`] moved to the top level.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mended {
    /// The entity.
    pub entity: EntityId,
    /// Its name.
    pub name: String,
    /// The parent it had, which led back to it.
    pub parent: EntityId,
}

impl fmt::Display for Mended {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({}) was inside itself: its parents led back to it. It's now at the top level",
            self.name, self.entity
        )
    }
}

impl Area {
    /// The entities above `id`: its parent, that one's parent, and so on
    /// up to the top level or a parent that isn't in the area. In a
    /// damaged area whose parents lead round in a loop the walk ends
    /// (having passed an entity of the loop a few times at most) and
    /// [`Ancestors::looped`] says so. Every walk up the tree goes through
    /// here, so none can hang.
    pub fn ancestors(&self, id: EntityId) -> Ancestors<'_> {
        Ancestors {
            area: self,
            next: self.get(id).and_then(|e| e.parent),
            mark: id,
            since_mark: 1,
            reach: 1,
            looped: false,
        }
    }

    /// Takes every loop of parents out of the area, and says what it
    /// did. A loop can't be made in the editor (the edit is refused), so
    /// one is a damaged or hand-edited file: each loop's entity with the
    /// lowest ID is moved to the top level, which leaves the others under
    /// it, as close to the file as a tree can be. The readers call this,
    /// so that no area in use has a loop. One pass over the area.
    pub fn mend_loops(&mut self) -> Vec<Mended> {
        // Each entity is walked up from once: `walk` says which walk
        // first passed it, so meeting one of this same walk again is a
        // loop, and meeting one of an earlier walk is a way already known
        // to end.
        let mut walk: HashMap<EntityId, usize> = HashMap::with_capacity(self.len());
        let mut cut = Vec::new();
        let mut path = Vec::new();
        for (number, (start, _)) in self.iter().enumerate() {
            path.clear();
            let mut at = Some(start);
            while let Some(id) = at.filter(|id| self.contains(*id)) {
                match walk.get(&id) {
                    Some(&earlier) if earlier != number => break,
                    Some(_) => {
                        let from = path.iter().position(|&p| p == id).unwrap_or(0);
                        cut.extend(path[from..].iter().copied().min());
                        break;
                    }
                    None => {}
                }
                walk.insert(id, number);
                path.push(id);
                at = self.get(id).and_then(|e| e.parent);
            }
        }
        cut.into_iter()
            .filter_map(|entity: EntityId| {
                let data = self.get_mut(entity)?;
                let parent = data.parent.take()?;
                Some(Mended {
                    entity,
                    name: data.name.clone(),
                    parent,
                })
            })
            .collect()
    }
}

#[cfg(test)]
#[path = "tree_tests.rs"]
mod tests;
