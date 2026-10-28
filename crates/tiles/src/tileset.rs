//! A tileset (`.soltileset`, Phase 9, D4 and D5): the terrains a tile
//! map's cells can be (grass, tilled, water, fence), each with the
//! one-character code a map file writes for it, a colour, and how its art
//! is chosen from its neighbours (its kind); the picture the art comes
//! from (a PNG or an Aseprite file, cut into tiles of one size); which of
//! the picture's tiles fills each of a terrain's slots (its pieces, more
//! than one to a slot for variations, some animated); and, for a terrain
//! of its own rules, those rules. A settings file in the Lust style with a
//! block per terrain, piece and rule, each by a stable random ID, so two
//! people's changes merge:
//!
//! ```text
//! -- A tileset. Written by Solis; safe to edit by hand.
//! tileset version 2;
//!
//! picture = "6QK3C0F1M8E2Z";
//! tile = [16, 16];
//!
//! terrain 5RT0M2C8K1V4D {
//!     name = "tilled";
//!     code = "t";
//!     colour = [0.42, 0.29, 0.18];
//!     kind = "16-tile";
//! }
//!
//! piece 2H7W1Q0R3S4T5 {
//!     terrain = "5RT0M2C8K1V4D";
//!     slot = 15;
//!     at = [3, 1];
//! }
//! ```
//!
//! The first terrain is what a new map's ground is filled with. Read as a
//! hostile file: within limits, anything it can't use skipped with a note.

use area::settings::{self, BlockOut, Field};
use reflect::{AssetId, Id, Value};
use std::fmt;

/// The layout version this build writes and reads: 2 added the picture,
/// kinds, pieces and rules (D5).
pub const VERSION: u32 = 2;
const KIND: &str = "tileset";
/// The file's extension.
pub const EXTENSION: &str = "soltileset";
/// The biggest tileset file read.
pub const MAX_BYTES: usize = 1 << 20;
/// The most terrains a tileset has.
pub const MAX_TERRAINS: usize = 64;
/// The most pieces a tileset has.
pub const MAX_PIECES: usize = 2048;
/// The most rules a tileset has.
pub const MAX_RULES: usize = 256;
/// The longest terrain name, in characters.
pub const MAX_NAME: usize = 32;
/// The biggest tile, in pixels each way.
pub const MAX_TILE: u32 = 256;
/// The furthest column or row of the picture a piece can be at.
pub const MAX_AT: u32 = 4095;
/// The most frames an animated piece has.
pub const MAX_FRAMES: u32 = 64;
/// The shortest and longest time a frame of an animated piece shows, in
/// milliseconds.
pub const FRAME_MS: (u32, u32) = (10, 10_000);
/// The heaviest a variation's weight can be.
pub const MAX_WEIGHT: f32 = 1000.0;
/// The biggest rule pattern, in cells each way (odd: the cell is in the
/// middle).
pub const MAX_PATTERN: usize = 9;

/// This file's version and upgraders (Phase 8, C1).
pub static FORMAT: reflect::upgrade::Format<settings::BlockSettings> = reflect::upgrade::Format {
    name: "tileset",
    current: VERSION,
    version_of: |text| reflect::upgrade::header_version(text, &[KIND]),
    steps: &[art_came_later],
};

/// Version 1 to 2: nothing to change. Version 1 had no art; every terrain
/// is plain, which is what a terrain without a kind reads as.
fn art_came_later(_: &mut settings::BlockSettings) -> Result<(), String> {
    Ok(())
}

/// How a terrain's art is chosen from its neighbours.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum TileKind {
    /// One piece (or its variations), whatever its neighbours: grass.
    #[default]
    Plain,
    /// 16-tile, on the dual grid: art drawn half a tile off the cells, each
    /// piece chosen by the four cells at its corners. The fewest drawings
    /// for smooth edges.
    Dual16,
    /// 47-tile ("blob"): each cell's piece chosen by its eight neighbours,
    /// corners counted only where both sides are the same.
    Blob47,
    /// Edges (fences, walls, roads): each cell's piece chosen by which of
    /// its four sides join one of its own.
    Edges,
    /// The tileset's own rules for this terrain, tried in order.
    Rules,
}

impl TileKind {
    /// Every kind, as menus list them.
    pub const ALL: [Self; 5] = [
        Self::Plain,
        Self::Dual16,
        Self::Blob47,
        Self::Edges,
        Self::Rules,
    ];

    /// The word the file writes, and menus show.
    pub fn word(self) -> &'static str {
        match self {
            Self::Plain => "plain",
            Self::Dual16 => "16-tile",
            Self::Blob47 => "47-tile",
            Self::Edges => "edges",
            Self::Rules => "rules",
        }
    }

    /// The kind written `word`.
    pub fn from_word(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.word() == word)
    }
}

