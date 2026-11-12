//! The shelf (spec 153-158): a 34 px bar (the grip, the shelf's tabs,
//! and the tools of the tab in front at the right) over columns laid side
//! by side. The grip's drag sets its height; Ctrl + Space folds it to its
//! bar and back; each ribbon tab remembers its shelf tab, height and fold
//! (`Arrangement`). Columns wrap under each other when the window is too
//! narrow for them side by side.

use super::workspaces::{Column, OpenNow, ShelfTab, Width, shelf_tabs};
use super::{RibbonTab, Shell};
use crate::icons::Icon;
use crate::panel::Panel;
use crate::panels::PanelViewer;
use crate::tokens::{self, size, text};
use crate::unbuilt::{self, Entry, UnbuiltId};
use crate::widgets;
use egui::{Align, Align2, Id, Layout, Rect, Sense, Ui, UiBuilder, WidgetInfo, pos2, vec2};

/// The share of a column a second panel takes, under the first (Hints
/// under Properties, as it was docked).
const LOWER_SHARE: f32 = 0.28;

/// How tall the shelf is, bar and all, in a middle `room` tall: what the
/// tab remembers, kept so the center keeps its least height.
pub(crate) fn height(shell: &Shell, room: f32) -> f32 {
    let arrangement = shell.arrangement(shell.tab);
    let content = if arrangement.shelf_folded {
        0.0
    } else {
        let most = (room - size::SHELF_BAR_H - size::CENTER_MIN_H).max(0.0);
        arrangement.shelf_height.min(most)
    };
    (size::SHELF_BAR_H + content).min(room)
}

/// Draws the shelf into `rect` (bar and content).
pub(crate) fn shelf(
    ui: &mut Ui,
    rect: Rect,
    shell: &mut Shell,
    open: OpenNow,
    viewer: &mut PanelViewer<'_>,
) {
    let tab = shell.tab;
    let entries = unbuilt::filter(shelf_tabs(tab, open));
    let built: Vec<ShelfTab> = entries
        .iter()
        .filter_map(|e| match e {
            Entry::Built(t) => Some(*t),
            Entry::Unbuilt(_) => None,
        })
        .collect();
    let front = shell.arrangement(tab).shelf_tab_of(&built);
    let bar = rect.with_max_y(rect.top() + size::SHELF_BAR_H);
    bar_background(ui, bar);
    grip(ui, bar, shell, rect.height() - size::SHELF_BAR_H);
    let tabs_left = bar.left() + size::ROW_PAD + size::SHELF_GRIP[0] + size::ROW_PAD;
    let mut bar_ui = ui.new_child(
        UiBuilder::new()
            .max_rect(Rect::from_min_max(
                pos2(tabs_left, bar.top()),
                pos2(bar.right() - size::ROW_PAD, bar.bottom()),
            ))
            .layout(Layout::left_to_right(Align::Center))
            .id_salt("shelf bar"),
    );
    bar_ui.spacing_mut().item_spacing.x = size::TAB_BAR_GAP;
    let mut clicked = None;
    for entry in &entries {
        match entry {
            Entry::Built(shelf) => {
                let words = tab_words(*shelf, viewer);
                let response = shelf_tab(&mut bar_ui, *shelf, &words, Some(*shelf) == front);
                if response.clicked() {
                    clicked = Some(*shelf);
                }
                if shelf.closes_a_file() && close_mark(&mut bar_ui, *shelf) {
                    viewer.close(shelf.panels().next().unwrap_or(Panel::Frames));
                }
            }
            Entry::Unbuilt(id) => {
                unbuilt::shelf_tab(&mut bar_ui, *id);
            }
        }
    }
    bar_ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = size::GAP;
        tools(ui, tab, front, viewer);
    });
    if let Some(chosen) = clicked {
        let arrangement = shell.arrangement_mut(tab);
        arrangement.shelf_tab = chosen;
        arrangement.shelf_folded = false;
    }
    let content = rect.with_min_y(bar.bottom());
    if content.height() < 1.0 {
        return;
    }
    if let Some(front) = clicked.or(front) {
        columns(ui, content, front, shell, viewer);
    }
}

