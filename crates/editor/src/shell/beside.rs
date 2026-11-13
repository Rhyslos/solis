//! Home's lists beside the editor groups in a side layout (decision
//! 0109): the Area, Properties and Assets lists in their columns and the
//! strip, each with its header, and the borders between everything in
//! the middle, which a drag moves and a double-click puts back.

use super::Shell;
use super::layouts::{Border, PanelLayout, Side};
use crate::panels::PanelViewer;
use crate::tokens;
use crate::tokens::size;
use egui::{CursorIcon, Id, Rect, Sense, Ui, WidgetInfo, WidgetType};

/// Draws the lists `place` puts in `upper` (the middle above the shelf),
/// and lets their borders be dragged.
pub(super) fn lists(
    ui: &mut Ui,
    upper: Rect,
    place: &Side,
    shell: &mut Shell,
    layout: PanelLayout,
    viewer: &mut PanelViewer<'_>,
) {
    // The 1 px lines between the columns, the center and the strip.
    let painter = ui.painter();
    for border in &place.borders {
        painter.rect_filled(border.line.intersect(upper), 0.0, tokens::line());
    }
    for placed in &place.lists {
        super::shelf::column(ui, placed.rect, &placed.column, true, shell, viewer);
    }
    drag_borders(ui, &place.borders, shell, layout);
}

/// Lets each of `borders` be dragged (the pointer showing which way it
/// moves), and put back to its standard by a double-click. What a drag
/// sets is kept for the tab in front, in `layout`.
pub(super) fn drag_borders(ui: &Ui, borders: &[Border], shell: &mut Shell, layout: PanelLayout) {
    let tab = shell.tab;
    for border in borders {
        let reach = if border.measure.upright() {
            border.line.expand2(egui::vec2(size::DIVIDER_REACH, 0.0))
        } else {
            border.line.expand2(egui::vec2(0.0, size::DIVIDER_REACH))
        };
        let id = Id::new(("border", border.edge));
        let response = ui.interact(reach, id, Sense::click_and_drag());
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, border.name()));
        if response.hovered() || response.dragged() {
            ui.ctx().set_cursor_icon(if border.measure.upright() {
                CursorIcon::ResizeHorizontal
            } else {
                CursorIcon::ResizeVertical
            });
        }
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