/// How a piece animates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum Animation {
    /// It doesn't.
    #[default]
    None,
    /// `count` tiles of the picture, left to right from the piece's, each
    /// shown for `ms` milliseconds.
    Columns {
        /// How many frames (2 to [`MAX_FRAMES`]).
        count: u32,
        /// How long each shows.
        ms: u32,
    },
    /// The same tile in each of an Aseprite picture's frames, each shown
    /// for its own time.
    Frames,
}

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
    /// Its colour (sRGB, 0 to 1): drawn where it has no art.
    pub colour: [f32; 3],
    /// How its art is chosen.
    pub kind: TileKind,
}

/// One of the picture's tiles filling a slot of a terrain.
#[derive(Debug, Clone, PartialEq)]
pub struct Piece {
    /// Its ID, which never changes.
    pub id: Id,
    /// The terrain it's art for.
    pub terrain: Id,
    /// The slot it fills ([`crate::autotile::slot_count`] for its kind).
    pub slot: u32,
    /// Its tile in the picture: column and row.
    pub at: [u32; 2],
    /// How often it's picked among the pieces of the same slot.
    pub weight: f32,
    /// How it animates.
    pub animation: Animation,
}

/// What a rule asks of a cell around the one being drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cond {
    /// Anything.
    Any,
    /// The same terrain as the cell.
    Same,
    /// Anything but the same terrain.
    Other,
    /// Nothing on this layer (or past the map's edge).
    Empty,
    /// This terrain (by code).
    Terrain(char),
}

impl Cond {
    /// The character a pattern writes for it.
    pub fn char(self) -> char {
        match self {
            Self::Any => '?',
            Self::Same => '=',
            Self::Other => '!',
            Self::Empty => '_',
            Self::Terrain(c) => c,
        }
    }

    /// The condition written `c` (`*`, the cell itself, is `Same`).
    pub fn of(c: char) -> Option<Self> {
        Some(match c {
            '?' => Self::Any,
            '=' | '*' => Self::Same,
            '!' => Self::Other,
            '_' => Self::Empty,
            c if usable_code(c) => Self::Terrain(c),
            _ => return None,
        })
    }
}

/// A rule of a terrain of its own rules: where the cells around one of
/// its cells match `pattern`, the cell shows a piece of `slot`.
#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    /// Its ID, which never changes.
    pub id: Id,
    /// The terrain it's for.
    pub terrain: Id,
    /// How many cells across its pattern is (1, 3, 5, 7 or 9).
    pub size: usize,
    /// Its pattern, row by row from the top left (`size` × `size`, the
    /// middle the cell itself).
    pub pattern: Vec<Cond>,
    /// The slot whose pieces it shows.
    pub slot: u32,
}

impl Rule {
    /// Its pattern's rows as the file writes them (`*` in the middle).
    pub fn rows(&self) -> Vec<String> {
        let mid = self.size / 2;
        (0..self.size)
            .map(|y| {
                (0..self.size)
                    .map(|x| {
                        if x == mid && y == mid {
                            '*'
                        } else {
                            self.pattern[y * self.size + x].char()
                        }
                    })
                    .collect()
            })
            .collect()
    }
}

/// A tileset.
#[derive(Debug, Clone, PartialEq)]
pub struct Tileset {
    /// The picture its art comes from (a PNG or an Aseprite file), if any.
    pub picture: Option<AssetId>,
    /// Its tiles' size in pixels: width and height.
    pub tile: [u32; 2],
    /// Its terrains, in order; the first fills a new map, and later ones
    /// are drawn over earlier ones where their art overlaps.
    pub terrains: Vec<Terrain>,
    /// Which tile fills each terrain's slots.
    pub pieces: Vec<Piece>,
    /// The rules of terrains of their own rules, tried in order.
    pub rules: Vec<Rule>,
}

impl Default for Tileset {
    fn default() -> Self {
        Self {
            picture: None,
            tile: [16, 16],
            terrains: Vec::new(),
            pieces: Vec::new(),
            rules: Vec::new(),
        }
    }
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

    /// The terrain with ID `id`.
    pub fn terrain(&self, id: Id) -> Option<&Terrain> {
        self.terrains.iter().find(|t| t.id == id)
    }

    /// A code no terrain uses yet (from a short list of easy ones).
    pub fn free_code(&self) -> Option<char> {
        ".tw#~,:;+o%&@abcdefghijklmnopqrsuvxyz0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ"
            .chars()
            .find(|c| self.by_code(*c).is_none())
    }

