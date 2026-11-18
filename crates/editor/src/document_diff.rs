//! The fewest edits that turn one level into another: what goes, what
//! comes, and for each entity in both only what differs (its name, its
//! link to a prefab, its parent and place, its components and fields).
//! Rebuilding a prefab's copies changes a little of many entities, and
//! taking every entity out and putting it back made saving a prefab with
//! a thousand copies take seconds (R5's scale case).

use level::{Edit, EntityData, Level};
use reflect::EntityId;
use std::collections::HashSet;

/// The edits that turn `old` into `new`, in an order that applies: what
/// goes first, then what comes (parents first), then what changes.
pub(crate) fn edits_between(old: &Level, new: &Level) -> Vec<Edit> {
    let mut edits = Vec::new();
    // What goes: the top of each tree that goes.
    let gone: HashSet<EntityId> = old
        .iter()
        .map(|(id, _)| id)
        .filter(|id| !new.contains(*id))
        .collect();
    for id in old.depth_first() {
        let parent_goes = old
            .get(id)
            .and_then(|e| e.parent)
            .is_some_and(|p| gone.contains(&p));
        if gone.contains(&id) && !parent_goes {
            edits.push(Edit::Despawn { root: id });
        }
    }
    // What comes: each new tree, parents first, under a parent there.
    let fresh: HashSet<EntityId> = new
        .iter()
        .map(|(id, _)| id)
        .filter(|id| !old.contains(*id))
        .collect();
    let mut placed = HashSet::new();
    for id in new.depth_first() {
        if !fresh.contains(&id) || placed.contains(&id) {
            continue;
        }
        let entities: Vec<(EntityId, EntityData)> = new
            .tree(id)
            .into_iter()
            .filter(|e| fresh.contains(e) && placed.insert(*e))
            .filter_map(|e| new.get(e).map(|data| (e, data.clone())))
            .collect();
        edits.push(Edit::Spawn { entities });
    }
    // What changes, entity by entity.
    for (id, now) in new.iter() {
        if let Some(was) = old.get(id)
            && was != now
        {
            changes(id, was, now, &mut edits);
        }
    }
    edits
}

/// The edits that make entity `id` `now` where it was `was`.
fn changes(id: EntityId, was: &EntityData, now: &EntityData, edits: &mut Vec<Edit>) {
    if was.name != now.name {
        edits.push(Edit::Rename {
            entity: id,
            name: now.name.clone(),
        });
    }
    if was.origin != now.origin {
        edits.push(Edit::SetOrigin {
            entity: id,
            origin: now.origin.clone(),
        });
    }
    if was.parent != now.parent || was.order != now.order {
        edits.push(Edit::Reparent {
            entity: id,
            parent: now.parent,
            order: now.order.clone(),
        });
    }
    for component in was.components.keys() {
        if !now.components.contains_key(component) {
            edits.push(Edit::RemoveComponent {
                entity: id,
                component: component.clone(),
            });
        }
    }
    for (component, data) in &now.components {
        let Some(before) = was.components.get(component) else {
            edits.push(Edit::AddComponent {
                entity: id,
                component: component.clone(),
                data: data.clone(),
            });
            continue;
        };
        for (field, value) in &data.fields {
            if before.fields.get(field) != Some(value) {
                edits.push(Edit::SetField {
                    entity: id,
                    component: component.clone(),
                    field: field.clone(),
                    value: Some(value.clone()),
                });
            }
        }
        for field in before.fields.keys() {
            if !data.fields.contains_key(field) {
                edits.push(Edit::SetField {
                    entity: id,
                    component: component.clone(),
                    field: field.clone(),
                    value: None,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use level::builtin::{Light, Transform};

    fn apply(level: &mut Level, edits: Vec<Edit>) {
        for edit in edits {
            level.apply(edit).unwrap();
        }
    }

    #[test]
    fn the_edits_make_the_one_level_the_other() {
        let mut old = Level::new();
        let keep = old.spawn(EntityData::new("Keep").with(&Transform::default()));
        let go = old.spawn(EntityData::new("Go"));
        let mut under = EntityData::new("Under go");
        under.parent = Some(go);
        old.spawn(under);
        // Taken out, renamed, a component added and a field changed, and
        // a new tree under the kept one.
        let mut new = Level::new();
        for (id, e) in old.iter() {
            if id != go && e.parent != Some(go) {
                new.insert(id, e.clone());
            }
        }
        let k = new.get_mut(keep).unwrap();
        k.name = "Kept".into();
        k.set(&Light::point());
        k.set(&Transform {
            position: glam::Vec3::X,
            ..Transform::default()
        });
        let mut child = EntityData::new("Child");
        child.parent = Some(keep);
        let child = new.spawn(child);
        let mut grandchild = EntityData::new("Grandchild");
        grandchild.parent = Some(child);
        new.spawn(grandchild);
        let edits = edits_between(&old, &new);
        assert!(edits.len() <= 6, "{edits:?}");
        apply(&mut old, edits);
        assert_eq!(old, new);
    }

    #[test]
    fn the_same_level_needs_no_edits() {
        let level = level::builtin::starter_level();
        assert!(edits_between(&level, &level).is_empty());
    }
}
