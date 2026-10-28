//! A tileset (`.soltileset`, Phase 9, D4): the terrains a tile map's cells
//! can be (grass, tilled, water, fence), each with the one-character code
//! a map file writes for it and, until tileset art arrives (D5), a plain
//! colour to draw it in. A settings file in the Lust style with a block
//! per terrain, by a stable random ID, so two people's changes merge:
//!
//! ```text
//! -- A tileset. Written by Solis; safe to edit by hand.
//! tileset version 1;
//!
//! terrain 5RT0M2C8K1V4D {
//!     name = "grass";
//!     code = ".";
//!     colour = [0.31, 0.48, 0.23];
//! }
//! ```
//!
//! The first terrain is what a new map's ground is filled with. Read as a
//! hostile file: within limits, a terrain it can't use skipped with a
//! note.

use area::settings::{self, BlockOut, Field};
use reflect::{Id, Value};
use std::fmt;

/// The layout version this build writes and reads.
pub const VERSION: u32 = 1;
const KIND: &str = "tileset";
/// The file's extension.
pub const EXTENSION: &str = "soltileset";
/// The biggest tileset file read.
pub const MAX_BYTES: usize = 256 << 10;
/// The most terrains a tileset has.
pub const MAX_TERRAINS: usize = 64;
/// The longest terrain name, in characters.
pub const MAX_NAME: usize = 32;

/// This file's version and upgraders (Phase 8, C1).
pub static FORMAT: reflect::upgrade::Format<settings::BlockSettings> = reflect::upgrade::Format {
    name: "tileset",
    current: VERSION,
    version_of: |text| reflect::upgrade::header_version(text, &[KIND]),
    steps: &[],
};

/// What a cell can be.
#[derive(Debug, Clone, PartialEq)]
pub struct Terrain {
    /// Its ID, which never changes.
    pub id: Id,
    /// Its name: letters, digits and `_` (game code will name it as a
    /// state).
    pub name: String,
    /// The character a map file writes for it.
    pub code: char,
    /// Its colour (sRGB, 0 to 1), drawn until the tileset has art.
    pub colour: [f32; 3],
}

/// A tileset.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Tileset {
    /// Its terrains, in order; the first fills a new map.
    pub terrains: Vec<Terrain>,
}

impl Tileset {
    /// The terrain written with `code`.
    pub fn by_code(&self, code: char) -> Option<&Terrain> {
        self.terrains.iter().find(|t| t.code == code)
    }

    /// The terrain called `name`.
    pub fn by_name(&self, name: &str) -> Option<&Terrain> {
        self.terrains.iter().find(|t| t.name == name)
    }

    /// A code no terrain uses yet (from a short list of easy ones).
    pub fn free_code(&self) -> Option<char> {
        ".tw#~,:;=+*o%&@abcdefghijklmnopqrsuvxyz0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ"
            .chars()
            .find(|c| self.by_code(*c).is_none())
    }
}

/// Whether `c` can be a terrain's code: printable, not a space, quote or
/// backslash (so a map's rows stay easy to read and write).
pub fn usable_code(c: char) -> bool {
    c.is_ascii_graphic() && c != '"' && c != '\\'
}

/// A name for a terrain, layer or value, checked: letters, digits and `_`,
/// at most [`MAX_NAME`] characters.
pub fn name_of(text: &str) -> Result<String, String> {
    let t = text.trim();
    if t.is_empty() {
        return Err("a name is needed".to_owned());
    }
    if t.chars().count() > MAX_NAME || !t.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(format!(
            "\"{t}\" isn't a name (letters, digits and _, at most {MAX_NAME})"
        ));
    }
    Ok(t.to_owned())
}

/// Why a tileset couldn't be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TilesetError {
    /// The line it went wrong on (from 1), or 0 for the whole file.
    pub line: usize,
    /// What went wrong, in plain words.
    pub message: String,
}

impl fmt::Display for TilesetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line == 0 {
            f.write_str(&self.message)
        } else {
            write!(f, "line {}: {}", self.line, self.message)
        }
    }
}

impl std::error::Error for TilesetError {}

/// A tileset as read, with notes about what was put right.
#[derive(Debug, Clone, PartialEq)]
pub struct ReadTileset {
    /// The tileset.
    pub tileset: Tileset,
    /// What was skipped, in plain words with lines.
    pub notes: Vec<String>,
}

/// Whether `path` names a tileset (by its extension).
pub fn is_tileset_file(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case(EXTENSION))
}

