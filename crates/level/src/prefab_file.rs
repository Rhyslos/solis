//! A prefab's file: `chicken.prefab.lst`, a Lust data file
//! ([decision 0081](../../../docs/decisions/0081-what-solis-makes-is-lust.md),
//! [decision 0118](../../../docs/decisions/0118-the-editor-rethink.md)),
//! version 1. Each entity of the prefab is a `Part`, and so is each change
//! a copy placed inside it makes to one of that copy's parts:
//!
//! ```text
//! -- A prefab. Written by Solis; safe to edit by hand: a part per entity, a line per setting.
//! fetch "solis/data/prefab":[Part];
//!
//! lock int version = 1;
//! lock string root = "0000000000001";
//! lock Part array parts = [
//!     new Part {
//!         id = "0000000000001",
//!         name = "Lamp post",
//!         order = "a0",
//!         settings = ["Transform.position = [0.0, 0.0, 0.0]"]
//!     },
//!     new Part {
//!         id = "0000000000003",
//!         name = "Crate",
//!         parent = "0000000000001",
//!         order = "a1",
//!         prefab = "1BWHNR3RV8PYY"
//!     },
//!     new Part { id = "0000000000003/0000000000002", removed = true },
//! ];
//! end
//! ```
//!
//! A part's `settings` are its components' fields, each written as a
//! level writes it (`Light.strength = 3.0`): a component's fields can be
//! of any kind, and a Lust class's field has one, so the value is kept in
//! the line's own words. A field left out says nothing: a change's
//! `parent = ""` moves the part out to the top level, while no `parent`
//! leaves it where its prefab has it ([`ParentChange`]).
//!
//! Reading turns the parts into the blocks a level's file has and reads
//! those with [`crate::text::read_file`], so one set of checks makes the
//! prefab from either layout. The older layout, an assembly's
//! `.solassembly` file (`assembly version 4 root ID;`), is still read
//! through the level format's own steps, until the app has moved it
//! (`app::older_files`).

use crate::document::Level;
use crate::lust_data::{self as data, DataError};
use crate::text::{self, FileKind, LevelFile, Override, ParentChange, ParseError};
use reflect::{EntityId, Registry};
use std::collections::BTreeMap;
use std::fmt::Write as _;

/// The word before `.lst` in a prefab's file name.
pub const KIND: &str = "prefab";

/// The end of a prefab's file name.
pub const ENDING: &str = ".prefab.lst";

/// The extension prefabs had while they were called assemblies, before
/// version 1: such a file is still read, and moved when its project opens.
pub const OLD_EXTENSION: &str = "solassembly";

/// The version this build writes. Version 1 is the first in Lust; the
/// older layout (an assembly's file) has its own versions, the level
/// format's, and is read through those.
pub const VERSION: u32 = 1;

/// The most bytes a prefab file may have: a level's most.
pub const MAX_BYTES: usize = text::MAX_BYTES;

/// A setting longer than this is broken over lines (after a comma) when
/// it's read, so a brush's long lists stay within a level line's most.
const LINE_BREAK_AFTER: usize = 4096;

/// What a prefab file holds, in Lust: written into every project's
/// `solis/data/prefab.lst`, and handed to Lust when a prefab is read.
pub const CLASSES: &str = "-- What a prefab file holds (Solis writes this; prefabs fetch it).\n\
class Part {\n    has {\n        string id = \"\";\n        string name = \"\";\n        \
string parent = \"\";\n        string order = \"\";\n        string prefab = \"\";\n        \
bool removed = false;\n        string array remove = [];\n        string array settings = [];\n    \
}\n}\n\npass Part;\nend\n";

/// Prefabs' version and upgraders. Version 1 is the first; an older
/// assembly's file isn't this format, and is read as the level format.
pub static FORMAT: reflect::upgrade::Format<LevelFile> = reflect::upgrade::Format {
    name: "prefab",
    current: VERSION,
    version_of: data::version_of,
    steps: &[],
};

const COMMENT: &str =
    "A prefab. Written by Solis; safe to edit by hand: a part per entity, a line per setting.";

/// Whether `text` is a prefab's older layout (an assembly's file), which
/// [`read`] still reads and the app moves to a `.prefab.lst`.
pub fn is_old_layout(text: &str) -> bool {
    !data::is_data(text)
}

