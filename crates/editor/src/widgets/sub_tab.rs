//! A File page's sub-tab (Preferences' General, Appearance… and Project
//! settings' General, 2D…): its word, brighter with the accent's line
//! under it while it is the one in front.

use super::{focus_ring, width_of, words};
use crate::interface_look;
use crate::tokens::{self, size, text};
use egui::{Align2, Rect, Response, Sense, Ui, WidgetInfo, vec2};

/// One sub-tab reading `label`; `front` while its page shows.
pub fn sub_tab(ui: &mut Ui, label: &str, front: bool) -> Response {
    let font = text::page_subtitle();
    let width = width_of(ui, label, font.clone()) + 2.0 * size::SUB_TAB_PAD;
    let (rect, response) = ui.allocate_exact_size(vec2(width, size::SUB_TAB_H), Sense::click());
    response.widget_info(|| WidgetInfo::selected(egui::WidgetType::Button, true, front, label));
    if ui.is_rect_visible(rect) {
        let ink = if front {
            tokens::text_emphasis()
        } else if response.hovered() {
            tokens::text_primary()
        } else {
            tokens::text_label()
        };
        let room = rect.with_max_y(rect.bottom() - size::TAB_UNDERLINE);
        words(ui, room, Align2::CENTER_CENTER, label, font, ink);
        if front {
            let line = Rect::from_min_max(
                egui::pos2(rect.left(), rect.bottom() - size::TAB_UNDERLINE),
                rect.right_bottom(),
            );
            ui.painter()
                .rect_filled(line, 0.0, interface_look::accent_fill());
        }
        focus_ring(ui, &response, rect, size::radius::SMALL);
    }
    response
}
