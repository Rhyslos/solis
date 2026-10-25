//! The document itself: entities by ID, each with a name, a parent, a
//! sibling order and components.

use reflect::{Component, ComponentData, EntityId};
use std::collections::BTreeMap;

/// One entity: its name, where it sits in the tree, and its components.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EntityData {
    /// The name shown in the Scene panel.
    pub name: String,
    /// The entity it belongs to, if any.
    pub parent: Option<EntityId>,
    /// Its place among its siblings: text that sorts in order, so a move
    /// can always fit between two others (decision 0020).
    pub order: String,
    /// Its components, by component name.
    pub components: BTreeMap<String, ComponentData>,
}

impl EntityData {
    /// An entity called `name`, with no components.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            parent: None,
            order: "a0".to_string(),
            components: BTreeMap::new(),
        }
    }

    /// The same entity, with `component` added (or replaced).
    #[must_use]
    pub fn with<T: Component>(mut self, component: &T) -> Self {
        self.set(component);
        self
    }

    /// Adds or replaces `component`.
    pub fn set<T: Component>(&mut self, component: &T) {
        self.components
            .insert(T::NAME.to_string(), component.to_data());
    }

    /// Its component of type `T`, if it has one.
    pub fn get<T: Component>(&self) -> Option<T> {
        self.components.get(T::NAME).map(T::from_data)
    }

    /// Whether it has a component called `name`.
    pub fn has(&self, name: &str) -> bool {
        self.components.contains_key(name)
    }
}

/// A scene: its entities, by ID. Sorted by ID, as scene files are.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Scene {
    entities: BTreeMap<EntityId, EntityData>,
}

impl Scene {
    /// An empty scene.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds `data` with a new random ID, and returns the ID.
    pub fn spawn(&mut self, data: EntityData) -> EntityId {
        let mut id = EntityId::random();
        while self.entities.contains_key(&id) {
            id = EntityId::random();
        }
        self.entities.insert(id, data);
        id
    }

    /// Puts `data` in under `id`, returning what was there.
    pub fn insert(&mut self, id: EntityId, data: EntityData) -> Option<EntityData> {
        self.entities.insert(id, data)
    }

    /// Takes the entity `id` out (its children keep pointing at it; see
    /// [`Self::remove_tree`]).
    pub fn remove(&mut self, id: EntityId) -> Option<EntityData> {
        self.entities.remove(&id)
    }

    /// Takes out `id` and everything below it, returning them parents
    /// first.
    pub fn remove_tree(&mut self, id: EntityId) -> Vec<(EntityId, EntityData)> {
        let ids = self.tree(id);
        ids.into_iter()
            .filter_map(|id| self.entities.remove(&id).map(|data| (id, data)))
            .collect()
    }

    /// `id` and everything below it, parents first.
    pub fn tree(&self, id: EntityId) -> Vec<EntityId> {
        if !self.entities.contains_key(&id) {
            return Vec::new();
        }
        let mut out = vec![id];
        let mut i = 0;
        while i < out.len() {
            let children = self.children(Some(out[i]));
            // A damaged scene could loop; never visit an entity twice.
            for child in children {
                if !out.contains(&child) {
                    out.push(child);
                }
            }
            i += 1;
        }
        out
    }

    /// The entity `id`.
    pub fn get(&self, id: EntityId) -> Option<&EntityData> {
        self.entities.get(&id)
    }

    /// The entity `id`, to change.
    pub fn get_mut(&mut self, id: EntityId) -> Option<&mut EntityData> {
        self.entities.get_mut(&id)
    }

    /// Whether there's an entity `id`.
    pub fn contains(&self, id: EntityId) -> bool {
        self.entities.contains_key(&id)
    }

    /// Every entity, sorted by ID.
    pub fn iter(&self) -> impl Iterator<Item = (EntityId, &EntityData)> {
        self.entities.iter().map(|(&id, data)| (id, data))
    }

    /// How many entities there are.
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    /// The children of `parent` (or the top-level entities, for `None`),
    /// in their sibling order (ties by ID). An entity whose parent is
    /// missing counts as top level, so nothing is ever hidden.
    pub fn children(&self, parent: Option<EntityId>) -> Vec<EntityId> {
        let mut out: Vec<(&str, EntityId)> = self
            .entities
            .iter()
            .filter(|(_, e)| {
                let effective = e.parent.filter(|p| self.entities.contains_key(p));
                effective == parent
            })
            .map(|(&id, e)| (e.order.as_str(), id))
            .collect();
        out.sort();
        out.into_iter().map(|(_, id)| id).collect()
    }

