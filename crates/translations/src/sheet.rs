//! The spreadsheet translators work in: a CSV of each key, its text in
//! the main language, then a column per language, every field in double
//! quotes (a quote inside doubled, line breaks allowed inside), as the
//! dialogue prototype's export writes it. Export writes every key; import
//! reads one back and says which lines change.

use crate::book::Translations;
use crate::language::Language;

/// The spreadsheet's name, in the project's folder.
pub const FILE_NAME: &str = "translations.csv";

/// `translations` as a spreadsheet: `key`, then each language's code (the
/// main one first), then a line a key with its text as written in each
/// (empty where a language has none).
pub fn export(translations: &Translations) -> String {
    let languages = translations.languages();
    let mut out = String::new();
    let header: Vec<String> = std::iter::once("key".to_owned())
        .chain(languages.iter().map(|l| l.code().to_owned()))
        .collect();
    write_row(&mut out, &header);
    for key in translations.keys() {
        let mut row = Vec::with_capacity(languages.len() + 1);
        row.push(key.clone());
        for language in &languages {
            row.push(
                translations
                    .line(*language, &key)
                    .map(|l| l.text)
                    .unwrap_or_default(),
            );
        }
        write_row(&mut out, &row);
    }
    out
}

/// One line, every field in quotes.
fn write_row(out: &mut String, fields: &[String]) {
    let quoted: Vec<String> = fields
        .iter()
        .map(|f| format!("\"{}\"", f.replace('"', "\"\"")))
        .collect();
    out.push_str(&quoted.join(","));
    out.push_str("\r\n");
}

/// One line of text the spreadsheet changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    /// The language.
    pub language: Language,
    /// The key.
    pub key: String,
    /// Its new text (empty: the line taken out).
    pub text: String,
}

/// What a spreadsheet changes, and what couldn't be taken.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Imported {
    /// The lines that change, in the spreadsheet's order.
    pub changes: Vec<Change>,
    /// What was passed over, and why, in plain words.
    pub notes: Vec<String>,
}

/// Reads a spreadsheet `text` against `translations`: each language
/// column the project has, for each key it has, where the text differs.
/// A column of a language the project hasn't, or a key it hasn't, is
/// passed over with a note; the main language's text of a table's cell
/// is the table's, so that column is passed over for them. Refused, in
/// plain words, when it can't be read or has no `key` column.
pub fn import(text: &str, translations: &Translations) -> Result<Imported, String> {
    let rows = level::table::text::parse(text, ',')?;
    let Some((header, rows)) = rows.split_first() else {
        return Err("the spreadsheet is empty".into());
    };
    let key_at = header
        .iter()
        .position(|h| matches!(h.trim().to_ascii_lowercase().as_str(), "key" | "keys"))
        .ok_or("its first line has no key column (key, then a column per language)")?;
    let mut out = Imported::default();
    let mut columns: Vec<(usize, Language)> = Vec::new();
    let languages = translations.languages();
    for (at, name) in header.iter().enumerate() {
        if at == key_at {
            continue;
        }
        let code = name.trim();
        match languages
            .iter()
            .find(|l| l.code().eq_ignore_ascii_case(code))
        {
            Some(language) => columns.push((at, *language)),
            None => out.notes.push(format!(
                "The column {code} isn't one of the project's languages, so it's passed over: \
                 add it in Project settings › Languages first."
            )),
        }
    }
    let main = translations.main();
    let mut unknown = 0usize;
    for row in rows {
        let Some(key) = row.get(key_at).map(|k| k.trim()) else {
            continue;
        };
        if key.is_empty() {
            continue;
        }
        if !translations.has(key) {
            unknown += 1;
            continue;
        }
        for (at, language) in &columns {
            if *language == main && translations.is_table_key(key) {
                continue;
            }
            let new = row.get(*at).map_or("", String::as_str).trim_end();
            let now = translations
                .line(*language, key)
                .map(|l| l.text)
                .unwrap_or_default();
            if new != now {
                out.changes.push(Change {
                    language: *language,
                    key: key.to_owned(),
                    text: new.to_owned(),
                });
            }
        }
    }
    if unknown > 0 {
        out.notes.push(format!(
            "{unknown} {} keys the project hasn't, so {} passed over: a new key is made in the \
             main language first.",
            if unknown == 1 {
                "line has"
            } else {
                "lines have"
            },
            if unknown == 1 { "it's" } else { "they're" },
        ));
    }
    Ok(out)
}

#[cfg(test)]
#[path = "sheet_tests.rs"]
mod tests;
