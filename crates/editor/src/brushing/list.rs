//! The brush list (the approved mock-up `01-brush-list.png`): "SHAPE
//! BRUSHES" over a row for each shape, its icon and name, the one B has
//! moved to highlighted, and "B next · Space use · Esc close" at its
//! foot. It floats beside the pointer where it was opened, over
//! everything else.

use crate::tokens::{self, size, text};
use crate::widgets::{self, menu, words};
use egui::{Align2, CornerRadius, Pos2, Rect, Sense, Stroke, Ui, WidgetInfo, pos2, vec2};
use mesh::Shape;

/// The list while it is open.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct List {
    /// The highlighted row, counted in [`Shape::ALL`].
    chosen: usize,
    /// Where its top left corner is, on screen.
    pub(crate) at: Pos2,
    /// Opened this frame: the click that opened it (the Shape button's)
    /// doesn't count as a click elsewhere.
    pub(crate) fresh: bool,
}

/// What the person did with the list this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Answer {
    /// Took this shape (a click on its row).
    Take(Shape),
    /// Clicked somewhere else.
    Close,
}

impl List {
    /// The list opened beside `pointer`, its first shape highlighted.
    pub(crate) fn beside(pointer: Pos2) -> Self {
        Self {
            chosen: 0,
            at: pointer + vec2(size::PLACE_LABEL_OFFSET, size::PLACE_LABEL_OFFSET),
            fresh: true,
        }
    }

    /// The highlighted shape.
    pub(crate) fn chosen(&self) -> Shape {
        Shape::ALL[self.chosen % Shape::ALL.len()]
    }

    /// B again: the highlight moves down one, from the last back to the
    /// first.
    pub(crate) fn next(&mut self) {
        self.chosen = (self.chosen + 1) % Shape::ALL.len();
    }
}

/// The keys at the list's foot, each with what it does.
const FOOT: [(&str, &str); 3] = [("B", "next"), ("Space", "use"), ("Esc", "close")];

/// Draws the list over everything, where it was opened (kept on screen).
/// Returns what was done with it: a row clicked, or a click elsewhere.
pub(crate) fn show(ctx: &egui::Context, list: &mut List) -> Option<Answer> {
    let mut answer = None;
    let shown = egui::Area::new(egui::Id::new("brush list"))
        .order(egui::Order::Foreground)
        .fixed_pos(list.at)
        .constrain(true)
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(tokens::card())
                .stroke(Stroke::new(widgets::hairline(ui), tokens::strong_line()))
                .corner_radius(CornerRadius::same(size::radius::CARD))
                .inner_margin(egui::Margin::same(size::MENU_PAD as i8))
                .show(ui, |ui| {
                    ui.set_width(size::BRUSH_LIST_W - 2.0 * size::MENU_PAD);
                    ui.spacing_mut().item_spacing.y = 0.0;
                    menu::caps(ui, "Shape brushes");
                    for (n, shape) in Shape::ALL.into_iter().enumerate() {
                        if row(ui, shape, n == list.chosen).clicked() {
                            answer = Some(Answer::Take(shape));
                        }
                    }
                    menu::line(ui);
                    foot(ui);
                });
        });
    let inside = shown.response.rect;
    let pressed_outside = ctx.input(|i| {
        i.pointer.any_pressed() && i.pointer.interact_pos().is_none_or(|p| !inside.contains(p))
    });
    if pressed_outside && !list.fresh && answer.is_none() {
        answer = Some(Answer::Close);
    }
    list.fresh = false;
    answer
}

/// One shape's row: its icon and name, highlighted while B has moved to
/// it, lifted under the pointer.
fn row(ui: &mut Ui, shape: Shape, chosen: bool) -> egui::Response {
    let width = ui.available_width();
    let (rect, response) =
        ui.allocate_exact_size(vec2(width, size::BRUSH_LIST_ROW_H), Sense::click());
    response.widget_info(|| {
        WidgetInfo::selected(egui::WidgetType::RadioButton, true, chosen, shape.name())
    });
    let radius = CornerRadius::same(size::radius::CONTROL);
    let accent = crate::interface_look::in_use().accent.colour();
    let ink = if chosen {
        ui.painter().rect_filled(
            rect,
            radius,
            tokens::mix(accent, tokens::card(), tokens::share::SELECTED_ROW),
        );
        ui.painter().rect_stroke(
            rect,
            radius,
            Stroke::new(
                widgets::hairline(ui),
                tokens::mix(accent, tokens::card(), tokens::share::OPEN_CARD),
            ),
            egui::StrokeKind::Inside,
        );
        tokens::text_emphasis()
    } else {
        if response.hovered() {
            ui.painter().rect_filled(rect, radius, tokens::hover());
        }
        tokens::text_body()
    };
    let [icon_side, icon_gap] = size::BRUSH_LIST_ICON;
    let icon = Rect::from_center_size(
        pos2(
            rect.left() + size::BRUSH_LIST_PAD + icon_side / 2.0,
            rect.center().y,
        ),
        vec2(icon_side, icon_side),
    );
    let icon_ink = if chosen { ink } else { tokens::text_label() };
    crate::icons::icon(ui.painter(), icon, super::icon(shape), icon_ink);
    let room = rect.with_min_x(icon.right() + icon_gap);
    words(
        ui,
        room,
        Align2::LEFT_CENTER,
        shape.name(),
        text::menu_item(),
        ink,
    );
    response
}

/// The foot: each key as a keycap, what it does after it, a dot between.
fn foot(ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.set_height(size::BRUSH_LIST_ROW_H);
        ui.spacing_mut().item_spacing.x = size::GAP;
        ui.add_space(size::MENU_PAD);
        for (n, (key, what)) in FOOT.into_iter().enumerate() {
            if n > 0 {
                ui.label(
                    egui::RichText::new("\u{b7}")
                        .font(text::ui())
                        .color(tokens::text_separator()),
                );
            }
            widgets::keycap::keycap(ui, key, widgets::keycap::KeycapState::Bound);
            ui.label(
                egui::RichText::new(what)
                    .font(text::ui())
                    .color(tokens::text_label()),
            );
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn b_moves_down_the_list_and_wraps_from_the_last_to_the_first() {
        let mut list = List::beside(Pos2::ZERO);
        assert_eq!(list.chosen(), Shape::Box, "it opens on the first");
        let mut seen = vec![list.chosen()];
        for _ in 1..Shape::ALL.len() {
            list.next();
            seen.push(list.chosen());
        }
        assert_eq!(seen, Shape::ALL, "the mock-up's order");
        list.next();
        assert_eq!(list.chosen(), Shape::Box, "after Wedge, Box again");
    }

    #[test]
    fn it_opens_beside_the_pointer() {
        let list = List::beside(pos2(100.0, 50.0));
        assert!(list.at.x > 100.0 && list.at.y > 50.0);
        assert!(list.fresh);
    }
}
