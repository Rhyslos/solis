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
    let mut fields: Vec<Field> = Vec::new();
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    while p.skip_space() {
        let line = p.line;
        let key = p.word()?;
        if let Some(first) = seen.get(&key) {
            return p.fail(&format!("{key} is set twice (first on line {first})"));
        }
        seen.insert(key.clone(), line);
        if fields.len() == MAX_SETTINGS_FIELDS {
            return p.fail(&format!("more than {MAX_SETTINGS_FIELDS} settings"));
        }
        p.skip_space();
        p.expect('=')?;
        let value = p.value(0)?;
        p.skip_space();
        p.expect(';')?;
        fields.push(Field { line, key, value });
    }
    Ok(Settings { version, fields })
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
        assert!(err("area version 1;").message.contains("isn't a sound file"));
        assert!(err("sound version 2;").message.contains("newer version"));
        assert!(err("sound version 0;").message.contains("no layout version 0"));
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
    fn comments_and_a_byte_order_mark_are_fine() {
        let text = "\u{feff}-- hi\nsound version 1; -- the header\n-- a = 1;\nb = [1, 2.5]; -- two\n";
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
