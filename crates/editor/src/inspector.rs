//! Fields drawn from a component's schema (decision 0019): the inspector
//! knows no component by name, so built-in, Lust and Lua components all
//! show the same way. Widgets never change the area: they report the
//! values the user typed or dragged, and the caller applies them (as undo
//! steps, decision 0022).

pub(crate) use crate::field_rows::fields;
use crate::field_rows::row_marked;
use crate::models::ModelInfo;
use crate::sprite_sheets::SpriteSheets;
use crate::tokens::{size, text};
use crate::widgets;
use egui::{RichText, Ui};
use glam::{EulerRot, Quat};
use reflect::{AssetId, ComponentData, FieldInfo, FieldKind, Offer, Value};
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
    row_marked(ui, (label, help), (false, false, false), add).0
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
/// degrees, a vec2 with a last 0 it doesn't show), if it has them.
fn three(field: &FieldInfo, value: &Value) -> Option<[f64; 3]> {
    match field.kind {
        FieldKind::Vec3 => value.as_floats::<3>().map(|v| v.map(f64::from)),
        FieldKind::Vec2 => value
            .as_floats::<2>()
            .map(|[x, y]| [f64::from(x), f64::from(y), 0.0]),
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

/// Whether a field's value isn't its standard one, for the kinds where
/// that's worth a dot: a number, a switch, words, a choice or a colour. A
/// place, a turn or a picked file starts empty and is meant to be set, so
/// it isn't marked.
pub(crate) fn moved_from_default(field: &FieldInfo, value: &Value) -> bool {
    matches!(
        field.kind,
        FieldKind::Bool
            | FieldKind::Int
            | FieldKind::Float
            | FieldKind::Text
            | FieldKind::Choice(_)
            | FieldKind::Color
    ) && value != &field.default
}

/// A value in a few characters, for a reset button: `3`, `0.5`, `on`,
/// `Walk`.
pub(crate) fn short(value: &Value) -> String {
    match value {
        Value::Bool(true) => "on".into(),
        Value::Bool(false) => "off".into(),
        Value::Int(i) => i.to_string(),
        Value::Float(f) => {
            let text = format!("{f:.3}");
            let text = text.trim_end_matches('0').trim_end_matches('.');
            if text == "-0" {
                "0".into()
            } else {
                text.into()
            }
        }
        Value::Text(t) if t.is_empty() => "empty".into(),
        Value::Text(t) => {
            let mut cut: String = t.chars().take(12).collect();
            if t.chars().count() > 12 {
                cut.push('…');
            }
            cut
        }
        _ => String::new(),
    }
}

/// "shared by 3" after a file used by more than one entity (`users`,
/// from `Document::asset_users`), so changing the file isn't a surprise
/// (decision 0085).
pub(crate) fn shared_by(ui: &mut Ui, users: usize) {
    if users > 1 {
        egui::Frame::new()
            .stroke(egui::Stroke::new(1.0, crate::theme::button_border()))
            .corner_radius(4.0)
            .inner_margin(egui::Margin::symmetric(5, 1))
            .show(ui, |ui| {
                ui.label(
                    RichText::new(format!("shared by {users}"))
                        .color(crate::theme::text_weak())
                        .size(11.0),
                )
            })
            .response
            .on_hover_text(format!(
                "{users} entities in this area use this file: a change to the file changes \
                 them all"
            ));
    }
}

/// One field's widget; `Some` with the new value when the user changed it
/// (and, for three numbers, which one). Values that differ between the
/// selected entities (`mixed`) show as "—".
pub(crate) fn field_widget(
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
            return whole(asset_picker(
                ui,
                (&field.label, value, mixed.whole),
                &sheets.listed(),
                (
                    "sprite sheet",
                    "No sprite sheets yet: put an Aseprite file in the project.",
                ),
            ));
        }
        (FieldKind::Text, Offer::SlicesOf(sheet_field)) => {
            let sheet = match data.get(sheet_field) {
                Some(Value::Asset(Some(r))) => sheets.get(r.asset),
                _ => None,
            };
            return whole(slice_picker(
                ui,
                (&field.label, value, mixed.whole),
                sheet.map(|s| &**s),
            ));
        }
        (FieldKind::Asset, Offer::Assets("animator")) => {
            return whole(asset_picker(
                ui,
                (&field.label, value, mixed.whole),
                &sheets.listed_animators(),
                (
                    "animator",
                    "No animators yet: right-click in the Library > New > Animator.",
                ),
            ));
        }
        (FieldKind::Asset, Offer::Assets("tile map")) => {
            return whole(asset_picker(
                ui,
                (&field.label, value, mixed.whole),
                &sheets.listed_tiles(true),
                (
                    "tile map",
                    "No tile maps yet: right-click in the Library > New > Tile map.",
                ),
            ));
        }
        (FieldKind::Text, Offer::StatesOf(animator_field)) => {
            let file = match data.get(animator_field) {
                Some(Value::Asset(Some(r))) => sheets.animator_file(r.asset),
                _ => None,
            };
            return whole(state_picker(
                ui,
                (&field.label, value, mixed.whole),
                file.map(|f| &**f),
            ));
        }
        (FieldKind::Text, Offer::DayColours) => {
            return whole(crate::day_colours::day_colours(ui, value, mixed.whole));
        }
        (FieldKind::Text, Offer::Screens) => {
            return whole(screen_picker(
                ui,
                (&field.label, value, mixed.whole),
                &sheets.screens,
            ));
        }
        (FieldKind::Text, Offer::ClipsOf(sheet_field)) => {
            let clips = match data.get(sheet_field) {
                Some(Value::Asset(Some(r))) => sheets.clips(r.asset),
                _ => None,
            };
            return whole(clip_picker(
                ui,
                (&field.label, value, mixed.whole),
                clips.map(|c| &**c),
            ));
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
            let width = ui.available_width();
            whole(
                widgets::field::framed(ui, width, None, |ui| ui.add(drag))
                    .changed()
                    .then_some(Value::Int(n)),
            )
        }
        FieldKind::Float => {
            let mut v = value.as_f64().unwrap_or(0.0);
            let width = ui.available_width();
            whole(number(ui, &mut v, field, (mixed.whole, None, width)).then_some(Value::Float(v)))
        }
        FieldKind::Text => {
            let mut text = match value {
                Value::Text(t) if !mixed.whole => t.clone(),
                _ => String::new(),
            };
            let edit = egui::TextEdit::singleline(&mut text)
                .frame(egui::Frame::NONE)
                .margin(egui::Margin::ZERO)
                .font(text::ribbon_small())
                .desired_width(f32::INFINITY)
                .hint_text(if mixed.whole { "—" } else { "" });
            let width = ui.available_width();
            whole(
                widgets::field::framed(ui, width, None, |ui| ui.add(edit))
                    .changed()
                    .then_some(Value::Text(text)),
            )
        }
        FieldKind::Choice(options) => {
            let current = match value {
                Value::Text(t) => t.clone(),
                _ => String::new(),
            };
            let mut chosen = None;
            let shown = if mixed.whole { "—" } else { current.as_str() };
            let width = ui.available_width();
            let button = widgets::field::dropdown_button(ui, &field.label, shown, width);
            egui::Popup::menu(&button).width(width).show(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                for option in options {
                    let picked = *option == current && !mixed.whole;
                    if widgets::menu::choice(ui, option, picked).clicked()
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
            let part = numbers(ui, &mut v, field, &mixed.parts);
            part.map(|p| (Value::floats(&v.map(|c| c as f32)), Some(p)))
        }
        FieldKind::Vec2 => {
            let mut v = value.as_floats::<2>().unwrap_or([0.0; 2]).map(f64::from);
            let part = numbers(ui, &mut v, field, &mixed.parts[..2]);
            part.map(|p| (Value::floats(&v.map(|c| c as f32)), Some(p)))
        }
        FieldKind::Rotation => {
            let mut degrees = degrees_of(value);
            let part = numbers(ui, &mut degrees, field, &mixed.parts);
            part.map(|p| (rotation_of(degrees), Some(p)))
        }
        FieldKind::Color => {
            // Game code's colours may be see-through: the picker sets the
            // colour and keeps that.
            let [r, g, b, a] = value.as_colour().unwrap_or([1.0; 4]);
            let mut bytes = [r, g, b].map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8);
            whole(ui.color_edit_button_srgb(&mut bytes).changed().then(|| {
                let [r, g, b] = bytes.map(|b| f32::from(b) / 255.0);
                Value::colour([r, g, b, a])
            }))
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
        FieldKind::Entity => whole(entity_picker(ui, (&field.label, value, mixed.whole), area)),
    }
}

/// Which row of a picker's list a field's value is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Row {
    /// The first row: what an empty value means ("None", "Its start").
    Empty,
    /// One of the names listed, by its place.
    Listed(usize),
    /// Neither: values that differ between the entities selected, or a
    /// name no longer listed.
    Other,
}

/// A field offering a list of names to pick from (a sheet, an animator, a
/// tile map, a screen, a state, a slice, a clip): one drop-down for them
/// all, the kit's.
struct Picker<'a> {
    /// The field's name, for screen readers.
    label: &'a str,
    /// The first row: what an empty value means.
    empty: &'a str,
    /// The names to pick from.
    names: Vec<&'a str>,
    /// The row the value is now.
    now: Row,
    /// What the box says when the value is none of the rows.
    other: String,
    /// Quiet words ending the list: how to make the first one.
    note: Option<&'a str>,
    /// Shown under the field: the value names something that isn't there.
    warning: Option<String>,
}

