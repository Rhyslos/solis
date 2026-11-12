//! The rows of the design's menus (the approved additions, round 2: the
//! viewport's pills, the Area column's and a card's right-click menus): 24
//! tall, lifted under the pointer, a check mark before the choice in force,
//! small capitals over a group, a line between groups.
//!
//! They go inside egui's own menus (`egui::Popup::menu`, `Response::
//! context_menu`), which `theme.rs` already gives the design's frame.

use super::{focus_ring, width_of, words};
use crate::icons::{self, Icon};
use crate::tokens::{self, size, text};
use egui::{Align2, CornerRadius, Rect, Response, Sense, Ui, WidgetInfo, pos2, vec2};

/// A row that does something: `label`, and quiet `after` words at its
/// right (a key, a count) if any.
pub fn item(ui: &mut Ui, label: &str, after: &str) -> Response {
    row(ui, label, after, None, egui::WidgetType::Button)
}

/// A row that is one of several choices: a check mark before it while it
/// is the one in force.
pub fn choice(ui: &mut Ui, label: &str, chosen: bool) -> Response {
    row(ui, label, "", Some(chosen), egui::WidgetType::RadioButton)
}

/// A choice with quiet `after` words at its right: its key.
pub fn choice_with(ui: &mut Ui, label: &str, after: &str, chosen: bool) -> Response {
    row(
        ui,
        label,
        after,
        Some(chosen),
        egui::WidgetType::RadioButton,
    )
}

/// A row that switches `on`: a check mark before it while on. A click
/// switches it (the menu stays open).
pub fn check(ui: &mut Ui, on: &mut bool, label: &str) -> Response {
    let response = row(ui, label, "", Some(*on), egui::WidgetType::Checkbox);
    if response.clicked() {
        *on = !*on;
    }
    response
}

/// Small capitals over a group of rows: "VIEWS".
pub fn caps(ui: &mut Ui, words: &str) {
    ui.add_space(size::MENU_PAD);
    let (rect, _) = ui.allocate_exact_size(
        vec2(ui.available_width(), size::MENU_CAPS_H),
        Sense::hover(),
    );
    let job = text::caps(words, text::SECTION_CAPS, tokens::text_meta());
    let galley = ui.painter().layout_job(job);
    let at = pos2(
        rect.left() + size::FIELD_PAD,
        rect.center().y - galley.size().y / 2.0,
    );
    ui.painter().galley(at, galley, tokens::text_meta());
}

/// The line between groups of rows.
pub fn line(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(
        vec2(ui.available_width(), 2.0 * size::MENU_PAD + 1.0),
        Sense::hover(),
    );
    ui.painter().hline(
        rect.shrink2(vec2(size::FIELD_PAD, 0.0)).x_range(),
        rect.center().y,
        egui::Stroke::new(super::hairline(ui), tokens::line()),
    );
}

/// How wide a menu of these `labels` must be, at least, so none is cut.
pub fn width_for(ui: &Ui, labels: &[&str]) -> f32 {
    let widest = labels
        .iter()
        .map(|label| width_of(ui, label, text::menu_item()))
        .fold(0.0, f32::max);
    widest + 2.0 * size::FIELD_PAD + size::MENU_CHECK_W + size::MENU_AFTER_W
}

fn row(
    ui: &mut Ui,
    label: &str,
    after: &str,
    checked: Option<bool>,
    kind: egui::WidgetType,
) -> Response {
    let width = ui.available_width().max(width_for(ui, &[label]));
    let (rect, response) = ui.allocate_exact_size(vec2(width, size::MENU_ROW_H), Sense::click());
    response.widget_info(|| match checked {
        Some(on) => WidgetInfo::selected(kind, ui.is_enabled(), on, label),
        None => WidgetInfo::labeled(kind, ui.is_enabled(), label),
    });
    if ui.is_rect_visible(rect) {
        if response.hovered() || response.has_focus() {
            ui.painter().rect_filled(
                rect,
                CornerRadius::same(size::radius::SMALL),
                tokens::hover(),
            );
        }
        let room = rect.shrink2(vec2(size::FIELD_PAD, 0.0));
        if checked == Some(true) {
            let mark = Rect::from_center_size(
                pos2(room.left() + size::MENU_CHECK_ICON / 2.0, room.center().y),
                vec2(size::MENU_CHECK_ICON, size::MENU_CHECK_ICON),
            );
            icons::icon(ui.painter(), mark, Icon::Check, tokens::text_body());
        }
        let words_room = room.with_min_x(room.left() + size::MENU_CHECK_W);
        let ink = if response.hovered() {
            tokens::text_emphasis()
        } else {
            tokens::text_primary()
        };
        words(
            ui,
            words_room,
            Align2::LEFT_CENTER,
            label,
            text::menu_item(),
            ink,
        );
        if !after.is_empty() {
            words(
                ui,
                room,
                Align2::RIGHT_CENTER,
                after,
                text::menu_item(),
                tokens::text_meta(),
            );
        }
        focus_ring(ui, &response, rect, size::radius::SMALL);
    }
    response
}
