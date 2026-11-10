//! A tile map as a Lust data file (`farm.tilemap.lst`, version 3;
//! [decision 0081](../../../docs/decisions/0081-what-solis-makes-is-lust.md)).
//! The same parts as before, each a Lust thing in a list of its kind:
//!
//! ```text
//! -- A tile map. Written by Solis; safe to edit by hand.
//! fetch "solis/data/tilemap":[ValueKind, TileMap, Layer, Value, Cells, Exact, Numbers, Switches, Choices];
//!
//! lock int version = 3;
//! lock TileMap tiles = new TileMap { tileset = "1BWHNR3RV8PYY", size = [60, 40] };
//! lock Layer array layers = [
//!     new Layer { id = "3KD8V0Q2MX7TA", name = "Ground", over = false },
//! ];
//! lock Value array values = [
//!     new Value { id = "7PQ2B5N8WK1ZC", name = "crop", kind = ValueKind.Choice, options = ["wheat", "corn"] },
//! ];
//! lock Cells array cells = [
//!     new Cells { layer = "3KD8V0Q2MX7TA", chunk = [0, 0], rows = ["....tttt", "....tttt"] },
//! ];
//! lock Exact array exact = [
//!     new Exact { layer = "3KD8V0Q2MX7TA", chunk = [0, 0], cells = [[4, 1]], pieces = ["5GQ1D9W7XK2PB"] },
//! ];
//! lock Numbers array numbers = [new Numbers { value = "growth", chunk = [0, 0], cells = [[4, 0, 2]] }];
//! lock Switches array switches = [new Switches { value = "watered", chunk = [0, 0], cells = [[4, 0]] }];
//! lock Choices array choices = [
//!     new Choices { value = "crop", chunk = [0, 0], cells = [[4, 0]], picks = ["wheat"] },
//! ];
//! end
//! ```
//!
//! Reading turns it into the blocks the older layout had, so one set of
//! checks (`map_file`) makes the map from either.

use crate::map::{CHUNK, MAX_LAYERS, MAX_SIDE, MAX_VALUES};
use area::lust_data::{self as data, DataError, Fixed, Values};
use area::settings::{Block, BlockOut, BlockSettings, Field};
use reflect::Value;
use std::collections::HashMap;

/// The word before `.lst` in a tile map's file name.
pub const KIND: &str = "tilemap";

/// What a tile map file holds, in Lust: written into every project's
/// `solis/data/tilemap.lst`, and handed to Lust when a map is read.
pub const CLASSES: &str = "-- What a tile map file holds (Solis writes this; tile maps fetch it).\n\
state ValueKind = [Number, Switch, Choice];\n\n\
class TileMap {\n    has {\n        string tileset = \"\";\n        int array size = [];\n    }\n}\n\n\
class Layer {\n    has {\n        string id = \"\";\n        string name = \"\";\n        bool over = false;\n    }\n}\n\n\
class Value {\n    has {\n        string id = \"\";\n        string name = \"\";\n        ValueKind kind = ValueKind.Number;\n        string array options = [];\n    }\n}\n\n\
class Cells {\n    has {\n        string layer = \"\";\n        int array chunk = [];\n        string array rows = [];\n    }\n}\n\n\
class Exact {\n    has {\n        string layer = \"\";\n        int array chunk = [];\n        int array array cells = [];\n        string array pieces = [];\n    }\n}\n\n\
class Numbers {\n    has {\n        string value = \"\";\n        int array chunk = [];\n        int array array cells = [];\n    }\n}\n\n\
class Switches {\n    has {\n        string value = \"\";\n        int array chunk = [];\n        int array array cells = [];\n    }\n}\n\n\
class Choices {\n    has {\n        string value = \"\";\n        int array chunk = [];\n        int array array cells = [];\n        string array picks = [];\n    }\n}\n\n\
pass ValueKind, TileMap, Layer, Value, Cells, Exact, Numbers, Switches, Choices;\nend\n";