/// The bar's fill and its lines above and below.
fn bar_background(ui: &Ui, bar: Rect) {
    let painter = ui.painter();
    painter.rect_filled(bar, 0.0, tokens::shelf_bar());
    let ppp = ui.ctx().pixels_per_point();
    tokens::hline(painter, bar.x_range(), bar.top(), tokens::line());
    tokens::hline(
        painter,
        bar.x_range(),
        bar.bottom() - tokens::hairline(ppp),
        tokens::line(),
    );
}

/// The grip at the bar's left, and the bar's top edge: dragging either
/// sets the shelf's height (`drawn` is its content's height now).
fn grip(ui: &Ui, bar: Rect, shell: &mut Shell, drawn: f32) {
    let [w, h] = size::SHELF_GRIP;
    let spot = Rect::from_min_size(
        pos2(bar.left() + size::ROW_PAD, bar.center().y - h / 2.0),
        vec2(w, h),
    );
    ui.painter()
        .rect_filled(spot, size::radius::GRIP, tokens::stronger_line());
    let reach = spot.expand2(vec2(size::GAP, (size::SHELF_BAR_H - h) / 2.0));
    let held = ui.interact(reach, Id::new("shelf grip"), Sense::drag());
    held.widget_info(|| WidgetInfo::labeled(egui::WidgetType::Other, true, "Shelf grip"));
    let edge = Rect::from_x_y_ranges(
        bar.x_range(),
        bar.top() - size::DIVIDER_REACH..=bar.top() + size::DIVIDER_REACH,
    );
    let edge = ui.interact(edge, Id::new("shelf edge"), Sense::drag());
    for response in [&held, &edge] {
        if response.hovered() || response.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
        }
    }
    let moved: f32 = [held, edge].iter().map(|r| r.drag_delta().y).sum();
    if moved != 0.0 {
        // From the height drawn (a remembered one may not fit), by as
        // much as the pointer moved: up makes the shelf taller.
        let arrangement = shell.arrangement_mut(shell.tab);
        arrangement.shelf_folded = false;
        arrangement.shelf_height =
            (drawn - moved).clamp(size::SHELF_MIN_H, super::saved::SHELF_MAX);
    }
}

/// What a shelf tab says: its name, and for Problems how many there are.
fn tab_words(tab: ShelfTab, viewer: &PanelViewer<'_>) -> String {
    let problems = viewer.problem_count();
    if tab == ShelfTab::Problems && problems > 0 {
        format!("{} \u{b7} {problems}", tab.name())
    } else {
        tab.name().to_owned()
    }
}

/// One shelf tab: 26 px, rounded; the one in front filled.
fn shelf_tab(ui: &mut Ui, tab: ShelfTab, words: &str, front: bool) -> egui::Response {
    let font = if front {
        text::tab_active()
    } else {
        text::tab()
    };
    let galley = ui
        .painter()
        .layout_no_wrap(words.to_owned(), font, tokens::text_label());
    let width = 2.0 * size::ROW_PAD + galley.size().x;
    let (rect, response) = ui.allocate_exact_size(vec2(width, size::shelf_tab_h()), Sense::click());
    response
        .widget_info(|| WidgetInfo::selected(egui::WidgetType::Button, true, front, tab.name()));
    if ui.is_rect_visible(rect) {
        let (fill, ink) = if front {
            (Some(tokens::line()), tokens::text_emphasis())
        } else if response.hovered() {
            (Some(tokens::row_hover()), tokens::text_primary())
        } else {
            (None, tokens::text_label())
        };
        if let Some(fill) = fill {
            ui.painter().rect_filled(rect, size::radius::TAB, fill);
        }
        let font = if front {
            text::tab_active()
        } else {
            text::tab()
        };
        ui.painter()
            .text(rect.center(), Align2::CENTER_CENTER, words, font, ink);
        widgets::focus_ring(ui, &response, rect, size::radius::TAB);
    }
    response
}