    /// The pieces of `terrain`'s `slot` (its variations), in order.
    pub fn pieces_of(&self, terrain: Id, slot: u32) -> impl Iterator<Item = (usize, &Piece)> {
        self.pieces
            .iter()
            .enumerate()
            .filter(move |(_, p)| p.terrain == terrain && p.slot == slot)
    }

    /// Whether `terrain` has any art at all.
    pub fn has_art(&self, terrain: Id) -> bool {
        self.picture.is_some() && self.pieces.iter().any(|p| p.terrain == terrain)
    }

    /// The rules of `terrain`, in order.
    pub fn rules_of(&self, terrain: Id) -> impl Iterator<Item = &Rule> {
        self.rules.iter().filter(move |r| r.terrain == terrain)
    }
}

/// Whether `c` can be a terrain's code: printable, not a space, quote or
/// backslash (so a map's rows stay easy to read and write), and not one
/// of the characters a rule's pattern uses for itself.
pub fn usable_code(c: char) -> bool {
    c.is_ascii_graphic() && !matches!(c, '"' | '\\' | '?' | '=' | '!' | '_' | '*')
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

/// A whole number from 0 to `most`.
fn whole(v: &Value, most: u32) -> Option<u32> {
    match v {
        Value::Int(i) => u32::try_from(*i).ok().filter(|&i| i <= most),
        _ => None,
    }
}

/// Two whole numbers from `least` to `most`.
fn pair(v: &Value, least: u32, most: u32) -> Option<[u32; 2]> {
    let Value::List(items) = v else {
        return None;
    };
    match items.as_slice() {
        [a, b] => {
            let a = whole(a, most).filter(|&a| a >= least)?;
            let b = whole(b, most).filter(|&b| b >= least)?;
            Some([a, b])
        }
        _ => None,
    }
}

/// Reads a tileset (version 1 or 2).
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
    let mut read = settings::read_block_settings_within(
        text,
        KIND,
        VERSION,
        MAX_TERRAINS + MAX_PIECES + MAX_RULES + 64,
    )
    .map_err(|e| TilesetError {
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
        match (f.key.as_str(), &f.value) {
            ("picture", Value::Text(t)) if t.is_empty() => {}
            ("picture", Value::Text(t)) => match t.parse::<Id>() {
                Ok(id) => tileset.picture = Some(AssetId(id)),
                Err(_) => notes.push(format!(
                    "line {}: \"{t}\" isn't an asset ID, so the tileset has no picture",
                    f.line
                )),
            },
            ("tile", v) => match pair(v, 1, MAX_TILE) {
                Some(t) => tileset.tile = t,
                None => notes.push(format!(
                    "line {}: a tile's size is two whole numbers from 1 to {MAX_TILE}, so it's \
                     16 × 16",
                    f.line
                )),
            },
            _ => notes.push(format!(
                "line {}: \"{}\" isn't a tileset setting, so it's skipped",
                f.line, f.key
            )),
        }
    }
    // Terrains first, wherever they are, so pieces and rules can name them.
    for block in read.blocks.iter().filter(|b| b.word == "terrain") {
        let line = block.line;
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
        unknown(
            &block.fields,
            &["name", "code", "colour", "kind"],
            "terrain",
            &mut notes,
        );
    }
    let mut ids: std::collections::HashSet<Id> = tileset.terrains.iter().map(|t| t.id).collect();
    for block in &read.blocks {
        let line = block.line;
        match block.word.as_str() {
            "terrain" => {}
            "piece" => {
                if tileset.pieces.len() == MAX_PIECES {
                    notes.push(format!(
                        "line {line}: more than {MAX_PIECES} pieces; the rest are skipped"
                    ));
                    continue;
                }
                match piece(&tileset, &block.name, &block.fields, &ids) {
                    Ok(p) => {
                        ids.insert(p.id);
                        tileset.pieces.push(p);
                    }
                    Err(why) => {
                        notes.push(format!("line {line}: {why}, so this piece is skipped"));
                    }
                }
                unknown(
                    &block.fields,
                    &[
                        "terrain",
                        "slot",
                        "at",
                        "weight",
                        "animation",
                        "frames",
                        "ms",
                    ],
                    "piece",
                    &mut notes,
                );
            }
            "rule" => {
                if tileset.rules.len() == MAX_RULES {
                    notes.push(format!(
                        "line {line}: more than {MAX_RULES} rules; the rest are skipped"
                    ));
                    continue;
                }
                match rule(&tileset, &block.name, &block.fields, &ids) {
                    Ok(r) => {
                        ids.insert(r.id);
                        tileset.rules.push(r);
                    }
                    Err(why) => notes.push(format!("line {line}: {why}, so this rule is skipped")),
                }
                unknown(
                    &block.fields,
                    &["terrain", "pattern", "slot"],
                    "rule",
                    &mut notes,
                );
            }
            word => notes.push(format!(
                "line {line}: \"{word}\" isn't a part of a tileset, so it's skipped"
            )),
        }
    }
    Ok(ReadTileset { tileset, notes })
}

fn unknown(fields: &[Field], known: &[&str], what: &str, notes: &mut Vec<String>) {
    for f in fields {
        if !known.contains(&f.key.as_str()) {
            notes.push(format!(
                "line {}: \"{}\" isn't a {what} setting, so it's skipped",
                f.line, f.key
            ));
        }
    }
}

fn new_id(id: &str, taken: &std::collections::HashSet<Id>) -> Result<Id, String> {
    let id: Id = id.parse().map_err(|_| format!("\"{id}\" isn't an ID"))?;
    if taken.contains(&id) {
        return Err("a second part with this ID".to_owned());
    }
    Ok(id)
}

/// The terrain a piece or rule names.
fn its_terrain<'a>(set: &'a Tileset, fields: &[Field]) -> Result<&'a Terrain, String> {
    match get(fields, "terrain") {
        Some(Value::Text(t)) => t
            .parse::<Id>()
            .ok()
            .and_then(|id| set.terrain(id))
            .ok_or_else(|| format!("no terrain has the ID \"{t}\"")),
        _ => Err("it needs its terrain's ID in quotes".to_owned()),
    }
}

