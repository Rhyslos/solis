//! One Fluent file (`locale/nb/shop.ftl`), read within limits: its lines
//! (each a key, where it's written, its text as written, and the values
//! it takes) and its mistakes, each at its line. Files are the project's,
//! so they're read as hostile: past the limits the rest is left out with a
//! note, and nothing in one can stop the game.

use crate::keys;
use fluent_syntax::ast;

/// The most bytes one `.ftl` file may have.
pub const MAX_FILE_BYTES: usize = 4 << 20;

/// The most characters one line of text may have, as written.
pub const MAX_TEXT_CHARS: usize = 10_000;

/// The most lines one language may have, all its files together.
pub const MAX_LINES: usize = 100_000;

/// How a value is used: shown as text, or as a number (which picks a
/// plural, `{ $count -> [one] … }`, or is formatted as one).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ValueKind {
    /// Shown as it is: a name, a word.
    Text,
    /// A number: it picks between forms, or is passed to `NUMBER()`.
    Number,
}

/// One line of text in a language.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    /// Its key (`Shop.Welcome`).
    pub key: String,
    /// The file it's written in (`locale/nb/shop.ftl`).
    pub file: String,
    /// Its line in the file, from 1.
    pub line: u32,
    /// Its text as written after the `=`, every line of it, indents taken
    /// off (`Welcome, { $name }!`).
    pub text: String,
    /// The values it takes (`name`), each once, in the order first used.
    pub values: Vec<(String, ValueKind)>,
}

/// A mistake in a file, where it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileProblem {
    /// The file (`locale/nb/shop.ftl`).
    pub file: String,
    /// Its line, from 1 (0 for the whole file).
    pub line: u32,
    /// What's wrong, in plain words.
    pub message: String,
}

/// A file, read.
#[derive(Debug, Default)]
pub struct ReadFile {
    /// Its lines, in the order written.
    pub lines: Vec<Line>,
    /// Its mistakes.
    pub problems: Vec<FileProblem>,
}

/// Reads `text`, the file at `path`.
pub fn read(path: &str, text: &str) -> ReadFile {
    let mut read = ReadFile::default();
    let problem = |line: u32, message: String| FileProblem {
        file: path.to_owned(),
        line,
        message,
    };
    if text.len() > MAX_FILE_BYTES {
        read.problems.push(problem(
            0,
            format!(
                "The file is {} KB; a translation file can be up to {} KB, so it's left out.",
                text.len() >> 10,
                MAX_FILE_BYTES >> 10
            ),
        ));
        return read;
    }
    let resource = match fluent_syntax::parser::parse(text) {
        Ok(resource) => resource,
        Err((resource, errors)) => {
            for e in errors.iter().take(100) {
                read.problems.push(problem(
                    line_at(text, e.pos.start),
                    format!("This isn't Fluent: {}.", e.kind),
                ));
            }
            resource
        }
    };
    let places = blocks(text);
    let mut seen: Vec<&str> = Vec::new();
    for entry in &resource.body {
        let ast::Entry::Message(message) = entry else {
            continue;
        };
        let id = message.id.name;
        // The second of two of a name is the second block of it.
        let before = seen.iter().filter(|s| **s == id).count();
        seen.push(id);
        let (line, written) = places
            .iter()
            .filter(|b| b.id == id)
            .nth(before)
            .map_or((0, String::new()), |b| (b.line, b.text.clone()));
        let Some(value) = &message.value else {
            read.problems.push(problem(
                line,
                format!("{id} has only attributes; Solis reads a line's text after its =."),
            ));
            continue;
        };
        let key = match keys::key_of(id) {
            Ok(key) => key,
            Err(why) => {
                read.problems.push(problem(
                    line,
                    format!(
                        "{id} can't be a key: {why} (in a file, its dots are dashes: Shop-Welcome)"
                    ),
                ));
                continue;
            }
        };
        if written.chars().count() > MAX_TEXT_CHARS {
            read.problems.push(problem(
                line,
                format!("{key} is longer than {MAX_TEXT_CHARS} characters, so it's left out."),
            ));
            continue;
        }
        if read.lines.iter().any(|l| l.key.eq_ignore_ascii_case(&key)) {
            read.problems.push(problem(
                line,
                format!("{key} is written twice in this file; the first is used."),
            ));
            continue;
        }
        let mut values = Vec::new();
        pattern_values(value, &mut values, 0);
        read.lines.push(Line {
            key,
            file: path.to_owned(),
            line,
            text: written,
            values,
        });
    }
    read
}

