//! Reversible edits to the area document
//! ([decision 0022](../../../docs/decisions/0022-undo-as-reversible-edits.md)).
//! Every change the editor makes is one of these. Applying an edit gives
//! back its inverse, which is how undo works; the same edits will describe
//! changes in the Git panel and in assembly overrides.

use crate::document::{Area, EntityData, Origin};
use reflect::{ComponentData, EntityId, Value};
use std::fmt;

/// One reversible change to an area.
#[derive(Debug, Clone, PartialEq)]
pub enum Edit {
    /// Sets one field of a component, or clears it with `None` (it then
    /// reads as the field's starting value).
    SetField {
        /// The entity.
        entity: EntityId,
        /// The component's name.
        component: String,
        /// The field's name.
        field: String,
        /// The new value.
        value: Option<Value>,
    },
    /// Adds a component the entity doesn't have.
    AddComponent {
        /// The entity.
        entity: EntityId,
        /// The component's name.
        component: String,
        /// Its values.
        data: ComponentData,
    },
    /// Takes a component off an entity.
    RemoveComponent {
        /// The entity.
        entity: EntityId,
        /// The component's name.
        component: String,
    },
    /// Adds entities: the first, and any below it (each later one's parent
    /// is earlier in the list). None of the IDs may be in use.
    Spawn {
        /// The entities, parents first.
        entities: Vec<(EntityId, EntityData)>,
    },
    /// Deletes an entity and everything below it.
    Despawn {
        /// The entity.
        root: EntityId,
    },
    /// Renames an entity.
    Rename {
        /// The entity.
        entity: EntityId,
        /// Its new name.
        name: String,
    },
    /// Links an entity to the assembly it comes from, or unlinks it
    /// (Unpack).
    SetOrigin {
        /// The entity.
        entity: EntityId,
        /// Where it comes from, or `None` for a plain entity.
        origin: Option<Origin>,
    },
    /// Moves an entity under a new parent (or to the top level), at
    /// `order` among its new siblings.
    Reparent {
        /// The entity.
        entity: EntityId,
        /// Its new parent.
        parent: Option<EntityId>,
        /// Its place among its new siblings.
        order: String,
    },
}

/// Why an edit couldn't be applied. The area is left unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditError {
    /// The entity isn't in the area.
    NoEntity(EntityId),
    /// The entity has no such component.
    NoComponent(EntityId, String),
    /// The entity already has that component.
    HasComponent(EntityId, String),
    /// An entity with that ID is already in the area.
    IdInUse(EntityId),
    /// The entities to spawn aren't one tree, parents first.
    NotATree,
    /// The new parent is the entity itself or below it.
    Loop(EntityId),
}

impl fmt::Display for EditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoEntity(id) => write!(f, "entity {id} isn't in the area"),
            Self::NoComponent(id, c) => write!(f, "entity {id} has no {c}"),
            Self::HasComponent(id, c) => write!(f, "entity {id} already has a {c}"),
            Self::IdInUse(id) => write!(f, "the ID {id} is already in use"),
            Self::NotATree => write!(f, "the new entities aren't one tree"),
            Self::Loop(id) => write!(f, "{id} can't go under itself or its own children"),
        }
    }
}

impl std::error::Error for EditError {}

impl Edit {
    /// About how many bytes this edit keeps alive, for limiting history by
    /// memory.
    pub fn bytes(&self) -> usize {
        let base = std::mem::size_of::<Self>();
        base + match self {
            Self::SetField {
                component,
                field,
                value,
                ..
            } => component.len() + field.len() + value.as_ref().map_or(0, value_bytes),
            Self::AddComponent {
                component, data, ..
            } => component.len() + data_bytes(data),
            Self::RemoveComponent { component, .. } => component.len(),
            Self::Spawn { entities } => entities.iter().map(|(_, e)| entity_bytes(e)).sum(),
            Self::Despawn { .. } => 0,
            Self::Rename { name, .. } => name.len(),
            Self::Reparent { order, .. } => order.len(),
            Self::SetOrigin { origin, .. } => origin.as_ref().map_or(0, |o| o.path.len() * 8 + 16),
        }
    }
}

fn value_bytes(value: &Value) -> usize {
    std::mem::size_of::<Value>()
        + match value {
            Value::Text(t) => t.len(),
            Value::List(items) => items.iter().map(value_bytes).sum(),
            _ => 0,
        }
}

fn data_bytes(data: &ComponentData) -> usize {
    data.fields
        .iter()
        .map(|(name, value)| name.len() + value_bytes(value))
        .sum()
}

fn entity_bytes(entity: &EntityData) -> usize {
    std::mem::size_of::<(EntityId, EntityData)>()
        + entity.name.len()
        + entity.order.len()
        + entity
            .components
            .iter()
            .map(|(name, data)| name.len() + data_bytes(data))
            .sum::<usize>()
}

