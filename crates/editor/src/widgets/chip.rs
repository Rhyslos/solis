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
