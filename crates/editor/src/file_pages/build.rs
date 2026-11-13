//! File › Project settings › Build (round 2's `12-project-settings-build`):
//! Export for playtesting ([decision 0093](../../../../docs/decisions/0093-the-playtest-export.md)),
//! once a window of its own. The game's name, the area it starts in and
//! its window are the project's (kept in it by each export); the folder
//! is this computer's. Export makes the folder, beside the card that
//! says how the last one went.

use super::settings::Setting;
use super::{plain, sentence};
use crate::export_dialog::{game_name, usable_area};
use crate::tokens::{self, size, text};
use crate::unbuilt::{self, UnbuiltId};
use crate::widgets::{self, button::Button, field, segmented::segmented};
use crate::workbench::{Workbench, WorkbenchAction};
use egui::Ui;

impl Workbench {
    /// The control of one of Build's settings (and of General's Game
    /// starts in, the same setting).
    pub(super) fn build_control(
        &mut self,
        ui: &mut Ui,
        setting: Setting,
        asked: &mut Vec<WorkbenchAction>,
    ) {
        if self.open.export.is_none() {
            self.prepare_export();
        }
        let areas = self.areas();
        let Some(form) = &mut self.open.export else {
            return;
        };
        let mut export_now = false;
        match setting {
            Setting::GameName => {
                field::text_field(
                    ui,
                    "Game's name",
                    &mut form.playtest.name,
                    "",
                    size::SETTING_FIELD_W,
                )
                .on_hover_text("The program's name, the window's title and the saves folder's");
            }
            Setting::StartsIn => {
                let words: Vec<(usize, &str)> = areas
                    .iter()
                    .enumerate()
                    .map(|(n, a)| (n, a.trim_end_matches(".solarea")))
                    .collect();
                if words.is_empty() {
                    plain(
                        ui,
                        "No area: save one first",
                        text::row(),
                        tokens::text_meta(),
                    );
                    return;
                }
                let mut chosen = areas
                    .iter()
                    .position(|a| *a == form.playtest.area)
                    .unwrap_or(0);
                let before = chosen;
                field::dropdown(
                    ui,
                    "Game starts in",
                    &mut chosen,
                    &words,
                    size::SETTING_FIELD_W,
                );
                if chosen != before || form.playtest.area.is_empty() {
                    form.playtest.area = areas[chosen].clone();
                }
            }
            Setting::Window => {
                let sizes = crate::export_dialog::sizes();
                let words: Vec<([u32; 2], String)> = sizes
                    .iter()
                    .map(|s| (*s, format!("{} × {}", s[0], s[1])))
                    .collect();
                let choices: Vec<([u32; 2], &str)> =
                    words.iter().map(|(s, w)| (*s, w.as_str())).collect();
                field::dropdown(
                    ui,
                    "Window size",
                    &mut form.playtest.size,
                    &choices,
                    size::KEY_COLUMN_W,
                );
                let modes = [(false, "Windowed"), (true, "Fullscreen")];
                segmented(ui, "Window", &mut form.playtest.fullscreen, &modes);
            }
            Setting::ExportFolder => {
                let ready = form.ready();
                let label = if form.busy { "Exporting…" } else { "Export" };
                let export = ui
                    .add_enabled_ui(ready, |ui| Button::primary(label).show(ui))
                    .inner;
                let export = if form.busy {
                    export.on_disabled_hover_text("Exporting: wait for it to finish")
                } else if !usable_area(&form.playtest.area) {
                    export.on_disabled_hover_text("Choose the area the game starts in first")
                } else {
                    export.on_disabled_hover_text("Choose a folder to export to first")
                };
                export_now = export.clicked();
                if widgets::button::icon(ui, crate::icons::Icon::FolderOpen, "Choose the folder")
                    .clicked()
                {
                    asked.push(WorkbenchAction::ChooseExportFolder);
                }
                field::text_field(
                    ui,
                    "Into the folder",
                    &mut form.folder,
                    "A folder to export to",
                    size::KEY_COLUMN_W,
                )
                .on_hover_text(
                    "Made for the export. A folder that has files in already isn't used: the \
                         suggested one gets a number, (2), and one you chose is refused",
                );
            }
            _ => {}
        }
        if export_now && let Some(request) = self.export_from_form() {
            asked.push(WorkbenchAction::ExportPlaytest(request));
        }
    }

