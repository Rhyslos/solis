//! Keeping the program up to date while it runs ([decision
//! 0043](../../../docs/decisions/0043-installed-and-updated-from-github.md),
//! [decision 0045](../../../docs/decisions/0045-a-splash-screen-that-asks-before-updating.md)).
//! Solis asks GitHub on the splash screen as it starts (`startup.rs`); this
//! is the rest: Help > Check for updates asks whether to update when
//! there's a newer build, Update now downloads it while you work, and the
//! installer runs when the window closes (opening Solis again after
//! Restart to update). A check the splash couldn't wait for finishes
//! here, quietly.

use crate::branding;
use editor::{LogLevel, UpdateNotice, UpdateOffer, Workbench};
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use updater::{Release, Status};

/// An installer to start when the window has closed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pending {
    /// The downloaded, checked installer.
    pub installer: PathBuf,
    /// Whether to open the program again afterwards (Restart to update,
    /// or Update now on the splash).
    pub relaunch: bool,
}

/// Shared between the window and [`crate::window::run`], which starts the
/// installer once the window is gone.
pub type PendingSlot = Arc<Mutex<Option<Pending>>>;

/// Where GitHub's builds are, and who's asking.
pub fn source() -> updater::Source {
    updater::Source {
        repo: branding::UPDATE_REPO.to_string(),
        installer_name: branding::INSTALLER_NAME.to_string(),
        user_agent: format!("{}/{}", branding::APP_ID, env!("CARGO_PKG_VERSION")),
    }
}

/// This copy: its build, whether it was installed, where updates go.
pub fn this_copy() -> updater::Copy {
    let installed = std::env::current_exe().is_ok_and(|exe| updater::is_installed(&exe));
    updater::Copy {
        build: updater::this_build(),
        installed,
        folder: updater::updates_folder(branding::APP_ID),
    }
}

/// Starts a check, waking the window when it moves on.
pub fn start_check(ctx: &eframe::egui::Context) -> Receiver<Status> {
    let ctx = ctx.clone();
    updater::start(source(), this_copy(), move || ctx.request_repaint())
}

/// Starts downloading `release`, waking the window when it moves on.
pub fn start_download(ctx: &eframe::egui::Context, release: Release) -> Receiver<Status> {
    let ctx = ctx.clone();
    updater::start_download(source(), release, this_copy().folder, move || {
        ctx.request_repaint()
    })
}

/// The window's side of updating, once the editor is open.
pub struct Updates {
    /// The check or download under way.
    working: Option<Receiver<Status>>,
    /// Whether the check under way was asked for (Help > Check for
    /// updates): its answer is shown, and a newer build asked about.
    asked: bool,
    /// The newer build last found, for Update now.
    offered: Option<Release>,
    /// The build skipped with Skip this version, if any (saved).
    skipped: Option<u64>,
    /// Whether `skipped` changed since it was last written to the
    /// settings: written at once, so a crash soon after doesn't lose it.
    skip_unsaved: bool,
    /// What happens when the window closes.
    pending: PendingSlot,
}

impl Updates {
    /// Nothing checked yet; `skipped` as saved; a downloaded build goes in
    /// `pending`.
    pub fn new(pending: PendingSlot, skipped: Option<u64>) -> Self {
        Self {
            working: None,
            asked: false,
            offered: None,
            skipped,
            skip_unsaved: false,
            pending,
        }
    }

    /// The build skipped with Skip this version, to save.
    pub fn skipped(&self) -> Option<u64> {
        self.skipped
    }

    /// Installs `installer` when the window closes, opening Solis again
    /// after (Update now on the splash).
    pub fn install_on_close(&mut self, installer: PathBuf) {
        if let Ok(mut pending) = self.pending.lock() {
            *pending = Some(Pending {
                installer,
                relaunch: true,
            });
        }
    }

    /// Skips `build` from now on (Skip this version).
    pub fn skip(&mut self, build: u64) {
        if self.skipped != Some(build) {
            self.skipped = Some(build);
            self.skip_unsaved = true;
        }
    }

    /// The skipped build, once, if it changed since this was last asked:
    /// for saving it straight away.
    pub fn take_unsaved_skip(&mut self) -> Option<Option<u64>> {
        std::mem::take(&mut self.skip_unsaved).then_some(self.skipped)
    }

