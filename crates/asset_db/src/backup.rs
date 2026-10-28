//! A copy before Solis changes an older file (Phase 8, C1;
//! [decision 0053](../../../docs/decisions/0053-compatibility-before-2d.md)).
//!
//! Opening a file written by an older Solis upgrades it in memory
//! (`reflect::upgrade`); the file on disk stays as it was until it's
//! saved. Saving goes through [`save`], which, when the file on disk
//! isn't in the current format:
//!
//! 1. checks the new text reads back as what's being saved;
//! 2. copies the old file to `.solis/backups/<date and time>/`, keeping
//!    its place in the project, and checks the copy;
//! 3. swaps the new file in (atomically), and checks it on disk.
//!
//! If any step fails, the old file stays as it was and the error says
//! why. A file already in the current format is simply saved: Git is its
//! undo. Old backups are trimmed by age and total size, never the newest.

use crate::project::{CACHE_FOLDER, own_folder, read_limited, write_atomically};
use reflect::upgrade::Versioned;
use std::fmt;
use std::path::{Path, PathBuf};

/// The backups' folder, inside the project's `.solis/`.
pub const BACKUPS_FOLDER: &str = "backups";
/// Backups older than this many days go (but never the newest).
pub const KEEP_DAYS: i64 = 90;
/// Past this many bytes in all, the oldest backups go (but never the
/// newest).
pub const MAX_TOTAL_BYTES: u64 = 1 << 30;
/// How a backup's folder is named: sortable, and allowed on Windows.
const STAMP: &str = "%Y-%m-%d %H-%M-%S";
/// The length of a [`STAMP`]: a folder may add " 2", " 3" after it.
const STAMP_LEN: usize = 19;
/// Backups younger than this many days are never taken away to make room,
/// however big (a save game's backup may be its only copy).
pub const KEEP_FRESH_DAYS: i64 = 7;
/// How much of a file is read to find its version: the header line, or a
/// TOML file's top-level keys.
const HEAD_BYTES: usize = 64 << 10;
/// The biggest file copied to the backups (the biggest any format reads).
pub const MAX_BACKUP_BYTES: u64 = 256 << 20;

/// Where a project's backups are.
pub fn folder(root: &Path) -> PathBuf {
    root.join(CACHE_FOLDER).join(BACKUPS_FOLDER)
}

/// Why a file wasn't saved. The old file is as it was.
#[derive(Debug)]
pub enum SaveError {
    /// The file isn't inside the project.
    Outside(PathBuf),
    /// The new text doesn't read back as what's being saved.
    Check(String),
    /// The copy or the new file didn't read back the same.
    ReadBack(PathBuf),
    /// The disk refused.
    Io(std::io::Error),
}

impl fmt::Display for SaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Outside(p) => write!(f, "{} isn't inside the project", p.display()),
            Self::Check(why) => write!(
                f,
                "the upgraded file didn't read back the same ({why}), so the old one was kept"
            ),
            Self::ReadBack(p) => write!(
                f,
                "{} didn't read back the same after writing, so the old file was kept",
                p.display()
            ),
            Self::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for SaveError {}

