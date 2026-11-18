//! Watching a project's folder. Events are only hints that something may
//! have changed: the database then looks again and decides
//! ([decision 0021](../../../docs/decisions/0021-asset-database-and-models.md)),
//! which copes with programs that save by renaming, events that come
//! twice, events that are lost (an overflow), and Solis's own writes.

use notify::Watcher as _;
use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

/// How long the folder must be quiet before a change is reported, so a
/// program writing a file in pieces is reported once.
pub(crate) const QUIET: Duration = Duration::from_millis(300);

/// The longest a burst goes unreported: a folder that never goes quiet
/// for [`QUIET`] (a program writing a file every 0.2 s, a long copy) is
/// still reported this often, or the Library never followed it (the
/// second review pass's N-20).
pub(crate) const LONGEST: Duration = Duration::from_secs(2);

/// Watches a folder and everything in it.
pub struct Watcher {
    watcher: RefCell<notify::RecommendedWatcher>,
    /// When each event (or error, such as lost events) arrived.
    events: Receiver<Instant>,
    burst: Cell<Burst>,
    /// A project's folder, and the folders at its top watched with
    /// everything in them ([`Watcher::new_project`]).
    project: Option<(PathBuf, RefCell<HashSet<PathBuf>>)>,
}

impl std::fmt::Debug for Watcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Watcher")
    }
}

impl Watcher {
    /// Starts watching `folder` and everything in it.
    pub fn new(folder: &Path) -> notify::Result<Self> {
        let watcher = Self::listening(|_| true)?;
        watcher
            .watcher
            .borrow_mut()
            .watch(folder, notify::RecursiveMode::Recursive)?;
        Ok(watcher)
    }

    /// Starts watching a project's folder (decision 0118: its files are
    /// in it): what is directly in it, and each folder at its top with
    /// everything in it, except Solis's own ([`crate::is_own`]: the cache,
    /// Git's history, builds, saves), which are never watched, so writing
    /// the cache or a build isn't heard and a big history costs nothing.
    /// A folder made at the top later is watched once it's heard.
    pub fn new_project(root: &Path) -> notify::Result<Self> {
        let top = root.to_path_buf();
        let mut watcher = Self::listening(move |path| {
            path.strip_prefix(&top)
                .map_or(true, |inside| !crate::is_own(inside))
        })?;
        watcher
            .watcher
            .borrow_mut()
            .watch(root, notify::RecursiveMode::NonRecursive)?;
        watcher.project = Some((root.to_path_buf(), RefCell::new(HashSet::new())));
        watcher.follow_top_folders();
        Ok(watcher)
    }

    /// A watcher with nothing watched yet, hearing an event when `heard`
    /// takes any of its paths (and always when it names none, or is an
    /// error such as lost events: looked at again to be sure).
    fn listening(heard: impl Fn(&Path) -> bool + Send + 'static) -> notify::Result<Self> {
        let (sender, events) = channel();
        let watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
            let wanted = match &event {
                // A file or folder only looked at (Solis reading the
                // folder, say) changed nothing.
                Ok(event) if looked_at(&event.kind) => false,
                Ok(event) => event.paths.is_empty() || event.paths.iter().any(|p| heard(p)),
                Err(_) => true,
            };
            if wanted {
                let _ = sender.send(Instant::now());
            }
        })?;
        Ok(Self {
            watcher: RefCell::new(watcher),
            events,
            burst: Cell::new(Burst::default()),
            project: None,
        })
    }

    /// Watches each folder at the project's top that isn't watched yet
    /// (and isn't Solis's own), and forgets those that went.
    fn follow_top_folders(&self) {
        let Some((root, watched)) = &self.project else {
            return;
        };
        let Ok(entries) = std::fs::read_dir(root) else {
            return;
        };
        let now: HashSet<PathBuf> = entries
            .flatten()
            .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
            .filter(|e| !crate::is_own(Path::new(&e.file_name())))
            .map(|e| e.path())
            .collect();
        let mut watched = watched.borrow_mut();
        let mut watcher = self.watcher.borrow_mut();
        for folder in now.difference(&watched.clone()) {
            if watcher
                .watch(folder, notify::RecursiveMode::Recursive)
                .is_ok()
            {
                watched.insert(folder.clone());
            }
        }
        watched.retain(|folder| {
            let there = now.contains(folder);
            if !there {
                let _ = watcher.unwatch(folder);
            }
            there
        });
    }

    /// Whether anything happened since last asked and the folder has
    /// been quiet for `QUIET` since, or it began `LONGEST` ago (an error,
    /// such as lost events, counts: look again to be sure). Never waits.
    pub fn changed(&self) -> bool {
        let mut burst = self.burst.get();
        while let Ok(at) = self.events.try_recv() {
            burst.heard(at);
        }
        let settled = burst.settled(Instant::now());
        self.burst.set(burst);
        if settled {
            self.follow_top_folders();
        }
        settled
    }
}

/// Whether `kind` is something only being opened or read, not changed:
/// a file closed after writing still counts.
fn looked_at(kind: &notify::EventKind) -> bool {
    use notify::event::{AccessKind, AccessMode};
    matches!(kind, notify::EventKind::Access(access)
        if !matches!(access, AccessKind::Close(AccessMode::Write)))
}

