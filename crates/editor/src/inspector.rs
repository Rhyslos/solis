//! Fields drawn from a component's schema (decision 0019): the inspector
//! knows no component by name, so built-in, Lust and Lua components all
//! show the same way. Widgets never change the area: they report the
//! values the user typed or dragged, and the caller applies them (as undo
//! steps, decision 0022).

use crate::models::ModelInfo;
use crate::sprite_sheets::SpriteSheets;
use egui::{RichText, Ui};
use glam::{EulerRot, Quat};
use reflect::{AssetId, ComponentData, ComponentSchema, FieldInfo, FieldKind, Offer, Value};
use std::collections::BTreeMap;

/// What the user did to a field this frame.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum FieldChange {
    /// Gave it a new value.
    Set {
        /// The field's name.
        field: String,
        /// Its new value.
        value: Value,
        /// For a field of three numbers (a position, a rotation in
        /// degrees), which one was changed and to what: with several
        /// entities selected, only that one changes on each.
        part: Option<(usize, f64)>,
    },
    /// Put it back to its assembly's value.
    Revert(String),
    /// Made its value the assembly's, for every copy.
    Apply(String),
}

/// The order rotations are shown in: turning around Z (heading) last, so a
/// light's X tilts it from straight down and its Z turns it round.
const EULER: EulerRot = EulerRot::ZXY;

/// A labelled row: the field's name on the left, its widget after it.
pub(crate) fn row<R>(ui: &mut Ui, label: &str, help: &str, add: impl FnOnce(&mut Ui) -> R) -> R {
    row_marked(ui, label, help, false, add)
}

/// A row whose name is bold when `changed` (a placed assembly's copy that
/// differs from the assembly).
fn row_marked<R>(
    ui: &mut Ui,
    label: &str,
    help: &str,
    changed: bool,
    add: impl FnOnce(&mut Ui) -> R,
) -> R {
    ui.horizontal(|ui| {
        let text = if changed {
            RichText::new(label)
                .strong()
                .color(crate::theme::TEXT_STRONG)
        } else {
            RichText::new(label).color(crate::theme::TEXT_WEAK)
        };
        let label = ui.add_sized(egui::vec2(96.0, 18.0), egui::Label::new(text).truncate());
        if !help.is_empty() {
            label.on_hover_text(help);
        }
        add(ui)
    })
    .inner
}

/// Whether a field's value differs between the selected entities: the
/// whole value, and each of its three numbers (a position, or a rotation's
/// degrees).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Mixed {
    pub(crate) whole: bool,
    pub(crate) parts: [bool; 3],
}

impl Mixed {
    /// How `value` (the entity Properties shows) compares with the same
    /// field in `others`.
    pub(crate) fn of(field: &FieldInfo, value: &Value, others: &[&ComponentData]) -> Self {
        let mut mixed = Self::default();
        let mine = three(field, value);
        for other in others {
            let theirs = other.get(&field.name).unwrap_or(&field.default);
            if theirs == value {
                continue;
            }
            mixed.whole = true;
            if let (Some(a), Some(b)) = (mine, three(field, theirs)) {
                for i in 0..3 {
                    mixed.parts[i] |= (a[i] - b[i]).abs() > 1e-4;
                }
            } else {
                mixed.parts = [true; 3];
            }
        }
        mixed
    }
}

/// A field's three numbers as Properties shows them (a rotation in
/// degrees), if it has three.
fn three(field: &FieldInfo, value: &Value) -> Option<[f64; 3]> {
    match field.kind {
        FieldKind::Vec3 => value.as_floats::<3>().map(|v| v.map(f64::from)),
        FieldKind::Rotation => Some(degrees_of(value)),
        _ => None,
    }
}

/// A rotation's degrees around X, Y and Z, as Properties shows them.
pub(crate) fn degrees_of(value: &Value) -> [f64; 3] {
    let q = value
        .as_floats::<4>()
        .map(Quat::from_array)
        .filter(|q| q.length_squared() > 1e-12)
        .map_or(Quat::IDENTITY, Quat::normalize);
    let (z, x, y) = q.to_euler(EULER);
    [x, y, z].map(|a| f64::from(a.to_degrees()))
}

