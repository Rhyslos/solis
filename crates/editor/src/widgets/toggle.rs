//! The design's switch: a 32 × 18 track with a knob that sits at the
//! right, on the accent, when on, and at the left, on grey, when off.

use super::focus_ring;
use crate::interface_look;
use crate::tokens::{self, size};
use egui::{CornerRadius, Response, Sense, Ui, WidgetInfo, pos2, vec2};

/// A switch for `on`. `name` is what it switches: a screen reader says
/// it, and tests find the switch by it. A click, Space or Enter flips it.
pub fn toggle(ui: &mut Ui, on: &mut bool, name: &str) -> Response {
    let [width, height] = size::TOGGLE;
    let (rect, mut response) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    response.widget_info(|| {
        WidgetInfo::selected(egui::WidgetType::Checkbox, ui.is_enabled(), *on, name)
    });
    if ui.is_rect_visible(rect) {
        let (track, knob) = if *on {
            (interface_look::accent_fill(), tokens::app())
        } else {
            (tokens::strong_line(), tokens::text_meta())
        };
        let round = (height / 2.0) as u8;
        ui.painter()
            .rect_filled(rect, CornerRadius::same(round), track);
        let inset = (height - size::TOGGLE_KNOB) / 2.0;
        let reach = size::TOGGLE_KNOB / 2.0;
        let x = if *on {
            rect.right() - inset - reach
        } else {
            rect.left() + inset + reach
        };
        ui.painter()
            .circle_filled(pos2(x, rect.center().y), reach, knob);
        focus_ring(ui, &response, rect, round);
    }
    response
}