    /// The card beside Build: what an export holds, and how the last one
    /// went.
    pub(super) fn export_card(&mut self, ui: &mut Ui) {
        let Some(form) = &self.open.export else {
            return;
        };
        let name = game_name(&form.playtest.name);
        let program = self.info.name.clone();
        let unsaved = self.open.document.is_dirty();
        let done = form.done.clone();
        let mut open = None;
        widgets::card::card(ui, "Export for playtesting", |ui| {
            ui.set_width(ui.available_width());
            sentence(
                ui,
                &format!(
                    "A folder to zip and send. Your friend runs {name}.exe inside it: no {program} \
                     needed. It packs the program, your code, areas, sprites, sounds and screens; \
                     saves and settings go in the player's Saved Games/{name} folder."
                ),
            );
            if unsaved {
                plain(
                    ui,
                    "The open area has changes that aren't saved: the export holds it as last saved.",
                    text::row(),
                    tokens::WARNING,
                );
            }
            match &done {
                Some(Ok(folder)) => {
                    plain(
                        ui,
                        &format!("Exported to {}.", folder.display()),
                        text::row(),
                        tokens::text_primary(),
                    );
                    if Button::plain("Open folder").show(ui).clicked() {
                        open = Some(folder.clone());
                    }
                }
                Some(Err(why)) => drop(plain(
                    ui,
                    &format!("Not exported: {why}."),
                    text::row(),
                    tokens::ERROR_TEXT,
                )),
                None => {}
            }
        });
        ui.add_space(size::CARD_GAP);
        if let Some(folder) = open {
            self.open.outbox.push(WorkbenchAction::ShowFolder(folder));
        }
    }
}

impl Workbench {
    /// The Build tab's page (round 3, `10-build-tab`): Export for
    /// playtesting in the File pages' pattern, in `rect` (where the
    /// shell put `Panel::Export`), with what goes in the folder and the
    /// unfinished targets beside it.
    pub(crate) fn build_page(
        &mut self,
        ui: &mut Ui,
        rect: egui::Rect,
        asked: &mut Vec<WorkbenchAction>,
    ) {
        let [pad_y, pad_x] = size::FILE_PAGE_PAD;
        let mut page = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(rect.shrink2(egui::vec2(pad_x, pad_y)))
                .id_salt("build page")
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        page.set_clip_rect(rect.intersect(ui.clip_rect()));
        ui.painter().rect_filled(rect, 0.0, tokens::shelf_bar());
        if self.open.project.is_none() {
            plain(
                &mut page,
                "Open or make a project first (File): an export is made from a project.",
                text::ui(),
                tokens::text_meta(),
            );
            return;
        }
        if self.open.export.is_none() {
            self.prepare_export();
        }
        let name = self
            .open
            .export
            .as_ref()
            .map_or_else(String::new, |f| game_name(&f.playtest.name));
        let subtitle = format!(
            "A folder to zip and send. Your friend runs {name}.exe inside it: no {} needed.",
            self.info.name
        );
        super::page_head(&mut page, "Export for playtesting", &subtitle, |_| {});
        page.add_space(size::PAGE_BLOCK_GAP);
        egui::ScrollArea::vertical()
            .id_salt("build page rows")
            .auto_shrink([false, false])
            .show(&mut page, |ui| {
                super::with_side(
                    ui,
                    &mut (self, asked),
                    |ui, (w, asked)| {
                        for setting in [
                            Setting::GameName,
                            Setting::StartsIn,
                            Setting::ExportFolder,
                            Setting::Window,
                        ] {
                            w.setting(ui, setting, asked);
                        }
                    },
                    |ui, (w, _)| {
                        w.export_card(ui);
                        if !unbuilt::hidden() {
                            unbuilt::card(ui, UnbuiltId::BuildTargets);
                        }
                    },
                );
            });
    }
}