fn get<'a>(fields: &'a [Field], key: &str) -> Option<&'a Value> {
    fields.iter().find(|f| f.key == key).map(|f| &f.value)
}

/// Reads a tileset.
pub fn read(text: &str) -> Result<ReadTileset, TilesetError> {
    if text.len() > MAX_BYTES {
        return Err(TilesetError {
            line: 0,
            message: format!(
                "the file is {} KB; tilesets up to {} KB are read",
                text.len() >> 10,
                MAX_BYTES >> 10
            ),
        });
    }
    let mut read =
        settings::read_block_settings(text, KIND, VERSION).map_err(|e| TilesetError {
            line: e.line,
            message: e.message,
        })?;
    FORMAT
        .upgrade(read.version, &mut read)
        .map_err(|e| TilesetError {
            line: 1,
            message: e.to_string(),
        })?;
    let mut notes = Vec::new();
    let mut tileset = Tileset::default();
    for f in &read.fields {
        notes.push(format!(
            "line {}: \"{}\" isn't a tileset setting, so it's skipped",
            f.line, f.key
        ));
    }
    for block in &read.blocks {
        let line = block.line;
        if block.word != "terrain" {
            notes.push(format!(
                "line {line}: \"{}\" isn't a part of a tileset, so it's skipped",
                block.word
            ));
            continue;
        }
        if tileset.terrains.len() == MAX_TERRAINS {
            notes.push(format!(
                "line {line}: more than {MAX_TERRAINS} terrains; the rest are skipped"
            ));
            break;
        }
        match terrain(&tileset, &block.name, &block.fields) {
            Ok(t) => tileset.terrains.push(t),
            Err(why) => notes.push(format!("line {line}: {why}, so this terrain is skipped")),
        }
        for f in &block.fields {
            if !matches!(f.key.as_str(), "name" | "code" | "colour") {
                notes.push(format!(
                    "line {}: \"{}\" isn't a terrain setting, so it's skipped",
                    f.line, f.key
                ));
            }
        }
    }
    Ok(ReadTileset { tileset, notes })
}

fn terrain(set: &Tileset, id: &str, fields: &[Field]) -> Result<Terrain, String> {
    let id: Id = id.parse().map_err(|_| format!("\"{id}\" isn't an ID"))?;
    if set.terrains.iter().any(|t| t.id == id) {
        return Err("a second terrain with this ID".to_owned());
    }
    let name = match get(fields, "name") {
        Some(Value::Text(t)) => name_of(t)?,
        _ => return Err("a terrain needs a name in quotes".to_owned()),
    };
    if set.by_name(&name).is_some() {
        return Err(format!("a second terrain called \"{name}\""));
    }
    let code = match get(fields, "code") {
        Some(Value::Text(t)) => {
            let mut chars = t.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) if usable_code(c) => c,
                _ => {
                    return Err(format!(
                        "\"{t}\" isn't a code (one printable character, not a space, quote or \
                         backslash)"
                    ));
                }
            }
        }
        _ => return Err("a terrain needs a code in quotes".to_owned()),
    };
    if set.by_code(code).is_some() {
        return Err(format!("a second terrain written \"{code}\""));
    }
    let colour = match get(fields, "colour") {
        None => [0.5; 3],
        Some(v) => v
            .as_floats::<3>()
            .filter(|c| c.iter().all(|v| v.is_finite()))
            .map(|c| c.map(|v| v.clamp(0.0, 1.0)))
            .ok_or("its colour should be three numbers from 0 to 1")?,
    };
    Ok(Terrain {
        id,
        name,
        code,
        colour,
    })
}

/// Writes a tileset that [`read`] reads back.
pub fn write(tileset: &Tileset) -> String {
    let blocks: Vec<BlockOut<'_>> = tileset
        .terrains
        .iter()
        .map(|t| {
            (
                "terrain",
                t.id.to_string(),
                vec![
                    ("name", Value::Text(t.name.clone())),
                    ("code", Value::Text(t.code.to_string())),
                    ("colour", Value::floats(&t.colour)),
                ],
            )
        })
        .collect();
    settings::write_block_settings(
        KIND,
        VERSION,
        "A tileset. Written by Solis; safe to edit by hand.\n\
         Each terrain is something a tile map's cell can be: its name, the one character\n\
         a map file writes for it, and its colour. The first fills a new map.",
        &[],
        &blocks,
    )
}

#[cfg(test)]
#[path = "tileset_tests.rs"]
pub(crate) mod tests;
