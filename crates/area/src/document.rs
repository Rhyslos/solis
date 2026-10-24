//! The document itself: entities by ID, each with a name, a parent, a
//! sibling order and components.

use reflect::{AssetId, Component, ComponentData, EntityId};
use std::collections::BTreeMap;

/// Where an entity comes from when it's part of an assembly placed in the
/// area (decision 0020): the assembly, the placed copy's root, and the
/// way to it inside the assembly (empty for the root itself; one ID per
/// assembly nested on the way).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Origin {
    /// The assembly.
    pub assembly: AssetId,
    /// The placed copy's root entity in the area.
    pub instance: EntityId,
    /// The entity inside the assembly: empty for the root, else the IDs
    /// leading to it through nested assemblies.
    pub path: Vec<EntityId>,
}

/// One entity: its name, where it sits in the tree, and its components.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EntityData {
    /// The name shown in the Area panel.
    pub name: String,
    /// The entity it belongs to, if any.
    pub parent: Option<EntityId>,
    /// Its place among its siblings: text that sorts in order, so a move
    /// can always fit between two others (decision 0020).
    pub order: String,
    /// Its components, by component name.
    pub components: BTreeMap<String, ComponentData>,
    /// The assembly it comes from, if it's part of a placed one.
    pub origin: Option<Origin>,
}

impl EntityData {
    /// An entity called `name`, with no components.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            parent: None,
            order: "a0".to_string(),
            components: BTreeMap::new(),
            origin: None,
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

/// An area: its entities, by ID. Sorted by ID, as area files are.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Area {
    entities: BTreeMap<EntityId, EntityData>,
}