impl From<std::io::Error> for SaveError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// Saves `bytes` as `path`, a file of `format` in the project at `root`.
/// When the file on disk isn't in the current format (older, newer,
/// damaged, or not text), it's copied to the backups first and `check`
/// must accept the new text: it reads it back and compares it with what's
/// being saved. Returns where the backup went, if one was made.
pub fn save(
    root: &Path,
    path: &Path,
    bytes: &[u8],
    format: &dyn Versioned,
    check: impl FnOnce(&str) -> Result<(), String>,
) -> Result<Option<PathBuf>, SaveError> {
    // Only the start is read to see whether it's current: the whole file
    // is read only when it's going to be copied.
    let head = match read_head(path) {
        Ok(head) => head,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            write_atomically(path, bytes)?;
            return Ok(None);
        }
        Err(e) => return Err(e.into()),
    };
    let current = format.version_of(utf8_prefix(&head)) == Some(format.current());
    let relative = path
        .strip_prefix(root)
        .ok()
        .filter(|r| {
            r.components()
                .all(|c| matches!(c, std::path::Component::Normal(_)))
        })
        .map(Path::to_path_buf);
    // A file inside Solis's own folder (a backup opened to restore it) is
    // never backed up, so a trim can't take away what was just saved.
    let in_own = relative
        .as_ref()
        .is_some_and(|r| r.starts_with(CACHE_FOLDER));
    if current || in_own {
        write_atomically(path, bytes)?;
        return Ok(None);
    }
    let relative = relative.ok_or_else(|| SaveError::Outside(path.to_path_buf()))?;
    let old = read_limited(path, MAX_BACKUP_BYTES)?;
    // 1. The new file reads back as what's being saved.
    let text = std::str::from_utf8(bytes).map_err(|_| SaveError::Check("it isn't text".into()))?;
    check(text).map_err(SaveError::Check)?;
    // 2. The old file is copied, and the copy checked.
    // Solis's own folders, never links planted to send the copy elsewhere.
    own_folder(root, &format!("{CACHE_FOLDER}/{BACKUPS_FOLDER}"))?;
    let (stamp_folder, copy) = free_copy(root, &relative, &old);
    if let Some(parent) = copy.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // Saved twice in one second: the first copy is the older file.
    if !copy.exists() {
        write_atomically(&copy, &old)?;
    }
    if read_limited(&copy, MAX_BACKUP_BYTES)? != old {
        return Err(SaveError::ReadBack(copy));
    }
    // 3. The new file is swapped in, and checked on disk.
    write_atomically(path, bytes)?;
    if read_limited(path, MAX_BACKUP_BYTES.max(bytes.len() as u64))? != bytes {
        let _ = write_atomically(path, &old);
        return Err(SaveError::ReadBack(path.to_path_buf()));
    }
    let _ = trim_keeping(
        root,
        chrono::Local::now().naive_local(),
        MAX_TOTAL_BYTES,
        &stamp_folder,
    );
    Ok(Some(copy))
}

/// The start of the file at `path` (up to [`HEAD_BYTES`]), if it's an
/// ordinary file.
fn read_head(path: &Path) -> std::io::Result<Vec<u8>> {
    use std::io::Read;
    if !std::fs::metadata(path)?.is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "it isn't an ordinary file",
        ));
    }
    let mut head = Vec::with_capacity(HEAD_BYTES);
    std::fs::File::open(path)?
        .take(HEAD_BYTES as u64)
        .read_to_end(&mut head)?;
    Ok(head)
}

/// The part of `bytes` that's whole UTF-8 text (a read cut short may end
/// part-way through a letter).
fn utf8_prefix(bytes: &[u8]) -> &str {
    match std::str::from_utf8(bytes) {
        Ok(text) => text,
        Err(e) => std::str::from_utf8(&bytes[..e.valid_up_to()]).unwrap_or_default(),
    }
}

/// The backup folder for now, and where `relative` is copied in it: the
/// date and time, or with " 2", " 3" after it when a different file is
/// already there under that name (clocks put back an hour).
fn free_copy(root: &Path, relative: &Path, old: &[u8]) -> (PathBuf, PathBuf) {
    let stamp = chrono::Local::now().format(STAMP).to_string();
    for n in 1..100 {
        let name = if n == 1 {
            stamp.clone()
        } else {
            format!("{stamp} {n}")
        };
        let stamp_folder = folder(root).join(name);
        let copy = stamp_folder.join(relative);
        let free = match std::fs::read(&copy) {
            Err(_) => true,
            Ok(there) => there == old,
        };
        if free {
            return (stamp_folder, copy);
        }
    }
    let stamp_folder = folder(root).join(format!("{stamp} {}", std::process::id()));
    let copy = stamp_folder.join(relative);
    (stamp_folder, copy)
}