/// The × after a shelf tab that shows a file of its own. Returns whether
/// it was clicked.
fn close_mark(ui: &mut Ui, tab: ShelfTab) -> bool {
    let (rect, response) =
        ui.allocate_exact_size(vec2(size::ICON_SMALL, size::ICON_SMALL), Sense::click());
    let name = format!("Close {}", tab.name());
    response.widget_info(|| WidgetInfo::labeled(egui::WidgetType::Button, true, &name));
    let ink = if response.hovered() {
        tokens::text_emphasis()
    } else {
        tokens::text_meta()
    };
    crate::icons::icon(ui.painter(), rect.shrink(2.0), Icon::Close, ink);
    response.on_hover_text("Closes the clips file").clicked()
}

/// The tools at the bar's right, laid out from the right: the Console's
/// filters, search, Clear and Copy while it's in front; Home's animation
/// transport (not built yet).
fn tools(ui: &mut Ui, tab: RibbonTab, front: Option<ShelfTab>, viewer: &mut PanelViewer<'_>) {
    if front == Some(ShelfTab::Place) {
        // What to do, where the design has its hint (spec 264).
        ui.label(
            egui::RichText::new(
                "Drag into the area, or double-click to place where the camera looks",
            )
            .font(crate::tokens::text::ui())
            .color(crate::tokens::text_meta()),
        );
        return;
    }
    if tab == RibbonTab::Home {
        let parts = vec![
            Entry::Unbuilt(UnbuiltId::HomeShelfFrameCounter),
            Entry::Unbuilt(UnbuiltId::HomeShelfTransport),
        ];
        for entry in unbuilt::filter::<()>(parts) {
            match entry {
                Entry::Unbuilt(UnbuiltId::HomeShelfTransport) => {
                    for icon in [Icon::NextKey, Icon::Play, Icon::PreviousKey] {
                        unbuilt::icon_button(ui, UnbuiltId::HomeShelfTransport, icon);
                    }
                }
                Entry::Unbuilt(part) => {
                    unbuilt::status_item(ui, part);
                }
                Entry::Built(()) => {}
            }
        }
    }
    if front == Some(ShelfTab::Console) {
        crate::console::console_tools(ui, viewer.console);
    }
    if tab == RibbonTab::Audio && front == Some(ShelfTab::SoundEvents) {
        // Who is ducking whom while the game plays (round 3).
        let ducking = viewer.mixer.ducking_now();
        let words = match (viewer.playing, ducking.is_empty()) {
            (false, _) => "Meters move while the game plays".to_owned(),
            (true, true) => "Playing".to_owned(),
            (true, false) => format!("Playing \u{b7} {}", ducking.join(", ")),
        };
        let ink = if viewer.playing {
            crate::tokens::WARNING
        } else {
            crate::tokens::text_meta()
        };
        ui.label(
            egui::RichText::new(words)
                .font(crate::tokens::text::ui())
                .color(ink),
        );
    }
    if tab == RibbonTab::Library && viewer.project.is_some() {
        // "48 items · 1 selected" (spec 315): the folder shown.
        let state = &viewer.library_state;
        let items = viewer
            .library
            .iter()
            .filter(|item| item.folder() == state.folder)
            .count();
        let picked = usize::from(state.selected.is_some());
        ui.label(
            egui::RichText::new(format!("{items} items \u{b7} {picked} selected"))
                .font(crate::tokens::text::ui())
                .color(crate::tokens::text_meta()),
        );
    }
}

/// The front shelf tab's columns in `content`, side by side, or under
/// each other when there isn't room.
fn columns(
    ui: &mut Ui,
    content: Rect,
    tab: ShelfTab,
    shell: &mut Shell,
    viewer: &mut PanelViewer<'_>,
) {
    let columns = tab.columns();
    // The 1 px gaps between columns show this.
    ui.painter().rect_filled(content, 0.0, tokens::line());
    for (column, place) in columns.iter().zip(places(content, columns)) {
        let body = if columns.len() > 1 && !column.own_header {
            header(ui, place, column, viewer)
        } else {
            place
        };
        ui.painter().rect_filled(body, 0.0, tokens::panel());
        let [upper, lower] = match column.panels {
            [one] => [(Some(*one), body), (None, Rect::NOTHING)],
            [first, second, ..] => {
                let y = (body.bottom() - body.height() * LOWER_SHARE).round();
                tokens::hline(ui.painter(), body.x_range(), y, tokens::line());
                [
                    (Some(*first), body.with_max_y(y)),
                    (Some(*second), body.with_min_y(y + 1.0)),
                ]
            }
            [] => continue,
        };
        for (panel, rect) in [upper, lower] {
            let Some(panel) = panel else { continue };
            let mut panel_ui = ui.new_child(
                UiBuilder::new()
                    .max_rect(rect)
                    .id_salt(("shelf column", panel)),
            );
            panel_ui.set_clip_rect(rect.intersect(ui.clip_rect()));
            viewer.draw_panel(&mut panel_ui, panel);
            shell.drawn.push((panel, rect));
        }
    }
}

