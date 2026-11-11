//! How an unfinished part is drawn: one function for each shape the
//! frame has, each taking the part's id and nothing about looks.
//!
//! The look is the one the project lead chose (look A of
//! `docs/design/g-layout/additions/round-1/`): the part sits where the
//! design puts it, at its real size; its words are in the lowest
//! readable grey and its icon fainter still; a 5 px hollow square marks
//! it, at the top right corner of a button or a tool, or after the words
//! of a tab or a row; and it never lights up under the pointer. It reads
//! as "not here yet", never as broken or switched off.

use super::{DESCRIPTION, Unbuilt, UnbuiltId, get, note, say};
use crate::icons::{self, Icon};
use crate::tokens::{self, size, text};
use crate::widgets::{self, rows, width_of, words};
use egui::{
    Align, Align2, Color32, CornerRadius, FontId, Layout, Pos2, Rect, Response, RichText, Sense,
    Stroke, StrokeKind, Ui, UiBuilder, WidgetInfo, pos2, vec2,
};

/// The grey of an unfinished part's words: the lowest the design reads.
pub(super) fn ink() -> Color32 {
    tokens::text_meta()
}

/// The grey of an unfinished part's icon.
pub(super) fn icon_ink() -> Color32 {
    tokens::text_separator()
}

/// Starts drawing `id`: notes it for this frame and gives its line of
/// the table.
pub(super) fn begin(ui: &Ui, id: UnbuiltId) -> &'static Unbuilt {
    note(ui.ctx(), id);
    get(id)
}

/// Finishes an unfinished part: its name and "Not built yet" for screen
/// readers and tests, what it will do on hover, and a click that only
/// says so in the status bar.
fn finish(ui: &Ui, response: Response, part: &Unbuilt) -> Response {
    response.widget_info(|| WidgetInfo::labeled(egui::WidgetType::Button, true, part.name));
    ui.ctx()
        .accesskit_node_builder(response.id, |node| node.set_description(DESCRIPTION));
    if response.clicked() {
        say(ui.ctx(), part.id);
    }
    response.on_hover_text(part.hover_text())
}

/// The mark: a hollow square with its top left corner at `corner`, on
/// whole pixels so its 1 px outline is sharp.
pub(super) fn mark(ui: &Ui, corner: Pos2) {
    let scale = ui.ctx().pixels_per_point();
    let corner = pos2(
        tokens::on_pixel(corner.x, scale),
        tokens::on_pixel(corner.y, scale),
    );
    let square = Rect::from_min_size(corner, vec2(size::UNBUILT_MARK, size::UNBUILT_MARK));
    ui.painter().rect_stroke(
        square,
        CornerRadius::ZERO,
        Stroke::new(widgets::hairline(ui), tokens::text_faint()),
        StrokeKind::Inside,
    );
}

/// The mark in the top right corner of a button, `inset` from its edges.
fn corner_mark(ui: &Ui, rect: Rect, inset: f32) {
    mark(
        ui,
        pos2(
            rect.right() - inset - size::UNBUILT_MARK,
            rect.top() + inset,
        ),
    );
}

/// How much room words with the mark after them take.
fn marked_width(ui: &Ui, label: &str, font: FontId) -> f32 {
    width_of(ui, label, font) + size::UNBUILT_MARK_GAP + size::UNBUILT_MARK
}

/// `label` from `left`, level with the middle of `rect`, and the mark
/// after it, sitting just above the line the letters stand on.
fn marked_words(ui: &Ui, rect: Rect, left: f32, label: &str, font: FontId, colour: Color32) {
    let room = Rect::from_min_max(pos2(left, rect.top()), rect.right_bottom());
    let drawn = words(ui, room, Align2::LEFT_CENTER, label, font, colour);
    mark(
        ui,
        pos2(
            drawn.right() + size::UNBUILT_MARK_GAP,
            rect.center().y - size::UNBUILT_MARK / 2.0 + size::UNBUILT_MARK_DROP,
        ),
    );
}

