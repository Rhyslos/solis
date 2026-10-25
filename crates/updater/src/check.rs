//! One check for updates, on a thread of its own: ask GitHub, decide, and
//! download when there's something newer.

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
    /// A newer build is downloading (its number).
    Downloading(u64),
    /// A newer build is downloaded and checked, ready to install.
    Ready {
        /// Its build number.
        build: u64,
        /// The installer.
        installer: PathBuf,
    },
    /// A newer build exists, but this copy wasn't installed with the
    /// installer, so it isn't downloaded (its number).
    NotInstalled(u64),
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

/// Runs one check, telling `report` each step; gives the last status.
pub fn run(remote: &dyn Remote, copy: &Copy, report: &mut dyn FnMut(Status)) -> Status {
    let Some(this) = copy.build else {
        return Status::FromSource;
    };
    report(Status::Checking);
    let release = match remote.latest() {
        Ok(release) => release,
        Err(why) => return Status::Failed(why),
    };
    if release.build <= this {
        return Status::UpToDate(this);
    }
    if !copy.installed {
        return Status::NotInstalled(release.build);
    }
    report(Status::Downloading(release.build));
    match remote.download(&release, &copy.folder) {
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
    let (sender, receiver) = mpsc::channel();
    let spawned = std::thread::Builder::new().name("updates".into()).spawn({
        let sender = sender.clone();
        move || {
            let mut report = |status: Status| {
                let _ = sender.send(status);
                wake();
            };
            let last = run(&remote, &copy, &mut report);
            report(last);
        }
    });
    if let Err(e) = spawned {
        let _ = sender.send(Status::Failed(format!("couldn't start checking: {e}")));
    }
    receiver
}
