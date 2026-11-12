//! What the File pages share: a page's heading, plain words, a setting
//! row whose control is a button, section capitals, and the layout of
//! settings beside a column of cards.

use crate::icons::Icon;
use crate::tokens::{self, size, text};
use crate::widgets::{self, button::Button};
use egui::{Align, Color32, Layout, RichText, Ui, UiBuilder, vec2};

/// A page's heading: its title and the line under it on the left, and
/// what `tools` adds at the right (searches and buttons), level with it.
pub(super) fn page_head(ui: &mut Ui, title: &str, subtitle: &str, tools: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = size::GAP;
            plain(ui, title, text::page_title(), tokens::text_emphasis());
            plain(ui, subtitle, text::page_subtitle(), tokens::text_meta());
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = size::GAP + size::GAP;
            tools(ui);
        });
    });
}

/// Words that can't be selected.
pub(super) fn plain(ui: &mut Ui, words: &str, font: egui::FontId, ink: Color32) -> egui::Response {
    ui.add(egui::Label::new(RichText::new(words).font(font).color(ink)).selectable(false))
}

/// Words that wrap, quiet: a card's or a section's sentence.
pub(super) fn sentence(ui: &mut Ui, words: &str) {
    ui.add(
        egui::Label::new(
            RichText::new(words)
                .font(text::setting_hint())
                .line_height(Some(text::SETTING_HINT_LINE_H))
                .color(tokens::text_meta()),
        )
        .selectable(false)
        .wrap(),
    );
}

/// A setting row whose control is a button: "Open the manual ↗". Returns
/// whether it was clicked. `outside` marks a button that leaves Solis
/// (the browser, another program).
pub(super) fn button_row(
    ui: &mut Ui,
    name: &str,
    hint: &str,
    label: &str,
    outside: bool,
) -> egui::Response {
    widgets::rows::setting_row(ui, name, hint, |ui| {
        let button = Button::plain(label);
        if outside {
            button.icon_after(Icon::External).show(ui)
        } else {
            button.show(ui)
        }
    })
}

/// Section capitals with the gap above them.
pub(super) fn section(ui: &mut Ui, words: &str) {
    ui.add_space(size::SECTION_GAP);
    widgets::rows::section_caps(ui, words);
}

/// Lays a page out as its settings on the left (no wider than
/// `size::SETTINGS_MAX_W`) and a column of cards on the right, as the
/// round-2 pictures do. On a narrow window the column goes under them.
pub(super) fn with_side<S>(
    ui: &mut Ui,
    state: &mut S,
    main: impl FnOnce(&mut Ui, &mut S),
    side: impl FnOnce(&mut Ui, &mut S),
) {
    let whole = ui.available_rect_before_wrap();
    let room = whole.width() - size::PAGE_SIDE_W - size::PAGE_SIDE_GAP;
    if room < size::SETTING_CONTROL_W * 2.0 {
        main(ui, state);
        ui.add_space(size::PAGE_BLOCK_GAP);
        side(ui, state);
        return;
    }
    let main_w = room.min(size::SETTINGS_MAX_W);
    let left = egui::Rect::from_min_size(whole.min, vec2(main_w, whole.height()));
    let right = egui::Rect::from_min_size(
        whole.min + vec2(main_w + size::PAGE_SIDE_GAP, 0.0),
        vec2(size::PAGE_SIDE_W, whole.height()),
    );
    let mut left_ui = ui.new_child(
        UiBuilder::new()
            .max_rect(left)
            .layout(Layout::top_down(Align::Min)),
    );
    main(&mut left_ui, state);
    let mut right_ui = ui.new_child(
        UiBuilder::new()
            .max_rect(right)
            .layout(Layout::top_down(Align::Min)),
    );
    right_ui.spacing_mut().item_spacing.y = size::CARD_GAP;
    side(&mut right_ui, state);
    let bottom = left_ui
        .min_rect()
        .bottom()
        .max(right_ui.min_rect().bottom());
    ui.allocate_rect(
        egui::Rect::from_min_max(whole.min, egui::pos2(whole.right(), bottom)),
        egui::Sense::hover(),
    );
}
