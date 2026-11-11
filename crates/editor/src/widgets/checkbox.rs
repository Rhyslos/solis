//! The design's checkbox: a 12 × 12 rounded square, filled with the
//! accent and ticked when on, a grey outline when off.

use super::{border, focus_ring, width_of, words};
use crate::icons::{self, Icon};
use crate::interface_look;
use crate::tokens::{self, size, text};
use egui::{Align2, CornerRadius, Rect, Response, Sense, Ui, WidgetInfo, pos2, vec2};

/// A checkbox for `on` with `label` after it. A click on the box or the
/// label, Space or Enter flips it.
pub fn checkbox(ui: &mut Ui, on: &mut bool, label: &str) -> Response {
    let font = text::ribbon_small();
    let label_w = if label.is_empty() {
        0.0
    } else {
        size::GAP + width_of(ui, label, font.clone())
    };
    let wanted = vec2(size::CHECKBOX + label_w, size::RIBBON_ROW_H);
    let (rect, mut response) = ui.allocate_exact_size(wanted, Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    response.widget_info(|| {
        WidgetInfo::selected(egui::WidgetType::Checkbox, ui.is_enabled(), *on, label)
    });
    if ui.is_rect_visible(rect) {
        let square = Rect::from_min_size(
            pos2(rect.left(), rect.center().y - size::CHECKBOX / 2.0),
            vec2(size::CHECKBOX, size::CHECKBOX),
        );
        // On whole pixels, so its outline is sharp.
        let scale = ui.ctx().pixels_per_point();
        let corner = pos2(
            tokens::on_pixel(square.left(), scale),
            tokens::on_pixel(square.top(), scale),
        );
        let square = Rect::from_min_size(corner, square.size());
        draw_box(ui, square, *on);
        let room = Rect::from_min_max(
            pos2(square.right() + size::GAP, rect.top()),
            rect.right_bottom(),
        );
        words(
            ui,
            room,
            Align2::LEFT_CENTER,
            label,
            font,
            tokens::text_body(),
        );
        focus_ring(ui, &response, square, size::radius::CHECKBOX);
    }
    response
}

/// The box alone, on or off: also what a ticked row of a menu shows.
pub fn draw_box(ui: &Ui, square: Rect, on: bool) {
    if on {
        ui.painter().rect_filled(
            square,
            CornerRadius::same(size::radius::CHECKBOX),
            interface_look::accent_fill(),
        );
        icons::icon(
            ui.painter(),
            square.shrink(1.0),
            Icon::Check,
            tokens::ON_ACCENT,
        );
    } else {
        border(ui, square, size::radius::CHECKBOX, tokens::stronger_line());
    }
}
