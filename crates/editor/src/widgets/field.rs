//! The design's fields: a dark box 24 tall with a thin border that turns
//! to the accent while the keyboard is in it. Three kinds: words, a
//! number (with an axis letter before it if it is one of X, Y and Z),
//! and a drop-down list.
//!
//! The editing inside is egui's own (its text box, its number that can
//! be dragged), drawn without its frame; the frame, the letter and the
//! list's arrow are drawn here.

use super::{arrow_step, focus_ring, words};
use crate::interface_look;
use crate::tokens::{self, size, text};
use egui::{
    Align, Align2, Color32, CornerRadius, Frame, Layout, Margin, Rect, Response, RichText, Sense,
    Stroke, Ui, UiBuilder, WidgetInfo, pos2, vec2,
};

/// An axis, for the small letter before a number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    /// Left and right: red.
    X,
    /// Forward and back: green.
    Y,
    /// Up and down: blue.
    Z,
}

impl Axis {
    /// X, Y and Z, in order.
    pub const ALL: [Self; 3] = [Self::X, Self::Y, Self::Z];

    fn letter(self) -> &'static str {
        match self {
            Self::X => "X",
            Self::Y => "Y",
            Self::Z => "Z",
        }
    }

    fn colour(self) -> Color32 {
        match self {
            Self::X => tokens::AXIS_X_TEXT,
            Self::Y => tokens::AXIS_Y_TEXT,
            Self::Z => tokens::AXIS_Z_TEXT,
        }
    }
}

/// A field's box in `rect`: its fill, and its border, in the accent
/// while `focused`.
fn box_shape(ui: &Ui, rect: Rect, focused: bool) -> egui::epaint::RectShape {
    let edge = if focused {
        interface_look::accent_fill()
    } else {
        tokens::field_border()
    };
    egui::epaint::RectShape::new(
        rect,
        CornerRadius::same(size::radius::CONTROL),
        tokens::field(),
        Stroke::new(super::hairline(ui), edge),
        egui::StrokeKind::Inside,
    )
}

/// A field `width` wide around what `inside` draws, which says whether
/// the keyboard is in it. The box is painted after its contents are laid
/// out (its border depends on them) but under them.
fn boxed<R>(ui: &mut Ui, width: f32, inside: impl FnOnce(&mut Ui) -> (R, bool)) -> (Rect, R) {
    boxed_at(ui, vec2(width, size::field_h()), inside)
}

/// A field's box of any `wanted` size (a page's search is taller than a
/// row's field), as [`boxed`].
fn boxed_at<R>(
    ui: &mut Ui,
    wanted: egui::Vec2,
    inside: impl FnOnce(&mut Ui) -> (R, bool),
) -> (Rect, R) {
    let (rect, _) = ui.allocate_exact_size(wanted, Sense::hover());
    let under = ui.painter().add(egui::Shape::Noop);
    let room = rect.shrink2(vec2(size::FIELD_PAD, 0.0));
    let mut child = ui.new_child(
        UiBuilder::new()
            .max_rect(room)
            .layout(Layout::left_to_right(Align::Center)),
    );
    child.spacing_mut().item_spacing.x = size::GAP;
    // egui's own editing, without egui's own frame.
    let visuals = child.visuals_mut();
    visuals.selection.stroke = Stroke::NONE;
    for state in [
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.noninteractive,
    ] {
        state.bg_stroke = Stroke::NONE;
        state.bg_fill = Color32::TRANSPARENT;
        state.weak_bg_fill = Color32::TRANSPARENT;
    }
    let (answer, focused) = inside(&mut child);
    ui.painter().set(under, box_shape(ui, rect, focused));
    (rect, answer)
}

/// A field for words, `width` wide, showing `hint` quietly while empty.
/// `name` is what it holds: a screen reader says it, and tests find the
/// field by it.
pub fn text_field(ui: &mut Ui, name: &str, value: &mut String, hint: &str, width: f32) -> Response {
    let (_, response) = boxed(ui, width, |ui| {
        let response = ui.add(
            egui::TextEdit::singleline(value)
                .frame(Frame::NONE)
                .margin(Margin::ZERO)
                .font(text::ribbon_small())
                .text_color(tokens::text_primary())
                .hint_text(RichText::new(hint).color(tokens::text_meta()))
                .desired_width(f32::INFINITY),
        );
        named(
            ui,
            &response,
            egui::WidgetType::TextEdit,
            name,
            value.as_str(),
        );
        let focused = response.has_focus();
        (response, focused)
    });
    response
}