/// The lists of a map file's things.
const LISTS: [&str; 7] = [
    "layers", "values", "cells", "exact", "numbers", "switches", "choices",
];

/// The most things a map file has: every layer and value, and for each
/// chunk of the biggest map every layer's cells and exact tiles and every
/// value's cells (1,048,624). The older layout's limit on its blocks is
/// lower because it keeps a chunk's values in one block.
const MAX_THINGS: usize = MAX_LAYERS
    + MAX_VALUES
    + (MAX_SIDE.div_ceil(CHUNK) as usize).pow(2) * (2 * MAX_LAYERS + MAX_VALUES);

const NAMES: [&str; 9] = [
    "ValueKind",
    "TileMap",
    "Layer",
    "Value",
    "Cells",
    "Exact",
    "Numbers",
    "Switches",
    "Choices",
];

/// A chunk name `x.y` as Lust's `[x, y]`.
fn chunk_list(name: &str) -> String {
    let parts: Vec<&str> = name.split('.').collect();
    format!("[{}]", parts.join(", "))
}

/// A cell's `[x, y, …]` as whole numbers, and its third part if any.
fn cell_parts(cell: &Value) -> (Vec<i64>, Option<&Value>) {
    let Value::List(parts) = cell else {
        return (Vec::new(), None);
    };
    let at = parts
        .iter()
        .take(2)
        .filter_map(|p| {
            if let Value::Int(n) = p {
                Some(*n)
            } else {
                None
            }
        })
        .collect();
    (at, parts.get(2))
}

