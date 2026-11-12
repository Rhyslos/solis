//! The center: one or two editor groups side by side (spec 148-151),
//! each a 30 px strip of document tabs over the document in front, with
//! a 1 px divider between them that a drag moves. When the window is too
//! narrow for two side by side, they stack (spec 1145). Which documents
//! each group holds is `workspaces::groups`; which is in front, and the
//! split, are the tab's `Arrangement`.
//!
//! The area's tab carries a **+** (New area, Open area…) and a menu on a
//! right-click (Save, Save as…, Close, Close others, Show in the
//! Library), as mock-up round 1 drew them; a document that comes with a
//! file has a × that closes the file.

use super::workspaces::{OpenNow, SPLIT_LIMIT, groups};
use super::{RibbonTab, Shell, ShellCommand};
use crate::icons::{self, Icon};
use crate::panel::Panel;
use crate::panels::PanelViewer;
use crate::tokens::{self, size, text};
use crate::widgets;
use egui::{Align2, CornerRadius, Id, Popup, Rect, Response, Sense, Ui, WidgetInfo, pos2, vec2};

/// What was asked of the document tabs this frame.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct DocAsks {
    /// Commands chosen from the tabs' menus.
    pub(crate) commands: Vec<ShellCommand>,
    /// The area's file is to be shown in the Library.
    pub(crate) show_area_in_library: bool,
    /// An unfinished part's page asked for unfinished parts to be hidden.
    pub(crate) hide_unfinished: bool,
}

/// Draws the center into `rect`: the groups of the tab in front, each
/// with its tab strip and the document in front.
pub(crate) fn center(
    ui: &mut Ui,
    rect: Rect,
    shell: &mut Shell,
    open: OpenNow,
    viewer: &mut PanelViewer<'_>,
) -> DocAsks {
    let tab = shell.tab;
    let documents = groups(tab, open);
    let mut asks = DocAsks::default();
    let side_by_side = rect.width() >= 2.0 * size::GROUP_MIN_W;
    let places = match documents.len() {
        0 => {
            if tab == RibbonTab::Animate && !crate::unbuilt::hidden() {
                // Neither a clips file nor an animator: the sequencer's
                // page (round 3, `06-animate-sequencer-unbuilt.png`).
                use crate::{icons::Icon, unbuilt::UnbuiltId};
                let page = &mut ui.new_child(egui::UiBuilder::new().max_rect(rect));
                let answer = crate::unbuilt::page(page, UnbuiltId::AnimShelfSequencer, Icon::Track);
                asks.hide_unfinished = answer.hide;
            }
            return asks;
        }
        1 => vec![rect],
        _ => split(ui, rect, shell, side_by_side),
    };
    for (index, (docs, place)) in documents.iter().zip(places).enumerate() {
        let front = shell.arrangement(tab).front_of(index, docs);
        let strip = place.with_max_y(place.top() + size::DOC_TAB_H);
        let body = place.with_min_y(strip.bottom());
        let chosen = tab_strip(ui, strip, (tab, index), docs, front, viewer, &mut asks);
        if let Some(chosen) = chosen {
            shell.arrangement_mut(tab).front[index] = Some(chosen);
        }
        let Some(front) = chosen.or(front) else {
            continue;
        };
        let mut body_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(body)
                .id_salt(("group", index)),
        );
        body_ui.set_clip_rect(body.intersect(ui.clip_rect()));
        ui.painter().rect_filled(body, 0.0, tokens::panel());
        viewer.draw_panel(&mut body_ui, front);
        shell.drawn.push((front, body));
    }
    asks
}

/// Splits `rect` between two groups at the tab's split, side by side or
/// stacked, and lets a drag on the divider move it.
fn split(ui: &mut Ui, rect: Rect, shell: &mut Shell, side_by_side: bool) -> Vec<Rect> {
    let tab = shell.tab;
    let share = shell.arrangement(tab).split;
    let (first, second, line) = if side_by_side {
        let x = (rect.left() + rect.width() * share).round();
        (
            rect.with_max_x(x),
            rect.with_min_x(x + 1.0),
            Rect::from_x_y_ranges(x..=x + 1.0, rect.y_range()),
        )
    } else {
        let y = (rect.top() + rect.height() * share).round();
        (
            rect.with_max_y(y),
            rect.with_min_y(y + 1.0),
            Rect::from_x_y_ranges(rect.x_range(), y..=y + 1.0),
        )
    };
    ui.painter().rect_filled(line, 0.0, tokens::group_gap());
    let reach = line.expand(size::DIVIDER_REACH);
    let response = ui.interact(reach, Id::new(("group divider", tab)), Sense::drag());
    response.widget_info(|| WidgetInfo::labeled(egui::WidgetType::Other, true, "Group divider"));
    if response.hovered() || response.dragged() {
        ui.ctx().set_cursor_icon(if side_by_side {
            egui::CursorIcon::ResizeHorizontal
        } else {
            egui::CursorIcon::ResizeVertical
        });
    }
    if response.dragged()
        && let Some(at) = response.interact_pointer_pos()
    {
        let share = if side_by_side {
            (at.x - rect.left()) / rect.width().max(1.0)
        } else {
            (at.y - rect.top()) / rect.height().max(1.0)
        };
        shell.arrangement_mut(tab).split = share.clamp(SPLIT_LIMIT, 1.0 - SPLIT_LIMIT);
    }
    vec![first, second]
}