/// Where each column goes in `content`: side by side with 1 px gaps, the
/// fixed ones at their width and the rest sharing what's left; or, when
/// that leaves a column narrower than its least, each under the one
/// before.
fn places(content: Rect, columns: &[Column]) -> Vec<Rect> {
    let gaps = columns.len().saturating_sub(1) as f32;
    let fixed: f32 = columns
        .iter()
        .map(|c| match c.width {
            Width::Fixed(w) => w,
            Width::Rest => 0.0,
        })
        .sum();
    let rest_count = columns
        .iter()
        .filter(|c| c.width == Width::Rest)
        .count()
        .max(1) as f32;
    let rest = (content.width() - fixed - gaps) / rest_count;
    if rest < size::COLUMN_MIN_W && columns.len() > 1 {
        let n = columns.len() as f32;
        let each = ((content.height() - (n - 1.0)) / n).floor();
        return (0..columns.len())
            .map(|i| {
                let top = content.top() + i as f32 * (each + 1.0);
                Rect::from_min_size(pos2(content.left(), top), vec2(content.width(), each))
            })
            .collect();
    }
    let mut x = content.left();
    columns
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let w = match c.width {
                Width::Fixed(w) => w,
                Width::Rest => rest,
            };
            let last = i + 1 == columns.len();
            let right = if last {
                content.right()
            } else {
                (x + w).round()
            };
            let rect = Rect::from_x_y_ranges(x..=right, content.y_range());
            x = right + 1.0;
            rect
        })
        .collect()
}

/// A column's 30 px header: its title, a count on the right where it has
/// one, and its panel's own tools left of the count. Returns the body
/// under it.
fn header(ui: &mut Ui, place: Rect, column: &Column, viewer: &mut PanelViewer<'_>) -> Rect {
    let head = place.with_max_y(place.top() + size::COLUMN_HEADER_H);
    let painter = ui.painter();
    painter.rect_filled(head, 0.0, tokens::panel());
    tokens::hline(painter, head.x_range(), head.bottom() - 1.0, tokens::line());
    // Named for screen readers (and tests), as a tab was before.
    let named = ui.interact(
        head,
        Id::new(("column header", column.title)),
        Sense::hover(),
    );
    named.widget_info(|| WidgetInfo::labeled(egui::WidgetType::Label, true, column.title));
    let title = painter.text(
        pos2(head.left() + size::ROW_PAD, head.center().y),
        Align2::LEFT_CENTER,
        column.title,
        text::header(),
        tokens::text_emphasis(),
    );
    if let Some(note) = viewer.column_note(column.panels[0]) {
        painter.text(
            pos2(title.right() + size::GAP, head.center().y),
            Align2::LEFT_CENTER,
            note,
            text::ui(),
            tokens::text_meta(),
        );
    }
    let mut right = head.right() - size::ROW_PAD;
    if let Some(count) = viewer.column_count(column.panels[0]) {
        let drawn = painter.text(
            pos2(right, head.center().y),
            Align2::RIGHT_CENTER,
            count,
            text::header(),
            tokens::text_meta(),
        );
        right = drawn.left() - size::GAP;
    }
    let tools = Rect::from_min_max(
        pos2(head.left() + size::ROW_PAD, head.top()),
        pos2(right, head.bottom() - 1.0),
    );
    let mut tools_ui = ui.new_child(
        UiBuilder::new()
            .id_salt(("column tools", column.title))
            .max_rect(tools)
            .layout(egui::Layout::right_to_left(egui::Align::Center)),
    );
    tools_ui.spacing_mut().item_spacing.x = 0.0;
    viewer.column_tools(&mut tools_ui, column.panels[0]);
    place.with_min_y(head.bottom())
}
