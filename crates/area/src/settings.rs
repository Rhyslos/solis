//! Settings files in the same Lust style as areas
//! ([decision 0020](../../../docs/decisions/0020-scene-files-in-lust-style.md)):
//! a sound event (`.solsound`), and later the mixer. A header
//! `<kind> version N;`, then one `key = value;` line per setting, with
//! `--` comments, so Git merges them as cleanly as areas.
//!
//! ```text
//! -- A sound event. Safe to edit by hand.
//! sound version 1;
//!
//! volume = -3.0;
//! bus = "Effects";
//! ```

use crate::text::{MAX_BYTES, MAX_LINE, ParseError, Parser, write_value};
use reflect::Value;
use std::collections::BTreeMap;
use std::fmt::Write as _;

/// The most fields a settings file may hold.
pub const MAX_SETTINGS_FIELDS: usize = 4096;

/// One `key = value;` line of a settings file.
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    /// The line it's on (from 1).
    pub line: usize,
    /// Its name.
    pub key: String,
    /// Its value.
    pub value: Value,
}

/// A settings file as read: its layout version and its fields, in the
/// order written.
#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    /// The layout version it was written with.
    pub version: u32,
    /// Its fields.
    pub fields: Vec<Field>,
}

/// Reads a settings file in the same Lust style as areas (a sound event,
/// the mixer): a header `<kind> version N;`, then `key = value;` lines,
/// with `--` comments. Read as a hostile file, like an area: within
/// limits, refusing a newer layout than `newest`, a key used twice, or
/// anything malformed, always with its line.
pub fn read_settings(text: &str, kind: &str, newest: u32) -> Result<Settings, ParseError> {
    let read = read_block_settings(text, kind, newest)?;
    if let Some(block) = read.blocks.first() {
        return Err(ParseError {
            line: block.line,
            message: format!("{} should be followed by '='", block.word),
        });
    }
    Ok(Settings {
        version: read.version,
        fields: read.fields,
    })
}

/// A block of a settings file: `word name { key = value; … }` (an
/// animator's states: `state 3KD8V0Q2MX7TA { name = "Walk"; … }`).
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    /// The line it starts on (from 1).
    pub line: usize,
    /// What it is (`state`).
    pub word: String,
    /// Its name or ID.
    pub name: String,
    /// Its fields, in the order written.
    pub fields: Vec<Field>,
}

/// A settings file with blocks as read: its layout version, the fields
/// outside blocks, and its blocks, each in the order written.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BlockSettings {
    /// The layout version it was written with.
    pub version: u32,
    /// The `key = value;` lines outside blocks.
    pub fields: Vec<Field>,
    /// The blocks.
    pub blocks: Vec<Block>,
}

/// Reads a settings file that may hold blocks as well as fields (an
/// animator, `.solanim`), as hostile as [`read_settings`]: within limits
/// ([`MAX_SETTINGS_FIELDS`] fields and blocks in all), refusing a newer
/// layout, a key used twice in one place, or anything malformed, always
/// with its line. Two blocks may share a word and name: what that means
/// is the reader's to say.
pub fn read_block_settings(
    text: &str,
    kind: &str,
    newest: u32,
) -> Result<BlockSettings, ParseError> {
    read_block_settings_within(text, kind, newest, MAX_SETTINGS_FIELDS)
}

