//! The search while it is open: the field, the list under it, and the
//! keys that choose and do a result.

use super::{SearchItem, SearchTarget};
use crate::theme;
use crate::workbench::{Workbench, WorkbenchAction};
use egui::{RichText, Ui};

/// The list's width, in points.
const WIDTH: f32 = 560.0;

impl Workbench {
    /// The search list, at the top middle of the window, while it's open.
    pub(crate) fn search_list(&mut self, ui: &Ui, actions: &mut Vec<WorkbenchAction>) {
        if !self.search.open {
            return;
        }
        let shown = self.search_results();
        // Keys: Up and Down choose, Enter does it, Esc closes.
        let (up, down, enter, escape) = ui.input_mut(|i| {
            (
                i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp),
                i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown),
                i.consume_key(egui::Modifiers::NONE, egui::Key::Enter),
                i.consume_key(egui::Modifiers::NONE, egui::Key::Escape),
            )
        });
        if down && !shown.is_empty() {
            self.search.chosen = (self.search.chosen + 1).min(shown.len() - 1);
        }
        if up {
            self.search.chosen = self.search.chosen.saturating_sub(1);
        }
        self.search.chosen = self.search.chosen.min(shown.len().saturating_sub(1));
        let screen = ui.ctx().content_rect();
        let at = egui::pos2(screen.center().x - WIDTH / 2.0, screen.top() + 44.0);
        let mut picked: Option<SearchTarget> = None;
        let shown_area = egui::Area::new(egui::Id::new("search"))
            .order(egui::Order::Foreground)
            .fixed_pos(at)
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style())
                    .inner_margin(egui::Margin::same(8))
                    .show(ui, |ui| {
                        ui.set_width(WIDTH);
                        let field = ui.add(
                            egui::TextEdit::singleline(&mut self.search.query)
                                .hint_text("Search actions, files, entities and panels")
                                .desired_width(WIDTH),
                        );
                        field.widget_info(|| {
                            egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, "Search")
                        });
                        if self.search.focus {
                            field.request_focus();
                            self.search.focus = false;
                        }
                        if field.changed() {
                            self.search.chosen = 0;
                        }
                        ui.add_space(4.0);
                        if shown.is_empty() {
                            ui.label(
                                RichText::new(format!(
                                    "Nothing is called “{}”.",
                                    self.search.query.trim()
                                ))
                                .color(theme::text_weak()),
                            );
                        }
                        egui::ScrollArea::vertical()
                            .max_height(360.0)
                            .show(ui, |ui| {
                                for (n, item) in shown.iter().enumerate() {
                                    let chosen = n == self.search.chosen;
                                    let row = result_row(ui, item, chosen);
                                    if chosen && (down || up) {
                                        row.scroll_to_me(None);
                                    }
                                    if row.clicked() {
                                        picked = Some(item.target.clone());
                                    }
                                }
                            });
                    });
            });
        if enter && let Some(item) = shown.get(self.search.chosen) {
            picked = Some(item.target.clone());
        }
        let clicked_outside = ui.input(|i| {
            i.pointer.any_pressed()
                && i.pointer
                    .interact_pos()
                    .is_some_and(|p| !shown_area.response.rect.contains(p))
        });
        if let Some(target) = picked {
            self.search.open = false;
            self.do_search(target, ui.ctx(), actions);
        } else if escape || clicked_outside {
            self.search.open = false;
        }
    }
}

/// One result's row: its name, greyed detail, shortcut and kind; lit when
/// chosen.
fn result_row(ui: &mut Ui, item: &SearchItem, chosen: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(WIDTH, 24.0), egui::Sense::click());
    let painter = ui.painter();
    if chosen || response.hovered() {
        painter.rect_filled(
            rect,
            4.0,
            ui.visuals()
                .selection
                .bg_fill
                .gamma_multiply(if chosen { 0.6 } else { 0.3 }),
        );
    }
    let font = egui::FontId::proportional(13.0);
    let name = painter.text(
        rect.left_center() + egui::vec2(8.0, 0.0),
        egui::Align2::LEFT_CENTER,
        &item.name,
        font,
        theme::text_strong(),
    );
    if !item.detail.is_empty() {
        painter.text(
            name.right_center() + egui::vec2(8.0, 0.0),
            egui::Align2::LEFT_CENTER,
            &item.detail,
            egui::FontId::proportional(12.0),
            theme::text_weak(),
        );
    }
    let kind = painter.text(
        rect.right_center() - egui::vec2(8.0, 0.0),
        egui::Align2::RIGHT_CENTER,
        item.kind.label(),
        egui::FontId::proportional(11.0),
        theme::text_weak(),
    );
    if !item.keys.is_empty() {
        painter.text(
            kind.left_center() - egui::vec2(12.0, 0.0),
            egui::Align2::RIGHT_CENTER,
            item.keys,
            egui::FontId::monospace(11.5),
            theme::text(),
        );
    }
    let label = item.name.clone();
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &label));
    response
}
