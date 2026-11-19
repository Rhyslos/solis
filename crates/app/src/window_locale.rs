//! What the window does for the game's text (stage 1, translations):
//! reading the project's `languages.lst` and its translation files
//! (`locale/<language>/*.ftl`) when it opens and when they change on
//! disk, saving what Project settings › Languages and the Translations tab
//! change (and writing game code's names again), and the translators'
//! spreadsheet out and back in.

use super::App;
use editor::LogLevel;
use level::languages::Languages;
use std::path::{Path, PathBuf};

/// What tells the files changed: each one's path, when it was last
/// changed and its length.
pub(super) type Stamp = Vec<(PathBuf, Option<std::time::SystemTime>, u64)>;

/// The most bytes of all translation files read together.
const MOST_BYTES: u64 = 256 << 20;

/// The project's translation files in `files` (its files' folder), each
/// path inside it (`locale/nb/shop.ftl`): `.ftl` files one folder down in
/// `locale/`, never through a link.
fn translation_files(files: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(languages) = std::fs::read_dir(files.join(translations::FOLDER)) else {
        return out;
    };
    for language in languages.flatten() {
        if !language.file_type().is_ok_and(|t| t.is_dir()) {
            continue;
        }
        let Ok(inside) = std::fs::read_dir(language.path()) else {
            continue;
        };
        for file in inside.flatten() {
            let path = file.path();
            if file.file_type().is_ok_and(|t| t.is_file())
                && path.extension().is_some_and(|e| e == "ftl")
                && out.len() < 64 * translations::ftl::MAX_LINES
            {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

/// The stamp of `root`'s `languages.lst` and `files`' translation files,
/// from their metadata alone (nothing read).
pub(super) fn stamp(root: &Path, files: &Path) -> Stamp {
    std::iter::once(root.join(level::languages::FILE_NAME))
        .chain(translation_files(files))
        .filter_map(|path| {
            let meta = std::fs::metadata(&path).ok()?;
            Some((path, meta.modified().ok(), meta.len()))
        })
        .collect()
}

/// Reads the project's languages (`root/languages.lst`) and translation
/// files (in `files`), with notes on anything that couldn't be read.
pub(crate) fn load_locale(
    root: &Path,
    files: &Path,
) -> (Languages, Vec<(String, String)>, Vec<String>) {
    let mut notes = Vec::new();
    let file = level::languages::FILE_NAME;
    let languages =
        match crate::play_files::read_limited(&root.join(file), level::languages::MAX_BYTES as u64)
        {
            Ok(Some(text)) => match level::languages::read(&text) {
                Ok(read) => {
                    notes.extend(read.notes.into_iter().map(|n| format!("{file}: {n}")));
                    read.languages
                }
                Err(why) => {
                    notes.push(format!(
                        "{file} {why}, so the project is written in English alone until it's fixed."
                    ));
                    Languages::default()
                }
            },
            Ok(None) => Languages::default(),
            Err(why) => {
                notes.push(format!("Couldn't read {file}: {why}."));
                Languages::default()
            }
        };
    let mut read = Vec::new();
    let mut total = 0u64;
    for path in translation_files(files) {
        let inside = path
            .strip_prefix(files)
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();
        let limit = translations::ftl::MAX_FILE_BYTES as u64;
        match crate::play_files::read_limited(&path, limit) {
            Ok(Some(text)) => {
                total += text.len() as u64;
                if total > MOST_BYTES {
                    notes.push(format!(
                        "The translation files are bigger than {} MB together, so {inside} and \
                         the rest are left out.",
                        MOST_BYTES >> 20
                    ));
                    break;
                }
                read.push((inside, text));
            }
            Ok(None) => {}
            Err(why) => notes.push(format!("Couldn't read {inside}: {why}.")),
        }
    }
    (languages, read, notes)
}

impl App {
    /// Reads the project's languages and text again when they changed on
    /// disk since last read or written (a pull, an edit by hand), and
    /// writes game code's names again: only metadata is looked at
    /// otherwise.
    pub(super) fn follow_locale(&mut self) {
        let Some(root) = self.projects.root().map(Path::to_path_buf) else {
            return;
        };
        let Some(files) = self.projects.db().map(|db| db.project().files()) else {
            return;
        };
        let now = stamp(&root, &files);
        if now == self.locale_seen {
            return;
        }
        self.locale_seen = now;
        let (languages, read, notes) = load_locale(&root, &files);
        self.workbench.set_locale(languages, read);
        for note in notes {
            self.workbench.log(LogLevel::Warning, note);
        }
        self.text_names_changed();
    }

    /// The game's text gives game code new names (a key added, a
    /// language, a line's values): the project's names are written again
    /// and the code checked.
    pub(super) fn text_names_changed(&mut self) {
        let texts = self.workbench.text_names().0;
        let names = crate::game_code::Animation::of(self.workbench.sprite_sheets(), texts);
        if self.game_code.set_animation(names)
            && let Some(assets) = self.projects.db().map(|db| db.project().files())
        {
            self.game_code.check(assets);
        }
    }

    /// Saves the project's languages.
    pub(super) fn save_languages(&mut self, languages: &Languages) {
        let Some(root) = self.projects.root().map(Path::to_path_buf) else {
            return;
        };
        let text = level::languages::write(languages);
        match asset_db::write_atomically(&root.join(level::languages::FILE_NAME), text.as_bytes()) {
            Ok(()) => {
                if let Some(files) = self.projects.db().map(|db| db.project().files()) {
                    self.locale_seen = stamp(&root, &files);
                }
            }
            Err(why) => self.workbench.log(
                LogLevel::Error,
                format!("Couldn't save the project's languages: {why}."),
            ),
        }
        self.text_names_changed();
    }

    /// Saves a translation file (`path` inside the project's files).
    pub(super) fn save_locale_file(&mut self, path: &str, text: &str) {
        let Some(files) = self.projects.db().map(|db| db.project().files()) else {
            return;
        };
        let inside = Path::new(path);
        let safe = inside.starts_with(translations::FOLDER)
            && inside.extension().is_some_and(|e| e == "ftl")
            && inside
                .components()
                .all(|c| matches!(c, std::path::Component::Normal(_)));
        if !safe {
            return;
        }
        let file = files.join(inside);
        let written = file
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| asset_db::write_atomically(&file, text.as_bytes()));
        match written {
            Ok(()) => {
                if let Some(root) = self.projects.root().map(Path::to_path_buf) {
                    self.locale_seen = stamp(&root, &files);
                }
            }
            Err(why) => self
                .workbench
                .log(LogLevel::Error, format!("Couldn't save {path}: {why}.")),
        }
        self.text_names_changed();
    }

    /// Writes the translators' spreadsheet in the project's folder.
    pub(super) fn export_translations(&mut self, text: &str) {
        let Some(root) = self.projects.root().map(Path::to_path_buf) else {
            return;
        };
        let file = translations::sheet::FILE_NAME;
        match asset_db::write_atomically(&root.join(file), text.as_bytes()) {
            Ok(()) => self.workbench.log(
                LogLevel::Info,
                format!(
                    "Wrote {file} in the project's folder: a key, the main language and a column \
                     for each language. Send it to translators, then Import it back."
                ),
            ),
            Err(why) => self
                .workbench
                .log(LogLevel::Error, format!("Couldn't write {file}: {why}.")),
        }
    }

    /// Reads the translators' spreadsheet back from the project's folder.
    pub(super) fn read_translations(&mut self) {
        let Some(root) = self.projects.root().map(Path::to_path_buf) else {
            return;
        };
        let file = translations::sheet::FILE_NAME;
        match crate::play_files::read_limited(
            &root.join(file),
            level::table::text::MAX_BYTES as u64,
        ) {
            Ok(Some(text)) => self.workbench.import_translations(&text),
            Ok(None) => self.workbench.log(
                LogLevel::Warning,
                format!(
                    "There's no {file} in the project's folder: Export spreadsheet writes one."
                ),
            ),
            Err(why) => self
                .workbench
                .log(LogLevel::Warning, format!("Couldn't read {file}: {why}.")),
        }
    }
}

#[cfg(test)]
#[path = "window_locale_tests.rs"]
mod tests;
