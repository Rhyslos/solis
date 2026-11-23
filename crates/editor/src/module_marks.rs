//! The marks on placed copies (board 07, panel 3): a link on each copy's
//! root in the Level panel, and an orange dot on a copy that changes some
//! settings and on each of its parts that does. A plain entity has none.
//!
//! What a copy changes is what a level's file would keep for it: the
//! copies alone are collapsed against their modules, worked out again
//! only when the level or the modules change, so plain entities cost
//! nothing however many there are.

use crate::Workbench;
use level::module::{Expander, derive};
use level::text::FileKind;
use reflect::EntityId;
use std::collections::HashSet;
use std::sync::Arc;

/// Which entities carry which marks.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct ModuleMarks {
    /// The roots of placed copies: a link each.
    pub(crate) linked: HashSet<EntityId>,
    /// Copies' roots and parts with changes of their own: a dot each.
    pub(crate) changed: HashSet<EntityId>,
}

impl ModuleMarks {
    /// `id`'s marks: (a link, a dot).
    pub(crate) fn of(&self, id: EntityId) -> (bool, bool) {
        (self.linked.contains(&id), self.changed.contains(&id))
    }
}

/// What the marks were last worked out for: the level's revision and the
/// modules' count of changes.
pub(crate) type MarksKey = (u64, u64);

impl Workbench {
    /// The marks for the open level, kept until it or its modules change.
    pub(crate) fn module_marks(&mut self) -> Arc<ModuleMarks> {
        let key = (self.open.document.revision(), self.sources);
        if let Some((kept, marks)) = &self.open.marks_kept
            && *kept == key
        {
            return Arc::clone(marks);
        }
        let marks = Arc::new(self.work_out_marks());
        self.open.marks_kept = Some((key, Arc::clone(&marks)));
        marks
    }

    fn work_out_marks(&self) -> ModuleMarks {
        let level = self.open.document.level();
        let mut copies = level::Level::new();
        let mut marks = ModuleMarks::default();
        for (id, entity) in level.iter() {
            let Some(origin) = &entity.origin else {
                continue;
            };
            if origin.path.is_empty() && origin.instance == id {
                marks.linked.insert(id);
            }
            copies.insert(id, entity.clone());
        }
        if marks.linked.is_empty() {
            return marks;
        }
        let lookup = |asset| self.open.modules.get(&asset).cloned();
        let Ok(file) = Expander::new(&lookup).collapse(&copies, FileKind::Level) else {
            return marks;
        };
        for ((root, path), changes) in &file.overrides {
            // Where a copy stands is its own (every copy's differs), so
            // its root's Transform alone isn't a changed setting here;
            // Properties still marks it.
            let placed_only = path.is_empty()
                && changes.remove.is_empty()
                && changes.components.keys().all(|c| c == "Transform");
            if changes.is_empty() || placed_only {
                continue;
            }
            marks.changed.insert(*root);
            if !path.is_empty() {
                marks.changed.insert(derive(*root, path));
            }
        }
        marks
    }
}

#[cfg(test)]
#[path = "module_marks_tests.rs"]
mod tests;