/// A large ribbon button that isn't built: its icon over its name, the
/// mark in its top right corner.
pub fn ribbon_big(ui: &mut Ui, id: UnbuiltId, icon: Icon) -> Response {
    let part = begin(ui, id);
    let [width, height] = size::RIBBON_BIG;
    let (rect, response) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    if ui.is_rect_visible(rect) {
        // The icon, a gap and the label, together in the middle.
        let label_h = text::RIBBON_LABEL_SIZE + 2.0;
        let stack = size::RIBBON_BIG_ICON + size::RIBBON_BIG_GAP + label_h;
        let top = rect.center().y - stack / 2.0;
        let spot = Rect::from_min_size(
            pos2(rect.center().x - size::RIBBON_BIG_ICON / 2.0, top),
            vec2(size::RIBBON_BIG_ICON, size::RIBBON_BIG_ICON),
        );
        icons::icon(ui.painter(), spot, icon, icon_ink());
        let label = Rect::from_min_max(
            pos2(rect.left(), spot.bottom() + size::RIBBON_BIG_GAP),
            pos2(rect.right(), spot.bottom() + size::RIBBON_BIG_GAP + label_h),
        );
        // A name too long for the button ends in "…": pointing at the
        // button says it whole.
        let font = text::ribbon_label();
        if width_of(ui, part.name, font.clone()) <= label.width() {
            words(ui, label, Align2::CENTER_CENTER, part.name, font, ink());
        } else {
            widgets::cut_off(ui, label.shrink2(vec2(2.0, 0.0)), part.name, font, ink());
        }
        corner_mark(ui, rect, size::UNBUILT_CORNER);
    }
    finish(ui, response, part)
}

/// What a small ribbon row is, apart from its name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowKind {
    /// A name alone.
    Plain,
    /// A box to tick, then the name.
    Check,
    /// The name, then the small arrow of a list.
    List,
}

/// How wide [`ribbon_row`] draws `id`: a ribbon lays its groups out
/// from their widths before it draws them.
pub fn ribbon_row_width(ui: &Ui, id: UnbuiltId, kind: RowKind) -> f32 {
    let (before, after) = row_extras(kind);
    let content = before + width_of(ui, get(id).name, text::ribbon_small()) + after;
    2.0 * size::RIBBON_ROW_PAD + content + size::GAP + size::UNBUILT_MARK
}

/// The room a row's kind takes before and after its name.
fn row_extras(kind: RowKind) -> (f32, f32) {
    let before = match kind {
        RowKind::Check => size::CHECKBOX + size::GAP,
        RowKind::Plain | RowKind::List => 0.0,
    };
    let after = match kind {
        RowKind::List => LIST_ARROW + 3.0,
        RowKind::Plain | RowKind::Check => 0.0,
    };
    (before, after)
}

/// A small ribbon row that isn't built, as tall as the room it is given
/// (a ribbon's rows are a pixel shorter where three are stacked).
pub fn ribbon_row(ui: &mut Ui, id: UnbuiltId, kind: RowKind) -> Response {
    let part = begin(ui, id);
    let font = text::ribbon_small();
    let (before, after) = row_extras(kind);
    let width = ribbon_row_width(ui, id, kind);
    let height = ui.available_height().min(size::RIBBON_ROW_H);
    let (rect, response) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    if ui.is_rect_visible(rect) {
        let mut left = rect.left() + size::RIBBON_ROW_PAD;
        if kind == RowKind::Check {
            let square = Rect::from_min_size(
                pos2(left, rect.center().y - size::CHECKBOX / 2.0),
                vec2(size::CHECKBOX, size::CHECKBOX),
            );
            widgets::border(ui, square, size::radius::CHECKBOX, tokens::strong_line());
            left += before;
        }
        let room = Rect::from_min_max(pos2(left, rect.top()), rect.right_bottom());
        let drawn = words(ui, room, Align2::LEFT_CENTER, part.name, font, ink());
        let mut right = drawn.right();
        if kind == RowKind::List {
            list_arrow(ui, pos2(right + 3.0 + LIST_ARROW / 2.0, rect.center().y));
            right += after;
        }
        let top = rect.center().y - size::UNBUILT_MARK / 2.0 + size::UNBUILT_MARK_DROP;
        mark(ui, pos2(right + size::GAP, top));
    }
    finish(ui, response, part)
}

/// How wide the small arrow of a list is.
const LIST_ARROW: f32 = size::LIST_ARROW;

/// The small filled arrow after a row that would open a list.
fn list_arrow(ui: &Ui, middle: Pos2) {
    let half = LIST_ARROW / 2.0;
    ui.painter().add(egui::Shape::convex_polygon(
        vec![
            pos2(middle.x - half, middle.y - half / 2.0),
            pos2(middle.x + half, middle.y - half / 2.0),
            pos2(middle.x, middle.y + half * 0.75),
        ],
        ink(),
        Stroke::NONE,
    ));
}

/// A square button holding an icon alone, `side` across, the mark
/// `inset` from its top right corner.
fn icon_square(
    ui: &mut Ui,
    id: UnbuiltId,
    icon: Icon,
    side: f32,
    drawn: f32,
    inset: f32,
) -> Response {
    let part = begin(ui, id);
    let (rect, response) = ui.allocate_exact_size(vec2(side, side), Sense::click());
    if ui.is_rect_visible(rect) {
        let spot = Rect::from_center_size(rect.center(), vec2(drawn, drawn));
        icons::icon(ui.painter(), spot, icon, icon_ink());
        corner_mark(ui, rect, inset);
    }
    finish(ui, response, part)
}