impl Area {
    /// Applies `edit` and returns the edit that undoes it. On an error the
    /// area is unchanged.
    pub fn apply(&mut self, edit: Edit) -> Result<Edit, EditError> {
        match edit {
            Edit::SetField {
                entity,
                component,
                field,
                value,
            } => {
                let data = self
                    .get_mut(entity)
                    .ok_or(EditError::NoEntity(entity))?
                    .components
                    .get_mut(&component)
                    .ok_or_else(|| EditError::NoComponent(entity, component.clone()))?;
                let old = match value {
                    Some(value) => data.fields.insert(field.clone(), value),
                    None => data.fields.remove(&field),
                };
                Ok(Edit::SetField {
                    entity,
                    component,
                    field,
                    value: old,
                })
            }
            Edit::AddComponent {
                entity,
                component,
                data,
            } => {
                let e = self.get_mut(entity).ok_or(EditError::NoEntity(entity))?;
                if e.components.contains_key(&component) {
                    return Err(EditError::HasComponent(entity, component));
                }
                e.components.insert(component.clone(), data);
                Ok(Edit::RemoveComponent { entity, component })
            }
            Edit::RemoveComponent { entity, component } => {
                let e = self.get_mut(entity).ok_or(EditError::NoEntity(entity))?;
                let data = e
                    .components
                    .remove(&component)
                    .ok_or_else(|| EditError::NoComponent(entity, component.clone()))?;
                Ok(Edit::AddComponent {
                    entity,
                    component,
                    data,
                })
            }
            Edit::Spawn { entities } => {
                let Some(&(root, _)) = entities.first() else {
                    return Err(EditError::NotATree);
                };
                // The IDs seen so far, so a big spawn (a model of a
                // thousand parts unpacked) is checked in one pass.
                let mut earlier = std::collections::HashSet::with_capacity(entities.len());
                for (i, (id, data)) in entities.iter().enumerate() {
                    if self.contains(*id) || earlier.contains(id) {
                        return Err(EditError::IdInUse(*id));
                    }
                    let parent_earlier = data.parent.is_some_and(|p| earlier.contains(&p));
                    if i > 0 && !parent_earlier {
                        return Err(EditError::NotATree);
                    }
                    earlier.insert(*id);
                }
                for (id, data) in entities {
                    self.insert(id, data);
                }
                Ok(Edit::Despawn { root })
            }
            Edit::Despawn { root } => {
                if !self.contains(root) {
                    return Err(EditError::NoEntity(root));
                }
                Ok(Edit::Spawn {
                    entities: self.remove_tree(root),
                })
            }
            Edit::Rename { entity, name } => {
                let e = self.get_mut(entity).ok_or(EditError::NoEntity(entity))?;
                let old = std::mem::replace(&mut e.name, name);
                Ok(Edit::Rename { entity, name: old })
            }
            Edit::SetOrigin { entity, origin } => {
                let e = self.get_mut(entity).ok_or(EditError::NoEntity(entity))?;
                let old = std::mem::replace(&mut e.origin, origin);
                Ok(Edit::SetOrigin {
                    entity,
                    origin: old,
                })
            }
            Edit::Reparent {
                entity,
                parent,
                order,
            } => {
                if !self.contains(entity) {
                    return Err(EditError::NoEntity(entity));
                }
                if let Some(p) = parent {
                    if !self.contains(p) {
                        return Err(EditError::NoEntity(p));
                    }
                    // A loop if the new parent is the entity or under it:
                    // a walk up from the parent, which is as long as the
                    // tree is deep (and stops in a damaged area's loop).
                    let mut at = Some(p);
                    let mut steps = 0;
                    while let Some(current) = at {
                        if current == entity {
                            return Err(EditError::Loop(entity));
                        }
                        steps += 1;
                        if steps > self.len() {
                            break;
                        }
                        at = self.get(current).and_then(|e| e.parent);
                    }
                }
                let e = self.get_mut(entity).ok_or(EditError::NoEntity(entity))?;
                let old_parent = std::mem::replace(&mut e.parent, parent);
                let old_order = std::mem::replace(&mut e.order, order);
                Ok(Edit::Reparent {
                    entity,
                    parent: old_parent,
                    order: old_order,
                })
            }
        }
    }

