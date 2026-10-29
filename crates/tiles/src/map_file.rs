//! A tile map file (`.soltilemap`, Phase 9, D4): a map saved as readable
//! text in chunks of 32 × 32 cells, so a Git diff shows exactly the cells
//! that changed. Its tileset, its size, a block per layer and per value
//! (by stable random IDs), then each chunk's rows of terrain codes per
//! layer (a space for an empty cell), and each chunk's values where they
//! aren't at their start:
//!
//! ```text
//! -- A tile map. Written by Solis; safe to edit by hand.
//! tilemap version 1;
//!
//! tileset = "1BWHNR3RV8PYY";
//! size = [60, 40];
//!
//! layer 3KD8V0Q2MX7TA { name = "Ground"; over = false; }
//! value 7PQ2B5N8WK1ZC { name = "crop"; kind = "choice"; options = ["wheat", "corn"]; }
//!
//! cells 3KD8V0Q2MX7TA.0.0 { rows = ["....tttt", "....tttt"]; }
//! values 0.0 { crop = [[4, 0, "wheat"]]; growth = [[4, 0, 2]]; watered = [[4, 0]]; }
//! exact 3KD8V0Q2MX7TA.0.0 { cells = [[4, 1, "5GQ1D9W7XK2PB"]]; }
//! ```
//!
//! A chunk with every cell empty, or every value at its start, is left
//! out. The art isn't stored: it's worked out from the terrains, except
//! for exact tiles (version 2, D6): cells painted to show one piece of the
//! tileset as it is, each [x, y, piece's ID] within the chunk. Read as a
//! hostile file: within limits, a part it can't use skipped with a note;
//! a code the tileset hasn't is kept (Problems says so).

use crate::map::{
    CHUNK, CellValue, MAX_LAYERS, MAX_OPTIONS, MAX_SIDE, MAX_VALUES, TileMap, ValueKind,
};
use crate::tileset::{name_of, usable_code};
use area::settings::{self, BlockOut, Field};
use reflect::{AssetId, Id, Value};
use std::fmt;

/// The layout version this build writes and reads.
pub const VERSION: u32 = 2;
const KIND: &str = "tilemap";
/// The file's extension.
pub const EXTENSION: &str = "soltilemap";
/// The biggest map file read.
pub const MAX_BYTES: usize = 64 << 20;
/// The most blocks a map file has (every chunk of every layer of the
/// biggest map, and its values').
const MAX_BLOCKS: usize = 300_000;
/// A new map's size.
pub const STANDARD_SIZE: [u32; 2] = [64, 64];

/// This file's version and upgraders (Phase 8, C1).
pub static FORMAT: reflect::upgrade::Format<settings::BlockSettings> = reflect::upgrade::Format {
    name: "tile map",
    current: VERSION,
    version_of: |text| reflect::upgrade::header_version(text, &[KIND]),
    steps: &[exact_came_later],
};

/// Version 1 to 2: nothing to change. Version 1 had no exact tiles; every
/// cell is autotiled, which is what a cell without one is.
fn exact_came_later(_: &mut settings::BlockSettings) -> Result<(), String> {
    Ok(())
}

/// Why a map file couldn't be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapFileError {
    /// The line it went wrong on (from 1), or 0 for the whole file.
    pub line: usize,
    /// What went wrong, in plain words.
    pub message: String,
}

impl fmt::Display for MapFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line == 0 {
            f.write_str(&self.message)
        } else {
            write!(f, "line {}: {}", self.line, self.message)
        }
    }
}

impl std::error::Error for MapFileError {}

/// A map file as read, with notes about what was put right.
#[derive(Debug, Clone, PartialEq)]
pub struct ReadMap {
    /// The map.
    pub map: TileMap,
    /// What was skipped, in plain words with lines.
    pub notes: Vec<String>,
}

/// Whether `path` names a tile map file (by its extension).
pub fn is_map_file(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case(EXTENSION))
}

fn get<'a>(fields: &'a [Field], key: &str) -> Option<&'a Value> {
    fields.iter().find(|f| f.key == key).map(|f| &f.value)
}

