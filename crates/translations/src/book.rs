//! Every language's text, read from the project's Fluent files and its
//! data tables, and found by key in any language: [`Translations`], made
//! once each time a file changes; and [`Texts`], what a running game asks
//! (the translations and the player's language).
//!
//! A line missing in a language is shown in the main language, so a game
//! works with no translations at all; a key with no line anywhere shows
//! the key itself, so it's seen and fixed.

use crate::ftl::{self, FileProblem, Line, ValueKind};
use crate::language::Language;
use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentArgs, FluentResource, FluentValue};
use std::collections::HashMap;
use std::sync::Arc;

/// The folder the translation files are in, inside the project's files:
/// a folder per language beside it (`locale/nb/shop.ftl`).
pub const FOLDER: &str = "locale";

/// The most translation files read for one language.
pub(crate) const MAX_FILES: usize = 1_000;

/// A value handed to a line of text: `{ $name }`, `{ $count }`.
#[derive(Debug, Clone, PartialEq)]
pub enum TextValue {
    /// Words.
    Text(String),
    /// A number, which picks a plural's form.
    Number(f64),
}

/// One language's lines, by key in small letters, and its formatter.
struct LanguageText {
    language: Language,
    lines: HashMap<String, Line>,
    /// The keys in the order read (files in path order, then as written).
    order: Vec<String>,
    bundle: FluentBundle<FluentResource>,
}

impl std::fmt::Debug for LanguageText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LanguageText")
            .field("language", &self.language)
            .field("lines", &self.lines.len())
            .finish_non_exhaustive()
    }
}

/// A data table's translated cell: its key and its text in the main
/// language, as the table holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableText {
    /// Its key (`Items.IronSword.description`), made once and never
    /// changed.
    pub key: String,
    /// The table's file, for Problems (`data/items.table.lst`).
    pub file: String,
    /// Its text in the main language.
    pub text: String,
}

/// The project's text in every language.
#[derive(Debug)]
pub struct Translations {
    /// Each language, the main one first.
    languages: Vec<LanguageText>,
    /// The tables' translated cells, by key in small letters.
    tables: HashMap<String, TableText>,
    table_order: Vec<String>,
    /// What's wrong with the files read.
    problems: Vec<FileProblem>,
}

impl Default for Translations {
    fn default() -> Self {
        Self::new(&level::languages::Languages::default(), &[], Vec::new())
    }
}

impl Translations {
    /// The project's text: `languages` (as `languages.lst` has them),
    /// `files` (each path inside the project's files and its text; those
    /// not in `locale/<language>/` and not ending `.ftl` are passed over)
    /// and the tables' translated cells.
    pub fn new(
        languages: &level::languages::Languages,
        files: &[(String, String)],
        tables: Vec<TableText>,
    ) -> Self {
        let mut problems = Vec::new();
        let mut files: Vec<&(String, String)> = files
            .iter()
            .filter(|(p, _)| p.starts_with(&format!("{FOLDER}/")) && p.ends_with(".ftl"))
            .collect();
        files.sort_by(|a, b| a.0.cmp(&b.0));
        let mut out = Vec::new();
        for code in languages.all() {
            let Ok(language) = Language::new(code) else {
                continue;
            };
            let own: Vec<&(String, String)> = files
                .iter()
                .copied()
                .filter(|(p, _)| in_language(p) == Some(code.as_str()))
                .collect();
            out.push(language_text(language, &own, &mut problems));
        }
        for (path, _) in &files {
            let lang = in_language(path).unwrap_or_default();
            if !languages.has(lang) {
                problems.push(FileProblem {
                    file: path.clone(),
                    line: 0,
                    message: format!(
                        "{lang} isn't one of the project's languages, so this file is left out: \
                         add it in Project settings › Languages."
                    ),
                });
            }
        }
        let mut by_key = HashMap::with_capacity(tables.len());
        let mut table_order = Vec::with_capacity(tables.len());
        for cell in tables {
            let lower = cell.key.to_ascii_lowercase();
            if let std::collections::hash_map::Entry::Vacant(free) = by_key.entry(lower.clone()) {
                table_order.push(lower);
                free.insert(cell);
            }
        }
        Self {
            languages: out,
            tables: by_key,
            table_order,
            problems,
        }
    }

    /// The project's languages, the main one first.
    pub fn languages(&self) -> Vec<Language> {
        self.languages.iter().map(|l| l.language).collect()
    }

