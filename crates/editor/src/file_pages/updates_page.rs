//! File › Updates (round 2's `10-file-updates`): the notice card while a
//! newer build is out, this copy, the newest build with Check now,
//! looking for updates at start, the releases page and the changes list,
//! and beside them what the newer build brings.

use super::{ProgramFile, page_head, plain, section, sentence};
use crate::tokens::{self, size, text};
use crate::updates::UpdateNotice as N;
use crate::widgets::{self, button::Button, rows, toggle};
use crate::workbench::{Workbench, WorkbenchAction};
use egui::{Align, Layout, Ui};

/// The notice card's words: a small heading, a big line and what it
/// means, while a newer build is out (none otherwise).
fn notice_words(update: &N, name: &str) -> Option<(String, String)> {
    Some(match update {
        N::Available(b) => (
            format!("Build {b}"),
            "Out now: download it while you work; it installs when you close or restart.".into(),
        ),
        N::Downloading(b) => (format!("Build {b}"), "Downloading…".into()),
        N::Ready(b) => (
            format!("Build {b}"),
            format!(
                "Downloaded and checked. {name} closes (asking about unsaved work first), \
                 updates and opens again. Your projects aren't touched."
            ),
        ),
        N::NotInstalled(b) => (
            format!("Build {b}"),
            "This copy wasn't installed with the installer, so it can't update itself: get the \
             new build from the releases page."
                .into(),
        ),
        _ => return None,
    })
}

/// What the Newest build row says of the newest build.
fn newest_words(update: &N) -> (String, String) {
    match update {
        N::UpToDate(b) => (format!("build {b}"), "You have it.".into()),
        N::Checking => ("…".into(), "Checking: asking GitHub.".into()),
        N::NotChecked => ("—".into(), "Not checked yet: Check now asks GitHub.".into()),
        N::FromSource => (
            "—".into(),
            "Copies built from source don't update themselves; installed copies do.".into(),
        ),
        N::Failed(why) => ("—".into(), format!("Couldn't check: {why}")),
        other => match other.newer_build() {
            Some(b) => (format!("build {b}"), "Newer than this copy.".into()),
            None => ("—".into(), String::new()),
        },
    }
}

