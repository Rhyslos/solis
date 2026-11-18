//! Properties' commands for several entities at once, and copying and
//! pasting components' values. Each is one undo step, however many
//! entities it changes (decision 0022).

use crate::commands::merge_key;
use crate::console::LogLevel;
use crate::inspector::{degrees_of, rotation_of};
use crate::workbench::Workbench;
use level::Edit;
use reflect::{Component, ComponentData, EntityId, FieldKind, Value};

impl Workbench {
    /// Sets a field on every one of `entities` that has the component: the
    /// whole `value`, or only number `part` of three (a position's X, a
    /// rotation's degrees around Y) on each, keeping the rest of its own.
    pub(crate) fn set_field_many(
        &mut self,
        entities: &[EntityId],
        component: &str,
        field: &str,
        value: &Value,
        part: Option<(usize, f64)>,
        gesture: u64,
    ) {
        let Some(info) = self
            .open
            .document
            .registry
            .get(component)
            .and_then(|s| s.fields.iter().find(|f| f.name == field))
            .cloned()
        else {
            return;
        };
        let level = self.open.document.level();
        // While the game plays, what's shown as an entity's value (in
        // Properties, by the Console's `get`) is the game's, which may
        // have moved on from the level's: setting the value the level
        // already holds is a change too, made so that it's given to the
        // game (`Changes::set`).
        let playing = self.open.playing.is_some();
        let edits: Vec<Edit> = entities
            .iter()
            .filter_map(|&entity| {
                let data = level.get(entity)?.components.get(component)?;
                let old = data.get(field).unwrap_or(&info.default);
                let new = match (part, &info.kind) {
                    (Some((i, x)), FieldKind::Vec3) if i < 3 => {
                        let mut v = old.as_floats::<3>().unwrap_or([0.0; 3]);
                        v[i] = x as f32;
                        Value::floats(&v)
                    }
                    (Some((i, x)), FieldKind::Vec2) if i < 2 => {
                        let mut v = old.as_floats::<2>().unwrap_or([0.0; 2]);
                        v[i] = x as f32;
                        Value::floats(&v)
                    }
                    (Some((i, x)), FieldKind::Rotation) if i < 3 => {
                        let mut d = degrees_of(old);
                        d[i] = x;
                        rotation_of(d)
                    }
                    _ => value.clone(),
                };
                (playing || new != *old).then(|| {
                    // A brush's Made as makes its mesh again too (B2).
                    let made_as = (component == level::builtin::Brush::NAME && field == "shape")
                        .then(|| self.made_as_edits(entity, &new))
                        .flatten();
                    made_as.unwrap_or_else(|| {
                        vec![Edit::SetField {
                            entity,
                            component: component.to_string(),
                            field: field.to_string(),
                            value: Some(new),
                        }]
                    })
                })
            })
            .flatten()
            .collect();
        let label = format!("Change {component}.{field} on {} entities", entities.len());
        let merge = merge_key((component, field, entities), gesture);
        self.command(&label, edits, Some(merge), None);
    }

    /// Gives every one of `entities` that lacks it the component, with its
    /// starting values.
    pub(crate) fn add_component_many(&mut self, entities: &[EntityId], component: &str) {
        // One ground per level (decision 0111): never on several at once,
        // nor beside the one the level has.
        if component == level::builtin::Ground::NAME
            && (entities.len() > 1 || self.level_ground().is_some())
        {
            self.log(
                crate::console::LogLevel::Warning,
                level::builtin::ONE_PER_LEVEL,
            );
            return;
        }
        let Some(schema) = self.open.document.registry.get(component) else {
            return;
        };
        let data = ComponentData::defaults(schema);
        let level = self.open.document.level();
        let edits: Vec<Edit> = entities
            .iter()
            .filter(|&&e| level.get(e).is_some_and(|d| !d.has(component)))
            .map(|&entity| Edit::AddComponent {
                entity,
                component: component.to_string(),
                data: data.clone(),
            })
            .collect();
        self.command(&format!("Add {component}"), edits, None, None);
    }

    /// Takes the component off every one of `entities` that has it.
    pub(crate) fn remove_component_many(&mut self, entities: &[EntityId], component: &str) {
        let level = self.open.document.level();
        let edits: Vec<Edit> = entities
            .iter()
            .filter(|&&e| level.get(e).is_some_and(|d| d.has(component)))
            .map(|&entity| Edit::RemoveComponent {
                entity,
                component: component.to_string(),
            })
            .collect();
        self.command(&format!("Remove {component}"), edits, None, None);
    }

    /// Copies `entity`'s components' values (or only `only`'s), to paste.
    pub(crate) fn copy_components(&mut self, entity: EntityId, only: Option<&str>) {
        let Some(data) = self.open.document.level().get(entity) else {
            return;
        };
        let copied: Vec<(String, ComponentData)> = data
            .components
            .iter()
            .filter(|(name, _)| only.is_none_or(|o| o == name.as_str()))
            .map(|(name, values)| (name.clone(), values.clone()))
            .collect();
        let what = match only {
            Some(name) => format!("{name}'s values"),
            None => format!("{} components' values", copied.len()),
        };
        self.open.copied = copied;
        self.log(LogLevel::Info, format!("Copied {what} from {}.", data.name));
    }

    /// Pastes the copied values (or only `only`'s) onto `entities`: fields
    /// that differ are set, and components an entity lacks are added.
    pub(crate) fn paste_components(&mut self, entities: &[EntityId], only: Option<&str>) {
        let level = self.open.document.level();
        let mut edits = Vec::new();
        for (name, values) in &self.open.copied {
            if only.is_some_and(|o| o != name) {
                continue;
            }
            for &entity in entities {
                let Some(data) = level.get(entity) else {
                    continue;
                };
                match data.components.get(name) {
                    None => edits.push(Edit::AddComponent {
                        entity,
                        component: name.clone(),
                        data: values.clone(),
                    }),
                    Some(have) => {
                        for (field, value) in &values.fields {
                            if have.get(field) != Some(value) {
                                edits.push(Edit::SetField {
                                    entity,
                                    component: name.clone(),
                                    field: field.clone(),
                                    value: Some(value.clone()),
                                });
                            }
                        }
                    }
                }
            }
        }
        let label = match only {
            Some(name) => format!("Paste {name}"),
            None => "Paste components".to_string(),
        };
        self.command(&label, edits, None, None);
    }

    /// Puts every field of the component back to its standard value
    /// (`standard`), on each of `entities`.
    pub(crate) fn reset_component(&mut self, entities: &[EntityId], component: &str) {
        let Some(schema) = self.open.document.registry.get(component) else {
            return;
        };
        let level = self.open.document.level();
        let mut edits = Vec::new();
        for &entity in entities {
            let Some(have) = level.get(entity).and_then(|d| d.components.get(component)) else {
                continue;
            };
            // A sun goes back to a sun's values, not a lamp's.
            let standard = crate::standard::of(component, have);
            for field in &schema.fields {
                let value = crate::standard::value(standard.as_ref(), field);
                if have.get(&field.name) != Some(value) {
                    edits.push(Edit::SetField {
                        entity,
                        component: component.to_string(),
                        field: field.name.clone(),
                        value: Some(value.clone()),
                    });
                }
            }
        }
        self.command(&format!("Reset {component}"), edits, None, None);
    }
}

#[cfg(test)]
#[path = "workbench_panels_tests.rs"]
mod tests;
