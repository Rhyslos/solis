//! File › Preferences › Layout (decision 0118, board 06): every layout as
//! a card with its drawn miniature, its name and one line saying what it
//! does, three to a row. Pointing at one shows it large in the Preview
//! box beside them; a click applies it at once and a small note says
//! "Layout is now …" with Undo. "In use" marks the one in use. Under
//! them, whether each tab keeps its own panel sizes, and Reset sizes.

use super::layout_miniature::miniature;
use super::settings::Setting;
use super::{plain, sentence};
use crate::icons::{self, Icon};
use crate::shell::PanelLayout;
use crate::tokens::{self, size, text};
use crate::widgets::{self, button::Button};
use crate::workbench::{Workbench, WorkbenchAction};
use egui::{Align2, CornerRadius, Rect, Sense, Stroke, StrokeKind, Ui, WidgetInfo, pos2, vec2};

/// Cards in a row.
const ACROSS: usize = 3;
/// A card's miniature's height for its width.
const PICTURE_SHARE: f32 = 0.6;
/// The room under a card's miniature: its name and its line.
const WORDS_H: f32 = 64.0;

impl Workbench {
    /// The Layout sub-tab: the cards and the Preview box side by side,
    /// then the panels' sizes.
    pub(super) fn layout_page(&mut self, ui: &mut Ui, asked: &mut Vec<WorkbenchAction>) {
        widgets::rows::section_caps(ui, "Layout");
        sentence(ui, Setting::Layout.hint());
        ui.add_space(size::GAP * 2.0);
        // Room for the page's scroll bar at the right.
        let whole = ui.available_width() - size::GAP * 3.0;
        let side = size::PAGE_SIDE_W.min(whole * 0.3);
        let cards_w = (whole - side - size::PAGE_SIDE_GAP).max(whole * 0.5);
        ui.horizontal_top(|ui| {
            let pointed = ui
                .allocate_ui(vec2(cards_w, 0.0), |ui| {
                    ui.vertical(|ui| self.layout_cards(ui, cards_w)).inner
                })
                .inner;
            ui.add_space(size::PAGE_SIDE_GAP);
            ui.allocate_ui(vec2(side, 0.0), |ui| {
                ui.vertical(|ui| {
                    preview(ui, pointed.unwrap_or(self.layout()), pointed.is_some());
                });
            });
        });
        ui.add_space(size::SECTION_GAP);
        ui.allocate_ui(vec2(cards_w, 0.0), |ui| {
            ui.vertical(|ui| {
                for setting in [Setting::SizesPerTab, Setting::ResetSizes] {
                    self.setting(ui, setting, asked);
                }
            });
        });
        self.layout_note_box(ui);
    }

    /// The cards, three to a row. Returns the one pointed at, if any.
    fn layout_cards(&mut self, ui: &mut Ui, width: f32) -> Option<PanelLayout> {
        let gap = size::GAP * 2.0;
        let card_w = ((width - gap * (ACROSS - 1) as f32) / ACROSS as f32).floor();
        let picture_h = (card_w * PICTURE_SHARE).round();
        let card_h = picture_h + WORDS_H;
        let in_use = self.layout();
        let mut pointed = None;
        let mut chosen = None;
        for row in PanelLayout::ALL.chunks(ACROSS) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for layout in row {
                    let (rect, response) =
                        ui.allocate_exact_size(vec2(card_w, card_h), Sense::click());
                    response.widget_info(|| {
                        WidgetInfo::selected(
                            egui::WidgetType::Button,
                            true,
                            *layout == in_use,
                            layout.name(),
                        )
                    });
                    if response.hovered() {
                        pointed = Some(*layout);
                    }
                    if response.clicked() {
                        chosen = Some(*layout);
                    }
                    card(ui, rect, *layout, (*layout == in_use, response.hovered()));
                    widgets::focus_ring(ui, &response, rect, size::radius::CARD);
                }
            });
            ui.add_space(gap);
        }
        if let Some(layout) = chosen {
            self.choose_layout(ui.ctx(), layout);
        }
        pointed
    }

    /// The note after a layout was chosen, floating at the page's bottom
    /// right as board 06 draws it: "Layout is now X", Undo and ×.
    fn layout_note_box(&mut self, ui: &mut Ui) {
        let Some(now) = self.layout_note() else {
            return;
        };
        let mut undo = false;
        let mut close = false;
        let lift = size::STATUS_BAR_H + size::SECTION_GAP;
        egui::Area::new(egui::Id::new("layout note"))
            .anchor(Align2::RIGHT_BOTTOM, vec2(-size::SECTION_GAP, -lift))
            .order(egui::Order::Foreground)
            .show(ui.ctx(), |ui| {
                widgets::card::frame(ui)
                    .fill(crate::interface_look::notice())
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let (spot, _) = ui.allocate_exact_size(
                                vec2(size::ICON_SMALL, size::ICON_SMALL),
                                Sense::hover(),
                            );
                            icons::icon(ui.painter(), spot, Icon::Check, tokens::SUCCESS);
                            plain(ui, "Layout is now", text::ui(), tokens::text_primary());
                            plain(ui, now.name(), text::header(), tokens::text_emphasis());
                            undo = Button::plain("Undo")
                                .icon(Icon::Undo)
                                .show(ui)
                                .on_hover_text("Puts the layout before it back")
                                .clicked();
                            close =
                                widgets::button::icon(ui, Icon::Close, "Close the note").clicked();
                        });
                    });
            });
        if undo {
            self.undo_layout(ui.ctx());
        } else if close {
            self.close_layout_note();
        }
    }
}