/// As [`read_block_settings`], with room for up to `most` fields and
/// blocks together (a tile map's chunks are many).
pub fn read_block_settings_within(
    text: &str,
    kind: &str,
    newest: u32,
    most: usize,
) -> Result<BlockSettings, ParseError> {
    if text.len() > MAX_BYTES {
        return Err(ParseError {
            line: 0,
            message: format!(
                "the file is {} MB; the most read is {} MB",
                text.len() >> 20,
                MAX_BYTES >> 20
            ),
        });
    }
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    if let Some((line, _)) = text.lines().enumerate().find(|(_, l)| l.len() > MAX_LINE) {
        return Err(ParseError {
            line: line + 1,
            message: format!("a line longer than {MAX_LINE} characters"),
        });
    }
    let mut p = Parser {
        chars: text.char_indices().peekable(),
        line: 1,
    };
    if !p.skip_space() {
        return p.fail("the file is empty");
    }
    let found = p.word()?;
    if found != kind {
        return p.fail(&format!(
            "this isn't a {kind} file (it starts with \"{found}\", not \"{kind} version\")"
        ));
    }
    p.keyword("version")?;
    let version: u32 = p
        .word()?
        .parse()
        .or_else(|_| p.fail("the version should be a whole number"))?;
    if version > newest {
        return p.fail(&format!(
            "it was saved by a newer version (layout {version}; this one reads up to \
             {newest}). Update to open it."
        ));
    }
    if version == 0 {
        return p.fail("there is no layout version 0");
    }
    p.expect(';')?;
    let mut out = BlockSettings {
        version,
        ..BlockSettings::default()
    };
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    let mut count = 0usize;
    while p.skip_space() {
        let line = p.line;
        let key = p.word()?;
        count += 1;
        if count > most {
            return p.fail(&format!("more than {most} settings"));
        }
        p.skip_space();
        if p.peek() == Some('=') {
            if let Some(first) = seen.get(&key) {
                return p.fail(&format!("{key} is set twice (first on line {first})"));
            }
            seen.insert(key.clone(), line);
            out.fields.push(field_rest(&mut p, line, key)?);
            continue;
        }
        let name = p.word()?;
        p.expect('{')?;
        let mut fields: Vec<Field> = Vec::new();
        let mut inner: BTreeMap<String, usize> = BTreeMap::new();
        loop {
            if !p.skip_space() {
                return p.fail(&format!("the {key} {name} block isn't closed with '}}'"));
            }
            if p.peek() == Some('}') {
                p.expect('}')?;
                break;
            }
            let at = p.line;
            let inner_key = p.word()?;
            if let Some(first) = inner.get(&inner_key) {
                return p.fail(&format!(
                    "{inner_key} is set twice in {key} {name} (first on line {first})"
                ));
            }
            inner.insert(inner_key.clone(), at);
            if fields.len() == MAX_SETTINGS_FIELDS {
                return p.fail(&format!(
                    "more than {MAX_SETTINGS_FIELDS} settings in a block"
                ));
            }
            p.skip_space();
            fields.push(field_rest(&mut p, at, inner_key)?);
        }
        out.blocks.push(Block {
            line,
            word: key,
            name,
            fields,
        });
    }
    Ok(out)
}

/// The rest of a `key = value;` line, after its key.
fn field_rest(p: &mut Parser<'_>, line: usize, key: String) -> Result<Field, ParseError> {
    p.expect('=')?;
    let value = p.value(0)?;
    p.skip_space();
    p.expect(';')?;
    Ok(Field { line, key, value })
}

