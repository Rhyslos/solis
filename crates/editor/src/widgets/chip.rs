//! The design's chips: small rounded choices in a row (the Library's
//! kinds, the Place tab's filters, the input page's contexts).

use super::{border, focus_ring, width_of, words};
use crate::tokens::{self, size, text};
use egui::{Align2, Color32, CornerRadius, Rect, Response, Sense, Ui, WidgetInfo, pos2, vec2};

/// Where a chip is used: the design draws each a little differently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChipKind {
    /// The Library's kinds: 24 tall, with a border.
    Filter,
    /// The Place tab's filters: 22 tall, no border, medium weight.
    Quiet,
    /// The input page's contexts: 26 tall, with a border.
    Large,
}

impl ChipKind {
    /// Its height, its padding at each side, and its text's size.
    fn sizes(self) -> (f32, f32, f32) {
        match self {
            Self::Filter => (size::CHIP_H, 9.0, text::SMALL_SIZE),
            Self::Quiet => (size::CHIP_H - 2.0, 10.0, text::SMALL_SIZE),
            Self::Large => (size::CHIP_H + 2.0, 11.0, text::ROW_SIZE),
        }
    }
}

/// A chip saying `label`, lifted when `chosen`, with a kind's coloured
/// dot before the word if `dot` is given.
pub fn chip(
    ui: &mut Ui,
    kind: ChipKind,
    label: &str,
    chosen: bool,
    dot: Option<Color32>,
) -> Response {
    let (height, pad, text_size) = kind.sizes();
    let font = match kind {
        ChipKind::Filter => egui::FontId::proportional(text_size),
        ChipKind::Quiet => text::medium_at(text_size),
        ChipKind::Large if chosen => text::medium_at(text_size),
        ChipKind::Large => egui::FontId::proportional(text_size),
    };
    let dot_w = dot.map_or(0.0, |_| size::KIND_DOT + size::GAP);
    let width = 2.0 * pad + dot_w + width_of(ui, label, font.clone());
    let (rect, response) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    response.widget_info(|| {
        WidgetInfo::selected(egui::WidgetType::Button, ui.is_enabled(), chosen, label)
    });
    if !ui.is_rect_visible(rect) {
        return response;
    }
    let round = (height / 2.0) as u8;
    let bordered = kind != ChipKind::Quiet;
    let ink = if chosen {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(round), tokens::line());
        if bordered {
            border(ui, rect, round, tokens::stronger_line());
        }
        if kind == ChipKind::Filter {
            tokens::text_primary()
        } else {
            tokens::text_emphasis()
        }
    } else {
        if bordered {
            border(ui, rect, round, tokens::field_border());
        }
        if response.hovered() {
            tokens::text_primary()
        } else {
            tokens::text_label()
        }
    };
    let mut left = rect.left() + pad;
    if let Some(colour) = dot {
        let spot = Rect::from_min_size(
            pos2(left, rect.center().y - size::KIND_DOT / 2.0),
            vec2(size::KIND_DOT, size::KIND_DOT),
        );
        ui.painter()
            .rect_filled(spot, CornerRadius::same(size::radius::GRIP), colour);
        left += dot_w;
    }
    let room = Rect::from_min_max(pos2(left, rect.top()), rect.right_bottom());
    words(ui, room, Align2::LEFT_CENTER, label, font, ink);
    focus_ring(ui, &response, rect, round);
    response
}

/// A tag an entity carries (stage 1, tags; `docs/design/tags/02-entity-tags.png`):
/// its name in a rounded chip filled a step up from the card, with an ×
/// after it when `removable`. Gives back the ×'s response (named "Remove
/// tag …"), or the chip's when there's none.
pub fn tag_chip(ui: &mut Ui, label: &str, removable: bool) -> Response {
    let font = egui::FontId::proportional(text::SMALL_SIZE);
    let pad = 8.0;
    let cross = if removable { 12.0 } else { 0.0 };
    let gap = if removable { 4.0 } else { 0.0 };
    let height = size::CHIP_H - 2.0;
    let width = 2.0 * pad + width_of(ui, label, font.clone()) + gap + cross;
    let (rect, chip) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    chip.widget_info(|| WidgetInfo::labeled(egui::WidgetType::Label, true, label));
    let round = size::radius::CONTROL;
    let x_rect = Rect::from_min_max(
        pos2(rect.right() - pad - cross, rect.top()),
        pos2(rect.right() - pad / 2.0, rect.bottom()),
    );
    let response = if removable {
        let named = format!("Remove tag {label}");
        let x = ui.interact(x_rect, chip.id.with("remove"), Sense::click());
        x.widget_info(|| WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), &named));
        x
    } else {
        chip
    };
    if ui.is_rect_visible(rect) {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(round), tokens::line());
        border(ui, rect, round, tokens::field_border());
        let room = Rect::from_min_max(
            pos2(rect.left() + pad, rect.top()),
            pos2(rect.right() - pad - cross - gap, rect.bottom()),
        );
        words(
            ui,
            room,
            Align2::LEFT_CENTER,
            label,
            font,
            tokens::text_primary(),
        );
        if removable {
            let ink = if response.hovered() {
                tokens::text_emphasis()
            } else {
                tokens::text_meta()
            };
            words(
                ui,
                x_rect,
                Align2::CENTER_CENTER,
                "\u{d7}",
                egui::FontId::proportional(text::ROW_SIZE),
                ink,
            );
            focus_ring(ui, &response, x_rect, round);
        }
    }
    response
}

/// Room for a chip before a list of chips wraps: what [`tag_chip`] takes.
pub fn tag_chip_width(ui: &Ui, label: &str, removable: bool) -> f32 {
    let font = egui::FontId::proportional(text::SMALL_SIZE);
    let extra = if removable { 16.0 } else { 0.0 };
    16.0 + width_of(ui, label, font) + extra
}
