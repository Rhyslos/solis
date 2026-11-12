//! A component's fields as Properties' rows: the name, the field, and the
//! marks after it. Split from `inspector.rs` (which draws each kind of
//! field) so neither file is long. A row's name can be dragged sideways
//! to change a number (the project lead's answer 25: dragging the value
//! still works too), and a field the user changed while the game plays
//! keeps the accent border, with a way back to its value from before Play
//! (`04-properties-playing.png`).

use crate::inspector::{FieldChange, Mixed, field_widget, moved_from_default, shared_by, short};
use crate::models::ModelInfo;
use crate::sprite_sheets::SpriteSheets;
use crate::tokens::{self, size, text};
use egui::{Response, RichText, Ui};
use reflect::{AssetId, ComponentData, ComponentSchema, FieldInfo, FieldKind, Value};
use std::collections::BTreeMap;

/// A row whose name is bold when `changed` (a placed assembly's copy that
/// differs from the assembly), with a dot before it when `moved` (its
/// value isn't the standard one, decision 0085). Returns what `add`
/// returned and the name's response (a drag on it changes a number).
pub(crate) fn row_marked<R>(
    ui: &mut Ui,
    (label, help): (&str, &str),
    (changed, moved, scrubs): (bool, bool, bool),
    add: impl FnOnce(&mut Ui) -> R,
) -> (R, Response) {
    ui.horizontal(|ui| {
        let text = if changed {
            RichText::new(label)
                .font(text::row())
                .strong()
                .color(tokens::text_primary())
        } else {
            RichText::new(label)
                .font(text::row())
                .color(tokens::text_label())
        };
        let spacing = std::mem::replace(&mut ui.spacing_mut().item_spacing.x, 0.0);
        // The changed-from-its-start dot goes before the name, as the
        // design's settings mark it.
        let (dot, _) = ui.allocate_exact_size(
            egui::vec2(size::FIELD_DOT_W, size::field_h()),
            egui::Sense::hover(),
        );
        let sense = if scrubs {
            egui::Sense::drag()
        } else {
            egui::Sense::hover()
        };
        let name = ui
            .allocate_ui_with_layout(
                egui::vec2(size::FIELD_LABEL_W - size::FIELD_DOT_W, size::field_h()),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.add(egui::Label::new(text).truncate().sense(sense))
                },
            )
            .inner;
        ui.spacing_mut().item_spacing.x = spacing;
        if moved {
            ui.painter().circle_filled(
                dot.center(),
                size::FIELD_DOT / 2.0,
                crate::interface_look::accent_fill(),
            );
        }
        let name = if help.is_empty() {
            name
        } else {
            name.on_hover_text(help)
        };
        if scrubs && (name.hovered() || name.dragged()) {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }
        (add(ui), name)
    })
    .inner
}

/// Whether a field's name can be dragged to change it: a single number.
fn scrubs(field: &FieldInfo) -> bool {
    matches!(field.kind, FieldKind::Int | FieldKind::Float)
}

/// The value a drag on a number's name (`name`) gives, if it changed: as
/// a number field's drag does, at the field's speed and within its range,
/// counted from where the drag began.
fn scrubbed(name: &Response, field: &FieldInfo, value: &Value) -> Option<Value> {
    let id = name.id.with("scrub");
    let ctx = &name.ctx;
    let now = value.as_f64().unwrap_or(0.0);
    if name.drag_started() {
        ctx.data_mut(|d| d.insert_temp(id, (now, 0.0_f64)));
    }
    if !name.dragged() {
        return None;
    }
    let moved = f64::from(name.drag_delta().x);
    let (start, so_far) = ctx.data(|d| d.get_temp(id)).unwrap_or((now, 0.0));
    let so_far = so_far + moved;
    ctx.data_mut(|d| d.insert_temp(id, (start, so_far)));
    let whole = field.kind == FieldKind::Int;
    let speed = field.hint.speed.unwrap_or(if whole { 1.0 } else { 0.1 });
    let mut wanted = start + so_far * speed;
    if let Some((low, high)) = field.hint.range {
        wanted = wanted.clamp(low, high);
    }
    let new = if whole {
        Value::Int(wanted.round() as i64)
    } else {
        Value::Float((wanted * 1000.0).round() / 1000.0)
    };
    (&new != value).then_some(new)
}

/// What a row's marks say: whether it differs from its assembly, whether
/// it isn't its standard value, and its value from before Play if the
/// user changed it since.
#[derive(Clone, Copy)]
struct Marks<'a> {
    changed: bool,
    moved: bool,
    played: Option<&'a Value>,
}

/// What a click after a field asked for.
#[derive(Default)]
struct After {
    /// Put this value back (the standard one, or the one from before
    /// Play).
    reset: Option<Value>,
    revert: bool,
    apply: bool,
}

