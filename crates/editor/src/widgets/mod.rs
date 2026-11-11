//! The kit: the design's controls, each drawn exactly as the project
//! lead's design has it (specification, lines 98 to 125, and the screens
//! that show them), from the tokens and nothing else.
//!
//! Every control here:
//!
//! - takes its colours, sizes and type from `tokens`, so it follows the
//!   accent, the density and the interface scale;
//! - can be reached and used from the keyboard: Tab moves to it, a 1 px
//!   accent ring outside it shows where the keyboard is ([`focus_ring`]),
//!   Space or Enter does what a click does, and the arrow keys move
//!   through a row of choices or along a slider;
//! - has a name a screen reader can say, which is also how the tests
//!   find it.
//!
//! The screens take these up one milestone at a time; egui's own
//! controls, styled by `theme.rs`, stay inside the panels that haven't
//! been rebuilt yet.

pub mod button;
pub mod card;
pub mod checkbox;
pub mod chip;
pub mod dashed;
pub mod field;
pub mod keycap;
pub mod pill;
pub mod rows;
pub mod segmented;
pub mod slider;
pub mod swatch;
pub mod tile;
pub mod toggle;

#[cfg(test)]
pub(crate) mod sheet;

use crate::interface_look;
use crate::tokens::{self, size};
use egui::{Align2, Color32, CornerRadius, FontId, Key, Rect, Response, Stroke, StrokeKind, Ui};

/// A line one whole pixel wide at the scale `ui` is drawn at.
pub(crate) fn hairline(ui: &Ui) -> f32 {
    size::LINE * tokens::hairline(ui.ctx().pixels_per_point())
}

/// A 1 px border in `colour`, inside `rect`.
pub(crate) fn border(ui: &Ui, rect: Rect, radius: u8, colour: Color32) {
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(radius),
        Stroke::new(hairline(ui), colour),
        StrokeKind::Inside,
    );
}

/// The ring that shows where the keyboard is: 1 px of the accent, just
/// outside the control (Solis's own; the design shows none). Drawn only
/// while `response` has the keyboard's focus.
pub fn focus_ring(ui: &Ui, response: &Response, rect: Rect, radius: u8) {
    if !response.has_focus() {
        return;
    }
    ui.painter().rect_stroke(
        rect,
        // One further out than the control's own corners.
        CornerRadius::same(radius.saturating_add(1)),
        Stroke::new(hairline(ui), interface_look::accent_fill()),
        StrokeKind::Outside,
    );
}

/// Words drawn in one line, clipped to `rect` rather than spilling out.
pub(crate) fn words(
    ui: &Ui,
    rect: Rect,
    anchor: Align2,
    text: &str,
    font: FontId,
    colour: Color32,
) -> Rect {
    let at = anchor.pos_in_rect(&rect);
    ui.painter()
        .with_clip_rect(rect.intersect(ui.clip_rect()))
        .text(at, anchor, text, font, colour)
}

/// How wide `text` is in `font`.
pub(crate) fn width_of(ui: &Ui, text: &str, font: FontId) -> f32 {
    ui.painter()
        .layout_no_wrap(text.to_owned(), font, Color32::PLACEHOLDER)
        .size()
        .x
}

/// Which way the arrow keys ask to go, while `response` holds the
/// keyboard: -1 for Left or Up, 1 for Right or Down. Holding the
/// keyboard, the control keeps the arrows to itself (otherwise they move
/// to the next control).
pub(crate) fn arrow_step(ui: &Ui, response: &Response) -> i32 {
    if !response.has_focus() {
        return 0;
    }
    ui.memory_mut(|memory| {
        memory.set_focus_lock_filter(
            response.id,
            egui::EventFilter {
                horizontal_arrows: true,
                vertical_arrows: true,
                ..Default::default()
            },
        );
    });
    ui.input(|input| {
        let back = input.key_pressed(Key::ArrowLeft) || input.key_pressed(Key::ArrowUp);
        let on = input.key_pressed(Key::ArrowRight) || input.key_pressed(Key::ArrowDown);
        i32::from(on) - i32::from(back)
    })
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
