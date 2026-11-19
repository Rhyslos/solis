//! Data tables (stage 1): any game's data, rows and typed columns, in one
//! Lust data file (`items.table.lst`,
//! [decision 0081](../../../../docs/decisions/0081-what-solis-makes-is-lust.md)),
//! edited in the Table tab and read by game code with its spelling checked
//! ([decision 0128](../../../../docs/decisions/0128-data-tables.md)).
//!
//! A table has a key column, each row's ID (`IronSword`), which game code
//! names as a state's option (`Item.IronSword`), and up to [`MAX_COLUMNS`]
//! typed columns: whole numbers, numbers, text, yes or no, a choice of the
//! column's own options, one of the project's names (a sound, a picture, a
//! prefab…) and colours. Rows keep their order, which is the order game
//! code lists them in.
//!
//! The file is [`file`]'s; spreadsheet text (CSV, and the tab-separated
//! text copied between spreadsheets) is [`text`]'s.

pub mod file;
pub mod text;

use reflect::NameKind;
use std::sync::Arc;

/// The most rows a table may have.
pub const MAX_ROWS: usize = 100_000;

/// The most columns a table may have, besides its IDs.
pub const MAX_COLUMNS: usize = 64;

/// The most cells a table may have in all (rows × columns): a table of
/// 100,000 rows has room for 20 columns, one of 31,250 for all 64.
pub const MAX_CELLS: usize = 2_000_000;

/// The most options a choice column may have.
pub const MAX_OPTIONS: usize = 1_024;

/// The longest a cell's text may be, in characters.
pub const MAX_TEXT_CHARS: usize = 10_000;

/// The longest an ID, a column's name or an option may be, in characters.
pub const MAX_NAME_CHARS: usize = 60;

/// The field every row's class has for its ID (`row.id`), which no column
/// may be called.
pub const ID_FIELD: &str = "id";

/// What a project with no rows gives its key state as its one option
/// (Lust's states need one), so no row may have it.
pub const NO_ROW: &str = "None";

/// What a column holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColumnKind {
    /// Whole numbers (`int`).
    Whole,
    /// Numbers with a point (`float`).
    Number,
    /// Text (`string`).
    Text,
    /// Yes or no (`bool`).
    YesNo,
    /// One of the column's own options, a state of the table's own
    /// (`ItemRarity`).
    Choice(Vec<String>),
    /// One of the project's names of this kind (`Picture`).
    Name(NameKind),
    /// A colour.
    Colour,
}

impl ColumnKind {
    /// Every kind a new column can be, in the order the + Column menu
    /// lists them (a choice with one option to start).
    pub fn all() -> Vec<Self> {
        let mut all = vec![
            Self::Text,
            Self::Whole,
            Self::Number,
            Self::YesNo,
            Self::Choice(vec!["First".into()]),
            Self::Colour,
        ];
        all.extend(NameKind::ALL.into_iter().map(Self::Name));
        all
    }

    /// Its word in the file: `whole`, `choice`, or the project's names'
    /// state (`Picture`).
    pub fn word(&self) -> &'static str {
        match self {
            Self::Whole => "whole",
            Self::Number => "number",
            Self::Text => "text",
            Self::YesNo => "yesno",
            Self::Choice(_) => "choice",
            Self::Name(kind) => kind.state(),
            Self::Colour => "colour",
        }
    }

    /// The kind a file's word names (a choice with `options`).
    pub fn of_word(word: &str, options: Vec<String>) -> Option<Self> {
        Some(match word {
            "whole" => Self::Whole,
            "number" => Self::Number,
            "text" => Self::Text,
            "yesno" => Self::YesNo,
            "choice" => Self::Choice(options),
            "colour" => Self::Colour,
            state => Self::Name(NameKind::of_state(state)?),
        })
    }

    /// What it's called in the Table tab: `whole number`, `yes or no`,
    /// `picture`.
    pub fn label(&self) -> String {
        match self {
            Self::Whole => "whole number".into(),
            Self::Number => "number".into(),
            Self::Text => "text".into(),
            Self::YesNo => "yes or no".into(),
            Self::Choice(_) => "choice".into(),
            Self::Name(kind) => kind.state().to_ascii_lowercase(),
            Self::Colour => "colour".into(),
        }
    }

    /// The cell a new row has in a column of this kind.
    pub fn empty(&self) -> Cell {
        match self {
            Self::Whole => Cell::Whole(0),
            Self::Number => Cell::Number(0.0),
            Self::Text => Cell::Text(String::new()),
            Self::YesNo => Cell::YesNo(false),
            Self::Choice(options) => Cell::Option(options.first().cloned().unwrap_or_default()),
            Self::Name(_) => Cell::Option(String::new()),
            Self::Colour => Cell::Colour([1.0, 1.0, 1.0, 1.0]),
        }
    }
}