/// Writes the map's fields and blocks (as the older layout had them) as a
/// Lust data file.
pub fn write(
    comment: &str,
    version: u32,
    fields: &[(&str, Value)],
    blocks: &[BlockOut<'_>],
) -> String {
    let mut out = data::header(comment, KIND, &NAMES, version);
    let text_of = |key: &str| fields.iter().find(|(k, _)| *k == key).map(|(_, v)| v);
    let tileset = match text_of("tileset") {
        Some(Value::Text(t)) => t.clone(),
        _ => String::new(),
    };
    let size = match text_of("size") {
        Some(Value::List(items)) => data::ints(items.iter().filter_map(|v| match v {
            Value::Int(n) => Some(*n),
            _ => None,
        })),
        _ => "[]".to_string(),
    };
    out.push_str(&format!(
        "lock TileMap tiles = {};\n",
        data::thing(
            "TileMap",
            &[("tileset", data::text(&tileset)), ("size", size)]
        )
    ));
    let field = |fields: &[(&str, Value)], key: &str| -> Option<Value> {
        fields
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| v.clone())
    };
    let text_field = |fields: &[(&str, Value)], key: &str| match field(fields, key) {
        Some(Value::Text(t)) => t,
        _ => String::new(),
    };
    let mut lists: [(&str, &str, Vec<String>); 7] = [
        ("Layer", "layers", Vec::new()),
        ("Value", "values", Vec::new()),
        ("Cells", "cells", Vec::new()),
        ("Exact", "exact", Vec::new()),
        ("Numbers", "numbers", Vec::new()),
        ("Switches", "switches", Vec::new()),
        ("Choices", "choices", Vec::new()),
    ];
    for (word, name, fields) in blocks {
        match *word {
            "layer" => {
                let over = matches!(field(fields, "over"), Some(Value::Bool(true)));
                lists[0].2.push(data::thing(
                    "Layer",
                    &[
                        ("id", data::text(name)),
                        ("name", data::text(&text_field(fields, "name"))),
                        ("over", over.to_string()),
                    ],
                ));
            }
            "value" => {
                let kind = match text_field(fields, "kind").as_str() {
                    "switch" => "ValueKind.Switch",
                    "choice" => "ValueKind.Choice",
                    _ => "ValueKind.Number",
                };
                let mut parts = vec![
                    ("id", data::text(name)),
                    ("name", data::text(&text_field(fields, "name"))),
                    ("kind", kind.to_string()),
                ];
                if let Some(Value::List(options)) = field(fields, "options") {
                    let options: Vec<String> = options
                        .iter()
                        .filter_map(|o| {
                            if let Value::Text(t) = o {
                                Some(t.clone())
                            } else {
                                None
                            }
                        })
                        .collect();
                    parts.push(("options", data::texts(options.iter().map(String::as_str))));
                }
                lists[1].2.push(data::thing("Value", &parts));
            }
            "cells" | "exact" => {
                let (layer, chunk) = name.split_once('.').unwrap_or((name, "0.0"));
                let Some(Value::List(items)) =
                    field(fields, if *word == "cells" { "rows" } else { "cells" })
                else {
                    continue;
                };
                if *word == "cells" {
                    let rows: Vec<String> = items
                        .iter()
                        .filter_map(|r| {
                            if let Value::Text(t) = r {
                                Some(t.clone())
                            } else {
                                None
                            }
                        })
                        .collect();
                    lists[2].2.push(data::thing(
                        "Cells",
                        &[
                            ("layer", data::text(layer)),
                            ("chunk", chunk_list(chunk)),
                            ("rows", data::texts(rows.iter().map(String::as_str))),
                        ],
                    ));
                } else {
                    let mut at = Vec::new();
                    let mut pieces = Vec::new();
                    for cell in &items {
                        let (xy, piece) = cell_parts(cell);
                        if let Some(Value::Text(p)) = piece {
                            at.push(data::ints(xy));
                            pieces.push(p.clone());
                        }
                    }
                    lists[3].2.push(data::thing(
                        "Exact",
                        &[
                            ("layer", data::text(layer)),
                            ("chunk", chunk_list(chunk)),
                            ("cells", format!("[{}]", at.join(", "))),
                            ("pieces", data::texts(pieces.iter().map(String::as_str))),
                        ],
                    ));
                }
            }
            _ => {
                // A chunk's values: each value's cells, by its kind.
                for (value, list) in fields {
                    let Value::List(cells) = list else {
                        continue;
                    };
                    let mut numbers = Vec::new();
                    let mut switches = Vec::new();
                    let mut choices = Vec::new();
                    let mut picks = Vec::new();
                    for cell in cells {
                        let (mut xy, third) = cell_parts(cell);
                        match third {
                            Some(Value::Int(n)) => {
                                xy.push(*n);
                                numbers.push(data::ints(xy));
                            }
                            Some(Value::Text(t)) => {
                                choices.push(data::ints(xy));
                                picks.push(t.clone());
                            }
                            _ => switches.push(data::ints(xy)),
                        }
                    }
                    let base = [("value", data::text(value)), ("chunk", chunk_list(name))];
                    if !numbers.is_empty() {
                        let mut parts = base.to_vec();
                        parts.push(("cells", format!("[{}]", numbers.join(", "))));
                        lists[4].2.push(data::thing("Numbers", &parts));
                    }
                    if !switches.is_empty() {
                        let mut parts = base.to_vec();
                        parts.push(("cells", format!("[{}]", switches.join(", "))));
                        lists[5].2.push(data::thing("Switches", &parts));
                    }
                    if !choices.is_empty() {
                        let mut parts = base.to_vec();
                        parts.push(("cells", format!("[{}]", choices.join(", "))));
                        parts.push(("picks", data::texts(picks.iter().map(String::as_str))));
                        lists[6].2.push(data::thing("Choices", &parts));
                    }
                }
            }
        }
    }
    for (class, name, items) in &lists {
        data::list(&mut out, class, name, items);
    }
    out.push_str("end\n");
    out
}

/// A field for the checks, on the line its list starts.
fn field(line: usize, key: &str, value: Value) -> Field {
    Field {
        line,
        key: key.to_string(),
        value,
    }
}