    /// Carries on with the splash's check, which GitHub was too slow to
    /// answer before Solis opened: its answer only shows under Help.
    pub fn adopt(&mut self, check: Receiver<Status>, workbench: &mut Workbench) {
        self.working = Some(check);
        self.asked = false;
        workbench.set_update(UpdateNotice::Checking);
    }

    /// Called every frame: takes in how the check or download is going,
    /// and keeps the pending install in step with the editor.
    pub fn frame(&mut self, workbench: &mut Workbench) {
        if let Some(receiver) = &self.working {
            let arrived: Vec<Status> = receiver.try_iter().collect();
            let ended = arrived.iter().any(Status::is_final);
            for status in arrived {
                self.follow(workbench, status);
            }
            if ended {
                self.working = None;
            }
        }
        if let Ok(mut pending) = self.pending.lock()
            && let Some(pending) = pending.as_mut()
        {
            pending.relaunch = workbench.restarting();
        }
    }

    /// Checks now (Help > Check for updates).
    pub fn check_now(&mut self, workbench: &mut Workbench, ctx: &eframe::egui::Context) {
        if self.working.is_none() {
            self.asked = true;
            self.working = Some(start_check(ctx));
            workbench.set_update(UpdateNotice::Checking);
        }
    }

    /// Downloads the build offered (the update question's Update now).
    pub fn download(&mut self, workbench: &mut Workbench, ctx: &eframe::egui::Context) {
        if let Some(release) = self.offered.clone()
            && self.working.is_none()
        {
            workbench.set_update(UpdateNotice::Downloading(release.build));
            self.working = Some(start_download(ctx, release));
        }
    }

    /// Takes in one step of a check or download.
    fn follow(&mut self, workbench: &mut Workbench, status: Status) {
        let name = branding::APP_NAME;
        let (notice, said) = match status {
            Status::Checking => (UpdateNotice::Checking, None),
            Status::Downloading(build) => (UpdateNotice::Downloading(build), None),
            Status::UpToDate(build) => (
                UpdateNotice::UpToDate(build),
                self.asked.then(|| {
                    (
                        LogLevel::Info,
                        format!("{name} is up to date (build {build})."),
                    )
                }),
            ),
            Status::Available(release) => {
                let build = release.build;
                let copy = this_copy();
                let said = if self.asked {
                    workbench.offer_update(UpdateOffer {
                        latest: build,
                        current: copy.build,
                        installed: copy.installed,
                    });
                    None
                } else if updater::should_offer(build, self.skipped, false) {
                    Some((
                        LogLevel::Info,
                        format!("Build {build} of {name} is available: Help > Check for updates."),
                    ))
                } else {
                    None
                };
                self.offered = Some(release);
                (UpdateNotice::Available(build), said)
            }
            Status::Ready { build, installer } => {
                if let Ok(mut pending) = self.pending.lock() {
                    *pending = Some(Pending {
                        installer,
                        relaunch: false,
                    });
                }
                (
                    UpdateNotice::Ready(build),
                    Some((
                        LogLevel::Info,
                        format!(
                            "Build {build} of {name} is downloaded. It installs when you close \
                             {name}, or now with Restart to update (top right)."
                        ),
                    )),
                )
            }
            Status::FromSource => (
                UpdateNotice::FromSource,
                self.asked.then(|| {
                    (
                        LogLevel::Info,
                        format!(
                            "This copy of {name} was built from source, so it doesn't update \
                             itself. Installed copies do: see {}.",
                            releases_page()
                        ),
                    )
                }),
            ),
            Status::Failed(why) => {
                let level = if self.asked {
                    LogLevel::Warning
                } else {
                    LogLevel::Info
                };
                (
                    UpdateNotice::Failed(why.clone()),
                    Some((level, format!("Couldn't check for updates: {why}."))),
                )
            }
        };
        if let Some((level, text)) = said {
            workbench.log(level, text);
        }
        workbench.set_update(notice);
    }
}

/// The page listing every build.
pub fn releases_page() -> String {
    format!("https://github.com/{}/releases", branding::UPDATE_REPO)
}

