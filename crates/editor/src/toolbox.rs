//! The toolbox: the viewport group's column of tools down its left edge
//! (spec 177; the approved additions, round 2, `03-toolbox.png`). Select,
//! Move, Rotate, Scale and Transform; while a tile map is selected, its
//! painting tools under a divider; then the design's four tools that
//! aren't built yet.
//!
//! It belongs to the group, not to each view: with two or four views
//! there is still one toolbox. When the column is shorter than its tools
//! (a tall shelf), it scrolls.

use crate::icons::{self, Icon};
use crate::interface_look;
use crate::tile_paint::{PaintSettings, PaintTool};
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

/// What was clicked in the toolbox.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Picked {
    /// A transform tool.
    Tool(Tool),
    /// A tile map's painting tool.
    Paint(PaintTool),
}

/// Draws the toolbox in `rect`: `tool` is the tool in use, `paint` the
/// selected map's painting tools (shown while one is selected). Returns
/// the tool clicked.
pub(crate) fn toolbox(
    ui: &mut Ui,
    rect: Rect,
    tool: Tool,
    paint: &PaintSettings,
) -> Option<Picked> {
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
                    picked = Some(Picked::Tool(each));
                }
            }
            if paint.allowed {
                divider(ui);
                for each in PaintTool::ALL {
                    let chosen = paint.tool == Some(each);
                    if tool_square(ui, each.icon(), chosen, &each.hint(), each.name()).clicked() {
                        picked = Some(Picked::Paint(each));
                    }
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
