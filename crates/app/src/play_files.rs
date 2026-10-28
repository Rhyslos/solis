//! The project's files for playing ([decision 0037](../../../docs/decisions/0037-input-as-named-actions.md)):
//! its input actions (`input.solinput`, beside `project.solis`), its
//! buses (`mixer.solmix`, Phase 7, S4) and its recordings of play
//! (`recordings/*.solrec`).

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};

/// The folder in a project that keeps save games (decision 0038).
pub const SAVES: &str = "saves";
/// A save game's file extension.
pub const SAVE_EXTENSION: &str = "solsave";
/// The biggest save game read.
pub const MAX_SAVE_BYTES: u64 = 256 << 20;

/// Writes a save game as `slot` (already checked: letters, digits, `_`
/// and `-`) in the project's saves folder, replacing one there; gives
/// back where.
/// Gives back where it went, and where a copy of an older save in that
/// slot was kept, if there was one (Phase 8, C2).
pub fn save_game(
    root: &Path,
    slot: &str,
    text: &str,
) -> std::io::Result<(PathBuf, Option<PathBuf>)> {
    save_game_within(root, slot, text, MAX_SAVES_BYTES)
}

/// [`save_game`], with the saves folder holding at most `room` bytes.
fn save_game_within(
    root: &Path,
    slot: &str,
    text: &str,
    room: u64,
) -> std::io::Result<(PathBuf, Option<PathBuf>)> {
    if !script::valid_slot(slot) {
        return Err(std::io::Error::other("that isn't a save slot's name"));
    }
    let folder = asset_db::own_folder(root, SAVES)?;
    let path = folder.join(format!("{slot}.{SAVE_EXTENSION}"));
    // The other saves: how many, and how big together.
    let (mut count, mut bytes) = (0usize, 0u64);
    for entry in std::fs::read_dir(&folder)?.filter_map(Result::ok) {
        let other = entry.path();
        if other == path
            || !other
                .extension()
                .is_some_and(|x| x.eq_ignore_ascii_case(SAVE_EXTENSION))
        {
            continue;
        }
        count += 1;
        bytes = bytes.saturating_add(entry.metadata().map_or(0, |m| m.len()));
    }
    if count >= MAX_SAVE_SLOTS {
        return Err(std::io::Error::other(format!(
            "the saves folder already has {MAX_SAVE_SLOTS} saves; game code can replace one, \
             but not add more"
        )));
    }
    if bytes.saturating_add(text.len() as u64) > room {
        return Err(std::io::Error::other(format!(
            "the saves folder would hold more than {} MB; delete some saves to make room",
            room >> 20
        )));
    }
    // A save in that slot made by another Solis, earlier or newer, is
    // kept in the backups first: the saves folder isn't in Git, so it's
    // the only copy. Only its first line is read to tell (a save can be
    // 256 MB); a damaged one or one of this version is simply replaced.
    let older = first_line(&path)
        .and_then(|line| (editor::SAVE_FORMAT.version_of)(&line))
        .is_some_and(|v| v != editor::SAVE_FORMAT.current);
    if older {
        let copy = crate::upgrades::save(root, &path, text, &editor::SAVE_FORMAT, |t| {
            editor::save_read_back(t)
        })
        .map_err(std::io::Error::other)?;
        return Ok((path, copy));
    }
    write_whole(&path, text)?;
    Ok((path, None))
}

/// The first line of the file at `path` (at most 64 bytes of it), if it's
/// an ordinary file.
fn first_line(path: &Path) -> Option<String> {
    use std::io::Read;
    if !std::fs::symlink_metadata(path).ok()?.is_file() {
        return None;
    }
    let mut head = Vec::with_capacity(64);
    std::fs::File::open(path)
        .ok()?
        .take(64)
        .read_to_end(&mut head)
        .ok()?;
    let text = String::from_utf8_lossy(&head);
    Some(text.split('\n').next().unwrap_or_default().to_owned())
}