/// The rotation for degrees around X, Y and Z.
pub(crate) fn rotation_of(degrees: [f64; 3]) -> Value {
    let [x, y, z] = degrees.map(|d| (d as f32).to_radians());
    Value::quat(Quat::from_euler(EULER, z, x, y))
}

/// Draws every field of `schema` with its value in `data`, and returns what
/// the user did. Fields `data` doesn't have show their starting value. With
/// `assembly` (the component as the entity's assembly has it), fields that
/// differ are bold, with Revert and Apply buttons. `others` are the same
/// component on the other selected entities: values that differ show as
/// "—".
pub(crate) fn fields(
    ui: &mut Ui,
    schema: &ComponentSchema,
    data: &ComponentData,
    assembly: Option<&ComponentData>,
    others: &[&ComponentData],
    (models, sheets): (&BTreeMap<AssetId, ModelInfo>, &SpriteSheets),
    area: &area::Area,
) -> Vec<FieldChange> {
    let mut changes = Vec::new();
    for field in &schema.fields {
        let value = data.get(&field.name).unwrap_or(&field.default);
        let mixed = Mixed::of(field, value, others);
        let changed =
            assembly.is_some_and(|a| a.get(&field.name).unwrap_or(&field.default) != value);
        let id = ui.id().with((&schema.name, &field.name));
        ui.push_id(id, |ui| {
            let (set, revert, apply) =
                row_marked(ui, &field.label, field.hint.help, changed, |ui| {
                    let set = field_widget(ui, field, value, (models, sheets, data), area, mixed);
                    if !changed {
                        return (set, false, false);
                    }
                    let small = |ui: &mut Ui, text: &str, hover: &str, name: String| {
                        let button = ui
                            .add(
                                egui::Button::new(
                                    RichText::new(text).color(crate::theme::TEXT_WEAK),
                                )
                                .frame(false),
                            )
                            .on_hover_text(hover);
                        button.widget_info(|| {
                            egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &name)
                        });
                        button.clicked()
                    };
                    let revert = small(
                        ui,
                        "↺",
                        "Back to the assembly's value",
                        format!("Revert {}", field.label),
                    );
                    let apply = small(
                        ui,
                        "↑",
                        "Make this the assembly's value, for every copy",
                        format!("Apply {}", field.label),
                    );
                    (set, revert, apply)
                });
            if let Some((value, part)) = set {
                changes.push(FieldChange::Set {
                    field: field.name.clone(),
                    value,
                    part,
                });
            }
            if revert {
                changes.push(FieldChange::Revert(field.name.clone()));
            }
            if apply {
                changes.push(FieldChange::Apply(field.name.clone()));
            }
        });
    }
    changes
}

