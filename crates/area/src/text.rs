//! Areas as text ([decision 0020](../../../docs/decisions/0020-scene-files-in-lust-style.md)):
//! Lust-style data blocks, one per entity, sorted by ID, one line per
//! field, made so Git merges two people's changes cleanly.
//!
//! ```text
//! -- An area. Safe to edit by hand: one block per entity, one line per field.
//! area version 1;
//!
//! entity 2B8D4NE6TV0HC {
//!     name = "Sun";
//!     order = "a0";
//!     Light.color = [1.0, 0.95, 0.88];
//!     Light.strength = 3.0;
//! }
//! ```
//!
//! This module only turns text into an [`Area`] and back; reading and
//! writing files is the app's job. The reader is written for hostile
//! files (CONTRIBUTING.md, rule 9): sizes are checked before anything is
//! kept, and every mistake is reported with its line, never a crash.

use crate::document::{Area, EntityData};
use reflect::{AssetId, AssetRef, ComponentData, EntityId, FieldKind, Registry, Value};
use std::collections::BTreeMap;
use std::fmt::{self, Write as _};

/// The layout version this build writes and reads.
pub const VERSION: u32 = 1;
/// The biggest area file read.
pub const MAX_BYTES: usize = 64 << 20;
/// The longest line read.
pub const MAX_LINE: usize = 64 << 10;
/// The most entities in one file.
pub const MAX_ENTITIES: usize = 1_000_000;
/// The most fields in one entity's block.
pub const MAX_FIELDS: usize = 4096;
/// How deep lists may nest.
const MAX_DEPTH: usize = 16;

/// Why a file couldn't be read, and where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// The line it went wrong on (from 1), or 0 for the whole file.
    pub line: usize,
    /// What went wrong, in plain words.
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line == 0 {
            write!(f, "{}", self.message)
        } else {
            write!(f, "line {}: {}", self.line, self.message)
        }
    }
}

impl std::error::Error for ParseError {}

// ---------------------------------------------------------------- write ---

/// `area` as text. Components `registry` knows write their fields in their
/// declared order (fields it doesn't know come after, sorted); others are
/// written sorted. `asset_path` names an asset for a comment beside its
/// ID, which is only for people: it's never read back.
pub fn write(
    area: &Area,
    registry: &Registry,
    asset_path: &dyn Fn(AssetId) -> Option<String>,
) -> String {
    let mut out = String::new();
    out.push_str("-- An area. Safe to edit by hand: one block per entity, one line per field.\n");
    let _ = writeln!(out, "area version {VERSION};");
    for (id, entity) in area.iter() {
        out.push('\n');
        let _ = writeln!(out, "entity {id} {{");
        let _ = writeln!(out, "    name = {};", quoted(&entity.name));
        if let Some(parent) = entity.parent {
            let _ = writeln!(out, "    parent = \"{parent}\";");
        }
        let _ = writeln!(out, "    order = {};", quoted(&entity.order));
        for (component, data) in &entity.components {
            let declared: Vec<&str> = registry
                .get(component)
                .map(|s| s.fields.iter().map(|f| f.name.as_str()).collect())
                .unwrap_or_default();
            let mut names: Vec<&str> = declared
                .iter()
                .copied()
                .filter(|f| data.fields.contains_key(*f))
                .collect();
            names.extend(
                data.fields
                    .keys()
                    .map(String::as_str)
                    .filter(|f| !declared.contains(f)),
            );
            if names.is_empty() {
                // A component with no fields still has to be kept.
                let _ = writeln!(out, "    {component} = {{}};");
            }
            for field in names {
                let value = &data.fields[field];
                let mut line = format!("    {component}.{field} = ");
                write_value(&mut line, value);
                line.push(';');
                if let Value::Asset(Some(r)) = value
                    && let Some(path) = asset_path(r.asset)
                {
                    let _ = write!(line, "  -- {}", path.replace('\n', " "));
                }
                out.push_str(&line);
                out.push('\n');
            }
        }
        out.push_str("}\n");
    }
    out
}