impl Area {
    /// An empty area.
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
        // Most deleted entities have nothing under them: one look says so.
        if !self.entities.values().any(|e| e.parent == Some(id)) {
            return vec![id];
        }
        // The children of every entity at once, rather than a look over
        // the whole area for each one under `id`.
        let children = self.hierarchy();
        let mut out = vec![id];
        let mut seen = std::collections::HashSet::from([id]);
        let mut i = 0;
        while i < out.len() {
            // A damaged area could loop; never visit an entity twice.
            for &child in children.get(&Some(out[i])).map_or(&[][..], Vec::as_slice) {
                if seen.insert(child) {
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

    /// Every entity's children at once, in sibling order (`None` holds the
    /// top level), worked out in one pass, for drawing the whole tree.
    pub fn hierarchy(&self) -> std::collections::HashMap<Option<EntityId>, Vec<EntityId>> {
        let mut map: std::collections::HashMap<Option<EntityId>, Vec<(&str, EntityId)>> =
            std::collections::HashMap::new();
        for (&id, e) in &self.entities {
            let parent = e.parent.filter(|p| self.entities.contains_key(p));
            map.entry(parent).or_default().push((e.order.as_str(), id));
        }
        map.into_iter()
            .map(|(parent, mut kids)| {
                kids.sort();
                (parent, kids.into_iter().map(|(_, id)| id).collect())
            })
            .collect()
    }

    /// Every entity in tree order: each parent, then its children, as the
    /// Area panel lists them. Entities caught in a loop of a damaged area
    /// come last, so nothing is ever left out.
    pub fn depth_first(&self) -> Vec<EntityId> {
        crate::Tree::of(self).order().to_vec()
    }

    /// Which of `ids` comes first in tree order (the Area panel's order):
    /// how "the first sun" or "the first camera" is chosen everywhere, in
    /// the editor and the game alike. Cheap for one candidate; with a
    /// [`crate::Tree`] kept, [`crate::Tree::first_of`] is cheap for many.
    pub fn first_in_tree(&self, ids: impl IntoIterator<Item = EntityId>) -> Option<EntityId> {
        let ids: Vec<EntityId> = ids.into_iter().collect();
        match ids.as_slice() {
            [] => None,
            [one] => Some(*one),
            _ => crate::Tree::of(self).first_of(ids),
        }
    }

    /// A sibling order that sorts after every child of `parent` now, so a
    /// new entity goes at the end of the list.
    pub fn order_at_end(&self, parent: Option<EntityId>) -> String {
        let last = self
            .entities
            .values()
            .filter(|e| e.parent.filter(|p| self.entities.contains_key(p)) == parent)
            .map(|e| e.order.as_str())
            .max();
        order_after(last)
    }

    /// Where `id` is in the world: its transform after its parents'. An
    /// entity without a `Transform` sits where its parent is. A loop in a
    /// damaged area stops the walk instead of hanging.
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

/// A short text that sorts after `last` (or the first one, "a0").
///
/// Orders written by Solis are a counting number in base 36 (digits, then
/// lower-case letters), led by a letter for how many digits follow ("a"
/// one, "b" two…), so they sort as numbers do and stay short: "a0" to
/// "az", then "b10", and so on. Anything else (a hand-edited file) gets a
/// "1" added, which still sorts after it.
pub fn order_after(last: Option<&str>) -> String {
    const DIGITS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let Some(last) = last.filter(|l| !l.is_empty()) else {
        return "a0".to_string();
    };
    let parsed = (|| {
        let (head, digits) = last.split_at_checked(1)?;
        let len = usize::from(head.as_bytes()[0].checked_sub(b'a')?) + 1;
        if len > 12 || digits.len() != len {
            return None;
        }
        u64::from_str_radix(digits, 36).ok()
    })();
    let Some(n) = parsed.and_then(|n| n.checked_add(1)) else {
        return format!("{last}1");
    };
    let mut digits = Vec::new();
    let mut rest = n;
    loop {
        digits.push(DIGITS[(rest % 36) as usize]);
        rest /= 36;
        if rest == 0 {
            break;
        }
    }
    digits.reverse();
    let head = char::from(b'a' + (digits.len() - 1) as u8);
    format!("{head}{}", String::from_utf8_lossy(&digits))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtin::{Light, Transform};

    #[test]
    fn entities_are_spawned_found_and_removed() {
        let mut s = Area::new();
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
        let mut s = Area::new();
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
        let mut s = Area::new();
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
        let mut s = Area::new();
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

    #[test]
    fn orders_after_always_sort_later_and_stay_short() {
        let mut key = order_after(None);
        assert_eq!(key, "a0");
        for _ in 0..2_000 {
            let next = order_after(Some(&key));
            assert!(next > key, "{next} after {key}");
            key = next;
        }
        assert!(key.len() <= 4, "{key}");
        assert_eq!(order_after(Some("a9")), "aa");
        assert_eq!(order_after(Some("az")), "b10");
        // Hand-edited orders still get one that sorts after them.
        for odd in ["a~", "a0000", "zzz", "B"] {
            assert!(order_after(Some(odd)).as_str() > odd, "{odd}");
        }
    }

    #[test]
    fn the_tree_comes_out_in_order_and_new_entities_go_last() {
        let mut s = Area::new();
        let root = s.spawn(EntityData::new("Root"));
        let mut a = EntityData::new("A");
        a.parent = Some(root);
        a.order = s.order_at_end(Some(root));
        let a = s.spawn(a);
        let mut b = EntityData::new("B");
        b.parent = Some(root);
        b.order = s.order_at_end(Some(root));
        let b = s.spawn(b);
        let mut top = EntityData::new("Top");
        top.order = s.order_at_end(None);
        let top = s.spawn(top);
        assert_eq!(s.children(Some(root)), [a, b]);
        assert_eq!(s.depth_first(), [root, a, b, top]);
        assert_eq!(s.hierarchy()[&Some(root)], [a, b]);
        // A loop still lists everyone.
        s.get_mut(root).unwrap().parent = Some(b);
        assert_eq!(s.depth_first().len(), 4);
    }
}