/// One field's widget; `Some` with the new value when the user changed it
/// (and, for three numbers, which one). Values that differ between the
/// selected entities (`mixed`) show as "—".
fn field_widget(
    ui: &mut Ui,
    field: &FieldInfo,
    value: &Value,
    (models, sheets, data): (&BTreeMap<AssetId, ModelInfo>, &SpriteSheets, &ComponentData),
    area: &area::Area,
    mixed: Mixed,
) -> Option<(Value, Option<(usize, f64)>)> {
    let whole = |v: Option<Value>| v.map(|v| (v, None));
    // What the schema says to offer (Phase 9, D2).
    match (&field.kind, field.hint.offers) {
        (FieldKind::Asset, Offer::Assets("sprite sheet")) => {
            return whole(sheet_picker(ui, value, sheets, mixed.whole));
        }
        (FieldKind::Text, Offer::SlicesOf(sheet_field)) => {
            let sheet = match data.get(sheet_field) {
                Some(Value::Asset(Some(r))) => sheets.get(r.asset),
                _ => None,
            };
            return whole(slice_picker(ui, value, sheet.map(|s| &**s), mixed.whole));
        }
        (FieldKind::Asset, Offer::Assets("animator")) => {
            return whole(asset_picker(
                ui,
                value,
                &sheets.listed_animators(),
                mixed.whole,
                (
                    "animator",
                    "No animators yet: Library > Import > New animator.",
                ),
            ));
        }
        (FieldKind::Asset, Offer::Assets("tile map")) => {
            return whole(asset_picker(
                ui,
                value,
                &sheets.listed_tiles(true),
                mixed.whole,
                (
                    "tile map",
                    "No tile maps yet: Library > Import > New tile map.",
                ),
            ));
        }
        (FieldKind::Text, Offer::StatesOf(animator_field)) => {
            let file = match data.get(animator_field) {
                Some(Value::Asset(Some(r))) => sheets.animator_file(r.asset),
                _ => None,
            };
            return whole(state_picker(ui, value, file.map(|f| &**f), mixed.whole));
        }
        (FieldKind::Text, Offer::ClipsOf(sheet_field)) => {
            let clips = match data.get(sheet_field) {
                Some(Value::Asset(Some(r))) => sheets.clips(r.asset),
                _ => None,
            };
            return whole(clip_picker(ui, value, clips.map(|c| &**c), mixed.whole));
        }
        _ => {}
    }
    match &field.kind {
        FieldKind::Bool => {
            let mut on = matches!(value, Value::Bool(true));
            whole(
                ui.add(egui::Checkbox::new(&mut on, "").indeterminate(mixed.whole))
                    .changed()
                    .then_some(Value::Bool(on)),
            )
        }
        FieldKind::Int => {
            let mut n = match value {
                Value::Int(n) => *n,
                _ => 0,
            };
            let mut drag = egui::DragValue::new(&mut n).suffix(field.hint.unit);
            if mixed.whole {
                drag = drag.custom_formatter(|_, _| "—".to_string());
            }
            whole(ui.add(drag).changed().then_some(Value::Int(n)))
        }
        FieldKind::Float => {
            let mut v = value.as_f64().unwrap_or(0.0);
            whole(number(ui, &mut v, field, mixed.whole).then_some(Value::Float(v)))
        }
        FieldKind::Text => {
            let mut text = match value {
                Value::Text(t) if !mixed.whole => t.clone(),
                _ => String::new(),
            };
            let edit = egui::TextEdit::singleline(&mut text).hint_text(if mixed.whole {
                "—"
            } else {
                ""
            });
            whole(ui.add(edit).changed().then_some(Value::Text(text)))
        }
        FieldKind::Choice(options) => {
            let current = match value {
                Value::Text(t) => t.clone(),
                _ => String::new(),
            };
            let mut chosen = None;
            let shown = if mixed.whole { "—" } else { current.as_str() };
            egui::ComboBox::from_id_salt("choice")
                .selected_text(shown)
                .show_ui(ui, |ui| {
                    for option in options {
                        let picked = *option == current && !mixed.whole;
                        if ui.selectable_label(picked, option).clicked()
                            && (*option != current || mixed.whole)
                        {
                            chosen = Some(Value::Text(option.clone()));
                        }
                    }
                });
            whole(chosen)
        }
        FieldKind::Vec3 => {
            let mut v = value.as_floats::<3>().unwrap_or([0.0; 3]).map(f64::from);
            let mut part = None;
            for (i, c) in v.iter_mut().enumerate() {
                if number(ui, c, field, mixed.parts[i]) {
                    part = Some((i, *c));
                }
            }
            part.map(|p| (Value::floats(&v.map(|c| c as f32)), Some(p)))
        }
        FieldKind::Rotation => {
            let mut degrees = degrees_of(value);
            let mut part = None;
            for (i, c) in degrees.iter_mut().enumerate() {
                if number(ui, c, field, mixed.parts[i]) {
                    part = Some((i, *c));
                }
            }
            part.map(|p| (rotation_of(degrees), Some(p)))
        }
        FieldKind::Color => {
            let c = value.as_floats::<3>().unwrap_or([1.0; 3]);
            let mut bytes = c.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8);
            whole(
                ui.color_edit_button_srgb(&mut bytes)
                    .changed()
                    .then(|| Value::floats(&bytes.map(|b| f32::from(b) / 255.0))),
            )
        }
        FieldKind::Asset => {
            let text = match value {
                Value::Asset(Some(r)) => models.get(&r.asset).map_or_else(
                    || format!("Missing asset {}", r.asset),
                    |m| {
                        let mut text =
                            format!("{} · {} tris", m.name, crate::models::grouped(m.triangles));
                        if !m.lods.is_empty() {
                            let levels: Vec<String> =
                                m.lods.iter().map(|&t| crate::models::grouped(t)).collect();
                            text.push_str(&format!(" · far away {}", levels.join(" → ")));
                        }
                        text
                    },
                ),
                _ => "None".to_string(),
            };
            ui.label(text);
            None
        }
        FieldKind::Entity => whole(entity_picker(ui, value, area, mixed.whole)),
    }
}

