//! The open scene as the editor holds it: the scene document, the
//! registry of components, and what's selected.

use reflect::{EntityId, Registry};
use scene::Scene;
use std::collections::BTreeSet;

/// What's selected: a set of entities, and the one Properties shows.
/// Never saved in files (decision 0022).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Selection {
    entities: BTreeSet<EntityId>,
    primary: Option<EntityId>,
}

impl Selection {
    /// Selects only `id`.
    pub fn select_only(&mut self, id: EntityId) {
        self.entities.clear();
        self.entities.insert(id);
        self.primary = Some(id);
    }

    /// Selects nothing.
    pub fn clear(&mut self) {
        self.entities.clear();
        self.primary = None;
    }

    /// The entity Properties shows.
    pub fn primary(&self) -> Option<EntityId> {
        self.primary
    }

    /// Whether `id` is selected.
    pub fn contains(&self, id: EntityId) -> bool {
        self.entities.contains(&id)
    }

    /// Every selected entity, by ID.
    pub fn iter(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.entities.iter().copied()
    }

    /// Forgets entities that are gone from `scene`.
    pub fn retain_existing(&mut self, scene: &Scene) {
        self.entities.retain(|&id| scene.contains(id));
        if self.primary.is_some_and(|id| !scene.contains(id)) {
            self.primary = self.entities.iter().next().copied();
        }
    }
}

/// The open scene, its components' schemas, and the selection.
#[derive(Debug, Clone)]
pub struct Document {
    /// The scene.
    pub scene: Scene,
    /// Every component the editor knows.
    pub registry: Registry,
    /// What's selected.
    pub selection: Selection,
}

impl Default for Document {
    fn default() -> Self {
        Self {
            scene: scene::builtin::starter_scene(),
            // The built-in schemas are fixed and tested; an error here would
            // be a bug caught by the tests, never by users.
            registry: scene::builtin::registry().unwrap_or_default(),
            selection: Selection::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scene::EntityData;

    #[test]
    fn a_new_document_has_the_starter_scene_and_every_built_in() {
        let d = Document::default();
        assert_eq!(d.scene.len(), 2);
        assert!(d.registry.get("Light").is_some());
        assert_eq!(d.selection.primary(), None);
    }

    #[test]
    fn the_selection_forgets_entities_that_are_gone() {
        let mut d = Document::default();
        let id = d.scene.spawn(EntityData::new("Box"));
        d.selection.select_only(id);
        assert!(d.selection.contains(id));
        d.scene.remove(id);
        d.selection.retain_existing(&d.scene);
        assert_eq!(d.selection.primary(), None);
        assert_eq!(d.selection.iter().count(), 0);
    }
}