    /// The main language.
    pub fn main(&self) -> Language {
        self.languages
            .first()
            .map_or_else(Language::english, |l| l.language)
    }

    /// Every key: the main language's lines in the order read, then the
    /// tables' cells.
    pub fn keys(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        if let Some(main) = self.languages.first() {
            out.extend(
                main.order
                    .iter()
                    .filter(|k| !self.tables.contains_key(*k))
                    .filter_map(|k| main.lines.get(k).map(|l| l.key.clone())),
            );
        }
        out.extend(
            self.table_order
                .iter()
                .filter_map(|k| self.tables.get(k).map(|t| t.key.clone())),
        );
        out
    }

    /// The keys game code names as `Text` options: the main language's
    /// lines (the tables' cells are read through their tables).
    pub fn code_keys(&self) -> Vec<String> {
        self.languages.first().map_or_else(Vec::new, |main| {
            main.order
                .iter()
                .filter(|k| !self.tables.contains_key(*k))
                .filter_map(|k| main.lines.get(k).map(|l| l.key.clone()))
                .collect()
        })
    }

    /// Whether `key` is one (any capitals).
    pub fn has(&self, key: &str) -> bool {
        let lower = key.to_ascii_lowercase();
        self.tables.contains_key(&lower)
            || self
                .languages
                .first()
                .is_some_and(|m| m.lines.contains_key(&lower))
    }

    /// `key`'s line as written in `language`, if that language has it; a
    /// table's cell in the main language is its table's text.
    pub fn line(&self, language: Language, key: &str) -> Option<Line> {
        let lower = key.to_ascii_lowercase();
        if language == self.main()
            && let Some(cell) = self.tables.get(&lower)
        {
            return Some(Line {
                key: cell.key.clone(),
                file: cell.file.clone(),
                line: 0,
                text: cell.text.clone(),
                values: Vec::new(),
            });
        }
        self.of(language)?.lines.get(&lower).cloned()
    }

    /// Whether `key` is a table's cell.
    pub fn is_table_key(&self, key: &str) -> bool {
        self.tables.contains_key(&key.to_ascii_lowercase())
    }

    /// The file `key`'s line is in for `language`, or the one a new line
    /// goes in (`locale/nb/shop.ftl`).
    pub fn file_for(&self, language: Language, key: &str) -> String {
        self.of(language)
            .and_then(|l| l.lines.get(&key.to_ascii_lowercase()))
            .map_or_else(
                || {
                    format!(
                        "{FOLDER}/{}/{}",
                        language.code(),
                        crate::keys::file_for(key)
                    )
                },
                |l| l.file.clone(),
            )
    }

    /// The values the main language's line for `key` takes.
    pub fn values(&self, key: &str) -> Vec<(String, ValueKind)> {
        self.line(self.main(), key)
            .map(|l| l.values)
            .unwrap_or_default()
    }

    /// What's wrong with the files read, each where it is.
    pub fn problems(&self) -> &[FileProblem] {
        &self.problems
    }

    /// How many of the keys `language` has a line for.
    pub fn count(&self, language: Language) -> usize {
        if language == self.main() {
            return self.keys().len();
        }
        let Some(own) = self.of(language) else {
            return 0;
        };
        self.keys()
            .iter()
            .filter(|k| own.lines.contains_key(&k.to_ascii_lowercase()))
            .count()
    }

    /// The keys `language` has no line for, in order.
    pub fn missing(&self, language: Language) -> Vec<String> {
        if language == self.main() {
            return Vec::new();
        }
        let own = self.of(language);
        self.keys()
            .into_iter()
            .filter(|k| own.is_none_or(|l| !l.lines.contains_key(&k.to_ascii_lowercase())))
            .collect()
    }

    /// `key`'s text in `language`, with `values`: that language's line, else
    /// the main language's, else the key itself.
    pub fn text(&self, language: Language, key: &str, values: &[(&str, TextValue)]) -> String {
        let lower = key.to_ascii_lowercase();
        let main = self.main();
        if language != main
            && let Some(text) = self.format(language, &lower, values)
        {
            return text;
        }
        if let Some(cell) = self.tables.get(&lower) {
            return cell.text.clone();
        }
        self.format(main, &lower, values)
            .unwrap_or_else(|| key.to_owned())
    }

