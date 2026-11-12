//! The toolbox: the viewport group's column of tools down its left edge
//! (spec 177; the approved additions, round 2, `03-toolbox.png`). Select,
//! Move, Rotate, Scale and Transform, then the design's four tools that
//! aren't built yet. A tile map's painting tools are on its own ribbon,
//! HOME TOOLS · Tile map (round 3, the project lead's answer 27); while
//! one is in use, no transform tool is lit.
//!
//! It belongs to the group, not to each view: with two or four views
//! there is still one toolbox. When the column is shorter than its tools
//! (a tall shelf), it scrolls.

use crate::icons::{self, Icon};
use crate::interface_look;
use crate::tile_paint::PaintSettings;
use crate::tokens::{self, size};
use crate::unbuilt::{self, Entry, UnbuiltId};
use crate::viewport::Tool;
use egui::{CornerRadius, Rect, Response, Sense, Ui, UiBuilder, vec2};

/// The design's tools that aren't built, below the divider, with their
/// icons; none while unfinished parts are hidden.
fn unbuilt_tools() -> Vec<(Icon, UnbuiltId)> {
    let tools = [
        (Icon::PoseBones, Entry::Unbuilt(UnbuiltId::HomeToolPose)),
        (
            Icon::MotionPath,
            Entry::Unbuilt(UnbuiltId::HomeToolMotionPath),
        ),
        (Icon::Key, Entry::Unbuilt(UnbuiltId::HomeToolAddKey)),
        (Icon::Measure, Entry::Unbuilt(UnbuiltId::HomeToolMeasure)),
    ];
    tools
        .into_iter()
        .filter_map(
            |(icon, entry): (Icon, Entry<()>)| match unbuilt::filter(vec![entry]).pop() {
                Some(Entry::Unbuilt(id)) => Some((icon, id)),
                _ => None,
            },
        )
        .collect()
}

/// Draws the toolbox in `rect`: `tool` is the tool in use, `paint` the
/// tile painter (no transform tool is lit while it paints). Returns the
/// tool clicked.
pub(crate) fn toolbox(ui: &mut Ui, rect: Rect, tool: Tool, paint: &PaintSettings) -> Option<Tool> {
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
            .id_salt("toolbox")
            .max_rect(rect)
            .layout(egui::Layout::top_down(egui::Align::Center)),
    );
    let mut picked = None;
    egui::ScrollArea::vertical()
        .id_salt("toolbox scroll")
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
        .auto_shrink(false)
        .show(&mut column, |ui| {
            ui.set_width(rect.width());
            ui.spacing_mut().item_spacing.y = size::TOOL_GAP;
            ui.add_space(size::TOOLBOX_PAD);
            let painting = paint.painting();
            for each in Tool::ALL {
                let chosen = !painting && each == tool;
                if tool_square(ui, each.icon(), chosen, &each.hint(), &each.hint()).clicked() {
                    picked = Some(each);
                }
            }
            let unbuilt = unbuilt_tools();
            if !unbuilt.is_empty() {
                divider(ui);
            }
            for (icon, id) in unbuilt {
                unbuilt::tool(ui, id, icon);
            }
            ui.add_space(size::TOOLBOX_PAD);
        });
    picked
}

/// A column of tools down the left of `rect` other than the viewport's
/// (the game-screen canvas's, round 3 `07-ui-editor-a.png`): each tool's
/// icon, whether it is in use, its hover text and its name. Returns the
/// index of the one clicked.
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

/// The short line between kinds of tool.
fn divider(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(
        vec2(size::TOOL_DIVIDER, size::TOOL_DIVIDER_GAP * 2.0 + 1.0),
        Sense::hover(),
    );
    ui.painter().hline(
        rect.x_range(),
        rect.center().y,
        egui::Stroke::new(crate::widgets::hairline(ui), tokens::field_border()),
    );
}