/// A toolbox tool that isn't built.
pub fn tool(ui: &mut Ui, id: UnbuiltId, icon: Icon) -> Response {
    icon_square(
        ui,
        id,
        icon,
        size::TOOL_BUTTON,
        size::TOOL_ICON,
        size::UNBUILT_CORNER,
    )
}

/// An icon button that isn't built.
pub fn icon_button(ui: &mut Ui, id: UnbuiltId, icon: Icon) -> Response {
    icon_square(
        ui,
        id,
        icon,
        size::ICON_BUTTON,
        size::ICON_SMALL,
        size::UNBUILT_CORNER - 1.0,
    )
}

/// Words with the mark after them, in a strip `height` tall with `pad`
/// at each side: the shape of every tab.
fn marked_strip(ui: &mut Ui, id: UnbuiltId, height: f32, pad: f32, font: FontId) -> Response {
    let part = begin(ui, id);
    let width = 2.0 * pad + marked_width(ui, part.name, font.clone());
    let (rect, response) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    if ui.is_rect_visible(rect) {
        marked_words(ui, rect, rect.left() + pad, part.name, font, ink());
    }
    finish(ui, response, part)
}

/// A ribbon tab, or a contextual tab, that isn't built.
pub fn tab(ui: &mut Ui, id: UnbuiltId) -> Response {
    marked_strip(
        ui,
        id,
        size::RIBBON_TAB_H,
        size::RIBBON_TAB_PAD,
        text::tab(),
    )
}

/// A shelf tab that isn't built.
pub fn shelf_tab(ui: &mut Ui, id: UnbuiltId) -> Response {
    marked_strip(ui, id, size::shelf_tab_h(), size::ROW_PAD, text::tab())
}

/// A sub-tab of a File page that isn't built.
pub fn sub_tab(ui: &mut Ui, id: UnbuiltId) -> Response {
    marked_strip(
        ui,
        id,
        size::SUB_TAB_H,
        size::SUB_TAB_PAD,
        text::page_subtitle(),
    )
}

/// Something the status bar would say, that isn't built.
pub fn status_item(ui: &mut Ui, id: UnbuiltId) -> Response {
    marked_strip(ui, id, size::STATUS_BAR_H, 0.0, text::ribbon_small())
}

/// A link that isn't built.
pub fn link(ui: &mut Ui, id: UnbuiltId) -> Response {
    let height = ui.text_style_height(&egui::TextStyle::Body);
    marked_strip(ui, id, height, 0.0, text::ui())
}

/// A chip that isn't built.
pub fn chip(ui: &mut Ui, id: UnbuiltId) -> Response {
    marked_strip(
        ui,
        id,
        size::CHIP_H - 2.0,
        size::DASHED_BUTTON_PAD,
        text::medium_at(text::SMALL_SIZE),
    )
}

/// An item of the File page's menu column that isn't built: as wide as
/// the column, its icon before its name.
pub fn menu_item(ui: &mut Ui, id: UnbuiltId, icon: Icon) -> Response {
    let part = begin(ui, id);
    let wanted = vec2(ui.available_width(), size::FILE_MENU_ITEM_H);
    let (rect, response) = ui.allocate_exact_size(wanted, Sense::click());
    if ui.is_rect_visible(rect) {
        let left = rect.left() + size::FILE_MENU_ITEM_PAD;
        let spot = Rect::from_min_size(
            pos2(left, rect.center().y - size::TOOL_ICON / 2.0),
            vec2(size::TOOL_ICON, size::TOOL_ICON),
        );
        icons::icon(ui.painter(), spot, icon, icon_ink());
        let words_left = spot.right() + size::FILE_MENU_ITEM_GAP;
        marked_words(ui, rect, words_left, part.name, text::menu_item(), ink());
    }
    finish(ui, response, part)
}