/// `file` (a prefab's contents, as [`crate::prefab::Expander::collapse`]
/// makes them) as a prefab file. Refuses a level, and contents that
/// would be too big to read back.
pub fn write(file: &LevelFile, registry: &Registry) -> Result<String, ParseError> {
    let FileKind::Prefab { root } = file.kind else {
        return Err(whole("a level can't be written as a prefab"));
    };
    unseen_characters(file)?;
    let mut parts: Vec<(String, String)> = Vec::new();
    for (id, entity) in file.level.iter() {
        let mut fields = vec![("name", data::text(&entity.name))];
        if let Some(parent) = entity.parent {
            fields.push(("parent", data::text(&parent.to_string())));
        }
        fields.push(("order", data::text(&entity.order)));
        let settings = match &entity.origin {
            Some(origin) if origin.path.is_empty() => {
                fields.push(("prefab", data::text(&origin.prefab.to_string())));
                match file.overrides.get(&(id, Vec::new())) {
                    Some(changes) => {
                        push_remove(&mut fields, &changes.remove);
                        text::settings_of(&changes.components, registry)
                    }
                    None => Vec::new(),
                }
            }
            _ => text::settings_of(&entity.components, registry),
        };
        let key = id.to_string();
        parts.push((key.clone(), part(&key, fields, &settings)));
    }
    for ((copy, path), changes) in &file.overrides {
        if path.is_empty() {
            continue;
        }
        let mut key = copy.to_string();
        for step in path {
            let _ = write!(key, "/{step}");
        }
        let fields = change_fields(changes);
        let settings = text::settings_of(&changes.components, registry);
        parts.push((key.clone(), part(&key, fields, &settings)));
    }
    parts.sort_by(|a, b| a.0.cmp(&b.0));
    let mut out = data::header(COMMENT, KIND, &["Part"], VERSION);
    let _ = writeln!(out, "lock string root = {};", data::text(&root.to_string()));
    if parts.is_empty() {
        out.push_str("lock Part array parts = [];\n");
    } else {
        out.push_str("lock Part array parts = [\n");
        for (_, written) in parts {
            out.push_str(&written);
        }
        out.push_str("];\n");
    }
    out.push_str("end\n");
    data::fits(&out, MAX_BYTES).map_err(from_data)?;
    Ok(out)
}

/// Refuses names, orders and components holding a character that can't
/// be seen (a control character other than a line break or a tab): a
/// data file can't keep one, so the prefab wouldn't read back as itself.
fn unseen_characters(file: &LevelFile) -> Result<(), ParseError> {
    let unseen = |text: &str| {
        text.chars()
            .any(|c| c.is_control() && !matches!(c, '\n' | '\t'))
    };
    let entities = file.level.iter().flat_map(|(_, e)| {
        [e.name.as_str(), e.order.as_str()]
            .into_iter()
            .chain(e.components.keys().map(String::as_str))
    });
    let changes = file.overrides.values().flat_map(|c| {
        c.name
            .iter()
            .chain(&c.order)
            .chain(&c.remove)
            .chain(c.components.keys())
            .map(String::as_str)
    });
    match entities.chain(changes).find(|t| unseen(t)) {
        Some(text) => Err(whole(format!(
            "\"{}\" holds a character that can't be seen, which a prefab's file can't keep: \
             type it again without it",
            shown(&text.replace(|c: char| c.is_control(), "?"))
        ))),
        None => Ok(()),
    }
}

/// A change's own fields, as its level block would say them.
fn change_fields(changes: &Override) -> Vec<(&'static str, String)> {
    let mut fields = Vec::new();
    if let Some(name) = &changes.name {
        fields.push(("name", data::text(name)));
    }
    match changes.parent {
        ParentChange::Unchanged => {}
        ParentChange::TopLevel => fields.push(("parent", data::text(""))),
        ParentChange::Under(parent) => fields.push(("parent", data::text(&parent.to_string()))),
    }
    if let Some(order) = &changes.order {
        fields.push(("order", data::text(order)));
    }
    if changes.removed {
        fields.push(("removed", "true".into()));
    }
    push_remove(&mut fields, &changes.remove);
    fields
}

