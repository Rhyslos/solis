//! How Properties lays out its cards (spec 199-213; the approved
//! additions, round 2, `04-properties-*.png` and `01-hints-card-a.png`):
//! the cards side by side as many as fit at their least width, each row
//! of cards as tall as its tallest; a card's title row with ? (its manual
//! page) and ⋯ (its menu, which a right-click on the card opens too); and
//! Hints as the last, quieter card.

use crate::icons::Icon;
use crate::properties::PropertyRequest;
use crate::tokens::{self, size, text};
use crate::unbuilt::{self, Entry, UnbuiltId};
use crate::widgets::{self, menu};
use egui::RichText;
use egui::{
    Align, Align2, CornerRadius, Id, Layout, Rect, Response, Sense, Stroke, StrokeKind, Ui,
    UiBuilder, pos2, vec2,
};
use reflect::EntityId;

/// How many cards go side by side in `width`: as many as fit at the
/// least card width, but never more than there are.
pub(crate) fn across(width: f32, cards: usize) -> usize {
    let fit = ((width + size::CARD_GAP) / (size::CARD_MIN_W + size::CARD_GAP)).floor() as usize;
    fit.clamp(1, cards.max(1))
}

/// One row of `count` cards, of `slots` across the width (a last row
/// with fewer cards keeps the others' width). `draw` draws card `i`'s
/// contents and gets the card's background response (for its right-click
/// menu). Every card in the row is as tall as the tallest.
pub(crate) fn card_row(
    ui: &mut Ui,
    salt: &str,
    (count, slots): (usize, usize),
    mut draw: impl FnMut(&mut Ui, usize, &Response),
) {
    let room = ui.available_rect_before_wrap();
    let slots = slots.max(1);
    let gaps = (slots - 1) as f32 * size::CARD_GAP;
    let width = ((room.width() - gaps) / slots as f32).floor();
    let top = room.top();
    let row_id = ui.id().with(("card row", salt));
    // The row's height last frame: the cards' backgrounds are laid down
    // first (under their contents, and under the pointer last), so a
    // right-click on a card's empty space reaches them.
    let last: f32 = ui.data(|d| d.get_temp(row_id)).unwrap_or(0.0);
    let mut bottom = top;
    let mut drawn = Vec::with_capacity(count);
    for i in 0..count {
        let left = room.left() + i as f32 * (width + size::CARD_GAP);
        let card = Rect::from_min_size(pos2(left, top), vec2(width, last.max(1.0)));
        let under = ui.painter().add(egui::Shape::Noop);
        let background = ui.interact(card, row_id.with(i), Sense::click());
        let inside = Rect::from_min_max(
            pos2(left + size::CARD_PAD, top + size::CARD_PAD),
            pos2(left + width - size::CARD_PAD, f32::INFINITY),
        );
        let mut child = ui.new_child(
            UiBuilder::new()
                .id_salt(i)
                .max_rect(inside)
                .layout(Layout::top_down(Align::Min)),
        );
        child.set_width(inside.width());
        child.spacing_mut().item_spacing.y = size::CARD_INNER_GAP;
        draw(&mut child, i, &background);
        bottom = bottom.max(child.min_rect().bottom() + size::CARD_PAD);
        drawn.push((under, left));
    }
    let height = bottom - top;
    for (under, left) in drawn {
        let card = Rect::from_min_size(pos2(left, top), vec2(width, height));
        ui.painter().set(under, card_shape(ui, card));
    }
    ui.data_mut(|d| d.insert_temp(row_id, height));
    ui.advance_cursor_after_rect(Rect::from_min_max(
        room.left_top(),
        pos2(room.right(), bottom),
    ));
}

/// A card's fill and border in `rect`.
fn card_shape(ui: &Ui, rect: Rect) -> egui::Shape {
    egui::Shape::Rect(egui::epaint::RectShape::new(
        rect,
        CornerRadius::same(size::radius::CARD),
        tokens::card(),
        Stroke::new(widgets::hairline(ui), tokens::card_border()),
        StrokeKind::Inside,
    ))
}

