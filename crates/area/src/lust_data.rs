//! Lust data files ([decision 0081](../../../docs/decisions/0081-what-solis-makes-is-lust.md)):
//! what Solis makes (tile maps, tilesets, and the rest as they move over)
//! is written as an ordinary Lust file whose top level holds only `lock`
//! values, and read back with Lust's own reader (`lust::read_values`),
//! which runs nothing and checks every type, class, field and option:
//!
//! ```text
//! -- A tile map. Written by Solis; safe to edit by hand.
//! fetch "solis/data/tilemap":[TileMap, Layer];
//!
//! lock int version = 3;
//! lock TileMap tiles = new TileMap { tileset = "1BWHNR3RV8PYY", size = [60, 40] };
//! lock Layer array layers = [
//!     new Layer { id = "3KD8V0Q2MX7TA", name = "Ground", over = false },
//! ];
//! end
//! ```
//!
//! The classes a kind of file holds are Lust too, kept with the crate that
//! reads it (`tiles::map_file::CLASSES`) and written into every project's
//! `solis/data/` so Ledger and `lust check` see them. Reading hands Lust
//! that text itself, so a file never depends on what's on disk beside it.
//!
//! This module is what every such format shares: writing values exactly,
//! reading a file within limits, and reading its values with plain-words
//! mistakes. A file read is someone else's, and Lust has no budget, so
//! only what the writers here write is handed to it ([`read`]).

pub use lust::{Fixed, Values};
use std::fmt::Write as _;

/// The folder inside a project's code where the data files' classes are
/// written (`solis/data/tilemap.lst`).
pub const CLASSES_FOLDER: &str = "data";

/// The engine's code folder, which data files fetch their classes from
/// (`fetch "solis/data/tilemap"`). It's the app's `branding::CODE_FOLDER`,
/// which a test there checks; RENAMING.md lists it.
pub const ENGINE_FOLDER: &str = "solis";

/// A file's own name when its text is handed to Lust.
const ENTRY: &str = "data.lst";

/// How many characters of a text a mistake about it shows.
const SHOWN: usize = 40;

/// Why a data file couldn't be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataError {
    /// The line it went wrong on (from 1), or 0 for the whole file.
    pub line: usize,
    /// What went wrong, in plain words.
    pub message: String,
}

impl std::fmt::Display for DataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.line == 0 {
            f.write_str(&self.message)
        } else {
            write!(f, "line {}: {}", self.line, self.message)
        }
    }
}

impl std::error::Error for DataError {}

/// A mistake on `line` (0 for the whole file).
pub fn error(line: usize, message: impl Into<String>) -> DataError {
    DataError {
        line,
        message: message.into(),
    }
}

/// Whether `c` is a character no data file's text holds: a control
/// character other than a line break or a tab, which can't be seen and
/// which Lust has no way to write but as itself. [`text`] leaves these
/// out and [`read`] refuses a file with one, so every text that's read is
/// written back as exactly itself.
fn unwritable(c: char) -> bool {
    c.is_control() && !matches!(c, '\n' | '\t')
}

