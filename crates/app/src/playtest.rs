//! The playtest export ([decision 0093](../../../docs/decisions/0093-the-playtest-export.md)):
//! a folder holding this program renamed after the game, a playtest file
//! beside it, and the project's files in `game/`. The program, finding
//! the playtest file beside itself as it starts, opens straight into the
//! game. The player's saves, settings and a crash log go in their own
//! Saved Games folder, never beside the program.

use editor::export_dialog::Playtest;
use script::settings::{PlayerSettings, Setting};
use std::path::{Path, PathBuf};

/// The file beside an exported game's program saying what to play.
pub const PLAYTEST_FILE: &str = "playtest.txt";
/// The folder beside it holding the project's files.
pub const GAME_FOLDER: &str = "game";
/// The player's settings, in their saves folder.
pub const SETTINGS_FILE: &str = "settings.txt";
/// The crash log, in their saves folder.
pub const CRASH_FILE: &str = "crash log.txt";
/// The most an export copies (a project bigger than this is likely a
/// mistake, such as a folder of videos in the assets).
pub const MAX_EXPORT_BYTES: u64 = 8 << 30;
/// The biggest settings file read.
const MAX_SETTINGS_BYTES: u64 = 4 << 10;

/// The project's folders an export leaves out: the cache, builds, the
/// editor's saves and recordings, backups, and Git's history.
const LEFT_OUT: &[&str] = &[
    asset_db::CACHE_FOLDER,
    "builds",
    crate::play_files::SAVES,
    crate::play_files::RECORDINGS,
    asset_db::backup::BACKUPS_FOLDER,
    ".git",
];

/// An exported game found beside the program: where its files are, and
/// what to play.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    /// The project's folder (`game/` beside the program).
    pub root: PathBuf,
    /// What to play.
    pub playtest: Playtest,
}

/// The exported game beside the program at `program`, if there is one:
/// a playtest file and a `game` folder. A playtest file that can't be read
/// is an error to show, not a reason to open the editor.
pub fn beside(program: &Path) -> Option<Result<Found, String>> {
    let folder = program.parent()?;
    let file = folder.join(PLAYTEST_FILE);
    if !file.is_file() {
        return None;
    }
    Some(read_found(folder, &file))
}

fn read_found(folder: &Path, file: &Path) -> Result<Found, String> {
    let bytes = asset_db::read_limited(file, editor::export_dialog::MAX_PLAYTEST_BYTES)
        .map_err(|e| format!("{PLAYTEST_FILE} can't be read: {e}"))?;
    let text = String::from_utf8_lossy(&bytes);
    let playtest =
        Playtest::parse(&text).map_err(|why| format!("{PLAYTEST_FILE} is damaged: {why}"))?;
    let root = folder.join(GAME_FOLDER);
    if !root.join(asset_db::PROJECT_FILE).is_file() {
        return Err(format!(
            "the {GAME_FOLDER} folder beside the program is missing or incomplete"
        ));
    }
    Ok(Found { root, playtest })
}

/// The program's file name for `game`: its name, with `.exe` on Windows.
pub fn program_name(game: &str) -> String {
    let name = editor::export_dialog::game_name(game);
    if cfg!(windows) {
        format!("{name}.exe")
    } else if name.eq_ignore_ascii_case(GAME_FOLDER) || name.eq_ignore_ascii_case(PLAYTEST_FILE) {
        // Beside the game folder and the playtest file, it needs a name
        // of its own.
        format!("{name} (game)")
    } else {
        name
    }
}

/// What an export made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exported {
    /// The folder.
    pub folder: PathBuf,
    /// How many of the project's files were copied.
    pub files: usize,
    /// How many bytes they hold, the program included.
    pub bytes: u64,
}

