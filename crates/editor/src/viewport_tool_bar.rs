//! The tool bar floating at the foot of the view in use (decision 0114;
//! the approved mock-ups `docs/design/tools-1/07-floating-tools-editing.png`
//! and `08-floating-tools-plain.png`): Select, Move, Rotate, Scale and
//! Transform in one rounded frame, each with its key small in a corner,
//! the tool in use filled with the accent. While a brush's insides are
//! edited, a line and then Points, Edges and Faces follow, the mode in
//! use lit.
//!
//! It is centred at the view's foot, moved left of the bottom-right
//! pills when they would meet, its buttons shrunk when the row is short;
//! where even that doesn't fit it goes above the pills, and in a view
//! narrower than itself its buttons shrink to fit (`place`). A tile
//! map's painting tools are on their own ribbon: while one is in use, no
//! transform tool is lit.

use super::{Tool, Viewport};
use crate::brushing::edit::Mode;
use crate::icons::{self, Icon};
use crate::interface_look;
use crate::tokens::{self, size, text};
use crate::widgets;
use egui::{Align2, CornerRadius, Rect, Response, Sense, Ui, UiBuilder, pos2, vec2};

/// Where the bar goes and how big its buttons are.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Placed {
    /// The bar's frame.
    pub(crate) frame: Rect,
    /// Each button's side.
    pub(crate) button: f32,
}

/// The bar's width with `buttons` buttons of side `side`, and the line
/// before the last `after` of them when there are any.
fn width(buttons: usize, side: f32, after: usize) -> f32 {
    let gaps = buttons.saturating_sub(1) as f32 * size::TOOL_BAR_GAP;
    let line = if after > 0 {
        size::TOOL_BAR_DIVIDER[0] - size::TOOL_BAR_GAP
    } else {
        0.0
    };
    buttons as f32 * side + gaps + line + 2.0 * size::TOOL_BAR_PAD
}

/// The side of `buttons` buttons (the last `after` of them after a
/// line) that fits `room` points across: the full side if it fits,
/// shrunk to fit if not, or `None` when even `least` doesn't.
fn fitting(room: f32, buttons: usize, after: usize, least: f32) -> Option<f32> {
    let full = size::TOOL_BAR_BUTTON;
    if width(buttons, full, after) <= room || buttons == 0 {
        return Some(full);
    }
    let side = (room - width(buttons, 0.0, after)) / buttons as f32;
    (side >= least).then_some(side)
}

/// Where the bar of `buttons` buttons (the last `after` of them after a
/// line) goes in `view`, clear of the bottom-right pills at `corner` and
/// of the left of the view: on the pills' row, centred and moved left of
/// the pills when they would meet, its buttons shrunk if the row is
/// short; else above the pills, if that is clear of the pills at the
/// view's top; else (a view too short for three rows) on the row, from
/// the left. In a view narrower than the bar its buttons shrink further,
/// to fit it.
pub(crate) fn place(view: Rect, corner: Option<Rect>, buttons: usize, after: usize) -> Placed {
    let inner = view.shrink2(vec2(size::PILL_MARGIN, 0.0));
    let foot = view.bottom() - size::TOOL_BAR_MARGIN;
    let beside = corner.map_or(inner.right(), |c| c.left() - size::PILL_GAP);
    let roof = view.top() + size::PILL_MARGIN + size::PILL_H + size::PILL_GAP;
    let at = |(left, right): (f32, f32), bottom: f32, button: f32| {
        let bar = vec2(
            width(buttons, button, after),
            button + 2.0 * size::TOOL_BAR_PAD,
        );
        let half = bar.x / 2.0;
        let x = if right - left < bar.x {
            left + half
        } else {
            view.center().x.clamp(left + half, right - half)
        };
        Placed {
            frame: Rect::from_min_size(pos2(x - half, bottom - bar.y), bar),
            button,
        }
    };
    let row = (inner.left(), beside);
    if let Some(button) = fitting(row.1 - row.0, buttons, after, size::TOOL_BAR_LEAST_BUTTON) {
        return at(row, foot, button);
    }
    let across = fitting(inner.width(), buttons, after, size::TOOL_BAR_LEAST_BUTTON)
        .or_else(|| {
            fitting(
                inner.width(),
                buttons,
                after,
                size::TOOL_BAR_SMALLEST_BUTTON,
            )
        })
        .unwrap_or(size::TOOL_BAR_SMALLEST_BUTTON);
    if let Some(pills) = corner {
        let above = at(
            (inner.left(), inner.right()),
            pills.top() - size::PILL_GAP,
            across,
        );
        if above.frame.top() >= roof {
            return above;
        }
    }
    at(row, foot, across)
}

/// What one button shows.
struct Look<'a> {
    icon: Icon,
    key: &'a str,
    hint: &'a str,
    lit: bool,
}

