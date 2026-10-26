//! Starting up behind the splash screen ([decision
//! 0045](../../../docs/decisions/0045-a-splash-screen-that-asks-before-updating.md)):
//! the splash shows for at least [`SPLASH_AT_LEAST`] while GitHub is asked
//! for a newer build. When the answer comes, Solis opens, or asks whether
//! to update first. Updating downloads the build on the splash, checks it,
//! and closes so its installer can run and open the new build.
//!
//! This is the plain logic, without a window, so tests can drive it; the
//! window (`window.rs`) draws the splash and the question, and does what
//! [`Next`] says.

use editor::{LogLevel, UpdateAnswer, UpdateOffer};
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::time::Duration;
use updater::{Release, Status};

/// The shortest time the splash shows, so a quick start doesn't just
/// flash it (as Aventail's).
pub const SPLASH_AT_LEAST: Duration = Duration::from_millis(600);

/// The longest the splash waits for GitHub: offline, or slow, Solis opens
/// anyway, and a late answer only shows under Help.
pub const CHECK_WAIT: Duration = Duration::from_secs(5);

/// What the window should do now.
#[derive(Debug, Clone, PartialEq)]
pub enum Next {
    /// Keep showing the splash (and the question, if one is asked).
    Wait,
    /// Open the editor, saying this in the Console first if anything.
    Open(Option<(LogLevel, String)>),
    /// The new build is downloaded and checked: close, so its installer
    /// can run and open it.
    Install {
        /// The installer.
        installer: PathBuf,
    },
}

#[derive(Debug, Clone, PartialEq)]
enum Phase {
    /// Asking GitHub (or, with checking off, only showing the splash).
    Checking,
    /// A newer build is out: the question is asked.
    Asking(Release),
    /// Downloading the build chosen.
    Downloading(u64),
    /// Ready to open, saying this first.
    Done(Option<(LogLevel, String)>),
}

/// Starting up, from the first frame until the editor opens.
pub struct Startup {
    phase: Phase,
    check: Option<Receiver<Status>>,
    download: Option<Receiver<Status>>,
    /// The build skipped with Skip this version, if any; changes when
    /// the person skips another.
    skipped: Option<u64>,
    /// Whether the installer put this copy here.
    installed: bool,
    /// This copy's build.
    current: Option<u64>,
    /// The question's Skip this version tick.
    pub skip_tick: bool,
}

impl Startup {
    /// Starting, with `check` under way (none when checking is off),
    /// `skipped` the build skipped before.
    pub fn new(
        check: Option<Receiver<Status>>,
        skipped: Option<u64>,
        installed: bool,
        current: Option<u64>,
    ) -> Self {
        Self {
            phase: Phase::Checking,
            check,
            download: None,
            skipped,
            installed,
            current,
            skip_tick: false,
        }
    }

    /// What the splash says now.
    pub fn doing(&self) -> String {
        match &self.phase {
            Phase::Checking if self.check.is_some() => "Checking for updates…".into(),
            Phase::Checking | Phase::Done(_) => "Starting…".into(),
            Phase::Asking(release) => format!("Build {} is available", release.build),
            Phase::Downloading(build) => format!("Downloading build {build}…"),
        }
    }

    /// The question to ask, while one is asked.
    pub fn offer(&self) -> Option<UpdateOffer> {
        match &self.phase {
            Phase::Asking(release) => Some(UpdateOffer {
                latest: release.build,
                current: self.current,
                installed: self.installed,
            }),
            _ => None,
        }
    }

    /// The build skipped now (Skip this version), to save.
    pub fn skipped(&self) -> Option<u64> {
        self.skipped
    }

    /// The check, if it's still under way when the editor opens (GitHub
    /// was slow): its answer can still show under Help.
    pub fn take_check(&mut self) -> Option<Receiver<Status>> {
        self.check.take()
    }

    /// Takes in what's arrived, `elapsed` since the splash appeared, and
    /// says what to do.
    pub fn step(&mut self, elapsed: Duration) -> Next {
        self.take_in_check();
        if let Some(next) = self.take_in_download() {
            return next;
        }
        if self.phase == Phase::Checking && (self.check.is_none() || elapsed >= CHECK_WAIT) {
            // Checking off, or GitHub too slow: open; a late answer shows
            // under Help.
            self.phase = Phase::Done(None);
        }
        match &self.phase {
            Phase::Done(note) if elapsed >= SPLASH_AT_LEAST => Next::Open(note.clone()),
            _ => Next::Wait,
        }
    }

    fn take_in_check(&mut self) {
        if self.phase != Phase::Checking {
            return;
        }
        let Some(check) = &self.check else { return };
        let Some(last) = check.try_iter().find(Status::is_final) else {
            return;
        };
        self.check = None;
        self.phase = match last {
            Status::Available(release)
                if updater::should_offer(release.build, self.skipped, false) =>
            {
                Phase::Asking(release)
            }
            Status::Available(release) => Phase::Done(Some((
                LogLevel::Info,
                format!(
                    "Build {} is available; you chose to skip it. Help > Check for updates asks \
                     again.",
                    release.build
                ),
            ))),
            Status::Failed(why) => Phase::Done(Some((
                LogLevel::Info,
                format!("Couldn't check for updates: {why}."),
            ))),
            _ => Phase::Done(None),
        };
    }

    fn take_in_download(&mut self) -> Option<Next> {
        let Phase::Downloading(build) = self.phase else {
            return None;
        };
        let download = self.download.as_ref()?;
        let last = download.try_iter().find(Status::is_final)?;
        self.download = None;
        match last {
            Status::Ready { installer, .. } => Some(Next::Install { installer }),
            Status::Failed(why) => {
                self.phase = Phase::Done(Some((
                    LogLevel::Warning,
                    format!(
                        "Couldn't download build {build}: {why}. This build opens instead; Help \
                         > Check for updates tries again."
                    ),
                )));
                None
            }
            _ => None,
        }
    }

    /// The person answered the question. `download` starts downloading
    /// the build (asked only for Update now).
    pub fn answer(
        &mut self,
        answer: UpdateAnswer,
        download: impl FnOnce(Release) -> Receiver<Status>,
    ) {
        let Phase::Asking(release) = self.phase.clone() else {
            return;
        };
        self.phase = match answer {
            UpdateAnswer::UpdateNow => {
                let build = release.build;
                self.download = Some(download(release));
                Phase::Downloading(build)
            }
            UpdateAnswer::OpenDownloadPage => Phase::Done(None),
            UpdateAnswer::NotNow { skip } => {
                if skip {
                    self.skipped = Some(release.build);
                }
                Phase::Done(None)
            }
        };
    }
}

#[cfg(test)]
#[path = "startup_tests.rs"]
mod tests;