/// A block to write: its word, its name or ID, and its fields in order.
pub type BlockOut<'a> = (&'a str, String, Vec<(&'a str, Value)>);

/// Writes a settings file with blocks that [`read_block_settings`] reads
/// back: the fields as [`write_settings`] does, then each block, its
/// fields indented a line each, with a blank line between blocks.
pub fn write_block_settings(
    kind: &str,
    version: u32,
    comment: &str,
    fields: &[(&str, Value)],
    blocks: &[BlockOut<'_>],
) -> String {
    let mut out = write_settings(kind, version, comment, fields);
    for (word, name, fields) in blocks {
        let _ = writeln!(out, "\n{word} {name} {{");
        for (key, value) in fields {
            let _ = write!(out, "    {key} = ");
            write_value(&mut out, value);
            out.push_str(";\n");
        }
        out.push_str("}\n");
    }
    out
}

/// Writes a settings file that [`read_settings`] reads back: `comment` as
/// `--` lines, the header, then one `key = value;` line per field, in the
/// order given.
pub fn write_settings(kind: &str, version: u32, comment: &str, fields: &[(&str, Value)]) -> String {
    let mut out = String::new();
    for line in comment.lines() {
        let _ = writeln!(out, "-- {line}");
    }
    let _ = writeln!(out, "{kind} version {version};");
    out.push('\n');
    for (key, value) in fields {
        let _ = write!(out, "{key} = ");
        write_value(&mut out, value);
        out.push_str(";\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_round_trip() {
        let fields = [
            ("volume", Value::Float(-3.0)),
            ("loop", Value::Bool(false)),
            ("at_once", Value::Int(8)),
            ("bus", Value::Text("Effects \"loud\"".into())),
            (
                "clips",
                Value::List(vec![Value::Text("2EGJNFMTCSQ8G".into())]),
            ),
        ];
        let text = write_settings("sound", 1, "A sound event.\nSafe to edit.", &fields);
        assert!(text.starts_with("-- A sound event.\n-- Safe to edit.\nsound version 1;\n"));
        let read = read_settings(&text, "sound", 1).unwrap();
        assert_eq!(read.version, 1);
        let back: Vec<(&str, Value)> = read
            .fields
            .iter()
            .map(|f| (f.key.as_str(), f.value.clone()))
            .collect();
        assert_eq!(back, fields);
        assert_eq!(read.fields[0].line, 5);
    }

    #[test]
    fn mistakes_are_refused_with_their_line() {
        let err = |text: &str| read_settings(text, "sound", 1).unwrap_err();
        assert_eq!(err("").message, "the file is empty");
        assert!(
            err("area version 1;")
                .message
                .contains("isn't a sound file")
        );
        assert!(err("sound version 2;").message.contains("newer version"));
        assert!(
            err("sound version 0;")
                .message
                .contains("no layout version 0")
        );
        assert!(err("sound version x;").message.contains("whole number"));
        let twice = err("sound version 1;\na = 1;\na = 2;");
        assert_eq!(twice.line, 3);
        assert!(twice.message.contains("set twice (first on line 2)"));
        assert_eq!(err("sound version 1;\n\nvolume 3;").line, 3);
        assert_eq!(err("sound version 1;\nv = ;").line, 2);
        assert!(err("sound version 1;\nv = 1").message.contains("';'"));
        let long = format!("sound version 1;\nv = \"{}\";", "x".repeat(MAX_LINE + 1));
        assert_eq!(err(&long).line, 2);
    }

    #[test]
    fn blocks_round_trip_beside_fields() {
        let blocks: Vec<BlockOut<'_>> = vec![
            (
                "state",
                "3KD8V0Q2MX7TA".into(),
                vec![
                    ("name", Value::Text("Walk".into())),
                    ("speed", Value::Float(1.5)),
                ],
            ),
            ("state", "7PQ2B5N8WK1ZC".into(), Vec::new()),
        ];
        let text = write_block_settings(
            "animator",
            1,
            "An animator.",
            &[("entry", Value::Text("3KD8V0Q2MX7TA".into()))],
            &blocks,
        );
        assert!(text.contains("state 3KD8V0Q2MX7TA {\n    name = \"Walk\";\n"));
        let read = read_block_settings(&text, "animator", 1).unwrap();
        assert_eq!(read.fields.len(), 1);
        assert_eq!(read.blocks.len(), 2);
        assert_eq!(read.blocks[0].word, "state");
        assert_eq!(read.blocks[0].name, "3KD8V0Q2MX7TA");
        assert_eq!(read.blocks[0].fields[1].value, Value::Float(1.5));
        assert_eq!(read.blocks[0].line, 6);
        assert!(read.blocks[1].fields.is_empty());
    }

    #[test]
    fn block_mistakes_are_refused_with_their_line() {
        let err = |text: &str| read_block_settings(text, "animator", 1).unwrap_err();
        let open = err("animator version 1;\nstate A {\n name = 1;\n");
        assert!(open.message.contains("isn't closed"), "{}", open.message);
        let twice = err("animator version 1;\nstate A {\n a = 1;\n a = 2;\n}");
        assert_eq!(twice.line, 4);
        assert!(twice.message.contains("twice in state A"));
        assert_eq!(err("animator version 1;\nstate A b = 1;").line, 2);
        assert_eq!(err("animator version 1;\nstate { }").line, 2);
        // A plain settings file has no blocks.
        let blocky = read_settings("sound version 1;\nstate A { }", "sound", 1).unwrap_err();
        assert!(blocky.message.contains("followed by '='"));
    }

    #[test]
    fn comments_and_a_byte_order_mark_are_fine() {
        let text =
            "\u{feff}-- hi\nsound version 1; -- the header\n-- a = 1;\nb = [1, 2.5]; -- two\n";
        let read = read_settings(text, "sound", 1).unwrap();
        assert_eq!(read.fields.len(), 1);
        assert_eq!(
            read.fields[0].value,
            Value::List(vec![Value::Int(1), Value::Float(2.5)])
        );
    }

    #[test]
    fn too_many_settings_are_refused() {
        let mut text = "sound version 1;\n".to_owned();
        for i in 0..=MAX_SETTINGS_FIELDS {
            let _ = writeln!(text, "k{i} = 1;");
        }
        assert!(
            read_settings(&text, "sound", 1)
                .unwrap_err()
                .message
                .contains("more than")
        );
    }
}
