//! The side columns (decision 0118, board 03): each slot's part drawn
//! with its header (its title, quiet words after it, its panel's own
//! tools, a count and the ⋯ menu that hides it), and the edges between
//! everything in the middle, which a drag moves and a double-click puts
//! back. The frame's three accent edges carry a grip at their middle, and
//! every edge shows the accent while it is pointed at or dragged.

use super::Shell;
use super::layouts::{Border, PanelLayout, Placed};
use super::workspaces::{Column, Slot};
use crate::icons::Icon;
use crate::panels::PanelViewer;
use crate::tokens::{self, size, text};
use crate::widgets;
use egui::{
    Align2, CursorIcon, Id, Rect, Sense, Ui, UiBuilder, WidgetInfo, WidgetType, pos2, vec2,
};

/// The share of a column a second panel takes, under the first (the
/// Parameters under a clips file's Clips on Animate, the Library under
/// how placed things land).
const LOWER_SHARE: f32 = 0.5;

/// What the side columns asked for this frame.
#[derive(Debug, Default, PartialEq)]
pub(super) struct ColumnAsks {
    /// The slot a ⋯ menu asked to hide.
    pub(super) hide: Option<Slot>,
    /// An unfinished part's page asked for unfinished parts to be hidden.
    pub(super) hide_unfinished: bool,
    /// The Library column's Place switch was clicked.
    pub(super) place: bool,
}

/// Draws the side columns' parts.
pub(super) fn columns(
    ui: &mut Ui,
    placed: &[Placed],
    shell: &mut Shell,
    viewer: &mut PanelViewer<'_>,
) -> ColumnAsks {
    let mut asks = ColumnAsks::default();
    for part in placed {
        if part_with_header(ui, part, shell, viewer, &mut asks) {
            asks.hide = Some(part.slot);
        }
    }
    asks
}

/// One part: its header, then its panels (or its unfinished page).
/// Returns whether its ⋯ menu asked to hide it.
fn part_with_header(
    ui: &mut Ui,
    part: &Placed,
    shell: &mut Shell,
    viewer: &mut PanelViewer<'_>,
    asks: &mut ColumnAsks,
) -> bool {
    let home = shell.tab == super::RibbonTab::Home;
    let (body, hide, place) = header(ui, part, home, viewer);
    asks.place |= place;
    ui.painter().rect_filled(body, 0.0, tokens::panel());
    if let Some(crate::unbuilt::Entry::Unbuilt(id)) = part.column.unbuilt {
        let page = &mut ui.new_child(UiBuilder::new().max_rect(body));
        page.set_clip_rect(body.intersect(ui.clip_rect()));
        asks.hide_unfinished |= crate::unbuilt::page(page, id, Icon::Profiler).hide;
        return hide;
    }
    match part.column.panels {
        [one] => super::bottom::draw(ui, body, *one, shell, viewer),
        [first, second, ..] => {
            let y = (body.bottom() - body.height() * LOWER_SHARE).round();
            tokens::hline(ui.painter(), body.x_range(), y, tokens::line());
            super::bottom::draw(ui, body.with_max_y(y), *first, shell, viewer);
            super::bottom::draw(ui, body.with_min_y(y + 1.0), *second, shell, viewer);
        }
        [] => {}
    }
    hide
}

/// A part's 30 px header: its title and quiet words after it, then at
/// the right its ⋯ menu, a count and its panel's own tools (the
/// Library's Place switch on Home). Returns the body under it, whether
/// the menu asked to hide the part, and whether Place was clicked.
fn header(
    ui: &mut Ui,
    part: &Placed,
    home: bool,
    viewer: &mut PanelViewer<'_>,
) -> (Rect, bool, bool) {
    let column = &part.column;
    let place = part.rect;
    let head = place.with_max_y(place.top() + size::COLUMN_HEADER_H);
    // Held to the header: a long note never runs into the centre.
    let painter = ui.painter().with_clip_rect(head.intersect(ui.clip_rect()));
    painter.rect_filled(head, 0.0, tokens::panel());
    tokens::hline(
        &painter,
        head.x_range(),
        head.bottom() - 1.0,
        tokens::line(),
    );
    // Named for screen readers (and tests), as a tab was before.
    let named = ui.interact(
        head,
        Id::new(("column header", column.title)),
        Sense::hover(),
    );
    named.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, column.title));
    let title = painter.text(
        pos2(head.left() + size::ROW_PAD, head.center().y),
        Align2::LEFT_CENTER,
        column.title,
        text::header(),
        tokens::text_emphasis(),
    );
    let first = column.panels.first().copied();
    let mut words_end = title.right();
    if let Some(note) = first.and_then(|p| viewer.column_note(p)) {
        let drawn = painter.text(
            pos2(title.right() + size::GAP, head.center().y),
            Align2::LEFT_CENTER,
            &note,
            text::ui(),
            tokens::text_meta(),
        );
        words_end = drawn.right();
        let named = ui.interact(
            drawn,
            Id::new(("column note", column.title)),
            Sense::hover(),
        );
        named.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, &note));
    }
    let tools = Rect::from_min_max(
        pos2(words_end + size::GAP, head.top()),
        pos2(head.right() - size::GAP, head.bottom() - 1.0),
    );
    let mut tools_ui = ui.new_child(
        UiBuilder::new()
            .id_salt(("column tools", column.title))
            .max_rect(tools)
            .layout(egui::Layout::right_to_left(egui::Align::Center)),
    );
    tools_ui.spacing_mut().item_spacing.x = 0.0;
    let hide = more_menu(&mut tools_ui, column, viewer);
    if let Some(count) = first.and_then(|p| viewer.column_count(p)) {
        tools_ui.add_space(size::GAP);
        tools_ui.add(
            egui::Label::new(
                egui::RichText::new(count)
                    .font(text::header())
                    .color(tokens::text_meta()),
            )
            .selectable(false),
        );
        tools_ui.add_space(size::GAP);
    }
    let mut placing = false;
    if home && part.slot == Slot::Library {
        placing = super::bottom::place_switch(&mut tools_ui, viewer.placing);
    }
    if let Some(panel) = first {
        viewer.column_tools(&mut tools_ui, panel);
    }
    (place.with_min_y(head.bottom()), hide, placing)
}