/// The most the saves folder holds, all saves together (game code saving
/// big games in every slot would otherwise fill the disk).
pub const MAX_SAVES_BYTES: u64 = 2 << 30;

/// The most save slots a project keeps (game code saving under a new
/// name every tick would otherwise fill the disk).
pub const MAX_SAVE_SLOTS: usize = 100;

/// Where the save game `slot` is kept, if the name is usable.
pub fn save_path(root: &Path, slot: &str) -> Option<PathBuf> {
    script::valid_slot(slot).then(|| root.join(SAVES).join(format!("{slot}.{SAVE_EXTENSION}")))
}

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
        Ok(bindings) => (
            bindings,
            crate::upgrades::older_note(input::FILE, &text, &input::BINDINGS_FORMAT),
        ),
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

/// The most backups of a damaged input actions or mixer file kept
/// (`input.solinput.bak`, `.bak2` … `.bak9`).
const MAX_BINDINGS_BACKUPS: usize = 9;

/// The project's buses (`mixer.solmix`), or the standard ones when it has
/// none; with notes for the Console: what was put right, or why the file
/// couldn't be read (then the standard buses are used until it's saved
/// again, which keeps the damaged one as a backup).
pub fn load_mixer(root: &Path) -> (sound::Mixer, Vec<String>) {
    let file = sound::mixer::FILE_NAME;
    let limit = sound::mixer::MAX_BYTES as u64;
    let text = match read_limited(&root.join(file), limit) {
        Ok(Some(text)) => text,
        Ok(None) => return (sound::Mixer::default(), Vec::new()),
        Err(why) => {
            return (
                sound::Mixer::default(),
                vec![format!(
                    "Couldn't read {file}: {why}; the standard buses are used."
                )],
            );
        }
    };
    match sound::mixer::read(&text) {
        Ok(read) => (
            read.mixer,
            crate::upgrades::older_note(file, &text, &sound::mixer::FORMAT)
                .into_iter()
                .chain(read.notes.into_iter().map(|n| format!("{file}, {n}.")))
                .collect(),
        ),
        Err(why) => (
            sound::Mixer::default(),
            vec![format!(
                "{file} {why}, so the standard buses are used until it's fixed (a change in \
                 the Mixer writes it again, keeping this one as a backup)."
            )],
        ),
    }
}

/// What saving kept of the file that was there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kept {
    /// It couldn't be read, so it was kept beside it under this name.
    Damaged(String),
    /// It was in an older format, so a copy is in the backups, here
    /// (Phase 8, C1).
    Older(PathBuf),
}

/// Writes the project's buses, keeping a damaged file first as a backup
/// as [`save_bindings`] does, and an older one in the backups; says what
/// it kept.
pub fn save_mixer(root: &Path, mixer: &sound::Mixer) -> std::io::Result<Option<Kept>> {
    save_keeping_damaged(
        root,
        sound::mixer::FILE_NAME,
        &sound::mixer::write(mixer),
        &sound::mixer::FORMAT,
        |text| {
            sound::mixer::read(text)
                .map(|read| sound::mixer::write(&read.mixer))
                .map_err(|e| e.to_string())
        },
    )
}

/// Writes the project's input actions. A file there that can't be read
/// (a mistake made by hand, so the ready-made actions were used) is kept
/// first as `input.solinput.bak` (or `.bak2` and on, up to `.bak9`, never
/// replacing one: past that the oldest is replaced), so its actions
/// aren't lost; one in an older format is copied to the backups first.
/// Says what it kept.
pub fn save_bindings(root: &Path, text: &str) -> std::io::Result<Option<Kept>> {
    save_keeping_damaged(root, input::FILE, text, &input::BINDINGS_FORMAT, |text| {
        input::Bindings::read(text)
            .map(|b| b.write())
            .map_err(|e| e.to_string())
    })
}

