//! The game's timers' names, found where its Lust starts them
//! ([decision 0125](../../../docs/decisions/0125-named-timers-now-blocks-later.md)).
//!
//! A timer's name is a project state, `Timer`, in `solis/project.lst`, as
//! the engine's calls and `TimerFinished` need one state for the whole
//! game: Lust can't yet hand an engine call a component's own state. So no
//! list is kept anywhere: the call that starts a timer names it
//! (`startTimer(Timer.DoorOpens, 2.0)`), Solis gathers every name started
//! in the game's Lust into `Timer`, and a misspelt name anywhere else
//! (`stopTimer(Timer.DoorOpnes)`, `when Timer.DorOpens`) is a mistake
//! before the game runs.

use crate::code::Code;
use std::collections::BTreeSet;

/// The calls that start a timer, and so name one.
pub const STARTS: &[&str] = &["startTimer", "startTimerEvery", "startTimerAtHour"];

/// The most timer names a game has: past it, the rest can't be started
/// from Lust (a guard against generated code).
pub const MAX_NAMES: usize = 1_024;

/// Every timer name the game's Lust starts (`Timer.DoorOpens` in a call
/// to one of [`STARTS`]), sorted, each once. Comments and text are
/// skipped, and so is the engine's own folder (which describes the
/// engine, and holds no game's timers).
pub fn names_started(code: &Code) -> Vec<String> {
    names_in(&code.files)
}

/// [`names_started`] of the game's Lust `files` (path and text): also
/// each timer made with `new` (the uniformity pass, U6): `new DoorOpens {
/// after = 2 }` names `DoorOpens`, a `new` of a name that isn't a sound
/// event's or something the game declares, with `after`, `every` or
/// `atHour` in its braces.
pub fn names_in(files: &[(String, String)]) -> Vec<String> {
    let mut names = BTreeSet::new();
    let states = crate::project_names::states_in(files);
    let sounds: BTreeSet<&str> = states
        .get("Sound")
        .map(|s| s.iter().map(String::as_str).collect())
        .unwrap_or_default();
    let mut declared: BTreeSet<String> = BTreeSet::new();
    for (path, text) in files {
        if crate::describe::in_engine_folder(path) {
            continue;
        }
        for (_, found) in level::lust_guard::scan(text) {
            if let level::lust_guard::Found::Declared(_, name) = found {
                declared.insert(name.to_owned());
            }
        }
    }
    for (path, text) in files {
        if crate::describe::in_engine_folder(path) {
            continue;
        }
        let words = words_of(text);
        for (i, word) in words.iter().enumerate() {
            // new Name { … after = … }
            if word == "new"
                && let [name, open, ..] = &words[i + 1..]
                && open == "{"
                && crate::timers::valid_name(name)
                && !sounds.contains(name.as_str())
                && !declared.contains(name)
                && timing_inside(&words[i + 3..])
            {
                names.insert(name.clone());
                if names.len() >= MAX_NAMES {
                    return names.into_iter().collect();
                }
                continue;
            }
            if !STARTS.contains(&word.as_str()) {
                continue;
            }
            // startTimer ( Timer . Name
            if let [open, state, dot, name, ..] = &words[i + 1..]
                && open == "("
                && state == "Timer"
                && dot == "."
                && crate::timers::valid_name(name)
            {
                names.insert(name.clone());
                if names.len() >= MAX_NAMES {
                    return names.into_iter().collect();
                }
            }
        }
    }
    names.into_iter().collect()
}

/// Whether the braces that `words` start inside (just after the `{`)
/// set one of a timer's fields at their own level: `after =`.
fn timing_inside(words: &[String]) -> bool {
    let mut depth = 0usize;
    for (i, word) in words.iter().enumerate() {
        match word.as_str() {
            "{" | "[" | "(" => depth += 1,
            "}" | "]" | ")" if depth == 0 => return false,
            "}" | "]" | ")" => depth -= 1,
            w if depth == 0
                && crate::made::TIMINGS.contains(&w)
                && words.get(i + 1).is_some_and(|n| n == "=") =>
            {
                return true;
            }
            _ => {}
        }
    }
    false
}

/// The words and marks of Lust `text`, with its comments and text left
/// out: names (letters, digits and `_`) whole, every other mark alone.
pub(crate) fn words_of(text: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '-' if chars.peek() == Some(&'-') => {
                chars.next();
                skip_comment(&mut chars);
            }
            '/' if chars.peek() == Some(&'/') => skip_line(&mut chars),
            '"' => skip_text(&mut chars),
            c if c.is_ascii_alphanumeric() || c == '_' => {
                let mut word = String::from(c);
                while let Some(&next) = chars.peek() {
                    if !(next.is_ascii_alphanumeric() || next == '_') {
                        break;
                    }
                    word.push(next);
                    chars.next();
                }
                words.push(word);
            }
            c if c.is_whitespace() => {}
            c => words.push(c.to_string()),
        }
    }
    words
}

/// Past a comment begun with `--`: a block (`--[[ … ]]`) or the rest of
/// the line.
fn skip_comment(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    if chars.peek() != Some(&'[') {
        return skip_line(chars);
    }
    chars.next();
    if chars.peek() != Some(&'[') {
        return skip_line(chars);
    }
    let mut last = '\0';
    for c in chars.by_ref() {
        if last == ']' && c == ']' {
            return;
        }
        last = c;
    }
}

/// Past the rest of the line.
fn skip_line(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    for c in chars.by_ref() {
        if c == '\n' {
            return;
        }
    }
}

/// Past a text's closing quote (a `\` keeps the next character in it).
fn skip_text(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                chars.next();
            }
            '"' | '\n' => return,
            _ => {}
        }
    }
}
