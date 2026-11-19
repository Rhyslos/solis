//! A table's file, `items.table.lst`: a Lust data file
//! ([decision 0081](../../../../docs/decisions/0081-what-solis-makes-is-lust.md)),
//! version 2, a row a line so Git shows exactly which rows changed:
//!
//! ```text
//! -- A data table (the Table tab). Written by Solis; safe to edit by hand.
//! fetch "solis/data/table":[Table, Column, Row];
//!
//! lock int version = 2;
//! lock Table table = new Table { rowName = "Item" };
//! lock Column array columns = [
//!     new Column { name = "price", kind = "whole", translated = false, options = [] },
//!     new Column { name = "rarity", kind = "choice", translated = false, options = ["Common", "Rare"] },
//! ];
//! lock Row array rows = [
//!     new Row { id = "Apple", cells = ["5", "Common"] },
//! ];
//! end
//! ```
//!
//! A table, row or column renamed once its text is translated keeps the
//! name its cells' keys were made from as `key` (`new Row { id =
//! "SteelSword", key = "IronSword", … }`), so a key never changes.
//!
//! Every cell is written as text, each read by its column's kind. A row,
//! column or cell that can't be used is left out (or made its column's
//! empty value) with a note; the rest are kept. Past the limits, the
//! first rows and columns are kept with a note.

use super::{
    Column, ColumnKind, MAX_CELLS, MAX_COLUMNS, MAX_OPTIONS, MAX_ROWS, Row, Table, option_problem,
    text,
};
use crate::lust_data::{self as data, DataError};
use std::sync::Arc;

/// The word naming its classes' file (`solis/data/table.lst`) and the end
/// of its name (`items.table.lst`).
pub const KIND: &str = "table";

/// The version this build writes.
pub const VERSION: u32 = 2;

/// The most bytes a table's file may have.
pub const MAX_BYTES: usize = 96 << 20;

/// What a table's file holds, in Lust: written into every project's
/// `solis/data/table.lst`, and handed to Lust when a file is read.
pub const CLASSES: &str = "-- What a data table's file holds (Solis writes this; tables fetch it).\n\
class Table {\n    has {\n        string rowName = \"\";\n        string key = \"\";\n    }\n}\n\n\
class Column {\n    has {\n        string name = \"\";\n        string key = \"\";\n        string kind = \"text\";\n        bool translated = false;\n        string array options = [];\n    }\n}\n\n\
class Row {\n    has {\n        string id = \"\";\n        string key = \"\";\n        string array cells = [];\n    }\n}\n\n\
pass Table, Column, Row;\nend\n";

/// The tables' version and upgraders. Version 2 (stage 1, translations)
/// keeps the key part of a renamed table, row or column (`key`); a file of
/// version 1 has none, so its parts are its names.
pub static FORMAT: reflect::upgrade::Format<Table> = reflect::upgrade::Format {
    name: "data table",
    current: VERSION,
    version_of: data::version_of,
    steps: &[|_| Ok(())],
};

const COMMENT: &str = "A data table (the Table tab). Written by Solis; safe to edit by hand.";

/// A table's file, read: the table, and notes on anything left out.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Read {
    /// The table.
    pub table: Table,
    /// What was left out or changed, and why, in plain words.
    pub notes: Vec<String>,
}

/// Whether `path` is a table's file (`items.table.lst`).
pub fn is_table_file(path: &std::path::Path) -> bool {
    reflect::data_files::is(path, KIND)
}

