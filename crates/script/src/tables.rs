//! Data tables while the game runs (stage 1, data tables;
//! [decision 0128](../../../docs/decisions/0128-data-tables.md)): the
//! project's tables, read-only, by the name game code calls each
//! (`Items`).
//!
//! Tables are constants, not game state: game code reads them and never
//! changes them, so they're in no checksum and no save. Only their
//! [`Tables::hash`] is kept, in recordings and saves, so playing one back
//! with a changed table warns.
//!
//! The editor puts them in each phase's [`crate::WorldView`]; game code
//! reaches them through its host (Rust: [`crate::Host::tables`]) or the
//! natives each table's Lust file declares (`Items:row(Item.IronSword)`,
//! Lua's `solis.Items:row("IronSword")`). Those are lent for each table
//! from functions each Luau makes once for the tables it was given
//! ([`Lent`]): a row is made into Lua the first time it's asked for and
//! kept, so asking again is a lookup in a Lua table, never a trip into
//! Rust. New tables (a table saved while the game runs) are new
//! functions, so the next phase reads the new rows.

use level::table::{Cell, Row, Table};
use mlua::{Function, Lua, Table as LuaTable, Value as LuaValue};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

/// One table, with its rows found by ID and its columns by name.
#[derive(Debug)]
pub struct TableRef {
    /// The table.
    pub table: Arc<Table>,
    ids: HashMap<String, usize>,
    columns: HashMap<String, usize>,
}

impl TableRef {
    fn new(table: Arc<Table>) -> Self {
        let ids = table
            .rows
            .iter()
            .enumerate()
            .map(|(at, r)| (r.id.clone(), at))
            .collect();
        let columns = table
            .columns
            .iter()
            .enumerate()
            .map(|(at, c)| (c.name.clone(), at))
            .collect();
        Self {
            table,
            ids,
            columns,
        }
    }

    /// The row with this ID.
    pub fn row(&self, id: &str) -> Option<RowRef<'_>> {
        let at = *self.ids.get(id)?;
        Some(RowRef {
            row: &self.table.rows[at],
            columns: &self.columns,
        })
    }

    /// Every row, in the table's order.
    pub fn rows(&self) -> impl Iterator<Item = RowRef<'_>> {
        self.table.rows.iter().map(|row| RowRef {
            row,
            columns: &self.columns,
        })
    }

    /// The ID nearest `id` by spelling, for a message.
    fn nearest(&self, id: &str) -> Option<&str> {
        let lower = id.to_ascii_lowercase();
        self.table
            .rows
            .iter()
            .take(20_000)
            .map(|r| (distance(&lower, &r.id.to_ascii_lowercase()), r.id.as_str()))
            .filter(|(d, _)| *d <= 3.max(id.len() / 3))
            .min_by_key(|(d, _)| *d)
            .map(|(_, id)| id)
    }
}

/// How many letters must change to make `a` into `b`.
fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut last: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut now = vec![i + 1; b.len() + 1];
        for (j, cb) in b.iter().enumerate() {
            let swap = last[j] + usize::from(ca != *cb);
            now[j + 1] = swap.min(last[j + 1] + 1).min(now[j] + 1);
        }
        last = now;
    }
    last[b.len()]
}

/// One row, its cells read by column name.
#[derive(Debug, Clone, Copy)]
pub struct RowRef<'a> {
    /// The row.
    pub row: &'a Row,
    columns: &'a HashMap<String, usize>,
}

impl<'a> RowRef<'a> {
    /// Its ID.
    pub fn id(&self) -> &'a str {
        &self.row.id
    }

    /// The cell in the column called `column`.
    pub fn cell(&self, column: &str) -> Option<&'a Cell> {
        self.row.cells.get(*self.columns.get(column)?)
    }

    /// A whole number column's value.
    pub fn whole(&self, column: &str) -> Option<i64> {
        match self.cell(column)? {
            Cell::Whole(n) => Some(*n),
            _ => None,
        }
    }

    /// A number column's value (a whole number's too).
    pub fn number(&self, column: &str) -> Option<f64> {
        match self.cell(column)? {
            Cell::Number(x) => Some(*x),
            Cell::Whole(n) => Some(*n as f64),
            _ => None,
        }
    }

    /// A text column's value, or a choice's or a name's option.
    pub fn text(&self, column: &str) -> Option<&'a str> {
        match self.cell(column)? {
            Cell::Text(t) | Cell::Option(t) => Some(t),
            _ => None,
        }
    }

    /// A yes-or-no column's value.
    pub fn yes(&self, column: &str) -> Option<bool> {
        match self.cell(column)? {
            Cell::YesNo(b) => Some(*b),
            _ => None,
        }
    }

    /// A colour column's value: red, green, blue and see-through.
    pub fn colour(&self, column: &str) -> Option<[f32; 4]> {
        match self.cell(column)? {
            Cell::Colour(c) => Some(*c),
            _ => None,
        }
    }
}

/// The project's tables while the game runs, by the name game code calls
/// each. Cheap to copy: every copy shares them.
#[derive(Debug, Clone, Default)]
pub struct Tables {
    inner: Arc<Inner>,
}

#[derive(Debug, Default)]
struct Inner {
    by_name: BTreeMap<String, Arc<TableRef>>,
    hash: u64,
}