    /// Applies `edits` in order and returns the edits that undo them all
    /// (in the order to apply them). If one fails, those before it are
    /// undone, so the area is unchanged.
    pub fn apply_all(&mut self, edits: Vec<Edit>) -> Result<Vec<Edit>, EditError> {
        let mut undo = Vec::with_capacity(edits.len());
        for edit in edits {
            match self.apply(edit) {
                Ok(inverse) => undo.push(inverse),
                Err(why) => {
                    for inverse in undo.into_iter().rev() {
                        // Undoing what was just done can't fail.
                        let _ = self.apply(inverse);
                    }
                    return Err(why);
                }
            }
        }
        undo.reverse();
        Ok(undo)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtin::{Light, Transform};

    fn sample() -> (Area, EntityId, EntityId) {
        let mut s = Area::new();
        let a = s.spawn(EntityData::new("A").with(&Transform::default()));
        let mut child = EntityData::new("B").with(&Light::point());
        child.parent = Some(a);
        let b = s.spawn(child);
        (s, a, b)
    }

    #[test]
    fn every_edit_is_undone_by_its_inverse() {
        let (mut s, a, b) = sample();
        let before = s.clone();
        let new_id = EntityId::random();
        let edits = vec![
            Edit::SetField {
                entity: b,
                component: "Light".into(),
                field: "strength".into(),
                value: Some(Value::Float(5.0)),
            },
            Edit::SetField {
                entity: b,
                component: "Light".into(),
                field: "range".into(),
                value: None,
            },
            Edit::AddComponent {
                entity: a,
                component: "Health".into(),
                data: ComponentData::default(),
            },
            Edit::RemoveComponent {
                entity: b,
                component: "Light".into(),
            },
            Edit::Rename {
                entity: a,
                name: "Renamed".into(),
            },
            Edit::Reparent {
                entity: b,
                parent: None,
                order: "z".into(),
            },
            Edit::Spawn {
                entities: vec![(new_id, EntityData::new("New"))],
            },
            Edit::SetOrigin {
                entity: b,
                origin: Some(Origin {
                    assembly: reflect::AssetId::random(),
                    instance: a,
                    path: vec![b],
                }),
            },
            Edit::Despawn { root: a },
        ];
        for edit in edits {
            let inverse = s.apply(edit.clone()).unwrap();
            let changed = s.clone();
            let again = s.apply(inverse).unwrap();
            assert_eq!(s, before, "{edit:?}");
            // The inverse of the inverse redoes it.
            s.apply(again).unwrap();
            assert_eq!(s, changed, "{edit:?}");
            s = before.clone();
        }
    }

    #[test]
    fn bad_edits_change_nothing() {
        let (mut s, a, b) = sample();
        let before = s.clone();
        let ghost = EntityId::random();
        let bad = [
            Edit::Rename {
                entity: ghost,
                name: "x".into(),
            },
            Edit::SetField {
                entity: a,
                component: "Light".into(),
                field: "strength".into(),
                value: None,
            },
            Edit::AddComponent {
                entity: a,
                component: "Transform".into(),
                data: ComponentData::default(),
            },
            Edit::Spawn {
                entities: vec![(a, EntityData::new("Clash"))],
            },
            Edit::Spawn { entities: vec![] },
            Edit::Spawn {
                entities: vec![
                    (ghost, EntityData::new("Root")),
                    (EntityId::random(), EntityData::new("Not under it")),
                ],
            },
            Edit::Reparent {
                entity: a,
                parent: Some(b),
                order: "a0".into(),
            },
            Edit::Reparent {
                entity: a,
                parent: Some(a),
                order: "a0".into(),
            },
            Edit::Despawn { root: ghost },
        ];
        for edit in bad {
            assert!(s.apply(edit.clone()).is_err(), "{edit:?}");
            assert_eq!(s, before, "{edit:?}");
        }
    }

    #[test]
    fn a_failed_batch_is_rolled_back() {
        let (mut s, a, _) = sample();
        let before = s.clone();
        let result = s.apply_all(vec![
            Edit::Rename {
                entity: a,
                name: "Changed".into(),
            },
            Edit::Despawn {
                root: EntityId::random(),
            },
        ]);
        assert!(result.is_err());
        assert_eq!(s, before);
        let undo = s
            .apply_all(vec![
                Edit::Rename {
                    entity: a,
                    name: "One".into(),
                },
                Edit::Rename {
                    entity: a,
                    name: "Two".into(),
                },
            ])
            .unwrap();
        s.apply_all(undo).unwrap();
        assert_eq!(s, before);
    }

    #[test]
    fn despawn_takes_the_whole_tree_and_spawn_brings_it_back() {
        let (mut s, a, b) = sample();
        let inverse = s.apply(Edit::Despawn { root: a }).unwrap();
        assert!(s.is_empty());
        let Edit::Spawn { entities } = &inverse else {
            panic!("{inverse:?}")
        };
        assert_eq!(
            entities.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
            [a, b]
        );
        assert!(inverse.bytes() > 0);
        s.apply(inverse).unwrap();
        assert_eq!(s.get(b).unwrap().parent, Some(a));
    }
}