fn quoted(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c if c.is_control() => {
                let _ = write!(out, "\\u{{{:x}}}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The shortest text that reads back as exactly this number: as an `f32`
/// when it is one (every number Solis's own components hold), otherwise as
/// an `f64`, written so it can't be mistaken for an `f32`'s text (see
/// [`read_float`]). Always with a `.` or an `e`, so it reads as a decimal.
fn float_text(v: f64) -> String {
    let single = v as f32;
    if f64::from(single) == v {
        return single_text(single);
    }
    let rounded = single_text(single);
    [format!("{v:?}"), format!("{v:e}"), format!("{v:.17e}")]
        .into_iter()
        .map(|t| decimal(t, v))
        .find(|t| *t != rounded)
        .unwrap_or_else(|| format!("{v:.17e}"))
}

fn single_text(v: f32) -> String {
    decimal(format!("{v:?}"), f64::from(v))
}

fn decimal(text: String, v: f64) -> String {
    if text.contains(['.', 'e', 'E']) || !v.is_finite() {
        text
    } else {
        format!("{text}.0")
    }
}

fn write_value(out: &mut String, value: &Value) {
    match value {
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Int(n) => {
            let _ = write!(out, "{n}");
        }
        // Not finite numbers can't be written; they never reach a document
        // through the inspector, and come back as nothing.
        Value::Float(v) if !v.is_finite() => out.push_str("0.0"),
        Value::Float(v) => out.push_str(&float_text(*v)),
        Value::Text(t) => out.push_str(&quoted(t)),
        Value::List(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                write_value(out, item);
            }
            out.push(']');
        }
        Value::Asset(None) | Value::Entity(None) => out.push_str("\"\""),
        Value::Asset(Some(r)) => match r.part {
            Some(part) => {
                let _ = write!(out, "\"{}#{part}\"", r.asset);
            }
            None => {
                let _ = write!(out, "\"{}\"", r.asset);
            }
        },
        Value::Entity(Some(id)) => {
            let _ = write!(out, "\"{id}\"");
        }
    }
}

// ----------------------------------------------------------------- read ---

/// Reads an area from `text`. Values are turned into the kinds `registry`
/// declares (asset and entity references, decimals written as whole
/// numbers); components it doesn't know are kept exactly as read. A file
/// from a newer version is refused, never half-read.
pub fn read(text: &str, registry: &Registry) -> Result<Area, ParseError> {
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
    p.header()?;
    let mut area = Area::new();
    let mut lines: BTreeMap<EntityId, (usize, String)> = BTreeMap::new();
    while p.skip_space() {
        let start = p.line;
        p.keyword("entity")?;
        let id_text = p.word()?;
        if id_text.contains('/') {
            return p.fail("overrides inside assemblies (a/b) can't be read yet");
        }
        let id: EntityId = id_text.parse().map_err(|e| ParseError {
            line: start,
            message: format!("{e}"),
        })?;
        let entity = p.block(registry)?;
        if let Some((first, name)) = lines.get(&id) {
            return Err(ParseError {
                line: start,
                message: format!(
                    "the ID {id} is used twice: by \"{name}\" (line {first}) and \"{}\"",
                    entity.name
                ),
            });
        }
        if lines.len() >= MAX_ENTITIES {
            return p.fail(&format!("more than {MAX_ENTITIES} entities"));
        }
        lines.insert(id, (start, entity.name.clone()));
        area.insert(id, entity);
    }
    Ok(area)
}

struct Parser<'a> {
    chars: std::iter::Peekable<std::str::CharIndices<'a>>,
    line: usize,
}

impl Parser<'_> {
    fn fail<T>(&self, message: &str) -> Result<T, ParseError> {
        Err(ParseError {
            line: self.line,
            message: message.to_string(),
        })
    }

    fn peek(&mut self) -> Option<char> {
        self.chars.peek().map(|&(_, c)| c)
    }

    fn next(&mut self) -> Option<char> {
        let c = self.chars.next().map(|(_, c)| c);
        if c == Some('\n') {
            self.line += 1;
        }
        c
    }

    /// Skips spaces, line ends and `--` comments; whether anything is left.
    fn skip_space(&mut self) -> bool {
        loop {
            match self.peek() {
                Some(c) if c.is_whitespace() => {
                    self.next();
                }
                Some('-') => {
                    let rest = self.chars.clone().nth(1).map(|(_, c)| c);
                    if rest != Some('-') {
                        return true;
                    }
                    while self.peek().is_some_and(|c| c != '\n') {
                        self.next();
                    }
                }
                Some(_) => return true,
                None => return false,
            }
        }
    }

    fn expect(&mut self, wanted: char) -> Result<(), ParseError> {
        self.skip_space();
        match self.next() {
            Some(c) if c == wanted => Ok(()),
            Some(c) => self.fail(&format!("expected '{wanted}' but found '{c}'")),
            None => self.fail(&format!("expected '{wanted}' but the file ended")),
        }
    }

    /// A run of letters, digits and `_ . / #`.
    fn word(&mut self) -> Result<String, ParseError> {
        self.skip_space();
        let mut out = String::new();
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || matches!(c, '_' | '.' | '/' | '#') {
                out.push(c);
                self.next();
            } else {
                break;
            }
        }
        if out.is_empty() {
            let found = self
                .peek()
                .map_or("the end of the file".to_string(), |c| format!("'{c}'"));
            return self.fail(&format!("expected a name but found {found}"));
        }
        Ok(out)
    }

    fn keyword(&mut self, wanted: &str) -> Result<(), ParseError> {
        let word = self.word()?;
        if word == wanted {
            Ok(())
        } else {
            self.fail(&format!("expected \"{wanted}\" but found \"{word}\""))
        }
    }

    /// `area version N;`, refusing newer versions.
    fn header(&mut self) -> Result<(), ParseError> {
        if !self.skip_space() {
            return self.fail("the file is empty");
        }
        let kind = self.word()?;
        if kind != "area" {
            return self.fail(&format!(
                "this isn't an area file (it starts with \"{kind}\", not \"area version\")"
            ));
        }
        self.keyword("version")?;
        let version: u32 = self
            .word()?
            .parse()
            .or_else(|_| self.fail("the version should be a whole number"))?;
        if version > VERSION {
            return self.fail(&format!(
                "it was saved by a newer version (layout {version}; this one reads up to \
                 {VERSION}). Update to open it."
            ));
        }
        if version == 0 {
            return self.fail("there is no layout version 0");
        }
        self.expect(';')
    }

    /// `{ key = value; … }`
    fn block(&mut self, registry: &Registry) -> Result<EntityData, ParseError> {
        self.expect('{')?;
        let mut entity = EntityData::new("");
        let mut fields = 0usize;
        loop {
            self.skip_space();
            if self.peek() == Some('}') {
                self.next();
                return Ok(entity);
            }
            if self.peek().is_none() {
                return self.fail("the block isn't closed with '}'");
            }
            fields += 1;
            if fields > MAX_FIELDS {
                return self.fail(&format!("more than {MAX_FIELDS} fields in one entity"));
            }
            let line = self.line;
            let key = self.word()?;
            self.expect('=')?;
            self.skip_space();
            if self.peek() == Some('{') {
                // `Component = {};`: a component with no fields.
                self.next();
                self.expect('}')?;
                self.expect(';')?;
                entity.components.entry(key).or_default();
                continue;
            }
            let value = self.value(0)?;
            self.expect(';')?;
            self.apply(&mut entity, &key, value, line, registry)?;
        }
    }

    fn apply(
        &self,
        entity: &mut EntityData,
        key: &str,
        value: Value,
        line: usize,
        registry: &Registry,
    ) -> Result<(), ParseError> {
        let wrong = |what: &str| {
            Err(ParseError {
                line,
                message: format!("{key} should be {what}"),
            })
        };
        match (key, value) {
            ("name", Value::Text(t)) => entity.name = t,
            ("name", _) => return wrong("text"),
            ("order", Value::Text(t)) => entity.order = t,
            ("order", _) => return wrong("text"),
            ("parent", Value::Text(t)) => match t.parse() {
                Ok(id) => entity.parent = Some(id),
                Err(e) => return wrong(&format!("an entity's ID ({e})")),
            },
            ("parent", _) => return wrong("an entity's ID in quotes"),
            (key, value) => {
                let Some((component, field)) = key.split_once('.') else {
                    return wrong("Component.field (a component's name, a dot and a field)");
                };
                if component.is_empty() || field.is_empty() || field.contains('.') {
                    return wrong("Component.field");
                }
                let kind = registry
                    .get(component)
                    .and_then(|s| s.field(field))
                    .map(|f| f.kind.clone());
                let value = match kind {
                    Some(kind) => coerce(value, &kind),
                    None => value,
                };
                let data: &mut ComponentData =
                    entity.components.entry(component.to_string()).or_default();
                if data.fields.insert(field.to_string(), value).is_some() {
                    return Err(ParseError {
                        line,
                        message: format!("{key} is given twice"),
                    });
                }
            }
        }
        Ok(())
    }

    fn value(&mut self, depth: usize) -> Result<Value, ParseError> {
        if depth > MAX_DEPTH {
            return self.fail("lists nested too deeply");
        }
        self.skip_space();
        match self.peek() {
            Some('"') => self.string().map(Value::Text),
            Some('[') => {
                self.next();
                let mut items = Vec::new();
                loop {
                    self.skip_space();
                    if self.peek() == Some(']') {
                        self.next();
                        return Ok(Value::List(items));
                    }
                    items.push(self.value(depth + 1)?);
                    self.skip_space();
                    match self.next() {
                        Some(',') => {}
                        Some(']') => return Ok(Value::List(items)),
                        Some(c) => {
                            return self.fail(&format!("expected ',' or ']' but found '{c}'"));
                        }
                        None => return self.fail("a list isn't closed with ']'"),
                    }
                }
            }
            Some(c) if c.is_ascii_digit() || c == '-' || c == '+' => self.number(),
            Some(_) => {
                let word = self.word()?;
                match word.as_str() {
                    "true" => Ok(Value::Bool(true)),
                    "false" => Ok(Value::Bool(false)),
                    other => self.fail(&format!("\"{other}\" isn't a value (text goes in quotes)")),
                }
            }
            None => self.fail("a value is missing at the end of the file"),
        }
    }

    fn number(&mut self) -> Result<Value, ParseError> {
        let mut text = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+') {
                text.push(c);
                self.next();
            } else {
                break;
            }
        }
        let decimal = text.contains(['.', 'e', 'E']);
        if !decimal && let Ok(n) = text.parse::<i64>() {
            return Ok(Value::Int(n));
        }
        match text.parse::<f64>() {
            Ok(v) if v.is_finite() => Ok(Value::Float(read_float(v, &text))),
            Ok(_) => self.fail(&format!("{text} is too big a number")),
            Err(_) => self.fail(&format!("{text} isn't a number")),
        }
    }

    fn string(&mut self) -> Result<String, ParseError> {
        let line = self.line;
        self.next(); // the opening quote
        let mut out = String::new();
        loop {
            match self.next() {
                Some('"') => return Ok(out),
                Some('\\') => match self.next() {
                    Some('\\') => out.push('\\'),
                    Some('"') => out.push('"'),
                    Some('n') => out.push('\n'),
                    Some('t') => out.push('\t'),
                    Some('r') => out.push('\r'),
                    Some('u') => {
                        self.expect('{')?;
                        let mut hex = String::new();
                        while let Some(c) = self.peek().filter(char::is_ascii_hexdigit) {
                            if hex.len() >= 6 {
                                break;
                            }
                            hex.push(c);
                            self.next();
                        }
                        self.expect('}')?;
                        let c = u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32);
                        match c {
                            Some(c) => out.push(c),
                            None => return self.fail("a \\u{…} that isn't a character"),
                        }
                    }
                    Some(c) => return self.fail(&format!("\\{c} isn't a known escape")),
                    None => break,
                },
                Some('\n') => {
                    return Err(ParseError {
                        line,
                        message: "text runs past the end of its line (a \" is missing)".into(),
                    });
                }
                Some(c) => out.push(c),
                None => break,
            }
        }
        Err(ParseError {
            line,
            message: "text isn't closed with \"".into(),
        })
    }
}

