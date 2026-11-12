//! Columns side by side inside one shelf tab, each under the shelf's own
//! 30 px header (spec 196-199): its title, quiet words after it, a count
//! or tools at its right, and a hairline between one column and the
//! next. The 2D editors' shelf tabs are these (the approved additions,
//! round 3): the Map tab's layers, terrains, cell values and cell; a
//! tileset's terrains, terrain and slot; a picture's crops and the crop
//! picked; a clip's playback, events and sheet.
//!
//! A column that doesn't fit gives way: the fixed ones shrink together,
//! never below the narrowest a column may be.

use crate::tokens::{self, size, text};
use egui::{Align, Align2, Id, Layout, Rect, Sense, Ui, UiBuilder, WidgetInfo, pos2};

/// How wide a column is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Wide {
    /// This many points.
    Fixed(f32),
    /// What the fixed ones leave (shared, if several).
    Rest,
}

/// One column: its header's words and its width.
#[derive(Debug, Clone, PartialEq)]
pub struct Part {
    /// The title, bright.
    pub title: String,
    /// Quiet words after the title ("Tile map · 48 × 32 cells").
    pub note: String,
    /// How wide it is.
    pub width: Wide,
}

impl Part {
    /// A column titled `title` with `note` after it.
    pub fn new(title: impl Into<String>, note: impl Into<String>, width: Wide) -> Self {
        Self {
            title: title.into(),
            note: note.into(),
            width,
        }
    }
}

/// The widths of `parts` across `room`.
pub(crate) fn widths(parts: &[Part], room: f32) -> Vec<f32> {
    let fixed: f32 = parts
        .iter()
        .map(|p| match p.width {
            Wide::Fixed(w) => w,
            Wide::Rest => 0.0,
        })
        .sum();
    let rests = parts.iter().filter(|p| p.width == Wide::Rest).count();
    let rest_room = rests as f32 * size::COLUMN_MIN_W;
    // Fixed columns shrink together when they and the rest don't fit.
    let squeeze = if fixed > 0.0 && fixed + rest_room > room {
        ((room - rest_room).max(0.0) / fixed).max(0.0)
    } else {
        1.0
    };
    let left = (room - fixed * squeeze).max(0.0);
    parts
        .iter()
        .map(|p| match p.width {
            Wide::Fixed(w) => (w * squeeze).max(size::COLUMN_MIN_W.min(w)),
            Wide::Rest => left / rests.max(1) as f32,
        })
        .collect()
}

/// Lays `parts` side by side over the whole of `ui`: each one's header,
/// with `tools(ui, index)` at its right (laid out right to left), and its
/// body from `body(ui, index)`, which scrolls when it is taller than the
/// column.
pub fn columns(
    ui: &mut Ui,
    parts: &[Part],
    mut tools: impl FnMut(&mut Ui, usize),
    mut body: impl FnMut(&mut Ui, usize),
) {
    let whole = ui.max_rect();
    let mut left = whole.left();
    for (index, (part, width)) in parts.iter().zip(widths(parts, whole.width())).enumerate() {
        let rect = Rect::from_min_max(pos2(left, whole.top()), pos2(left + width, whole.bottom()));
        left += width;
        if index > 0 {
            tokens::vline(ui.painter(), rect.left(), rect.y_range(), tokens::line());
        }
        let below = header(ui, rect, part, |ui| tools(ui, index));
        let mut inside = ui.new_child(
            UiBuilder::new()
                .id_salt(("shelf column", index))
                .max_rect(below)
                .layout(Layout::top_down(Align::Min)),
        );
        inside.set_clip_rect(below.intersect(ui.clip_rect()));
        egui::ScrollArea::vertical()
            .id_salt(("shelf column body", index))
            .auto_shrink(false)
            .show(&mut inside, |ui| {
                egui::Frame::new()
                    .inner_margin(egui::Margin::symmetric(
                        size::ROW_PAD as i8,
                        size::CARD_INNER_GAP as i8,
                    ))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        body(ui, index);
                    });
            });
    }
    ui.advance_cursor_after_rect(whole);
}

/// A column's header in `rect`'s top, as the shelf draws one. Returns the
/// room under it.
fn header(ui: &mut Ui, rect: Rect, part: &Part, tools: impl FnOnce(&mut Ui)) -> Rect {
    let head = rect.with_max_y(rect.top() + size::COLUMN_HEADER_H);
    let painter = ui.painter_at(head);
    painter.rect_filled(head, 0.0, tokens::panel());
    tokens::hline(
        &painter,
        head.x_range(),
        head.bottom() - 1.0,
        tokens::line(),
    );
    // Named for screen readers (and tests) by its title and its note.
    let named = ui.interact(head, Id::new(("part header", &part.title)), Sense::hover());
    let name = if part.note.is_empty() {
        part.title.clone()
    } else {
        format!("{} \u{b7} {}", part.title, part.note)
    };
    named.widget_info(|| WidgetInfo::labeled(egui::WidgetType::Label, true, &name));
    let title = painter.text(
        pos2(head.left() + size::ROW_PAD, head.center().y),
        Align2::LEFT_CENTER,
        &part.title,
        text::header(),
        tokens::text_emphasis(),
    );
    if !part.note.is_empty() {
        painter.text(
            pos2(title.right() + size::GAP, head.center().y),
            Align2::LEFT_CENTER,
            &part.note,
            text::ui(),
            tokens::text_meta(),
        );
    }
    let room = Rect::from_min_max(
        pos2(title.right() + size::GAP, head.top()),
        pos2(head.right() - size::ROW_PAD, head.bottom() - 1.0),
    );
    let mut strip = ui.new_child(
        UiBuilder::new()
            .id_salt(("part tools", &part.title))
            .max_rect(room)
            .layout(Layout::right_to_left(Align::Center)),
    );
    strip.spacing_mut().item_spacing.x = size::GAP;
    tools(&mut strip);
    rect.with_min_y(head.bottom())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_columns_keep_their_width_and_the_rest_takes_what_is_left() {
        let parts = [
            Part::new("a", "", Wide::Fixed(290.0)),
            Part::new("b", "", Wide::Rest),
            Part::new("c", "", Wide::Fixed(300.0)),
        ];
        assert_eq!(widths(&parts, 1440.0), [290.0, 850.0, 300.0]);
    }

    #[test]
    fn a_narrow_shelf_squeezes_the_fixed_columns_and_keeps_the_rest_readable() {
        let parts = [
            Part::new("a", "", Wide::Fixed(400.0)),
            Part::new("b", "", Wide::Rest),
        ];
        let w = widths(&parts, 500.0);
        assert!(w[1] >= size::COLUMN_MIN_W, "{w:?}");
        assert!((w[0] + w[1] - 500.0).abs() < 0.01, "{w:?}");
        // Nothing at all: nothing negative.
        assert!(widths(&parts, 0.0).iter().all(|w| *w >= 0.0));
    }
}
