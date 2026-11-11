//! The design's card: a lifted box with a title, holding a few rows (a
//! component in Properties, a module, a details block).

use crate::tokens::{self, size, text};
use egui::{Align, CornerRadius, Frame, InnerResponse, Layout, Margin, RichText, Stroke, Ui};

/// The frame every card is drawn in: the card's fill, a 1 px border,
/// corners of 8 and padding of 10.
pub fn frame(ui: &Ui) -> Frame {
    Frame::new()
        .fill(tokens::card())
        .stroke(Stroke::new(super::hairline(ui), tokens::card_border()))
        .corner_radius(CornerRadius::same(size::radius::CARD))
        .inner_margin(Margin::same(size::CARD_PAD as i8))
}

/// A card titled `title`, with what `contents` draws under it.
pub fn card<R>(ui: &mut Ui, title: &str, contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
    frame(ui).show(ui, |ui| {
        // A card's rows run downwards, whatever it is laid out in.
        ui.with_layout(Layout::top_down(Align::Min), |ui| {
            ui.spacing_mut().item_spacing.y = size::CARD_INNER_GAP;
            ui.add(
                egui::Label::new(
                    RichText::new(title)
                        .font(text::header())
                        .color(tokens::text_primary()),
                )
                .selectable(false),
            );
            contents(ui)
        })
        .inner
    })
}
