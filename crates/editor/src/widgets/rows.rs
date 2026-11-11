//! The design's rows: a setting with its control, a name beside its
//! value, the small capitals over a section, and a column's header.

use crate::tokens::{self, size, text};
use egui::{Align, Layout, Rangef, Rect, RichText, Sense, Ui, UiBuilder, pos2, vec2};

/// Quiet words that can't be selected.
fn plain(ui: &mut Ui, words: RichText) -> egui::Response {
    ui.add(egui::Label::new(words).selectable(false))
}

/// The line under a row, a section or a header: one sharp pixel.
fn underline(ui: &Ui, across: Rangef, y: f32, colour: egui::Color32) {
    tokens::hline(ui.painter(), across, y, colour);
}

/// A setting's name and the line that explains it, on the left of a
/// setting row. `ink` is the name's colour.
pub(crate) fn setting_words(ui: &mut Ui, name: &str, hint: &str, ink: egui::Color32) {
    ui.spacing_mut().item_spacing.y = size::SETTING_HINT_GAP;
    plain(
        ui,
        RichText::new(name).font(text::setting_name()).color(ink),
    );
    if !hint.is_empty() {
        ui.add(
            egui::Label::new(
                RichText::new(hint)
                    .font(text::setting_hint())
                    .line_height(Some(text::SETTING_HINT_LINE_H))
                    .color(tokens::text_meta()),
            )
            .selectable(false)
            .wrap(),
        );
    }
}

/// One setting: its `name` and `hint` on the left, and on the right, in
/// a column 260 wide, the control that `control` draws (against the
/// right edge). A hairline closes the row. Returns what `control` does.
pub fn setting_row<R>(
    ui: &mut Ui,
    name: &str,
    hint: &str,
    control: impl FnOnce(&mut Ui) -> R,
) -> R {
    setting_row_with(
        ui,
        |ui| setting_words(ui, name, hint, tokens::text_primary()),
        control,
    )
}

/// A setting row whose left side is drawn by `words` (a row that isn't
/// built yet draws its name its own way).
///
/// The row is one piece of the layout: 12 above its words, 12 below,
/// and its hairline. Rows follow one another with whatever gap the list
/// they are in has, which for a list of settings is none.
pub(crate) fn setting_row_with<R>(
    ui: &mut Ui,
    words: impl FnOnce(&mut Ui),
    control: impl FnOnce(&mut Ui) -> R,
) -> R {
    let across = ui.available_rect_before_wrap().x_range();
    let top = ui.cursor().top();
    let inside_top = top + size::SETTING_ROW_PAD;
    let words_w = (across.span() - size::SETTING_CONTROL_W - size::SETTING_COLUMN_GAP)
        .max(size::SETTING_WORDS_MIN_W);
    let left = Rect::from_min_size(pos2(across.min, inside_top), vec2(words_w, 0.0));
    let mut words_ui = ui.new_child(
        UiBuilder::new()
            .max_rect(left)
            .layout(Layout::top_down(Align::Min)),
    );
    words(&mut words_ui);
    let height = words_ui.min_rect().height().max(size::SETTING_ROW_MIN_H);
    // The control sits against the right edge, level with the middle of
    // the words.
    let right = Rect::from_min_size(
        pos2(across.max - size::SETTING_CONTROL_W, inside_top),
        vec2(size::SETTING_CONTROL_W, height),
    );
    let mut control_ui = ui.new_child(
        UiBuilder::new()
            .max_rect(right)
            .layout(Layout::right_to_left(Align::Center)),
    );
    let answer = control(&mut control_ui);
    let row = Rect::from_min_size(
        pos2(across.min, top),
        vec2(
            across.span(),
            height + 2.0 * size::SETTING_ROW_PAD + size::LINE,
        ),
    );
    ui.allocate_rect(row, Sense::hover());
    underline(ui, across, row.bottom() - size::LINE, tokens::row_hover());
    answer
}

/// A name and its value on one line: the name quiet, in a column 96
/// wide; the value bright, cut off with "…" where it doesn't fit.
pub fn key_value(ui: &mut Ui, key: &str, value: &str) {
    let across = ui.available_rect_before_wrap().x_range();
    let (line, _) = ui.allocate_exact_size(vec2(across.span(), KEY_VALUE_H), Sense::hover());
    let (name, rest) = line.split_left_right_at_x(line.left() + size::KEY_COLUMN_W);
    let rest = rest.with_min_x(rest.left() + size::KEY_VALUE_GAP);
    for (rect, words, ink) in [
        (name, key, tokens::text_meta()),
        (rest, value, tokens::text_primary()),
    ] {
        let mut part = ui.new_child(
            UiBuilder::new()
                .max_rect(rect)
                .layout(Layout::left_to_right(Align::Center)),
        );
        let words = RichText::new(words).font(text::row()).color(ink);
        part.add(egui::Label::new(words).selectable(false).truncate());
    }
}

/// The height of a line of a name and its value.
const KEY_VALUE_H: f32 = 15.0;

/// The small capitals over a group of settings or a table: "THEME".
pub fn section_caps(ui: &mut Ui, words: &str) -> egui::Response {
    ui.add(
        egui::Label::new(text::caps(words, text::SECTION_CAPS, tokens::text_meta()))
            .selectable(false),
    )
}

/// A column's header: its `title` in a 30 px strip with a hairline
/// under it, and whatever `tools` adds at the right.
pub fn column_header(ui: &mut Ui, title: &str, tools: impl FnOnce(&mut Ui)) {
    let across = ui.available_rect_before_wrap().x_range();
    let (rect, _) =
        ui.allocate_exact_size(vec2(across.span(), size::COLUMN_HEADER_H), Sense::hover());
    let inside = rect.shrink2(vec2(size::ROW_PAD, 0.0));
    let mut strip = ui.new_child(
        UiBuilder::new()
            .max_rect(inside)
            .layout(Layout::left_to_right(Align::Center)),
    );
    strip.spacing_mut().item_spacing.x = size::CARD_INNER_GAP;
    plain(
        &mut strip,
        RichText::new(title)
            .font(text::header())
            .color(tokens::text_body()),
    );
    strip.with_layout(Layout::right_to_left(Align::Center), tools);
    underline(ui, across, rect.bottom() - size::LINE, tokens::line());
}