    /// Where `id` is in the world: its transform after its parents'. An
    /// entity without a `Transform` sits where its parent is. A loop in a
    /// damaged scene stops the walk instead of hanging.
    pub fn world_matrix(&self, id: EntityId) -> glam::Mat4 {
        let mut matrix = glam::Mat4::IDENTITY;
        let mut at = Some(id);
        let mut steps = 0;
        while let Some(current) = at {
            let Some(entity) = self.entities.get(&current) else {
                break;
            };
            if let Some(t) = entity.get::<crate::builtin::Transform>() {
                matrix = t.matrix() * matrix;
            }
            steps += 1;
            if steps > self.entities.len() {
                break;
            }
            at = entity.parent;
        }
        matrix
    }

    /// Every entity with a `T` component, with it, sorted by ID.
    pub fn with<T: Component>(&self) -> impl Iterator<Item = (EntityId, T)> + '_ {
        self.entities
            .iter()
            .filter_map(|(&id, e)| e.get::<T>().map(|c| (id, c)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtin::{Light, Transform};

    #[test]
    fn entities_are_spawned_found_and_removed() {
        let mut s = Scene::new();
        let a = s.spawn(EntityData::new("A").with(&Transform::default()));
        let b = s.spawn(EntityData::new("B"));
        assert_eq!(s.len(), 2);
        assert_ne!(a, b);
        assert!(s.get(a).unwrap().get::<Transform>().is_some());
        assert!(s.get(b).unwrap().get::<Transform>().is_none());
        assert_eq!(s.with::<Transform>().count(), 1);
        assert_eq!(s.remove(a).unwrap().name, "A");
        assert!(!s.contains(a));
    }

    #[test]
    fn trees_come_out_parents_first_and_in_order() {
        let mut s = Scene::new();
        let root = s.spawn(EntityData::new("Root"));
        let mut second = EntityData::new("Second");
        second.parent = Some(root);
        second.order = "b0".into();
        let second = s.spawn(second);
        let mut first = EntityData::new("First");
        first.parent = Some(root);
        let first = s.spawn(first);
        let mut grandchild = EntityData::new("Grandchild").with(&Light::default());
        grandchild.parent = Some(first);
        let grandchild = s.spawn(grandchild);
        assert_eq!(s.children(Some(root)), [first, second]);
        assert_eq!(s.tree(root), [root, first, second, grandchild]);
        let gone = s.remove_tree(root);
        assert_eq!(gone.len(), 4);
        assert!(s.is_empty());
    }

    #[test]
    fn world_matrices_add_up_parents() {
        let mut s = Scene::new();
        let parent = s.spawn(EntityData::new("P").with(&Transform {
            position: glam::Vec3::new(10.0, 0.0, 0.0),
            ..Transform::default()
        }));
        let mut child = EntityData::new("C").with(&Transform {
            position: glam::Vec3::new(0.0, 2.0, 0.0),
            ..Transform::default()
        });
        child.parent = Some(parent);
        let child = s.spawn(child);
        let at = s.world_matrix(child).transform_point3(glam::Vec3::ZERO);
        assert!(at.abs_diff_eq(glam::Vec3::new(10.0, 2.0, 0.0), 1e-6));
        // A loop ends.
        s.get_mut(parent).unwrap().parent = Some(child);
        assert!(s.world_matrix(child).is_finite());
    }

    #[test]
    fn an_orphan_shows_at_the_top_and_a_loop_never_hangs() {
        let mut s = Scene::new();
        let ghost = EntityId::random();
        let mut orphan = EntityData::new("Orphan");
        orphan.parent = Some(ghost);
        let orphan = s.spawn(orphan);
        assert_eq!(s.children(None), [orphan]);
        // Two entities that are each other's parent (a damaged file).
        let a = EntityId::random();
        let b = EntityId::random();
        let mut ea = EntityData::new("A");
        ea.parent = Some(b);
        let mut eb = EntityData::new("B");
        eb.parent = Some(a);
        s.insert(a, ea);
        s.insert(b, eb);
        assert_eq!(s.tree(a).len(), 2);
    }
}