/// One column: its name (what game code calls it: `row.price`), what it
/// holds, and whether its text is translated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Column {
    /// Its name, as game code names it (`price`).
    pub name: String,
    /// What it holds.
    pub kind: ColumnKind,
    /// A text column marked for translating (the translations milestone
    /// reads it; until then it changes nothing).
    pub translated: bool,
}

impl Column {
    /// A column called `name` holding `kind`.
    pub fn new(name: impl Into<String>, kind: ColumnKind) -> Self {
        Self {
            name: name.into(),
            kind,
            translated: false,
        }
    }

    /// Its name as the Table tab heads it: `Price`, `Days to grow`.
    pub fn label(&self) -> String {
        label_of(&self.name)
    }
}

/// `daysToGrow` as `Days to grow`.
pub fn label_of(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    for (i, c) in name.chars().enumerate() {
        if i == 0 {
            out.push(c.to_ascii_uppercase());
        } else if c.is_ascii_uppercase() {
            out.push(' ');
            out.push(c.to_ascii_lowercase());
        } else if c == '_' {
            out.push(' ');
        } else {
            out.push(c);
        }
    }
    out
}

/// One cell's value.
#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    /// A whole number.
    Whole(i64),
    /// A number with a point (always finite).
    Number(f64),
    /// Text.
    Text(String),
    /// Yes or no.
    YesNo(bool),
    /// An option's name: a choice's (`Rare`), or one of the project's
    /// names (`Sprites.Apple`; empty for none chosen).
    Option(String),
    /// Red, green, blue and see-through, each from 0 to 1.
    Colour([f32; 4]),
}

/// One row: its ID and its cells, one for each column, in order.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    /// Its ID: what game code names it (`Item.IronSword`).
    pub id: String,
    /// Its cells, one a column.
    pub cells: Vec<Cell>,
}

/// A data table.
///
/// Rows are shared ([`Arc`]), so keeping the whole table as an undo step
/// costs a pointer a row, and a change copies only the rows it changes.
#[derive(Debug, Clone, Default)]
pub struct Table {
    /// What each row is, as game code names its state (`Item`), its
    /// class (`ItemRow`) and its columns' own options (`ItemRarity`).
    pub row_name: String,
    /// Its columns, in order.
    pub columns: Vec<Column>,
    /// Its rows, in order.
    pub rows: Vec<Arc<Row>>,
}

impl PartialEq for Table {
    fn eq(&self, other: &Self) -> bool {
        self.row_name == other.row_name
            && self.columns == other.columns
            && self.rows.len() == other.rows.len()
            && self
                .rows
                .iter()
                .zip(&other.rows)
                .all(|(a, b)| Arc::ptr_eq(a, b) || a == b)
    }
}

impl Table {
    /// An empty table whose rows are each a `row_name`.
    pub fn new(row_name: impl Into<String>) -> Self {
        Self {
            row_name: row_name.into(),
            ..Self::default()
        }
    }

    /// The row with this ID, and where it is.
    pub fn find(&self, id: &str) -> Option<(usize, &Row)> {
        self.rows
            .iter()
            .enumerate()
            .find(|(_, r)| r.id == id)
            .map(|(at, r)| (at, &**r))
    }

    /// The column called `name`, and where it is.
    pub fn column(&self, name: &str) -> Option<(usize, &Column)> {
        self.columns
            .iter()
            .enumerate()
            .find(|(_, c)| c.name == name)
    }

    /// Why a row can't be added: the table is full.
    pub fn row_room(&self) -> Result<(), String> {
        if self.rows.len() >= MAX_ROWS {
            return Err(format!(
                "A table can have at most {MAX_ROWS} rows: split it into two tables."
            ));
        }
        if (self.rows.len() + 1) * self.columns.len() > MAX_CELLS {
            return Err(format!(
                "A table can have at most {MAX_CELLS} cells in all: with {} columns, that's {} \
                 rows. Take out a column or split the table.",
                self.columns.len(),
                MAX_CELLS / self.columns.len().max(1)
            ));
        }
        Ok(())
    }

    /// Why a column can't be added: the table is full.
    pub fn column_room(&self) -> Result<(), String> {
        if self.columns.len() >= MAX_COLUMNS {
            return Err(format!(
                "A table can have at most {MAX_COLUMNS} columns: split it into two tables."
            ));
        }
        if self.rows.len() * (self.columns.len() + 1) > MAX_CELLS {
            return Err(format!(
                "A table can have at most {MAX_CELLS} cells in all, so with {} rows it can't \
                 have another column. Split the table.",
                self.rows.len()
            ));
        }
        Ok(())
    }

