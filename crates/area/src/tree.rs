//! An area's tree, worked out once: each entity's children in sibling
//! order, every entity in tree order, and where each one is in the world.
//! The editor keeps one until the tree changes, so the Area panel, the
//! viewport and "the first sun" don't each walk the area again every
//! frame.

use crate::builtin::Transform;
use crate::document::Area;
use glam::Mat4;
use reflect::EntityId;
use std::collections::HashMap;

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
    /// The same as [`Area::world_matrix`] for each.
    pub fn world_matrices(&self, area: &Area) -> HashMap<EntityId, Mat4> {
        let mut world: HashMap<EntityId, Mat4> = HashMap::with_capacity(self.order.len());
        for &id in self.reachable() {
            let Some(entity) = area.get(id) else {
                continue;
            };
            let local = entity
                .get::<Transform>()
                .map_or(Mat4::IDENTITY, |t| t.matrix());
            let parent = entity.parent.and_then(|p| world.get(&p));
            world.insert(id, parent.map_or(local, |p| *p * local));
        }
        for &id in &self.order[self.reachable..] {
            world.insert(id, area.world_matrix(id));
        }
        world
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::EntityData;
    use glam::Vec3;

    fn at(x: f32) -> Transform {
        Transform {
            position: Vec3::new(x, 0.0, 0.0),
            ..Transform::default()
        }
    }

    #[test]
    fn the_tree_lists_parents_first_and_places_everything_like_the_area() {
        let mut area = Area::default();
        let ship = area.spawn(EntityData::new("Ship").with(&at(10.0)));
        let mut mast = EntityData::new("Mast").with(&at(1.0));
        mast.parent = Some(ship);
        let mast = area.spawn(mast);
        let mut sail = EntityData::new("Sail").with(&at(0.5));
        sail.parent = Some(mast);
        let sail = area.spawn(sail);
        let buoy = area.spawn(EntityData::new("Buoy"));
        let tree = Tree::of(&area);
        assert_eq!(tree.order(), area.depth_first().as_slice());
        assert_eq!(tree.children(Some(ship)), [mast]);
        assert_eq!(
            tree.first_of([sail, buoy, mast]),
            area.first_in_tree([sail, buoy, mast])
        );
        assert_eq!(tree.first_of([EntityId::random()]), None);
        let world = tree.world_matrices(&area);
        for id in [ship, mast, sail, buoy] {
            assert_eq!(world[&id], area.world_matrix(id));
        }
        assert!(!tree.in_loop(sail));
    }

    #[test]
    fn a_loop_of_parents_is_listed_last_and_still_placed() {
        let mut area = Area::default();
        let a = area.spawn(EntityData::new("A").with(&at(1.0)));
        let b = area.spawn(EntityData::new("B").with(&at(2.0)));
        let top = area.spawn(EntityData::new("Top"));
        area.get_mut(a).unwrap().parent = Some(b);
        area.get_mut(b).unwrap().parent = Some(a);
        let tree = Tree::of(&area);
        assert_eq!(tree.reachable(), [top]);
        assert_eq!(tree.order().len(), 3);
        assert!(tree.in_loop(a) && tree.in_loop(b));
        let world = tree.world_matrices(&area);
        assert_eq!(world[&a], area.world_matrix(a));
        assert_eq!(world.len(), 3);
    }
}