/// `text` as a Lust text: in quotes, with `"`, `\`, braces, line breaks
/// and tabs written so they read back as themselves. Other control
/// characters are left out ([`unwritable`]).
pub fn text(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '{' => out.push_str("\\{"),
            '}' => out.push_str("\\}"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c if unwritable(c) => {}
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// A number as the shortest text that reads back as exactly it. A number
/// that isn't finite can't be written, and is written as 0.
pub fn float(v: f64) -> String {
    if !v.is_finite() {
        return "0.0".to_string();
    }
    decimal(format!("{v:?}"))
}

/// An `f32` as the shortest text that reads back as exactly it once read
/// and made an `f32` again (`0.1`, not `0.10000000149011612`).
pub fn single(v: f32) -> String {
    if !v.is_finite() {
        return "0.0".to_string();
    }
    decimal(format!("{v:?}"))
}

/// Number text with a `.` or an `e`, so Lust reads it as a decimal.
fn decimal(text: String) -> String {
    if text.contains(['.', 'e', 'E']) {
        text
    } else {
        format!("{text}.0")
    }
}

/// Texts as a Lust list: `["a", "b"]`.
pub fn texts<'a>(items: impl IntoIterator<Item = &'a str>) -> String {
    let parts: Vec<String> = items.into_iter().map(text).collect();
    format!("[{}]", parts.join(", "))
}

/// Whole numbers as a Lust list: `[4, 0, 2]`.
pub fn ints(items: impl IntoIterator<Item = i64>) -> String {
    let parts: Vec<String> = items.into_iter().map(|n| n.to_string()).collect();
    format!("[{}]", parts.join(", "))
}

/// A file's start: its comment, what it fetches from `solis/data/<kind>`,
/// and its version.
pub fn header(comment: &str, kind: &str, classes: &[&str], version: u32) -> String {
    let mut out = String::new();
    for line in comment.lines() {
        let _ = writeln!(out, "-- {line}");
    }
    let _ = writeln!(
        out,
        "fetch \"{ENGINE_FOLDER}/{CLASSES_FOLDER}/{kind}\":[{}];\n\nlock int version = {version};",
        classes.join(", ")
    );
    out
}

/// A list of things, one a line: `lock Layer array layers = [ … ];`, each
/// item already written (`new Layer { … }`).
pub fn list(out: &mut String, class: &str, name: &str, items: &[String]) {
    if items.is_empty() {
        let _ = writeln!(out, "lock {class} array {name} = [];");
        return;
    }
    let _ = writeln!(out, "lock {class} array {name} = [");
    for item in items {
        let _ = writeln!(out, "    {item},");
    }
    out.push_str("];\n");
}

/// A thing: `new Layer { id = "…", name = "Ground" }`, its fields already
/// written.
pub fn thing(class: &str, fields: &[(&str, String)]) -> String {
    let parts: Vec<String> = fields.iter().map(|(k, v)| format!("{k} = {v}")).collect();
    format!("new {class} {{ {} }}", parts.join(", "))
}

/// The version a data file says it is (`lock int version = 3;`), if it's
/// one.
pub fn version_of(text: &str) -> Option<u32> {
    text.lines()
        .map(str::trim)
        .find_map(|l| l.strip_prefix("lock int version ="))
        .and_then(|rest| rest.trim().strip_suffix(';'))
        .and_then(|n| n.trim().parse().ok())
}

/// Whether `text` is a Lust data file (rather than an older format of
/// the same kind).
pub fn is_data(text: &str) -> bool {
    version_of(text).is_some()
}

/// Reads a data file of `kind` whose classes are `classes` (Lust text),
/// refusing one bigger than `max_bytes`, one that holds anything but
/// values of those classes ([`crate::lust_check`]: Lust takes minutes over
/// a file full of mistakes, so it's handed none), and one whose text holds
/// a character that couldn't be written back ([`unwritable`]). Every
/// mistake comes back with its line; nothing in the file runs. The time
/// it takes grows only with the file's length.
pub fn read(text: &str, kind: &str, classes: &str, max_bytes: usize) -> Result<Values, DataError> {
    if text.len() > max_bytes {
        return Err(error(0, format!("it's bigger than {} MB", max_bytes >> 20)));
    }
    if let Some((line, found)) = crate::lust_guard::scan(text).next() {
        return Err(error(line as usize, not_data(found)));
    }
    crate::lust_check::Classes::read(kind, classes)?.check(text)?;
    let files = vec![
        (ENTRY.to_string(), text.to_string()),
        (
            format!("{ENGINE_FOLDER}/{CLASSES_FOLDER}/{kind}.lst"),
            classes.to_string(),
        ),
    ];
    let engine = lust::Engine {
        folders: vec![ENGINE_FOLDER.into()],
    };
    let values = lust::read_values_with(&engine, ENTRY, &files).map_err(|problems| {
        let first = problems.first();
        error(
            first.map_or(0, |p| p.line as usize),
            first.map_or_else(|| "it couldn't be read".to_string(), |p| p.message.clone()),
        )
    })?;
    let lost = values
        .values
        .iter()
        .find_map(|v| Some((v.line, unwritable_text(&v.value)?)));
    match lost {
        Some((line, text)) => Err(error(line as usize, not_plain(text))),
        None => Ok(values),
    }
}

/// The first text in `value` (itself, or in its list or fields) holding a
/// character that couldn't be written back.
fn unwritable_text(value: &Fixed) -> Option<&str> {
    match value {
        Fixed::Text(text) => text.contains(unwritable).then_some(text),
        Fixed::List(items) => items.iter().find_map(unwritable_text),
        Fixed::Thing { fields, .. } => fields.iter().find_map(|(_, v)| unwritable_text(v)),
        _ => None,
    }
}

/// Why `text`, holding a character that couldn't be written back, keeps a
/// file from being read: with the start of the text (a `?` for each
/// character that can't be seen) and the first such character's number,
/// so it can be found.
fn not_plain(text: &str) -> String {
    let shown: String = text
        .chars()
        .take(SHOWN)
        .map(|c| if c.is_control() { '?' } else { c })
        .collect();
    let code = text.chars().find(|&c| unwritable(c)).map_or(0, u32::from);
    format!(
        "the text \"{shown}\" holds a character that can't be seen (U+{code:04X}), which a \
         data file can't keep: type the text again without it"
    )
}

/// Why what a guard found keeps a file from being a data file. Lust would
/// refuse each too, but only after checking the whole file, which two
/// classes with one name or a long run of `!` keep it from finishing
/// ([`crate::lust_guard`]): so none of them reaches it.
fn not_data(found: crate::lust_guard::Found<'_>) -> String {
    use crate::lust_guard::{Found, MAX_PREFIX_RUN};
    match found {
        Found::Declared(..) => "a data file holds only lock values: classes, methods and the \
                                rest go in a file of their own"
            .to_string(),
        Found::Value => "text in a data file is plain: it can't have values in { } (write \\{ \
                         for a plain {)"
            .to_string(),
        Found::LongRun => {
            format!("more than {MAX_PREFIX_RUN} `!` in a row, which no data file has")
        }
    }
}

/// The value called `name`, or a mistake saying it's missing.
pub fn get<'a>(values: &'a Values, name: &str) -> Result<&'a Fixed, DataError> {
    values
        .get(name)
        .ok_or_else(|| error(0, format!("it has no {name}")))
}