impl Tables {
    /// The tables in `list` (each its path without `.table.lst`, and the
    /// table), each by the name game code calls it (`data/items` is
    /// `Items`). One whose name can't be a Lust name, or is taken by an
    /// earlier one or by `taken` (the engine's own objects), is left out:
    /// its Lust file isn't written either (`table_code::table_files`).
    pub fn new(list: &[(String, Arc<Table>)], taken: &[&str]) -> Self {
        let mut by_name = BTreeMap::new();
        let mut hash = level::table::FNV_START;
        for (path, table) in list {
            let Some(named) = crate::table_code::named(path, &table.row_name) else {
                continue;
            };
            let clash = |n: &str| {
                taken.iter().any(|t| t.eq_ignore_ascii_case(n))
                    || by_name.keys().any(|k: &String| k.eq_ignore_ascii_case(n))
            };
            if clash(&named.object) {
                continue;
            }
            hash = level::table::fnv(named.object.as_bytes(), hash);
            hash = level::table::fnv(&table.content_hash().to_le_bytes(), hash);
            by_name.insert(named.object, Arc::new(TableRef::new(Arc::clone(table))));
        }
        let hash = if by_name.is_empty() { 0 } else { hash };
        Self {
            inner: Arc::new(Inner { by_name, hash }),
        }
    }

    /// The table game code calls `name` (`Items`).
    pub fn get(&self, name: &str) -> Option<&TableRef> {
        self.inner.by_name.get(name).map(|t| &**t)
    }

    /// Every table's name, in order.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.inner.by_name.keys().map(String::as_str)
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.inner.by_name.is_empty()
    }

    /// A number that changes whenever any table does (0 when there are
    /// none), the same on every computer.
    pub fn hash(&self) -> u64 {
        self.inner.hash
    }

    /// Whether `other` is these very tables (not only equal ones).
    fn same(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

/// Makes a table's two natives from the function Rust builds a row with
/// and the rows' IDs in order: `row(_, id)` keeps each row made, and
/// `rows(_)` gives a new list of them all each time (so game code may
/// change its list). Made once, before any game code runs.
pub(crate) const MAKE: &str = r#"
local error, type, create, move = error, type, table.create, table.move
return function(name, build, ids)
  local cache = {}
  local list
  local function row(_, id)
    local r = cache[id]
    if r ~= nil then return r end
    if type(id) ~= "string" then
      error(name .. ":row(id) wants one of its rows' IDs, such as " .. tostring(ids[1] or "Apple"), 2)
    end
    local made, why = build(id)
    if made == nil then error(why, 2) end
    cache[id] = made
    return made
  end
  local function rows(_)
    if list == nil then
      local all = create(#ids)
      for i = 1, #ids do all[i] = row(nil, ids[i]) end
      list = all
    end
    return move(list, 1, #list, 1, create(#list))
  end
  return row, rows
end
"#;

/// What a Luau lends for the tables it was last given: the tables, and
/// their natives by key (`Items_row`, `Items_rows`).
#[derive(Default)]
pub(crate) struct Lent {
    from: Tables,
    natives: Option<LuaTable>,
}

/// A cell as Lust and Lua read it.
fn cell_value(lua: &Lua, cell: &Cell) -> mlua::Result<LuaValue> {
    Ok(match cell {
        Cell::Whole(n) => LuaValue::Number(*n as f64),
        Cell::Number(x) => LuaValue::Number(*x),
        Cell::Text(t) | Cell::Option(t) => LuaValue::String(lua.create_string(t)?),
        Cell::YesNo(b) => LuaValue::Boolean(*b),
        Cell::Colour(c) => LuaValue::Table(crate::convert::numbers_table(
            lua,
            &["r", "g", "b", "a"],
            c,
        )?),
    })
}

/// The function that makes `name`'s rows into Lua: a row by ID, or nil
/// and why there's none.
fn builder(lua: &Lua, name: &str, table: Arc<TableRef>) -> mlua::Result<Function> {
    let name = name.to_string();
    lua.create_function(move |lua, id: mlua::LuaString| {
        let id = id.to_str()?;
        let Some(row) = table.row(&id) else {
            let near = table
                .nearest(&id)
                .map_or_else(String::new, |n| format!(": did you mean {n}?"));
            let why = format!("{name} has no row {id}{near}");
            return Ok((LuaValue::Nil, Some(why)));
        };
        let made = lua.create_table_with_capacity(0, table.table.columns.len() + 1)?;
        made.raw_set(level::table::ID_FIELD, row.id())?;
        for (column, cell) in table.table.columns.iter().zip(&row.row.cells) {
            made.raw_set(column.name.as_str(), cell_value(lua, cell)?)?;
        }
        Ok((LuaValue::Table(made), None))
    })
}

impl Lent {
    /// Puts each table's natives into `natives` (the ones lent while game
    /// code runs), made again first when `tables` aren't the ones they
    /// were made for. `make` is [`MAKE`], and `groups` tells Lua's `solis`
    /// the tables' names.
    pub(crate) fn lend(
        &mut self,
        lua: &Lua,
        make: &Function,
        groups: &Function,
        tables: Tables,
        natives: &LuaTable,
    ) -> mlua::Result<()> {
        if !self.from.same(&tables) || (self.natives.is_none() && !tables.is_empty()) {
            let made = lua.create_table()?;
            let names = lua.create_sequence_from(tables.names())?;
            for (name, table) in &tables.inner.by_name {
                let ids = lua.create_sequence_from(table.table.rows.iter().map(|r| r.id.as_str()))?;
                let (row, rows): (Function, Function) =
                    make.call((name.as_str(), builder(lua, name, Arc::clone(table))?, ids))?;
                made.raw_set(format!("{name}_row"), row)?;
                made.raw_set(format!("{name}_rows"), rows)?;
            }
            groups.call::<()>(names)?;
            self.natives = Some(made);
            self.from = tables;
        }
        if let Some(made) = &self.natives {
            for pair in made.pairs::<LuaValue, LuaValue>() {
                let (key, native) = pair?;
                natives.raw_set(key, native)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "tables_tests.rs"]
mod tests;