/// Writes `text` as `file` in `root`, keeping the file there first as a
/// backup beside it when it can't be read (`read_back` fails on it), or
/// in the backups when it's readable but in an older format.
fn save_keeping_damaged(
    root: &Path,
    file: &str,
    text: &str,
    format: &dyn reflect::upgrade::Versioned,
    read_back: impl Fn(&str) -> Result<String, String>,
) -> std::io::Result<Option<Kept>> {
    let path = root.join(file);
    let old = read_limited(&path, 1 << 20);
    // Made by a newer Solis: never overwritten as if it were damaged (the
    // Phase 8 review), since this build can't write what it holds.
    if let Ok(Some(old)) = &old
        && let Some(found) = format.version_of(old)
        && found > format.current()
    {
        return Err(std::io::Error::other(format!(
            "{file} was saved by a newer version (format {found}; this one writes {}). Update \
             to change it, so nothing in it is lost",
            format.current()
        )));
    }
    let damaged = match old {
        Ok(Some(old)) => read_back(&old).is_err(),
        Ok(None) => false,
        Err(_) => path.exists(),
    };
    if !damaged {
        return crate::upgrades::save(root, &path, text, format, &read_back)
            .map(|copy| copy.map(Kept::Older))
            .map_err(std::io::Error::other);
    }
    let names: Vec<String> = (1..=MAX_BINDINGS_BACKUPS)
        .map(|n| match n {
            1 => format!("{file}.bak"),
            n => format!("{file}.bak{n}"),
        })
        .collect();
    // Names are looked at without following links (a dangling link
    // isn't "free"), and the backup is written as a new file put in
    // place, never through a link a project planted under its name.
    let there = |n: &String| std::fs::symlink_metadata(root.join(n)).ok();
    let name = names
        .iter()
        .find(|n| there(n).is_none())
        .or_else(|| {
            names
                .iter()
                .min_by_key(|n| there(n).and_then(|m| m.modified().ok()))
        })
        .cloned()
        .unwrap_or_else(|| names[0].clone());
    let old = asset_db::read_limited(&path, 1 << 20)?;
    asset_db::write_atomically(&root.join(&name), &old)?;
    write_whole(&path, text)?;
    Ok(Some(Kept::Damaged(name)))
}

/// Saves a recording in the project's recordings folder, named by when it
/// was made; gives back where.
pub fn save_recording(root: &Path, text: &str) -> std::io::Result<PathBuf> {
    let folder = asset_db::own_folder(root, RECORDINGS)?;
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
    // A fresh, uniquely named file renamed into place: never through a
    // file or link already there.
    asset_db::write_atomically(path, text.as_bytes())
}

/// A file's text, if it's there and within `limit` bytes.
pub fn read_limited(path: &Path, limit: u64) -> Result<Option<String>, String> {
    let bytes = match asset_db::read_limited(path, limit) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) if e.kind() == std::io::ErrorKind::FileTooLarge => {
            return Err(format!("it's bigger than {} MB", limit >> 20));
        }
        Err(e) => return Err(e.to_string()),
    };
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|_| "it isn't text".to_string())
}

/// What an Open window was asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayFile {
    /// A recording to replay.
    Recording,
    /// A save game to load.
    Save,
}

/// The Open window for a recording or a save game, on a thread of its
/// own.
#[derive(Default)]
pub struct RecordingDialog {
    answer: Option<(PlayFile, Receiver<Option<PathBuf>>)>,
}

impl RecordingDialog {
    /// Shows the Open window for `what`, starting in `folder`.
    pub fn ask(&mut self, what: PlayFile, folder: Option<PathBuf>) {
        if self.answer.is_some() {
            return;
        }
        let (sender, receiver) = channel();
        std::thread::spawn(move || {
            let (title, kind, extension) = match what {
                PlayFile::Recording => ("Replay a recording", "Recording", RECORDING_EXTENSION),
                PlayFile::Save => ("Load a save", "Save game", SAVE_EXTENSION),
            };
            let mut window = rfd::FileDialog::new()
                .set_title(title)
                .add_filter(kind, &[extension]);
            if let Some(folder) = folder {
                window = window.set_directory(folder);
            }
            let _ = sender.send(window.pick_file());
        });
        self.answer = Some((what, receiver));
    }