/// A sprite sheet field: the sheet's name, and the project's sheets (or
/// None) to pick another from.
fn sheet_picker(ui: &mut Ui, value: &Value, sheets: &SpriteSheets, mixed: bool) -> Option<Value> {
    let current = match value {
        Value::Asset(Some(r)) => Some(r.asset),
        _ => None,
    };
    let listed = sheets.listed();
    let shown = if mixed {
        "—".to_owned()
    } else {
        match current {
            None => "None".to_owned(),
            Some(id) => listed
                .iter()
                .find(|(_, i)| *i == id)
                .map_or_else(|| format!("Missing sheet {id}"), |(n, _)| n.clone()),
        }
    };
    let mut chosen = None;
    egui::ComboBox::from_id_salt("sheet")
        .selected_text(shown)
        .height(320.0)
        .show_ui(ui, |ui| {
            if ui
                .selectable_label(current.is_none() && !mixed, "None")
                .clicked()
            {
                chosen = Some(Value::Asset(None));
            }
            for (name, id) in &listed {
                if ui
                    .selectable_label(current == Some(*id) && !mixed, name)
                    .clicked()
                {
                    chosen = Some(Value::Asset(Some(reflect::AssetRef {
                        asset: *id,
                        part: None,
                    })));
                }
            }
            if listed.is_empty() {
                ui.label(
                    RichText::new("No sprite sheets yet: put an Aseprite file in the project.")
                        .color(crate::theme::TEXT_WEAK),
                );
            }
        });
    chosen
}

/// An asset field offering the files `listed` (animators, tile maps),
/// with `none` said when there are none (`what` names them).
fn asset_picker(
    ui: &mut Ui,
    value: &Value,
    listed: &[(String, AssetId)],
    mixed: bool,
    (what, none): (&str, &str),
) -> Option<Value> {
    let current = match value {
        Value::Asset(Some(r)) => Some(r.asset),
        _ => None,
    };
    let shown = if mixed {
        "—".to_owned()
    } else {
        match current {
            None => "None".to_owned(),
            Some(id) => listed
                .iter()
                .find(|(_, i)| *i == id)
                .map_or_else(|| format!("Missing {what} {id}"), |(n, _)| n.clone()),
        }
    };
    let mut chosen = None;
    egui::ComboBox::from_id_salt(what)
        .selected_text(shown)
        .show_ui(ui, |ui| {
            if ui
                .selectable_label(current.is_none() && !mixed, "None")
                .clicked()
            {
                chosen = Some(Value::Asset(None));
            }
            for (name, id) in listed {
                if ui
                    .selectable_label(current == Some(*id) && !mixed, name)
                    .clicked()
                {
                    chosen = Some(Value::Asset(Some(reflect::AssetRef {
                        asset: *id,
                        part: None,
                    })));
                }
            }
            if listed.is_empty() {
                ui.label(RichText::new(none).color(crate::theme::TEXT_WEAK));
            }
        });
    chosen
}

