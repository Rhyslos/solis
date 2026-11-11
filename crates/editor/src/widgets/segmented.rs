//! The design's segmented control: a few choices side by side in a dark
//! well, the chosen one lifted on a lighter fill.

use super::{arrow_step, border, focus_ring, width_of, words};
use crate::tokens::{self, size, text};
use egui::{Align2, CornerRadius, Id, Rect, Sense, Ui, WidgetInfo, pos2, vec2};

/// A segmented control over `choices` (each a value and its word), with
/// `chosen` lifted. `name` tells this control from others in the same
/// place. Returns whether another choice was made: by a click, by Space
/// or Enter on a segment, or by the arrow keys while a segment holds the
/// keyboard.
pub fn segmented<T: Copy + PartialEq>(
    ui: &mut Ui,
    name: &str,
    chosen: &mut T,
    choices: &[(T, &str)],
) -> bool {
    let font = text::medium_at(text::SMALL_SIZE);
    let widths: Vec<f32> = choices
        .iter()
        .map(|(_, word)| width_of(ui, word, font.clone()) + 2.0 * size::SEGMENT_PAD)
        .collect();
    let well = vec2(
        widths.iter().sum::<f32>() + 2.0 * size::SEGMENT_WELL_PAD,
        size::SEGMENT_H + 2.0 * size::SEGMENT_WELL_PAD,
    );
    let (rect, _) = ui.allocate_exact_size(well, Sense::hover());
    let visible = ui.is_rect_visible(rect);
    if visible {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(size::radius::TAB), tokens::app());
        border(ui, rect, size::radius::TAB, tokens::line());
    }
    let before = *chosen;
    let ids: Vec<Id> = (0..choices.len())
        .map(|index| ui.id().with(("segment", name, index)))
        .collect();
    // Where the arrow keys sent the keyboard: it goes there once every
    // segment is drawn, so no later segment takes the same key press.
    let mut moved_to = None;
    let mut left = rect.left() + size::SEGMENT_WELL_PAD;
    for (index, ((value, word), width)) in choices.iter().zip(&widths).enumerate() {
        let segment = Rect::from_min_size(
            pos2(left, rect.top() + size::SEGMENT_WELL_PAD),
            vec2(*width, size::SEGMENT_H),
        );
        left += width;
        let response = ui.interact(segment, ids[index], Sense::click());
        if response.clicked() {
            *chosen = *value;
        }
        // The arrows move the choice, and the keyboard goes with it.
        let step = arrow_step(ui, &response);
        let next = index as i32 + step;
        if step != 0 && (0..choices.len() as i32).contains(&next) {
            *chosen = choices[next as usize].0;
            moved_to = Some(ids[next as usize]);
        }
        let on = *chosen == *value;
        response.widget_info(|| {
            WidgetInfo::selected(egui::WidgetType::RadioButton, ui.is_enabled(), on, *word)
        });
        if !visible {
            continue;
        }
        let ink = if on {
            ui.painter().rect_filled(
                segment,
                CornerRadius::same(size::radius::SMALL),
                tokens::field_border(),
            );
            tokens::text_emphasis()
        } else if response.hovered() {
            tokens::text_primary()
        } else {
            tokens::text_label()
        };
        words(ui, segment, Align2::CENTER_CENTER, word, font.clone(), ink);
        focus_ring(ui, &response, segment, size::radius::SMALL);
    }
    if let Some(id) = moved_to {
        ui.memory_mut(|memory| memory.request_focus(id));
    }
    *chosen != before
}