/// A Lust list of numbers as the older layout's `[x, y, …]`.
fn int_values(items: &[Fixed]) -> Value {
    Value::List(
        items
            .iter()
            .filter_map(Fixed::as_int)
            .map(Value::Int)
            .collect(),
    )
}

/// A thing's chunk `[x, y]`, or none if it isn't two whole numbers.
fn chunk_of(thing: &Fixed) -> Option<[i64; 2]> {
    match data::field_ints(thing, "chunk")?.as_slice() {
        [x, y] => Some([*x, *y]),
        _ => None,
    }
}

/// A chunk `[x, y]` as its name `x.y`, or none.
fn chunk_name(thing: &Fixed) -> Option<String> {
    chunk_of(thing).map(|[x, y]| format!("{x}.{y}"))
}

/// Reads a tile map's Lust data file into the blocks the older layout had.
pub fn read(text: &str, max_bytes: usize) -> Result<BlockSettings, DataError> {
    let values = data::read(text, KIND, CLASSES, max_bytes)?;
    blocks(&values, data::version_of(text).unwrap_or(0))
}

/// What a map file's values hold, as the blocks the older layout had. A
/// file with more things than the biggest map has is refused before any
/// block is made of them.
fn blocks(values: &Values, version: u32) -> Result<BlockSettings, DataError> {
    let things: usize = LISTS
        .iter()
        .map(|list| data::things(values, list).len())
        .sum();
    if things > MAX_THINGS {
        return Err(data::error(
            0,
            format!(
                "it lists more layers, values and chunks of cells ({things}) than the \
                 biggest tile map has ({MAX_THINGS})"
            ),
        ));
    }
    let line_of = |name: &str| {
        values
            .values
            .iter()
            .find(|v| v.name == name)
            .map_or(0, |v| v.line as usize)
    };
    let tiles = data::get(values, "tiles")?;
    let line = line_of("tiles");
    let mut out = BlockSettings {
        version,
        fields: vec![
            field(
                line,
                "tileset",
                Value::Text(data::field_text(tiles, "tileset").to_string()),
            ),
            field(line, "size", int_values(data::field_list(tiles, "size"))),
        ],
        blocks: Vec::new(),
    };
    let mut block = |line: usize, word: &str, name: String, fields: Vec<Field>| {
        out.blocks.push(Block {
            line,
            word: word.to_string(),
            name,
            fields,
        });
    };
    let line = line_of("layers");
    for l in data::things(values, "layers") {
        block(
            line,
            "layer",
            data::field_text(l, "id").to_string(),
            vec![
                field(
                    line,
                    "name",
                    Value::Text(data::field_text(l, "name").to_string()),
                ),
                field(line, "over", Value::Bool(data::field_bool(l, "over"))),
            ],
        );
    }
    let line = line_of("values");
    for v in data::things(values, "values") {
        let kind = data::field_option(v, "kind")
            .unwrap_or("Number")
            .to_ascii_lowercase();
        let options = data::field_texts(v, "options");
        let mut fields = vec![
            field(
                line,
                "name",
                Value::Text(data::field_text(v, "name").to_string()),
            ),
            field(line, "kind", Value::Text(kind)),
        ];
        if !options.is_empty() {
            fields.push(field(
                line,
                "options",
                Value::List(
                    options
                        .into_iter()
                        .map(|o| Value::Text(o.to_string()))
                        .collect(),
                ),
            ));
        }
        block(line, "value", data::field_text(v, "id").to_string(), fields);
    }
    let line = line_of("cells");
    for c in data::things(values, "cells") {
        let chunk =
            chunk_name(c).ok_or_else(|| data::error(line, "a Cells' chunk should be [x, y]"))?;
        let rows = data::field_texts(c, "rows");
        block(
            line,
            "cells",
            format!("{}.{chunk}", data::field_text(c, "layer")),
            vec![field(
                line,
                "rows",
                Value::List(
                    rows.into_iter()
                        .map(|r| Value::Text(r.to_string()))
                        .collect(),
                ),
            )],
        );
    }
    let line = line_of("exact");
    for e in data::things(values, "exact") {
        let chunk =
            chunk_name(e).ok_or_else(|| data::error(line, "an Exact's chunk should be [x, y]"))?;
        let pieces = data::field_texts(e, "pieces");
        let cells = data::field_list(e, "cells");
        if cells.len() != pieces.len() {
            return Err(data::error(
                line,
                "an Exact needs a piece for each of its cells",
            ));
        }
        let items = cells
            .iter()
            .zip(pieces)
            .map(|(cell, piece)| {
                let Value::List(mut parts) = int_values(cell.as_list().unwrap_or_default()) else {
                    unreachable!("int_values makes a list")
                };
                parts.push(Value::Text(piece.to_string()));
                Value::List(parts)
            })
            .collect();
        block(
            line,
            "exact",
            format!("{}.{chunk}", data::field_text(e, "layer")),
            vec![field(line, "cells", Value::List(items))],
        );
    }
    out.blocks.extend(value_blocks(values, line_of)?);
    Ok(out)
}

