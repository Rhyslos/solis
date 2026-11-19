//! Each data table's own Lust file (stage 1, data tables;
//! [decision 0128](../../../docs/decisions/0128-data-tables.md)): like the
//! animators' and tile maps' (decisions 0078 and 0079), the game API
//! guidelines' rule 4. `data/items.table.lst` gives
//! `solis/tables/data/items.lst`: a state of the rows' IDs (`Item`), a
//! state for each choice column's options (`ItemRarity`), a class of one
//! row whose fields can be read but never changed (`ItemRow`), and an
//! object for the table itself (`Items`), with `row(Item which)` and
//! `rows()`.
//!
//! So `Items:row(Item.IronSword).price` is checked before the game runs: a
//! misspelt row is an option `Item` doesn't have, and a misspelt column a
//! field `ItemRow` doesn't have. The object's methods are natives whose
//! bodies Solis lends for each table it reads (`crate::tables`).

use crate::project_names::NO_OPTION;
use level::table::{Column, ColumnKind, ID_FIELD, Table};
use reflect::NameKind;
use std::collections::BTreeMap;
use std::sync::Arc;

/// The folder inside the engine's that each table's file is written into
/// (`solis/tables/data/items.lst`).
pub const FOLDER: &str = "tables";

/// One table as the editor read it, for its file.
#[derive(Debug, Clone, PartialEq)]
pub struct TableNames {
    /// Its path in the project's files, without `.table.lst`
    /// (`data/items`).
    pub path: String,
    /// The table.
    pub table: Arc<Table>,
}

/// The names game code reaches a table by: its object (`Items`), its
/// rows' state (`Item`) and class (`ItemRow`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Named {
    /// The table itself: `Items`.
    pub object: String,
    /// Its rows' IDs: `Item`.
    pub state: String,
    /// One row: `ItemRow`.
    pub class: String,
}

/// The names of the table at `path` (without `.table.lst`) whose rows are
/// each a `row_name`; none when its file name can't be a Lust name.
pub fn named(path: &str, row_name: &str) -> Option<Named> {
    let stem = path.rsplit('/').next()?;
    Some(Named {
        object: level::table::object_name_for(stem)?,
        state: row_name.to_string(),
        class: format!("{row_name}Row"),
    })
}

/// The state a choice column's options are, in a table whose rows are
/// each a `row_name`: `ItemRarity`.
pub fn choice_state(row_name: &str, column: &str) -> String {
    let mut upper = column.to_string();
    if let Some(first) = upper.get_mut(..1) {
        first.make_ascii_uppercase();
    }
    format!("{row_name}{upper}")
}

/// A list of options as Lust writes it, or the one standing for none.
fn listed<'a>(options: impl Iterator<Item = &'a str>) -> String {
    let all: Vec<&str> = options.collect();
    if all.is_empty() {
        NO_OPTION.to_string()
    } else {
        all.join(", ")
    }
}

/// The Lust type of a column's field, and its starting value.
fn field_of(
    column: &Column,
    row_name: &str,
    first_name: &dyn Fn(NameKind) -> String,
) -> (String, String) {
    match &column.kind {
        ColumnKind::Whole => ("int".into(), "0".into()),
        ColumnKind::Number => ("float".into(), "0.0".into()),
        ColumnKind::Text => ("string".into(), "\"\"".into()),
        ColumnKind::YesNo => ("bool".into(), "false".into()),
        ColumnKind::Choice(options) => {
            let state = choice_state(row_name, &column.name);
            let first = options.first().map_or(NO_OPTION, String::as_str);
            (state.clone(), format!("{state}.{first}"))
        }
        ColumnKind::Name(kind) => (
            kind.state().to_string(),
            format!("{}.{}", kind.state(), first_name(*kind)),
        ),
        ColumnKind::Colour => ("colour".into(), "[1, 1, 1]".into()),
    }
}

