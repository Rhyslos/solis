//! Home's note in a 2D or 3D project (board 08 of decision 0118): "No
//! Shading tab and no Lighting group: they are 3D only", in the room the
//! ribbon's groups leave before the Play group. It shows, while hints
//! are on, the first few times a project opens as, or is switched to,
//! that kind ([`KIND_NOTE_TIMES`]), and never again once its × is
//! clicked; the count is kept with the interface's look, on this
//! computer.

use super::RibbonTab;
use crate::icons::{self, Icon};
use crate::interface_look::{KIND_NOTE_GONE, KIND_NOTE_TIMES, accent_fill};
use crate::tokens::{self, size, text};
use crate::widgets::{self, width_of};
use crate::workbench::Workbench;
use egui::{Align2, CornerRadius, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

impl Workbench {
    /// The note Home's ribbon shows now, if any: in a 2D or 3D project, on
    /// Home's own ribbon, while hints are on, the first few times.
    pub(crate) fn kind_note(&self) -> Option<&'static str> {
        let seen = self.interface.kind_note_seen;
        let home = self.shell.tab == RibbonTab::Home
            && !self.placing()
            && !self.map_tools_in_front()
            && !self.brush_tools_in_front();
        let shows = home && self.hints_on() && (1..=KIND_NOTE_TIMES).contains(&seen);
        self.open.kind.note().filter(|_| shows)
    }

    /// The note's ×: it never shows again on this computer.
    pub(crate) fn dismiss_kind_note(&mut self) {
        self.interface.kind_note_seen = KIND_NOTE_GONE;
    }
}

/// Draws `words` as the note at the left of `room`, in the middle of its
/// height, if it fits there (a narrow window leaves it out). Returns
/// whether its × was clicked.
pub(super) fn draw(ui: &mut Ui, room: Rect, words: &str) -> bool {
    let font = text::ribbon_small();
    let inside = size::ICON_SMALL + size::PILL_ICON_GAP + width_of(ui, words, font.clone());
    let width = 2.0 * size::PILL_PAD + inside + size::PILL_ICON_GAP + size::ICON_SMALL;
    if width > room.width() {
        return false;
    }
    let rect = Rect::from_min_size(
        pos2(room.left(), room.center().y - size::PILL_H / 2.0),
        vec2(width, size::PILL_H),
    );
    let response = ui.interact(rect, ui.id().with("kind note"), Sense::hover());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, words));
    let painter = ui.painter();
    let round = CornerRadius::same((size::PILL_H / 2.0) as u8);
    let accent = crate::interface_look::in_use().accent;
    painter.rect_filled(rect, round, tokens::changes_pill(accent));
    painter.rect_stroke(
        rect,
        round,
        Stroke::new(size::LINE, accent_fill()),
        StrokeKind::Inside,
    );
    let mut left = rect.left() + size::PILL_PAD;
    let icon_at = |left: f32| {
        Rect::from_min_size(
            pos2(left, rect.center().y - size::ICON_SMALL / 2.0),
            vec2(size::ICON_SMALL, size::ICON_SMALL),
        )
    };
    icons::icon(painter, icon_at(left), Icon::Info, accent_fill());
    left += size::ICON_SMALL + size::PILL_ICON_GAP;
    let words_rect = Rect::from_min_max(pos2(left, rect.top()), rect.max);
    widgets::words(
        ui,
        words_rect,
        Align2::LEFT_CENTER,
        words,
        font,
        tokens::text_primary(),
    );
    let close = icon_at(rect.right() - size::PILL_PAD - size::ICON_SMALL);
    let closed = ui
        .interact(close, ui.id().with("kind note close"), Sense::click())
        .on_hover_text("Don't show this note again");
    closed.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Don't show this note again")
    });
    let ink = if closed.hovered() {
        tokens::text_primary()
    } else {
        tokens::text_meta()
    };
    icons::icon(ui.painter(), close, Icon::Close, ink);
    closed.clicked()
}
