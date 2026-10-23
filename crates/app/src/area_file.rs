//! Area files on disk ([decision 0020](../../../docs/decisions/0020-scene-files-in-lust-style.md)):
//! reading and saving them, the Open and Save windows, and noticing when
//! the open one changes on disk (a Git pull, another program).

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant, SystemTime};

/// The extension of area files.
pub const EXTENSION: &str = "solarea";
/// How often the open area's file is looked at for changes.
const CHECK_EVERY: Duration = Duration::from_secs(1);

/// What a file window was opened for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    /// Opening an area.
    Open,
    /// Saving the area under a new name.
    Save,
}

/// A file's size and time of last change.
type Stamp = (u64, Option<SystemTime>);

fn stamp(path: &Path) -> Option<Stamp> {
    let meta = std::fs::metadata(path).ok()?;
    Some((meta.len(), meta.modified().ok()))
}

/// The open area's file, as the window sees it.
#[derive(Debug, Default)]
pub struct AreaFiles {
    dialog: Option<(Purpose, Receiver<Option<PathBuf>>)>,
    /// The open area's file, and how it looked when last read or written.
    watched: Option<(PathBuf, Option<Stamp>)>,
    last_check: Option<Instant>,
}

impl AreaFiles {
    /// Reads the area or assembly file at `path`, within the reader's
    /// limits (placed assemblies not yet expanded).
    pub fn read(path: &Path, registry: &reflect::Registry) -> Result<area::text::AreaFile, String> {
        let size = std::fs::metadata(path).map_err(|e| e.to_string())?.len();
        if size > area::text::MAX_BYTES as u64 {
            return Err(format!(
                "it's {} MB; the most read is {} MB",
                size >> 20,
                area::text::MAX_BYTES >> 20
            ));
        }
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        let text = String::from_utf8(bytes).map_err(|_| "it isn't text".to_string())?;
        area::text::read_file(&text, registry).map_err(|e| e.to_string())
    }

    /// Writes `text` to `path` through a temporary file and a swap, and
    /// watches it from now on.
    pub fn save(&mut self, path: &Path, text: &str) -> std::io::Result<()> {
        if let Some(folder) = path.parent() {
            std::fs::create_dir_all(folder)?;
        }
        asset_db::write_atomically(path, text.as_bytes())?;
        self.watch(path);
        Ok(())
    }

    /// Watches `path` (just opened or saved) for changes from now on.
    pub fn watch(&mut self, path: &Path) {
        self.watched = Some((path.to_path_buf(), stamp(path)));
    }

    /// Stops watching (a new, unsaved area).
    pub fn forget(&mut self) {
        self.watched = None;
    }

    /// The watched file, if it changed since last read or written. Looks at
    /// most once a second, and never waits.
    pub fn changed_on_disk(&mut self) -> Option<PathBuf> {
        if self.last_check.is_some_and(|t| t.elapsed() < CHECK_EVERY) {
            return None;
        }
        self.last_check = Some(Instant::now());
        let (path, seen) = self.watched.as_mut()?;
        let now = stamp(path);
        // A file being removed isn't a change to load.
        if now.is_none() || now == *seen {
            return None;
        }
        *seen = now;
        Some(path.clone())
    }

    /// Shows the system's Open or Save window, starting in `folder`, on a
    /// thread of its own. The choice comes back from [`Self::chosen`].
    pub fn ask(&mut self, purpose: Purpose, folder: Option<PathBuf>, name: &str) {
        if self.dialog.is_some() {
            return;
        }
        let (sender, receiver) = channel();
        let name = format!("{name}.{EXTENSION}");
        std::thread::spawn(move || {
            let mut window = rfd::FileDialog::new().add_filter("Area", &[EXTENSION]);
            if let Some(folder) = folder {
                window = window.set_directory(folder);
            }
            let choice = match purpose {
                Purpose::Open => window.set_title("Open area").pick_file(),
                Purpose::Save => window
                    .set_title("Save area")
                    .set_file_name(name)
                    .save_file(),
            };
            let _ = sender.send(choice);
        });
        self.dialog = Some((purpose, receiver));
    }

    /// The file chosen in the Open or Save window, and what for, once it's
    /// closed: `Some((purpose, None))` if it was cancelled.
    pub fn chosen(&mut self) -> Option<(Purpose, Option<PathBuf>)> {
        let (purpose, receiver) = self.dialog.as_ref()?;
        let purpose = *purpose;
        match receiver.try_recv() {
            Ok(choice) => {
                self.dialog = None;
                Some((purpose, choice.map(|p| with_extension(&p))))
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => None,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.dialog = None;
                Some((purpose, None))
            }
        }
    }
}

/// `path`, ending in `.solarea` (a Save window may leave it off).
pub fn with_extension(path: &Path) -> PathBuf {
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case(EXTENSION))
    {
        path.to_path_buf()
    } else {
        let mut name = path.as_os_str().to_owned();
        name.push(".");
        name.push(EXTENSION);
        PathBuf::from(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(name: &str) -> PathBuf {
        let f = std::env::temp_dir().join(format!("solis-areas-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&f);
        std::fs::create_dir_all(&f).unwrap();
        f
    }

    #[test]
    fn an_area_is_saved_read_back_and_its_changes_noticed() {
        let dir = folder("save");
        let registry = area::builtin::registry().unwrap();
        let area = area::builtin::starter_area();
        let text = area::text::write(&area, &registry, &|_| None);
        let mut files = AreaFiles::default();
        let path = dir.join("areas/harbour.solarea");
        files.save(&path, &text).unwrap();
        assert_eq!(AreaFiles::read(&path, &registry).unwrap().area, area);
        assert_eq!(
            files.changed_on_disk(),
            None,
            "our own write isn't a change"
        );
        // Someone else writes it (a Git pull).
        std::thread::sleep(Duration::from_millis(1100));
        std::fs::write(&path, format!("{text}\n-- edited\n")).unwrap();
        assert_eq!(files.changed_on_disk(), Some(path.clone()));
        std::thread::sleep(Duration::from_millis(1100));
        assert_eq!(files.changed_on_disk(), None, "reported once");
        files.forget();
        assert_eq!(files.changed_on_disk(), None);
    }

    #[test]
    fn bad_files_are_explained() {
        let dir = folder("bad");
        let registry = area::builtin::registry().unwrap();
        let path = dir.join("x.solarea");
        std::fs::write(&path, [0xff, 0xfe, 0x00]).unwrap();
        assert!(
            AreaFiles::read(&path, &registry)
                .unwrap_err()
                .contains("text")
        );
        std::fs::write(&path, "area version 9;").unwrap();
        assert!(
            AreaFiles::read(&path, &registry)
                .unwrap_err()
                .contains("newer")
        );
        assert!(AreaFiles::read(&dir.join("missing.solarea"), &registry).is_err());
    }

    #[test]
    fn saved_names_get_the_extension() {
        assert_eq!(
            with_extension(Path::new("a/harbour")),
            Path::new("a/harbour.solarea")
        );
        assert_eq!(
            with_extension(Path::new("a/h.SOLAREA")),
            Path::new("a/h.SOLAREA")
        );
        assert_eq!(
            with_extension(Path::new("a/h.txt")),
            Path::new("a/h.txt.solarea")
        );
    }
}
