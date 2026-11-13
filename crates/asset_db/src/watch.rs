//! Watching the assets folder. Events are only hints that something may
//! have changed: the database then looks again and decides
//! ([decision 0021](../../../docs/decisions/0021-asset-database-and-models.md)),
//! which copes with programs that save by renaming, events that come
//! twice, events that are lost (an overflow), and Solis's own writes.

use notify::Watcher as _;
use std::cell::Cell;
use std::path::Path;
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
    _watcher: notify::RecommendedWatcher,
    /// When each event (or error, such as lost events) arrived.
    events: Receiver<Instant>,
    burst: Cell<Burst>,
}

impl std::fmt::Debug for Watcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Watcher")
    }
}

impl Watcher {
    /// Starts watching `folder`.
    pub fn new(folder: &Path) -> notify::Result<Self> {
        let (sender, events) = channel();
        let mut watcher = notify::recommended_watcher(move |_: notify::Result<notify::Event>| {
            let _ = sender.send(Instant::now());
        })?;
        watcher.watch(folder, notify::RecursiveMode::Recursive)?;
        Ok(Self {
            _watcher: watcher,
            events,
            burst: Cell::new(Burst::default()),
        })
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
        settled
    }
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