impl Picker<'_> {
    /// Draws it; the row picked, if another.
    fn show(&self, ui: &mut Ui) -> Option<Row> {
        let mut choices = Vec::with_capacity(self.names.len() + 1);
        choices.push((Row::Empty, self.empty));
        choices.extend(
            self.names
                .iter()
                .enumerate()
                .map(|(i, n)| (Row::Listed(i), *n)),
        );
        let shown = match self.now {
            Row::Empty => self.empty,
            Row::Listed(i) => self.names[i],
            Row::Other => self.other.as_str(),
        };
        let list = widgets::field::List {
            shown,
            note: self.note,
            width: ui.available_width(),
        };
        let mut chosen = self.now;
        let changed = widgets::field::dropdown_showing(ui, self.label, &mut chosen, &choices, list);
        if let Some(warning) = &self.warning {
            ui.label(RichText::new(warning).color(crate::theme::WARNING));
        }
        changed.then_some(chosen)
    }
}

/// An asset field offering the files `listed` (sprite sheets, animators,
/// tile maps), or None; `note` says how to make one when there are none
/// (`what` names them).
fn asset_picker(
    ui: &mut Ui,
    (label, value, mixed): (&str, &Value, bool),
    listed: &[(String, AssetId)],
    (what, note): (&str, &str),
) -> Option<Value> {
    let current = match value {
        Value::Asset(Some(r)) => Some(r.asset),
        _ => None,
    };
    let (now, other) = match current {
        _ if mixed => (Row::Other, "—".to_owned()),
        None => (Row::Empty, String::new()),
        Some(id) => match listed.iter().position(|(_, i)| *i == id) {
            Some(at) => (Row::Listed(at), String::new()),
            None => (Row::Other, format!("Missing {what} {id}")),
        },
    };
    let picked = Picker {
        label,
        empty: "None",
        names: listed.iter().map(|(n, _)| n.as_str()).collect(),
        now,
        other,
        note: listed.is_empty().then_some(note),
        warning: None,
    }
    .show(ui)?;
    match picked {
        Row::Empty => Some(Value::Asset(None)),
        Row::Listed(at) => Some(Value::Asset(Some(reflect::AssetRef {
            asset: listed[at].1,
            part: None,
        }))),
        Row::Other => None,
    }
}