/// A state name field: the animator's states (and its start, for none).
fn state_picker(
    ui: &mut Ui,
    value: &Value,
    file: Option<&animation::AnimatorFile>,
    mixed: bool,
) -> Option<Value> {
    let current = match value {
        Value::Text(t) => t.as_str(),
        _ => "",
    };
    let none = match file.and_then(animation::AnimatorFile::entry_state) {
        Some(s) => format!("Its start ({})", s.name),
        None => "Its start".to_owned(),
    };
    let shown = if mixed {
        "—".to_owned()
    } else if current.is_empty() {
        none.clone()
    } else {
        current.to_owned()
    };
    let mut chosen = None;
    egui::ComboBox::from_id_salt("animator state")
        .selected_text(shown)
        .show_ui(ui, |ui| {
            if ui
                .selectable_label(current.is_empty() && !mixed, none)
                .clicked()
            {
                chosen = Some(Value::Text(String::new()));
            }
            for state in file.map_or(&[][..], |f| &f.states) {
                if ui
                    .selectable_label(current == state.name && !mixed, &state.name)
                    .clicked()
                {
                    chosen = Some(Value::Text(state.name.clone()));
                }
            }
            if !current.is_empty()
                && file.is_some_and(|f| !f.states.iter().any(|s| s.name == current))
            {
                ui.label(
                    RichText::new(format!("\"{current}\" isn't one of its states"))
                        .color(crate::theme::WARNING),
                );
            }
        });
    chosen
}

/// A slice name field: the sheet's slices (and "bottom middle", for
/// none) to pick from.
fn slice_picker(
    ui: &mut Ui,
    value: &Value,
    sheet: Option<&assets::sprite_sheet::SpriteSheet>,
    mixed: bool,
) -> Option<Value> {
    const NONE: &str = "Bottom middle";
    let current = match value {
        Value::Text(t) => t.as_str(),
        _ => "",
    };
    let shown = if mixed {
        "—"
    } else if current.is_empty() {
        NONE
    } else {
        current
    };
    let mut chosen = None;
    egui::ComboBox::from_id_salt("slice")
        .selected_text(shown)
        .show_ui(ui, |ui| {
            if ui
                .selectable_label(current.is_empty() && !mixed, NONE)
                .clicked()
            {
                chosen = Some(Value::Text(String::new()));
            }
            for slice in sheet.map_or(&[][..], |s| &s.slices) {
                if ui
                    .selectable_label(current == slice.name && !mixed, &slice.name)
                    .clicked()
                {
                    chosen = Some(Value::Text(slice.name.clone()));
                }
            }
            if !current.is_empty() && sheet.is_some_and(|s| s.slice(current).is_none()) {
                ui.label(
                    RichText::new(format!("\"{current}\" isn't a slice of this sheet"))
                        .color(crate::theme::WARNING),
                );
            }
        });
    chosen
}

/// A clip name field (Phase 9, D3): the sheet's or clips file's clips to
/// pick from, and its first clip (or the still Frame) for none.
fn clip_picker(
    ui: &mut Ui,
    value: &Value,
    clips: Option<&animation::ClipSet>,
    mixed: bool,
) -> Option<Value> {
    let current = match value {
        Value::Text(t) => t.as_str(),
        _ => "",
    };
    let first = clips
        .and_then(|c| c.first.and_then(|i| c.clips.get(i)))
        .map(|c| c.name.as_str());
    let none = match first {
        Some(name) => format!("First ({name})"),
        None => "None (the Frame)".to_owned(),
    };
    let shown = if mixed {
        "—".to_owned()
    } else if current.is_empty() {
        none.clone()
    } else {
        current.to_owned()
    };
    let mut chosen = None;
    egui::ComboBox::from_id_salt("clip")
        .selected_text(shown)
        .show_ui(ui, |ui| {
            if ui
                .selectable_label(current.is_empty() && !mixed, none)
                .clicked()
            {
                chosen = Some(Value::Text(String::new()));
            }
            for clip in clips.map_or(&[][..], |c| &c.clips) {
                if ui
                    .selectable_label(current == clip.name && !mixed, &clip.name)
                    .clicked()
                {
                    chosen = Some(Value::Text(clip.name.clone()));
                }
            }
            if clips.is_some_and(|c| c.clips.is_empty()) {
                ui.label(
                    RichText::new("No clips: tag runs of frames in Aseprite.")
                        .color(crate::theme::TEXT_WEAK),
                );
            }
            if !current.is_empty() && clips.is_some_and(|c| c.get(current).is_none()) {
                ui.label(
                    RichText::new(format!("\"{current}\" isn't one of its clips"))
                        .color(crate::theme::WARNING),
                );
            }
        });
    chosen
}