/// A page's search: a magnifying glass, then a field for words, `width`
/// wide and as tall as the page's header buttons, showing `hint` while
/// empty. `name` is what a screen reader says.
pub fn search_field(
    ui: &mut Ui,
    name: &str,
    value: &mut String,
    hint: &str,
    width: f32,
) -> Response {
    let wanted = vec2(width, size::PAGE_BUTTON_H + 2.0);
    let (_, response) = boxed_at(ui, wanted, |ui| {
        let (spot, _) =
            ui.allocate_exact_size(vec2(size::ICON_SMALL, size::ICON_SMALL), Sense::hover());
        crate::icons::icon(
            ui.painter(),
            spot,
            crate::icons::Icon::Search,
            tokens::text_meta(),
        );
        let response = ui.add(
            egui::TextEdit::singleline(value)
                .frame(Frame::NONE)
                .margin(Margin::ZERO)
                .font(text::page_subtitle())
                .text_color(tokens::text_primary())
                .hint_text(RichText::new(hint).color(tokens::text_meta()))
                .desired_width(f32::INFINITY),
        );
        named(
            ui,
            &response,
            egui::WidgetType::TextEdit,
            name,
            value.as_str(),
        );
        let focused = response.has_focus();
        (response, focused)
    });
    response
}

/// Gives a field egui edits its name, for screen readers and tests, with
/// what it holds as its value.
fn named(ui: &Ui, response: &Response, kind: egui::WidgetType, name: &str, value: &str) {
    response.widget_info(|| {
        let mut info = WidgetInfo::labeled(kind, ui.is_enabled(), name);
        info.current_text_value = Some(value.to_owned());
        info
    });
}

/// The design's field box, `width` wide, round whatever `add` puts in
/// it (egui's own editing, drawn without its frame), with `axis`'s
/// letter first if it is one. The border turns to the accent while the
/// keyboard is in it or it is being dragged. Numbers are in the mono
/// font. Returns what `add` returned.
pub fn framed(
    ui: &mut Ui,
    width: f32,
    axis: Option<Axis>,
    add: impl FnOnce(&mut Ui) -> Response,
) -> Response {
    let (_, response) = boxed(ui, width, |ui| {
        if let Some(axis) = axis {
            ui.add(
                egui::Label::new(
                    RichText::new(axis.letter())
                        .font(text::mono_bold())
                        .color(axis.colour()),
                )
                .selectable(false),
            );
        }
        ui.style_mut().override_font_id = Some(text::mono());
        ui.visuals_mut().override_text_color = Some(tokens::text_primary());
        ui.spacing_mut().button_padding = vec2(0.0, 0.0);
        ui.spacing_mut().interact_size = vec2(0.0, 0.0);
        let response = add(ui);
        let focused = response.has_focus() || response.dragged();
        (response, focused)
    });
    response
}

/// A field for a number, `width` wide, in the mono font, with `axis`'s
/// letter before it. The number is typed, or dragged left and right;
/// `speed` is how much a point of dragging changes it. `name` is what
/// the number is, for screen readers and tests.
pub fn number_field(
    ui: &mut Ui,
    name: &str,
    value: &mut f64,
    axis: Option<Axis>,
    speed: f64,
    width: f32,
) -> Response {
    framed(ui, width, axis, |ui| {
        let response = ui.add(egui::DragValue::new(value).speed(speed));
        named(
            ui,
            &response,
            egui::WidgetType::DragValue,
            name,
            &value.to_string(),
        );
        response
    })
}

/// A field for a whole number, as [`number_field`] with no decimals
/// shown: a size in pixels, a count.
pub fn whole_number_field(ui: &mut Ui, name: &str, value: &mut f64, width: f32) -> Response {
    framed(ui, width, None, |ui| {
        let response = ui.add(egui::DragValue::new(value).speed(0.25).fixed_decimals(0));
        named(
            ui,
            &response,
            egui::WidgetType::DragValue,
            name,
            &value.to_string(),
        );
        response
    })
}

/// A drop-down field `width` wide showing `chosen`'s word, with the
/// list's small arrow at its right. A click, Space or Enter opens the
/// list of `choices`; with the list closed, the arrow keys step through
/// them. `name` tells this field from others and is what a screen reader
/// says. Returns whether another choice was made.
pub fn dropdown<T: Copy + PartialEq>(
    ui: &mut Ui,
    name: &str,
    chosen: &mut T,
    choices: &[(T, &str)],
    width: f32,
) -> bool {
    let shown = choices
        .iter()
        .find(|(value, _)| value == chosen)
        .map_or("", |(_, word)| word);
    let list = List {
        shown,
        note: None,
        width,
    };
    dropdown_showing(ui, name, chosen, choices, list)
}

/// What a [`dropdown_showing`] shows besides its choices.
#[derive(Debug, Clone, Copy)]
pub struct List<'a> {
    /// What the closed box says. It may be none of the choices: a dash
    /// for values that differ, a name that is no longer there.
    pub shown: &'a str,
    /// Quiet words ending the open list, if any: how to make the first
    /// choice when there are none yet.
    pub note: Option<&'a str>,
    /// How wide the field is.
    pub width: f32,
}

