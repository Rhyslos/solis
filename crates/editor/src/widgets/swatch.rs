//! The design's colour swatches: 22 × 22 rounded squares in a row, the
//! chosen one ringed in its own colour with a gap of the page between.

use super::{arrow_step, focus_ring};
use crate::tokens::size;
use egui::{Color32, CornerRadius, Id, Sense, Ui, WidgetInfo, vec2};

/// A row of swatches over `choices` (each a value, its colour and its
/// name), with `chosen` ringed. `page` is the colour behind the row: the
/// gap inside the ring is drawn in it. Returns whether another was chosen:
/// by a click, by Space or Enter, or by the arrow keys.
pub fn swatches<T: Copy + PartialEq>(
    ui: &mut Ui,
    name: &str,
    chosen: &mut T,
    choices: &[(T, Color32, &str)],
    page: Color32,
) -> bool {
    let before = *chosen;
    let ids: Vec<Id> = (0..choices.len())
        .map(|index| ui.id().with(("swatch", name, index)))
        .collect();
    let count = choices.len() as f32;
    let room = vec2(
        count * size::SWATCH + (count - 1.0).max(0.0) * size::SWATCH_GAP,
        size::SWATCH,
    );
    let (row, _) = ui.allocate_exact_size(room, Sense::hover());
    // As in a segmented control: the keyboard follows the arrow keys once
    // every swatch is drawn.
    let mut moved_to = None;
    for (index, (value, colour, word)) in choices.iter().enumerate() {
        let rect = egui::Rect::from_min_size(
            row.min + vec2(index as f32 * (size::SWATCH + size::SWATCH_GAP), 0.0),
            vec2(size::SWATCH, size::SWATCH),
        );
        let response = ui.interact(rect, ids[index], Sense::click());
        if response.clicked() {
            *chosen = *value;
        }
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
        if !ui.is_rect_visible(rect) {
            continue;
        }
        let radius = size::radius::TAB;
        let painter = ui.painter();
        if on {
            // From the outside in: a ring of its own colour, a ring of
            // the page, the swatch.
            let ring = rect.expand(2.0 * size::SWATCH_RING);
            let round = CornerRadius::same(radius + 2 * size::SWATCH_RING as u8);
            painter.rect_filled(ring, round, *colour);
            let gap = rect.expand(size::SWATCH_RING);
            let round = CornerRadius::same(radius + size::SWATCH_RING as u8);
            painter.rect_filled(gap, round, page);
        }
        painter.rect_filled(rect, CornerRadius::same(radius), *colour);
        // Outside the chosen one's own ring, where it has one.
        let outer = rect.expand(2.0 * size::SWATCH_RING);
        focus_ring(ui, &response, outer, radius + 2 * size::SWATCH_RING as u8);
    }
    if let Some(id) = moved_to {
        ui.memory_mut(|memory| memory.request_focus(id));
    }
    *chosen != before
}
