//! Checking for updates and downloading one, each on a thread of its own
//! ([decision 0045](../../../docs/decisions/0045-a-splash-screen-that-asks-before-updating.md)):
//! a check only asks GitHub and says whether a newer build is out; the
//! download waits until the person says Update now.

use crate::fetch::{self, Source};
use crate::release::Release;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};

/// How a check is going, or how it ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    /// Asking GitHub.
    Checking,
    /// This copy is the newest build (its number).
    UpToDate(u64),
    /// A newer build is out: what GitHub says of it. Nothing is
    /// downloaded until asked ([`start_download`]).
    Available(Release),
    /// A newer build is downloading (its number).
    Downloading(u64),
    /// A newer build is downloaded and checked, ready to install.
    Ready {
        /// Its build number.
        build: u64,
        /// The installer.
        installer: PathBuf,
    },
    /// This copy was built from source, so it has no build number to
    /// compare.
    FromSource,
    /// The check didn't work, and why.
    Failed(String),
}

impl Status {
    /// Whether this is where a check ends.
    pub fn is_final(&self) -> bool {
        !matches!(self, Status::Checking | Status::Downloading(_))
    }
}

/// What a check talks to: GitHub, or a stand-in in tests.
pub trait Remote {
    /// The latest release.
    fn latest(&self) -> Result<Release, String>;
    /// Downloads and checks `release`'s installer into `folder`.
    fn download(&self, release: &Release, folder: &Path) -> Result<PathBuf, String>;
}

impl Remote for Source {
    fn latest(&self) -> Result<Release, String> {
        fetch::latest(self)
    }
    fn download(&self, release: &Release, folder: &Path) -> Result<PathBuf, String> {
        fetch::download(self, release, folder)
    }
}

/// What a check found out about this copy.
#[derive(Debug, Clone)]
pub struct Copy {
    /// Its build number, if it has one ([`crate::this_build`]).
    pub build: Option<u64>,
    /// Whether the installer put it there ([`crate::is_installed`]).
    pub installed: bool,
    /// Where to download updates ([`crate::updates_folder`]).
    pub folder: PathBuf,
}

/// Runs one check, telling `report` each step; gives the last status:
/// up to date, a newer build available, built from source, or failed.
/// Never downloads.
pub fn run(remote: &dyn Remote, copy: &Copy, report: &mut dyn FnMut(Status)) -> Status {
    let Some(this) = copy.build else {
        return Status::FromSource;
    };
    report(Status::Checking);
    match remote.latest() {
        Ok(release) if release.build > this => Status::Available(release),
        Ok(_) => Status::UpToDate(this),
        Err(why) => Status::Failed(why),
    }
}

/// Downloads and checks `release`'s installer into `folder`, telling
/// `report` each step; gives the last status: ready, or failed.
pub fn download(
    remote: &dyn Remote,
    release: &Release,
    folder: &Path,
    report: &mut dyn FnMut(Status),
) -> Status {
    report(Status::Downloading(release.build));
    match remote.download(release, folder) {
        Ok(installer) => Status::Ready {
            build: release.build,
            installer,
        },
        Err(why) => Status::Failed(why),
    }
}

/// Starts a check on its own thread. Each step arrives on the returned
/// channel, ending with a final one ([`Status::is_final`]); `wake` is
/// called after each, so a window that sleeps when idle looks.
pub fn start<R: Remote + Send + 'static>(
    remote: R,
    copy: Copy,
    wake: impl Fn() + Send + 'static,
) -> Receiver<Status> {
    on_a_thread(wake, move |report| run(&remote, &copy, report))
}

/// Starts downloading `release` into `folder` on its own thread, the
/// steps arriving as [`start`]'s do.
pub fn start_download<R: Remote + Send + 'static>(
    remote: R,
    release: Release,
    folder: PathBuf,
    wake: impl Fn() + Send + 'static,
) -> Receiver<Status> {
    on_a_thread(wake, move |report| {
        download(&remote, &release, &folder, report)
    })
}

/// Runs `work` on a thread, sending each status it reports and the one it
/// ends with.
fn on_a_thread(
    wake: impl Fn() + Send + 'static,
    work: impl FnOnce(&mut dyn FnMut(Status)) -> Status + Send + 'static,
) -> Receiver<Status> {
    let (sender, receiver) = mpsc::channel();
    let spawned = std::thread::Builder::new().name("updates".into()).spawn({
        let sender = sender.clone();
        move || {
            let mut report = |status: Status| {
                let _ = sender.send(status);
                wake();
            };
            let last = work(&mut report);
            report(last);
        }
    });
    if let Err(e) = spawned {
        let _ = sender.send(Status::Failed(format!("couldn't start: {e}")));
    }
    receiver
}

/// Whether to offer `latest` when starting: not when it's the build the
/// person chose to skip (Skip this version). A newer build than the
/// skipped one is offered again. Asking (Help > Check for updates)
/// always offers.
pub fn should_offer(latest: u64, skipped: Option<u64>, asked: bool) -> bool {
    asked || skipped != Some(latest)
}
