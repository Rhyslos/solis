//! The project's languages and text in the editor (stage 1, translations;
//! [decision 0131](../../../docs/decisions/0131-translations-and-captions.md)):
//! its `languages.lst` and its `locale/<language>/*.ftl` files as the app
//! read them, made into one `Translations` with the data tables'
//! translated cells (made again only when a file or a table changes), and
//! the changes Project settings › Languages and the Translations tab make,
//! kept here at once and saved by the app.

use crate::workbench::{Workbench, WorkbenchAction};
use level::languages::Languages;
use std::collections::HashSet;
use std::sync::Arc;
use translations::{Language, TableText, Texts, Translations};

/// The project's languages and text as the editor has them.
#[derive(Debug, Default)]
pub(crate) struct Locale {
    /// The project's languages (`languages.lst`).
    pub(crate) languages: Languages,
    /// Its translation files, each path inside the project's files and
    /// its text, as read or last changed here.
    pub(crate) files: Vec<(String, String)>,
    /// Goes up with every change to the languages or the files.
    pub(crate) revision: u64,
    /// The translations, and what they were made from: the revision and
    /// each table's place in memory.
    made: Option<(Arc<Translations>, u64, Vec<usize>)>,
    /// The check's problems, with what they were made from.
    checked: Option<(u64, Vec<translations::FileProblem>)>,
}

/// The project's text: `languages`, its translation `files` and the
/// translated cells of the tables in `sheets`.
pub fn translations_of(
    languages: &Languages,
    files: &[(String, String)],
    sheets: &crate::sprite_sheets::SpriteSheets,
) -> Translations {
    let cells: Vec<TableText> = sheets
        .table_names()
        .iter()
        .filter_map(|t| {
            let object = script::table_code::named(&t.path, &t.table.row_name)?.object;
            let file = format!("{}.{}.lst", t.path, level::table::file::KIND);
            Some(
                t.table
                    .texts(&object)
                    .into_iter()
                    .map(move |(key, text)| TableText {
                        key,
                        file: file.clone(),
                        text,
                    }),
            )
        })
        .flatten()
        .collect();
    Translations::new(languages, files, cells)
}

/// What `translations` give game code, the engine's objects and the
/// tables' in `sheets` taken already, with notes on lines whose methods
/// can't be made.
pub fn text_names_of(
    translations: &Translations,
    sheets: &crate::sprite_sheets::SpriteSheets,
) -> (script::text_code::TextNames, Vec<String>) {
    let taken: Vec<String> = script::describe::owners_of_natives()
        .into_iter()
        .map(str::to_owned)
        .chain(
            sheets
                .table_names()
                .iter()
                .filter_map(|t| script::table_code::named(&t.path, &t.table.row_name))
                .map(|n| n.object),
        )
        .collect();
    script::text_code::TextNames::of(translations, &taken)
}

impl Workbench {
    /// Takes in the project's languages and translation files as read
    /// from its folder (when it opens, and when they change on disk).
    pub fn set_locale(&mut self, languages: Languages, files: Vec<(String, String)>) {
        let locale = &mut self.open.locale;
        if locale.languages == languages && locale.files == files {
            return;
        }
        locale.languages = languages;
        locale.files = files;
        locale.revision += 1;
    }

    /// The project's languages.
    pub fn languages(&self) -> &Languages {
        &self.open.locale.languages
    }

    /// The project's text in every language, with the data tables'
    /// translated cells, made again when either changed.
    pub fn translations(&mut self) -> Arc<Translations> {
        let tables = self.viewport.sheets.table_names();
        let from: Vec<usize> = tables
            .iter()
            .map(|t| Arc::as_ptr(&t.table) as usize)
            .collect();
        let locale = &mut self.open.locale;
        if let Some((made, revision, was)) = &locale.made
            && *revision == locale.revision
            && *was == from
        {
            return Arc::clone(made);
        }
        let made = Arc::new(translations_of(
            &locale.languages,
            &locale.files,
            &self.viewport.sheets,
        ));
        locale.made = Some((Arc::clone(&made), locale.revision, from));
        made
    }

    /// The text a game playing now reads: the translations in the
    /// player's language (theirs, the computer's, or the main one).
    pub(crate) fn game_texts(&mut self) -> Texts {
        let all = self.translations();
        let language = self.player_settings.language_of(&all.languages());
        Texts::new(all, language)
    }

    /// What the project's text gives game code (its keys, languages and
    /// lines with values), for `solis/project.lst` and `solis/texts.lst`,
    /// with notes on lines whose methods can't be made.
    pub fn text_names(&mut self) -> (script::text_code::TextNames, Vec<String>) {
        let all = self.translations();
        text_names_of(&all, &self.viewport.sheets)
    }

