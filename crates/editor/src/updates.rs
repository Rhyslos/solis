//! What the editor shows about updates ([decision
//! 0043](../../../docs/decisions/0043-installed-and-updated-from-github.md),
//! [decision 0045](../../../docs/decisions/0045-a-splash-screen-that-asks-before-updating.md)):
//! the update question when Check for updates (File > Updates, or the
//! search) finds a newer build, and a link in the tab bar when it's
//! downloaded. When Solis starts, the app asks on the splash screen
//! instead. The app does the checking and installing, and tells the
//! editor how it's going with [`Workbench::set_update`].

use crate::workbench::{Workbench, WorkbenchAction};
use egui::Ui;

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

/// What a newer build brings, as its release on GitHub says, for the
/// launcher's Updates page (decision 0086).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReleaseNotes {
    /// Its build number.
    pub build: u64,
    /// Its notes, as plain text (may be empty).
    pub notes: String,
    /// The day it was published (`2026-11-05`), if known.
    pub published: Option<String>,
}

impl UpdateNotice {
    /// The newer build this is about, if any.
    pub fn newer_build(&self) -> Option<u64> {
        match self {
            UpdateNotice::Available(b)
            | UpdateNotice::Downloading(b)
            | UpdateNotice::Ready(b)
            | UpdateNotice::NotInstalled(b) => Some(*b),
            _ => None,
        }
    }

    /// One line for Project settings' Engine version.
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
    /// What the newer build found brings, for the launcher's Updates
    /// page.
    pub fn set_release_notes(&mut self, notes: Option<ReleaseNotes>) {
        self.release_notes = notes;
    }

    /// Tells the editor how updating is going.
    pub fn set_update(&mut self, notice: UpdateNotice) {
        self.update = notice;
    }

    /// Asks the update question (File > Updates found a newer
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
        // File › Updates shows the same, with its buttons.
        if self.file_page_open() && self.file_pages.page == crate::file_pages::Page::Updates {
            self.offer = None;
            return;
        }
        let name = self.info.name.clone();
        let Some(answer) = crate::splash::update_question(ctx, &name, &offer, &mut self.skip_offer)
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

    /// The tab bar's Restart to update button, shown when a new build is
    /// ready. (It moves to the status bar with the rest of the frame.)
    pub(crate) fn restart_button(&mut self, ui: &mut Ui, actions: &mut Vec<WorkbenchAction>) {
        // A newer build that is out but not taken yet: its line leads to
        // File › Updates.
        if let UpdateNotice::Available(build) | UpdateNotice::NotInstalled(build) = self.update {
            let line = format!("Build {build} is out");
            if crate::widgets::button::link(ui, &line)
                .on_hover_text("File › Updates: what it brings, and getting it")
                .clicked()
            {
                self.show_file_page(crate::file_pages::Page::Updates);
            }
            return;
        }
        let UpdateNotice::Ready(build) = self.update else {
            return;
        };
        let clicked = crate::widgets::button::link(ui, "Restart to update")
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
    }
}

#[cfg(test)]
#[path = "updates_tests.rs"]
mod tests;
