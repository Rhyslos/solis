//! The translations' check, for Problems: lines missing in a language
//! (a count for each, the list in the Translations tab), keys nothing
//! uses, and values that don't match between languages, besides the
//! files' own mistakes.

use crate::book::Translations;
use crate::ftl::FileProblem;
use std::collections::HashSet;

/// How many keys a missing or unused note names before "and N more".
const NAMED: usize = 5;

/// Everything worth saying about `translations`, each where it is.
/// `used` holds every key game code, screens, sounds and tables use, in
/// small letters.
pub fn check(translations: &Translations, used: &HashSet<String>) -> Vec<FileProblem> {
    let mut out: Vec<FileProblem> = translations.problems().to_vec();
    let main = translations.main();
    let keys = translations.keys();
    for language in translations.languages().into_iter().skip(1) {
        let missing = translations.missing(language);
        if !missing.is_empty() {
            out.push(FileProblem {
                file: format!("{}/{}", crate::book::FOLDER, language.code()),
                line: 0,
                message: format!(
                    "{} is missing {} of {} lines: {}. The Translations tab lists them; until \
                     they're written, {} is shown.",
                    language.name(),
                    missing.len(),
                    keys.len(),
                    named(&missing),
                    main.name()
                ),
            });
        }
        for key in &keys {
            if let Some(problem) = values_problem(translations, language, key) {
                out.push(problem);
            }
        }
    }
    let unused: Vec<String> = translations
        .code_keys()
        .into_iter()
        .filter(|k| !used.contains(&k.to_ascii_lowercase()))
        .collect();
    if !unused.is_empty() {
        let first = translations.line(main, &unused[0]);
        out.push(FileProblem {
            file: first.as_ref().map_or_else(String::new, |l| l.file.clone()),
            line: first.map_or(0, |l| l.line),
            message: format!(
                "{} {} of the main language's text nothing uses: {}. Use {} in game code, a \
                 screen or a sound's caption, or take {} out.",
                unused.len(),
                if unused.len() == 1 { "line" } else { "lines" },
                named(&unused),
                if unused.len() == 1 { "it" } else { "them" },
                if unused.len() == 1 { "it" } else { "them" },
            ),
        });
    }
    out
}

/// A list of keys for a sentence: the first few, then how many more.
fn named(keys: &[String]) -> String {
    let shown: Vec<&str> = keys.iter().take(NAMED).map(String::as_str).collect();
    let more = keys.len().saturating_sub(NAMED);
    if more == 0 {
        shown.join(", ")
    } else {
        format!("{} and {more} more", shown.join(", "))
    }
}

/// Whether `key`'s line in `language` takes the same values as the main
/// language's: one it uses that the main doesn't give shows as
/// `{$name}`, and one it leaves out is lost.
fn values_problem(
    translations: &Translations,
    language: crate::Language,
    key: &str,
) -> Option<FileProblem> {
    let own = translations.line(language, key)?;
    let main = translations.line(translations.main(), key)?;
    let names = |l: &crate::ftl::Line| -> Vec<String> {
        let mut v: Vec<String> = l.values.iter().map(|(n, _)| n.clone()).collect();
        v.sort();
        v
    };
    let (theirs, ours) = (names(&main), names(&own));
    if theirs == ours {
        return None;
    }
    let extra: Vec<String> = ours
        .iter()
        .filter(|n| !theirs.contains(n))
        .map(|n| format!("${n}"))
        .collect();
    let lost: Vec<String> = theirs
        .iter()
        .filter(|n| !ours.contains(n))
        .map(|n| format!("${n}"))
        .collect();
    let mut said = Vec::new();
    if !extra.is_empty() {
        said.push(format!(
            "uses {}, which {} doesn't give (it would show as written)",
            extra.join(", "),
            translations.main().name()
        ));
    }
    if !lost.is_empty() {
        said.push(format!("leaves out {}", lost.join(", ")));
    }
    Some(FileProblem {
        file: own.file,
        line: own.line,
        message: format!(
            "{key} in {} {}: its values should match {}'s.",
            language.name(),
            said.join(" and "),
            translations.main().name()
        ),
    })
}

/// The keys `code` (a file of game code or a screen) names: each
/// `Text.<key>` as Lust writes it, each `<Group>Text:<method>` (a line
/// with values), and each `"Group.Name"` in quotes (Lua and Rust), any
/// of `keys`, added to `used` in small letters. The code is read once,
/// so the time grows with the code and the keys, never their product.
pub fn note_uses(code: &str, keys: &[String], used: &mut HashSet<String>) {
    let words = words_in(code);
    for key in keys {
        let k = key.to_ascii_lowercase();
        if used.contains(&k) {
            continue;
        }
        let group = key.split('.').next().unwrap_or_default();
        let method = format!(
            "{}:{}",
            crate::keys::group_object(group).to_ascii_lowercase(),
            crate::keys::method_name(key).to_ascii_lowercase()
        );
        if words.contains(&format!("text.{k}")) || words.contains(&k) || words.contains(&method) {
            used.insert(k);
        }
    }
}

/// The names in `code`, in small letters: each run of letters, digits,
/// `_` and `.` (`text.shop.welcome`, and a quoted `shop.welcome`), and
/// each `name:method` pair.
fn words_in(code: &str) -> HashSet<String> {
    let mut out = HashSet::new();
    let mut word = String::new();
    let mut last: Option<String> = None;
    let mut colon = false;
    for c in code.chars().chain(std::iter::once(' ')) {
        if c.is_ascii_alphanumeric() || c == '_' || c == '.' {
            word.push(c.to_ascii_lowercase());
            continue;
        }
        if !word.is_empty() {
            let w = word.trim_matches('.').to_owned();
            if colon && let Some(before) = &last {
                let object = before.rsplit('.').next().unwrap_or(before);
                out.insert(format!("{object}:{w}"));
            }
            out.insert(w.clone());
            last = Some(w);
            word.clear();
        }
        colon = c == ':';
        if !colon && !c.is_whitespace() {
            last = None;
        }
    }
    out
}

#[cfg(test)]
#[path = "check_tests.rs"]
mod tests;
