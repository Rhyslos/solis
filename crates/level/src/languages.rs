//! The project's languages (stage 1, translations): which languages its
//! game is translated into, and which one it's written in (its **main
//! language**, whose text is shown when another language lacks a line).
//! Made in Project settings › Languages and kept in `languages.lst` in the
//! project's folder, a Lust data file
//! ([decision 0081](../../../docs/decisions/0081-what-solis-makes-is-lust.md)),
//! version 1:
//!
//! ```text
//! -- The project's languages (Project settings › Languages). Written by Solis; safe to edit by hand.
//! fetch "solis/data/languages":[Languages];
//!
//! lock int version = 1;
//! lock Languages languages = new Languages { main = "en", all = ["en", "nb", "de"] };
//! end
//! ```
//!
//! Each language is a short code, as translators and Windows name them:
//! `en`, `nb`, `de`, `pt-BR`, `zh-Hans`. Its text is in
//! `locale/<code>/*.ftl`. A project without the file is written in English
//! and has no other language, so a game works with no translations at all.

use crate::lust_data::{self as data, DataError};

/// The file's name, in the project's folder.
pub const FILE_NAME: &str = "languages.lst";

/// The word naming its classes' file (`solis/data/languages.lst`).
pub const KIND: &str = "languages";

/// The version this build writes.
pub const VERSION: u32 = 1;

/// The most bytes a languages file may have.
pub const MAX_BYTES: usize = 64 << 10;

/// The most languages a project may have.
pub const MAX_LANGUAGES: usize = 64;

/// The longest a language's code may be, in characters (`zh-Hant-TW` is 10).
pub const MAX_CODE_CHARS: usize = 15;

/// The language a project is written in when it says nothing.
pub const STANDARD: &str = "en";

/// What a languages file holds, in Lust: written into every project's
/// `solis/data/languages.lst`, and handed to Lust when the file is read.
pub const CLASSES: &str = "-- What a project's languages file holds (Solis writes this; languages.lst fetches it).\n\
class Languages {\n    has {\n        string main = \"en\";\n        string array all = [];\n    }\n}\n\npass Languages;\nend\n";

/// The languages file's version and upgraders. Version 1 is the first.
pub static FORMAT: reflect::upgrade::Format<Languages> = reflect::upgrade::Format {
    name: "languages",
    current: VERSION,
    version_of: data::version_of,
    steps: &[],
};

const COMMENT: &str = "The project's languages (Project settings › Languages). Written by Solis; safe to edit by hand.";

/// The project's languages: the main one first, then the others in the
/// order they were added.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Languages {
    codes: Vec<String>,
}

impl Default for Languages {
    fn default() -> Self {
        Self {
            codes: vec![STANDARD.to_owned()],
        }
    }
}

/// A languages file, read: its languages, and notes on any left out.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Read {
    /// The languages.
    pub languages: Languages,
    /// What was left out, and why, in plain words.
    pub notes: Vec<String>,
}

impl Languages {
    /// The main language: the one the game is written in.
    pub fn main(&self) -> &str {
        self.codes.first().map_or(STANDARD, String::as_str)
    }

    /// Every language, the main one first.
    pub fn all(&self) -> &[String] {
        &self.codes
    }

    /// The languages other than the main one, in order.
    pub fn others(&self) -> &[String] {
        self.codes.get(1..).unwrap_or_default()
    }

    /// Whether the project has the language `code` (any capitals).
    pub fn has(&self, code: &str) -> bool {
        self.codes.iter().any(|c| c.eq_ignore_ascii_case(code))
    }

    /// Adds the language `code` at the end; or why it can't be.
    pub fn add(&mut self, code: &str) -> Result<(), String> {
        if let Some(why) = code_problem(code) {
            return Err(why);
        }
        if self.has(code) {
            return Err(format!("The project already has {code}."));
        }
        if self.codes.len() >= MAX_LANGUAGES {
            return Err(format!(
                "A project can have at most {MAX_LANGUAGES} languages."
            ));
        }
        self.codes.push(code.to_owned());
        Ok(())
    }

