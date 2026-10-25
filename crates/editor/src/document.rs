//! The open area as the editor holds it: the area document, the
//! registry of components, what's selected, and the undo history.
//! The area only ever changes through [`Document::edit`], so every change
//! can be undone ([decision 0022](../../../docs/decisions/0022-undo-as-reversible-edits.md)).

use area::{Area, Edit, EditError, History, MergeKey};
use reflect::{EntityId, Registry};
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

    /// Forgets entities that are gone from `area`.
    pub fn retain_existing(&mut self, area: &Area) {
        self.entities.retain(|&id| area.contains(id));
        if self.primary.is_some_and(|id| !area.contains(id)) {
            self.primary = self.entities.iter().next().copied();
        }
    }
}

/// The open area, its components' schemas, the selection and the undo
/// history.
#[derive(Debug, Clone)]
pub struct Document {
    /// The area: read it with [`Self::area`], change it with
    /// [`Self::edit`].
    area: Area,
    /// Every component the editor knows.
    pub registry: Registry,
    /// What's selected. Changing it is not an undo step.
    pub selection: Selection,
    history: History<Selection>,
}

impl Default for Document {
    fn default() -> Self {
        Self::new(area::builtin::starter_area())
    }
}

impl Document {
    /// A document for `area`, with the built-in components and no
    /// history.
    pub fn new(area: Area) -> Self {
        Self {
            area,
            // The built-in schemas are fixed and tested; an error here would
            // be a bug caught by the tests, never by users.
            registry: area::builtin::registry().unwrap_or_default(),
            selection: Selection::default(),
            history: History::default(),
        }
    }

    /// The area.
    pub fn area(&self) -> &Area {
        &self.area
    }

    /// Changes the area by `edits`, as one undo step called `label`.
    /// Steps with the same `merge` key in a row become one (a whole drag).
    /// `select` is the selection afterwards, if it changes. On an error
    /// nothing changes.
    pub fn edit(
        &mut self,
        label: &str,
        edits: Vec<Edit>,
        merge: Option<MergeKey>,
        select: Option<Selection>,
    ) -> Result<(), EditError> {
        let before = self.selection.clone();
        let after = select.unwrap_or_else(|| before.clone());
        self.history
            .apply(&mut self.area, label, edits, merge, before, after.clone())?;
        self.selection = after;
        self.selection.retain_existing(&self.area);
        Ok(())
    }

    /// Undoes the newest step and brings back the selection it started
    /// with. Returns its label, or `None` if there was nothing to undo.
    pub fn undo(&mut self) -> Option<Result<String, EditError>> {
        let done = self.history.undo(&mut self.area)?;
        Some(done.map(|done| {
            self.selection = done.selection;
            self.selection.retain_existing(&self.area);
            done.label
        }))
    }

    /// Redoes the step undone last and brings back the selection it ended
    /// with. Returns its label, or `None` if there was nothing to redo.
    pub fn redo(&mut self) -> Option<Result<String, EditError>> {
        let done = self.history.redo(&mut self.area)?;
        Some(done.map(|done| {
            self.selection = done.selection;
            self.selection.retain_existing(&self.area);
            done.label
        }))
    }

    /// The undo and redo steps.
    pub fn history(&self) -> &History<Selection> {
        &self.history
    }

    /// Ends the newest step, so the next edit starts a new one.
    pub fn close_step(&mut self) {
        self.history.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use area::EntityData;

    #[test]
    fn a_new_document_has_the_starter_area_and_every_built_in() {
        let d = Document::default();
        assert_eq!(d.area().len(), 2);
        assert!(d.registry.get("Light").is_some());
        assert_eq!(d.selection.primary(), None);
    }

    #[test]
    fn the_selection_forgets_entities_that_are_gone() {
        let mut d = Document::default();
        let mut area = d.area().clone();
        let id = area.spawn(EntityData::new("Box"));
        d.selection.select_only(id);
        assert!(d.selection.contains(id));
        let area = d.area().clone();
        d.selection.retain_existing(&area);
        assert_eq!(d.selection.primary(), None);
        assert_eq!(d.selection.iter().count(), 0);
    }

    #[test]
    fn undo_brings_back_the_area_and_the_selection() {
        let mut d = Document::default();
        let start = d.area().clone();
        let id = EntityId::random();
        let mut selected = Selection::default();
        selected.select_only(id);
        d.edit(
            "Add Box",
            vec![Edit::Spawn {
                entities: vec![(id, EntityData::new("Box"))],
            }],
            None,
            Some(selected.clone()),
        )
        .unwrap();
        assert_eq!(d.selection, selected);
        d.edit("Delete Box", vec![Edit::Despawn { root: id }], None, None)
            .unwrap();
        assert_eq!(d.selection.primary(), None, "gone, so not selected");
        assert_eq!(d.undo().unwrap().unwrap(), "Delete Box");
        assert_eq!(d.selection, selected, "selected again, as before the step");
        assert_eq!(d.undo().unwrap().unwrap(), "Add Box");
        assert_eq!(d.area(), &start);
        assert_eq!(d.selection.primary(), None);
        assert!(d.undo().is_none());
        assert_eq!(d.redo().unwrap().unwrap(), "Add Box");
        assert_eq!(d.selection, selected);
        // A bad edit changes nothing and leaves no step.
        let steps = d.history().undo_len();
        assert!(
            d.edit(
                "Bad",
                vec![Edit::Despawn {
                    root: EntityId::random()
                }],
                None,
                None
            )
            .is_err()
        );
        assert_eq!(d.history().undo_len(), steps);
    }
}