/// Exports the project in `root` to `folder` as `playtest`, with
/// `program` (this program) renamed after the game. The folder is made;
/// one that's there must be empty, so an export never mixes with or
/// replaces anything.
pub fn export(
    root: &Path,
    program: &Path,
    playtest: &Playtest,
    folder: &Path,
) -> Result<Exported, String> {
    if !editor::export_dialog::usable_area(&playtest.area) {
        return Err("choose an area for the game to start in".into());
    }
    let assets = root.join(asset_db::ASSETS_FOLDER);
    if !assets.join(&playtest.area).is_file() {
        return Err(format!("the area {} isn't in the project", playtest.area));
    }
    if folder.as_os_str().is_empty() || folder.is_relative() {
        return Err("choose a whole folder path, such as one in Documents".into());
    }
    // Inside the project, the export would copy itself.
    if let (Ok(r), Some(f)) = (root.canonicalize(), canonical_ancestor(folder))
        && f.starts_with(&r)
    {
        return Err("the folder is inside the project: choose one outside it".into());
    }
    // Measured first, so a project too big is refused before anything is
    // copied.
    let size = measure(root, true, 0)?;
    if size > MAX_EXPORT_BYTES {
        return Err(too_big());
    }
    let made_folder = match std::fs::read_dir(folder) {
        Ok(mut entries) => {
            if entries.next().is_some() {
                return Err(format!(
                    "{} already has files in it: choose a new or empty folder",
                    folder.display()
                ));
            }
            false
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            std::fs::create_dir_all(folder)
                .map_err(|e| format!("couldn't make the folder: {e}"))?;
            true
        }
        Err(e) => return Err(format!("couldn't use the folder: {e}")),
    };
    let outcome = fill(root, program, playtest, folder);
    if outcome.is_err() {
        // Nothing half-made is left in the way of trying again.
        let _ = std::fs::remove_dir_all(folder.join(GAME_FOLDER));
        let _ = std::fs::remove_file(folder.join(program_name(&playtest.name)));
        let _ = std::fs::remove_file(folder.join(PLAYTEST_FILE));
        if made_folder {
            let _ = std::fs::remove_dir(folder);
        }
    }
    outcome
}

/// Fills the empty `folder`: the project, the program and what to play.
fn fill(
    root: &Path,
    program: &Path,
    playtest: &Playtest,
    folder: &Path,
) -> Result<Exported, String> {
    // Checked again now the folder is there: a path that went through a
    // folder that wasn't there yet can only be told apart now.
    if let (Ok(r), Ok(f)) = (root.canonicalize(), folder.canonicalize())
        && f.starts_with(&r)
    {
        return Err("the folder is inside the project: choose one outside it".into());
    }
    let mut made = Exported {
        folder: folder.to_path_buf(),
        files: 0,
        bytes: 0,
    };
    copy_tree(root, &folder.join(GAME_FOLDER), true, &mut made)?;
    let to = folder.join(program_name(&playtest.name));
    made.bytes +=
        std::fs::copy(program, &to).map_err(|e| format!("couldn't copy the program: {e}"))?;
    let shipped = Playtest {
        folder: None,
        ..playtest.clone()
    };
    asset_db::write_atomically(&folder.join(PLAYTEST_FILE), shipped.text().as_bytes())
        .map_err(|e| format!("couldn't write {PLAYTEST_FILE}: {e}"))?;
    Ok(made)
}

fn too_big() -> String {
    format!(
        "the project holds more than {} GB; take what the game doesn't use out of the assets",
        MAX_EXPORT_BYTES >> 30
    )
}

/// How many bytes `from`'s files hold, as [`copy_tree`] would copy them,
/// stopping once past [`MAX_EXPORT_BYTES`].
fn measure(from: &Path, top: bool, mut sum: u64) -> Result<u64, String> {
    let entries =
        std::fs::read_dir(from).map_err(|e| format!("couldn't read {}: {e}", from.display()))?;
    for entry in entries.filter_map(Result::ok) {
        if top && LEFT_OUT.iter().any(|left| entry.file_name() == *left) {
            continue;
        }
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_dir() {
            sum = measure(&entry.path(), false, sum)?;
        } else if kind.is_file() {
            sum = sum.saturating_add(entry.metadata().map_or(0, |m| m.len()));
        }
        if sum > MAX_EXPORT_BYTES {
            break;
        }
    }
    Ok(sum)
}

/// The nearest folder of `path` that's there, made whole.
fn canonical_ancestor(path: &Path) -> Option<PathBuf> {
    path.ancestors().find_map(|a| a.canonicalize().ok())
}

/// Copies the folder `from` into `to` (made), leaving out links and, at
/// the top, [`LEFT_OUT`].
fn copy_tree(from: &Path, to: &Path, top: bool, made: &mut Exported) -> Result<(), String> {
    std::fs::create_dir_all(to).map_err(|e| format!("couldn't make {}: {e}", to.display()))?;
    let entries =
        std::fs::read_dir(from).map_err(|e| format!("couldn't read {}: {e}", from.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("couldn't read {}: {e}", from.display()))?;
        let name = entry.file_name();
        if top && LEFT_OUT.iter().any(|left| name == *left) {
            continue;
        }
        let kind = entry
            .file_type()
            .map_err(|e| format!("couldn't read {}: {e}", entry.path().display()))?;
        let target = to.join(&name);
        if kind.is_dir() {
            copy_tree(&entry.path(), &target, false, made)?;
        } else if kind.is_file() {
            let bytes = std::fs::copy(entry.path(), &target)
                .map_err(|e| format!("couldn't copy {}: {e}", entry.path().display()))?;
            made.files += 1;
            made.bytes += bytes;
            if made.bytes > MAX_EXPORT_BYTES {
                return Err(too_big());
            }
        }
        // Links are left out: the copy is what the game holds itself.
    }
    Ok(())
}