/// `table` as a file that [`read`] reads back.
pub fn write(table: &Table) -> String {
    let mut out = data::header(COMMENT, KIND, &["Table", "Column", "Row"], VERSION);
    let mut fields = vec![("rowName", data::text(&table.row_name))];
    if !table.kept.table.is_empty() {
        fields.push(("key", data::text(&table.kept.table)));
    }
    out.push_str(&format!(
        "lock Table table = {};\n",
        data::thing("Table", &fields)
    ));
    let columns: Vec<String> = table
        .columns
        .iter()
        .map(|c| {
            let options: &[String] = match &c.kind {
                ColumnKind::Choice(options) => options,
                _ => &[],
            };
            let mut fields = vec![("name", data::text(&c.name))];
            if let Some(key) = table.kept.columns.get(&c.name) {
                fields.push(("key", data::text(key)));
            }
            fields.extend([
                ("kind", data::text(c.kind.word())),
                ("translated", c.translated.to_string()),
                ("options", data::texts(options.iter().map(String::as_str))),
            ]);
            data::thing("Column", &fields)
        })
        .collect();
    data::list(&mut out, "Column", "columns", &columns);
    let mut rows = String::with_capacity(table.rows.len() * (16 + 12 * table.columns.len()));
    if table.rows.is_empty() {
        rows.push_str("lock Row array rows = [];\n");
    } else {
        rows.push_str("lock Row array rows = [\n");
        for row in &table.rows {
            rows.push_str("    new Row { id = ");
            rows.push_str(&data::text(&row.id));
            if let Some(key) = table.kept.rows.get(&row.id) {
                rows.push_str(", key = ");
                rows.push_str(&data::text(key));
            }
            rows.push_str(", cells = [");
            for (n, cell) in row.cells.iter().enumerate() {
                if n > 0 {
                    rows.push_str(", ");
                }
                rows.push_str(&data::text(&text::cell_text(cell)));
            }
            rows.push_str("] },\n");
        }
        rows.push_str("];\n");
    }
    out.push_str(&rows);
    out.push_str("end\n");
    out
}

/// Reads a table's file, whose rows are each a `fallback_row_name` if it
/// names none. Refused, with its line, only when it can't be read at all
/// or is from a newer version.
pub fn read(text: &str, fallback_row_name: &str) -> Result<Read, DataError> {
    let version = data::version_of(text).unwrap_or(0);
    if version > VERSION {
        return Err(data::error(
            0,
            format!(
                "it was saved by a newer version (data table {version}; this one reads up to \
                 {VERSION}). Update to open it."
            ),
        ));
    }
    let values = data::read(text, KIND, CLASSES, MAX_BYTES)?;
    let mut notes = Vec::new();
    let row_name = values
        .get("table")
        .map(|t| data::field_text(t, "rowName"))
        .unwrap_or("");
    let row_name = match option_problem(row_name, "What each row is called") {
        None if row_name != super::NO_ROW => row_name.to_string(),
        _ => {
            if !row_name.is_empty() {
                notes.push(format!(
                    "Each row's name \"{}\" can't be named in game code, so rows are called {} \
                     here.",
                    shown(row_name),
                    fallback_row_name
                ));
            }
            fallback_row_name.to_string()
        }
    };
    let mut table = Table::new(row_name);
    let part = |thing: &data::Fixed| {
        let key = data::field_text(thing, "key");
        (option_problem(key, "").is_none() || field_problem_free(key)).then(|| key.to_owned())
    };
    if let Some(key) = values.get("table").and_then(part) {
        table.kept.table = key;
    }
    let mut kept = Vec::new();
    for (at, thing) in data::things(&values, "columns").iter().enumerate() {
        if let Some(column) = read_column(&table, thing, &mut notes) {
            if table.columns.len() >= MAX_COLUMNS {
                notes.push(format!(
                    "Only the first {MAX_COLUMNS} columns are kept; the rest are left out."
                ));
                break;
            }
            if let Some(key) = part(thing).filter(|k| *k != column.name) {
                table.kept.columns.insert(column.name.clone(), key);
            }
            table.columns.push(column);
            kept.push(at);
        }
    }
    let things = data::things(&values, "rows");
    let mut ids = std::collections::HashSet::with_capacity(things.len());
    let most_rows = MAX_ROWS.min(MAX_CELLS / table.columns.len().max(1));
    for (n, thing) in things.iter().enumerate() {
        if table.rows.len() >= most_rows {
            notes.push(format!(
                "Only the first {most_rows} rows are kept; {} more are left out.",
                things.len() - n
            ));
            break;
        }
        if let Some(row) = read_row(&table, &kept, thing, &mut notes) {
            if ids.insert(row.id.to_ascii_lowercase()) {
                if let Some(key) = part(thing).filter(|k| *k != row.id) {
                    table.kept.rows.insert(row.id.clone(), key);
                }
                table.rows.push(Arc::new(row));
            } else {
                notes.push(format!(
                    "The row \"{}\" is left out: another row has that ID (game code can't tell \
                     IDs apart by capitals alone).",
                    row.id
                ));
            }
        }
    }
    Ok(Read { table, notes })
}