/// Draws every field of `schema` with its value in `data`, and returns what
/// the user did. Fields `data` doesn't have show their starting value. With
/// `assembly` (the component as the entity's assembly has it), fields that
/// differ are bold, with Revert and Apply buttons. `yours` are the fields
/// the user changed while playing, with their values from before Play.
/// `others` are the same component on the other selected entities: values
/// that differ show as "—". `users` says how many entities use each file
/// (`Document::asset_users`).
pub(crate) fn fields(
    ui: &mut Ui,
    schema: &ComponentSchema,
    data: &ComponentData,
    (assembly, yours): (Option<&ComponentData>, &BTreeMap<&str, Option<&Value>>),
    others: &[&ComponentData],
    (models, sheets): (&BTreeMap<AssetId, ModelInfo>, &SpriteSheets),
    (area, users): (&area::Area, &crate::document::AssetUsers),
) -> Vec<FieldChange> {
    let mut changes = Vec::new();
    for field in &schema.fields {
        let value = data.get(&field.name).unwrap_or(&field.default);
        let mixed = Mixed::of(field, value, others);
        let changed =
            assembly.is_some_and(|a| a.get(&field.name).unwrap_or(&field.default) != value);
        let played = yours
            .get(field.name.as_str())
            .map(|was| was.unwrap_or(&field.default))
            .filter(|was| *was != value);
        // Not its standard value: a dot, and a way back (decision 0085).
        // An assembly's copy shows how it differs from the assembly
        // instead, and a change while playing its value from before.
        let moved = !changed && !mixed.whole && moved_from_default(field, value);
        let marks = Marks {
            changed,
            moved: moved && played.is_none(),
            played,
        };
        let scrub = scrubs(field) && !mixed.whole;
        let id = ui.id().with((&schema.name, &field.name));
        ui.push_id(id, |ui| {
            let ((set, after), name) = row_marked(
                ui,
                (&field.label, field.hint.help),
                (changed, moved, scrub),
                |ui| {
                    let set = field_and_marks(ui, marks, |ui| {
                        field_widget(ui, field, value, (models, sheets, data), area, mixed)
                    });
                    if let Value::Asset(Some(used)) = value {
                        shared_by(ui, users.get(&used.asset).copied().unwrap_or(0));
                    }
                    (set, after_field(ui, field, marks))
                },
            );
            if let Some(value) = after.reset {
                changes.push(FieldChange::Set {
                    field: field.name.clone(),
                    value,
                    part: None,
                });
            }
            let scrubbed = scrub
                .then(|| scrubbed(&name, field, value))
                .flatten()
                .map(|v| (v, None));
            if let Some((value, part)) = set.or(scrubbed) {
                changes.push(FieldChange::Set {
                    field: field.name.clone(),
                    value,
                    part,
                });
            }
            if after.revert {
                changes.push(FieldChange::Revert(field.name.clone()));
            }
            if after.apply {
                changes.push(FieldChange::Apply(field.name.clone()));
            }
        });
    }
    changes
}

/// The field itself, filling the card less room for what follows it, with
/// the accent border kept round it while it differs from before Play.
fn field_and_marks<R>(ui: &mut Ui, marks: Marks<'_>, draw: impl FnOnce(&mut Ui) -> R) -> R {
    let after = if marks.moved || marks.changed || marks.played.is_some() {
        size::FIELD_AFTER_W
    } else {
        0.0
    };
    let room = (ui.available_width() - after).max(size::FIELD_MIN_W);
    let drawn = ui.allocate_ui_with_layout(
        egui::vec2(room, size::field_h()),
        egui::Layout::left_to_right(egui::Align::Center),
        draw,
    );
    if marks.played.is_some() {
        let rect = drawn.response.rect.intersect(ui.clip_rect());
        ui.painter().rect_stroke(
            rect,
            egui::CornerRadius::same(size::radius::CONTROL),
            egui::Stroke::new(size::LINE, crate::interface_look::accent_fill()),
            egui::StrokeKind::Inside,
        );
    }
    drawn.inner
}

/// The small buttons after a field: back to the value from before Play,
/// back to the standard value, or an assembly copy's Revert and Apply.
fn after_field(ui: &mut Ui, field: &FieldInfo, marks: Marks<'_>) -> After {
    let small = |ui: &mut Ui, words: String, hover: &str, name: String| {
        let button = ui
            .add(
                egui::Button::new(
                    RichText::new(words)
                        .font(text::mono_small())
                        .color(tokens::text_meta()),
                )
                .frame(false),
            )
            .on_hover_text(hover);
        button.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &name));
        button.clicked()
    };
    let mut after = After::default();
    if let Some(was) = marks.played {
        if small(
            ui,
            format!("↺ {}", short(was)),
            "Back to its value from before Play",
            format!("Put back {}", field.label),
        ) {
            after.reset = Some(was.clone());
        }
        return after;
    }
    if marks.moved {
        if small(
            ui,
            format!("↺ {}", short(&field.default)),
            "Back to its standard value",
            format!("Reset {}", field.label),
        ) {
            after.reset = Some(field.default.clone());
        }
        return after;
    }
    if marks.changed {
        after.revert = small(
            ui,
            "↺".into(),
            "Back to the assembly's value",
            format!("Revert {}", field.label),
        );
        after.apply = small(
            ui,
            "↑".into(),
            "Make this the assembly's value, for every copy",
            format!("Apply {}", field.label),
        );
    }
    after
}