/// Where a project remembers how it was last exported: in its own
/// folder, which isn't shared in Git.
const LAST_EXPORT: &str = "last export.txt";

/// How the project in `root` was last exported, if it was.
pub fn last_export(root: &Path) -> Option<Playtest> {
    let path = root.join(asset_db::CACHE_FOLDER).join(LAST_EXPORT);
    let bytes = asset_db::read_limited(&path, editor::export_dialog::MAX_PLAYTEST_BYTES).ok()?;
    Playtest::parse(&String::from_utf8_lossy(&bytes)).ok()
}

/// Remembers how the project in `root` was exported, for next time.
pub fn remember_export(root: &Path, playtest: &Playtest) -> std::io::Result<()> {
    let own = asset_db::own_folder(root, asset_db::CACHE_FOLDER)?;
    asset_db::write_atomically(&own.join(LAST_EXPORT), playtest.text().as_bytes())
}

/// The player's own folder for `game`'s saves, settings and crash log:
/// `Saved Games/<game>` on Windows, beside the other games', and the usual
/// data folder elsewhere. Not made here.
pub fn saved_games(game: &str) -> Option<PathBuf> {
    let name = editor::export_dialog::game_name(game);
    if cfg!(windows) {
        let home = std::env::var_os("USERPROFILE")?;
        Some(PathBuf::from(home).join("Saved Games").join(name))
    } else {
        let data = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))?;
        Some(data.join(name))
    }
}

/// The person's Documents folder, for exports to go in by default.
pub fn documents() -> Option<PathBuf> {
    let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })?;
    let home = PathBuf::from(home);
    let documents = home.join("Documents");
    Some(if documents.is_dir() { documents } else { home })
}

/// The player's settings as written in their settings file.
pub fn settings_text(settings: &PlayerSettings) -> String {
    let [w, h] = settings.resolution;
    format!(
        "text size {}\nui size {}\nfullscreen {}\nresolution {w} {h}\n",
        settings.text_size,
        settings.ui_size,
        if settings.fullscreen { "on" } else { "off" },
    )
}

/// Reads the player's settings file, starting from `start` (the export's
/// window). Anything odd is put right or left as it starts.
pub fn parse_settings(text: &str, start: PlayerSettings) -> PlayerSettings {
    let mut settings = start;
    for line in text.lines() {
        let words: Vec<&str> = line.split_whitespace().collect();
        let change = match words.as_slice() {
            ["text", "size", v] => v.parse().ok().map(Setting::TextSize),
            ["ui", "size", v] => v.parse().ok().map(Setting::UiSize),
            ["fullscreen", v] => Some(Setting::Fullscreen(*v == "on")),
            ["resolution", w, h] => match (w.parse(), h.parse()) {
                (Ok(w), Ok(h)) => Some(Setting::Resolution(w, h)),
                _ => None,
            },
            _ => None,
        };
        if let Some(change) = change {
            settings = settings.with(change);
        }
    }
    settings.sanitized()
}

/// The player's settings for `playtest`: saved ones in `saved`, or the
/// export's window.
pub fn load_settings(saved: Option<&Path>, playtest: &Playtest) -> PlayerSettings {
    let start = PlayerSettings::default()
        .with(Setting::Fullscreen(playtest.fullscreen))
        .with(Setting::Resolution(playtest.size[0], playtest.size[1]));
    let text = saved
        .map(|s| s.join(SETTINGS_FILE))
        .and_then(|f| asset_db::read_limited(&f, MAX_SETTINGS_BYTES).ok())
        .map(|b| String::from_utf8_lossy(&b).into_owned());
    match text {
        Some(text) => parse_settings(&text, start),
        None => start,
    }
}

/// Keeps the player's settings in `saved` (made if it isn't there).
pub fn save_settings(saved: &Path, settings: &PlayerSettings) -> std::io::Result<()> {
    std::fs::create_dir_all(saved)?;
    asset_db::write_atomically(
        &saved.join(SETTINGS_FILE),
        settings_text(settings).as_bytes(),
    )
}

/// Writes what went wrong to the crash log in `saved`, for a playtester
/// to send back: when, the game and program, what the program was doing,
/// and the last things it said.
pub fn write_crash_log(saved: &Path, about: &str, said: &[String]) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(saved)?;
    let path = saved.join(CRASH_FILE);
    let mut text = format!(
        "{about}\nWhen: {}\n\nWhat the game said last:\n",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
    );
    for line in said.iter().rev().take(40).rev() {
        text.push_str(line);
        text.push('\n');
    }
    asset_db::write_atomically(&path, text.as_bytes())?;
    Ok(path)
}

#[cfg(test)]
#[path = "playtest_tests.rs"]
mod tests;