/// A number read from `text`: if the text is exactly how an `f32` is
/// written, it's that `f32` (so every value Solis writes reads back to the
/// same bits); otherwise the nearest `f64`.
fn read_float(v: f64, text: &str) -> f64 {
    let single = v as f32;
    if single.is_finite() && single_text(single) == text.trim_start_matches('+') {
        f64::from(single)
    } else {
        v
    }
}

/// `value` as the kind a field declares, where the file's text can't say:
/// asset and entity references are written as text, and a decimal field
/// may be written as a whole number by hand. Anything that doesn't fit is
/// kept as it is.
fn coerce(value: Value, kind: &FieldKind) -> Value {
    match (kind, value) {
        (FieldKind::Float, Value::Int(n)) => Value::Float(n as f64),
        (FieldKind::Asset, Value::Text(t)) if t.is_empty() => Value::Asset(None),
        (FieldKind::Asset, Value::Text(t)) => {
            let (id, part) = match t.split_once('#') {
                Some((id, part)) => (id, part.parse().ok()),
                None => (t.as_str(), None),
            };
            match id.parse() {
                Ok(asset) => Value::Asset(Some(AssetRef { asset, part })),
                Err(_) => Value::Text(t),
            }
        }
        (FieldKind::Entity, Value::Text(t)) if t.is_empty() => Value::Entity(None),
        (FieldKind::Entity, Value::Text(t)) => match t.parse() {
            Ok(id) => Value::Entity(Some(id)),
            Err(_) => Value::Text(t),
        },
        (FieldKind::Vec3 | FieldKind::Rotation | FieldKind::Color, Value::List(items)) => {
            Value::List(
                items
                    .into_iter()
                    .map(|v| coerce(v, &FieldKind::Float))
                    .collect(),
            )
        }
        (_, value) => value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtin::{self, Light, Model, Transform};

    fn registry() -> Registry {
        builtin::registry().unwrap()
    }

    fn no_paths(_: AssetId) -> Option<String> {
        None
    }

    fn sample() -> Area {
        let mut area = builtin::starter_area();
        let asset = AssetId::random();
        let model = area.spawn(
            EntityData::new("Crate \"big\"\n2")
                .with(&Transform {
                    position: glam::Vec3::new(1.5, -2.25, 1e-7),
                    ..Transform::default()
                })
                .with(&Model {
                    asset: Some(AssetRef {
                        asset,
                        part: Some(3),
                    }),
                }),
        );
        let mut lamp = EntityData::new("Lamp").with(&Light::point());
        lamp.parent = Some(model);
        lamp.order = "a1".into();
        let lamp = area.spawn(lamp);
        // A component nobody registered, with every kind of value.
        area.get_mut(lamp).unwrap().components.insert(
            "Health".into(),
            ComponentData {
                fields: BTreeMap::from([
                    ("max_hp".into(), Value::Int(-4000)),
                    ("regen".into(), Value::Float(0.1)),
                    ("team".into(), Value::Text("Red \\ Blue\ttab".into())),
                    (
                        "path".into(),
                        Value::List(vec![
                            Value::List(vec![Value::Int(1), Value::Float(2.5)]),
                            Value::Bool(false),
                        ]),
                    ),
                ]),
            },
        );
        area.get_mut(lamp)
            .unwrap()
            .components
            .insert("Marker".into(), ComponentData::default());
        area
    }

    #[test]
    fn an_area_round_trips_through_text() {
        let area = sample();
        let text = write(&area, &registry(), &no_paths);
        let back = read(&text, &registry()).unwrap();
        assert_eq!(back, area, "{text}");
        // And writing it again gives the same text.
        assert_eq!(write(&back, &registry(), &no_paths), text);
    }

    #[test]
    fn the_text_reads_as_intended() {
        let mut area = Area::new();
        let id: EntityId = "2B8D4NE6TV0HC".parse().unwrap();
        let asset: AssetId = "4WQ0B7M2KD9TS".parse().unwrap();
        area.insert(
            id,
            EntityData::new("Sun")
                .with(&Light {
                    color: [1.0, 0.95, 0.88],
                    ..Light::sun()
                })
                .with(&Model {
                    asset: Some(AssetRef { asset, part: None }),
                }),
        );
        let text = write(&area, &registry(), &|_| Some("props/sun.glb".into()));
        assert!(text.contains(
            "area version 1;\n\nentity 2B8D4NE6TV0HC {\n    name = \"Sun\";\n    order = \"a0\";\n"
        ));
        assert!(text.contains("    Light.color = [1.0, 0.95, 0.88];\n"));
        assert!(text.contains("    Light.kind = \"Sun\";\n"));
        assert!(text.contains("    Model.asset = \"4WQ0B7M2KD9TS\";  -- props/sun.glb\n"));
        // Fields come in their declared order: kind before colour.
        assert!(text.find("Light.kind").unwrap() < text.find("Light.color").unwrap());
        assert!(text.ends_with("}\n") && !text.contains('\r'));
    }

    #[test]
    fn hand_edits_windows_line_ends_and_comments_are_read() {
        let text = "\u{feff}-- my level\r\narea version 1;\r\n\r\nentity 0000000000001 { -- the box\r\n  name = \"Box\";\r\n  Transform.position = [1, 2, 3];  -- whole numbers\r\n  Light.strength = 5;\r\n}\r\n";
        let area = read(text, &registry()).unwrap();
        let e = area.get("0000000000001".parse().unwrap()).unwrap();
        assert_eq!(e.name, "Box");
        assert_eq!(
            e.get::<Transform>().unwrap().position,
            glam::Vec3::new(1.0, 2.0, 3.0)
        );
        assert_eq!(
            e.components["Light"].get("strength"),
            Some(&Value::Float(5.0))
        );
    }

    #[test]
    fn mistakes_are_reported_with_their_line() {
        let cases = [
            ("", 1, "empty"),
            ("scene version 1;", 1, "isn't an area file"),
            ("area version 2;", 1, "newer version"),
            ("area version 0;", 1, "no layout version 0"),
            ("area version 1;\nentity X {}", 2, "isn't an ID"),
            (
                "area version 1;\nentity 0000000000001 {\n  name = \"A\";\n",
                4,
                "isn't closed",
            ),
            (
                "area version 1;\nentity 0000000000001 {\n  name = 5;\n}",
                3,
                "name should be text",
            ),
            (
                "area version 1;\nentity 0000000000001 {\n  Light.strength = 1;\n  Light.strength = 2;\n}",
                4,
                "twice",
            ),
            (
                "area version 1;\nentity 0000000000001 {\n  colour = \"red;\n}",
                3,
                "past the end",
            ),
            (
                "area version 1;\nentity 0000000000001 {\n  Light.on = yes;\n}",
                3,
                "isn't a value",
            ),
            (
                "area version 1;\nentity 0000000000001 {\n  Light.strength = 1e999;\n}",
                3,
                "too big",
            ),
            (
                "area version 1;\nentity 0000000000001 { name = \"A\"; }\nentity 0000000000001 { name = \"B\"; }",
                3,
                "used twice: by \"A\" (line 2) and \"B\"",
            ),
            (
                "area version 1;\nentity 0000000000001/0000000000002 {}",
                2,
                "assemblies",
            ),
        ];
        for (text, line, words) in cases {
            let err = read(text, &registry()).unwrap_err();
            assert_eq!(err.line, line, "{text:?}: {err}");
            assert!(err.message.contains(words), "{text:?}: {err}");
        }
    }

    #[test]
    fn limits_are_checked_before_reading() {
        let long = format!("area version 1;\n-- {}\n", "x".repeat(MAX_LINE + 1));
        assert_eq!(read(&long, &registry()).unwrap_err().line, 2);
        let deep = format!(
            "area version 1;\nentity 0000000000001 {{ A.b = {}; }}",
            "[".repeat(100)
        );
        assert!(
            read(&deep, &registry())
                .unwrap_err()
                .message
                .contains("deeply")
        );
        let many = format!(
            "area version 1;\nentity 0000000000001 {{ {} }}",
            (0..=MAX_FIELDS)
                .map(|i| format!("A.f{i} = 1;"))
                .collect::<String>()
        );
        assert!(
            read(&many, &registry())
                .unwrap_err()
                .message
                .contains("fields")
        );
    }

    #[test]
    fn rubbish_never_crashes_the_reader() {
        // Fuzzing: every prefix and thousands of corruptions of a real
        // file must be read or refused, never crash.
        let text = write(&sample(), &registry(), &no_paths);
        let registry = registry();
        for end in 0..text.len() {
            if text.is_char_boundary(end) {
                let _ = read(&text[..end], &registry);
            }
        }
        let bytes = text.as_bytes();
        let mut seed = 0xDEAD_BEEFu64;
        for _ in 0..5000 {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let mut broken = bytes.to_vec();
            for _ in 0..1 + seed % 3 {
                let at = (seed as usize / 7) % broken.len();
                let pieces = b"{}[]\";=-.,/#\\ \nax0e";
                broken[at] = pieces[(seed >> 20) as usize % pieces.len()];
                seed = seed.rotate_left(11);
            }
            if let Ok(text) = std::str::from_utf8(&broken) {
                let _ = read(text, &registry);
            }
        }
    }

    #[test]
    fn floats_read_back_to_the_same_bits() {
        // A sample of f32 bit patterns, spread over the whole range; the
        // complete check runs with the long tests.
        let mut bits = 1u32;
        for _ in 0..200_000 {
            bits = bits.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            check_f32(f32::from_bits(bits));
        }
        for v in [
            0.0f32,
            -0.0,
            1.0,
            0.1,
            f32::MIN_POSITIVE,
            f32::MAX,
            f32::MIN,
            1e-45,
        ] {
            check_f32(v);
        }
        // Numbers that aren't f32s keep their f64 value.
        for v in [0.1f64, 1.0 / 3.0, 1e300] {
            let text = float_text(v);
            assert_eq!(read_float(text.parse().unwrap(), &text), v, "{text}");
        }
    }

    fn check_f32(v: f32) {
        if !v.is_finite() {
            return;
        }
        let text = float_text(f64::from(v));
        let back = read_float(text.parse().unwrap(), &text);
        assert_eq!((back as f32).to_bits(), v.to_bits(), "{text}");
        assert_eq!(back, f64::from(v), "{text}");
    }

    #[test]
    #[ignore = "long: every f32 bit pattern; run by hand or at the phase gate"]
    fn every_f32_reads_back_exactly() {
        for bits in 0..=u32::MAX {
            check_f32(f32::from_bits(bits));
        }
    }
}