impl Workbench {
    pub(super) fn updates_page(&mut self, ui: &mut Ui, asked: &mut Vec<WorkbenchAction>) {
        page_head(
            ui,
            "Updates",
            "New builds come from GitHub. Your projects and settings are kept through every \
             update.",
            |_| {},
        );
        ui.add_space(size::PAGE_BLOCK_GAP);
        self.notice_card(ui, asked);
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                super::with_side(
                    ui,
                    self,
                    |ui, w| w.update_rows(ui, asked),
                    |ui, w| w.whats_new(ui),
                );
            });
    }

    /// The card across the top while a newer build is out: Not now and
    /// the next step (Download, Restart to update, the releases page).
    fn notice_card(&mut self, ui: &mut Ui, asked: &mut Vec<WorkbenchAction>) {
        let Some((big, under)) = notice_words(&self.update, &self.info.name) else {
            return;
        };
        if let N::Available(build) = self.update
            && self.file_pages.update_hidden == Some(build)
        {
            return;
        }
        let accent = self.accent;
        widgets::card::frame(ui)
            .fill(tokens::select_item(accent))
            .stroke(egui::Stroke::new(1.0, tokens::open_card_border(accent)))
            .inner_margin(egui::Margin::symmetric(16, 14))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = size::GAP;
                        ui.add(
                            egui::Label::new(text::caps(
                                "New build",
                                text::SECTION_CAPS,
                                crate::interface_look::accent_fill(),
                            ))
                            .selectable(false),
                        );
                        plain(ui, &big, text::page_title(), tokens::text_emphasis());
                        plain(ui, &under, text::page_subtitle(), tokens::text_body());
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        self.next_step(ui, asked);
                    });
                });
            });
        ui.add_space(size::PAGE_BLOCK_GAP);
    }

    /// The notice card's buttons.
    fn next_step(&mut self, ui: &mut Ui, asked: &mut Vec<WorkbenchAction>) {
        let step = match &self.update {
            N::Available(_) => Some(("Download", WorkbenchAction::DownloadUpdate)),
            N::Ready(_) => Some(("Restart to update", WorkbenchAction::Quit)),
            N::NotInstalled(_) => Some(("Open the releases page", WorkbenchAction::OpenUpdatePage)),
            _ => None,
        };
        if let Some((label, action)) = step
            && Button::primary(label).header().show(ui).clicked()
        {
            // Restart to update: the window closes to install it.
            self.restarting = action == WorkbenchAction::Quit;
            asked.push(action);
        }
        if let N::Available(build) = self.update
            && Button::plain("Not now").header().show(ui).clicked()
        {
            self.file_pages.update_hidden = Some(build);
        }
    }

    /// This copy, the newest build, looking at start, and the pages
    /// elsewhere.
    fn update_rows(&mut self, ui: &mut Ui, asked: &mut Vec<WorkbenchAction>) {
        let name = self.info.name.clone();
        let version = self.info.version.clone();
        section(ui, "This copy");
        let mine = self
            .this_build
            .map_or_else(|| "built from source".to_string(), |b| format!("build {b}"));
        rows::setting_row(
            ui,
            "You have",
            &format!("The build this copy of {name} is."),
            |ui| {
                plain(
                    ui,
                    &format!("{name} {version}"),
                    text::row(),
                    tokens::text_meta(),
                );
                plain(ui, &mine, text::mono(), tokens::text_primary());
            },
        );
        let (newest, said) = newest_words(&self.update);
        let published = self
            .release_notes
            .as_ref()
            .filter(|n| Some(n.build) == self.update.newer_build())
            .and_then(|n| n.published.clone())
            .map(|day| format!("Published {day}. "))
            .unwrap_or_default();
        let busy = matches!(self.update, N::Checking | N::Downloading(_));
        rows::setting_row(ui, "Newest build", &format!("{published}{said}"), |ui| {
            let check = ui
                .add_enabled_ui(!busy, |ui| widgets::button::link(ui, "Check now"))
                .inner;
            if check
                .on_hover_text("Asks GitHub for the newest build now")
                .clicked()
            {
                asked.push(WorkbenchAction::CheckForUpdates);
            }
            plain(ui, &newest, text::mono(), tokens::text_primary());
        });
        let mut auto = self.auto_update;
        let look = format!("Look for updates when {name} starts");
        rows::setting_row(
            ui,
            &look,
            "Asks GitHub for a newer build at each start. Only the program's name and version \
             are sent.",
            |ui| toggle::toggle(ui, &mut auto, &look),
        );
        if auto != self.auto_update {
            self.auto_update = auto;
            asked.push(WorkbenchAction::SetAutoUpdate(auto));
        }
        section(ui, "Elsewhere");
        let releases = super::button_row(
            ui,
            "The releases page",
            "Every build, with its setup file.",
            "Open the releases page",
            true,
        );
        if releases.clicked() {
            asked.push(WorkbenchAction::OpenUpdatePage);
        }
        let changes = super::button_row(
            ui,
            "Every change, build by build",
            "The full list of what changed.",
            "Open the changes list",
            true,
        );
        if changes.clicked() {
            asked.push(WorkbenchAction::OpenProgramFile(ProgramFile::Changes));
        }
    }

    /// What the newer build brings, as its release says, and what an
    /// update does to projects.
    fn whats_new(&self, ui: &mut Ui) {
        ui.add_space(size::SECTION_GAP);
        let notes = self
            .release_notes
            .as_ref()
            .filter(|n| Some(n.build) == self.update.newer_build());
        if let Some(notes) = notes {
            let title = format!("What's new in build {}", notes.build);
            widgets::card::card(ui, &title, |ui| {
                ui.set_width(ui.available_width());
                let text = if notes.notes.is_empty() {
                    "Its release says nothing more: see the changes list."
                } else {
                    notes.notes.as_str()
                };
                for line in text.lines().filter(|l| !l.trim().is_empty()) {
                    let line = line.trim_start_matches("> ");
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(line)
                                .font(text::row())
                                .color(tokens::text_body()),
                        )
                        .wrap(),
                    );
                }
            });
        }
        sentence(
            ui,
            "Projects opened with an older build open in a newer one. Files laid out the old way \
             are upgraded when saved, with a backup copy first, so you can go back.",
        );
    }
}
