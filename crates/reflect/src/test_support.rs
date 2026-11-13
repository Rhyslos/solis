//! What every crate's tests need (the `test-support` feature, only ever
//! turned on by a dev-dependency; the review's TEST-06).
//!
//! Tests that work with files make a folder of their own in the system's
//! temporary folder. Made by hand, nothing removed them, and each run (a
//! new process id) left new ones: over 10,000 after a day of runs.
//! [`TempFolder`] is the one way to make one; it goes when the test ends.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

/// A new, empty folder of a test's own in the system's temporary folder,
/// removed with everything in it when this is dropped: at the end of the
/// test, passed or failed. It is always one this made itself (never an
/// earlier run's), so dropping it removes nothing it didn't make.
#[derive(Debug)]
pub struct TempFolder {
    path: PathBuf,
}

impl TempFolder {
    /// Makes `solis-<name>-<process id>-<n>`, `n` a number this run
    /// hasn't used whose folder isn't there yet (one left by an earlier
    /// run with the same process id is passed over and left alone).
    ///
    /// # Errors
    /// If the temporary folder can't be written to.
    pub fn new(name: &str) -> std::io::Result<Self> {
        static MADE: AtomicU32 = AtomicU32::new(0);
        let stem = format!("solis-{name}-{}", std::process::id());
        let path = claim(&std::env::temp_dir(), &stem, &MADE)?;
        Ok(Self { path })
    }

    /// Where it is.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Makes `<stem>-<n>` in `base`, `n` taken from `next` until one isn't
/// there yet, and returns it.
fn claim(base: &Path, stem: &str, next: &AtomicU32) -> std::io::Result<PathBuf> {
    loop {
        let path = base.join(format!("{stem}-{}", next.fetch_add(1, Ordering::Relaxed)));
        match std::fs::create_dir(&path) {
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            made => return made.map(|()| path),
        }
    }
}

impl std::ops::Deref for TempFolder {
    type Target = Path;

    fn deref(&self) -> &Path {
        &self.path
    }
}

impl AsRef<Path> for TempFolder {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempFolder {
    fn drop(&mut self) {
        // Never through a link: `remove_dir_all` removes a link inside,
        // not what it points at.
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_temp_folder_is_new_and_empty_and_goes_when_dropped() {
        let folder = TempFolder::new("temp-folder").unwrap();
        assert!(folder.is_dir());
        assert_eq!(std::fs::read_dir(&folder).unwrap().count(), 0);
        std::fs::create_dir_all(folder.join("deep/er")).unwrap();
        std::fs::write(folder.join("deep/er/file.txt"), b"x").unwrap();
        let path = folder.to_path_buf();
        drop(folder);
        assert!(!path.exists());
    }

    #[test]
    fn two_temp_folders_of_one_name_are_two_folders() {
        let (a, b) = (
            TempFolder::new("temp-twice").unwrap(),
            TempFolder::new("temp-twice").unwrap(),
        );
        assert_ne!(a.path(), b.path());
        assert!(a.is_dir() && b.is_dir());
    }

    /// One left by an earlier run (the same process id, a test killed
    /// before its end) is neither used nor removed.
    #[test]
    fn a_folder_already_there_is_left_alone() {
        let base = TempFolder::new("temp-taken").unwrap();
        std::fs::create_dir(base.join("stem-0")).unwrap();
        std::fs::write(base.join("stem-0/theirs.txt"), b"x").unwrap();
        let next = AtomicU32::new(0);
        let ours = claim(&base, "stem", &next).unwrap();
        assert_eq!(ours, base.join("stem-1"));
        assert_eq!(std::fs::read_dir(&ours).unwrap().count(), 0);
        assert!(base.join("stem-0/theirs.txt").exists());
    }

    /// A link inside goes, not what it points at.
    #[cfg(unix)]
    #[test]
    fn dropping_one_never_follows_a_link_out() {
        let outside = TempFolder::new("temp-outside").unwrap();
        std::fs::write(outside.join("keep.txt"), b"x").unwrap();
        let folder = TempFolder::new("temp-link").unwrap();
        std::os::unix::fs::symlink(outside.path(), folder.join("link")).unwrap();
        drop(folder);
        assert!(outside.join("keep.txt").exists());
    }
}