/// The things in the list called `name` (none if it's missing).
pub fn things<'a>(values: &'a Values, name: &str) -> &'a [Fixed] {
    values
        .get(name)
        .and_then(Fixed::as_list)
        .unwrap_or_default()
}

/// A thing's field as text ("" if it's missing).
pub fn field_text<'a>(thing: &'a Fixed, name: &str) -> &'a str {
    thing.field(name).and_then(Fixed::as_text).unwrap_or("")
}

/// A thing's field as a whole number.
pub fn field_int(thing: &Fixed, name: &str) -> Option<i64> {
    thing.field(name).and_then(Fixed::as_int)
}

/// A thing's field as a number.
pub fn field_float(thing: &Fixed, name: &str) -> Option<f64> {
    thing.field(name).and_then(Fixed::as_float)
}

/// A thing's field as yes or no (false if it's missing).
pub fn field_bool(thing: &Fixed, name: &str) -> bool {
    thing.field(name).and_then(Fixed::as_bool).unwrap_or(false)
}

/// A thing's field as a list (empty if it's missing).
pub fn field_list<'a>(thing: &'a Fixed, name: &str) -> &'a [Fixed] {
    thing
        .field(name)
        .and_then(Fixed::as_list)
        .unwrap_or_default()
}

/// A thing's field as texts.
pub fn field_texts<'a>(thing: &'a Fixed, name: &str) -> Vec<&'a str> {
    field_list(thing, name)
        .iter()
        .filter_map(Fixed::as_text)
        .collect()
}

/// A thing's field as whole numbers, or none if any isn't one.
pub fn field_ints(thing: &Fixed, name: &str) -> Option<Vec<i64>> {
    field_list(thing, name).iter().map(Fixed::as_int).collect()
}

/// A thing's state field's option, without the state's name (`Choice` for
/// `ValueKind.Choice`).
pub fn field_option<'a>(thing: &'a Fixed, name: &str) -> Option<&'a str> {
    match thing.field(name)? {
        Fixed::Option(full) => Some(full.rsplit_once('.').map_or(full.as_str(), |(_, o)| o)),
        _ => None,
    }
}

