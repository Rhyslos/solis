//! The game's timers' names, found where its Lust starts them
//! ([decision 0124](../../../docs/decisions/0124-named-timers-now-blocks-later.md)).
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
    let mut names = BTreeSet::new();
    for (path, text) in &code.files {
        if crate::describe::in_engine_folder(path) {
            continue;
        }
        let words = words_of(text);
        for (i, word) in words.iter().enumerate() {
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

/// The words and marks of Lust `text`, with its comments and text left
/// out: names (letters, digits and `_`) whole, every other mark alone.
fn words_of(text: &str) -> Vec<String> {
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