/// Takes away backups older than [`KEEP_DAYS`] at `now`, then the oldest
/// while they come to more than [`MAX_TOTAL_BYTES`]; never the newest,
/// and never a folder not named as a backup. Returns how many went.
pub fn trim(root: &Path, now: chrono::NaiveDateTime) -> std::io::Result<usize> {
    trim_keeping(root, now, MAX_TOTAL_BYTES, Path::new(""))
}

/// [`trim`], keeping to `max_bytes` in all, and never taking away `keep`
/// (the backup just made). Backups younger than [`KEEP_FRESH_DAYS`] are
/// never taken away for size.
fn trim_keeping(
    root: &Path,
    now: chrono::NaiveDateTime,
    max_bytes: u64,
    keep: &Path,
) -> std::io::Result<usize> {
    let folder = folder(root);
    let mut backups: Vec<(chrono::NaiveDateTime, PathBuf, u64)> = Vec::new();
    let entries = match std::fs::read_dir(&folder) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(e) => return Err(e),
    };
    for entry in entries.flatten() {
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        // Only real folders: never follow a link out of the project.
        if !kind.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let Some(when) = name
            .to_str()
            .filter(|n| n.len() == STAMP_LEN || n.as_bytes().get(STAMP_LEN) == Some(&b' '))
            .and_then(|n| n.get(..STAMP_LEN))
            .and_then(|n| chrono::NaiveDateTime::parse_from_str(n, STAMP).ok())
        else {
            continue;
        };
        if entry.path() == keep {
            continue;
        }
        backups.push((when, entry.path(), size_of(&entry.path())));
    }
    backups.sort_by_key(|(when, _, _)| *when);
    // The newest is never taken away.
    backups.pop();
    let mut total: u64 = backups.iter().map(|(_, _, size)| size).sum();
    let mut gone = 0;
    for (when, path, size) in backups {
        let age = (now - when).num_days();
        let old = age > KEEP_DAYS;
        let too_big = total > max_bytes && age >= KEEP_FRESH_DAYS;
        if old || too_big {
            std::fs::remove_dir_all(&path)?;
            total -= size;
            gone += 1;
        }
    }
    Ok(gone)
}

