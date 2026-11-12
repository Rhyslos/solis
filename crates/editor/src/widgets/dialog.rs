//! The shared dialog card (mock-up round 1, `10-dialog-unsaved.png` and
//! `10-dialog-keep-changes.png`): a card over the dimmed window with a
//! title, what is being asked, and its answers in a row: the ones that
//! set something aside at the left, Cancel and the main answer at the
//! right, the main one in the accent. Every question the editor asks
//! that stops the work (unsaved changes, a file changed on disk, keep or
//! undo what was changed while playing) is this card.

use super::button::Button;
use crate::tokens::{self, size, text};
use egui::{Align, Frame, Layout, Margin, RichText, Stroke, Ui};

/// One answer to a dialog.
#[derive(Debug, Clone, Copy)]
pub struct Answer<'a> {
    /// Its words.
    pub label: &'a str,
    /// The main answer: in the accent, at the far right.
    pub main: bool,
    /// At the left of the row (an answer that sets the work aside).
    pub left: bool,
    /// What it does, on hover (empty: nothing).
    pub hint: &'a str,
}

impl<'a> Answer<'a> {
    /// An answer at the right.
    pub fn right(label: &'a str) -> Self {
        Self {
            label,
            main: false,
            left: false,
            hint: "",
        }
    }

    /// The main answer.
    pub fn main(label: &'a str) -> Self {
        Self {
            main: true,
            ..Self::right(label)
        }
    }

    /// An answer at the left.
    pub fn left(label: &'a str) -> Self {
        Self {
            left: true,
            ..Self::right(label)
        }
    }

    /// With what it does on hover.
    pub fn hint(self, hint: &'a str) -> Self {
        Self { hint, ..self }
    }
}

/// The width of a dialog card (from the mock-up).
const WIDTH: f32 = 420.0;

/// Shows the dialog called `id` with `title`, what `words` draws under
/// it, and `answers`. Returns the index of the answer clicked, if one
/// was.
pub fn dialog(
    ctx: &egui::Context,
    id: &str,
    title: &str,
    words: impl FnOnce(&mut Ui),
    answers: &[Answer<'_>],
) -> Option<usize> {
    let frame = Frame::new()
        .fill(tokens::panel())
        .stroke(Stroke::new(1.0, tokens::pill_border()))
        .corner_radius(size::radius::CARD)
        .inner_margin(Margin::same(size::DIALOG_PAD as i8));
    let mut chosen = None;
    egui::Modal::new(egui::Id::new(id))
        .frame(frame)
        .show(ctx, |ui| {
            ui.set_width(WIDTH);
            ui.spacing_mut().item_spacing.y = size::CARD_INNER_GAP;
            ui.label(
                RichText::new(title)
                    .font(text::dialog_title())
                    .color(tokens::text_emphasis()),
            );
            ui.style_mut().override_font_id = Some(text::ui());
            ui.visuals_mut().override_text_color = Some(tokens::text_body());
            words(ui);
            ui.add_space(size::CARD_INNER_GAP);
            ui.horizontal(|ui| {
                for (index, answer) in answers.iter().enumerate().filter(|(_, a)| a.left) {
                    if button(ui, answer) {
                        chosen = Some(index);
                    }
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    for (index, answer) in answers.iter().enumerate().rev().filter(|(_, a)| !a.left)
                    {
                        if button(ui, answer) {
                            chosen = Some(index);
                        }
                    }
                });
            });
        });
    chosen
}

/// One answer's button. Returns whether it was clicked.
fn button(ui: &mut Ui, answer: &Answer<'_>) -> bool {
    let button = if answer.main {
        Button::primary(answer.label)
    } else {
        Button::plain(answer.label)
    };
    let response = button.show(ui);
    let response = if answer.hint.is_empty() {
        response
    } else {
        response.on_hover_text(answer.hint)
    };
    response.clicked()
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable as _;

    #[test]
    fn answers_set_aside_stand_left_and_the_main_one_far_right() {
        let mut clicked = None;
        let mut h = Harness::builder()
            .with_size(egui::vec2(900.0, 600.0))
            .build_ui_state(
                |ui, clicked: &mut Option<usize>| {
                    if !crate::test_support::fonts_ready(ui.ctx()) {
                        return;
                    }
                    let answers = [
                        Answer::left("Don't save"),
                        Answer::right("Cancel"),
                        Answer::main("Save"),
                    ];
                    let words =
                        |ui: &mut Ui| drop(ui.label("Docks has changes that aren't saved."));
                    if let Some(i) = dialog(ui.ctx(), "test", "Unsaved changes", words, &answers) {
                        *clicked = Some(i);
                    }
                },
                clicked,
            );
        h.run();
        h.run();
        h.run();
        let at = |h: &Harness<'_, Option<usize>>, label: &str| h.get_by_label(label).rect();
        let (dont, cancel, save) = (at(&h, "Don't save"), at(&h, "Cancel"), at(&h, "Save"));
        assert!(dont.right() < cancel.left() && cancel.right() < save.left());
        assert!(h.query_by_label("Unsaved changes").is_some());
        h.get_by_label("Save").click();
        h.run();
        clicked = *h.state();
        assert_eq!(clicked, Some(2));
    }
}