/// Starts the pending installer, if any; called once the window is gone.
pub fn install_pending(pending: &PendingSlot) {
    let Some(pending) = pending.lock().ok().and_then(|mut p| p.take()) else {
        return;
    };
    if let Err(why) = updater::start_installer(&pending.installer, pending.relaunch) {
        log::warn!("{why}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workbench() -> Workbench {
        Workbench::new(crate::app_info(), None)
    }

    fn updates() -> Updates {
        Updates::new(Arc::new(Mutex::new(None)), None)
    }

    fn release(build: u64) -> Release {
        let asset = |name: &str| updater::Asset {
            name: name.into(),
            url: format!("https://github.com/x/y/releases/download/build-{build}/{name}"),
            size: 1,
        };
        Release {
            build,
            installer: asset("a"),
            checksum: asset("b"),
        }
    }

    #[test]
    fn a_ready_build_waits_for_the_window_to_close() {
        let mut w = workbench();
        let mut u = updates();
        u.follow(
            &mut w,
            Status::Ready {
                build: 9,
                installer: PathBuf::from("setup.exe"),
            },
        );
        assert_eq!(w.update(), &UpdateNotice::Ready(9));
        let pending = u.pending.lock().unwrap().clone().unwrap();
        assert_eq!(pending.installer, PathBuf::from("setup.exe"));
        assert!(!pending.relaunch);
        assert!(w.log_lines().iter().any(|l| l.text.contains("Build 9")));
    }

    #[test]
    fn asking_finds_a_newer_build_and_asks_whether_to_update() {
        let mut w = workbench();
        let mut u = updates();
        u.asked = true;
        u.follow(&mut w, Status::Available(release(12)));
        assert_eq!(w.update(), &UpdateNotice::Available(12));
        assert_eq!(u.offered.as_ref().map(|r| r.build), Some(12));
        assert!(
            w.log_lines().is_empty(),
            "asked in a window, not the Console"
        );
    }

    #[test]
    fn a_late_answer_from_the_splash_only_tells_and_respects_a_skip() {
        let mut w = workbench();
        let mut u = updates();
        u.follow(&mut w, Status::Available(release(12)));
        assert!(w.log_lines().last().unwrap().text.contains("Build 12"));
        let mut w = workbench();
        let mut u = Updates::new(Arc::new(Mutex::new(None)), Some(12));
        u.follow(&mut w, Status::Available(release(12)));
        assert!(w.log_lines().is_empty(), "skipped: not even a line");
        assert_eq!(w.update(), &UpdateNotice::Available(12), "Help still says");
    }

    #[test]
    fn quiet_checks_say_little_and_asked_ones_answer() {
        let mut w = workbench();
        let mut u = updates();
        u.follow(&mut w, Status::UpToDate(4));
        u.follow(&mut w, Status::FromSource);
        assert!(w.log_lines().is_empty(), "{:?}", w.log_lines());
        u.follow(&mut w, Status::Failed("offline".into()));
        assert_eq!(w.log_lines().last().unwrap().level, LogLevel::Info);
        u.asked = true;
        u.follow(&mut w, Status::UpToDate(4));
        assert!(w.log_lines().last().unwrap().text.contains("up to date"));
        u.follow(&mut w, Status::Failed("offline".into()));
        assert_eq!(w.log_lines().last().unwrap().level, LogLevel::Warning);
    }

    #[test]
    fn skipping_is_kept_and_saved_once() {
        let mut u = updates();
        assert_eq!(u.skipped(), None);
        assert_eq!(u.take_unsaved_skip(), None);
        u.skip(14);
        assert_eq!(u.skipped(), Some(14));
        assert_eq!(u.take_unsaved_skip(), Some(Some(14)));
        assert_eq!(u.take_unsaved_skip(), None, "once");
        u.skip(14);
        assert_eq!(u.take_unsaved_skip(), None, "the same again: nothing new");
    }

    #[test]
    fn nothing_pending_starts_nothing() {
        install_pending(&Arc::new(Mutex::new(None)));
        // A pending installer that has gone is only a log line.
        install_pending(&Arc::new(Mutex::new(Some(Pending {
            installer: PathBuf::from("/no/such/setup.exe"),
            relaunch: true,
        }))));
    }

    #[test]
    fn the_source_is_the_repository_in_branding() {
        let source = source();
        assert_eq!(source.repo, branding::UPDATE_REPO);
        assert_eq!(source.installer_name, branding::INSTALLER_NAME);
        assert!(source.user_agent.starts_with(branding::APP_ID));
    }
}
