//! What the editor shows about updates ([decision
//! 0043](../../../docs/decisions/0043-installed-and-updated-from-github.md),
//! [decision 0045](../../../docs/decisions/0045-a-splash-screen-that-asks-before-updating.md)):
//! a part of the Help menu, the update question when Help > Check for
//! updates finds a newer build, and a button in the top bar when it's
//! downloaded. When Solis starts, the app asks on the splash screen
//! instead. The app does the checking and installing, and tells the
//! editor how it's going with [`Workbench::set_update`].

use crate::theme;
use crate::workbench::{Workbench, WorkbenchAction};
use egui::{Color32, RichText, Ui};

/// How updating is going, as the app tells the editor.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum UpdateNotice {
    /// Nothing checked yet.
    #[default]
    NotChecked,
    /// Asking GitHub.
    Checking,
    /// This is the newest build (its number).
    UpToDate(u64),
    /// A newer build is out and wasn't taken (its number).
    Available(u64),
    /// A newer build is downloading (its number).
    Downloading(u64),
    /// A newer build is downloaded and checked: it installs when the
    /// program closes, or now with Restart to update (its number).
    Ready(u64),
    /// A newer build exists, but this copy wasn't installed with the
    /// installer, so it can't update itself (its number).
    NotInstalled(u64),
    /// This copy was built from source and doesn't update itself.
    FromSource,
    /// The last check didn't work, and why.
    Failed(String),
}

impl UpdateNotice {
    /// One line for the Help menu.
    pub fn line(&self) -> String {
        match self {
            UpdateNotice::NotChecked => "Not checked yet".into(),
            UpdateNotice::Checking => "Checking…".into(),
            UpdateNotice::UpToDate(build) => format!("Up to date (build {build})"),
            UpdateNotice::Available(build) => format!("Build {build} is available"),
            UpdateNotice::Downloading(build) => format!("Downloading build {build}…"),
            UpdateNotice::Ready(build) => {
                format!("Build {build} installs when you close")
            }
            UpdateNotice::NotInstalled(build) => {
                format!("Build {build} is out: install it to update by itself")
            }
            UpdateNotice::FromSource => "Built from source: no updates".into(),
            UpdateNotice::Failed(why) => format!("Couldn't check: {why}"),
        }
    }
}

impl Workbench {
    /// Tells the editor how updating is going.
    pub fn set_update(&mut self, notice: UpdateNotice) {
        self.update = notice;
    }

    /// Asks the update question (Help > Check for updates found a newer
    /// build); the answer comes back as an action.
    pub fn offer_update(&mut self, offer: crate::splash::UpdateOffer) {
        self.offer = Some(offer);
        self.skip_offer = false;
    }

    /// The update question, while it's asked.
    pub(crate) fn update_question(
        &mut self,
        ctx: &egui::Context,
        actions: &mut Vec<WorkbenchAction>,
    ) {
        use crate::splash::UpdateAnswer;
        let Some(offer) = self.offer.clone() else {
            return;
        };
        let name = self.info.name.clone();
        let Some(answer) =
            crate::splash::update_question(ctx, &name, &offer, &mut self.skip_offer, 0.0)
        else {
            return;
        };
        self.offer = None;
        actions.push(match answer {
            UpdateAnswer::UpdateNow => WorkbenchAction::DownloadUpdate,
            UpdateAnswer::OpenDownloadPage => WorkbenchAction::OpenUpdatePage,
            UpdateAnswer::NotNow { skip } => {
                WorkbenchAction::SkipUpdate(skip.then_some(offer.latest))
            }
        });
    }

    /// How updating is going, as last told.
    pub fn update(&self) -> &UpdateNotice {
        &self.update
    }

    /// Whether Solis checks for a newer build when it starts (Help >
    /// Check for updates when starting). Saved by the app.
    pub fn auto_update(&self) -> bool {
        self.auto_update
    }

    /// Whether the window is closing to install the update and open again
    /// (Restart to update), rather than just closing.
    pub fn restarting(&self) -> bool {
        self.restarting && matches!(self.update, UpdateNotice::Ready(_))
    }

    /// Sets [`Workbench::auto_update`], as saved.
    pub fn set_auto_update(&mut self, on: bool) {
        self.auto_update = on;
    }

    /// The Help menu's part about updates.
    pub(crate) fn updates_menu(&mut self, ui: &mut Ui, actions: &mut Vec<WorkbenchAction>) {
        let checking = matches!(
            self.update,
            UpdateNotice::Checking | UpdateNotice::Downloading(_)
        );
        let check = ui
            .add_enabled(!checking, egui::Button::new("Check for updates"))
            .on_hover_text(
                "Asks GitHub for the newest build now, and asks you whether to update if \
                 there is one (even one you skipped)",
            );
        if check.clicked() {
            actions.push(WorkbenchAction::CheckForUpdates);
            ui.close();
        }
        let mut auto = self.auto_update;
        if ui
            .checkbox(&mut auto, "Check for updates when starting")
            .on_hover_text(format!(
                "Asks GitHub for a newer build while {} starts, and asks you whether to \
                 update. Off: nothing is checked unless you choose Check for updates",
                self.info.name
            ))
            .changed()
        {
            self.auto_update = auto;
            actions.push(WorkbenchAction::SetAutoUpdate(auto));
        }
        ui.label(
            RichText::new(self.update.line())
                .small()
                .color(theme::TEXT_WEAK),
        );
    }

    /// The top bar's Restart to update button, shown when a new build is
    /// ready.
    pub(crate) fn restart_button(&mut self, ui: &mut Ui, actions: &mut Vec<WorkbenchAction>) {
        let UpdateNotice::Ready(build) = self.update else {
            return;
        };
        let button = egui::Button::new(RichText::new("Restart to update").color(Color32::WHITE))
            .fill(self.accent.colors().fill)
            .stroke(egui::Stroke::NONE)
            .min_size(egui::vec2(0.0, crate::top_bar::BAR_BUTTON.y));
        let clicked = ui
            .add(button)
            .on_hover_text(format!(
                "Build {build} is downloaded. Closes {name} (asking about unsaved work first), \
                 installs the new build and opens it again. Or keep working: it installs \
                 when you close.",
                name = self.info.name
            ))
            .clicked();
        if clicked {
            self.restarting = true;
            self.request(WorkbenchAction::Quit, actions);
        }
        ui.add_space(6.0);
    }
}

#[cfg(test)]
#[path = "updates_tests.rs"]
mod tests;
