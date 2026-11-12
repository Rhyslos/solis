//! The design's pill: a small see-through label floating over the
//! viewport ("Perspective ▾", "6.9 ms · 144 fps").

use super::{border, focus_ring, width_of, words};
use crate::tokens::{self, size, text};
use egui::{Align2, CornerRadius, Response, Sense, Ui, WidgetInfo, vec2};

/// A pill saying `label`. One that opens a list ends its label with ▾.
pub fn pill(ui: &mut Ui, label: &str) -> Response {
    let font = text::ribbon_small();
    let width = width_of(ui, label, font.clone()) + 2.0 * size::PILL_PAD;
    let (rect, response) = ui.allocate_exact_size(vec2(width, size::PILL_H), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label));
    if ui.is_rect_visible(rect) {
        let radius = size::radius::CONTROL;
        ui.painter()
            .rect_filled(rect, CornerRadius::same(radius), tokens::pill_fill());
        border(ui, rect, radius, tokens::pill_border());
        let ink = if response.hovered() {
            tokens::text_primary()
        } else {
            tokens::text_body()
        };
        words(ui, rect, Align2::CENTER_CENTER, label, font, ink);
        focus_ring(ui, &response, rect, radius);
    }
    response
}

/// The hint over the bottom middle of `view` while a tool is in use: each
/// key as a keycap and what it does after it ("Drag paint · Esc give
/// up"), on the label box's dark fill. Only today's keys are named.
/// Returns where it was drawn.
pub fn key_hints(ui: &mut Ui, view: egui::Rect, keys: &[(&str, &str)]) -> egui::Rect {
    let cap = |key: &str| {
        (width_of(ui, key, text::mono_small()) + 2.0 * size::FIELD_PAD).max(size::KEYCAP)
    };
    let width: f32 = keys
        .iter()
        .map(|(key, what)| cap(key) + width_of(ui, what, text::ui()) + 3.0 * size::GAP)
        .sum::<f32>()
        + 2.0 * size::PILL_PAD
        - size::GAP;
    let width = width.min(view.width() - 2.0 * size::PILL_MARGIN).max(0.0);
    let rect = egui::Rect::from_center_size(
        egui::pos2(
            view.center().x,
            view.bottom() - size::PILL_MARGIN - size::PLACE_HINT_H / 2.0,
        ),
        vec2(width, size::PLACE_HINT_H),
    );
    ui.painter().rect_filled(
        rect,
        CornerRadius::same(size::radius::CARD),
        tokens::label_box_fill(),
    );
    let mut row = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect.shrink2(vec2(size::PILL_PAD, 0.0)))
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    row.set_clip_rect(rect.intersect(ui.clip_rect()));
    row.spacing_mut().item_spacing.x = size::GAP;
    for (index, (key, what)) in keys.iter().enumerate() {
        if index > 0 {
            row.add_space(size::GAP);
        }
        super::keycap::keycap(&mut row, key, super::keycap::KeycapState::Bound);
        row.label(
            egui::RichText::new(*what)
                .font(text::ui())
                .color(tokens::text_body()),
        );
    }
    rect
}