    /// Why `id` can't be a row's ID (the row at `renaming` keeps its own),
    /// in plain words; `None` when it can.
    pub fn id_problem(&self, id: &str, renaming: Option<usize>) -> Option<String> {
        if let Some(why) = option_problem(id, "A row's ID") {
            return Some(why);
        }
        if id == NO_ROW {
            return Some(
                "None is what game code has when a table has no rows: choose another ID.".into(),
            );
        }
        let (_, same) = self
            .rows
            .iter()
            .enumerate()
            .find(|(at, r)| Some(*at) != renaming && r.id.eq_ignore_ascii_case(id))?;
        Some(if same.id == id {
            format!("There's already a row called {id}.")
        } else {
            format!(
                "There's already a row called {}: game code can't tell IDs apart by capitals \
                 alone.",
                same.id
            )
        })
    }

    /// Why `name` can't be a column's name (the column at `renaming` keeps
    /// its own), in plain words; `None` when it can.
    pub fn column_problem(&self, name: &str, renaming: Option<usize>) -> Option<String> {
        if let Some(why) = field_problem(name) {
            return Some(why);
        }
        let (_, same) = self
            .columns
            .iter()
            .enumerate()
            .find(|(at, c)| Some(*at) != renaming && c.name.eq_ignore_ascii_case(name))?;
        Some(format!(
            "There's already a column called {}{}.",
            same.name,
            if same.name == name {
                ""
            } else {
                ": game code can't tell names apart by capitals alone"
            }
        ))
    }

    /// An ID no row has, from `base` (`Row`, `Row2`, `Row3`…).
    pub fn free_id(&self, base: &str) -> String {
        let base = if option_problem(base, "").is_some() || base == NO_ROW {
            "Row".to_string()
        } else {
            base.to_string()
        };
        std::iter::once(base.clone())
            .chain((2..).map(|n| format!("{base}{n}")))
            .find(|id| self.id_problem(id, None).is_none())
            .unwrap_or(base)
    }

    /// A column name no column has, from `base` (`column`, `column2`…).
    pub fn free_column(&self, base: &str) -> String {
        let base = if field_problem(base).is_some() {
            "column".to_string()
        } else {
            base.to_string()
        };
        std::iter::once(base.clone())
            .chain((2..).map(|n| format!("{base}{n}")))
            .find(|name| self.column_problem(name, None).is_none())
            .unwrap_or(base)
    }

    /// A new row with this ID, each cell its column's empty one.
    pub fn new_row(&self, id: String) -> Row {
        Row {
            id,
            cells: self.columns.iter().map(|c| c.kind.empty()).collect(),
        }
    }

    /// A number that changes whenever anything in the table does, the same
    /// on every computer (FNV-1a over its file's text): kept in recordings
    /// and save games, so playing one back with a changed table warns.
    pub fn content_hash(&self) -> u64 {
        fnv(file::write(self).as_bytes(), FNV_START)
    }
}

/// FNV-1a's starting number.
pub const FNV_START: u64 = 0xcbf2_9ce4_8422_2325;

