//! Keeping the program up to date ([decision
//! 0043](../../../docs/decisions/0043-installed-and-updated-from-github.md)):
//! checks GitHub on a thread when the window opens and every few hours
//! (when the user hasn't turned that off), downloads a newer build while
//! they work, and starts its installer when the window closes.

use crate::branding;
use editor::{LogLevel, UpdateNotice, Workbench};
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use updater::Status;

/// How often to look again while the window stays open.
pub const CHECK_EVERY: Duration = Duration::from_secs(6 * 60 * 60);

/// An installer to start when the window has closed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pending {
    /// The downloaded, checked installer.
    pub installer: PathBuf,
    /// Whether to open the program again afterwards (Restart to update).
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

/// The window's side of updating.
pub struct Updates {
    /// The check under way.
    checking: Option<Receiver<Status>>,
    /// When the last check started.
    last_check: Option<Instant>,
    /// Whether the check under way was asked for (Help > Check for
    /// updates), so its outcome is said in the Console even when nothing
    /// changed.
    asked: bool,
    /// What happens when the window closes.
    pending: PendingSlot,
}

impl Updates {
    /// Nothing checked yet; a downloaded build goes in `pending`.
    pub fn new(pending: PendingSlot) -> Self {
        Self {
            checking: None,
            last_check: None,
            asked: false,
            pending,
        }
    }

    /// Called every frame: starts a check when one is due, takes in how
    /// it's going, and keeps the pending install in step with the editor.
    pub fn frame(&mut self, workbench: &mut Workbench, ctx: &eframe::egui::Context) {
        let due = self
            .last_check
            .is_none_or(|last| last.elapsed() >= CHECK_EVERY);
        if workbench.auto_update() && due && self.checking.is_none() && !is_ready(workbench) {
            self.start(workbench, ctx, false);
        }
        self.take_in(workbench);
        if let Ok(mut pending) = self.pending.lock()
            && let Some(pending) = pending.as_mut()
        {
            pending.relaunch = workbench.restarting();
        }
    }

    /// Checks now (Help > Check for updates).
    pub fn check_now(&mut self, workbench: &mut Workbench, ctx: &eframe::egui::Context) {
        if self.checking.is_none() {
            self.start(workbench, ctx, true);
        }
    }

    fn start(&mut self, workbench: &mut Workbench, ctx: &eframe::egui::Context, asked: bool) {
        self.last_check = Some(Instant::now());
        self.asked = asked;
        let ctx = ctx.clone();
        self.checking = Some(updater::start(source(), this_copy(), move || {
            ctx.request_repaint();
        }));
        workbench.set_update(UpdateNotice::Checking);
    }

    fn take_in(&mut self, workbench: &mut Workbench) {
        let Some(receiver) = &self.checking else {
            return;
        };
        let arrived: Vec<Status> = receiver.try_iter().collect();
        let mut ended = false;
        for status in arrived {
            ended |= status.is_final();
            self.follow(workbench, status);
        }
        if ended {
            self.checking = None;
        }
    }

    /// Takes in one step of a check.
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
            Status::NotInstalled(build) => (
                UpdateNotice::NotInstalled(build),
                Some((
                    LogLevel::Info,
                    format!(
                        "Build {build} of {name} is out. This copy wasn't installed with the \
                         installer, so it can't update itself: download {} from {} and run it \
                         once.",
                        branding::INSTALLER_NAME,
                        releases_page()
                    ),
                )),
            ),
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
                // A computer that's offline isn't worth a warning every few
                // hours; only when asked.
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
        // Saying the same again (a check every few hours finding the same
        // build) is noise.
        let changed = workbench.update() != &notice;
        if let Some((level, text)) = said
            && (changed || self.asked)
        {
            workbench.log(level, text);
        }
        workbench.set_update(notice);
    }
}

fn is_ready(workbench: &Workbench) -> bool {
    matches!(workbench.update(), UpdateNotice::Ready(_))
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
        Updates::new(Arc::new(Mutex::new(None)))
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
    fn quiet_checks_say_nothing_when_nothing_changed() {
        let mut w = workbench();
        let mut u = updates();
        u.follow(&mut w, Status::UpToDate(4));
        u.follow(&mut w, Status::FromSource);
        assert!(w.log_lines().is_empty(), "{:?}", w.log_lines());
        // Offline: said once, quietly, not every few hours.
        u.follow(&mut w, Status::Failed("offline".into()));
        u.follow(&mut w, Status::Failed("offline".into()));
        let lines: Vec<_> = w
            .log_lines()
            .iter()
            .filter(|l| l.text.contains("offline"))
            .collect();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].level, LogLevel::Info);
    }

    #[test]
    fn asked_checks_always_answer() {
        let mut w = workbench();
        let mut u = updates();
        u.asked = true;
        u.follow(&mut w, Status::UpToDate(4));
        u.follow(&mut w, Status::UpToDate(4));
        assert_eq!(
            w.log_lines()
                .iter()
                .filter(|l| l.text.contains("up to date"))
                .count(),
            2
        );
        u.follow(&mut w, Status::Failed("offline".into()));
        assert_eq!(w.log_lines().last().unwrap().level, LogLevel::Warning);
    }

    #[test]
    fn copies_not_installed_are_told_where_the_installer_is() {
        let mut w = workbench();
        let mut u = updates();
        u.follow(&mut w, Status::NotInstalled(12));
        let line = &w.log_lines().last().unwrap().text;
        assert!(line.contains(branding::INSTALLER_NAME), "{line}");
        assert!(line.contains("/releases"), "{line}");
        assert!(u.pending.lock().unwrap().is_none());
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
