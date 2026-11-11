//! A whole page that isn't built: a center of the editor, or a page of
//! File. It is the room it is given, on a File page's grey, with one
//! card in the middle saying what will be there and when.

use super::draw::{begin, icon_ink, ink, mark};
use super::{DESCRIPTION, Unbuilt, UnbuiltId};
use crate::icons::{self, Icon};
use crate::tokens::{self, size, text};
use crate::widgets::{self, width_of, words};
use egui::{
    Align, Align2, CornerRadius, Layout, Rect, Response, RichText, Sense, Stroke, StrokeKind, Ui,
    UiBuilder, WidgetInfo, pos2, vec2,
};

/// What a whole unbuilt page reports.
#[derive(Debug)]
pub struct PageAnswer {
    /// The card in the middle of the page.
    pub response: Response,
    /// Whether "Hide unfinished parts" was clicked: the caller turns the
    /// setting on.
    pub hide: bool,
}

/// A whole page that isn't built (a center, or a File page): the room
/// it is given, on a File page's grey, with one card in the middle: the
/// part's icon, its name, what it will do, when, and a link that hides
/// every unfinished part.
pub fn page(ui: &mut Ui, id: UnbuiltId, icon: Icon) -> PageAnswer {
    let part = begin(ui, id);
    let whole = ui.available_rect_before_wrap();
    ui.allocate_rect(whole, Sense::hover());
    ui.painter().rect_filled(whole, 0.0, tokens::shelf_bar());
    let width = size::UNBUILT_CARD_W.min(whole.width());
    let [pad_top, pad_side, pad_foot] = size::UNBUILT_CARD_PAD;
    let inside_w = (width - 2.0 * pad_side).max(0.0);
    // Laid out once to learn its height, then placed in the middle.
    let height_id = ui.id().with(("unbuilt page height", id));
    let known: f32 = ui
        .ctx()
        .data(|data| data.get_temp(height_id))
        .unwrap_or(PAGE_CARD_H);
    let card = Rect::from_center_size(whole.center(), vec2(width, known));
    let radius = CornerRadius::same(size::radius::PROJECT_CARD);
    ui.painter().rect_filled(card, radius, tokens::panel());
    ui.painter().rect_stroke(
        card,
        radius,
        Stroke::new(widgets::hairline(ui), tokens::card_border()),
        StrokeKind::Inside,
    );
    let content = Rect::from_min_size(
        pos2(card.left() + pad_side, card.top() + pad_top),
        vec2(inside_w, 0.0),
    );
    let mut inside = ui.new_child(
        UiBuilder::new()
            .max_rect(content)
            .layout(Layout::top_down(Align::Center)),
    );
    inside.spacing_mut().item_spacing.y = size::CARD_INNER_GAP;
    let hide = page_card(&mut inside, part, icon);
    let needed = inside.min_rect().height() + pad_top + pad_foot;
    if (needed - known).abs() > 0.5 {
        ui.ctx()
            .data_mut(|data| data.insert_temp(height_id, needed));
        ui.ctx().request_repaint();
    }
    let response = ui.interact(card, ui.id().with(("unbuilt page", id)), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(egui::WidgetType::Other, true, part.name));
    ui.ctx()
        .accesskit_node_builder(response.id, |node| node.set_description(DESCRIPTION));
    PageAnswer { response, hide }
}

/// A page card's height before it has been laid out once.
const PAGE_CARD_H: f32 = 190.0;

/// What is on a page's card. Returns whether its link was clicked.
fn page_card(ui: &mut Ui, part: &Unbuilt, icon: Icon) -> bool {
    let (spot, _) = ui.allocate_exact_size(
        vec2(size::UNBUILT_PAGE_ICON, size::UNBUILT_PAGE_ICON),
        Sense::hover(),
    );
    icons::icon(ui.painter(), spot, icon, icon_ink());
    ui.add_space(6.0);
    ui.add(
        egui::Label::new(
            RichText::new(part.name)
                .font(text::heading())
                .color(tokens::text_primary()),
        )
        .selectable(false),
    );
    ui.add(
        egui::Label::new(
            RichText::new(part.what)
                .font(text::page_subtitle())
                .line_height(Some(12.5 * 1.45))
                .color(tokens::text_label()),
        )
        .selectable(false)
        .wrap(),
    );
    ui.add_space(6.0);
    // The mark, then the line that says when.
    let line = part.page_line();
    let font = text::row();
    let height = widgets::line_height(ui, &font);
    let gap = 7.0;
    let wanted = vec2(
        size::UNBUILT_MARK + gap + width_of(ui, &line, font.clone()),
        height,
    );
    let (when, said) = ui.allocate_exact_size(wanted, Sense::hover());
    said.widget_info(|| WidgetInfo::labeled(egui::WidgetType::Label, true, &line));
    mark(
        ui,
        pos2(when.left(), when.center().y - size::UNBUILT_MARK / 2.0),
    );
    let room = Rect::from_min_max(
        pos2(when.left() + size::UNBUILT_MARK + gap, when.top()),
        when.right_bottom(),
    );
    words(ui, room, Align2::LEFT_CENTER, &line, font, ink());
    ui.add_space(10.0);
    widgets::button::link_at(ui, "Hide unfinished parts", text::row()).clicked()
}