/// A text field offering `names` (a screen's, a state's, a slice's, a
/// clip's), with `empty` for an empty text. When the names are known
/// (`not_one`, the words after a name that isn't among them), a name not
/// listed is warned about under the field.
fn text_picker(
    ui: &mut Ui,
    (label, value, mixed): (&str, &Value, bool),
    (empty, names): (&str, Vec<&str>),
    note: Option<&str>,
    not_one: Option<&str>,
) -> Option<Value> {
    let current = match value {
        Value::Text(t) => t.as_str(),
        _ => "",
    };
    let at = names.iter().position(|n| *n == current);
    let (now, other) = if mixed {
        (Row::Other, "—".to_owned())
    } else if current.is_empty() {
        (Row::Empty, String::new())
    } else {
        at.map_or((Row::Other, current.to_owned()), |at| {
            (Row::Listed(at), String::new())
        })
    };
    let warning = not_one
        .filter(|_| !current.is_empty() && at.is_none())
        .map(|words| format!("\"{current}\" {words}"));
    let picker = Picker {
        label,
        empty,
        names,
        now,
        other,
        note,
        warning,
    };
    match picker.show(ui)? {
        Row::Empty => Some(Value::Text(String::new())),
        Row::Listed(at) => Some(Value::Text(picker.names[at].to_owned())),
        Row::Other => None,
    }
}

/// A Screen's choice of the game's screens (decision 0087).
fn screen_picker(ui: &mut Ui, field: (&str, &Value, bool), screens: &[String]) -> Option<Value> {
    let names = screens.iter().map(String::as_str).collect();
    let note = "No screens yet: make a Lust file in the ui folder (ui/hud.lst), holding an \
                assembly with a view.";
    text_picker(
        ui,
        field,
        ("None", names),
        screens.is_empty().then_some(note),
        Some("isn't one of the game's screens"),
    )
}

