//! Project settings › Input's table (Screen 13): a row per action, its
//! keys and gamepad buttons, a click picks it; the clashes under it.

use super::input_page::{Devices, InputPage, clashes, is_pad};
use super::plain;
use crate::tokens::{self, size, text};
use crate::widgets::{self, keycap};
use egui::{Align, Layout, Rect, Sense, Ui, vec2};
use input::{Action, ActionKind, Bindings};

/// Where the table's columns start, as shares of its width.
const COLUMNS: [f32; 4] = [0.0, 0.25, 0.36, 0.7];

/// An action's inputs of one kind of device, in order.
fn inputs_of(action: &Action, pad: bool) -> Vec<&str> {
    [
        &action.inputs,
        &action.up,
        &action.down,
        &action.left,
        &action.right,
    ]
    .into_iter()
    .flatten()
    .filter(|i| is_pad(i) == pad)
    .map(String::as_str)
    .collect()
}

/// The table of actions: a row each, a click picks one; the clashes
/// under it.
pub(super) fn actions_table(ui: &mut Ui, page: &mut InputPage, bindings: &Bindings) {
    let clashes = clashes(bindings);
    let words = page.search.trim().to_lowercase();
    widgets::card::frame(ui)
        .fill(tokens::panel())
        .inner_margin(0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 0.0;
            let width = ui.available_width();
            let (head, _) =
                ui.allocate_exact_size(vec2(width, size::COLUMN_HEADER_H), Sense::hover());
            for (share, caps) in
                COLUMNS
                    .iter()
                    .zip(["Action", "Type", "Keyboard & mouse", "Gamepad"])
            {
                let job = text::caps(caps, text::SECTION_CAPS, tokens::text_meta());
                let galley = ui.painter().layout_job(job);
                let at = egui::pos2(
                    head.left() + size::ROW_PAD + share * width,
                    head.center().y - galley.size().y / 2.0,
                );
                ui.painter().galley(at, galley, tokens::text_meta());
            }
            tokens::hline(ui.painter(), head.x_range(), head.bottom(), tokens::line());
            for (index, action) in bindings.actions.iter().enumerate() {
                let text = format!("{} {}", action.name, inputs_of(action, false).join(" "))
                    .to_lowercase();
                let pad_text = inputs_of(action, true).join(" ").to_lowercase();
                if !words
                    .split_whitespace()
                    .all(|w| text.contains(w) || pad_text.contains(w))
                {
                    continue;
                }
                let clashing = clashes
                    .iter()
                    .any(|(_, a, b)| *a == action.name || *b == action.name);
                if action_row(ui, page, index, action, clashing).clicked() {
                    page.picked = Some(index);
                    page.picked_binding = None;
                }
            }
            for (input, a, b) in &clashes {
                clash_line(ui, input, a, b);
            }
        });
}

/// One action's row. Returns its response, for the pick.
fn action_row(
    ui: &mut Ui,
    page: &InputPage,
    index: usize,
    action: &Action,
    clashing: bool,
) -> egui::Response {
    let width = ui.available_width();
    let height = size::KEYCAP + 2.0 * size::CARD_INNER_GAP;
    let (rect, row) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    let picked = page.picked == Some(index);
    row.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Button, true, picked, &action.name)
    });
    let accent = crate::interface_look::in_use().accent;
    if picked {
        ui.painter()
            .rect_filled(rect, 0.0, tokens::select_row(accent));
        let bar = Rect::from_min_size(rect.min, vec2(size::TAB_UNDERLINE, rect.height()));
        ui.painter().rect_filled(bar, 0.0, accent.colour());
    } else if row.hovered() {
        ui.painter().rect_filled(rect, 0.0, tokens::row_hover());
    }
    let column = |n: usize| {
        Rect::from_min_max(
            egui::pos2(rect.left() + size::ROW_PAD + COLUMNS[n] * width, rect.top()),
            rect.max,
        )
    };
    let name = widgets::words(
        ui,
        column(0),
        egui::Align2::LEFT_CENTER,
        &action.name,
        text::ui(),
        tokens::text_primary(),
    );
    if clashing {
        let spot = Rect::from_center_size(
            egui::pos2(
                name.right() + size::GAP + size::ICON_SMALL / 2.0,
                rect.center().y,
            ),
            vec2(size::ICON_SMALL, size::ICON_SMALL),
        );
        crate::icons::icon(
            ui.painter(),
            spot,
            crate::icons::Icon::Warning,
            tokens::WARNING,
        );
    }
    let kind = match action.kind {
        ActionKind::Button => "Button",
        ActionKind::Axis => "Axis",
    };
    widgets::words(
        ui,
        column(1),
        egui::Align2::LEFT_CENTER,
        kind,
        text::row(),
        tokens::text_label(),
    );
    for (n, pad) in [(2, false), (3, true)] {
        let shown = match page.devices {
            Devices::All => true,
            Devices::KeyboardMouse => !pad,
            Devices::Gamepad => pad,
        };
        if !shown {
            continue;
        }
        let room = if n == 2 {
            column(2).with_max_x(column(3).left() - size::GAP)
        } else {
            column(3)
        };
        let mut caps = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(room)
                .layout(Layout::left_to_right(Align::Center)),
        );
        caps.set_clip_rect(room);
        caps.spacing_mut().item_spacing.x = size::GAP;
        for input in inputs_of(action, pad) {
            let state = if clashing {
                keycap::KeycapState::Conflict
            } else {
                keycap::KeycapState::Bound
            };
            keycap::keycap(&mut caps, input.trim_start_matches("Pad "), state);
        }
    }
    tokens::hline(
        ui.painter(),
        rect.x_range(),
        rect.bottom(),
        tokens::row_hover(),
    );
    row
}

/// A clash under the table: "F is bound to both Interact and OpenMap."
fn clash_line(ui: &mut Ui, input: &str, a: &str, b: &str) {
    let words = format!("{input} is bound to both {a} and {b}. Both fire on one press.");
    egui::Frame::new()
        .fill(tokens::CONFLICT_BANNER)
        .inner_margin(egui::Margin::symmetric(
            size::ROW_PAD as i8,
            size::CARD_INNER_GAP as i8,
        ))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let (spot, _) = ui
                    .allocate_exact_size(vec2(size::ICON_SMALL, size::ICON_SMALL), Sense::hover());
                crate::icons::icon(
                    ui.painter(),
                    spot,
                    crate::icons::Icon::Warning,
                    tokens::WARNING,
                );
                plain(ui, &words, text::row(), tokens::CONFLICT_BANNER_TEXT);
            });
        });
}