/// A chunk's name, `x.y` (or a layer's chunk, `ID.x.y`), as chunk
/// numbers within the map.
fn chunk_at(map: &TileMap, text: &str) -> Result<[u32; 2], String> {
    let (x, y) = text
        .split_once('.')
        .ok_or_else(|| format!("\"{text}\" isn't a chunk (x.y)"))?;
    let [across, down] = map.chunks();
    match (x.parse::<u32>(), y.parse::<u32>()) {
        (Ok(x), Ok(y)) if x < across && y < down => Ok([x, y]),
        _ => Err(format!("\"{text}\" isn't a chunk of this map")),
    }
}

/// The blocks of a map file, in the order they're read (a layer before
/// its cells, the cells before their exact tiles).
const WORDS: [&str; 5] = ["layer", "value", "cells", "exact", "values"];

/// A layer's chunk, `ID.x.y`: the layer's index and the chunk.
fn layer_chunk(map: &TileMap, name: &str) -> Result<(usize, [u32; 2]), String> {
    let (layer_id, chunk) = name
        .split_once('.')
        .ok_or_else(|| format!("\"{name}\" isn't a layer's chunk (ID.x.y)"))?;
    let layer_id: Id = layer_id
        .parse()
        .map_err(|_| format!("\"{layer_id}\" isn't an ID"))?;
    let layer = map
        .layers()
        .iter()
        .position(|l| l.id == layer_id)
        .ok_or("its layer isn't one of the map's")?;
    Ok((layer, chunk_at(map, chunk)?))
}

/// Reads a map file.
pub fn read(text: &str) -> Result<ReadMap, MapFileError> {
    if text.len() > MAX_BYTES {
        return Err(MapFileError {
            line: 0,
            message: format!(
                "the file is {} MB; tile maps up to {} MB are read",
                text.len() >> 20,
                MAX_BYTES >> 20
            ),
        });
    }
    let mut read =
        settings::read_block_settings_within(text, KIND, VERSION, MAX_BLOCKS).map_err(|e| {
            MapFileError {
                line: e.line,
                message: e.message,
            }
        })?;
    FORMAT
        .upgrade(read.version, &mut read)
        .map_err(|e| MapFileError {
            line: 1,
            message: e.to_string(),
        })?;
    let mut notes = Vec::new();
    let mut tileset = None;
    let mut size = STANDARD_SIZE;
    for f in &read.fields {
        let line = f.line;
        match (f.key.as_str(), &f.value) {
            ("tileset", Value::Text(t)) if t.trim().is_empty() => {}
            ("tileset", Value::Text(t)) => match t.trim().parse::<AssetId>() {
                Ok(id) => tileset = Some(id),
                Err(_) => notes.push(format!(
                    "line {line}: the tileset isn't an asset's ID, so it's skipped"
                )),
            },
            ("size", v) => match v.as_floats::<2>() {
                Some([w, h])
                    if (1.0..=MAX_SIDE as f32).contains(&w)
                        && (1.0..=MAX_SIDE as f32).contains(&h) =>
                {
                    size = [w as u32, h as u32];
                }
                _ => {
                    return Err(MapFileError {
                        line,
                        message: format!(
                            "the size should be two whole numbers from 1 to {MAX_SIDE}"
                        ),
                    });
                }
            },
            (key, _) => notes.push(format!(
                "line {line}: \"{key}\" isn't a tile map setting, so it's skipped"
            )),
        }
    }
    let mut map = TileMap::bare(size, tileset);
    for word in WORDS {
        for block in read.blocks.iter().filter(|b| b.word == word) {
            let line = block.line;
            let result = match word {
                "layer" => layer(&mut map, &block.name, &block.fields),
                "value" => value(&mut map, &block.name, &block.fields),
                "cells" => cells(&mut map, &block.name, &block.fields),
                "exact" => exact(&mut map, &block.name, &block.fields),
                _ => values(&mut map, &block.name, &block.fields, &mut notes, line),
            };
            if let Err(why) = result {
                notes.push(format!(
                    "line {line}: {why}, so this {word} block is skipped"
                ));
            }
        }
    }
    for block in &read.blocks {
        if !WORDS.contains(&block.word.as_str()) {
            notes.push(format!(
                "line {}: \"{}\" isn't a part of a tile map, so it's skipped",
                block.line, block.word
            ));
        }
    }
    // What was read is the map as saved: nothing to draw again for it.
    let _ = map.take_dirty();
    Ok(ReadMap { map, notes })
}

