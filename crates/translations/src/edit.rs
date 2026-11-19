//! Changing one line in a Fluent file's text, keeping everything else as
//! written (comments, order, other lines): what the Translations tab and
//! a spreadsheet's import do.

use crate::ftl;
use crate::keys;

/// `file` (a Fluent file's text) with `key`'s line set to `text`, written
/// where it was or added at the end; or why `text` can't be written (it
/// isn't Fluent). Empty text takes the line out: a translation not made.
pub fn set_line(file: &str, key: &str, text: &str) -> Result<String, String> {
    let id = keys::fluent_id(key);
    if text.trim().is_empty() {
        return Ok(remove_line(file, key));
    }
    if text.chars().count() > ftl::MAX_TEXT_CHARS {
        return Err(format!(
            "A line of text is at most {} characters.",
            ftl::MAX_TEXT_CHARS
        ));
    }
    let entry = entry_text(&id, text);
    check_entry(&entry)?;
    let lines: Vec<&str> = file.lines().collect();
    let found = ftl::blocks(file).into_iter().find(|b| b.id == id);
    let mut out = String::with_capacity(file.len() + entry.len() + 2);
    match found {
        Some(block) => {
            for line in &lines[..block.lines.start] {
                out.push_str(line);
                out.push('\n');
            }
            out.push_str(&entry);
            // The line's attributes are kept, after its new text.
            for line in &lines[block.lines.clone()] {
                if line.trim_start().starts_with('.') {
                    out.push_str(line);
                    out.push('\n');
                }
            }
            for line in &lines[block.lines.end..] {
                out.push_str(line);
                out.push('\n');
            }
        }
        None => {
            out.push_str(file.trim_end());
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(&entry);
        }
    }
    Ok(out)
}

/// `file` with `key`'s line taken out (unchanged when it has none).
pub fn remove_line(file: &str, key: &str) -> String {
    let id = keys::fluent_id(key);
    let Some(block) = ftl::blocks(file).into_iter().find(|b| b.id == id) else {
        return file.to_owned();
    };
    let lines: Vec<&str> = file.lines().collect();
    let mut out = String::with_capacity(file.len());
    for line in lines[..block.lines.start]
        .iter()
        .chain(&lines[block.lines.end..])
    {
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// A line written as Fluent: `id = first line`, the rest indented.
fn entry_text(id: &str, text: &str) -> String {
    let mut lines = text.lines();
    let first = lines.next().unwrap_or_default();
    let mut out = format!("{id} = {}", first.trim_end());
    for line in lines {
        out.push('\n');
        if !line.trim().is_empty() {
            out.push_str("    ");
            out.push_str(line.trim_end());
        }
    }
    out.push('\n');
    out
}

/// Whether `entry` is one Fluent message, read back as written.
fn check_entry(entry: &str) -> Result<(), String> {
    match fluent_syntax::parser::parse(entry) {
        Ok(resource)
            if resource.body.len() == 1
                && matches!(&resource.body[0], fluent_syntax::ast::Entry::Message(m)
                    if m.value.is_some() && m.attributes.is_empty()) =>
        {
            Ok(())
        }
        Ok(_) => Err(
            "That text would be read as more than one line: start no line of it with ., [ or *."
                .into(),
        ),
        Err((_, errors)) => Err(format!(
            "That text isn't Fluent: {}. Write {{ and }} only around a value, such as {{ $name }}.",
            errors
                .first()
                .map_or_else(String::new, |e| e.kind.to_string())
        )),
    }
}

/// The files `changes` write, each path with its new text, starting from
/// each file's text now (`now`, empty for a file not made yet), and why any
/// change couldn't be made.
pub fn apply(
    changes: &[crate::sheet::Change],
    translations: &crate::Translations,
    now: impl Fn(&str) -> String,
) -> (Vec<(String, String)>, Vec<String>) {
    let mut files: Vec<(String, String)> = Vec::new();
    let mut notes = Vec::new();
    for change in changes {
        let path = translations.file_for(change.language, &change.key);
        let at = match files.iter().position(|(p, _)| *p == path) {
            Some(at) => at,
            None => {
                files.push((path.clone(), now(&path)));
                files.len() - 1
            }
        };
        match set_line(&files[at].1, &change.key, &change.text) {
            Ok(text) => files[at].1 = text,
            Err(why) => notes.push(format!(
                "{} in {} wasn't changed: {why}",
                change.key,
                change.language.name()
            )),
        }
    }
    (files, notes)
}

#[cfg(test)]
#[path = "edit_tests.rs"]
mod tests;