fn its_slot(fields: &[Field], kind: TileKind) -> Result<u32, String> {
    let most = crate::autotile::slot_count(kind).saturating_sub(1);
    match get(fields, "slot") {
        Some(v) => whole(v, most).ok_or_else(|| {
            format!(
                "its slot should be a whole number from 0 to {most} (its terrain is {})",
                kind.word()
            )
        }),
        None => Err("it needs a slot".to_owned()),
    }
}

fn piece(
    set: &Tileset,
    id: &str,
    fields: &[Field],
    taken: &std::collections::HashSet<Id>,
) -> Result<Piece, String> {
    let id = new_id(id, taken)?;
    let terrain = its_terrain(set, fields)?;
    let slot = its_slot(fields, terrain.kind)?;
    let at = get(fields, "at")
        .and_then(|v| pair(v, 0, MAX_AT))
        .ok_or_else(|| {
            format!("its place in the picture should be two whole numbers up to {MAX_AT}")
        })?;
    let weight = match get(fields, "weight") {
        None => 1.0,
        Some(v) => v
            .as_f32()
            .filter(|w| w.is_finite() && (0.0..=MAX_WEIGHT).contains(w))
            .ok_or_else(|| format!("its weight should be a number from 0 to {MAX_WEIGHT}"))?,
    };
    let animation = match get(fields, "animation") {
        None => Animation::None,
        Some(Value::Text(t)) if t == "none" => Animation::None,
        Some(Value::Text(t)) if t == "frames" => Animation::Frames,
        Some(Value::Text(t)) if t == "columns" => {
            let count = get(fields, "frames")
                .and_then(|v| whole(v, MAX_FRAMES))
                .filter(|&c| c >= 2)
                .ok_or_else(|| format!("an animation's frames should be 2 to {MAX_FRAMES}"))?;
            let ms = get(fields, "ms")
                .and_then(|v| whole(v, FRAME_MS.1))
                .filter(|&m| m >= FRAME_MS.0)
                .ok_or_else(|| {
                    format!(
                        "a frame's time should be {} to {} milliseconds",
                        FRAME_MS.0, FRAME_MS.1
                    )
                })?;
            Animation::Columns { count, ms }
        }
        Some(_) => {
            return Err("its animation should be \"none\", \"columns\" or \"frames\"".to_owned());
        }
    };
    Ok(Piece {
        id,
        terrain: terrain.id,
        slot,
        at,
        weight,
        animation,
    })
}