    /// The translations' problems for Problems: each file's mistakes,
    /// lines missing in each language, lines nothing uses and values that
    /// don't match. Worked out again only when the text, the tables or the
    /// game's code changed.
    pub(crate) fn translation_problems(&mut self) -> Vec<translations::FileProblem> {
        let all = self.translations();
        let program = self
            .open
            .program
            .as_ref()
            .map_or(0, |p| Arc::as_ptr(p) as usize as u64);
        let events = self.open.sound_events.len() as u64;
        let made =
            (Arc::as_ptr(&all) as usize as u64) ^ program.rotate_left(17) ^ events.rotate_left(41);
        if let Some((from, problems)) = &self.open.locale.checked
            && *from == made
        {
            return problems.clone();
        }
        let mut used: HashSet<String> = self
            .open
            .program
            .as_ref()
            .map(|p| p.texts_used.iter().cloned().collect())
            .unwrap_or_default();
        for state in self.open.sound_events.values() {
            if let crate::sound_view::EventState::Read { event, .. } = state {
                for key in [&event.caption, &event.speaker] {
                    if !key.is_empty() {
                        used.insert(key.to_ascii_lowercase());
                    }
                }
            }
        }
        let problems = translations::check::check(&all, &used);
        self.open.locale.checked = Some((made, problems.clone()));
        problems
    }

    /// Sets `key`'s line in `language` to `text` (empty takes it out): a
    /// table's cell in the main language in its table, any other line in
    /// its translation file, kept here and saved. Why it couldn't be.
    pub(crate) fn set_line(
        &mut self,
        language: Language,
        key: &str,
        text: &str,
    ) -> Result<(), String> {
        let all = self.translations();
        if language == all.main() && all.is_table_key(key) {
            return Err(
                "A table's text in the main language is written in its table: open it from the \
                 Library."
                    .into(),
            );
        }
        let path = all.file_for(language, key);
        let now = self
            .open
            .locale
            .files
            .iter()
            .find(|(p, _)| *p == path)
            .map_or_else(String::new, |(_, t)| t.clone());
        let text = translations::edit::set_line(&now, key, text)?;
        self.write_locale_file(path, text);
        Ok(())
    }

    /// Keeps `text` as the translation file `path` and has the app save it.
    pub(crate) fn write_locale_file(&mut self, path: String, text: String) {
        let locale = &mut self.open.locale;
        match locale.files.iter_mut().find(|(p, _)| *p == path) {
            Some((_, was)) if *was == text => return,
            Some((_, was)) => was.clone_from(&text),
            None => locale.files.push((path.clone(), text.clone())),
        }
        locale.revision += 1;
        self.open
            .outbox
            .push(WorkbenchAction::SaveLocaleFile(path, text));
    }

    /// The project's languages changed here: kept, and saved (which writes
    /// game code's names again).
    pub(crate) fn change_languages(&mut self, languages: Languages) {
        if languages == self.open.locale.languages {
            return;
        }
        self.open.locale.languages = languages.clone();
        self.open.locale.revision += 1;
        self.open
            .outbox
            .push(WorkbenchAction::SaveLanguages(languages));
    }

    /// Takes in a translators' spreadsheet's text: each line it changes
    /// written into its file, and what was passed over said in the Console.
    pub fn import_translations(&mut self, text: &str) {
        let all = self.translations();
        let imported = match translations::sheet::import(text, &all) {
            Ok(imported) => imported,
            Err(why) => {
                self.log(
                    crate::console::LogLevel::Warning,
                    format!(
                        "{} couldn't be read: {why}.",
                        translations::sheet::FILE_NAME
                    ),
                );
                return;
            }
        };
        let files = self.open.locale.files.clone();
        let (written, notes) = translations::edit::apply(&imported.changes, &all, |path| {
            files
                .iter()
                .find(|(p, _)| p == path)
                .map_or_else(String::new, |(_, t)| t.clone())
        });
        for (path, text) in written {
            self.write_locale_file(path, text);
        }
        for note in imported.notes.into_iter().chain(notes) {
            self.log(crate::console::LogLevel::Warning, note);
        }
        self.log(
            crate::console::LogLevel::Info,
            format!(
                "{} lines of text taken in from {}.",
                imported.changes.len(),
                translations::sheet::FILE_NAME
            ),
        );
    }

    /// Writes the translators' spreadsheet: the app saves it in the
    /// project's folder.
    pub(crate) fn export_translations(&mut self) {
        let text = translations::sheet::export(&self.translations());
        self.open
            .outbox
            .push(WorkbenchAction::ExportTranslations(text));
    }
}

#[cfg(test)]
#[path = "locale_tests.rs"]
mod tests;