/// A file's column, or none (with a note) when it can't be used.
fn read_column(table: &Table, thing: &data::Fixed, notes: &mut Vec<String>) -> Option<Column> {
    let name = data::field_text(thing, "name");
    if let Some(why) = table.column_problem(name, None) {
        notes.push(format!("The column \"{}\" is left out: {why}", shown(name)));
        return None;
    }
    let word = data::field_text(thing, "kind");
    let mut options = Vec::new();
    for option in data::field_texts(thing, "options") {
        if options.len() >= MAX_OPTIONS {
            notes.push(format!(
                "The column {name} keeps its first {MAX_OPTIONS} options; the rest are left out."
            ));
            break;
        }
        match option_problem(option, "An option") {
            None if !options
                .iter()
                .any(|o: &String| o.eq_ignore_ascii_case(option)) =>
            {
                options.push(option.to_string());
            }
            _ => notes.push(format!(
                "The option \"{}\" of the column {name} is left out: options are names game \
                 code can use, each once.",
                shown(option)
            )),
        }
    }
    if word == "choice" && options.is_empty() {
        options.push(super::NO_ROW.to_string());
    }
    let Some(kind) = ColumnKind::of_word(word, options) else {
        notes.push(format!(
            "The column {name} is left out: \"{}\" isn't a kind of column.",
            shown(word)
        ));
        return None;
    };
    let translated = data::field_bool(thing, "translated") && kind == ColumnKind::Text;
    Some(Column {
        name: name.to_string(),
        kind,
        translated,
    })
}

/// A file's row, its cells read by the columns kept (`kept`: where each
/// is in the file), or none (with a note) when it can't be used.
fn read_row(
    table: &Table,
    kept: &[usize],
    thing: &data::Fixed,
    notes: &mut Vec<String>,
) -> Option<Row> {
    let id = data::field_text(thing, "id");
    let problem = option_problem(id, "A row's ID").or_else(|| {
        (id == super::NO_ROW).then(|| "None is what game code has when a table has no rows.".into())
    });
    if let Some(why) = problem {
        notes.push(format!("The row \"{}\" is left out: {why}", shown(id)));
        return None;
    }
    let written = data::field_texts(thing, "cells");
    let mut cells = Vec::with_capacity(table.columns.len());
    for (column, at) in table.columns.iter().zip(kept) {
        let cell = match written.get(*at) {
            Some(t) => text::parse_cell(&column.kind, t).unwrap_or_else(|why| {
                notes.push(format!(
                    "Row {id}'s {} was \"{}\", which {why}; it's {} now.",
                    column.name,
                    shown(t),
                    text::cell_text(&column.kind.empty())
                ));
                column.kind.empty()
            }),
            None => column.kind.empty(),
        };
        cells.push(cell);
    }
    Some(Row {
        id: id.to_string(),
        cells,
    })
}

/// Whether `key` can be a part of a text key (a column's name is a
/// field's, so it may start with a small letter).
fn field_problem_free(key: &str) -> bool {
    !key.is_empty() && super::field_problem(key).is_none()
}

/// The start of `text`, for a note.
fn shown(text: &str) -> String {
    let mut out: String = text.chars().take(40).collect();
    if text.chars().count() > 40 {
        out.push('…');
    }
    out
}
