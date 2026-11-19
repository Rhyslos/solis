//! The project's fonts the game's screens use (stage 1, text layout),
//! read on a worker so the frame never waits on the disk (rule 10): each
//! file asked for is read once, and again when it changes on disk, within
//! `ui::text::MAX_FONT_BYTES`.

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

/// A file's size and time of last change: it's read again when they
/// change.
type Stamp = Option<(u64, Option<std::time::SystemTime>)>;

/// A font file read, or why it couldn't be: its path inside the assets.
type Read = (String, Result<Vec<u8>, String>);

/// How often the files asked for are looked at again for changes.
const LOOK_AGAIN: Duration = Duration::from_secs(1);

/// The fonts asked for, and the reads on their way.
#[derive(Debug)]
pub(crate) struct FontReads {
    wanted: BTreeSet<String>,
    tried: HashMap<String, Stamp>,
    looked: Option<Instant>,
    send: Sender<Read>,
    receive: Receiver<Read>,
}

impl Default for FontReads {
    fn default() -> Self {
        let (send, receive) = channel();
        Self {
            wanted: BTreeSet::new(),
            tried: HashMap::new(),
            looked: None,
            send,
            receive,
        }
    }
}

fn stamp(path: &Path) -> Stamp {
    let meta = std::fs::metadata(path).ok()?;
    Some((meta.len(), meta.modified().ok()))
}

/// The font file at `path`'s bytes, or why it can't be read, in plain
/// words.
pub(crate) fn read_font(path: &Path) -> Result<Vec<u8>, String> {
    let limit = ui::text::MAX_FONT_BYTES as u64;
    asset_db::read_limited(path, limit).map_err(|e| {
        if e.kind() == std::io::ErrorKind::FileTooLarge {
            format!("it's bigger than {} MB, the most a font may be", limit >> 20)
        } else {
            format!("it couldn't be read ({e})")
        }
    })
}

impl FontReads {
    /// Reads the fonts `workbench` asks for from `assets`: those newly
    /// asked for, and (once a second) those changed on disk, each on a
    /// worker; hands what's been read to `workbench`.
    pub(crate) fn follow(&mut self, workbench: &mut editor::Workbench, assets: Option<&Path>) {
        while let Ok((path, read)) = self.receive.try_recv() {
            if self.wanted.contains(&path) {
                workbench.set_font_file(path, read);
            }
        }
        let Some(assets) = assets else {
            return;
        };
        let asked = workbench.take_wanted_fonts();
        let due = self.looked.is_none_or(|at| at.elapsed() >= LOOK_AGAIN);
        if asked.is_none() && !due {
            return;
        }
        if let Some(wanted) = asked {
            self.tried.retain(|p, _| wanted.contains(p));
            self.wanted = wanted;
        }
        self.looked = Some(Instant::now());
        let to_read: Vec<(String, PathBuf)> = self
            .wanted
            .iter()
            .filter_map(|path| {
                let full = assets.join(path);
                let now = stamp(&full);
                (self.tried.insert(path.clone(), now) != Some(now)).then(|| (path.clone(), full))
            })
            .collect();
        if to_read.is_empty() {
            return;
        }
        let send = self.send.clone();
        let started = std::thread::Builder::new()
            .name("font reads".into())
            .spawn(move || {
                for (path, full) in to_read {
                    if send.send((path, read_font(&full))).is_err() {
                        return;
                    }
                }
            });
        if started.is_err() {
            // Tried again at the next look.
            self.tried.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_font_is_read_within_its_limit_and_a_missing_one_says_why() {
        let dir = reflect::test_support::TempFolder::new("font-reads").unwrap();
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("Lora.ttf");
        std::fs::write(&file, b"bytes").unwrap();
        assert_eq!(read_font(&file), Ok(b"bytes".to_vec()));
        assert!(
            read_font(&dir.join("gone.ttf"))
                .unwrap_err()
                .contains("couldn't be read")
        );
        let big = dir.join("big.ttf");
        let f = std::fs::File::create(&big).unwrap();
        f.set_len(ui::text::MAX_FONT_BYTES as u64 + 1).unwrap();
        assert!(read_font(&big).unwrap_err().contains("bigger than 32 MB"));
    }
}
