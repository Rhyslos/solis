//! Preferences › Shortcuts (round 2's `11-prefs-shortcuts`): today's
//! keys as a table in the style of the Input page, to read. The editor's
//! own keys are fixed in its code; changing one is an unfinished part.

use super::plain;
use crate::tokens::{self, size, text};
use crate::unbuilt::{self, Entry, UnbuiltId};
use crate::widgets::{self, field, keycap, segmented::segmented};
use crate::workbench::Workbench;
use egui::{Align, Layout, Ui, vec2};

/// What the table shows: every row, or only keys, or only the mouse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Device {
    #[default]
    All,
    Keys,
    Mouse,
}

/// A row: the action, where it works, and its keys. Keys are written as
/// they are pressed, `+` between keys held together, ` or ` between
/// choices; a mouse action is written in words.
type Shortcut = (&'static str, &'static str, &'static str);

/// The table's groups and rows, as the editor has them today.
pub(crate) const SHORTCUTS: &[(&str, &[Shortcut])] = &[
    (
        "Tools",
        &[
            ("Select", "The viewport", "Q"),
            ("Move", "The viewport", "W"),
            ("Rotate", "The viewport", "R"),
            ("Scale", "The viewport", "S"),
            ("Transform: move, rotate and scale", "The viewport", "T"),
            ("Lock or free an axis", "The viewport", "X or Y or Z"),
            ("World or object axes", "The viewport", "L"),
        ],
    ),
    (
        "Editing",
        &[
            ("Undo", "Everywhere, except while typing", "Ctrl+Z"),
            (
                "Redo",
                "Everywhere, except while typing",
                "Ctrl+Y or Ctrl+Shift+Z",
            ),
            ("New area", "Everywhere", "Ctrl+N"),
            ("Open area", "Everywhere", "Ctrl+O"),
            ("Save area", "Everywhere", "Ctrl+S"),
            ("Save area as", "Everywhere", "Ctrl+Shift+S"),
            (
                "Select everything",
                "The viewport and the Area column",
                "Ctrl+A",
            ),
            (
                "Duplicate the selection",
                "The viewport and the Area column",
                "Ctrl+D",
            ),
            (
                "Rename",
                "The viewport, the Area column; the Library's file",
                "F2",
            ),
            (
                "Delete the selection",
                "The viewport, the Area column; the Library's file",
                "Delete",
            ),
            ("Point the camera at the selection", "The viewport", "F"),
        ],
    ),
    (
        "Search and the shelf",
        &[
            (
                "Search or run a command",
                "Everywhere, with nothing being typed",
                "Space",
            ),
            ("Fold or unfold the shelf", "Everywhere", "Ctrl+Space"),
            ("Leave a File page", "File", "Esc"),
        ],
    ),
    (
        "The camera",
        &[
            ("Look around", "The viewport", "Right mouse drag"),
            (
                "Fly",
                "The viewport, holding the right mouse button",
                "W or A or S or D",
            ),
            (
                "Fly down, up",
                "The viewport, holding the right mouse button",
                "Q or E",
            ),
            (
                "Change the camera's speed",
                "The viewport, holding the right mouse button",
                "Mouse wheel",
            ),
            ("Slide the view", "The viewport", "Middle mouse drag"),
            (
                "Step forward or back; zoom a flat view",
                "The viewport",
                "Mouse wheel",
            ),
        ],
    ),
    (
        "Selecting",
        &[
            ("Select", "The viewport", "Click"),
            (
                "Add to or take from the selection",
                "The viewport",
                "Ctrl+Click",
            ),
            ("Select with a box", "The viewport", "Drag on empty space"),
            ("Select nothing; cancel a drag", "The viewport", "Esc"),
            ("Snapping on or off for one drag", "A gizmo", "Ctrl"),
            ("Ten times finer", "A gizmo", "Shift"),
        ],
    ),
];

/// Whether a row's keys are the mouse's.
fn is_mouse(keys: &str) -> bool {
    ["mouse", "Click", "Drag"]
        .iter()
        .any(|word| keys.contains(word))
}

impl Workbench {
    pub(super) fn shortcuts(&mut self, ui: &mut Ui) {
        let pages = &mut self.file_pages;
        ui.horizontal(|ui| {
            plain(
                ui,
                "Every key the editor has today. Where the pointer rests decides what a key acts on.",
                text::page_subtitle(),
                tokens::text_body(),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = size::CARD_INNER_GAP;
                for entry in unbuilt::filter(vec![Entry::<()>::Unbuilt(UnbuiltId::PrefsShortcutsEdit)]) {
                    if let Entry::Unbuilt(id) = entry {
                        unbuilt::chip(ui, id);
                    }
                }
                field::search_field(
                    ui,
                    "Find an action or a key",
                    &mut pages.shortcuts_search,
                    "Find an action or a key",
                    size::PAGE_SEARCH_W,
                );
                let choices = [(Device::All, "All"), (Device::Keys, "Keys"), (Device::Mouse, "Mouse")];
                segmented(ui, "Which shortcuts", &mut pages.shortcuts_device, &choices);
            });
        });
        ui.add_space(size::SECTION_GAP);
        let words = pages.shortcuts_search.trim().to_lowercase();
        let device = pages.shortcuts_device;
        widgets::card::frame(ui)
            .fill(tokens::panel())
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                let columns = column_lefts(ui.available_width());
                header(ui, &columns);
                for (group, rows) in SHORTCUTS {
                    let shown: Vec<&Shortcut> = rows
                        .iter()
                        .filter(|(action, place, keys)| {
                            let text = format!("{action} {place} {keys}").to_lowercase();
                            let kind = match device {
                                Device::All => true,
                                Device::Keys => !is_mouse(keys),
                                Device::Mouse => is_mouse(keys),
                            };
                            kind && words.split_whitespace().all(|w| text.contains(w))
                        })
                        .collect();
                    if shown.is_empty() {
                        continue;
                    }
                    ui.add_space(size::GAP);
                    widgets::rows::section_caps(ui, group);
                    for row in shown {
                        shortcut_row(ui, &columns, row);
                    }
                }
            });
    }
}

