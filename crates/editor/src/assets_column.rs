//! The Library's placing form (spec 270-276; round 2, `06-placing-a.png`):
//! while Home's Library is placing (its **Place** switch), chips, a search
//! and **Open Library** over the placeable files as tiles of at least
//! 92 px, the one being dragged dimmed with a "dragging" badge. A tile is
//! dragged into the viewport to place it, onto an entity in the Level
//! panel to place it under that, or onto a card's field to assign it; a
//! double-click places it where the camera looks; a right-click has the
//! Library's menu for it. Otherwise the same browser as the Library tab's
//! stands there (decision 0118).

use crate::library::{LibraryFiles, LibraryItem, LibraryRequest, LibraryState, Sounds};
use crate::tokens::{self, size, text};
use crate::widgets::{self, dashed};
use egui::{CornerRadius, Rect, Response, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

/// The placing form: the placeable files under its chips and search.
/// Returns what was asked for.
pub(crate) fn assets(
    ui: &mut Ui,
    has_project: bool,
    items: &LibraryFiles,
    state: &mut LibraryState,
    sounds: Sounds<'_>,
) -> Vec<LibraryRequest> {
    let mut requests = Vec::new();
    if !has_project {
        crate::panels::empty_note(ui, "No project is open: its files show here.");
        return requests;
    }
    place_bar(ui, state, &mut requests);
    let wanted = state.search.trim().to_lowercase();
    let shown: Vec<&LibraryItem> = items
        .with_lowered()
        .filter(|(item, lowered)| {
            let here = if wanted.is_empty() {
                item.folder() == state.folder
            } else {
                lowered.contains(&wanted)
            };
            here && item.kind.placeable() && state.filter.passes(item.kind)
        })
        .map(|(item, _)| item)
        .collect();
    let room = ui.available_rect_before_wrap().shrink(size::TILE_GAP * 2.0);
    let least = size::PLACE_TILE_MIN_W;
    let across = ((room.width() + size::TILE_GAP) / (least + size::TILE_GAP))
        .floor()
        .max(1.0) as usize;
    let width = ((room.width() - (across - 1) as f32 * size::TILE_GAP) / across as f32).floor();
    let height = tile_height(width);
    let tiles = shown.len();
    let rows = tiles.div_ceil(across);
    let mut inner = ui.new_child(egui::UiBuilder::new().max_rect(room));
    // Only the rows in view are drawn, so a folder of thousands of files
    // costs no more than a screenful (CONTRIBUTING.md, rule 18).
    egui::ScrollArea::vertical()
        .id_salt("assets column")
        .auto_shrink(false)
        .show_rows(&mut inner, height + size::TILE_GAP, rows, |ui, range| {
            for row in range {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = size::TILE_GAP;
                    for item in shown.iter().skip(row * across).take(across) {
                        let chosen = state.is_picked(&item.path);
                        let response = tile(ui, width, item, chosen, sounds);
                        crate::library::act_on(item, response, state, &mut requests);
                    }
                });
                ui.add_space(size::TILE_GAP);
            }
        });
    ui.advance_cursor_after_rect(room);
    requests
}

