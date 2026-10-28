//! The open area as the editor holds it: the area document, the
//! registry of components, what's selected, and the undo history.
//! The area only ever changes through [`Document::edit`], so every change
//! can be undone ([decision 0022](../../../docs/decisions/0022-undo-as-reversible-edits.md)).

use area::builtin::{Model, Sprite, Transform};
use area::{Area, Edit, EditError, History, MergeKey, Tree};
use glam::Mat4;
use reflect::{Component, EntityId, Registry};
use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

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

impl area::Weigh for Selection {
    fn weight(&self) -> usize {
        // Each ID, and the set's own bookkeeping per entry.
        self.entities.len() * (std::mem::size_of::<EntityId>() * 2) + 32
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
    /// What changed since last asked: see [`Self::take_changes`].
    changes: Changes,
    /// A number that changes whenever what Problems checks does: see
    /// [`Self::shape`].
    shape: u64,
    /// The area's tree, kept until entities are added, deleted or given
    /// another parent: see [`Self::tree`].
    tree: RefCell<Option<Arc<Tree>>>,
    /// Where every entity is in the world, for one revision: see
    /// [`Self::world_matrices`].
    world: RefCell<Option<(u64, Placed)>>,
}

/// Where every entity is in the world, shared: see
/// [`Document::world_matrices`].
pub type Placed = Arc<HashMap<EntityId, Mat4>>;

/// What changed in the area since last asked.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Changes {
    /// Entities whose name or components changed.
    pub entities: std::collections::BTreeSet<reflect::EntityId>,
    /// Of those, the ones whose Transform changed (moved in the editor):
    /// the others keep where the running game put them.
    pub moved: std::collections::BTreeSet<reflect::EntityId>,
    /// Whether anything more changed: entities added, deleted or moved to
    /// another parent, an undo or redo, or a whole new area.
    pub everything: bool,
}

impl Changes {
    /// Notes what `edits` change.
    fn note(&mut self, edits: &[Edit]) {
        for edit in edits {
            match edit {
                Edit::SetField {
                    entity, component, ..
                }
                | Edit::AddComponent {
                    entity, component, ..
                }
                | Edit::RemoveComponent { entity, component } => {
                    self.entities.insert(*entity);
                    if component == Transform::NAME {
                        self.moved.insert(*entity);
                    }
                }
                Edit::Rename { entity, .. } | Edit::SetOrigin { entity, .. } => {
                    self.entities.insert(*entity);
                }
                _ => self.everything = true,
            }
        }
        // Past a few hundred, building everything again is as quick.
        if self.entities.len() > MAX_CHANGED_ENTITIES {
            self.everything = true;
            self.entities.clear();
            self.moved.clear();
        }
    }

    /// Whether nothing changed.
    pub fn is_empty(&self) -> bool {
        !self.everything && self.entities.is_empty()
    }
}