/// The longest an open list grows before it scrolls.
pub const LIST_MAX_H: f32 = 320.0;

/// A [`dropdown`] whose box says what `list` says, which need not be one
/// of the `choices` (`chosen` may be none of them too: then the arrow keys
/// don't step, and any row picked is another choice). A long list scrolls.
pub fn dropdown_showing<T: Copy + PartialEq>(
    ui: &mut Ui,
    name: &str,
    chosen: &mut T,
    choices: &[(T, &str)],
    list: List<'_>,
) -> bool {
    let before = *chosen;
    let response = dropdown_button(ui, name, list.shown, list.width);
    let open = egui::Popup::is_id_open(ui.ctx(), egui::Popup::default_response_id(&response));
    if !open {
        let step = arrow_step(ui, &response);
        let at = choices.iter().position(|(value, _)| value == chosen);
        if let (Some(at), true) = (at, step != 0) {
            let next = (at as i32 + step).clamp(0, choices.len() as i32 - 1);
            *chosen = choices[next as usize].0;
        }
    }
    egui::Popup::menu(&response).width(list.width).show(|ui| {
        egui::ScrollArea::vertical()
            .max_height(LIST_MAX_H)
            .show(ui, |ui| {
                // The rows of a list touch.
                ui.spacing_mut().item_spacing.y = 0.0;
                for (value, word) in choices {
                    if list_row(ui, word, value == chosen).clicked() {
                        *chosen = *value;
                    }
                }
                if let Some(note) = list.note {
                    list_note(ui, note);
                }
            });
    });
    *chosen != before
}

/// Quiet words in an open list, saying something rather than offering
/// it: why it is empty, or that it shows only some.
pub fn list_note(ui: &mut Ui, note: &str) {
    ui.add(
        egui::Label::new(
            RichText::new(note)
                .font(text::row())
                .color(tokens::text_label()),
        )
        .wrap(),
    );
}

/// The closed drop-down: the field's box, `shown` in it, and the arrow.
/// Its list is opened from the response (`egui::Popup::menu`).
pub fn dropdown_button(ui: &mut Ui, name: &str, shown: &str, width: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(width, size::field_h()), Sense::click());
    response.widget_info(|| {
        let mut info = WidgetInfo::labeled(egui::WidgetType::ComboBox, ui.is_enabled(), name);
        info.current_text_value = Some(shown.to_owned());
        info
    });
    if ui.is_rect_visible(rect) {
        ui.painter().add(box_shape(ui, rect, false));
        let room = rect.shrink2(vec2(size::FIELD_PAD, 0.0));
        let arrow = Rect::from_min_max(pos2(room.right() - DROP_ARROW, room.top()), room.max);
        let text_room = Rect::from_min_max(room.min, pos2(arrow.left() - size::GAP, room.bottom()));
        words(
            ui,
            text_room,
            Align2::LEFT_CENTER,
            shown,
            text::ribbon_small(),
            tokens::text_primary(),
        );
        // The list's arrow: a small filled triangle, as the design's
        // pictures show it, drawn rather than taken from a font.
        let middle = arrow.center();
        let half = DROP_ARROW / 2.0;
        ui.painter().add(egui::Shape::convex_polygon(
            vec![
                pos2(middle.x - half, middle.y - half / 2.0),
                pos2(middle.x + half, middle.y - half / 2.0),
                pos2(middle.x, middle.y + half / 2.0),
            ],
            tokens::text_meta(),
            Stroke::NONE,
        ));
        focus_ring(ui, &response, rect, size::radius::CONTROL);
    }
    response
}

/// How wide the list's arrow is.
const DROP_ARROW: f32 = 6.0;

/// One row of an open list: 24 tall, lifted under the pointer, bright
/// when it is the choice in force. For a list [`dropdown`] can't make
/// (one with a box to find a row by name).
pub fn list_row(ui: &mut Ui, word: &str, chosen: bool) -> Response {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(vec2(width, size::MENU_ROW_H), Sense::click());
    response.widget_info(|| {
        WidgetInfo::selected(egui::WidgetType::RadioButton, ui.is_enabled(), chosen, word)
    });
    if ui.is_rect_visible(rect) {
        if response.hovered() || response.has_focus() {
            ui.painter().rect_filled(
                rect,
                CornerRadius::same(size::radius::SMALL),
                tokens::hover(),
            );
        }
        let ink = if chosen {
            tokens::text_emphasis()
        } else {
            tokens::text_body()
        };
        let room = rect.shrink2(vec2(size::FIELD_PAD, 0.0));
        words(ui, room, Align2::LEFT_CENTER, word, text::row(), ink);
    }
    response
}
