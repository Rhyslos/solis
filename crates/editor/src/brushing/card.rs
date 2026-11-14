//! The Brush card in Properties (Tools 1a, B3; the approved mock-up
//! `03-brush-edit-faces.png`): Made as; Size, the brush's box in metres
//! (typing one stretches the brush about its middle); Material (with the
//! unfinished part's mark until brush materials arrive); Solid with its
//! note; three boxes counting its points, edges and faces, the box of
//! the mode in use lit while its insides are edited; and what's picked.
//!
//! It replaces the rows that showed how long the brush's raw lists of
//! numbers are.

use super::edit::Mode;
use crate::inspector::{self, FieldChange};
use crate::tokens::{self, size, text};
use crate::unbuilt::{self, Entry, UnbuiltId};
use egui::{Align2, CornerRadius, Rect, RichText, Sense, Ui, vec2};
use glam::Vec3;
use reflect::{ComponentData, ComponentSchema, FieldInfo, FieldKind, Hint, Value};

/// What the card shows that the brush's values don't say outright,
/// worked out by the workbench from the brush's mesh (already read).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BrushFacts {
    /// How many points, edges and faces it has.
    pub(crate) counts: [usize; 3],
    /// The size of its box, in metres in its own space.
    pub(crate) size: Vec3,
    /// While its insides are edited: the mode, and what's picked in
    /// words ("1 face").
    pub(crate) editing: Option<(Mode, String)>,
}

/// What a change on the card asks for.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CardChange {
    /// A field of the brush's own (Made as, Solid).
    Field(FieldChange),
    /// A new size, in metres: the brush stretched about its middle.
    Size(Vec3),
}

/// The words after Solid's switch.
const SOLID_NOTE: &str = "players and physics collide with it";

/// Draws the Brush card's rows for `data` (with `schema`, the Brush's),
/// and says what was changed.
pub(crate) fn rows(
    ui: &mut Ui,
    (schema, data): (&ComponentSchema, &ComponentData),
    facts: &BrushFacts,
    assets: (
        &std::collections::BTreeMap<reflect::AssetId, crate::models::ModelInfo>,
        &crate::sprite_sheets::SpriteSheets,
    ),
    area: &area::Area,
) -> Vec<CardChange> {
    let mut changes = Vec::new();
    let field = |name: &str| schema.fields.iter().find(|f| f.name == name);
    let size_field =
        FieldInfo::new("size", "Size", FieldKind::Vec3, Value::vec3(facts.size)).hint(Hint {
            speed: Some(0.05),
            unit: " m",
            help: "The brush's box, in metres: typing a size stretches the brush about its \
                   middle.",
            ..Hint::default()
        });
    let mut top = ComponentSchema {
        name: schema.name.clone(),
        help: schema.help.clone(),
        fields: field("shape").cloned().into_iter().collect(),
    };
    top.fields.push(size_field);
    let mut shown = data.clone();
    shown.set("size", Value::vec3(facts.size));
    let users = crate::document::AssetUsers::default();
    for change in inspector::fields(
        ui,
        &top,
        &shown,
        (None, &Default::default()),
        &[],
        assets,
        (area, &users),
    ) {
        changes.push(match change {
            FieldChange::Set { field, value, .. } if field == "size" => {
                match value.as_floats::<3>() {
                    Some(v) => CardChange::Size(Vec3::from_array(v)),
                    None => continue,
                }
            }
            other => CardChange::Field(other),
        });
    }
    for entry in unbuilt::filter(vec![Entry::<()>::Unbuilt(UnbuiltId::BrushCardMaterial)]) {
        if let Entry::Unbuilt(id) = entry {
            unbuilt::field_row(ui, id);
        }
    }
    if let Some(solid) = field("solid") {
        changes.extend(solid_row(ui, solid, data, assets, area));
    }
    counts(ui, facts);
    picked_line(ui, facts);
    changes
}

/// Solid: its switch and the note after it.
fn solid_row(
    ui: &mut Ui,
    solid: &FieldInfo,
    data: &ComponentData,
    (models, sheets): (
        &std::collections::BTreeMap<reflect::AssetId, crate::models::ModelInfo>,
        &crate::sprite_sheets::SpriteSheets,
    ),
    area: &area::Area,
) -> Option<CardChange> {
    let value = data.get("solid").unwrap_or(&solid.default);
    inspector::row(ui, &solid.label, solid.hint.help, |ui| {
        let set = inspector::field_widget(
            ui,
            solid,
            value,
            (models, sheets, data),
            area,
            inspector::Mixed::default(),
        );
        ui.label(
            RichText::new(SOLID_NOTE)
                .font(text::row())
                .color(tokens::text_label()),
        );
        set
    })
    .map(|(value, part)| {
        CardChange::Field(FieldChange::Set {
            field: "solid".into(),
            value,
            part,
        })
    })
}

/// Three boxes side by side: how many points, edges and faces, the box of
/// the mode in use outlined in the accent while editing.
fn counts(ui: &mut Ui, facts: &BrushFacts) {
    let gap = size::CARD_GAP;
    let width = ((ui.available_width() - 2.0 * gap) / 3.0).max(0.0);
    let height = size::BRUSH_COUNT_H;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        for (mode, count) in Mode::ALL.into_iter().zip(facts.counts) {
            let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
            let lit = facts.editing.as_ref().is_some_and(|(m, _)| *m == mode);
            count_box(ui, rect, (count, mode.name()), lit);
        }
    });
}

/// One count's box: the number over the word.
fn count_box(ui: &Ui, rect: Rect, (count, word): (usize, &str), lit: bool) {
    let radius = size::radius::CONTROL;
    ui.painter()
        .rect_filled(rect, CornerRadius::same(radius), tokens::field());
    let edge = if lit {
        crate::interface_look::accent_fill()
    } else {
        tokens::field_border()
    };
    crate::widgets::border(ui, rect, radius, edge);
    let inner = rect.shrink2(vec2(size::FIELD_PAD * 2.0, size::FIELD_PAD));
    let (number, label) = inner.split_top_bottom_at_fraction(0.55);
    crate::widgets::words(
        ui,
        number,
        Align2::LEFT_BOTTOM,
        &crate::models::grouped(count),
        text::header(),
        tokens::text_emphasis(),
    );
    crate::widgets::words(
        ui,
        label,
        Align2::LEFT_CENTER,
        word,
        text::row(),
        tokens::text_label(),
    );
}

/// "Picked: 1 face · Faces mode" while editing; otherwise how to start.
fn picked_line(ui: &mut Ui, facts: &BrushFacts) {
    let mut job = egui::text::LayoutJob::default();
    let mut add = |words: &str, colour| {
        job.append(words, 0.0, egui::TextFormat::simple(text::row(), colour));
    };
    match &facts.editing {
        Some((mode, picked)) => {
            add("Picked: ", tokens::text_label());
            add(picked, tokens::text_primary());
            add(
                &format!(" \u{b7} {} mode", mode.name()),
                tokens::text_label(),
            );
        }
        None => add(
            "Tab edits its points, edges and faces.",
            tokens::text_label(),
        ),
    }
    ui.label(job);
}

/// The size of the box around a brush's values' mesh, as the card shows
/// it: from the read mesh, in metres.
pub(crate) fn facts_of(mesh: &mesh::Mesh, editing: Option<(Mode, String)>) -> BrushFacts {
    BrushFacts {
        counts: super::edit::counts(mesh),
        size: super::reshape::size_of(mesh),
        editing,
    }
}
