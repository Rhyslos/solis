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

    /// Adds `id`, making it the one Properties shows, or takes it out if
    /// it's already selected.
    pub fn toggle(&mut self, id: EntityId) {
        if self.entities.remove(&id) {
            if self.primary == Some(id) {
                self.primary = self.entities.iter().next().copied();
            }
        } else {
            self.entities.insert(id);
            self.primary = Some(id);
        }
    }

    /// Selects exactly `ids`, showing `primary` in Properties.
    pub fn select_all(&mut self, ids: impl IntoIterator<Item = EntityId>, primary: EntityId) {
        self.entities = ids.into_iter().collect();
        self.entities.insert(primary);
        self.primary = Some(primary);
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
    /// The file it was opened from or last saved to.
    file: Option<std::path::PathBuf>,
    /// The history's state when it was last saved (or opened).
    saved: u64,
    /// A number that changes whenever the area does: see
    /// [`Self::revision`].
    revision: u64,
}

/// Revision numbers, shared by every document, so no two areas (or two
/// versions of one) ever have the same one.
static NEXT_REVISION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

fn next_revision() -> u64 {
    NEXT_REVISION.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
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
            file: None,
            saved: 0,
            revision: next_revision(),
        }
    }

    /// A document for `area`, read from `file`: nothing to save yet.
    pub fn opened(area: Area, file: std::path::PathBuf) -> Self {
        Self {
            file: Some(file),
            ..Self::new(area)
        }
    }

    /// The file it belongs to, if it was opened or saved.
    pub fn file(&self) -> Option<&std::path::Path> {
        self.file.as_deref()
    }

    /// Its name: its file's, without the extension, or "Untitled area".
    pub fn name(&self) -> String {
        self.file.as_ref().and_then(|f| f.file_stem()).map_or_else(
            || "Untitled area".to_string(),
            |n| n.to_string_lossy().into_owned(),
        )
    }

    /// Whether it changed since it was opened or last saved.
    pub fn is_dirty(&self) -> bool {
        self.history.state() != self.saved
    }

    /// Notes that it was just saved to `file`.
    pub fn mark_saved(&mut self, file: std::path::PathBuf) {
        self.file = Some(file);
        self.saved = self.history.state();
    }

    /// Replaces the whole area with `area` (its file changed on disk), as
    /// one undo step called `label`; still counts as saved, since it now
    /// matches the file.
    pub fn reload(&mut self, area: Area, label: &str) -> Result<(), EditError> {
        self.replace_all(area, label)?;
        self.saved = self.history.state();
        Ok(())
    }

    /// Replaces the whole area with `area` (copies rebuilt from a changed
    /// assembly), as one undo step called `label`. If nothing was unsaved,
    /// nothing is now: the area's file would be the same.
    pub fn rebuild(&mut self, area: Area, label: &str) -> Result<(), EditError> {
        let clean = !self.is_dirty();
        self.replace_all(area, label)?;
        if clean {
            self.saved = self.history.state();
        }
        Ok(())
    }

    fn replace_all(&mut self, area: Area, label: &str) -> Result<(), EditError> {
        let mut edits: Vec<Edit> = Vec::new();
        // Everything there now goes, tree by tree…
        let old = &self.area;
        let mut gone = std::collections::HashSet::new();
        for id in old.depth_first() {
            if gone.contains(&id) {
                continue;
            }
            for inner in old.tree(id) {
                gone.insert(inner);
            }
            edits.push(Edit::Despawn { root: id });
        }
        // …and the file's entities come in, tree by tree.
        let mut placed = std::collections::HashSet::new();
        for id in area.depth_first() {
            if placed.contains(&id) {
                continue;
            }
            let entities: Vec<_> = area
                .tree(id)
                .into_iter()
                .filter(|e| placed.insert(*e))
                .filter_map(|e| area.get(e).map(|data| (e, data.clone())))
                .collect();
            edits.push(Edit::Spawn { entities });
        }
        let selection = self.selection.clone();
        self.edit(label, edits, None, Some(selection))
    }

    /// The area.
    pub fn area(&self) -> &Area {
        &self.area
    }

    /// A number standing for the area exactly as it is now: it changes
    /// with every edit, undo and redo (each frame of a drag too, though a
    /// drag is one undo step), and no other document ever has it. What's
    /// worked out from the area (the running game, Problems) is worked
    /// out again when it changes.
    pub fn revision(&self) -> u64 {
        self.revision
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
        self.revision = next_revision();
        self.selection = after;
        self.selection.retain_existing(&self.area);
        Ok(())
    }

    /// Undoes the newest step and brings back the selection it started
    /// with. Returns its label, or `None` if there was nothing to undo.
    pub fn undo(&mut self) -> Option<Result<String, EditError>> {
        let done = self.history.undo(&mut self.area)?;
        self.revision = next_revision();
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
        self.revision = next_revision();
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

    /// Gives the area a new revision number, as if it changed (it was put
    /// back after playing).
    pub(crate) fn touch(&mut self) {
        self.revision = next_revision();
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
