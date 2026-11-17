//! The launcher's column (board 01): the logo, the program's name and
//! version at the top; Projects, Learn and Updates (with how many new
//! builds there are); Preferences and the user guide at the foot.

use super::Page;
use crate::icons::Icon;
use crate::shell::file_page::file_item;
use crate::tokens::{self, launcher as l, size, text};
use crate::widgets::{self, words};
use crate::workbench::{Workbench, WorkbenchAction};
use egui::{Align, Align2, CornerRadius, Layout, Rect, Sense, Ui, pos2, vec2};

impl Workbench {
    /// Draws the column down the launcher's left.
    pub(super) fn launcher_side(&mut self, ui: &mut Ui, asked: &mut Vec<WorkbenchAction>) {
        let [pad_y, pad_x] = l::SIDE_PAD;
        egui::Panel::left("launcher side")
            .exact_size(l::SIDE_W)
            .resizable(false)
            .show_separator_line(false)
            .frame(
                egui::Frame::new()
                    .fill(tokens::ribbon())
                    .inner_margin(egui::Margin::symmetric(pad_x as i8, pad_y as i8)),
            )
            .show(ui, |ui| {
                let whole = ui.max_rect().expand2(vec2(pad_x, pad_y));
                tokens::vline(
                    ui.painter(),
                    whole.right() - size::LINE,
                    whole.y_range(),
                    tokens::line(),
                );
                self.brand(ui);
                ui.add_space(l::BRAND_GAP);
                ui.spacing_mut().item_spacing = vec2(0.0, size::FILE_MENU_GAP);
                for (icon, label, page) in [
                    (Icon::FourSquares, "Projects", Page::Projects),
                    (Icon::Book, "Learn", Page::Learn),
                    (Icon::Import, "Updates", Page::Updates),
                ] {
                    let front = self.launcher.page == page;
                    let item = file_item(ui, icon, label, front, true);
                    if page == Page::Updates && self.update.newer_build().is_some() {
                        new_count(ui, item.rect);
                    }
                    if item.clicked() {
                        self.show_launcher_page(page);
                    }
                }
                ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                    let guide = file_item(ui, Icon::Book, "User guide", false, true)
                        .on_hover_text("Opens the user guide in your browser");
                    let spot = Rect::from_center_size(
                        pos2(
                            guide.rect.right() - size::FILE_MENU_ITEM_PAD,
                            guide.rect.center().y,
                        ),
                        vec2(size::ICON_SMALL, size::ICON_SMALL),
                    );
                    crate::icons::icon(ui.painter(), spot, Icon::External, tokens::text_meta());
                    if guide.clicked() {
                        asked.push(WorkbenchAction::OpenGuide(None));
                    }
                    let front = self.launcher.page == Page::Preferences;
                    if file_item(ui, Icon::Sliders, "Preferences", front, true).clicked() {
                        self.show_launcher_page(Page::Preferences);
                    }
                });
            });
    }

    /// Brings `page` to the front of the launcher; Projects reads the
    /// list afresh.
    pub(crate) fn show_launcher_page(&mut self, page: Page) {
        if page == Page::Projects && self.launcher.page != Page::Projects {
            self.open.outbox.push(WorkbenchAction::ListProjects);
        }
        self.launcher.page = page;
    }

    /// The logo in its light tile, the program's name and its version.
    fn brand(&self, ui: &mut Ui) {
        let height = l::LOGO;
        let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
        let tile = Rect::from_min_size(
            pos2(row.left() + size::GAP, row.top()),
            vec2(l::LOGO, l::LOGO),
        );
        ui.painter().rect_filled(
            tile,
            CornerRadius::same(size::radius::CONTROL),
            tokens::PLAY_FILL,
        );
        // The logo is the app's picture (branding), never drawn here.
        if let Some(logo) = self.logo {
            let inside = tile.shrink(size::LOGO_INSET);
            egui::Image::new((logo, inside.size())).paint_at(ui, inside);
        }
        let left = tile.right() + l::LOGO_GAP;
        let name = Rect::from_min_max(pos2(left, row.top()), pos2(row.right(), row.center().y));
        words(
            ui,
            name,
            Align2::LEFT_BOTTOM,
            &self.info.name,
            text::card_name(),
            tokens::text_emphasis(),
        );
        let version = Rect::from_min_max(pos2(left, row.center().y), row.max);
        words(
            ui,
            version,
            Align2::LEFT_TOP,
            &format!("v{}", self.info.version),
            text::mono_small(),
            tokens::text_meta(),
        );
    }
}

/// "1 new" at the right of Updates while a newer build is out.
fn new_count(ui: &Ui, item: Rect) {
    let font = text::ribbon_small();
    let label = "1 new";
    let width = widgets::width_of(ui, label, font.clone()) + 2.0 * size::GAP;
    let badge = Rect::from_center_size(
        pos2(
            item.right() - size::FILE_MENU_ITEM_PAD - width / 2.0,
            item.center().y,
        ),
        vec2(width, size::STATUS_DOT * 3.0),
    );
    ui.painter().rect_filled(
        badge,
        CornerRadius::same(size::radius::CONTROL),
        crate::interface_look::accent_fill(),
    );
    words(
        ui,
        badge,
        Align2::CENTER_CENTER,
        label,
        font,
        tokens::ON_ACCENT,
    );
}