    /// Makes `code`, one of the project's languages, the main one; whether
    /// it was one.
    pub fn set_main(&mut self, code: &str) -> bool {
        let Some(at) = self.codes.iter().position(|c| c == code) else {
            return false;
        };
        let main = self.codes.remove(at);
        self.codes.insert(0, main);
        true
    }

    /// Takes the language `code` out of the project; whether it was one
    /// that could be (the main language can't).
    pub fn remove(&mut self, code: &str) -> bool {
        if self.main() == code {
            return false;
        }
        let before = self.codes.len();
        self.codes.retain(|c| c != code);
        self.codes.len() != before
    }
}

/// Why `code` can't be a language's code, in plain words; `None` when it
/// can. A code is a language (two or three small letters), then any parts
/// after a dash: a script (`Hans`) or a country (`BR`).
pub fn code_problem(code: &str) -> Option<String> {
    let example = "Use a language's short code, such as en, fr, nb, de or pt-BR";
    if code.is_empty() {
        return Some(format!("{example}."));
    }
    if code.chars().count() > MAX_CODE_CHARS {
        return Some(format!(
            "A language's code is at most {MAX_CODE_CHARS} characters. {example}."
        ));
    }
    let mut parts = code.split('-');
    let first = parts.next().unwrap_or_default();
    if !(2..=3).contains(&first.len()) || !first.chars().all(|c| c.is_ascii_lowercase()) {
        return Some(format!(
            "{code} doesn't start with a language: two or three small letters. {example}."
        ));
    }
    for part in parts {
        if !(2..=8).contains(&part.len()) || !part.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Some(format!(
                "{code} has a part that can't be one: each part after a dash is 2 to 8 letters \
                 or digits. {example}."
            ));
        }
    }
    None
}

/// `languages` as a languages file that [`read`] reads back.
pub fn write(languages: &Languages) -> String {
    let mut out = data::header(COMMENT, KIND, &["Languages"], VERSION);
    out.push_str(&format!(
        "lock Languages languages = {};\n",
        data::thing(
            "Languages",
            &[
                ("main", data::text(languages.main())),
                (
                    "all",
                    data::texts(languages.all().iter().map(String::as_str))
                ),
            ],
        )
    ));
    out.push_str("end\n");
    out
}

/// Reads a languages file. Refused, with its line, only when it can't be
/// read at all or is from a newer version; a language that can't be used
/// is left out with a note. A main language not in the list is added.
pub fn read(text: &str) -> Result<Read, DataError> {
    let version = data::version_of(text).unwrap_or(0);
    if version > VERSION {
        return Err(data::error(
            0,
            format!(
                "it was saved by a newer version (languages {version}; this one reads up to \
                 {VERSION}). Update to open it."
            ),
        ));
    }
    let values = data::read(text, KIND, CLASSES, MAX_BYTES)?;
    let thing = data::get(&values, "languages")?;
    let mut notes = Vec::new();
    let mut codes: Vec<String> = Vec::new();
    let main = data::field_text(thing, "main");
    let listed = data::field_texts(thing, "all");
    for code in std::iter::once(main).chain(listed) {
        let shown: String = code.chars().take(MAX_CODE_CHARS + 1).collect();
        if codes.iter().any(|c| c.eq_ignore_ascii_case(code)) {
            continue;
        }
        if codes.len() >= MAX_LANGUAGES {
            notes.push(format!(
                "Only the first {MAX_LANGUAGES} languages are kept."
            ));
            break;
        }
        match code_problem(code) {
            Some(why) => notes.push(format!("The language \"{shown}\" is left out: {why}")),
            None => codes.push(code.to_owned()),
        }
    }
    if codes.is_empty() {
        codes.push(STANDARD.to_owned());
        notes.push(format!(
            "No language could be read, so the main language is {STANDARD}."
        ));
    }
    Ok(Read {
        languages: Languages { codes },
        notes,
    })
}

#[cfg(test)]
#[path = "languages_tests.rs"]
mod tests;
