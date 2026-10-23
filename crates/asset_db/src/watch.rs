//! Watching the assets folder. Events are only hints that something may
//! have changed: the database then looks again and decides
//! ([decision 0021](../../../docs/decisions/0021-asset-database-and-models.md)),
//! which copes with programs that save by renaming, events that come
//! twice, events that are lost (an overflow), and Solis's own writes.

use notify_debouncer_mini::{DebounceEventResult, Debouncer, new_debouncer, notify};
use std::path::Path;
use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

/// How long the folder must be quiet before a change is reported, so a
/// program writing a file in pieces is reported once.
pub(crate) const QUIET: Duration = Duration::from_millis(300);

/// Watches a folder and everything in it.
pub struct Watcher {
    _debouncer: Debouncer<notify::RecommendedWatcher>,
    events: Receiver<DebounceEventResult>,
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
        let mut debouncer = new_debouncer(QUIET, move |result| {
            let _ = sender.send(result);
        })?;
        debouncer
            .watcher()
            .watch(folder, notify::RecursiveMode::Recursive)?;
        Ok(Self {
            _debouncer: debouncer,
            events,
        })
    }

    /// Whether anything happened since last asked (an error, such as lost
    /// events, counts: look again to be sure). Never waits.
    pub fn changed(&self) -> bool {
        let mut any = false;
        while self.events.try_recv().is_ok() {
            any = true;
        }
        any
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_folder;

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
