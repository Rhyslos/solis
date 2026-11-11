//! The design's keycap: a key's name on a small raised cap, as the input
//! page shows a binding.

use super::{dashed, focus_ring, hairline, width_of, words};
use crate::interface_look;
use crate::tokens::{self, size, text};
use egui::{Align2, CornerRadius, Rect, Response, Sense, Ui, WidgetInfo, pos2, vec2};

/// What a keycap is saying.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeycapState {
    /// A key that is bound.
    Bound,
    /// A key bound to two things: amber.
    Conflict,
    /// Waiting for a key to be pressed: the accent, dashed.
    Listening,
}

/// A keycap reading `key`.
pub fn keycap(ui: &mut Ui, key: &str, state: KeycapState) -> Response {
    let font = text::mono_small();
    let width = (width_of(ui, key, font.clone()) + 2.0 * size::FIELD_PAD).max(size::KEYCAP);
    let (rect, response) = ui.allocate_exact_size(vec2(width, size::KEYCAP), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), key));
    if !ui.is_rect_visible(rect) {
        return response;
    }
    let radius = size::radius::CONTROL;
    let round = CornerRadius::same(radius);
    let accent = interface_look::in_use().accent;
    let (fill, edge, ink) = match state {
        KeycapState::Bound => (
            tokens::line(),
            tokens::stronger_line(),
            tokens::text_primary(),
        ),
        KeycapState::Conflict => (
            tokens::CONFLICT_FILL,
            tokens::CONFLICT_BORDER,
            tokens::CONFLICT_TEXT,
        ),
        KeycapState::Listening => (
            tokens::listening(accent),
            accent.colour(),
            tokens::text_emphasis(),
        ),
    };
    let painter = ui.painter();
    if state == KeycapState::Listening {
        painter.rect_filled(rect, round, fill);
        dashed::rounded(ui, rect, radius, edge);
    } else {
        // The cap stands on a border twice as thick along its foot: the
        // edge colour under the cap's own fill.
        let line = hairline(ui);
        painter.rect_filled(rect, round, edge);
        let face = Rect::from_min_max(
            rect.min + vec2(line, line),
            pos2(
                rect.right() - line,
                rect.bottom() - size::KEYCAP_FOOT * line,
            ),
        );
        painter.rect_filled(face, CornerRadius::same(radius - 1), fill);
    }
    words(ui, rect, Align2::CENTER_CENTER, key, font, ink);
    focus_ring(ui, &response, rect, radius);
    response
}