    /// `key`'s text in `language` alone, formatted, if it has a line.
    fn format(
        &self,
        language: Language,
        lower: &str,
        values: &[(&str, TextValue)],
    ) -> Option<String> {
        let own = self.of(language)?;
        let line = own.lines.get(lower)?;
        let id = crate::keys::fluent_id(&line.key);
        let message = own.bundle.get_message(&id)?;
        let pattern = message.value()?;
        let mut args = FluentArgs::with_capacity(values.len());
        for (name, value) in values {
            match value {
                TextValue::Text(t) => args.set(*name, FluentValue::from(t.as_str())),
                TextValue::Number(n) => args.set(*name, FluentValue::from(*n)),
            }
        }
        let mut errors = Vec::new();
        let text =
            own.bundle
                .format_pattern(pattern, (!values.is_empty()).then_some(&args), &mut errors);
        Some(text.into_owned())
    }

    fn of(&self, language: Language) -> Option<&LanguageText> {
        self.languages.iter().find(|l| l.language == language)
    }
}

/// The language folder `path` is in (`nb` for `locale/nb/shop.ftl`).
pub fn in_language(path: &str) -> Option<&str> {
    let rest = path.strip_prefix(FOLDER)?.strip_prefix('/')?;
    let (language, file) = rest.split_once('/')?;
    (!file.is_empty()).then_some(language)
}

/// One language's text from its files.
fn language_text(
    language: Language,
    files: &[&(String, String)],
    problems: &mut Vec<FileProblem>,
) -> LanguageText {
    let mut bundle = FluentBundle::new_concurrent(vec![language.id()]);
    bundle.set_use_isolating(false);
    let mut lines: HashMap<String, Line> = HashMap::new();
    let mut order = Vec::new();
    for (n, (path, text)) in files.iter().enumerate() {
        if n >= MAX_FILES {
            problems.push(FileProblem {
                file: path.clone(),
                line: 0,
                message: format!(
                    "Only {MAX_FILES} files are read for each language; this one is left out."
                ),
            });
            break;
        }
        let read = ftl::read(path, text);
        problems.extend(read.problems);
        for line in read.lines {
            let lower = line.key.to_ascii_lowercase();
            if let Some(first) = lines.get(&lower) {
                problems.push(FileProblem {
                    file: line.file.clone(),
                    line: line.line,
                    message: format!(
                        "{} is written in {} too; the first is used.",
                        line.key, first.file
                    ),
                });
                continue;
            }
            if lines.len() >= ftl::MAX_LINES {
                problems.push(FileProblem {
                    file: line.file.clone(),
                    line: line.line,
                    message: format!(
                        "A language has at most {} lines; the rest are left out.",
                        ftl::MAX_LINES
                    ),
                });
                break;
            }
            order.push(lower.clone());
            lines.insert(lower, line);
        }
        if text.len() <= ftl::MAX_FILE_BYTES {
            let resource = match FluentResource::try_new((*text).clone()) {
                Ok(r) | Err((r, _)) => r,
            };
            // A name the bundle has already is the second of two: left out
            // above, so its error says nothing new.
            let _ = bundle.add_resource(resource);
        }
    }
    LanguageText {
        language,
        lines,
        order,
        bundle,
    }
}

/// What a running game reads text from: the translations, and the
/// player's language. Cheap to copy: copies share the translations.
#[derive(Debug, Clone)]
pub struct Texts {
    all: Arc<Translations>,
    language: Language,
}

impl Texts {
    /// `all`, shown in `language` (the main language when the project
    /// hasn't that one).
    pub fn new(all: Arc<Translations>, language: Language) -> Self {
        let language = crate::language::best(Some(language), &all.languages());
        Self { all, language }
    }

    /// The language shown.
    pub fn language(&self) -> Language {
        self.language
    }

    /// The translations.
    pub fn all(&self) -> &Arc<Translations> {
        &self.all
    }

    /// `key`'s text in the player's language.
    pub fn text(&self, key: &str) -> String {
        self.all.text(self.language, key, &[])
    }

    /// `key`'s text in the player's language, with values.
    pub fn text_with(&self, key: &str, values: &[(&str, TextValue)]) -> String {
        self.all.text(self.language, key, values)
    }

    /// Whether these are the very translations and language of `other`.
    pub fn same(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.all, &other.all) && self.language == other.language
    }
}

#[cfg(test)]
#[path = "book_tests.rs"]
mod tests;