/// What a card's title row was asked to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CardAsk {
    /// Open its manual page.
    Help,
    /// One of its menu's rows.
    Menu(CardMenu),
}

/// The rows of a card's menu (`04-properties-card-menu.png`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CardMenu {
    Copy,
    Paste,
    Reset,
    OpenCode,
    Remove,
}

/// What a card's menu offers.
#[derive(Debug, Clone, Copy)]
pub(crate) struct MenuHas<'a> {
    /// The component's name.
    pub name: &'a str,
    /// Whether values were copied to paste.
    pub can_paste: bool,
    /// Whether it is game code (Open code).
    pub code: bool,
    /// Whether it has a manual page (?).
    pub help: bool,
}

/// A card's title row: its name, then at the right ? and ⋯. Returns
/// what was asked for, also from the menu `background` opens on a
/// right-click.
pub(crate) fn title_row(
    ui: &mut Ui,
    has: MenuHas<'_>,
    hover: &str,
    background: &Response,
) -> Option<CardAsk> {
    let mut asked = None;
    let (line, _) = ui.allocate_exact_size(
        vec2(ui.available_width(), size::ICON_BUTTON),
        Sense::hover(),
    );
    let mut row = ui.new_child(
        UiBuilder::new()
            .max_rect(line)
            .layout(Layout::right_to_left(Align::Center)),
    );
    row.spacing_mut().item_spacing.x = 0.0;
    let more = widgets::button::icon(&mut row, Icon::More, &format!("{} menu", has.name));
    let from_more = egui::Popup::menu(&more).show(|ui| menu_rows(ui, has));
    if let Some(row) = from_more.and_then(|picked| picked.inner) {
        asked = Some(CardAsk::Menu(row));
    }
    let mut from_card = None;
    background.context_menu(|ui| from_card = menu_rows(ui, has));
    if let Some(row) = from_card {
        asked = Some(CardAsk::Menu(row));
    }
    if has.help
        && widgets::button::icon(&mut row, Icon::Help, &format!("About {}", has.name)).clicked()
    {
        asked = Some(CardAsk::Help);
    }
    let words = row.max_rect().with_max_x(row.cursor().right());
    let named = ui.interact(
        widgets::words(
            ui,
            words,
            Align2::LEFT_CENTER,
            has.name,
            text::card_name(),
            tokens::text_primary(),
        ),
        Id::new(("card title", has.name)),
        Sense::hover(),
    );
    named.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, has.name));
    named.on_hover_text(hover);
    asked
}

/// The rows of a card's menu. Returns the row clicked.
fn menu_rows(ui: &mut Ui, has: MenuHas<'_>) -> Option<CardMenu> {
    ui.spacing_mut().item_spacing.y = 0.0;
    let mut picked = None;
    if menu::item(ui, "Copy values", "").clicked() {
        picked = Some(CardMenu::Copy);
    }
    let paste = ui.add_enabled_ui(has.can_paste, |ui| menu::item(ui, "Paste values", ""));
    if paste
        .inner
        .on_disabled_hover_text(format!("Copy a {}'s values first", has.name))
        .clicked()
    {
        picked = Some(CardMenu::Paste);
    }
    if menu::item(ui, "Reset to defaults", "")
        .on_hover_text("Puts every setting back to its starting value")
        .clicked()
    {
        picked = Some(CardMenu::Reset);
    }
    if has.code {
        menu::line(ui);
        if menu::item(ui, "Open code", "").clicked() {
            picked = Some(CardMenu::OpenCode);
        }
    }
    menu::line(ui);
    if menu::item(ui, &format!("Remove {}", has.name), "").clicked() {
        picked = Some(CardMenu::Remove);
    }
    if picked.is_some() {
        ui.close();
    }
    picked
}