/// The bytes of the files in a folder, all the way down (links not
/// followed).
fn size_of(folder: &Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return 0;
    };
    entries
        .flatten()
        .map(|entry| match entry.file_type() {
            Ok(kind) if kind.is_dir() => size_of(&entry.path()),
            Ok(kind) if kind.is_file() => entry.metadata().map_or(0, |m| m.len()),
            _ => 0,
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use reflect::upgrade::{Format, header_version};

    /// A made-up format at version 2.
    static NOTE: Format<()> = Format {
        name: "note",
        current: 2,
        version_of: |text| header_version(text, &["note"]),
        steps: &[|_| Ok(())],
    };

    fn project(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "solis-backup-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("assets/levels")).unwrap();
        root
    }

    fn backups(root: &Path) -> Vec<PathBuf> {
        let mut found = Vec::new();
        let mut stack = vec![folder(root)];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
                if entry.file_type().unwrap().is_dir() {
                    stack.push(entry.path());
                } else {
                    found.push(entry.path());
                }
            }
        }
        found
    }

    #[test]
    fn an_older_file_is_copied_before_it_changes() {
        let root = project("older");
        let file = root.join("assets/levels/harbour.note");
        std::fs::write(&file, "note version 1;\nold").unwrap();
        let copy = save(&root, &file, b"note version 2;\nnew", &NOTE, |text| {
            assert!(text.contains("new"));
            Ok(())
        })
        .unwrap()
        .expect("backed up");
        assert_eq!(
            std::fs::read_to_string(&file).unwrap(),
            "note version 2;\nnew"
        );
        assert_eq!(
            std::fs::read_to_string(&copy).unwrap(),
            "note version 1;\nold"
        );
        assert!(
            copy.ends_with("assets/levels/harbour.note"),
            "keeps its place: {copy:?}"
        );
        assert!(copy.starts_with(folder(&root)));
        // Now current: saved again without another copy.
        let again = save(&root, &file, b"note version 2;\nnewer", &NOTE, |_| {
            panic!("only checked when upgrading")
        })
        .unwrap();
        assert_eq!(again, None);
        assert_eq!(backups(&root).len(), 1);
        // A new file: nothing to copy.
        let fresh = root.join("assets/new.note");
        assert_eq!(
            save(&root, &fresh, b"note version 2;", &NOTE, |_| Ok(())).unwrap(),
            None
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_failed_check_or_a_file_outside_changes_nothing() {
        let root = project("refused");
        let file = root.join("assets/a.note");
        std::fs::write(&file, "note version 1;\nold").unwrap();
        let refused = save(&root, &file, b"note version 2;\nlost", &NOTE, |_| {
            Err("a field went missing".into())
        })
        .unwrap_err();
        assert!(matches!(refused, SaveError::Check(_)));
        assert!(
            refused.to_string().contains("a field went missing"),
            "{refused}"
        );
        assert_eq!(
            std::fs::read_to_string(&file).unwrap(),
            "note version 1;\nold"
        );
        assert!(backups(&root).is_empty());
        // Not text: refused too.
        assert!(matches!(
            save(&root, &file, &[0xff, 0xfe], &NOTE, |_| Ok(())),
            Err(SaveError::Check(_))
        ));
        // Outside the project, or climbing out of it: refused.
        let elsewhere =
            std::env::temp_dir().join(format!("solis-outside-{}.note", std::process::id()));
        std::fs::write(&elsewhere, "note version 1;").unwrap();
        assert!(matches!(
            save(&root, &elsewhere, b"note version 2;", &NOTE, |_| Ok(())),
            Err(SaveError::Outside(_))
        ));
        let climbing = root.join("assets/../../escape.note");
        std::fs::write(&climbing, "note version 1;").unwrap();
        assert!(matches!(
            save(&root, &climbing, b"note version 2;", &NOTE, |_| Ok(())),
            Err(SaveError::Outside(_))
        ));
        let _ = std::fs::remove_file(&elsewhere);
        let _ = std::fs::remove_file(&climbing);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn damaged_newer_and_binary_files_are_copied_too() {
        let root = project("odd");
        for (name, old) in [
            ("damaged.note", b"not a note at all".as_slice()),
            ("newer.note", b"note version 9;".as_slice()),
            ("binary.note", [0_u8, 159, 146, 150].as_slice()),
        ] {
            let file = root.join("assets").join(name);
            std::fs::write(&file, old).unwrap();
            let copy = save(&root, &file, b"note version 2;", &NOTE, |_| Ok(()))
                .unwrap()
                .expect("copied");
            assert_eq!(std::fs::read(&copy).unwrap(), old);
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn old_and_big_backups_are_trimmed_but_never_the_newest() {
        let root = project("trim");
        let at = |s: &str| chrono::NaiveDateTime::parse_from_str(s, STAMP).unwrap();
        for (name, bytes) in [
            ("2025-01-01 10-00-00", 10),
            ("2026-09-01 10-00-00", 10),
            ("2026-10-01 10-00-00", 10),
            ("not a backup", 10),
        ] {
            let dir = folder(&root).join(name).join("assets");
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("a.note"), vec![b'x'; bytes]).unwrap();
        }
        // Over 90 days old goes; a folder not named as a backup stays.
        assert_eq!(trim(&root, at("2026-10-28 05-00-00")).unwrap(), 1);
        assert!(!folder(&root).join("2025-01-01 10-00-00").exists());
        assert!(folder(&root).join("not a backup").exists());
        // Far in the future: everything is old, but the newest stays.
        assert_eq!(trim(&root, at("2030-01-01 00-00-00")).unwrap(), 1);
        assert!(folder(&root).join("2026-10-01 10-00-00").exists());
        assert_eq!(trim(&root, at("2030-01-01 00-00-00")).unwrap(), 0);
        // Too big in all: the oldest go first, until it fits.
        for name in [
            "2026-10-20 10-00-00",
            "2026-10-21 10-00-00",
            "2026-10-22 10-00-00",
        ] {
            let dir = folder(&root).join(name);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("a.note"), vec![b'x'; 100]).unwrap();
        }
        assert_eq!(
            trim_keeping(&root, at("2026-10-28 05-00-00"), 150, Path::new("")).unwrap(),
            2
        );
        assert!(folder(&root).join("2026-10-21 10-00-00").exists());
        assert!(folder(&root).join("2026-10-22 10-00-00").exists());
        // No backups at all: nothing to do.
        let empty = project("trim-empty");
        assert_eq!(trim(&empty, at("2030-01-01 00-00-00")).unwrap(), 0);
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&empty);
    }

    #[test]
    fn a_file_inside_solis_own_folder_is_saved_without_a_backup() {
        // Regression (the Phase 8 review): restoring a backup by opening and
        // saving it inside .solis/backups must not trim it away.
        let root = project("own");
        let inside = folder(&root).join("2020-01-01 10-00-00/assets/a.note");
        std::fs::create_dir_all(inside.parent().unwrap()).unwrap();
        std::fs::write(&inside, "note version 1;").unwrap();
        assert_eq!(
            save(&root, &inside, b"note version 2;", &NOTE, |_| Ok(())).unwrap(),
            None
        );
        assert_eq!(std::fs::read_to_string(&inside).unwrap(), "note version 2;");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_backup_just_made_and_fresh_ones_are_never_trimmed_for_size() {
        let root = project("fresh");
        let at = |s: &str| chrono::NaiveDateTime::parse_from_str(s, STAMP).unwrap();
        // A folder dated in the future (another computer's clock) would be
        // "the newest"; the backup just made still stays.
        for name in [
            "2099-01-01 00-00-00",
            "2026-10-27 10-00-00",
            "2026-10-28 04-00-00",
        ] {
            let dir = folder(&root).join(name);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("big.note"), vec![b'x'; 100]).unwrap();
        }
        let just_made = folder(&root).join("2026-10-28 04-00-00");
        let gone = trim_keeping(&root, at("2026-10-28 05-00-00"), 10, &just_made).unwrap();
        assert_eq!(gone, 0, "all younger than a week");
        assert!(just_made.exists());
        // A week on, the older ones go for size; the one just made doesn't.
        let gone = trim_keeping(&root, at("2026-11-10 05-00-00"), 10, &just_made).unwrap();
        assert_eq!(gone, 1);
        assert!(just_made.exists() && folder(&root).join("2099-01-01 00-00-00").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_backup_folder_name_used_again_gets_a_number() {
        // Clocks put back an hour: the same date and time comes round with
        // a different file under that name.
        let root = project("again");
        let relative = Path::new("assets/a.note");
        let (first, copy) = free_copy(&root, relative, b"one");
        std::fs::create_dir_all(copy.parent().unwrap()).unwrap();
        std::fs::write(&copy, "one").unwrap();
        let (same, _) = free_copy(&root, relative, b"one");
        assert_eq!(same, first, "the same file: the same copy");
        let (other, _) = free_copy(&root, relative, b"two");
        assert_ne!(other, first);
        assert!(other.to_string_lossy().ends_with(" 2"), "{other:?}");
        // The trim still reads it as a backup.
        assert_eq!(utf8_prefix(&[b'a', 0xe2, 0x82]), "a");
        let _ = std::fs::remove_dir_all(&root);
    }
}