/// A setting that isn't built: its name with the mark, the sentence
/// about it, and a switch that is off and fainter than a real one.
pub fn setting_row(ui: &mut Ui, id: UnbuiltId) -> Response {
    let part = begin(ui, id);
    let across = ui.available_rect_before_wrap();
    let top = ui.cursor().top();
    rows::setting_row_with(
        ui,
        |ui| {
            ui.spacing_mut().item_spacing.y = size::SETTING_HINT_GAP;
            let font = text::setting_name();
            let height = widgets::line_height(ui, &font);
            let wanted = vec2(marked_width(ui, part.name, font.clone()), height);
            let (name, _) = ui.allocate_exact_size(wanted, Sense::hover());
            marked_words(ui, name, name.left(), part.name, font, ink());
            ui.add(
                egui::Label::new(
                    RichText::new(format!("{}.", part.what))
                        .font(text::setting_hint())
                        .line_height(Some(text::SETTING_HINT_LINE_H))
                        .color(tokens::text_meta()),
                )
                .selectable(false)
                .wrap(),
            );
        },
        faint_switch,
    );
    // The whole row is the part: it can be pointed at and clicked
    // anywhere.
    let row = Rect::from_min_max(
        pos2(across.left(), top),
        pos2(across.right(), ui.cursor().top()),
    );
    let response = ui.interact(row, ui.id().with(("unbuilt setting", id)), Sense::click());
    finish(ui, response, part)
}

/// A switch that is off, drawn fainter than one that works.
fn faint_switch(ui: &mut Ui) {
    let [width, height] = size::TOGGLE;
    let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    let round = CornerRadius::same((height / 2.0) as u8);
    ui.painter().rect_filled(rect, round, tokens::line());
    let reach = size::TOGGLE_KNOB / 2.0;
    let inset = (height - size::TOGGLE_KNOB) / 2.0;
    ui.painter().circle_filled(
        pos2(rect.left() + inset + reach, rect.center().y),
        reach,
        tokens::text_separator(),
    );
}

/// A named field that isn't built: its name in the names' column, and
/// an empty, fainter field beside it.
pub fn field_row(ui: &mut Ui, id: UnbuiltId) -> Response {
    let part = begin(ui, id);
    let wanted = vec2(ui.available_width(), size::field_h());
    let (rect, response) = ui.allocate_exact_size(wanted, Sense::click());
    if ui.is_rect_visible(rect) {
        marked_words(ui, rect, rect.left(), part.name, text::row(), ink());
        let field = Rect::from_min_max(
            pos2(
                rect.left() + size::KEY_COLUMN_W + size::KEY_VALUE_GAP,
                rect.top(),
            ),
            rect.right_bottom(),
        );
        if field.width() > size::FIELD_PAD {
            widgets::border(ui, field, size::radius::CONTROL, tokens::line());
        }
    }
    finish(ui, response, part)
}

/// A card that isn't built: its name with the mark as the title, and
/// the sentence about it.
pub fn card(ui: &mut Ui, id: UnbuiltId) -> Response {
    let part = begin(ui, id);
    let inner = widgets::card::frame(ui).show(ui, |ui| {
        ui.with_layout(Layout::top_down(Align::Min), |ui| {
            ui.spacing_mut().item_spacing.y = size::CARD_INNER_GAP;
            let font = text::header();
            let height = widgets::line_height(ui, &font);
            let wanted = vec2(marked_width(ui, part.name, font.clone()), height);
            let (title, _) = ui.allocate_exact_size(wanted, Sense::hover());
            marked_words(ui, title, title.left(), part.name, font, ink());
            sentence(ui, part, text::row(), tokens::text_meta());
        });
    });
    let response = ui.interact(
        inner.response.rect,
        ui.id().with(("unbuilt card", id)),
        Sense::click(),
    );
    finish(ui, response, part)
}

/// The sentence about a part, as words that wrap.
fn sentence(ui: &mut Ui, part: &Unbuilt, font: FontId, colour: Color32) {
    ui.add(
        egui::Label::new(
            RichText::new(format!("{}.", part.what))
                .font(font)
                .color(colour),
        )
        .selectable(false)
        .wrap(),
    );
}

/// A shelf column that isn't built: its header, with the mark after its
/// name, over the sentence about it. It fills the room it is given.
pub fn column(ui: &mut Ui, id: UnbuiltId) -> Response {
    let part = begin(ui, id);
    let whole = ui.available_rect_before_wrap();
    let (rect, _) =
        ui.allocate_exact_size(vec2(whole.width(), size::COLUMN_HEADER_H), Sense::hover());
    marked_words(
        ui,
        rect,
        rect.left() + size::ROW_PAD,
        part.name,
        text::header(),
        ink(),
    );
    tokens::hline(
        ui.painter(),
        rect.x_range(),
        rect.bottom() - size::LINE,
        tokens::line(),
    );
    let body = Rect::from_min_max(rect.left_bottom(), whole.right_bottom()).shrink(size::ROW_PAD);
    let mut inside = ui.new_child(UiBuilder::new().max_rect(body));
    sentence(&mut inside, part, text::row(), tokens::text_meta());
    let response = ui.interact(whole, ui.id().with(("unbuilt column", id)), Sense::click());
    finish(ui, response, part)
}