/// A burst of events, reported once the folder has been quiet for
/// `QUIET` after the last of them, or once it has gone on for `LONGEST`.
#[derive(Debug, Clone, Copy, Default)]
struct Burst {
    /// The first and the newest event of the burst.
    heard: Option<(Instant, Instant)>,
}

impl Burst {
    fn heard(&mut self, at: Instant) {
        self.heard = Some(
            self.heard
                .map_or((at, at), |(first, last)| (first.min(at), last.max(at))),
        );
    }

    /// Whether a burst is to be reported by `now`: it has ended, or gone
    /// on too long. If so, it's reported and forgotten; events after it
    /// start another.
    fn settled(&mut self, now: Instant) -> bool {
        let report = self.heard.is_some_and(|(first, last)| {
            now.saturating_duration_since(last) >= QUIET
                || now.saturating_duration_since(first) >= LONGEST
        });
        if report {
            self.heard = None;
        }
        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_folder;

    #[test]
    fn nothing_heard_is_never_a_change() {
        let mut burst = Burst::default();
        assert!(!burst.settled(Instant::now() + QUIET * 10));
    }

    #[test]
    fn a_burst_is_reported_once_after_it_goes_quiet() {
        let start = Instant::now();
        let mut burst = Burst::default();
        burst.heard(start);
        assert!(!burst.settled(start), "not yet quiet");
        assert!(!burst.settled(start + QUIET / 2), "not yet quiet");
        assert!(burst.settled(start + QUIET), "quiet long enough");
        assert!(!burst.settled(start + QUIET * 2), "reported once");
    }

    #[test]
    fn a_file_written_in_pieces_is_one_change() {
        // Each piece comes before the folder has been quiet long enough,
        // so the quiet time runs from the last.
        let start = Instant::now();
        let mut burst = Burst::default();
        for piece in 0..5 {
            let at = start + QUIET / 2 * piece;
            burst.heard(at);
            assert!(!burst.settled(at), "piece {piece}");
        }
        let last = start + QUIET / 2 * 4;
        assert!(!burst.settled(last + QUIET / 2));
        assert!(burst.settled(last + QUIET));
        assert!(!burst.settled(last + QUIET * 3));
    }

    #[test]
    fn an_event_handed_over_late_still_counts_from_the_newest() {
        let start = Instant::now();
        let mut burst = Burst::default();
        burst.heard(start + QUIET);
        burst.heard(start);
        assert!(!burst.settled(start + QUIET + QUIET / 2));
        assert!(burst.settled(start + QUIET * 2));
    }

    #[test]
    fn a_folder_that_never_goes_quiet_is_still_reported_every_two_seconds() {
        // N-20: a file written every 0.2 s never left the folder quiet
        // for 0.3 s, so nothing was reported while it went on.
        let start = Instant::now();
        let mut burst = Burst::default();
        let mut reported = Vec::new();
        for tick in 0..50u32 {
            let at = start + Duration::from_millis(200) * tick;
            burst.heard(at);
            if burst.settled(at) {
                reported.push(at - start);
            }
        }
        let millis: Vec<u128> = reported.iter().map(Duration::as_millis).collect();
        assert_eq!(millis, [2000, 4200, 6400, 8600], "every 2 s or so");
        // The last part of it is still reported once it goes quiet.
        let last = start + Duration::from_millis(200) * 49;
        assert!(burst.settled(last + QUIET));
        assert!(!burst.settled(last + LONGEST * 2), "reported once");
    }

    #[test]
    fn a_project_s_watcher_hears_its_files_but_not_solis_s_own_folders() {
        let root = test_folder("watch-project");
        std::fs::create_dir_all(root.join(".solis/cache")).unwrap();
        std::fs::create_dir_all(root.join("builds")).unwrap();
        let Ok(watcher) = Watcher::new_project(&root) else {
            return;
        };
        let heard_within = |ms: u64| {
            let deadline = Instant::now() + Duration::from_millis(ms);
            let mut seen = false;
            while !seen && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(50));
                seen = watcher.changed();
            }
            seen
        };
        std::fs::write(root.join(".solis/cache/x.solbin"), b"cache").unwrap();
        std::fs::write(root.join("builds/game.exe"), b"build").unwrap();
        assert!(!heard_within(1500), "Solis's own folders aren't heard");
        std::fs::write(root.join("crate.glb"), b"mine").unwrap();
        assert!(heard_within(10_000), "the project's files are");
        // A folder made at the top is watched with what's in it.
        std::fs::create_dir_all(root.join("farm")).unwrap();
        assert!(heard_within(10_000), "the new folder");
        std::fs::write(root.join("farm/hay.png"), b"hay").unwrap();
        assert!(heard_within(10_000), "a file in it");
    }

    #[test]
    fn a_save_by_rename_is_noticed() {
        let folder = test_folder("watch");
        std::fs::create_dir_all(&folder).unwrap();
        let Ok(watcher) = Watcher::new(&folder) else {
            // Some build machines can't watch; the database's rescans are
            // tested on their own.
            return;
        };
        assert!(!watcher.changed());
        std::fs::write(folder.join("crate.glb.tmp"), b"new").unwrap();
        std::fs::rename(folder.join("crate.glb.tmp"), folder.join("crate.glb")).unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        let mut seen = false;
        while !seen && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(50));
            seen = watcher.changed();
        }
        assert!(seen, "the change was reported");
    }
}