/// The most entities an entity field's list shows at once: type to find
/// the rest.
const MAX_LISTED: usize = 200;

/// An entity field: the entity's name, and a list to pick another (or
/// None) from, with a box to find one by name in a big area.
fn entity_picker(ui: &mut Ui, value: &Value, area: &area::Area, mixed: bool) -> Option<Value> {
    let current = match value {
        Value::Entity(Some(id)) => Some(*id),
        _ => None,
    };
    let shown = if mixed {
        "—".to_string()
    } else {
        match current {
            None => "None".to_string(),
            Some(id) => area
                .get(id)
                .map_or_else(|| "Missing entity".to_string(), |e| e.name.clone()),
        }
    };
    let find_id = ui.id().with("find");
    let mut find: String = ui.data(|d| d.get_temp(find_id)).unwrap_or_default();
    let mut chosen = None;
    let combo = egui::ComboBox::from_id_salt("entity")
        .selected_text(shown)
        .height(320.0)
        .show_ui(ui, |ui| {
            ui.add(
                egui::TextEdit::singleline(&mut find)
                    .hint_text("Find by name")
                    .desired_width(180.0),
            );
            if ui
                .selectable_label(current.is_none() && !mixed, "None")
                .clicked()
            {
                chosen = Some(Value::Entity(None));
            }
            let wanted = find.to_lowercase();
            let mut listed = 0usize;
            let mut more = false;
            for (id, entity) in area.iter() {
                if !wanted.is_empty() && !entity.name.to_lowercase().contains(&wanted) {
                    continue;
                }
                if listed == MAX_LISTED {
                    more = true;
                    break;
                }
                listed += 1;
                let picked = current == Some(id) && !mixed;
                if ui
                    .selectable_label(picked, &entity.name)
                    .on_hover_text(id.to_string())
                    .clicked()
                {
                    chosen = Some(Value::Entity(Some(id)));
                }
            }
            if more {
                ui.label(
                    RichText::new(format!("Only the first {MAX_LISTED}: type to find others."))
                        .color(crate::theme::TEXT_WEAK),
                );
            }
        });
    if let Some(id) = current {
        combo.response.on_hover_text(id.to_string());
    }
    ui.data_mut(|d| d.insert_temp(find_id, find));
    chosen.filter(|v| mixed || *v != *value)
}

/// A number field: drags within the hint's range, but never changes a
/// value that's outside it just by being shown (decision 0019). A value
/// that differs between the selected entities (`mixed`) shows as "—".
fn number(ui: &mut Ui, value: &mut f64, field: &FieldInfo, mixed: bool) -> bool {
    let mut drag = egui::DragValue::new(value)
        .speed(field.hint.speed.unwrap_or(0.1))
        .max_decimals(3)
        .suffix(field.hint.unit)
        .clamp_existing_to_range(false);
    if mixed {
        drag = drag.custom_formatter(|_, _| "—".to_string());
    }
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
        let sun = area::builtin::sun_rotation(150.0, 50.0);
        let (z, x, _) = sun.to_euler(EULER);
        assert!((x.to_degrees() - 40.0).abs() < 0.5, "{}", x.to_degrees());
        assert!((z.to_degrees() - 30.0).abs() < 0.5, "{}", z.to_degrees());
    }
}