/// A state name field: the animator's states (and its start, for none).
fn state_picker(
    ui: &mut Ui,
    field: (&str, &Value, bool),
    file: Option<&animation::AnimatorFile>,
) -> Option<Value> {
    let empty = match file.and_then(animation::AnimatorFile::entry_state) {
        Some(s) => format!("Its start ({})", s.name),
        None => "Its start".to_owned(),
    };
    let names = file
        .map_or(&[][..], |f| &f.states)
        .iter()
        .map(|s| s.name.as_str());
    text_picker(
        ui,
        field,
        (&empty, names.collect()),
        None,
        file.map(|_| "isn't one of its states"),
    )
}

/// A slice name field: the sheet's slices (and "bottom middle", for
/// none) to pick from.
fn slice_picker(
    ui: &mut Ui,
    field: (&str, &Value, bool),
    sheet: Option<&assets::sprite_sheet::SpriteSheet>,
) -> Option<Value> {
    let names = sheet
        .map_or(&[][..], |s| &s.slices)
        .iter()
        .map(|s| s.name.as_str());
    text_picker(
        ui,
        field,
        ("Bottom middle", names.collect()),
        None,
        sheet.map(|_| "isn't a slice of this sheet"),
    )
}

/// A clip name field (Phase 9, D3): the sheet's or clips file's clips to
/// pick from, and its first clip (or the still Frame) for none.
fn clip_picker(
    ui: &mut Ui,
    field: (&str, &Value, bool),
    clips: Option<&animation::ClipSet>,
) -> Option<Value> {
    let first = clips
        .and_then(|c| c.first.and_then(|i| c.clips.get(i)))
        .map(|c| c.name.as_str());
    let empty = match first {
        Some(name) => format!("First ({name})"),
        None => "None (the Frame)".to_owned(),
    };
    let names = clips
        .map_or(&[][..], |c| &c.clips)
        .iter()
        .map(|c| c.name.as_str());
    text_picker(
        ui,
        field,
        (&empty, names.collect()),
        clips
            .is_some_and(|c| c.clips.is_empty())
            .then_some("No clips: tag runs of frames in Aseprite."),
        clips.map(|_| "isn't one of its clips"),
    )
}

/// The most entities an entity field's list shows at once: type to find
/// the rest.
const MAX_LISTED: usize = 200;

/// An entity field: the entity's name, and a list to pick another (or
/// None) from, with a box to find one by name in a big area. Built from
/// the kit's drop-down pieces, since its list has the box at the top.
fn entity_picker(
    ui: &mut Ui,
    (label, value, mixed): (&str, &Value, bool),
    area: &area::Area,
) -> Option<Value> {
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
    let width = ui.available_width();
    let button = widgets::field::dropdown_button(ui, label, &shown, width);
    egui::Popup::menu(&button).width(width).show(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut find)
                .hint_text("Find by name")
                .desired_width(f32::INFINITY),
        );
        egui::ScrollArea::vertical()
            .max_height(widgets::field::LIST_MAX_H)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                if widgets::field::list_row(ui, "None", current.is_none() && !mixed).clicked() {
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
                    if widgets::field::list_row(ui, &entity.name, picked)
                        .on_hover_text(id.to_string())
                        .clicked()
                    {
                        chosen = Some(Value::Entity(Some(id)));
                    }
                }
                if more {
                    widgets::field::list_note(
                        ui,
                        &format!("Only the first {MAX_LISTED}: type to find others."),
                    );
                }
            });
    });
    if let Some(id) = current {
        button.on_hover_text(id.to_string());
    }
    ui.data_mut(|d| d.insert_temp(find_id, find));
    chosen.filter(|v| mixed || *v != *value)
}

/// Two or three numbers side by side (a vec2, a place, a rotation), each
/// with its axis letter, sharing the room. Returns which changed, and to
/// what.
fn numbers(
    ui: &mut Ui,
    values: &mut [f64],
    field: &FieldInfo,
    mixed: &[bool],
) -> Option<(usize, f64)> {
    let gap = ui.spacing().item_spacing.x;
    let n = values.len() as f32;
    let width = ((ui.available_width() - (n - 1.0) * gap) / n).max(size::FIELD_MIN_W / 3.0);
    let mut part = None;
    for (i, (c, axis)) in values.iter_mut().zip(widgets::field::Axis::ALL).enumerate() {
        if number(ui, c, field, (mixed[i], Some(axis), width)) {
            part = Some((i, *c));
        }
    }
    part
}