fn layer(map: &mut TileMap, id: &str, fields: &[Field]) -> Result<(), String> {
    let id: Id = id.parse().map_err(|_| format!("\"{id}\" isn't an ID"))?;
    if map.layers().iter().any(|l| l.id == id) {
        return Err("a second layer with this ID".to_owned());
    }
    if map.layers().len() >= MAX_LAYERS {
        return Err(format!("more than {MAX_LAYERS} layers"));
    }
    let name = match get(fields, "name") {
        Some(Value::Text(t)) => name_of(t)?,
        _ => return Err("a layer needs a name in quotes".to_owned()),
    };
    if map.layer_named(&name).is_some() {
        return Err(format!("a second layer called \"{name}\""));
    }
    let over = match get(fields, "over") {
        None => false,
        Some(Value::Bool(b)) => *b,
        Some(_) => return Err("over should be true or false".to_owned()),
    };
    map.add_layer_with_id(id, &name, over)
        .map(drop)
        .map_err(|_| "too many layers".to_owned())
}

fn value(map: &mut TileMap, id: &str, fields: &[Field]) -> Result<(), String> {
    let id: Id = id.parse().map_err(|_| format!("\"{id}\" isn't an ID"))?;
    if map.values().iter().any(|v| v.id == id) {
        return Err("a second value with this ID".to_owned());
    }
    if map.values().len() >= MAX_VALUES {
        return Err(format!("more than {MAX_VALUES} values"));
    }
    let name = match get(fields, "name") {
        Some(Value::Text(t)) => name_of(t)?,
        _ => return Err("a value needs a name in quotes".to_owned()),
    };
    if map.value_named(&name).is_some() {
        return Err(format!("a second value called \"{name}\""));
    }
    let kind = match get(fields, "kind") {
        Some(Value::Text(k)) if k == "number" => ValueKind::Number,
        Some(Value::Text(k)) if k == "switch" => ValueKind::Switch,
        Some(Value::Text(k)) if k == "choice" => {
            let mut options: Vec<String> = Vec::new();
            match get(fields, "options") {
                None => {}
                Some(Value::List(items)) => {
                    for item in items {
                        let Value::Text(t) = item else {
                            return Err("its options should be names in quotes".to_owned());
                        };
                        let o = name_of(t)?;
                        if options.contains(&o) {
                            return Err(format!("\"{o}\" is an option twice"));
                        }
                        if options.len() == MAX_OPTIONS {
                            return Err(format!("more than {MAX_OPTIONS} options"));
                        }
                        options.push(o);
                    }
                }
                Some(_) => return Err("its options should be a list".to_owned()),
            }
            ValueKind::Choice(options)
        }
        _ => return Err("its kind should be \"number\", \"switch\" or \"choice\"".to_owned()),
    };
    map.add_value_with_id(id, &name, kind)
        .map(drop)
        .map_err(|_| "too many values or options".to_owned())
}

fn cells(map: &mut TileMap, name: &str, fields: &[Field]) -> Result<(), String> {
    let (layer, [cx, cy]) = layer_chunk(map, name)?;
    let Some(Value::List(rows)) = get(fields, "rows") else {
        return Err("it needs its rows: rows = [\"…\", …]".to_owned());
    };
    if rows.len() > CHUNK as usize {
        return Err(format!("more than {CHUNK} rows"));
    }
    let [w, h] = map.size();
    for (r, row) in rows.iter().enumerate() {
        let Value::Text(row) = row else {
            return Err("a row should be text in quotes".to_owned());
        };
        let y = cy * CHUNK + r as u32;
        if y >= h {
            break;
        }
        for (c, ch) in row.chars().take(CHUNK as usize).enumerate() {
            let x = cx * CHUNK + c as u32;
            if x >= w {
                break;
            }
            let code = if ch == ' ' {
                0
            } else if usable_code(ch) {
                ch as u8
            } else {
                return Err(format!("'{ch}' isn't a terrain's code"));
            };
            let _ = map.set_code(layer, x, y, code);
        }
    }
    Ok(())
}

