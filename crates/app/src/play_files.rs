//! The project's files for playing ([decision 0037](../../../docs/decisions/0037-input-as-named-actions.md)):
//! its input actions (`input.solinput`, beside `project.solis`) and its
//! recordings of play (`recordings/*.solrec`).

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};

/// The folder in a project that keeps recordings of play.
pub const RECORDINGS: &str = "recordings";
/// A recording's file extension.
pub const RECORDING_EXTENSION: &str = "solrec";

/// The project's input actions: its file's, the ready-made ones when it
/// has none, or the ready-made ones and why when its file can't be read.
pub fn load_bindings(root: &Path) -> (input::Bindings, Option<String>) {
    let path = root.join(input::FILE);
    let text = match read_limited(&path, 1 << 20) {
        Ok(Some(text)) => text,
        Ok(None) => return (input::Bindings::default(), None),
        Err(why) => {
            return (
                input::Bindings::default(),
                Some(format!(
                    "Couldn't read {}: {why}; the ready-made actions are used.",
                    input::FILE
                )),
            );
        }
    };
    match input::Bindings::read(&text) {
        Ok(bindings) => (bindings, None),
        Err(why) => (
            input::Bindings::default(),
            Some(format!(
                "{} {why}, so the ready-made actions are used until it's fixed (Edit > Input \
                 actions… writes it again).",
                input::FILE
            )),
        ),
    }
}

/// Writes the project's input actions.
pub fn save_bindings(root: &Path, text: &str) -> std::io::Result<()> {
    write_whole(&root.join(input::FILE), text)
}

/// Saves a recording in the project's recordings folder, named by when it
/// was made; gives back where.
pub fn save_recording(root: &Path, text: &str) -> std::io::Result<PathBuf> {
    let folder = root.join(RECORDINGS);
    std::fs::create_dir_all(&folder)?;
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let mut path = folder.join(format!("play-{secs}.{RECORDING_EXTENSION}"));
    let mut n = 2;
    while path.exists() {
        path = folder.join(format!("play-{secs}-{n}.{RECORDING_EXTENSION}"));
        n += 1;
    }
    write_whole(&path, text)?;
    Ok(path)
}

/// Writes `text` through a file beside `path`, then puts it in place, so
/// a crash never leaves half a file.
fn write_whole(path: &Path, text: &str) -> std::io::Result<()> {
    let partial = path.with_extension("partial");
    std::fs::write(&partial, text)?;
    std::fs::rename(&partial, path)
}

/// A file's text, if it's there and within `limit` bytes.
pub fn read_limited(path: &Path, limit: u64) -> Result<Option<String>, String> {
    let size = match std::fs::metadata(path) {
        Ok(m) => m.len(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.to_string()),
    };
    if size > limit {
        return Err(format!("it's bigger than {} MB", limit >> 20));
    }
    std::fs::read_to_string(path)
        .map(Some)
        .map_err(|e| e.to_string())
}

/// The Open window for a recording, on a thread of its own.
#[derive(Default)]
pub struct RecordingDialog {
    answer: Option<Receiver<Option<PathBuf>>>,
}

impl RecordingDialog {
    /// Shows the Open window, starting in `folder`.
    pub fn ask(&mut self, folder: Option<PathBuf>) {
        if self.answer.is_some() {
            return;
        }
        let (sender, receiver) = channel();
        std::thread::spawn(move || {
            let mut window = rfd::FileDialog::new()
                .set_title("Replay a recording")
                .add_filter("Recording", &[RECORDING_EXTENSION]);
            if let Some(folder) = folder {
                window = window.set_directory(folder);
            }
            let _ = sender.send(window.pick_file());
        });
        self.answer = Some(receiver);
    }

    /// The file chosen, once the window closed (`Some(None)`: cancelled).
    pub fn chosen(&mut self) -> Option<Option<PathBuf>> {
        let answer = self.answer.as_ref()?.try_recv().ok()?;
        self.answer = None;
        Some(answer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("solis-play-files-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn bindings_are_the_project_s_or_the_ready_made_ones_with_why() {
        let root = scratch("bindings");
        assert_eq!(load_bindings(&root), (input::Bindings::default(), None));
        let mut mine = input::Bindings::default();
        mine.rebind("Jump", "J");
        save_bindings(&root, &mine.write()).unwrap();
        assert_eq!(load_bindings(&root), (mine, None));
        std::fs::write(root.join(input::FILE), "[[action]]\nname = \"bad name\"\n").unwrap();
        let (bindings, note) = load_bindings(&root);
        assert_eq!(bindings, input::Bindings::default());
        assert!(note.unwrap().contains("ready-made"));
        std::fs::write(root.join(input::FILE), vec![b'#'; 2 << 20]).unwrap();
        assert!(load_bindings(&root).1.unwrap().contains("bigger"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn recordings_go_in_their_folder_and_never_replace_one() {
        let root = scratch("recordings");
        let a = save_recording(&root, "one").unwrap();
        let b = save_recording(&root, "two").unwrap();
        assert_ne!(a, b);
        assert_eq!(a.parent().unwrap(), root.join(RECORDINGS));
        assert_eq!(std::fs::read_to_string(a).unwrap(), "one");
        assert_eq!(std::fs::read_to_string(b).unwrap(), "two");
        let _ = std::fs::remove_dir_all(root);
    }
}
