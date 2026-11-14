//! **Documentation**, at the tab bar's right end beside the project's
//! name (decision 0112): opens the user guide. It is drawn as the tab
//! bar's tabs that aren't in front are, words alone in their grey that
//! brighten under the pointer, so it reads as part of the bar; it is
//! there on every tab, the File pages included, with or without a
//! project. A click only: it has no key (rule 17).

use super::ShellCommand;
use crate::tokens::{self, size, text};
use crate::widgets::{self, width_of, words};
use crate::workbench::Workbench;
use egui::{Align2, Sense, Ui, WidgetInfo, vec2};

/// What the button says.
pub(crate) const DOCUMENTATION: &str = "Documentation";

impl Workbench {
    /// Draws the button where `ui` lays it out (right to left), and asks
    /// for the guide when it is clicked.
    pub(super) fn documentation_button(&mut self, ui: &mut Ui) {
        let width = width_of(ui, DOCUMENTATION, text::tab()) + 2.0 * size::RIBBON_TAB_PAD;
        let height = size::RIBBON_TAB_H;
        let (rect, response) = ui.allocate_exact_size(vec2(width, height), Sense::click());
        response.widget_info(|| WidgetInfo::labeled(egui::WidgetType::Button, true, DOCUMENTATION));
        if ui.is_rect_visible(rect) {
            let ink = if response.hovered() {
                tokens::text_primary()
            } else {
                tokens::text_label()
            };
            words(
                ui,
                rect,
                Align2::CENTER_CENTER,
                DOCUMENTATION,
                text::tab(),
                ink,
            );
            widgets::focus_ring(ui, &response, rect, size::radius::TAB);
        }
        let response = response.on_hover_text(ShellCommand::OpenGuide.hint().to_string());
        if response.clicked() {
            let mut asked = Vec::new();
            ShellCommand::OpenGuide.run(self, ui.ctx(), &mut asked);
            self.open.outbox.extend(asked);
        }
    }
}