/// A chunk's exact tiles. One on an empty cell (or past the map's edge)
/// is skipped: there's nothing for it to show over.
fn exact(map: &mut TileMap, name: &str, fields: &[Field]) -> Result<(), String> {
    let (layer, [cx, cy]) = layer_chunk(map, name)?;
    let Some(Value::List(items)) = get(fields, "cells") else {
        return Err("it needs its cells: cells = [[x, y, \"piece\"], …]".to_owned());
    };
    if items.len() > (CHUNK * CHUNK) as usize {
        return Err(format!("more than {} cells", CHUNK * CHUNK));
    }
    for item in items {
        let (x, y, piece) = match item {
            Value::List(parts) => match parts.as_slice() {
                [Value::Int(x), Value::Int(y), Value::Text(p)]
                    if (0..i64::from(CHUNK)).contains(x) && (0..i64::from(CHUNK)).contains(y) =>
                {
                    (*x as u32, *y as u32, p)
                }
                _ => return Err("a cell should be [x, y, \"piece\"] within the chunk".to_owned()),
            },
            _ => return Err("a cell should be [x, y, \"piece\"] within the chunk".to_owned()),
        };
        let piece: Id = piece
            .parse()
            .map_err(|_| format!("\"{piece}\" isn't a piece's ID"))?;
        let _ = map.set_exact(layer, cx * CHUNK + x, cy * CHUNK + y, Some(piece));
    }
    Ok(())
}

fn values(
    map: &mut TileMap,
    chunk: &str,
    fields: &[Field],
    notes: &mut Vec<String>,
    line: usize,
) -> Result<(), String> {
    let [cx, cy] = chunk_at(map, chunk)?;
    for f in fields {
        let Some(v) = map.value_named(&f.key) else {
            notes.push(format!(
                "line {}: \"{}\" isn't one of the map's values, so it's skipped",
                f.line, f.key
            ));
            continue;
        };
        let Value::List(items) = &f.value else {
            notes.push(format!("line {line}: {} should be a list", f.key));
            continue;
        };
        let kind = map.values()[v].kind.clone();
        for item in items {
            let Value::List(parts) = item else {
                return Err(format!("{} should be [[x, y, value], …]", f.key));
            };
            let (x, y) = match parts.first().zip(parts.get(1)) {
                Some((Value::Int(x), Value::Int(y)))
                    if (0..i64::from(CHUNK)).contains(x) && (0..i64::from(CHUNK)).contains(y) =>
                {
                    (cx * CHUNK + *x as u32, cy * CHUNK + *y as u32)
                }
                _ => {
                    return Err(format!(
                        "a cell of {} should start with its x and y in the chunk",
                        f.key
                    ));
                }
            };
            let to = match (&kind, parts.get(2), parts.len()) {
                (ValueKind::Number, Some(Value::Int(n)), 3) => CellValue::Number((*n).clamp(
                    -i64::from(crate::map::MAX_NUMBER),
                    i64::from(crate::map::MAX_NUMBER),
                )
                    as i32),
                (ValueKind::Switch, None, 2) => CellValue::Switch(true),
                (ValueKind::Choice(options), Some(Value::Text(o)), 3) => {
                    match options.iter().position(|n| n == o) {
                        Some(p) => CellValue::Choice(Some(p as u8)),
                        None => {
                            notes.push(format!(
                                "line {line}: \"{o}\" isn't one of {}'s options, so that cell is left empty",
                                f.key
                            ));
                            continue;
                        }
                    }
                }
                _ => {
                    return Err(format!(
                        "a cell of {} doesn't fit its kind ({})",
                        f.key,
                        kind.word()
                    ));
                }
            };
            // Outside the map at its edge: skipped.
            let _ = map.set_value(v, x, y, to);
        }
    }
    Ok(())
}