/// The line `at` (a byte in `text`) is on, from 1.
fn line_at(text: &str, at: usize) -> u32 {
    let at = at.min(text.len());
    let upto = text.get(..at).unwrap_or(text);
    (upto.bytes().filter(|b| *b == b'\n').count() + 1) as u32
}

/// How deep values are looked for inside placeables and forms.
const MAX_DEPTH: usize = 16;

/// Adds the values `pattern` uses to `out`.
fn pattern_values(pattern: &ast::Pattern<&str>, out: &mut Vec<(String, ValueKind)>, depth: usize) {
    if depth > MAX_DEPTH {
        return;
    }
    for element in &pattern.elements {
        if let ast::PatternElement::Placeable { expression } = element {
            expression_values(expression, out, depth + 1);
        }
    }
}

fn expression_values(e: &ast::Expression<&str>, out: &mut Vec<(String, ValueKind)>, depth: usize) {
    match e {
        ast::Expression::Select { selector, variants } => {
            inline_values(selector, out, depth, ValueKind::Number);
            for variant in variants {
                pattern_values(&variant.value, out, depth + 1);
            }
        }
        ast::Expression::Inline(inline) => inline_values(inline, out, depth, ValueKind::Text),
    }
}

fn inline_values(
    e: &ast::InlineExpression<&str>,
    out: &mut Vec<(String, ValueKind)>,
    depth: usize,
    kind: ValueKind,
) {
    if depth > MAX_DEPTH {
        return;
    }
    match e {
        ast::InlineExpression::VariableReference { id } => note(out, id.name, kind),
        ast::InlineExpression::FunctionReference { id, arguments } => {
            let kind = if id.name == "NUMBER" {
                ValueKind::Number
            } else {
                kind
            };
            for argument in &arguments.positional {
                inline_values(argument, out, depth + 1, kind);
            }
        }
        ast::InlineExpression::Placeable { expression } => {
            expression_values(expression, out, depth + 1);
        }
        _ => {}
    }
}

/// Notes a value used: a number once it's used as one anywhere.
fn note(out: &mut Vec<(String, ValueKind)>, name: &str, kind: ValueKind) {
    if let Some((_, was)) = out.iter_mut().find(|(n, _)| n == name) {
        *was = (*was).max(kind);
    } else if out.len() < 64 {
        out.push((name.to_owned(), kind));
    }
}

/// A message's place in a file, found by its lines: its name, its line,
/// and its text as written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Block {
    pub(crate) id: String,
    pub(crate) line: u32,
    /// The text after `=`, indents taken off, attributes left out.
    pub(crate) text: String,
    /// The lines it spans (from 0, the end not included), attributes and
    /// all: what replacing it replaces.
    pub(crate) lines: std::ops::Range<usize>,
}

/// Whether `name` is a Fluent message's name.
fn is_id(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Every message's block in `text`, in order: a line starting with a name
/// and `=`, then the indented or empty lines after it.
pub(crate) fn blocks(text: &str) -> Vec<Block> {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    let mut at = 0;
    while at < lines.len() {
        let line = lines[at];
        let starts = line
            .split_once('=')
            .filter(|(name, _)| !line.starts_with([' ', '\t']) && is_id(name.trim_end()));
        let Some((name, first)) = starts else {
            at += 1;
            continue;
        };
        let mut end = at + 1;
        while end < lines.len() && (lines[end].starts_with(' ') || lines[end].trim().is_empty()) {
            end += 1;
        }
        // Empty lines at the end belong to no message.
        while end > at + 1 && lines[end - 1].trim().is_empty() {
            end -= 1;
        }
        out.push(Block {
            id: name.trim_end().to_owned(),
            line: (at + 1) as u32,
            text: block_text(first, &lines[at + 1..end]),
            lines: at..end,
        });
        at = end;
    }
    out
}

/// A block's text: the rest of its first line, then the lines after it
/// up to its first attribute, their shared indent taken off.
fn block_text(first: &str, rest: &[&str]) -> String {
    let body: Vec<&str> = rest
        .iter()
        .take_while(|l| !l.trim_start().starts_with('.'))
        .copied()
        .collect();
    let indent = body
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.len() - l.trim_start().len())
        .min()
        .unwrap_or(0);
    let mut out: Vec<String> = Vec::with_capacity(body.len() + 1);
    let first = first.trim();
    if !first.is_empty() {
        out.push(first.to_owned());
    }
    for line in body {
        out.push(line.get(indent..).unwrap_or("").trim_end().to_owned());
    }
    while out.last().is_some_and(String::is_empty) {
        out.pop();
    }
    out.join("\n")
}

#[cfg(test)]
#[path = "ftl_tests.rs"]
mod tests;
