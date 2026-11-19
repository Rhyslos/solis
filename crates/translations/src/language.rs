//! A language, by the short code translators and Windows use (`en`, `nb`,
//! `pt-BR`), kept inline so the player's settings can be copied freely.

use std::fmt;
use unic_langid::LanguageIdentifier;

/// The longest code kept, in bytes (`level::languages::MAX_CODE_CHARS`).
const MAX: usize = level::languages::MAX_CODE_CHARS;

/// A language's code: `en`, `nb`, `pt-BR`, `zh-Hans`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Language {
    bytes: [u8; MAX],
    len: u8,
}

/// Languages' own names, as a list of languages offers them, by code.
/// The editor's letters are Latin, Greek and Cyrillic, so a language
/// written otherwise is named in English.
const NAMES: &[(&str, &str)] = &[
    ("ar", "Arabic"),
    ("bg", "Български"),
    ("ca", "Català"),
    ("cs", "Čeština"),
    ("da", "Dansk"),
    ("de", "Deutsch"),
    ("el", "Ελληνικά"),
    ("en", "English"),
    ("en-GB", "English (UK)"),
    ("en-US", "English (US)"),
    ("es", "Español"),
    ("es-419", "Español (Latinoamérica)"),
    ("et", "Eesti"),
    ("fi", "Suomi"),
    ("fr", "Français"),
    ("fr-CA", "Français (Canada)"),
    ("he", "Hebrew"),
    ("hi", "Hindi"),
    ("hr", "Hrvatski"),
    ("hu", "Magyar"),
    ("id", "Bahasa Indonesia"),
    ("is", "Íslenska"),
    ("it", "Italiano"),
    ("ja", "Japanese"),
    ("ko", "Korean"),
    ("lt", "Lietuvių"),
    ("lv", "Latviešu"),
    ("nb", "Norsk"),
    ("nl", "Nederlands"),
    ("nn", "Nynorsk"),
    ("pl", "Polski"),
    ("pt", "Português"),
    ("pt-BR", "Português (Brasil)"),
    ("ro", "Română"),
    ("ru", "Русский"),
    ("sk", "Slovenčina"),
    ("sl", "Slovenščina"),
    ("sr", "Српски"),
    ("sv", "Svenska"),
    ("th", "Thai"),
    ("tr", "Türkçe"),
    ("uk", "Українська"),
    ("vi", "Tiếng Việt"),
    ("zh-Hans", "Chinese (simplified)"),
    ("zh-Hant", "Chinese (traditional)"),
];

impl Language {
    /// The language `code`, or why it can't be one, in plain words.
    pub fn new(code: &str) -> Result<Self, String> {
        if let Some(why) = level::languages::code_problem(code) {
            return Err(why);
        }
        let mut bytes = [0; MAX];
        bytes[..code.len()].copy_from_slice(code.as_bytes());
        Ok(Self {
            bytes,
            len: code.len() as u8,
        })
    }

    /// English, a project's main language when it says nothing.
    pub fn english() -> Self {
        Self::new(level::languages::STANDARD).unwrap_or(Self {
            bytes: *b"en\0\0\0\0\0\0\0\0\0\0\0\0\0",
            len: 2,
        })
    }

    /// Its code.
    pub fn code(&self) -> &str {
        std::str::from_utf8(&self.bytes[..usize::from(self.len)]).unwrap_or_default()
    }

    /// The language part alone: `pt` for `pt-BR`.
    pub fn base(&self) -> &str {
        self.code().split('-').next().unwrap_or_default()
    }

    /// Its name, as a list of languages shows it: its own name (`Norsk`,
    /// `Deutsch`) where the editor's letters can write it, else in English;
    /// the code itself for a language Solis doesn't know.
    pub fn name(&self) -> String {
        let code = self.code();
        NAMES
            .iter()
            .find(|(c, _)| c.eq_ignore_ascii_case(code))
            .or_else(|| NAMES.iter().find(|(c, _)| *c == self.base()))
            .map_or_else(|| code.to_owned(), |(_, name)| (*name).to_owned())
    }

    /// The name game code calls it, as an option of `Language`: `En`,
    /// `PtBr`, `ZhHans`.
    pub fn option(&self) -> String {
        self.code()
            .split('-')
            .map(|part| {
                let mut chars = part.chars();
                chars.next().map_or_else(String::new, |first| {
                    first.to_ascii_uppercase().to_string() + &chars.as_str().to_ascii_lowercase()
                })
            })
            .collect()
    }

    /// For Fluent: the language its plural rules come from.
    pub(crate) fn id(&self) -> LanguageIdentifier {
        self.code().parse().unwrap_or_default()
    }
}

impl fmt::Debug for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Language({})", self.code())
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl Default for Language {
    fn default() -> Self {
        Self::english()
    }
}

/// Every language Solis can name, for + Language's list.
pub fn known() -> Vec<Language> {
    NAMES
        .iter()
        .filter_map(|(code, _)| Language::new(code).ok())
        .collect()
}

/// The language the computer is set to (Windows' display language), if it
/// can be read and named.
pub fn system() -> Option<Language> {
    let found = sys_locale::get_locale()?;
    // `en-US`, sometimes `en_US.UTF-8` elsewhere.
    let code = found.split(['.', '@']).next()?.replace('_', "-");
    Language::new(&code)
        .ok()
        .or_else(|| Language::new(code.split('-').next()?).ok())
}

/// Which of the project's `languages` (the main one first) to show a
/// player who wants `wanted`: that one, else one of the same language
/// (`nb` for `nb-NO`, `pt-BR` for `pt`), else the main one.
pub fn best(wanted: Option<Language>, languages: &[Language]) -> Language {
    let main = languages.first().copied().unwrap_or_default();
    let Some(wanted) = wanted else {
        return main;
    };
    languages
        .iter()
        .find(|l| l.code().eq_ignore_ascii_case(wanted.code()))
        .or_else(|| languages.iter().find(|l| l.code() == wanted.base()))
        .or_else(|| languages.iter().find(|l| l.base() == wanted.base()))
        .copied()
        .unwrap_or(main)
}

/// The language whose option game code calls `option` (`PtBr`), among
/// `languages`.
pub fn of_option(option: &str, languages: &[Language]) -> Option<Language> {
    languages
        .iter()
        .find(|l| l.option().eq_ignore_ascii_case(option))
        .copied()
}

#[cfg(test)]
#[path = "language_tests.rs"]
mod tests;
