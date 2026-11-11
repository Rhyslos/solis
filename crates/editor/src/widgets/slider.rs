//! The design's slider: a thin track, filled with the accent as far as
//! its round light knob.

use super::{arrow_step, focus_ring};
use crate::interface_look;
use crate::tokens::{self, size};
use egui::{CornerRadius, Rect, Response, Sense, Ui, WidgetInfo, pos2, vec2};
use std::ops::RangeInclusive;

/// How many arrow presses take the knob from one end to the other.
const KEY_STEPS: f32 = 50.0;

/// A slider for `value` within `range`, as wide as the room it is given.
/// Dragging or clicking moves the knob; so do the arrow keys while it
/// holds the keyboard. `name` is what it sets.
pub fn slider(ui: &mut Ui, value: &mut f32, range: RangeInclusive<f32>, name: &str) -> Response {
    let (low, high) = (*range.start(), *range.end());
    let span = (high - low).max(f32::EPSILON);
    let wanted = vec2(
        ui.available_width().max(size::SLIDER_KNOB * 2.0),
        size::SLIDER_KNOB,
    );
    let (rect, mut response) = ui.allocate_exact_size(wanted, Sense::click_and_drag());
    let reach = size::SLIDER_KNOB / 2.0;
    // The knob's middle runs from half a knob inside each end.
    let (from, to) = (rect.left() + reach, rect.right() - reach);
    let before = *value;
    if let Some(pointer) = response.interact_pointer_pos() {
        let share = ((pointer.x - from) / (to - from).max(1.0)).clamp(0.0, 1.0);
        *value = low + share * span;
    }
    let step = arrow_step(ui, &response);
    if step != 0 {
        *value += step as f32 * span / KEY_STEPS;
    }
    if !value.is_finite() {
        *value = low;
    }
    *value = value.clamp(low, high);
    if *value != before {
        response.mark_changed();
    }
    response.widget_info(|| WidgetInfo::slider(ui.is_enabled(), f64::from(*value), name));
    if ui.is_rect_visible(rect) {
        let share = (*value - low) / span;
        let knob = pos2(from + share * (to - from), rect.center().y);
        let track = Rect::from_center_size(rect.center(), vec2(rect.width(), size::SLIDER_TRACK));
        let round = CornerRadius::same(size::radius::GRIP);
        let painter = ui.painter();
        painter.rect_filled(track, round, tokens::field_border());
        let filled = Rect::from_min_max(track.min, pos2(knob.x, track.bottom()));
        painter.rect_filled(filled, round, interface_look::accent_fill());
        painter.circle_filled(knob, reach, tokens::text_primary());
        let ring = Rect::from_center_size(knob, vec2(size::SLIDER_KNOB, size::SLIDER_KNOB));
        focus_ring(ui, &response, ring, reach as u8);
    }
    response
}