/// The placing form's bar over the tiles: Favourites and Recent (not
/// built yet), the folder, the kinds that can be placed with their dots,
/// the search, and **Open Library**.
fn place_bar(ui: &mut Ui, state: &mut LibraryState, requests: &mut Vec<LibraryRequest>) {
    use crate::library::Filter;
    use crate::unbuilt::{self, Entry, UnbuiltId};
    use crate::widgets::chip::{ChipKind, chip};
    ui.add_space(size::TILE_GAP * 2.0);
    ui.horizontal_wrapped(|ui| {
        ui.add_space(size::TILE_GAP * 2.0);
        ui.spacing_mut().item_spacing = vec2(size::GAP, size::ROW_GAP);
        let later = vec![
            Entry::<()>::Unbuilt(UnbuiltId::PlaceChipFavorites),
            Entry::Unbuilt(UnbuiltId::PlaceChipRecent),
        ];
        for entry in unbuilt::filter(later) {
            if let Entry::Unbuilt(part) = entry {
                unbuilt::chip(ui, part);
            }
        }
        for filter in [Filter::All, Filter::Models, Filter::Sprites, Filter::Maps] {
            let dot = filter.dot();
            if chip(
                ui,
                ChipKind::Quiet,
                filter.name(),
                state.filter == filter,
                dot,
            )
            .clicked()
            {
                state.filter = filter;
            }
        }
        let field = widgets::field::text_field(
            ui,
            "Find something to place",
            &mut state.search,
            "Find something to place",
            size::PLACE_SEARCH_W,
        );
        let _ = field;
        if ui
            .link(
                egui::RichText::new("Open Library")
                    .font(text::ui())
                    .color(tokens::LINK),
            )
            .on_hover_text("The whole Library, on its own tab")
            .clicked()
        {
            requests.push(LibraryRequest::ShowLibraryTab);
        }
    });
    ui.add_space(size::TILE_GAP);
}

/// How tall a tile `width` wide is: its square, and its name under it.
fn tile_height(width: f32) -> f32 {
    width + size::GAP + size::ASSET_NAME_H
}

/// One file's tile: its square (2 px of the accent inside it when
/// picked) and its name under it, cut off with "…".
fn tile(ui: &mut Ui, width: f32, item: &LibraryItem, chosen: bool, sounds: Sounds<'_>) -> Response {
    let (rect, response) =
        ui.allocate_exact_size(vec2(width, tile_height(width)), Sense::click_and_drag());
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Button, true, chosen, &item.name)
    });
    if ui.is_rect_visible(rect) {
        let square = Rect::from_min_size(rect.min, vec2(width, width));
        let round = CornerRadius::same(size::radius::TAB);
        let fill = if response.hovered() {
            tokens::card_border()
        } else {
            tokens::hover()
        };
        ui.painter().rect_filled(square, round, fill);
        crate::library::paint_thumbnail(&ui.painter().with_clip_rect(square), square, item, sounds);
        if response.dragged() {
            dragging(ui, square);
        }
        if chosen {
            ui.painter().rect_stroke(
                square,
                round,
                Stroke::new(size::TILE_OUTLINE, crate::interface_look::accent_fill()),
                StrokeKind::Inside,
            );
        }
        let name = Rect::from_min_max(pos2(rect.left(), square.bottom() + size::GAP), rect.max);
        widgets::cut_off(ui, name, &item.name, text::tile_name(), tokens::text_tile());
    }
    response.on_hover_text(format!("{} · {}", item.kind.name(), item.path))
}

/// A tile being dragged out (spec 274): dimmed, a dashed accent border,
/// and a small "dragging" badge at its bottom left.
fn dragging(ui: &Ui, square: Rect) {
    let accent = crate::interface_look::accent_fill();
    ui.painter().rect_filled(
        square,
        CornerRadius::same(size::radius::TAB),
        tokens::app().gamma_multiply(1.0 - tokens::DRAGGED_OPACITY),
    );
    dashed::rounded(ui, square, size::radius::TAB, accent);
    let font = text::tile_kind();
    let words = ui
        .painter()
        .layout_no_wrap("dragging".to_owned(), font, crate::tokens::ON_ACCENT);
    let badge = Rect::from_min_size(
        pos2(
            square.left() + size::TILE_PAD,
            square.bottom() - size::TILE_PAD - words.size().y - 2.0,
        ),
        words.size() + vec2(2.0 * size::ROW_GAP, 2.0),
    );
    ui.painter()
        .rect_filled(badge, CornerRadius::same(size::radius::SMALL), accent);
    ui.painter().galley(
        badge.min + vec2(size::ROW_GAP, 1.0),
        words,
        crate::tokens::ON_ACCENT,
    );
}