/// The Hints card, last and quieter: its title with "Turn hints off" at
/// the right, then one sentence a line, the first brighter. Returns
/// whether "Turn hints off" was clicked.
pub(crate) fn hints_card(ui: &mut Ui, hints: &[String]) -> bool {
    let mut off = false;
    let frame = egui::Frame::new()
        .fill(tokens::panel())
        .stroke(Stroke::new(widgets::hairline(ui), tokens::card_border()))
        .corner_radius(CornerRadius::same(size::radius::CARD))
        .inner_margin(egui::Margin::same(size::CARD_PAD as i8));
    frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.spacing_mut().item_spacing.y = size::ROW_GAP;
        ui.horizontal(|ui| {
            let (spot, _) =
                ui.allocate_exact_size(vec2(size::ICON_SMALL, size::ICON_SMALL), Sense::hover());
            crate::icons::icon(ui.painter(), spot, Icon::Hint, tokens::text_label());
            // Painted: the card is found by its link, and Preferences
            // keeps the name "Hints" for its switch.
            let width = widgets::width_of(ui, "Hints", text::card_name());
            let (title, _) = ui.allocate_exact_size(vec2(width, size::ICON_SMALL), Sense::hover());
            widgets::words(
                ui,
                title,
                Align2::LEFT_CENTER,
                "Hints",
                text::card_name(),
                tokens::text_primary(),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                off = widgets::button::link(ui, "Turn hints off")
                    .on_hover_text("File \u{203a} Preferences turns them back on")
                    .clicked();
                if off {
                    crate::shell::status::tell(
                        ui.ctx(),
                        "Hints are off: File \u{203a} Preferences turns them back on",
                    );
                }
            });
        });
        for (n, hint) in hints.iter().enumerate() {
            let ink = if n == 0 {
                tokens::text_primary()
            } else {
                tokens::text_label()
            };
            ui.add(egui::Label::new(egui::RichText::new(hint).font(text::ui()).color(ink)).wrap());
        }
    });
    off
}

/// The Model card's rows the design has and Solis doesn't yet: a model
/// has one setting today, the model itself (found while mocking up,
/// round 2).
pub(crate) fn model_rows_to_come(ui: &mut Ui) {
    let rows: Vec<Entry<()>> = vec![
        Entry::Unbuilt(UnbuiltId::HomeObjectModelMaterial),
        Entry::Unbuilt(UnbuiltId::HomeObjectModelShadows),
    ];
    for entry in unbuilt::filter(rows) {
        if let Entry::Unbuilt(part) = entry {
            unbuilt::field_row(ui, part);
        }
    }
}

/// A card for a component whose game code isn't there: what it is, and
/// Remove.
pub(crate) fn missing_card(
    ui: &mut Ui,
    id: EntityId,
    component: &str,
    requests: &mut Vec<PropertyRequest>,
) {
    ui.label(
        RichText::new(format!("Missing component: {component}"))
            .font(text::card_name())
            .color(tokens::WARNING),
    )
    .on_hover_text(
        "Its script or plugin isn't there. Its values are kept as they are until you remove it.",
    );
    if widgets::button::Button::plain("Remove").show(ui).clicked() {
        requests.push(PropertyRequest::RemoveComponent {
            entity: id,
            component: component.to_owned(),
        });
    }
}

/// A game code card's second line: the file it's written in, quiet and
/// in mono, and **Open code** at the right (only Lust opens in its
/// program; other code is shown in its folder, and the button says so).
/// Returns whether the button was clicked.
pub(crate) fn code_line(ui: &mut Ui, component: &str, file: &str) -> bool {
    let shown = file.rsplit_once('/').map_or(file, |(_, n)| n);
    let opens = crate::library::LibraryKind::Code.opens_in_its_program(file);
    let (words, hint) = if opens {
        (
            "Open code",
            format!("Opens {file}, where {component} is written, in your code editor"),
        )
    } else {
        (
            "Show in folder",
            format!("Shows {file}, where {component} is written, in its folder"),
        )
    };
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(shown)
                .font(text::mono_small())
                .color(tokens::text_meta()),
        )
        .on_hover_text(file);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let button = widgets::button::Button::plain(words)
                .icon(crate::icons::Icon::External)
                .show(ui);
            button.widget_info(|| {
                egui::WidgetInfo::labeled(
                    egui::WidgetType::Button,
                    true,
                    format!("Open {component}'s code"),
                )
            });
            button.on_hover_text(hint).clicked()
        })
        .inner
    })
    .inner
}