    /// What was asked for and the file chosen, once the window closed
    /// (`None` inside: cancelled).
    pub fn chosen(&mut self) -> Option<(PlayFile, Option<PathBuf>)> {
        let (what, receiver) = self.answer.as_ref()?;
        let answer = match receiver.try_recv() {
            Ok(answer) => answer,
            Err(std::sync::mpsc::TryRecvError::Empty) => return None,
            // The window's thread ended without answering: as cancelled,
            // so the next Replay or Load opens one again.
            Err(std::sync::mpsc::TryRecvError::Disconnected) => None,
        };
        let what = *what;
        self.answer = None;
        Some((what, answer))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The name a damaged file was kept under, if it was.
    fn kept_as(kept: Option<Kept>) -> Option<String> {
        match kept {
            Some(Kept::Damaged(name)) => Some(name),
            _ => None,
        }
    }

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("solis-play-files-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_mixer_is_the_project_s_or_the_standard_one_and_a_damaged_one_is_kept() {
        let root = scratch("mixer");
        // None yet: the standard buses, nothing to say.
        assert_eq!(load_mixer(&root), (sound::Mixer::default(), Vec::new()));
        let mut mine = sound::Mixer::default();
        mine.add_bus("Effects").unwrap();
        mine.buses[1].volume_db = -4.0;
        assert_eq!(save_mixer(&root, &mine).unwrap(), None);
        assert_eq!(load_mixer(&root), (mine.clone(), Vec::new()));
        // Out of range by hand: brought in, and said.
        let file = root.join(sound::mixer::FILE_NAME);
        let text = std::fs::read_to_string(&file)
            .unwrap()
            .replace("-4.0", "-400.0");
        std::fs::write(&file, text).unwrap();
        let (read, notes) = load_mixer(&root);
        assert_eq!(read.buses[1].volume_db, -60.0);
        assert_eq!(notes.len(), 1, "{notes:?}");
        // Damaged: the standard buses, why, and kept as a backup when the
        // Mixer saves again.
        std::fs::write(&file, "mixer version 1;\nbus1 = 3;\n").unwrap();
        let (read, notes) = load_mixer(&root);
        assert_eq!(read, sound::Mixer::default());
        assert!(notes[0].contains("line 2"), "{notes:?}");
        assert_eq!(
            kept_as(save_mixer(&root, &read).unwrap()).as_deref(),
            Some("mixer.solmix.bak")
        );
        assert!(root.join("mixer.solmix.bak").is_file());
        assert_eq!(load_mixer(&root), (sound::Mixer::default(), Vec::new()));
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
    fn a_damaged_bindings_file_is_kept_before_it_is_written_again() {
        let root = scratch("bindings-backup");
        let damaged = "[[action]]\nname = \"MyAction\"\ninputs = [\"Q\"]\ntypo\n";
        std::fs::write(root.join(input::FILE), damaged).unwrap();
        let written = input::Bindings::default().write();
        let kept = save_bindings(&root, &written).unwrap();
        assert_eq!(kept_as(kept).as_deref(), Some("input.solinput.bak"));
        let backup = root.join("input.solinput.bak");
        assert_eq!(std::fs::read_to_string(&backup).unwrap(), damaged);
        assert_eq!(
            std::fs::read_to_string(root.join(input::FILE)).unwrap(),
            written
        );
        // A readable file isn't backed up; a second mistake gets a
        // backup of its own, the first one kept.
        assert_eq!(save_bindings(&root, &written).unwrap(), None);
        std::fs::write(root.join(input::FILE), "broken again [[").unwrap();
        assert_eq!(
            kept_as(save_bindings(&root, &written).unwrap()).as_deref(),
            Some("input.solinput.bak2")
        );
        assert_eq!(std::fs::read_to_string(&backup).unwrap(), damaged);
        assert_eq!(
            std::fs::read_to_string(root.join("input.solinput.bak2")).unwrap(),
            "broken again [["
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn a_project_keeps_at_most_its_number_of_saves() {
        let root = scratch("many-saves");
        for n in 0..MAX_SAVE_SLOTS {
            save_game(&root, &format!("slot_{n}"), "x").unwrap();
        }
        assert!(save_game(&root, "one_more", "x").is_err(), "no more slots");
        save_game(&root, "slot_3", "replaced").unwrap();
        assert_eq!(
            std::fs::read_to_string(save_path(&root, "slot_3").unwrap()).unwrap(),
            "replaced"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn a_link_planted_in_the_project_never_sends_a_write_elsewhere() {
        // The security review: the bindings and saves were written through
        // a fixed name ("input.partial") that a project could ship as a
        // link to a file outside it.
        let root = scratch("planted-link");
        let outside = root.with_extension("outside");
        std::fs::write(&outside, "mine").unwrap();
        std::os::unix::fs::symlink(&outside, root.join("input.partial")).unwrap();
        save_bindings(&root, &input::Bindings::default().write()).unwrap();
        assert_eq!(std::fs::read_to_string(&outside).unwrap(), "mine");
        // The third security audit: a damaged file's backup was copied
        // through a link planted as `input.solinput.bak` (a dangling one
        // looked free), and through the oldest when all nine were taken.
        let target = root.with_extension("startup");
        std::fs::write(root.join(input::FILE), "@echo off\r\ncalc.exe [[").unwrap();
        std::os::unix::fs::symlink(&target, root.join("input.solinput.bak")).unwrap();
        let kept = save_bindings(&root, &input::Bindings::default().write()).unwrap();
        assert!(!target.exists(), "nothing written through the link");
        assert_eq!(kept_as(kept).as_deref(), Some("input.solinput.bak2"));
        for n in 3..=9 {
            std::os::unix::fs::symlink(&outside, root.join(format!("input.solinput.bak{n}")))
                .unwrap();
        }
        std::fs::write(root.join(input::FILE), "broken [[").unwrap();
        save_bindings(&root, &input::Bindings::default().write()).unwrap();
        assert_eq!(std::fs::read_to_string(&outside).unwrap(), "mine");
        assert!(!target.exists());
        // Saves and recordings never go into a folder that's a link.
        let elsewhere = root.with_extension("elsewhere");
        std::fs::create_dir_all(&elsewhere).unwrap();
        std::os::unix::fs::symlink(&elsewhere, root.join(SAVES)).unwrap();
        std::os::unix::fs::symlink(&elsewhere, root.join(RECORDINGS)).unwrap();
        assert!(save_game(&root, "quick", "x").is_err());
        assert!(save_recording(&root, "x").is_err());
        assert!(std::fs::read_dir(&elsewhere).unwrap().next().is_none());
        let _ = std::fs::remove_dir_all(elsewhere);
        let _ = std::fs::remove_file(outside);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn the_saves_folder_holds_at_most_its_room() {
        // The third security audit: a hundred big saves could fill the disk.
        let root = scratch("saves-room");
        let room = 1000;
        save_game_within(&root, "a", &"x".repeat(600), room).unwrap();
        assert!(save_game_within(&root, "b", &"x".repeat(500), room).is_err());
        save_game_within(&root, "b", &"x".repeat(400), room).unwrap();
        // Replacing a slot counts its new size, not the old one too.
        save_game_within(&root, "a", &"x".repeat(550), room).unwrap();
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn saves_go_in_their_folder_by_slot_and_nowhere_else() {
        let root = scratch("saves");
        let (path, copy) = save_game(&root, "quick", "first").unwrap();
        assert_eq!(copy, None, "nothing older was there");
        assert_eq!(path, root.join(SAVES).join("quick.solsave"));
        save_game(&root, "quick", "second").unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "second",
            "a slot is replaced"
        );
        for bad in [
            "../outside",
            "a/b",
            "",
            "has space",
            "CON",
            "nul",
            "Com1",
            "LPT9",
        ] {
            assert!(save_game(&root, bad, "x").is_err(), "{bad}");
            assert!(save_path(&root, bad).is_none());
        }
        assert!(!root.join("outside.solsave").exists());
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