/// The ⋯ menu at a header's right: its panel's own items, then Hide.
/// Returns whether Hide was chosen.
fn more_menu(ui: &mut Ui, column: &Column, viewer: &mut PanelViewer<'_>) -> bool {
    let name = format!("More of {}", column.title);
    let more = widgets::button::icon(ui, Icon::More, &name)
        .on_hover_text(format!("{}'s menu, and Hide", column.title));
    let mut hide = false;
    egui::Popup::menu(&more).show(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        if let Some(panel) = column.panels.first()
            && viewer.column_menu(ui, *panel)
        {
            widgets::menu::line(ui);
        }
        let words = format!("Hide {}", column.title);
        if widgets::menu::item(ui, &words, "").clicked() {
            hide = true;
            ui.close();
        }
    });
    hide
}

/// Lets each of `borders` be dragged (the pointer showing which way it
/// moves, the line taking the accent), and put back to its standard by a
/// double-click. What a drag sets is kept for the tab in front, in
/// `layout`. The frame's accent edges carry a grip at their middle.
pub(super) fn drag_borders(ui: &Ui, borders: &[Border], shell: &mut Shell, layout: PanelLayout) {
    let tab = shell.tab;
    for border in borders {
        let upright = border.measure.upright();
        let reach = if upright {
            border.line.expand2(vec2(size::DIVIDER_REACH, 0.0))
        } else {
            border.line.expand2(vec2(0.0, size::DIVIDER_REACH))
        };
        let id = Id::new(("border", border.edge));
        let response = ui.interact(reach, id, Sense::click_and_drag());
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, border.name()));
        let held = response.hovered() || response.dragged();
        if held {
            ui.ctx().set_cursor_icon(if upright {
                CursorIcon::ResizeHorizontal
            } else {
                CursorIcon::ResizeVertical
            });
        }
        paint_edge(ui, border, held);
        if response.double_clicked() {
            shell.sizes.forget(tab, layout, border.edge);
        } else if response.dragged()
            && let Some(at) = response.interact_pointer_pos()
        {
            // Max then min: never a panic, even on a range that closed up.
            let value = border
                .measure
                .value(at)
                .max(*border.range.start())
                .min(*border.range.end());
            shell.sizes.set(tab, layout, border.edge, value);
        }
    }
}

/// An edge's line (the accent while `held`) and its grip, if it has one.
fn paint_edge(ui: &Ui, border: &Border, held: bool) {
    let painter = ui.painter();
    let accent = crate::interface_look::accent_fill();
    if held {
        let thick = if border.measure.upright() {
            border.line.expand2(vec2(0.5, 0.0))
        } else {
            border.line.expand2(vec2(0.0, 0.5))
        };
        painter.rect_filled(thick, 0.0, accent);
    } else {
        painter.rect_filled(border.line, 0.0, tokens::line());
    }
    if !border.grip {
        return;
    }
    let [long, thin] = size::EDGE_GRIP;
    let grip_size = if border.measure.upright() {
        vec2(thin, long)
    } else {
        vec2(long, thin)
    };
    let grip = Rect::from_center_size(border.line.center(), grip_size);
    let fill = if held {
        accent
    } else {
        tokens::stronger_line()
    };
    painter.rect_filled(grip, size::radius::GRIP, fill);
}