/// How a settings file's parts are kept as Lust (see [`write_blocks`]):
/// its top-level settings as one thing, and each kind of block as a list.
pub struct Shape<'a> {
    /// The kind of file: the word before `.lst`, and `solis/data/<kind>`.
    pub kind: &'a str,
    /// Every class and state the file fetches.
    pub fetches: &'a [&'a str],
    /// The class of its top-level settings and the value's name
    /// (`Tileset`, `tileset`).
    pub top: (&'a str, &'a str),
    /// For each block word, its class and its list's name
    /// (`terrain` → `Terrain`, `terrains`); a block's name is its `id`.
    pub blocks: &'a [(&'a str, &'a str, &'a str)],
}

/// A value as Lust writes it.
pub fn value(v: &reflect::Value) -> String {
    use reflect::Value as V;
    match v {
        V::Bool(b) => b.to_string(),
        V::Int(n) => n.to_string(),
        // An f32's own shortest text when it is one (a colour's 0.42), so
        // the file reads naturally; it reads back as that f32 exactly.
        V::Float(f) if f.is_finite() => crate::text::float_text(*f),
        V::Float(_) => "0.0".to_string(),
        V::Text(t) => text(t),
        V::List(items) => {
            let parts: Vec<String> = items.iter().map(value).collect();
            format!("[{}]", parts.join(", "))
        }
        V::Asset(None) | V::Entity(None) => "\"\"".to_string(),
        V::Asset(Some(r)) => match r.part {
            Some(part) => text(&format!("{}#{part}", r.asset)),
            None => text(&r.asset.to_string()),
        },
        V::Entity(Some(id)) => text(&id.to_string()),
    }
}

/// A Lust fixed value as the settings layer's: a state's option is its
/// option's name; `nothing` is none.
pub fn from_fixed(f: &Fixed) -> Option<reflect::Value> {
    use reflect::Value as V;
    Some(match f {
        Fixed::Int(n) => V::Int(*n),
        Fixed::Float(x) => V::Float(*x),
        Fixed::Text(t) => V::Text(t.clone()),
        Fixed::Bool(b) => V::Bool(*b),
        Fixed::Nothing => return None,
        Fixed::Option(full) => V::Text(
            full.rsplit_once('.')
                .map_or(full.as_str(), |(_, o)| o)
                .to_string(),
        ),
        Fixed::List(items) => V::List(items.iter().filter_map(from_fixed).collect()),
        Fixed::Thing { .. } => return None,
    })
}

