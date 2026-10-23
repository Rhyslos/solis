//! Fields drawn from a component's schema (decision 0019): the inspector
//! knows no component by name, so built-in, Lust and Lua components all
//! show the same way. Widgets never change the scene: they report the
//! values the user typed or dragged, and the caller applies them (as undo
//! steps, decision 0022).

use crate::models::ModelInfo;
use egui::{RichText, Ui};
use glam::{EulerRot, Quat};
use reflect::{AssetId, ComponentData, ComponentSchema, FieldInfo, FieldKind, Value};
use std::collections::BTreeMap;

/// A field the user changed this frame, and its new value.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FieldChange {
    /// The field's name.
    pub field: String,
    /// Its new value.
    pub value: Value,
}

/// The order rotations are shown in: turning around Z (heading) last, so a
/// light's X tilts it from straight down and its Z turns it round.
const EULER: EulerRot = EulerRot::ZXY;

/// A labelled row: the field's name on the left, its widget after it.
pub(crate) fn row<R>(ui: &mut Ui, label: &str, help: &str, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.horizontal(|ui| {
        let label = ui.add_sized(
            egui::vec2(96.0, 18.0),
            egui::Label::new(RichText::new(label).color(crate::theme::TEXT_WEAK)).truncate(),
        );
        if !help.is_empty() {
            label.on_hover_text(help);
        }
        add(ui)
    })
    .inner
}

/// Draws every field of `schema` with its value in `data`, and returns the
/// fields the user changed. Fields `data` doesn't have show their starting
/// value.
pub(crate) fn fields(
    ui: &mut Ui,
    schema: &ComponentSchema,
    data: &ComponentData,
    models: &BTreeMap<AssetId, ModelInfo>,
) -> Vec<FieldChange> {
    let mut changes = Vec::new();
    for field in &schema.fields {
        let value = data.get(&field.name).unwrap_or(&field.default);
        let id = ui.id().with((&schema.name, &field.name));
        ui.push_id(id, |ui| {
            if let Some(value) = field_widget(ui, field, value, models) {
                changes.push(FieldChange {
                    field: field.name.clone(),
                    value,
                });
            }
        });
    }
    changes
}

/// One field's widget; `Some` with the new value when the user changed it.
fn field_widget(
    ui: &mut Ui,
    field: &FieldInfo,
    value: &Value,
    models: &BTreeMap<AssetId, ModelInfo>,
) -> Option<Value> {
    let help = field.hint.help;
    row(ui, &field.label, help, |ui| match &field.kind {
        FieldKind::Bool => {
            let mut on = matches!(value, Value::Bool(true));
            ui.checkbox(&mut on, "")
                .changed()
                .then_some(Value::Bool(on))
        }
        FieldKind::Int => {
            let mut n = match value {
                Value::Int(n) => *n,
                _ => 0,
            };
            ui.add(egui::DragValue::new(&mut n).suffix(field.hint.unit))
                .changed()
                .then_some(Value::Int(n))
        }
        FieldKind::Float => {
            let mut v = value.as_f64().unwrap_or(0.0);
            number(ui, &mut v, field).then_some(Value::Float(v))
        }
        FieldKind::Text => {
            let mut text = match value {
                Value::Text(t) => t.clone(),
                _ => String::new(),
            };
            ui.text_edit_singleline(&mut text)
                .changed()
                .then_some(Value::Text(text))
        }
        FieldKind::Choice(options) => {
            let current = match value {
                Value::Text(t) => t.clone(),
                _ => String::new(),
            };
            let mut chosen = None;
            egui::ComboBox::from_id_salt("choice")
                .selected_text(&current)
                .show_ui(ui, |ui| {
                    for option in options {
                        if ui.selectable_label(*option == current, option).clicked()
                            && *option != current
                        {
                            chosen = Some(Value::Text(option.clone()));
                        }
                    }
                });
            chosen
        }
        FieldKind::Vec3 => {
            let mut v = value.as_floats::<3>().unwrap_or([0.0; 3]).map(f64::from);
            let mut changed = false;
            for c in &mut v {
                changed |= number(ui, c, field);
            }
            changed.then(|| Value::floats(&v.map(|c| c as f32)))
        }
        FieldKind::Rotation => {
            let q = value
                .as_floats::<4>()
                .map(Quat::from_array)
                .filter(|q| q.length_squared() > 1e-12)
                .map_or(Quat::IDENTITY, Quat::normalize);
            let (z, x, y) = q.to_euler(EULER);
            let mut degrees = [x, y, z].map(|a| f64::from(a.to_degrees()));
            let mut changed = false;
            for c in &mut degrees {
                changed |= number(ui, c, field);
            }
            changed.then(|| {
                let [x, y, z] = degrees.map(|d| (d as f32).to_radians());
                Value::quat(Quat::from_euler(EULER, z, x, y))
            })
        }
        FieldKind::Color => {
            let c = value.as_floats::<3>().unwrap_or([1.0; 3]);
            let mut bytes = c.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8);
            ui.color_edit_button_srgb(&mut bytes)
                .changed()
                .then(|| Value::floats(&bytes.map(|b| f32::from(b) / 255.0)))
        }
        FieldKind::Asset => {
            let text = match value {
                Value::Asset(Some(r)) => models.get(&r.asset).map_or_else(
                    || format!("Missing asset {}", r.asset),
                    |m| format!("{} · {} tris", m.name, crate::models::grouped(m.triangles)),
                ),
                _ => "None".to_string(),
            };
            ui.label(text);
            None
        }
        FieldKind::Entity => {
            let text = match value {
                Value::Entity(Some(id)) => id.to_string(),
                _ => "None".to_string(),
            };
            ui.label(text);
            None
        }
    })
}

/// A number field: drags within the hint's range, but never changes a
/// value that's outside it just by being shown (decision 0019).
fn number(ui: &mut Ui, value: &mut f64, field: &FieldInfo) -> bool {
    let mut drag = egui::DragValue::new(value)
        .speed(field.hint.speed.unwrap_or(0.1))
        .max_decimals(3)
        .suffix(field.hint.unit)
        .clamp_existing_to_range(false);
    if let Some((low, high)) = field.hint.range {
        drag = drag.range(low..=high);
    }
    ui.add(drag).changed()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotations_read_back_through_degrees() {
        for (x, y, z) in [
            (40.0f32, 0.0f32, 30.0f32),
            (0.0, 0.0, 0.0),
            (-20.0, 10.0, 170.0),
        ] {
            let q = Quat::from_euler(EULER, z.to_radians(), x.to_radians(), y.to_radians());
            let (rz, rx, ry) = q.to_euler(EULER);
            assert!((rx.to_degrees() - x).abs() < 1e-3, "{x} {y} {z}");
            assert!((ry.to_degrees() - y).abs() < 1e-3);
            assert!((rz.to_degrees() - z).abs() < 1e-3);
        }
        // The starting sun reads as a tilt of 40° turned 30° round.
        let sun = scene::builtin::sun_rotation(150.0, 50.0);
        let (z, x, _) = sun.to_euler(EULER);
        assert!((x.to_degrees() - 40.0).abs() < 0.5, "{}", x.to_degrees());
        assert!((z.to_degrees() - 30.0).abs() < 0.5, "{}", z.to_degrees());
    }
}