/// The Lust file for one table (its path and text; none when the table
/// can't be named at all), and notes on anything game code can't reach.
/// `first_name` gives each kind of the project's names its first option
/// (or `None` when it has none), a row's field's starting value.
pub fn table_file(
    made_by: &str,
    folder: &str,
    names: &TableNames,
    first_name: &dyn Fn(NameKind) -> String,
) -> (Option<(String, String)>, Vec<String>) {
    let table = &names.table;
    let path = &names.path;
    let Some(named) = named(path, &table.row_name) else {
        return (
            None,
            vec![format!(
                "The table {path} can't be reached from game code: its file name must start with \
                 a letter and hold only letters, digits and spaces. Rename it to use it from code."
            )],
        );
    };
    let Named {
        object,
        state,
        class,
    } = &named;
    let wanted: Vec<&str> = NameKind::ALL
        .into_iter()
        .filter(|k| table.columns.iter().any(|c| c.kind == ColumnKind::Name(*k)))
        .map(NameKind::state)
        .collect();
    let mut text = format!(
        "-- Made by {made_by} from {path}.table.lst: its rows as game code reads them. It's\n\
         -- written again whenever the table changes, so changes made here don't last.\n\n"
    );
    if !wanted.is_empty() {
        text.push_str(&format!(
            "fetch \"{folder}/project\":[{}];\n\n",
            wanted.join(", ")
        ));
    }
    text.push_str(&format!(
        "-- The table's rows, by their IDs.\nstate {state} = [{}];\n\n",
        listed(table.rows.iter().map(|r| r.id.as_str()))
    ));
    let mut passed = vec![state.clone()];
    for column in &table.columns {
        if let ColumnKind::Choice(options) = &column.kind {
            let choice = choice_state(state, &column.name);
            text.push_str(&format!(
                "-- The options of the column {}.\nstate {choice} = [{}];\n\n",
                column.name,
                listed(options.iter().map(String::as_str))
            ));
            passed.push(choice);
        }
    }
    let first_row = table.rows.first().map_or(NO_OPTION, |r| r.id.as_str());
    text.push_str(&format!(
        "-- One row of the table. Its values can be read, never changed.\nclass {class} {{\n    has {{\n        lock {state} {ID_FIELD} = {state}.{first_row};\n"
    ));
    for column in &table.columns {
        let (kind, start) = field_of(column, state, first_name);
        text.push_str(&format!("        lock {kind} {} = {start};\n", column.name));
    }
    text.push_str("    }\n}\n\n");
    text.push_str(&format!(
        "-- The table itself.\nobject {object} {{\n\
         \x20   -- The row with this ID.\n    method {class} row({state} which) native;\n\n\
         \x20   -- Every row, in the table's order.\n    method {class} array rows() native;\n}}\n\n"
    ));
    passed.push(class.clone());
    passed.push(object.clone());
    text.push_str(&format!("pass {};\nend\n", passed.join(", ")));
    let file = format!("{folder}/{FOLDER}/{path}.lst");
    (Some((file, text)), Vec::new())
}

/// Every table's file, each table's names once: a second table whose
/// object, state or class another has already is left out with a note
/// (two tables called `items` in different folders), and so is one whose
/// names are taken by the engine's (`taken`: `World`, `Input`…).
pub fn table_files(
    made_by: &str,
    folder: &str,
    tables: &[TableNames],
    taken: &[&str],
    first_name: &dyn Fn(NameKind) -> String,
) -> (Vec<(String, String)>, Vec<String>) {
    let mut files = Vec::new();
    let mut notes = Vec::new();
    let mut used: BTreeMap<String, String> = taken
        .iter()
        .map(|t| (t.to_ascii_lowercase(), format!("the engine's {t}")))
        .collect();
    for names in tables {
        let Some(named) = named(&names.path, &names.table.row_name) else {
            notes.extend(table_file(made_by, folder, names, first_name).1);
            continue;
        };
        let mut wanted = vec![
            named.object.clone(),
            named.state.clone(),
            named.class.clone(),
        ];
        wanted.extend(names.table.columns.iter().filter_map(|c| {
            matches!(c.kind, ColumnKind::Choice(_)).then(|| choice_state(&named.state, &c.name))
        }));
        if let Some((clash, owner)) = wanted
            .iter()
            .find_map(|w| used.get(&w.to_ascii_lowercase()).map(|o| (w, o)))
        {
            notes.push(format!(
                "The table {} can't be reached from game code: {clash} is {owner}'s name \
                 already. Rename the table, or what its rows are called.",
                names.path
            ));
            continue;
        }
        for w in wanted {
            used.insert(w.to_ascii_lowercase(), format!("the table {}", names.path));
        }
        let (file, said) = table_file(made_by, folder, names, first_name);
        notes.extend(said);
        files.extend(file);
    }
    (files, notes)
}

#[cfg(test)]
#[path = "table_code_tests.rs"]
mod tests;