/// The most changed entities [`Changes`] lists; more count as everything.
const MAX_CHANGED_ENTITIES: usize = 512;

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
            changes: Changes {
                everything: true,
                ..Changes::default()
            },
            shape: next_revision(),
            tree: RefCell::new(None),
            world: RefCell::new(None),
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

    /// Notes that its file was moved to `file` (renamed or moved in the
    /// Library), or deleted (`None`): saving goes to the new place, or,
    /// with no file, asks where and counts as unsaved.
    pub fn file_moved(&mut self, file: Option<std::path::PathBuf>) {
        if file.is_none() {
            // No state of the history ever has this number.
            self.saved = u64::MAX;
        }
        self.file = file;
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
        let edits = self.replace_edits(&area);
        let selection = self.selection.clone();
        self.edit(label, edits, None, Some(selection))
    }

    /// Replaces the whole area with `area` for the running game (a save
    /// game loaded), not as an undo step: Stop puts the area back anyway.
    /// Returns how many edits couldn't be made.
    pub fn replace_by_game(&mut self, area: &Area) -> usize {
        let edits = self.replace_edits(area);
        self.edit_by_game(edits)
    }

    /// The edits that turn the area into `area`: everything there goes,
    /// and `area`'s entities come in, tree by tree.
    fn replace_edits(&self, area: &Area) -> Vec<Edit> {
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
        edits
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

    /// A number standing for what the Problems panel checks: it changes
    /// with every edit except moving, turning and resizing (a Transform's
    /// fields) and other component values, other than which model a
    /// Model shows and a Sprite's fields. So a drag doesn't look for
    /// problems every frame.
    pub fn shape(&self) -> u64 {
        self.shape
    }

    /// The area's tree: children in order, tree order, the first of some
    /// entities. Worked out once, and again only after entities are
    /// added, deleted or given another parent.
    pub fn tree(&self) -> Arc<Tree> {
        let mut kept = self.tree.borrow_mut();
        Arc::clone(kept.get_or_insert_with(|| Arc::new(Tree::of(&self.area))))
    }

    /// Where every entity is in the world, worked out once per
    /// [`Self::revision`] (parents first, one multiplication each), for
    /// everything that draws or measures the area in a frame.
    pub fn world_matrices(&self) -> Placed {
        if let Some((revision, world)) = &*self.world.borrow()
            && *revision == self.revision
        {
            return Arc::clone(world);
        }
        let world = Arc::new(self.tree().world_matrices(&self.area));
        *self.world.borrow_mut() = Some((self.revision, Arc::clone(&world)));
        world
    }

    /// Where `id` is in the world (identity for an entity not in the
    /// area), from [`Self::world_matrices`].
    pub fn world_matrix(&self, id: EntityId) -> Mat4 {
        self.world_matrices()
            .get(&id)
            .copied()
            .unwrap_or(Mat4::IDENTITY)
    }

    /// Notes a change to the area: a new revision; a new shape unless
    /// only component values changed (`values_only`); and the tree worked
    /// out again unless it's the same (`same_tree`).
    fn changed(&mut self, values_only: bool, same_tree: bool) {
        self.revision = next_revision();
        if !values_only {
            self.shape = next_revision();
        }
        if !same_tree {
            *self.tree.get_mut() = None;
        }
    }

    /// What changed in the area since this was last asked (everything,
    /// for a new document).
    pub fn take_changes(&mut self) -> Changes {
        std::mem::take(&mut self.changes)
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
        let mut changes = Changes::default();
        changes.note(&edits);
        // Only values, other than which model a Model shows and a
        // Sprite's fields (its sheet).
        let values_only = edits.iter().all(|e| {
            matches!(e, Edit::SetField { component, .. }
                if component != Model::NAME && component != Sprite::NAME)
        });
        let same_tree = !edits.iter().any(|e| {
            matches!(
                e,
                Edit::Spawn { .. } | Edit::Despawn { .. } | Edit::Reparent { .. }
            )
        });
        self.history
            .apply(&mut self.area, label, edits, merge, before, after.clone())?;
        self.changed(values_only, same_tree);
        self.changes.entities.extend(changes.entities);
        self.changes.moved.extend(changes.moved);
        self.changes.everything |= changes.everything;
        self.selection = after;
        self.selection.retain_existing(&self.area);
        Ok(())
    }

    /// Changes the area by `edits` made by the running game (copies of
    /// assemblies it spawned, entities it took out), each on its own and
    /// not as an undo step: Stop puts the area back anyway. They aren't
    /// [`Self::take_changes`]' changes: the game made them in its world
    /// too. Returns how many couldn't be made (an entity already there,
    /// or gone).
    pub fn edit_by_game(&mut self, edits: Vec<Edit>) -> usize {
        let mut failed = 0;
        for edit in edits {
            if self.area.apply(edit).is_err() {
                failed += 1;
            }
        }
        // The game's world has them already: they aren't changes for it
        // to take in, only for the editor's own views of the area.
        self.changed(false, false);
        self.selection.retain_existing(&self.area);
        failed
    }

    /// Undoes the newest step and brings back the selection it started
    /// with. Returns its label, or `None` if there was nothing to undo.
    pub fn undo(&mut self) -> Option<Result<String, EditError>> {
        let done = self.history.undo(&mut self.area)?;
        self.changed(false, false);
        self.changes.everything = true;
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
        self.changed(false, false);
        self.changes.everything = true;
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
        self.changed(false, false);
        self.changes.everything = true;
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
    fn the_kept_tree_and_places_follow_every_change() {
        use area::builtin::Transform;
        let mut d = Document::default();
        let id = EntityId::random();
        d.edit(
            "Add Box",
            vec![Edit::Spawn {
                entities: vec![(id, EntityData::new("Box").with(&Transform::default()))],
            }],
            None,
            None,
        )
        .unwrap();
        let tree = d.tree();
        assert!(tree.order().contains(&id));
        assert!(Arc::ptr_eq(&tree, &d.tree()), "kept, not worked out again");
        let shape = d.shape();
        // Moving it: the same tree and shape, a new place.
        d.edit(
            "Move",
            vec![Edit::SetField {
                entity: id,
                component: "Transform".into(),
                field: "position".into(),
                value: Some(reflect::Value::vec3(glam::Vec3::X)),
            }],
            None,
            None,
        )
        .unwrap();
        assert!(Arc::ptr_eq(&tree, &d.tree()));
        assert_eq!(d.shape(), shape, "Problems needn't look again");
        assert_eq!(d.world_matrix(id), d.area().world_matrix(id));
        assert_eq!(d.world_matrix(id).w_axis.x, 1.0);
        // Renaming changes the shape; deleting changes the tree too.
        d.edit(
            "Rename",
            vec![Edit::Rename {
                entity: id,
                name: "Crate".into(),
            }],
            None,
            None,
        )
        .unwrap();
        assert_ne!(d.shape(), shape);
        d.edit("Delete", vec![Edit::Despawn { root: id }], None, None)
            .unwrap();
        assert!(!d.tree().order().contains(&id));
        assert_eq!(d.world_matrix(id), Mat4::IDENTITY);
        // Undo brings it back, and the tree follows.
        d.undo().unwrap().unwrap();
        assert!(d.tree().order().contains(&id));
        assert_eq!(d.world_matrix(id).w_axis.x, 1.0);
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

    #[test]
    fn changes_name_the_entities_touched_or_say_everything() {
        let mut d = Document::default();
        assert!(d.take_changes().everything, "a new area is all new");
        assert!(d.take_changes().is_empty(), "taken once");
        let sun = d.area().iter().next().unwrap().0;
        d.edit(
            "Rename",
            vec![Edit::Rename {
                entity: sun,
                name: "Big sun".into(),
            }],
            None,
            None,
        )
        .unwrap();
        let changes = d.take_changes();
        assert!(!changes.everything);
        assert_eq!(changes.entities.into_iter().collect::<Vec<_>>(), [sun]);
        d.edit(
            "Add",
            vec![Edit::Spawn {
                entities: vec![(EntityId::random(), EntityData::new("Crate"))],
            }],
            None,
            None,
        )
        .unwrap();
        assert!(d.take_changes().everything);
        d.undo();
        assert!(d.take_changes().everything);
        // A failed edit changes nothing.
        let missing = EntityId::random();
        assert!(
            d.edit(
                "Rename",
                vec![Edit::Rename {
                    entity: missing,
                    name: "x".into()
                }],
                None,
                None
            )
            .is_err()
        );
        assert!(d.take_changes().is_empty());
    }
}
