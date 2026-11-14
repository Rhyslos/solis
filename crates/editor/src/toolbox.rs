//! A toolbox: a column of tools down the left of an editor's view, as
//! the UI canvas has (round 3, `07-ui-editor-a.png`). The area's own
//! tools float at the foot of the view instead (`viewport_tool_bar.rs`,
//! decision 0114).
//!
//! When the column is shorter than its tools (a tall shelf), it scrolls.

use crate::icons::{self, Icon};
use crate::interface_look;
use crate::tokens::{self, size};
use egui::{CornerRadius, Rect, Response, Sense, Ui, UiBuilder, vec2};

/// A column of tools down the left of `rect` (the game-screen canvas's,
/// round 3 `07-ui-editor-a.png`): each tool's icon, whether it is in
/// use, its hover text and its name. Returns the index of the one
/// clicked.
pub(crate) fn tools_column(
    ui: &mut Ui,
    rect: Rect,
    tools: &[(Icon, bool, String, &str)],
) -> Option<usize> {
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, tokens::shelf_bar());
    let line = crate::widgets::hairline(ui);
    painter.vline(
        rect.right() - line / 2.0,
        rect.y_range(),
        egui::Stroke::new(line, tokens::line()),
    );
    let mut column = ui.new_child(
        UiBuilder::new()
            .id_salt("tools column")
            .max_rect(rect)
            .layout(egui::Layout::top_down(egui::Align::Center)),
    );
    let mut picked = None;
    egui::ScrollArea::vertical()
        .id_salt("tools column scroll")
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
        .auto_shrink(false)
        .show(&mut column, |ui| {
            ui.set_width(rect.width());
            ui.spacing_mut().item_spacing.y = size::TOOL_GAP;
            ui.add_space(size::TOOLBOX_PAD);
            for (index, (icon, chosen, hint, name)) in tools.iter().enumerate() {
                if tool_square(ui, *icon, *chosen, hint, name).clicked() {
                    picked = Some(index);
                }
            }
            ui.add_space(size::TOOLBOX_PAD);
        });
    picked
}

/// One tool: a 34 px square, the accent filling it while in use, lifted
/// under the pointer; `hint` (its name and key) on hover, `label` for
/// screen readers and tests.
fn tool_square(ui: &mut Ui, icon: Icon, chosen: bool, hint: &str, label: &str) -> Response {
    let (rect, response) =
        ui.allocate_exact_size(vec2(size::TOOL_BUTTON, size::TOOL_BUTTON), Sense::click());
    let response = response.on_hover_text(hint);
    response
        .widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, true, chosen, label));
    if ui.is_rect_visible(rect) {
        let radius = CornerRadius::same(size::radius::MENU);
        let (fill, ink) = if chosen {
            (interface_look::accent_fill(), tokens::app())
        } else if response.hovered() {
            (tokens::hover(), tokens::text_primary())
        } else {
            (egui::Color32::TRANSPARENT, tokens::text_label())
        };
        ui.painter().rect_filled(rect, radius, fill);
        icons::icon(
            ui.painter(),
            Rect::from_center_size(rect.center(), egui::Vec2::splat(size::TOOL_ICON)),
            icon,
            ink,
        );
        crate::widgets::focus_ring(ui, &response, rect, size::radius::MENU);
    }
    response
}