/// Writes a map file that [`read`] reads back.
pub fn write(map: &TileMap) -> String {
    let [w, h] = map.size();
    let fields = [
        (
            "tileset",
            Value::Text(map.tileset.map_or_else(String::new, |s| s.to_string())),
        ),
        (
            "size",
            Value::List(vec![Value::Int(i64::from(w)), Value::Int(i64::from(h))]),
        ),
    ];
    let mut blocks: Vec<BlockOut<'_>> = Vec::new();
    for l in map.layers() {
        blocks.push((
            "layer",
            l.id.to_string(),
            vec![
                ("name", Value::Text(l.name.clone())),
                ("over", Value::Bool(l.over)),
            ],
        ));
    }
    for v in map.values() {
        let mut f = vec![
            ("name", Value::Text(v.name.clone())),
            ("kind", Value::Text(v.kind.word().to_owned())),
        ];
        if let ValueKind::Choice(options) = &v.kind {
            f.push((
                "options",
                Value::List(options.iter().map(|o| Value::Text(o.clone())).collect()),
            ));
        }
        blocks.push(("value", v.id.to_string(), f));
    }
    let [across, down] = map.chunks();
    for (li, l) in map.layers().iter().enumerate() {
        let cells = map.layer_cells(li).unwrap_or_default();
        for cy in 0..down {
            for cx in 0..across {
                let ys = cy * CHUNK..((cy + 1) * CHUNK).min(h);
                let xs = cx * CHUNK..((cx + 1) * CHUNK).min(w);
                let rows: Vec<String> = ys
                    .clone()
                    .map(|y| {
                        xs.clone()
                            .map(|x| match cells[(y * w + x) as usize] {
                                0 => ' ',
                                c => char::from(c),
                            })
                            .collect()
                    })
                    .collect();
                if rows.iter().all(|r| r.trim().is_empty()) {
                    continue;
                }
                blocks.push((
                    "cells",
                    format!("{}.{cx}.{cy}", l.id),
                    vec![(
                        "rows",
                        Value::List(rows.into_iter().map(Value::Text).collect()),
                    )],
                ));
            }
        }
    }
    for (li, l) in map.layers().iter().enumerate() {
        let mut by_chunk: std::collections::BTreeMap<(u32, u32), Vec<Value>> =
            std::collections::BTreeMap::new();
        for (i, piece) in map.exact_tiles(li) {
            let (x, y) = (i % w, i / w);
            by_chunk
                .entry((y / CHUNK, x / CHUNK))
                .or_default()
                .push(Value::List(vec![
                    Value::Int(i64::from(x % CHUNK)),
                    Value::Int(i64::from(y % CHUNK)),
                    Value::Text(piece.to_string()),
                ]));
        }
        for ((cy, cx), cells) in by_chunk {
            blocks.push((
                "exact",
                format!("{}.{cx}.{cy}", l.id),
                vec![("cells", Value::List(cells))],
            ));
        }
    }
    for cy in 0..down {
        for cx in 0..across {
            let mut f: Vec<(&str, Value)> = Vec::new();
            for (vi, def) in map.values().iter().enumerate() {
                let mut cells = Vec::new();
                for y in cy * CHUNK..((cy + 1) * CHUNK).min(h) {
                    for x in cx * CHUNK..((cx + 1) * CHUNK).min(w) {
                        if map.is_default(vi, x, y) {
                            continue;
                        }
                        let (lx, ly) = (i64::from(x - cx * CHUNK), i64::from(y - cy * CHUNK));
                        let mut cell = vec![Value::Int(lx), Value::Int(ly)];
                        match map.value(vi, x, y) {
                            Some(CellValue::Number(n)) => cell.push(Value::Int(i64::from(n))),
                            Some(CellValue::Choice(Some(o))) => {
                                if let ValueKind::Choice(options) = &def.kind {
                                    cell.push(Value::Text(options[usize::from(o)].clone()));
                                }
                            }
                            _ => {}
                        }
                        cells.push(Value::List(cell));
                    }
                }
                if !cells.is_empty() {
                    f.push((def.name.as_str(), Value::List(cells)));
                }
            }
            if !f.is_empty() {
                blocks.push(("values", format!("{cx}.{cy}"), f));
            }
        }
    }
    settings::write_block_settings(
        KIND,
        VERSION,
        "A tile map. Written by Solis; safe to edit by hand.\n\
         Cells are in chunks of 32 by 32: each layer's rows of terrain codes (a space\n\
         for an empty cell), and the values of cells that aren't at their start,\n\
         each [x, y, value] within the chunk. Exact tiles are cells that show one\n\
         piece of the tileset as it is, each [x, y, piece] within the chunk.",
        &fields,
        &blocks,
    )
}

#[cfg(test)]
#[path = "map_file_tests.rs"]
mod tests;
