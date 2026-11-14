//! File › About (round 2's `10-file-about`): the program's name, version
//! and build, what it is, its licence and the notices, and the way to
//! Updates and the user guide. Every name on it is read from `AppInfo`.

use super::{Page, ProgramFile, page_head, plain, section};
use crate::tokens::{self, size, text};
use crate::widgets::{self, rows};
use crate::workbench::{Workbench, WorkbenchAction};
use egui::{Sense, Ui, vec2};

/// The logo's size on About.
const LOGO: f32 = 72.0;

impl Workbench {
    pub(super) fn about_page(&mut self, ui: &mut Ui, asked: &mut Vec<WorkbenchAction>) {
        page_head(ui, "About", "The program you are running.", |_| {});
        ui.add_space(size::PAGE_BLOCK_GAP);
        let width = ui.available_width().min(size::SETTINGS_MAX_W);
        ui.allocate_ui(vec2(width, ui.available_height()), |ui| {
            self.about_card(ui);
            let name = self.info.name.clone();
            section(ui, "Licences");
            let licence = super::button_row(
                ui,
                &format!("{name}'s licence"),
                "MIT: use it, change it and share it freely.",
                "Open LICENSE",
                false,
            );
            if licence.clicked() {
                asked.push(WorkbenchAction::OpenProgramFile(ProgramFile::Licence));
            }
            let notices = super::button_row(
                ui,
                "Open-source libraries and fonts",
                &format!(
                    "The libraries built into {name} and the fonts it draws with, each with its \
                     licence."
                ),
                "Open the notices",
                false,
            );
            if notices.clicked() {
                asked.push(WorkbenchAction::OpenProgramFile(ProgramFile::Notices));
            }
            section(ui, "More");
            let updates = rows::setting_row(
                ui,
                "Updates",
                "This build, the newest build and what it brings.",
                |ui| widgets::button::link(ui, "Go to Updates"),
            );
            if updates.clicked() {
                self.show_file_page(Page::Updates);
            }
            if super::button_row(
                ui,
                "The user guide",
                "Every part of the editor, explained.",
                "Open the user guide",
                true,
            )
            .clicked()
            {
                asked.push(WorkbenchAction::OpenGuide(None));
            }
        });
    }

    /// The card at the top: the logo, the name, the version and build,
    /// and the two sentences that say what it is.
    fn about_card(&self, ui: &mut Ui) {
        widgets::card::frame(ui)
            .fill(tokens::panel())
            .corner_radius(egui::CornerRadius::same(size::radius::PROJECT_CARD))
            .inner_margin(egui::Margin::same(22))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 20.0;
                    match self.logo {
                        Some(logo) => {
                            ui.add(egui::Image::new((logo, vec2(LOGO, LOGO))));
                        }
                        None => {
                            ui.allocate_exact_size(vec2(LOGO, LOGO), Sense::hover());
                        }
                    }
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = size::GAP;
                        plain(
                            ui,
                            &self.info.name,
                            text::page_title(),
                            tokens::text_emphasis(),
                        );
                        let build = self.this_build.map_or_else(
                            || "built from source".to_string(),
                            |b| format!("build {b}"),
                        );
                        let version = format!("Version {} · {build}", self.info.version);
                        plain(ui, &version, text::mono(), tokens::text_label());
                        plain(
                            ui,
                            "A free, open-source 3D game engine.",
                            text::page_subtitle(),
                            tokens::text_body(),
                        );
                        plain(
                            ui,
                            "Free software under the MIT licence.",
                            text::page_subtitle(),
                            tokens::text_body(),
                        );
                    });
                });
            });
    }
}
