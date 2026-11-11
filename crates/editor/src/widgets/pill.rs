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