fn rule(
    set: &Tileset,
    id: &str,
    fields: &[Field],
    taken: &std::collections::HashSet<Id>,
) -> Result<Rule, String> {
    let id = new_id(id, taken)?;
    let terrain = its_terrain(set, fields)?;
    if terrain.kind != TileKind::Rules {
        return Err(format!(
            "its terrain \"{}\" isn't of its own rules",
            terrain.name
        ));
    }
    let slot = its_slot(fields, TileKind::Rules)?;
    let Some(Value::List(rows)) = get(fields, "pattern") else {
        return Err("it needs a pattern: a list of rows in quotes".to_owned());
    };
    let size = rows.len();
    if size % 2 == 0 || size > MAX_PATTERN {
        return Err(format!(
            "its pattern should be 1, 3, 5, 7 or 9 rows (it has {size})"
        ));
    }
    let mid = size / 2;
    let mut pattern = Vec::with_capacity(size * size);
    for (y, row) in rows.iter().enumerate() {
        let Value::Text(row) = row else {
            return Err("each row of its pattern should be in quotes".to_owned());
        };
        let chars: Vec<char> = row.chars().collect();
        if chars.len() != size {
            return Err(format!(
                "each row of its pattern should be {size} long, as many as rows"
            ));
        }
        for (x, &c) in chars.iter().enumerate() {
            let middle = x == mid && y == mid;
            if middle != (c == '*') {
                return Err("its pattern's middle, and only its middle, is \"*\"".to_owned());
            }
            pattern.push(Cond::of(c).ok_or_else(|| format!("\"{c}\" isn't part of a pattern"))?);
        }
    }
    Ok(Rule {
        id,
        terrain: terrain.id,
        size,
        pattern,
        slot,
    })
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
                        "\"{t}\" isn't a code (one printable character, not a space, quote, \
                         backslash or one of ? = ! _ *)"
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
    let kind = match get(fields, "kind") {
        None => TileKind::Plain,
        Some(Value::Text(t)) => TileKind::from_word(t).ok_or_else(|| {
            format!("\"{t}\" isn't a kind (plain, 16-tile, 47-tile, edges or rules)")
        })?,
        Some(_) => return Err("its kind should be a word in quotes".to_owned()),
    };
    Ok(Terrain {
        id,
        name,
        code,
        colour,
        kind,
    })
}

/// Writes a tileset that [`read`] reads back.
pub fn write(tileset: &Tileset) -> String {
    let mut blocks: Vec<BlockOut<'_>> = tileset
        .terrains
        .iter()
        .map(|t| {
            let mut fields = vec![
                ("name", Value::Text(t.name.clone())),
                ("code", Value::Text(t.code.to_string())),
                ("colour", Value::floats(&t.colour)),
            ];
            if t.kind != TileKind::Plain {
                fields.push(("kind", Value::Text(t.kind.word().to_owned())));
            }
            ("terrain", t.id.to_string(), fields)
        })
        .collect();
    for p in &tileset.pieces {
        let mut fields = vec![
            ("terrain", Value::Text(p.terrain.to_string())),
            ("slot", Value::Int(i64::from(p.slot))),
            (
                "at",
                Value::List(vec![
                    Value::Int(i64::from(p.at[0])),
                    Value::Int(i64::from(p.at[1])),
                ]),
            ),
        ];
        if p.weight != 1.0 {
            fields.push(("weight", Value::Float(f64::from(p.weight))));
        }
        match p.animation {
            Animation::None => {}
            Animation::Frames => fields.push(("animation", Value::Text("frames".into()))),
            Animation::Columns { count, ms } => {
                fields.push(("animation", Value::Text("columns".into())));
                fields.push(("frames", Value::Int(i64::from(count))));
                fields.push(("ms", Value::Int(i64::from(ms))));
            }
        }
        blocks.push(("piece", p.id.to_string(), fields));
    }
    for r in &tileset.rules {
        blocks.push((
            "rule",
            r.id.to_string(),
            vec![
                ("terrain", Value::Text(r.terrain.to_string())),
                (
                    "pattern",
                    Value::List(r.rows().into_iter().map(Value::Text).collect()),
                ),
                ("slot", Value::Int(i64::from(r.slot))),
            ],
        ));
    }
    let mut fields = Vec::new();
    if let Some(picture) = tileset.picture {
        fields.push(("picture", Value::Text(picture.0.to_string())));
    }
    if tileset.picture.is_some() || tileset.tile != [16, 16] {
        fields.push((
            "tile",
            Value::List(vec![
                Value::Int(i64::from(tileset.tile[0])),
                Value::Int(i64::from(tileset.tile[1])),
            ]),
        ));
    }
    settings::write_block_settings(
        KIND,
        VERSION,
        "A tileset. Written by Solis; safe to edit by hand.\n\
         Each terrain is something a tile map's cell can be: its name, the one character\n\
         a map file writes for it, its colour, and how its art is chosen (its kind).\n\
         Each piece is a tile of the picture filling one of a terrain's slots; each rule\n\
         a pattern of the cells around one, for a terrain of its own rules.",
        &fields,
        &blocks,
    )
}

#[cfg(test)]
#[path = "tileset_tests.rs"]
pub(crate) mod tests;