/// One group's strip of document tabs. Returns the document clicked to
/// the front, if one was.
fn tab_strip(
    ui: &mut Ui,
    strip: Rect,
    (tab, index): (RibbonTab, usize),
    documents: &[Panel],
    front: Option<Panel>,
    viewer: &mut PanelViewer<'_>,
    asks: &mut DocAsks,
) -> Option<Panel> {
    ui.painter().rect_filled(strip, 0.0, tokens::app());
    let mut chosen = None;
    let mut x = strip.left();
    for panel in documents {
        let title = viewer.tab_title(*panel);
        let closes = viewer.closes_a_file(*panel);
        let at = pos2(x, strip.top());
        let (response, close) = doc_tab(ui, at, &title, *panel == front.unwrap_or(*panel), closes);
        x = response.rect.right();
        if response.clicked() {
            chosen = Some(*panel);
        }
        if close {
            viewer.close(*panel);
        }
        doc_menu(&response, *panel, documents, viewer, asks);
    }
    let area_group = documents.contains(&Panel::Viewport) && tab != RibbonTab::Animate;
    if area_group {
        plus(ui, pos2(x, strip.top()), (tab, index), asks);
    }
    if RibbonTab::CONTEXT.contains(&tab) && index == 0 {
        back_link(ui, strip, tab.parent(), asks);
    }
    if documents == [Panel::Bus] {
        viewer.bus_switch(ui, strip);
    }
    chosen
}

/// "‹ Back to Library" at the right of a LIBRARY TOOLS document's strip
/// (round 3): its parent tab in front again, the file still open.
fn back_link(ui: &mut Ui, strip: Rect, parent: RibbonTab, asks: &mut DocAsks) {
    let mut side = ui.new_child(
        egui::UiBuilder::new()
            .id_salt(("back to", parent))
            .max_rect(strip.shrink2(vec2(size::ROW_PAD, 0.0)))
            .layout(egui::Layout::right_to_left(egui::Align::Center)),
    );
    let words = format!("\u{2039} Back to {}", parent.name());
    if crate::widgets::button::link(&mut side, &words).clicked() {
        asks.commands.push(ShellCommand::Tab(parent));
    }
}

/// One document tab at `at`: the name, and a × when it closes a file.
/// Returns its response and whether its × was clicked.
fn doc_tab(
    ui: &mut Ui,
    at: egui::Pos2,
    title: &str,
    front: bool,
    closes: bool,
) -> (Response, bool) {
    let font = text::tab();
    let words = ui
        .painter()
        .layout_no_wrap(title.to_owned(), font.clone(), tokens::text_meta());
    let close_w = if closes {
        size::DOC_TAB_GAP + size::ICON_SMALL
    } else {
        0.0
    };
    let width = 2.0 * size::DOC_TAB_PAD + words.size().x + close_w;
    let rect = Rect::from_min_size(at, vec2(width, size::DOC_TAB_H));
    let response = ui.interact(rect, Id::new(("document tab", title)), Sense::click());
    response.widget_info(|| WidgetInfo::selected(egui::WidgetType::Button, true, front, title));
    let painter = ui.painter();
    let (fill, ink) = if front {
        (tokens::viewport(), tokens::text_emphasis())
    } else if response.hovered() {
        (tokens::hover(), tokens::text_primary())
    } else {
        (tokens::shelf_bar(), tokens::text_meta())
    };
    painter.rect_filled(rect, CornerRadius::ZERO, fill);
    if front {
        // The 1 px accent line along the front tab's top (spec 150).
        let accent = crate::interface_look::in_use().accent.colors().fill;
        painter.rect_filled(rect.with_max_y(rect.top() + 1.0), 0.0, accent);
    }
    painter.text(
        pos2(rect.left() + size::DOC_TAB_PAD, rect.center().y),
        Align2::LEFT_CENTER,
        title,
        font,
        ink,
    );
    let mut close = false;
    if closes {
        let spot = Rect::from_center_size(
            pos2(
                rect.right() - size::DOC_TAB_PAD - size::ICON_SMALL / 2.0,
                rect.center().y,
            ),
            vec2(size::ICON_SMALL, size::ICON_SMALL),
        );
        let x = ui.interact(spot, Id::new(("close document", title)), Sense::click());
        let name = format!("Close {title}");
        x.widget_info(|| WidgetInfo::labeled(egui::WidgetType::Button, true, &name));
        let x_ink = if x.hovered() {
            tokens::text_emphasis()
        } else {
            tokens::text_meta()
        };
        icons::icon(ui.painter(), spot.shrink(2.0), Icon::Close, x_ink);
        close = x.clicked();
        x.on_hover_text("Closes the file");
    }
    widgets::focus_ring(ui, &response, rect, 0);
    (response, close)
}