/// One card: its miniature, its name (with "In use" at the right of the
/// one in use) and its line; its border lit while pointed at, and the
/// accent's round the one in use.
fn card(ui: &Ui, rect: Rect, layout: PanelLayout, (in_use, pointed): (bool, bool)) {
    let painter = ui.painter();
    let round = CornerRadius::same(size::radius::CARD);
    let accent = crate::interface_look::accent_fill();
    let fill = if in_use {
        crate::interface_look::selected()
    } else {
        tokens::card()
    };
    painter.rect_filled(rect, round, fill);
    let border = match (in_use, pointed) {
        (true, _) => Stroke::new(1.5, accent),
        (false, true) => Stroke::new(1.0, tokens::strong_line()),
        (false, false) => Stroke::new(1.0, tokens::card_border()),
    };
    painter.rect_stroke(rect, round, border, StrokeKind::Inside);
    let pad = size::GAP;
    let picture_h = (rect.width() * PICTURE_SHARE).round();
    let picture = Rect::from_min_size(
        rect.min + vec2(pad, pad),
        vec2(rect.width() - 2.0 * pad, picture_h - pad),
    );
    miniature(painter, picture, layout, false);
    let name_at = pos2(
        rect.left() + size::ROW_PAD,
        picture.bottom() + size::ROW_PAD + 2.0,
    );
    painter.text(
        name_at,
        Align2::LEFT_CENTER,
        layout.name(),
        text::header(),
        tokens::text_emphasis(),
    );
    if in_use {
        let at = pos2(rect.right() - size::ROW_PAD, name_at.y);
        let words = painter.text(at, Align2::RIGHT_CENTER, "In use", text::ui(), accent);
        let named = ui.interact(words, egui::Id::new(("in use", layout)), Sense::hover());
        named.widget_info(|| WidgetInfo::labeled(egui::WidgetType::Label, true, "In use"));
        let tick = Rect::from_center_size(
            pos2(words.left() - size::GAP - size::ICON_SMALL / 2.0, at.y),
            vec2(size::ICON_SMALL, size::ICON_SMALL),
        );
        icons::icon(painter, tick, Icon::Check, accent);
    }
    let line_rect = Rect::from_min_max(
        pos2(rect.left() + size::ROW_PAD, name_at.y + size::GAP * 1.5),
        pos2(rect.right() - size::ROW_PAD, rect.bottom() - size::GAP),
    );
    let galley = painter.layout(
        layout.line().to_owned(),
        text::setting_hint(),
        tokens::text_meta(),
        line_rect.width(),
    );
    painter
        .with_clip_rect(line_rect)
        .galley(line_rect.min, galley, tokens::text_meta());
}

/// The Preview box: the layout pointed at (or the one in use) large,
/// each part named, and what it does.
fn preview(ui: &mut Ui, layout: PanelLayout, pointing: bool) {
    widgets::card::frame(ui).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            plain(ui, "Preview", text::header(), tokens::text_emphasis());
            plain(ui, layout.name(), text::ui(), tokens::text_meta());
            if pointing {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    plain(
                        ui,
                        "while you point at it",
                        text::ui(),
                        tokens::text_faint(),
                    );
                });
            }
        });
        let width = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(vec2(width, (width * 0.62).round()), Sense::hover());
        miniature(ui.painter(), rect, layout, true);
        sentence(ui, layout.more());
    });
    ui.add_space(size::GAP * 2.0);
    ui.horizontal_wrapped(|ui| {
        let (spot, _) =
            ui.allocate_exact_size(vec2(size::ICON_SMALL, size::ICON_SMALL), Sense::hover());
        icons::icon(ui.painter(), spot, Icon::Info, tokens::text_meta());
        ui.add(
            egui::Label::new(
                egui::RichText::new("A layout changes where panels sit, never what they do.")
                    .font(text::ui())
                    .color(tokens::text_meta()),
            )
            .wrap()
            .selectable(false),
        );
    });
}