fn push_remove(fields: &mut Vec<(&'static str, String)>, remove: &[String]) {
    if !remove.is_empty() {
        fields.push(("remove", data::texts(remove.iter().map(String::as_str))));
    }
}

/// One part, as its lines in the list: on one line when it has no
/// settings, otherwise a field a line and a setting a line, so a change to
/// one setting changes one line.
fn part(id: &str, fields: Vec<(&str, String)>, settings: &[String]) -> String {
    let mut all = vec![("id", data::text(id))];
    all.extend(fields);
    if settings.is_empty() {
        return format!("    {},\n", data::thing("Part", &all));
    }
    let mut out = String::from("    new Part {\n");
    for (key, value) in &all {
        let _ = writeln!(out, "        {key} = {value},");
    }
    out.push_str("        settings = [\n");
    for (i, setting) in settings.iter().enumerate() {
        let comma = if i + 1 < settings.len() { "," } else { "" };
        let _ = writeln!(out, "            {}{comma}", data::text(setting));
    }
    out.push_str("        ]\n    },\n");
    out
}

/// Reads a prefab file: a `.prefab.lst`, or an assembly's older file.
/// Every mistake comes back with its line (a part's, the line its list
/// starts on), never a crash; a newer version is refused.
pub fn read(text: &str, registry: &Registry) -> Result<LevelFile, ParseError> {
    if is_old_layout(text) {
        let file = text::read_file(text, registry)?;
        return match file.kind {
            FileKind::Prefab { .. } => Ok(file),
            FileKind::Level => Err(whole("it's a level, not a prefab")),
        };
    }
    let version = data::version_of(text).unwrap_or(0);
    if version > VERSION {
        return Err(whole(format!(
            "it was saved by a newer version (prefab {version}; this one reads up to \
             {VERSION}). Update to open it."
        )));
    }
    let values = data::read(text, KIND, CLASSES, MAX_BYTES).map_err(from_data)?;
    let root = data::get(&values, "root").map_err(from_data)?;
    let root = root.as_text().unwrap_or_default();
    if !is_id(root, false) {
        return Err(whole(format!(
            "its root \"{}\" isn't an entity's ID",
            shown(root)
        )));
    }
    let line = values
        .values
        .iter()
        .find(|v| v.name == "parts")
        .map_or(0, |v| v.line as usize);
    let parts = data::things(&values, "parts");
    if parts.len() > text::MAX_ENTITIES.saturating_mul(2) {
        return Err(at(line, "it has more parts than a prefab may"));
    }
    let mut blocks = format!(
        "{} version {} root {root};\n",
        text::PREFAB_WORD,
        text::VERSION
    );
    let mut starts: Vec<(usize, String)> = Vec::with_capacity(parts.len());
    let mut lines = 2;
    for thing in parts {
        let id = data::field_text(thing, "id");
        starts.push((lines, id.to_string()));
        let block =
            block_of(thing).map_err(|why| at(line, format!("the part {}: {why}", shown(id))))?;
        lines += block.matches('\n').count();
        blocks.push_str(&block);
    }
    let mut file = text::read_file(&blocks, registry).map_err(|e| {
        let part = starts
            .iter()
            .rev()
            .find(|(start, _)| *start <= e.line)
            .map(|(_, id)| id.as_str());
        match part {
            Some(id) => at(line, format!("the part {}: {}", shown(id), e.message)),
            None => at(line, e.message),
        }
    })?;
    FORMAT
        .upgrade(version, &mut file)
        .map_err(|e| whole(e.to_string()))?;
    Ok(file)
}

/// One part as a level file's block, checking that what goes into it is
/// what the part says and no more: IDs are IDs, and each setting is one
/// field's line, so none can end its block early.
fn block_of(thing: &data::Fixed) -> Result<String, String> {
    let id = data::field_text(thing, "id");
    if !is_id(id, true) {
        return Err("its id isn't an entity's ID, or a copy's and its part's".into());
    }
    let mut out = format!("entity {id} {{\n");
    if let Some(name) = thing.field("name").and_then(data::Fixed::as_text) {
        let _ = writeln!(out, "    name = {};", text::quoted(name));
    }
    for key in ["parent", "prefab"] {
        if let Some(value) = thing.field(key).and_then(data::Fixed::as_text) {
            let empty_is_top = key == "parent";
            if !(is_id(value, false) || empty_is_top && value.is_empty()) {
                return Err(format!("its {key} \"{}\" isn't an ID", shown(value)));
            }
            let _ = writeln!(out, "    {key} = \"{value}\";");
        }
    }
    if let Some(order) = thing.field("order").and_then(data::Fixed::as_text) {
        let _ = writeln!(out, "    order = {};", text::quoted(order));
    }
    if thing.field("removed").and_then(data::Fixed::as_bool) == Some(true) {
        out.push_str("    removed = true;\n");
    }
    if thing.field("remove").is_some() {
        let remove: Vec<String> = data::field_texts(thing, "remove")
            .into_iter()
            .map(text::quoted)
            .collect();
        let _ = writeln!(out, "    remove = [{}];", remove.join(", "));
    }
    for setting in data::field_texts(thing, "settings") {
        let line = one_setting(setting)?;
        let _ = writeln!(out, "    {line};");
    }
    out.push_str("}\n");
    Ok(out)
}

/// Whether `id` is an entity's ID (letters and digits), or with `path`,
/// a copy's root and the way to one of its parts (`root/part`).
fn is_id(id: &str, path: bool) -> bool {
    !id.is_empty()
        && id.len() <= 4096
        && id.split('/').count() <= text::MAX_NESTING + 1
        && (path || !id.contains('/'))
        && id
            .split('/')
            .all(|step| !step.is_empty() && step.chars().all(|c| c.is_ascii_alphanumeric()))
}

/// A setting checked to be one field's line: a name, then `=`, then a
/// value with no `;`, `{` or `}` and no line break outside its quotes
/// (`Light = {}` is a component with no fields). Long ones are broken over
/// lines after commas.
fn one_setting(setting: &str) -> Result<String, String> {
    let bad = || format!("the setting \"{}\" isn't one field's line", shown(setting));
    let starts_well = setting
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_');
    if !starts_well || !setting.contains('=') {
        return Err(bad());
    }
    let (body, empty) = match setting.strip_suffix("= {}") {
        Some(name) => (name, true),
        None => (setting, false),
    };
    let mut out = String::with_capacity(setting.len() + 16);
    let (mut quoted, mut escaped, mut since_break) = (false, false, 0usize);
    for c in body.chars() {
        if matches!(c, '\n' | '\r') {
            return Err(bad());
        }
        out.push(c);
        since_break += 1;
        if quoted {
            match (escaped, c) {
                (true, _) => escaped = false,
                (false, '\\') => escaped = true,
                (false, '"') => quoted = false,
                _ => {}
            }
            continue;
        }
        match c {
            '"' => quoted = true,
            ';' | '{' | '}' => return Err(bad()),
            ',' if since_break > LINE_BREAK_AFTER => {
                out.push_str("\n        ");
                since_break = 0;
            }
            _ => {}
        }
    }
    if quoted {
        return Err(bad());
    }
    if empty {
        out.push_str("= {}");
    }
    Ok(out)
}

/// The start of `text`, to say which one is meant.
fn shown(text: &str) -> String {
    let start: String = text.chars().take(40).collect();
    if start.len() < text.len() {
        format!("{start}…")
    } else {
        start
    }
}

fn whole(message: impl Into<String>) -> ParseError {
    at(0, message)
}

fn at(line: usize, message: impl Into<String>) -> ParseError {
    ParseError {
        line,
        message: message.into(),
    }
}

fn from_data(e: DataError) -> ParseError {
    ParseError {
        line: e.line,
        message: e.message,
    }
}

/// An empty prefab: one entity, `name`, at the top, which is its root
/// (New prefab in the Library).
pub fn empty(name: &str) -> LevelFile {
    let root = EntityId::random();
    let mut level = Level::new();
    level.insert(
        root,
        crate::document::EntityData::new(name).with(&crate::builtin::Transform::default()),
    );
    LevelFile {
        kind: FileKind::Prefab { root },
        level,
        overrides: BTreeMap::new(),
    }
}

#[cfg(test)]
#[path = "prefab_file_tests.rs"]
mod tests;