/// `bytes` mixed into `hash` with FNV-1a.
pub fn fnv(bytes: &[u8], mut hash: u64) -> u64 {
    for b in bytes {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

/// Names Lust keeps for its own objects, which no option may have.
const LUST_OBJECTS: &[&str] = &[
    "Maths", "Files", "Time", "Json", "Web", "Random", "Rotation", "Colour", "Ease",
];

/// Lust's own words and Lua's, which no column may be called.
const RESERVED: &[&str] = &[
    "class",
    "object",
    "node",
    "assembly",
    "create",
    "has",
    "method",
    "hook",
    "return",
    "like",
    "new",
    "nothing",
    "schema",
    "route",
    "view",
    "style",
    "keep",
    "pass",
    "fetch",
    "get",
    "lua",
    "is",
    "then",
    "or",
    "either",
    "not",
    "true",
    "false",
    "loop",
    "stop",
    "skip",
    "in",
    "try",
    "error",
    "state",
    "save",
    "load",
    "saved",
    "end",
    "database",
    "int",
    "float",
    "string",
    "bool",
    "array",
    "cMap",
    "map",
    "native",
    "component",
    "check",
    "when",
    "lock",
    "print",
    "wait",
    "ask",
    "rotation",
    "colour",
    "vec2",
    "vec3",
    "entity",
    "this",
    "me",
    "and",
    "break",
    "do",
    "else",
    "elseif",
    "for",
    "function",
    "goto",
    "if",
    "local",
    "nil",
    "repeat",
    "until",
    "while",
    "continue",
    "self",
];

/// Why `name` can't be an option of a state (a row's ID, a choice's
/// option, a row's name), with `what` naming it ("A row's ID"); `None`
/// when it can.
pub fn option_problem(name: &str, what: &str) -> Option<String> {
    let Some(first) = name.chars().next() else {
        return Some(format!("{what} can't be empty."));
    };
    if name.chars().count() > MAX_NAME_CHARS {
        return Some(format!(
            "{what} can be at most {MAX_NAME_CHARS} characters."
        ));
    }
    if !first.is_ascii_uppercase() {
        return Some(format!(
            "{what} starts with a capital letter, as IronSword does: game code names it so. Try \
             {}.",
            suggestion(name)
        ));
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Some(format!(
            "{what} holds only letters, digits and _, with no spaces: game code names it so. \
             Try {}.",
            suggestion(name)
        ));
    }
    let letters: Vec<char> = name.chars().filter(char::is_ascii_alphabetic).collect();
    if letters.len() >= 2 && letters.iter().all(char::is_ascii_uppercase) {
        return Some(format!(
            "{what} can't be all capitals, which Lust keeps for its own: try {}.",
            suggestion(name)
        ));
    }
    if LUST_OBJECTS.contains(&name) {
        return Some(format!(
            "{name} is one of Lust's own names: choose another."
        ));
    }
    None
}

/// Why `name` can't be a column's (a field of each row's class), in
/// plain words; `None` when it can.
pub fn field_problem(name: &str) -> Option<String> {
    let Some(first) = name.chars().next() else {
        return Some("A column needs a name.".into());
    };
    if name.chars().count() > MAX_NAME_CHARS {
        return Some(format!(
            "A column's name can be at most {MAX_NAME_CHARS} characters."
        ));
    }
    if !first.is_ascii_lowercase() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Some(format!(
            "A column's name starts with a small letter and holds only letters, digits and _, \
             as daysToGrow does: game code reads it as row.daysToGrow. Try {}.",
            field_suggestion(name)
        ));
    }
    if name == ID_FIELD {
        return Some("id is the row's ID, which every table has: choose another name.".into());
    }
    if RESERVED.contains(&name) {
        return Some(format!(
            "{name} is a word Lust or Lua keeps for itself: choose another name."
        ));
    }
    None
}

/// A name like `name` an option can have: its words joined, each starting
/// with a capital (`iron sword` is `IronSword`).
pub fn suggestion(name: &str) -> String {
    let words = name
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .filter(|w| !w.is_empty());
    let mut out = String::new();
    for word in words {
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            out.push(first.to_ascii_uppercase());
            let rest: String = chars.collect();
            if word
                .chars()
                .filter(char::is_ascii_alphabetic)
                .all(|c| c.is_ascii_uppercase())
            {
                out.push_str(&rest.to_ascii_lowercase());
            } else {
                out.push_str(&rest);
            }
        }
    }
    if !out.starts_with(|c: char| c.is_ascii_uppercase()) {
        out.insert_str(0, "Row");
    }
    out.chars().take(MAX_NAME_CHARS).collect()
}

/// A name like `name` a column can have: `Days to grow` is `daysToGrow`.
pub fn field_suggestion(name: &str) -> String {
    let joined = suggestion(name);
    let joined = joined
        .strip_prefix("Row")
        .filter(|r| !r.is_empty())
        .unwrap_or(&joined);
    let mut chars = joined.chars();
    let out = match chars.next() {
        Some(first) if first.is_ascii_alphabetic() => {
            first.to_ascii_lowercase().to_string() + chars.as_str()
        }
        _ => format!("column{joined}"),
    };
    if field_problem(&out).is_some() {
        format!("{out}Value")
    } else {
        out
    }
}

/// What each row is called, from the table's file name: `items` gives
/// `Item` (a final s dropped, ies made y), `weather` gives `WeatherRow`.
pub fn row_name_for(stem: &str) -> String {
    let base = suggestion(stem);
    let base = base
        .strip_prefix("Row")
        .filter(|r| !r.is_empty())
        .map_or(base.clone(), str::to_string);
    let single = if let Some(stem) = base.strip_suffix("ies").filter(|s| !s.is_empty()) {
        format!("{stem}y")
    } else if base.ends_with('s') && !base.ends_with("ss") && base.len() > 1 {
        base[..base.len() - 1].to_string()
    } else {
        format!("{base}Row")
    };
    if option_problem(&single, "").is_some() {
        "Row".into()
    } else {
        single
    }
}

/// What game code calls a table, from its file name: `items` gives
/// `Items`, `shop prices` gives `ShopPrices`; `None` when it can't be a
/// Lust name.
pub fn object_name_for(stem: &str) -> Option<String> {
    let name = suggestion(stem);
    if name.starts_with("Row") && !stem.starts_with(['r', 'R']) {
        return None;
    }
    option_problem(&name, "").is_none().then_some(name)
}

#[cfg(test)]
mod tests;