/// Writes a settings file's fields and blocks as a Lust data file of
/// `shape`. A block's fields are written as its thing's, in order, after
/// its `id`; a field `value` turns into Lust with [`value`], unless
/// `options` says it's a state's option (`(word, field, state)`), written
/// as `State.Option`.
pub fn write_blocks(
    shape: &Shape<'_>,
    comment: &str,
    version: u32,
    fields: &[(&str, reflect::Value)],
    blocks: &[crate::settings::BlockOut<'_>],
    options: &[(&str, &str, &str)],
) -> String {
    let mut out = header(comment, shape.kind, shape.fetches, version);
    let written = |word: &str, key: &str, v: &reflect::Value| -> String {
        match (options.iter().find(|(w, k, _)| *w == word && *k == key), v) {
            (Some((_, _, state)), reflect::Value::Text(t)) => format!("{state}.{t}"),
            _ => value(v),
        }
    };
    let top: Vec<(&str, String)> = fields
        .iter()
        .map(|(k, v)| (*k, written("", k, v)))
        .collect();
    let _ = writeln!(
        out,
        "lock {} {} = {};",
        shape.top.0,
        shape.top.1,
        thing(shape.top.0, &top)
    );
    for (word, class, list_name) in shape.blocks {
        let items: Vec<String> = blocks
            .iter()
            .filter(|(w, _, _)| w == word)
            .map(|(_, name, fields)| {
                let mut parts = vec![("id", text(name))];
                parts.extend(fields.iter().map(|(k, v)| (*k, written(word, k, v))));
                thing(class, &parts)
            })
            .collect();
        list(&mut out, class, list_name, &items);
    }
    out.push_str("end\n");
    out
}

/// Reads a Lust data file of `shape` back into a settings file's fields
/// and blocks (each on the line its list starts), for the format's own
/// checks to read as they read the older layout.
pub fn read_blocks(
    text: &str,
    shape: &Shape<'_>,
    classes: &str,
    max_bytes: usize,
) -> Result<crate::settings::BlockSettings, DataError> {
    use crate::settings::{Block, BlockSettings, Field};
    let values = read(text, shape.kind, classes, max_bytes)?;
    let line_of = |name: &str| {
        values
            .values
            .iter()
            .find(|v| v.name == name)
            .map_or(0, |v| v.line as usize)
    };
    let fields_of = |thing: &Fixed, line: usize, skip: &str| -> Vec<Field> {
        match thing {
            Fixed::Thing { fields, .. } => fields
                .iter()
                .filter(|(k, _)| k != skip)
                .filter_map(|(k, v)| {
                    Some(Field {
                        line,
                        key: k.clone(),
                        value: from_fixed(v)?,
                    })
                })
                .collect(),
            _ => Vec::new(),
        }
    };
    let top = get(&values, shape.top.1)?;
    let mut out = BlockSettings {
        version: version_of(text).unwrap_or(0),
        fields: fields_of(top, line_of(shape.top.1), ""),
        blocks: Vec::new(),
    };
    for (word, _, list_name) in shape.blocks {
        let line = line_of(list_name);
        for item in things(&values, list_name) {
            out.blocks.push(Block {
                line,
                word: (*word).to_string(),
                name: field_text(item, "id").to_string(),
                fields: fields_of(item, line, "id"),
            });
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLASSES: &str = "state Kind = [Number, Switch];\n\
        class Item {\n    has {\n        string name = \"\";\n        float weight = 0;\n        \
        Kind kind = Kind.Number;\n        int array at = [];\n    }\n}\npass Kind, Item;\nend\n";

    fn sample(name: &str, weight: f64) -> String {
        let mut out = header("A test.\nTwo lines.", "test", &["Kind", "Item"], 2);
        let items = vec![thing(
            "Item",
            &[
                ("name", text(name)),
                ("weight", float(weight)),
                ("kind", "Kind.Switch".into()),
                ("at", ints([4, -1])),
            ],
        )];
        list(&mut out, "Item", "items", &items);
        list(&mut out, "Item", "none", &[]);
        out.push_str("end\n");
        out
    }

    #[test]
    fn what_is_written_reads_back_exactly() {
        let odd = "a \"b\" {c} \\ \n\tdone";
        for weight in [
            0.1,
            1.0 / 3.0,
            -2.5e-7,
            f64::from(0.1_f32),
            3.4028235e38,
            2.0,
        ] {
            let text = sample(odd, weight);
            assert_eq!(version_of(&text), Some(2));
            let values =
                read(&text, "test", CLASSES, 1 << 20).unwrap_or_else(|e| panic!("{e}\n{text}"));
            let item = &things(&values, "items")[0];
            assert_eq!(field_text(item, "name"), odd);
            assert_eq!(field_float(item, "weight"), Some(weight), "{text}");
            assert_eq!(field_option(item, "kind"), Some("Switch"));
            assert_eq!(field_ints(item, "at"), Some(vec![4, -1]));
            assert!(things(&values, "none").is_empty());
            assert!(things(&values, "missing").is_empty());
        }
        assert_eq!(float(f64::NAN), "0.0");
        assert_eq!(float(2.0), "2.0");
        assert_eq!(single(0.1), "0.1");
        assert_eq!(single(2.0), "2.0");
        assert_eq!(single(f32::INFINITY), "0.0");
        assert_eq!("0.1".parse::<f64>().unwrap() as f32, 0.1_f32);
        assert_eq!(text("bell\u{7}"), "\"bell\"");
    }

    #[test]
    fn every_character_is_written_back_or_refused() {
        // Found by the atlas fuzzer's first run: Lust reads a control
        // character in quotes as itself, and `text` leaves one out, so a
        // name read with one was written without it. Every character of
        // the first 65,536 and a few past them, a block at a time.
        let name_in = |text: &str| {
            read(text, "test", CLASSES, 1 << 20)
                .map(|values| field_text(&things(&values, "items")[0], "name").to_string())
        };
        let every: Vec<char> = ('\0'..='\u{ffff}')
            .chain(['\u{10000}', '\u{e0001}', '\u{10ffff}'])
            .collect();
        for block in every.chunks(256) {
            // Written by Solis, all but those it leaves out read back.
            let all: String = block.iter().collect();
            let kept: String = block.iter().filter(|c| !unwritable(**c)).collect();
            assert_eq!(name_in(&sample(&all, 1.0)), Ok(kept.clone()));
            // In a file as themselves (but the four Lust reads its own
            // way), each is read, so written back, as itself.
            let plain = kept.replace(['"', '\\', '{', '\n'], "");
            let file = sample("x", 1.0).replace("\"x\"", &format!("\"{plain}\""));
            assert_eq!(name_in(&file), Ok(plain.clone()));
            assert_eq!(name_in(&sample(&plain, 1.0)), Ok(plain));
        }
        // A file holding one that would be left out is refused, on the
        // line its value starts, with enough to find it.
        let unseen: Vec<char> = every.into_iter().filter(|c| unwritable(*c)).collect();
        assert_eq!(
            unseen.len(),
            63,
            "all 65 control characters but \\n and \\t"
        );
        for c in unseen {
            let file = sample("x", 1.0).replace("\"x\"", &format!("\"a{c}\tb\""));
            let why = name_in(&file).unwrap_err();
            assert_eq!(why.line, 6, "{c:?}");
            let code = u32::from(c);
            let said = format!("\"a??b\" holds a character that can't be seen (U+{code:04X})");
            assert!(why.message.contains(&said), "{c:?}: {why}");
        }
        // However long the text, the mistake stays short.
        let long = format!("{}\u{7}", "a".repeat(5000));
        let file = sample("x", 1.0).replace("\"x\"", &format!("\"{long}\""));
        assert!(name_in(&file).unwrap_err().message.len() < 200);
    }

    /// What reading `text` answers. A reader that never finishes fails the
    /// test here, rather than holding up the whole run: fixed, it answers
    /// in well under a thousandth of this long.
    fn read_in_time(text: &str) -> Result<Values, DataError> {
        let (sender, answer) = std::sync::mpsc::channel();
        let text = text.to_string();
        std::thread::spawn(move || sender.send(read(&text, "test", CLASSES, 1 << 20)));
        answer
            .recv_timeout(std::time::Duration::from_secs(30))
            .expect("the reader never finished")
    }

    #[test]
    fn two_things_with_one_name_are_refused_never_a_hang() {
        // Found by the tileset fuzzer's first run: Lust never finishes
        // checking two classes with one name, and a data file reached it
        // unguarded. The smallest such file, then every word that declares
        // one, and each way the second was hidden from the older guards.
        let header = "lock int version = 2;\n";
        for (line, body) in [
            (2, "class A{}class A{}"),
            (2, "class A {}\nclass A {}\nend\n"),
            (2, "object A {}\nobject A {}\nend\n"),
            (2, "component A {}\ncomponent A {}\nend\n"),
            (2, "node A {}\nnode A {}\nend\n"),
            (2, "schema A {}\nschema A {}\nend\n"),
            (
                2,
                "assembly A { has { int x = 0; } }\nassembly A { has { int x = 0; } }\nend\n",
            ),
            (2, "class A {}\nobject A {}\nend\n"),
            (2, "--[[ ]] class A {}\n--[[ ]] class A {}\nend\n"),
            (3, "// \"\\\nclass A {}\n// \"\\\nclass A {}\nend\n"),
            (2, "lua { \" } class A {}\nlua { \" } class A {}\nend\n"),
            (2, "' class A {}\n' class A {}\nend\n"),
            (2, "class @ A {}\nclass ! A {}\nend\n"),
            (3, "x = class\nclass A {}\nx = class\nclass A {}\nend\n"),
        ] {
            let why = read_in_time(&format!("{header}{body}")).unwrap_err();
            assert!(why.message.contains("only lock values"), "{body:?}: {why}");
            assert_eq!(why.line, line, "{body:?}");
        }
        // One class is as much a mistake as two, and so is one whose name
        // the file's own classes have.
        let one = sample("x", 1.0).replace("end\n", "class Item {}\nend\n");
        let why = read_in_time(&one).unwrap_err();
        assert!(why.message.contains("only lock values"), "{why}");
        assert_eq!(why.line, 10);
    }

    #[test]
    fn what_lust_is_slow_to_refuse_is_refused_first() {
        // The other case the game code's guards know: a long run of `!`.
        let run = "!".repeat(crate::lust_guard::MAX_PREFIX_RUN + 1);
        let bangs = sample("x", 1.0).replace("weight = 1.0", &format!("weight = {run}1.0"));
        let why = read_in_time(&bangs).unwrap_err();
        assert!(why.message.contains("in a row"), "{why}");
        assert_eq!(why.line, 7);
        // Text with a value in it could end somewhere else for Lust than
        // for the guards, so it's refused as Lust would refuse it.
        for name in ["{hp}", "{", "{(1)} class A {} class A {}"] {
            let text = sample("x", 1.0).replace("\"x\"", &format!("\"{name}\""));
            let why = read_in_time(&text).unwrap_err();
            assert!(why.message.contains("values in { }"), "{name:?}: {why}");
            assert_eq!(why.line, 7);
        }
        // Written by Solis, the same letters are only letters.
        let odd = format!("{{hp}} {{(1)}} class A {{}} class A {{}} {run} -- /* '");
        let values = read_in_time(&sample(&odd, 1.0)).unwrap();
        assert_eq!(field_text(&things(&values, "items")[0], "name"), odd);
    }

    #[test]
    fn no_mix_of_declarations_and_what_might_hide_them_hangs_the_reader() {
        // Fuzz-style: thousands of files of declarations, two or more of
        // one name, each after a few pieces that open or close a comment,
        // text or a lua block, or that Lust skips. Each must be answered,
        // and refused wherever the guards found something.
        const DECLARED: [&str; 8] = [
            "class A {}\n",
            "class A {} ",
            "object A {} ",
            "component A {}\n",
            "node A {} ",
            "schema A {} ",
            "assembly A { has { int x = 0; } } ",
            "class\nA\n{\n}\n",
        ];
        const BETWEEN: [&str; 32] = [
            "\"", "\\", "\n", "\r\n", "-- ", "--[[", "]] ", "// ", "/*", "lua ", "lua { ", "{ ",
            "} ", "'", "!", "@", "1", "1e", "2.5E+", "class ", "A ", "; ", "= ", "{a", "{(", "x ",
            "end\n", "é", "\u{a0}", "#", "&", "|",
        ];
        let mut seed = 0x5EC1_9000_D00D_F00Du64;
        let mut next = move |below: usize| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed >> 24) as usize % below
        };
        let mut refused = 0;
        for case in 0..3000 {
            let mut text = String::from("lock int version = 2;\n");
            for _ in 0..2 + next(3) {
                for _ in 0..next(4) {
                    text.push_str(BETWEEN[next(BETWEEN.len())]);
                }
                text.push_str(DECLARED[next(DECLARED.len())]);
            }
            let found = crate::lust_guard::scan(&text).next().is_some();
            let answer = read_in_time(&text);
            assert!(!found || answer.is_err(), "case {case}: {text:?}");
            refused += usize::from(found);
        }
        // Both sides of the guards are reached.
        assert!((1000..3000).contains(&refused), "{refused}");
    }

    #[test]
    fn mistakes_say_their_line_and_nothing_runs() {
        let text = sample("x", 1.0).replace("Kind.Switch", "Kind.Sideways");
        let why = read(&text, "test", CLASSES, 1 << 20).unwrap_err();
        assert_eq!(why.line, 7, "{why}");
        let runs = sample("x", 1.0).replace("weight = 1.0", "weight = Maths:sqrt(2)");
        assert!(read(&runs, "test", CLASSES, 1 << 20).is_err());
        let big = read(&sample("x", 1.0), "test", CLASSES, 10).unwrap_err();
        assert!(big.message.contains("bigger"), "{big}");
        assert!(!is_data("tilemap version 2;\n"));
        assert_eq!(version_of("lock int version = x;"), None);
    }
}