/// The **+** after the area's tab: New area and Open area….
fn plus(ui: &mut Ui, at: egui::Pos2, (tab, index): (RibbonTab, usize), asks: &mut DocAsks) {
    let rect = Rect::from_min_size(at, vec2(size::DOC_TAB_H, size::DOC_TAB_H));
    let response = ui.interact(rect, Id::new(("new document", tab, index)), Sense::click());
    response
        .widget_info(|| WidgetInfo::labeled(egui::WidgetType::Button, true, "New or open an area"));
    let ink = if response.hovered() {
        tokens::text_emphasis()
    } else {
        tokens::text_meta()
    };
    let spot = Rect::from_center_size(rect.center(), vec2(size::ICON_SMALL, size::ICON_SMALL));
    icons::icon(ui.painter(), spot, Icon::Plus, ink);
    let response = response.on_hover_text("A new area, or one of the project's areas");
    Popup::menu(&response).show(|ui| {
        for command in [ShellCommand::NewArea, ShellCommand::OpenArea] {
            if ui.button(command_words(command)).clicked() {
                asks.commands.push(command);
                ui.close();
            }
        }
    });
}

/// The words a document tab's menu shows for `command`.
fn command_words(command: ShellCommand) -> &'static str {
    match command {
        ShellCommand::NewArea => "New area",
        ShellCommand::OpenArea => "Open area\u{2026}",
        ShellCommand::SaveArea => "Save",
        ShellCommand::SaveAreaAs => "Save as\u{2026}",
        _ => "",
    }
}

/// A right-click on a document tab: the area's has Save, Save as…, Close,
/// Close others and Show in the Library; a file's has Close and Close
/// others.
fn doc_menu(
    response: &Response,
    panel: Panel,
    documents: &[Panel],
    viewer: &mut PanelViewer<'_>,
    asks: &mut DocAsks,
) {
    let others: Vec<Panel> = documents
        .iter()
        .copied()
        .filter(|p| *p != panel && viewer.closes_a_file(*p))
        .collect();
    let closes = viewer.closes_a_file(panel);
    if panel != Panel::Viewport && !closes {
        return;
    }
    Popup::context_menu(response).show(|ui| {
        if panel == Panel::Viewport {
            for command in [ShellCommand::SaveArea, ShellCommand::SaveAreaAs] {
                let words = command_words(command);
                let keys = command.keys();
                let shown = if keys.is_empty() {
                    words.to_owned()
                } else {
                    format!("{words}    {keys}")
                };
                if ui.button(shown).clicked() {
                    asks.commands.push(command);
                    ui.close();
                }
            }
            ui.separator();
        }
        if closes {
            if ui.button("Close").clicked() {
                viewer.close(panel);
                ui.close();
            }
        } else {
            faint(
                ui,
                "Close",
                "One area is open at a time: open or make another in its place",
            );
        }
        if others.is_empty() {
            faint(ui, "Close others", "Nothing else in this group closes");
        } else if ui.button("Close others").clicked() {
            for other in &others {
                viewer.close(*other);
            }
            ui.close();
        }
        if panel == Panel::Viewport {
            ui.separator();
            if ui.button("Show in the Library").clicked() {
                asks.show_area_in_library = true;
                ui.close();
            }
        }
    });
}

/// A menu item that can't be chosen now, with why on hover.
fn faint(ui: &mut Ui, words: &str, why: &str) {
    let faint = egui::RichText::new(words).color(tokens::text_faint());
    ui.add(egui::Label::new(faint).selectable(false))
        .on_hover_text(why);
}
