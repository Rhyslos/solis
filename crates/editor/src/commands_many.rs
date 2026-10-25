//! Properties' commands for several entities at once, and copying and
//! pasting components' values. Each is one undo step, however many
//! entities it changes (decision 0022).

use crate::commands::merge_key;
use crate::console::LogLevel;
use crate::inspector::{degrees_of, rotation_of};
use crate::workbench::Workbench;
use area::Edit;
use reflect::{ComponentData, EntityId, FieldKind, Value};

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
            .document
            .registry
            .get(component)
            .and_then(|s| s.fields.iter().find(|f| f.name == field))
            .cloned()
        else {
            return;
        };
        let area = self.document.area();
        let edits: Vec<Edit> = entities
            .iter()
            .filter_map(|&entity| {
                let data = area.get(entity)?.components.get(component)?;
                let old = data.get(field).unwrap_or(&info.default);
                let new = match (part, &info.kind) {
                    (Some((i, x)), FieldKind::Vec3) if i < 3 => {
                        let mut v = old.as_floats::<3>().unwrap_or([0.0; 3]);
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
                (new != *old).then(|| Edit::SetField {
                    entity,
                    component: component.to_string(),
                    field: field.to_string(),
                    value: Some(new),
                })
            })
            .collect();
        let label = format!("Change {component}.{field} on {} entities", entities.len());
        let merge = merge_key((component, field, entities), gesture);
        self.command(&label, edits, Some(merge), None);
    }

    /// Gives every one of `entities` that lacks it the component, with its
    /// starting values.
    pub(crate) fn add_component_many(&mut self, entities: &[EntityId], component: &str) {
        let Some(schema) = self.document.registry.get(component) else {
            return;
        };
        let data = ComponentData::defaults(schema);
        let area = self.document.area();
        let edits: Vec<Edit> = entities
            .iter()
            .filter(|&&e| area.get(e).is_some_and(|d| !d.has(component)))
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
        let area = self.document.area();
        let edits: Vec<Edit> = entities
            .iter()
            .filter(|&&e| area.get(e).is_some_and(|d| d.has(component)))
            .map(|&entity| Edit::RemoveComponent {
                entity,
                component: component.to_string(),
            })
            .collect();
        self.command(&format!("Remove {component}"), edits, None, None);
    }

    /// Copies `entity`'s components' values (or only `only`'s), to paste.
    pub(crate) fn copy_components(&mut self, entity: EntityId, only: Option<&str>) {
        let Some(data) = self.document.area().get(entity) else {
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
        self.copied = copied;
        self.log(LogLevel::Info, format!("Copied {what} from {}.", data.name));
    }

    /// Pastes the copied values (or only `only`'s) onto `entities`: fields
    /// that differ are set, and components an entity lacks are added.
    pub(crate) fn paste_components(&mut self, entities: &[EntityId], only: Option<&str>) {
        let area = self.document.area();
        let mut edits = Vec::new();
        for (name, values) in &self.copied {
            if only.is_some_and(|o| o != name) {
                continue;
            }
            for &entity in entities {
                let Some(data) = area.get(entity) else {
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

    /// Puts every field of the component back to its starting value, on
    /// each of `entities`.
    pub(crate) fn reset_component(&mut self, entities: &[EntityId], component: &str) {
        let Some(schema) = self.document.registry.get(component) else {
            return;
        };
        let area = self.document.area();
        let mut edits = Vec::new();
        for &entity in entities {
            let Some(have) = area.get(entity).and_then(|d| d.components.get(component)) else {
                continue;
            };
            for field in &schema.fields {
                if have.get(&field.name) != Some(&field.default) {
                    edits.push(Edit::SetField {
                        entity,
                        component: component.to_string(),
                        field: field.name.clone(),
                        value: Some(field.default.clone()),
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