/// One chunk's values, while a file's things are gathered.
struct ChunkValues<'a> {
    chunk: [i64; 2],
    /// The line of the list that first names the chunk.
    line: usize,
    /// Each value: its name, the line of the list that first names it in
    /// this chunk, and its cells.
    values: Vec<(&'a str, usize, Vec<Value>)>,
}

/// Each chunk's values, gathered from the three lists of them into one
/// `values` block per chunk as the older layout had: the chunks, and each
/// chunk's values, in the order the file first names them. A file may name
/// as many chunks, or as many values in one chunk, as it has things (they
/// aren't checked against the map until its blocks are read), so both are
/// looked up, never searched for: a search made the time grow with the
/// square of the things.
fn value_blocks(values: &Values, line_of: impl Fn(&str) -> usize) -> Result<Vec<Block>, DataError> {
    let mut chunks: Vec<ChunkValues<'_>> = Vec::new();
    let mut chunk_at: HashMap<[i64; 2], usize> = HashMap::new();
    let mut value_at: HashMap<(usize, &str), usize> = HashMap::new();
    for (list, with_picks) in [("numbers", false), ("switches", false), ("choices", true)] {
        let line = line_of(list);
        for t in data::things(values, list) {
            let chunk = chunk_of(t).ok_or_else(|| data::error(line, "a chunk should be [x, y]"))?;
            let cells = data::field_list(t, "cells");
            let picks = data::field_texts(t, "picks");
            if with_picks && picks.len() != cells.len() {
                return Err(data::error(
                    line,
                    "Choices need a pick for each of their cells",
                ));
            }
            let items = cells.iter().enumerate().map(|(i, cell)| {
                let Value::List(mut parts) = int_values(cell.as_list().unwrap_or_default()) else {
                    unreachable!("int_values makes a list")
                };
                if with_picks {
                    parts.push(Value::Text(picks[i].to_string()));
                }
                Value::List(parts)
            });
            let at = *chunk_at.entry(chunk).or_insert_with(|| {
                chunks.push(ChunkValues {
                    chunk,
                    line,
                    values: Vec::new(),
                });
                chunks.len() - 1
            });
            let named = &mut chunks[at].values;
            let key = data::field_text(t, "value");
            let value = *value_at.entry((at, key)).or_insert_with(|| {
                named.push((key, line, Vec::new()));
                named.len() - 1
            });
            named[value].2.extend(items);
        }
    }
    let blocks = chunks.into_iter().map(|gathered| Block {
        line: gathered.line,
        word: "values".to_string(),
        name: format!("{}.{}", gathered.chunk[0], gathered.chunk[1]),
        fields: (gathered.values.into_iter())
            .map(|(key, line, cells)| field(line, key, Value::List(cells)))
            .collect(),
    });
    Ok(blocks.collect())
}

#[cfg(test)]
#[path = "map_lust_tests.rs"]
mod tests;
