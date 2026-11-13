//! Text files shown in Properties (`editor::text_view`): the one picked is
//! read when it's picked and again whenever it changes on disk, at most
//! `editor::text_view::MAX_TEXT_BYTES`, as text.

use std::path::Path;

/// A file's size and time of last change: it's read again when they
/// change.
pub type Stamp = Option<(u64, Option<std::time::SystemTime>)>;

/// The file last read: its path inside the assets and its stamp then.
#[derive(Debug, Default)]
pub struct TextFiles {
    read: Option<(String, Stamp)>,
}

fn stamp(path: &Path) -> Stamp {
    let meta = std::fs::metadata(path).ok()?;
    Some((meta.len(), meta.modified().ok()))
}

/// The text of the file at `path`, or why it can't be shown.
pub fn read_text(path: &Path) -> editor::text_view::TextRead {
    let limit = editor::text_view::MAX_TEXT_BYTES;
    match asset_db::read_limited(path, limit) {
        Ok(bytes) if bytes.contains(&0) => Err("it isn't text".to_owned()),
        Ok(bytes) => Ok(String::from_utf8(bytes)
            .unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned())),
        Err(e) if e.kind() == std::io::ErrorKind::FileTooLarge => Err(format!(
            "it's bigger than {} KB: open it in its program",
            limit >> 10
        )),
        Err(e) => Err(format!("it couldn't be read ({e})")),
    }
}

impl TextFiles {
    /// Reads the text file `workbench` shows in Properties, if any, from
    /// `assets`, when it's newly picked or has changed on disk.
    pub fn follow(&mut self, workbench: &mut editor::Workbench, assets: Option<&Path>) {
        let (Some(wanted), Some(assets)) = (workbench.text_file_wanted(), assets) else {
            self.read = None;
            return;
        };
        let wanted = wanted.to_owned();
        let full = assets.join(&wanted);
        let now = stamp(&full);
        if self
            .read
            .as_ref()
            .is_some_and(|(path, then)| *path == wanted && *then == now)
        {
            return;
        }
        workbench.set_text_file(wanted.clone(), read_text(&full));
        self.read = Some((wanted, now));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_is_read_and_what_isn_t_text_says_why() {
        let dir = reflect::test_support::TempFolder::new("text-files").unwrap();
        std::fs::create_dir_all(&dir).unwrap();
        let notes = dir.join("notes.md");
        std::fs::write(&notes, "# Notes\nhéllo").unwrap();
        assert_eq!(read_text(&notes), Ok("# Notes\nhéllo".to_owned()));
        let binary = dir.join("odd.txt");
        std::fs::write(&binary, [0x41, 0, 0x42]).unwrap();
        assert_eq!(read_text(&binary), Err("it isn't text".to_owned()));
        let latin = dir.join("latin.txt");
        std::fs::write(&latin, [b'c', 0xe9, b'!']).unwrap();
        assert_eq!(read_text(&latin), Ok("c\u{fffd}!".to_owned()));
        let big = dir.join("big.txt");
        std::fs::write(
            &big,
            vec![b'a'; editor::text_view::MAX_TEXT_BYTES as usize + 1],
        )
        .unwrap();
        assert!(read_text(&big).unwrap_err().contains("bigger than 512 KB"));
        assert!(
            read_text(&dir.join("gone.md"))
                .unwrap_err()
                .contains("couldn't be read")
        );
    }
}