/// A number field `width` wide, with `axis`'s letter if it is one of
/// three: drags within the hint's range, but never changes a value
/// that's outside it just by being shown (decision 0019). A value that
/// differs between the selected entities (`mixed`) shows as "—".
fn number(
    ui: &mut Ui,
    value: &mut f64,
    field: &FieldInfo,
    (mixed, axis, width): (bool, Option<widgets::field::Axis>, f32),
) -> bool {
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
    widgets::field::framed(ui, width, axis, |ui| ui.add(drag)).changed()
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable as _;

    /// A text field offering "feet" and "head" (a sheet's slices), holding
    /// `value` (for several entities, if `mixed`); what's picked is kept.
    fn slices(value: &str, mixed: bool) -> Harness<'static, (Value, Vec<Value>)> {
        Harness::builder()
            .with_size(egui::vec2(300.0, 300.0))
            .build_ui_state(
                move |ui, (value, picked): &mut (Value, Vec<Value>)| {
                    let names = vec!["feet", "head"];
                    let field = ("Pivot", &*value, mixed);
                    let not_one = Some("isn't a slice of this sheet");
                    if let Some(v) = text_picker(ui, field, ("Bottom middle", names), None, not_one)
                    {
                        picked.push(v);
                    }
                },
                (Value::Text(value.into()), Vec::new()),
            )
    }

    fn shown(h: &Harness<'_, (Value, Vec<Value>)>) -> Option<String> {
        h.get_by_role_and_label(egui::accesskit::Role::ComboBox, "Pivot")
            .value()
    }

    #[test]
    fn a_picker_is_the_kit_s_drop_down_saying_what_it_holds() {
        let mut h = slices("feet", false);
        h.run();
        assert_eq!(shown(&h).as_deref(), Some("feet"));
        let mut h = slices("", false);
        h.run();
        assert_eq!(
            shown(&h).as_deref(),
            Some("Bottom middle"),
            "empty is its first row"
        );
        let mut h = slices("feet", true);
        h.run();
        assert_eq!(shown(&h).as_deref(), Some("—"), "values that differ");
    }

    #[test]
    fn a_name_that_isn_t_listed_is_shown_and_warned_about() {
        let mut h = slices("tail", false);
        h.run();
        assert_eq!(shown(&h).as_deref(), Some("tail"));
        h.get_by_label("\"tail\" isn't a slice of this sheet");
    }

    #[test]
    fn a_row_picked_is_the_new_value_and_the_same_one_is_no_change() {
        let mut h = slices("feet", false);
        h.run();
        h.get_by_role_and_label(egui::accesskit::Role::ComboBox, "Pivot")
            .click();
        h.run();
        h.get_by_label("head").click();
        h.run();
        assert_eq!(h.state().1, [Value::Text("head".into())]);
        h.get_by_role_and_label(egui::accesskit::Role::ComboBox, "Pivot")
            .click();
        h.run();
        h.get_by_label("Bottom middle").click();
        h.run();
        assert_eq!(h.state().1.last(), Some(&Value::Text(String::new())));
    }

    #[test]
    fn with_values_that_differ_any_row_picked_sets_them_all() {
        let mut h = slices("feet", true);
        h.run();
        h.get_by_role_and_label(egui::accesskit::Role::ComboBox, "Pivot")
            .click();
        h.run();
        h.get_by_label("feet").click();
        h.run();
        assert_eq!(h.state().1, [Value::Text("feet".into())]);
    }

    #[test]
    fn an_asset_no_longer_listed_says_it_is_missing() {
        let gone = AssetId::random();
        let mut h = Harness::builder()
            .with_size(egui::vec2(300.0, 300.0))
            .build_ui(move |ui| {
                let value = Value::Asset(Some(reflect::AssetRef {
                    asset: gone,
                    part: None,
                }));
                let field = ("Sheet", &value, false);
                asset_picker(ui, field, &[], ("sprite sheet", "No sprite sheets yet."));
            });
        h.run();
        let shown = h
            .get_by_role_and_label(egui::accesskit::Role::ComboBox, "Sheet")
            .value();
        assert_eq!(shown, Some(format!("Missing sprite sheet {gone}")));
        h.get_by_role_and_label(egui::accesskit::Role::ComboBox, "Sheet")
            .click();
        h.run();
        h.get_by_label("No sprite sheets yet.");
    }

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
