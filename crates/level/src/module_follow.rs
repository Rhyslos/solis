//! Copies following a module whose settings' values changed, without
//! rebuilding the level (Shared 1f's loose ends).
//!
//! Saving a module usually changes values: a lamp's strength, a crate's
//! colour. Then each copy follows by a change to the same fields, wherever
//! it doesn't change them itself, which is what collapsing every copy to
//! its own changes and expanding it again from the new module would give,
//! at a fraction of the cost: no copy is made twice and no level is
//! compared whole. Saving a module with 1,000 copies took 14 ms that way,
//! against a frame's 8 ms.
//!
//! (The one difference: a copy that lacks a field the module didn't
//! change keeps lacking it, where the whole way fills it in; that field
//! wasn't this save's.)
//!
//! Anything else (a part added, removed, renamed or moved, a component
//! added or taken off, a value that names an entity) gives [`None`], and
//! the caller rebuilds the copies the whole way ([`crate::module`]).

use crate::document::{EntityData, Level};
use crate::edit::Edit;
use crate::module::Walk;
use reflect::{AssetId, EntityId, Value};
use std::collections::HashMap;

/// One field's value before and after, in one component of one part.
#[derive(Debug, Clone, PartialEq)]
struct Changed {
    component: String,
    field: String,
    was: Option<Value>,
    now: Option<Value>,
}

/// What changed between two versions of a module, when only its values
/// did: the fields that changed, by the path to their part.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ValueChanges {
    by_path: HashMap<Vec<EntityId>, Vec<Changed>>,
}

impl ValueChanges {
    /// Whether nothing changed.
    pub fn is_empty(&self) -> bool {
        self.by_path.is_empty()
    }

    /// The changes between `old` and `new`, each a module's contents
    /// expanded (as `Expander::module` gives them) with its root, when
    /// only values differ: the same parts at the same paths, with the same
    /// names, parents, places, links and components, and no changed value
    /// naming an entity. `None` otherwise. The root's name, parent and
    /// place are each copy's own, so they don't count.
    pub fn between(old: (&Level, EntityId), new: (&Level, EntityId)) -> Option<Self> {
        let (old_walk, new_walk) = (Walk::of(old.0, old.1), Walk::of(new.0, new.1));
        if old_walk.order.len() != new_walk.order.len() {
            return None;
        }
        let new_at: HashMap<&[EntityId], EntityId> = new_walk
            .paths
            .iter()
            .map(|(&id, path)| (path.as_slice(), id))
            .collect();
        let path_of = |walk: &Walk, id: Option<EntityId>| id.map(|id| walk.paths.get(&id).cloned());
        let mut changes = Self::default();
        for (&old_id, path) in &old_walk.paths {
            let new_id = *new_at.get(path.as_slice())?;
            let (was, now) = (old.0.get(old_id)?, new.0.get(new_id)?);
            let root = path.is_empty();
            let same_place = root
                || (was.name == now.name
                    && was.order == now.order
                    && path_of(&old_walk, was.parent) == path_of(&new_walk, now.parent));
            let same_link = was.origin.as_ref().map(|o| (o.module, &o.path))
                == now.origin.as_ref().map(|o| (o.module, &o.path));
            if !same_place || !same_link || !was.components.keys().eq(now.components.keys()) {
                return None;
            }
            let fields = changed_fields(was, now)?;
            if !fields.is_empty() {
                changes.by_path.insert(path.clone(), fields);
            }
        }
        Some(changes)
    }

    /// The edits that make every copy of `module` in `level` follow these
    /// changes: each changed field of each part, where the copy has the
    /// module's old value (or none); a field the copy changed itself
    /// keeps the copy's value, as a copy's own change wins.
    pub fn follow(&self, level: &Level, module: AssetId) -> Vec<Edit> {
        let mut edits = Vec::new();
        for (id, entity) in level.iter() {
            let Some(origin) = entity.origin.as_ref().filter(|o| o.module == module) else {
                continue;
            };
            let Some(fields) = self.by_path.get(&origin.path) else {
                continue;
            };
            for change in fields {
                // A component this copy took off stays off.
                let Some(data) = entity.components.get(&change.component) else {
                    continue;
                };
                let has = data.fields.get(&change.field);
                if has.is_some() && has != change.was.as_ref() {
                    continue;
                }
                edits.push(Edit::SetField {
                    entity: id,
                    component: change.component.clone(),
                    field: change.field.clone(),
                    value: change.now.clone(),
                });
            }
        }
        edits
    }
}

/// The fields whose values differ between `was` and `now` (which have the
/// same components), or `None` if one of those values names an entity,
/// which each copy points at its own parts.
fn changed_fields(was: &EntityData, now: &EntityData) -> Option<Vec<Changed>> {
    let mut changed = Vec::new();
    for (component, before) in &was.components {
        let after = &now.components[component];
        let names = before.fields.keys().chain(after.fields.keys());
        let mut seen = std::collections::BTreeSet::new();
        for field in names.filter(|f| seen.insert(*f)) {
            let (a, b) = (before.fields.get(field), after.fields.get(field));
            if a == b {
                continue;
            }
            if a.is_some_and(names_entity) || b.is_some_and(names_entity) {
                return None;
            }
            changed.push(Changed {
                component: component.clone(),
                field: field.clone(),
                was: a.cloned(),
                now: b.cloned(),
            });
        }
    }
    Some(changed)
}

/// Whether `value` is, or holds, a reference to an entity.
fn names_entity(value: &Value) -> bool {
    match value {
        Value::Entity(_) => true,
        Value::List(items) => items.iter().any(names_entity),
        _ => false,
    }
}

#[cfg(test)]
#[path = "module_follow_tests.rs"]
mod tests;