/// Where each column starts, from the table's left.
fn column_lefts(width: f32) -> [f32; 3] {
    [0.0, width * 0.36, width * 0.68]
}

/// The table's heading.
fn header(ui: &mut Ui, columns: &[f32; 3]) {
    let (rect, _) = ui.allocate_exact_size(
        vec2(ui.available_width(), size::COLUMN_HEADER_H),
        egui::Sense::hover(),
    );
    for (left, words) in columns.iter().zip(["Action", "Where it works", "Key"]) {
        let at = egui::pos2(rect.left() + left, rect.center().y);
        let job = text::caps(words, text::SECTION_CAPS, tokens::text_meta());
        let galley = ui.painter().layout_job(job);
        ui.painter().galley(
            at - vec2(0.0, galley.size().y / 2.0),
            galley,
            tokens::text_meta(),
        );
    }
    tokens::hline(ui.painter(), rect.x_range(), rect.bottom(), tokens::line());
}

/// One row: the action, where it works, and its keys as keycaps.
fn shortcut_row(ui: &mut Ui, columns: &[f32; 3], (action, place, keys): &Shortcut) {
    let height = size::KEYCAP + 2.0 * size::GAP + size::GAP;
    let (rect, _) =
        ui.allocate_exact_size(vec2(ui.available_width(), height), egui::Sense::hover());
    let at = |n: usize| {
        egui::Rect::from_min_max(egui::pos2(rect.left() + columns[n], rect.top()), rect.max)
    };
    widgets::words(
        ui,
        at(0),
        egui::Align2::LEFT_CENTER,
        action,
        text::ui(),
        tokens::text_primary(),
    );
    widgets::words(
        ui,
        at(1),
        egui::Align2::LEFT_CENTER,
        place,
        text::row(),
        tokens::text_label(),
    );
    let mut keys_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(at(2))
            .layout(Layout::left_to_right(Align::Center)),
    );
    keys_ui.spacing_mut().item_spacing.x = size::GAP;
    for (n, choice) in keys.split(" or ").enumerate() {
        if n > 0 {
            plain(&mut keys_ui, "or", text::row(), tokens::text_meta());
        }
        if is_mouse(choice) {
            plain(&mut keys_ui, choice, text::row(), tokens::text_body());
            continue;
        }
        for (m, key) in choice.split('+').enumerate() {
            if m > 0 {
                plain(&mut keys_ui, "+", text::row(), tokens::text_meta());
            }
            keycap::keycap(&mut keys_ui, key, keycap::KeycapState::Bound);
        }
    }
    tokens::hline(
        ui.painter(),
        rect.x_range(),
        rect.bottom(),
        tokens::row_hover(),
    );
}