impl Viewport {
    /// Draws the tool bar in `view`, clear of the bottom-right pills at
    /// `corner`; a click picks its tool, or the brush's mode while one is
    /// edited. Returns where it is.
    pub(super) fn tool_bar(&mut self, ui: &mut Ui, view: Rect, corner: Option<Rect>) -> Rect {
        let editing = self.brushing.edit.as_ref().map(|edit| edit.mode);
        let after = if editing.is_some() {
            Mode::ALL.len()
        } else {
            0
        };
        let placed = place(view, corner, Tool::ALL.len() + after, after);
        let mut bar = ui.new_child(
            UiBuilder::new()
                .id_salt("viewport tool bar")
                .max_rect(placed.frame),
        );
        bar.set_clip_rect(view.intersect(ui.clip_rect()));
        frame(&bar, placed.frame);
        let side = placed.button;
        let mut x = placed.frame.left() + size::TOOL_BAR_PAD;
        let top = placed.frame.top() + size::TOOL_BAR_PAD;
        let spot = |x: &mut f32| {
            let rect = Rect::from_min_size(pos2(*x, top), vec2(side, side));
            *x += side + size::TOOL_BAR_GAP;
            rect
        };
        let painting = self.tile_overlay.paint.painting();
        for tool in Tool::ALL {
            let look = Look {
                icon: tool.icon(),
                key: tool.key().name(),
                hint: &tool.hint(),
                lit: !painting && tool == self.tool,
            };
            if button(&mut bar, spot(&mut x), &look).clicked() {
                self.tool = tool;
                self.tile_overlay.paint.tool = None;
            }
        }
        if let Some(now) = editing {
            let room = size::TOOL_BAR_DIVIDER[0];
            divider(&bar, x - size::TOOL_BAR_GAP, room, placed.frame.center().y);
            x += room - size::TOOL_BAR_GAP;
            for mode in Mode::ALL {
                let hint = format!("{} ({})", mode.name(), mode.key().name());
                let look = Look {
                    icon: mode.icon(),
                    key: mode.key().name(),
                    hint: &hint,
                    lit: mode == now,
                };
                if button(&mut bar, spot(&mut x), &look).clicked()
                    && let Some(edit) = self.brushing.edit.as_mut()
                {
                    edit.switch_to(mode);
                }
            }
        }
        placed.frame
    }
}

/// The bar's frame: the pills' fill and border.
fn frame(ui: &Ui, rect: Rect) {
    let radius = size::radius::CARD;
    ui.painter()
        .rect_filled(rect, CornerRadius::same(radius), tokens::pill_fill());
    widgets::border(ui, rect, radius, tokens::pill_border());
}

/// The short upright line between the tools and the brush's modes, in
/// the middle of `room` points from `left`.
fn divider(ui: &Ui, left: f32, room: f32, middle: f32) {
    let length = size::TOOL_BAR_DIVIDER[1];
    ui.painter().vline(
        left + room / 2.0,
        egui::Rangef::new(middle - length / 2.0, middle + length / 2.0),
        egui::Stroke::new(widgets::hairline(ui), tokens::stronger_line()),
    );
}

/// One button at `rect`: its icon in the middle, its key small in the
/// bottom-right corner, the accent filling it while lit (its icon dark),
/// lifted under the pointer; its name and key on hover, which is also
/// its name for screen readers and tests.
fn button(ui: &mut Ui, rect: Rect, look: &Look<'_>) -> Response {
    let id = ui.id().with(("tool bar button", look.hint));
    let response = ui
        .interact(rect, id, Sense::click())
        .on_hover_text(look.hint);
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Button, true, look.lit, look.hint)
    });
    if !ui.is_rect_visible(rect) {
        return response;
    }
    let radius = size::radius::TAB;
    let (fill, ink, key_ink) = if look.lit {
        (interface_look::accent_fill(), tokens::app(), tokens::app())
    } else if response.hovered() {
        (
            tokens::hover(),
            tokens::text_emphasis(),
            tokens::text_label(),
        )
    } else {
        (
            egui::Color32::TRANSPARENT,
            tokens::text_primary(),
            tokens::text_meta(),
        )
    };
    ui.painter()
        .rect_filled(rect, CornerRadius::same(radius), fill);
    // Shrunk in a narrow view, the icon keeps a little room round it,
    // and a button too small for its key leaves it to the hover text.
    let icon = size::TOOL_ICON.min(rect.width() - size::TOOL_BAR_ICON_ROOM);
    icons::icon(
        ui.painter(),
        Rect::from_center_size(rect.center(), vec2(icon, icon)),
        look.icon,
        ink,
    );
    if rect.width() >= size::TOOL_BAR_KEYED_BUTTON {
        widgets::words(
            ui,
            rect.shrink(size::TOOL_BAR_KEY_INSET),
            Align2::RIGHT_BOTTOM,
            look.key,
            text::ribbon_key(),
            key_ink,
        );
    }
    widgets::focus_ring(ui, &response, rect, radius);
    response
}

#[cfg(test)]
#[path = "viewport_tool_bar_tests.rs"]
mod tests;
