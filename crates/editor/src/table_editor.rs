//! A data table being changed in the Table tab (stage 1, data tables;
//! [decision 0128](../../../docs/decisions/0128-data-tables.md)): its rows
//! and columns added, renamed, deleted and moved, its cells set, cleared,
//! pasted and taken in from CSV. Every change is one undo step and keeps
//! the file whole (`FileHistory`, as every editor of a file of its own:
//! a table's rows are shared, so a step costs a pointer a row).
//!
//! What's picked (the cursor and the block it spans), the cell being
//! typed in, the search and the column sorted by are the tab's own, never
//! saved: sorting is for looking, and the file keeps its own order.

use crate::file_history::FileHistory;
use level::table::text as sheet;
use level::table::{Cell, Column, ColumnKind, Table};
use std::sync::Arc;

/// The most undo steps kept.
const MAX_UNDO: usize = 100;

/// A cell's place: its row in the file and its place in the row (0 is
/// the ID, 1 the first column).
pub(crate) type Place = (usize, usize);

/// A cell being typed in: where, and the text as typed so far.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Typing {
    /// The cell.
    pub(crate) at: Place,
    /// Its text now.
    pub(crate) text: String,
    /// Whether the text box has been given the keys yet.
    pub(crate) focused: bool,
}

/// The column the rows are shown sorted by, and which way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Sort {
    /// The place sorted by (0 is the IDs).
    pub(crate) place: usize,
    /// Lowest first.
    pub(crate) rising: bool,
}

/// A data table being changed, with its undo steps and the tab's view.
#[derive(Debug, Clone)]
pub(crate) struct TableEditor {
    /// Its path inside the project's files.
    pub(crate) path: String,
    /// The table as it is now.
    pub(crate) file: Table,
    history: FileHistory<Table>,
    /// The cursor: the cell the keys act on.
    pub(crate) cursor: Option<Place>,
    /// The other corner of the block picked with the cursor (Shift).
    pub(crate) anchor: Option<Place>,
    /// The cell being typed in.
    pub(crate) typing: Option<Typing>,
    /// Shows only rows with a cell holding this (any capitals).
    pub(crate) search: String,
    /// The column shown sorted by, for looking only.
    pub(crate) sort: Option<Sort>,
    /// The rows shown, in the order shown, worked out again when the
    /// table, the search or the sort changes.
    shown: Vec<usize>,
    /// What the shown rows were worked out from: the table's revision,
    /// the search and the sort.
    shown_from: Option<(u64, String, Option<Sort>)>,
    /// Goes up with every change to the table (undone, redone, taken in).
    revision: u64,
    /// What the last change had to say (a paste's or an import's fields
    /// that couldn't be taken), for the Console.
    pub(crate) said: Vec<String>,
    /// The tab's scroll should bring the cursor into view.
    pub(crate) reveal: bool,
    /// Where the grid's top left corner was drawn last (its headings'),
    /// as scrolled.
    pub(crate) drawn_at: Option<(f32, f32)>,
}

impl TableEditor {
    /// Opens `file`, read from `path`.
    pub(crate) fn new(path: String, file: Table) -> Self {
        let cursor = (!file.rows.is_empty()).then_some((0, 0));
        Self {
            path,
            file,
            history: FileHistory::new(MAX_UNDO),
            cursor,
            anchor: None,
            typing: None,
            search: String::new(),
            sort: None,
            shown: Vec::new(),
            shown_from: None,
            revision: 0,
            said: Vec::new(),
            reveal: false,
            drawn_at: None,
        }
    }

    /// How many places a row has: its ID and each column.
    pub(crate) fn places(&self) -> usize {
        self.file.columns.len() + 1
    }

    /// The rows shown, in the order shown (the search and sort applied).
    pub(crate) fn shown(&mut self) -> &[usize] {
        let now = (self.revision, self.search.clone(), self.sort);
        if self.shown_from.as_ref() != Some(&now) {
            self.shown = shown_rows(&self.file, &self.search, self.sort);
            self.shown_from = Some(now);
        }
        &self.shown
    }

    /// Changes the table as one undo step; whether it changed. What's
    /// picked is kept within the table afterwards.
    pub(crate) fn change(&mut self, change: impl FnOnce(&mut Table) -> bool) -> bool {
        let changed = self.history.change(&mut self.file, change);
        if changed {
            self.keep_picks();
        }
        changed
    }

    /// Undoes the last change. Whether there was one.
    pub(crate) fn undo(&mut self) -> bool {
        let undone = self.history.undo(&mut self.file);
        if undone {
            self.typing = None;
            self.keep_picks();
        }
        undone
    }

    /// Redoes the last change undone. Whether there was one.
    pub(crate) fn redo(&mut self) -> bool {
        let redone = self.history.redo(&mut self.file);
        if redone {
            self.typing = None;
            self.keep_picks();
        }
        redone
    }

    /// Keeps the cursor and the block inside the table, after a change.
    fn keep_picks(&mut self) {
        self.revision += 1;
        let (rows, places) = (self.file.rows.len(), self.places());
        let inside = |p: Option<Place>| {
            p.filter(|_| rows > 0)
                .map(|(r, c)| (r.min(rows - 1), c.min(places - 1)))
        };
        self.cursor = inside(self.cursor);
        self.anchor = inside(self.anchor);
        if self
            .typing
            .as_ref()
            .is_some_and(|t| t.at.0 >= rows || t.at.1 >= places)
        {
            self.typing = None;
        }
    }

    /// The block picked: its rows and places, each low to high (the
    /// cursor alone when nothing more is).
    pub(crate) fn block(
        &self,
    ) -> Option<(
        std::ops::RangeInclusive<usize>,
        std::ops::RangeInclusive<usize>,
    )> {
        let (r, c) = self.cursor?;
        let (ar, ac) = self.anchor.unwrap_or((r, c));
        Some((r.min(ar)..=r.max(ar), c.min(ac)..=c.max(ac)))
    }

    /// Whether the cell at `at` is in the block picked.
    pub(crate) fn picked(&self, at: Place) -> bool {
        self.block()
            .is_some_and(|(rows, places)| rows.contains(&at.0) && places.contains(&at.1))
    }

    /// Adds a row below the cursor's (or at the end), its ID a free one
    /// from the rows' name, and puts the cursor on its ID.
    pub(crate) fn add_row(&mut self) -> Result<(), String> {
        self.file.row_room()?;
        let at = self.cursor.map_or(self.file.rows.len(), |(r, _)| r + 1);
        let added = self.change(|table| {
            let id = table.free_id(&table.row_name.clone());
            let row = table.new_row(id);
            table.rows.insert(at.min(table.rows.len()), Arc::new(row));
            true
        });
        if added {
            self.pick((at.min(self.file.rows.len() - 1), 0));
        }
        Ok(())
    }

    /// Adds a column of `kind` after the cursor's (or at the end), named
    /// from its kind, and puts the cursor on it.
    pub(crate) fn add_column(&mut self, kind: ColumnKind) -> Result<(), String> {
        self.file.column_room()?;
        let at = self.cursor.map_or(self.file.columns.len(), |(_, c)| {
            c.min(self.file.columns.len())
        });
        let added = self.change(|table| {
            let base = match &kind {
                ColumnKind::Name(k) => k.state().to_ascii_lowercase(),
                ColumnKind::Whole | ColumnKind::Number => "amount".into(),
                ColumnKind::YesNo => "on".into(),
                ColumnKind::Choice(_) => "kind".into(),
                ColumnKind::Colour => "tint".into(),
                ColumnKind::Text => "words".into(),
            };
            let name = table.free_column(&base);
            table.columns.insert(at, Column::new(name, kind.clone()));
            for row in &mut table.rows {
                Arc::make_mut(row).cells.insert(at, kind.empty());
            }
            true
        });
        if added {
            let row = self.cursor.map_or(0, |(r, _)| r);
            self.cursor = Some((row, at + 1));
            self.anchor = None;
        }
        Ok(())
    }

    /// Renames the column at `column` (from 0); or why it can't be.
    pub(crate) fn rename_column(&mut self, column: usize, name: &str) -> Result<(), String> {
        if let Some(why) = self.file.column_problem(name, Some(column)) {
            return Err(why);
        }
        self.change(|table| match table.columns.get_mut(column) {
            Some(c) => {
                name.clone_into(&mut c.name);
                true
            }
            None => false,
        });
        Ok(())
    }

    /// Takes the column at `column` out, with its cells.
    pub(crate) fn delete_column(&mut self, column: usize) -> bool {
        self.change(|table| {
            if column >= table.columns.len() {
                return false;
            }
            table.columns.remove(column);
            for row in &mut table.rows {
                Arc::make_mut(row).cells.remove(column);
            }
            true
        })
    }

    /// Moves the column at `from` to `to` (both from 0), with its cells.
    pub(crate) fn move_column(&mut self, from: usize, to: usize) -> bool {
        let moved = self.change(|table| {
            let n = table.columns.len();
            if from >= n || to >= n || from == to {
                return false;
            }
            let column = table.columns.remove(from);
            table.columns.insert(to, column);
            for row in &mut table.rows {
                let cells = &mut Arc::make_mut(row).cells;
                let cell = cells.remove(from);
                cells.insert(to, cell);
            }
            true
        });
        if moved
            && let Some((r, c)) = self.cursor
            && c == from + 1
        {
            self.cursor = Some((r, to + 1));
        }
        moved
    }

    /// Marks a text column translated or not.
    pub(crate) fn set_translated(&mut self, column: usize, on: bool) -> bool {
        self.change(|table| match table.columns.get_mut(column) {
            Some(c) if c.kind == ColumnKind::Text => {
                c.translated = on;
                true
            }
            _ => false,
        })
    }

    /// Changes what a column holds, each cell turned into the new kind
    /// by its text (one that can't be is the kind's empty value).
    pub(crate) fn set_kind(&mut self, column: usize, kind: ColumnKind) -> bool {
        self.change(|table| {
            let Some(c) = table.columns.get_mut(column) else {
                return false;
            };
            c.kind = kind.clone();
            if kind != ColumnKind::Text {
                c.translated = false;
            }
            for row in &mut table.rows {
                let cell = &mut Arc::make_mut(row).cells[column];
                *cell = sheet::parse_cell(&kind, &sheet::cell_text(cell))
                    .unwrap_or_else(|_| kind.empty());
            }
            true
        })
    }

    /// Sets a choice column's options: cells holding an option renamed
    /// (same place in the list) follow it; cells holding one taken out
    /// become the first.
    pub(crate) fn set_options(
        &mut self,
        column: usize,
        options: Vec<String>,
    ) -> Result<(), String> {
        if options.is_empty() {
            return Err("A choice needs at least one option.".into());
        }
        for (n, option) in options.iter().enumerate() {
            if let Some(why) = level::table::option_problem(option, "An option") {
                return Err(why);
            }
            if options[..n].iter().any(|o| o.eq_ignore_ascii_case(option)) {
                return Err(format!("{option} is in the list twice."));
            }
        }
        self.change(|table| {
            let Some(c) = table.columns.get_mut(column) else {
                return false;
            };
            let ColumnKind::Choice(old) = &c.kind else {
                return false;
            };
            let old = old.clone();
            c.kind = ColumnKind::Choice(options.clone());
            for row in &mut table.rows {
                if let Cell::Option(o) = &row.cells[column] {
                    let now = old
                        .iter()
                        .position(|x| x == o)
                        .and_then(|at| {
                            if old.len() == options.len() {
                                options.get(at)
                            } else {
                                options.iter().find(|n| *n == o)
                            }
                        })
                        .unwrap_or(&options[0])
                        .clone();
                    if &now != o {
                        Arc::make_mut(row).cells[column] = Cell::Option(now);
                    }
                }
            }
            true
        });
        Ok(())
    }

    /// Sets what each row is called (`Item`); or why it can't be.
    pub(crate) fn set_row_name(&mut self, name: &str) -> Result<(), String> {
        if let Some(why) = level::table::option_problem(name, "What each row is called") {
            return Err(why);
        }
        self.change(|table| {
            if table.row_name == name {
                return false;
            }
            name.clone_into(&mut table.row_name);
            true
        });
        Ok(())
    }

    /// Takes rows `rows` (in the file's order) out.
    pub(crate) fn delete_rows(&mut self, rows: std::ops::RangeInclusive<usize>) -> bool {
        let deleted = self.change(|table| {
            let end = (*rows.end() + 1).min(table.rows.len());
            if *rows.start() >= end {
                return false;
            }
            table.rows.drain(*rows.start()..end);
            true
        });
        if deleted {
            self.anchor = None;
        }
        deleted
    }

    /// Moves the row at `from` to `to` (in the file's order).
    pub(crate) fn move_row(&mut self, from: usize, to: usize) -> bool {
        let moved = self.change(|table| {
            let n = table.rows.len();
            if from >= n || to >= n || from == to {
                return false;
            }
            let row = table.rows.remove(from);
            table.rows.insert(to, row);
            true
        });
        if moved
            && let Some((r, c)) = self.cursor
            && r == from
        {
            self.cursor = Some((to, c));
        }
        moved
    }

    /// Sets the cell at `at` from typed `text`; or why it can't be (the
    /// ID's place renames the row).
    pub(crate) fn set_text(&mut self, at: Place, text: &str) -> Result<(), String> {
        let (row, place) = at;
        if row >= self.file.rows.len() {
            return Ok(());
        }
        if place == 0 {
            let id = text.trim();
            if let Some(why) = self.file.id_problem(id, Some(row)) {
                return Err(why);
            }
            self.change(|table| {
                if table.rows[row].id == id {
                    return false;
                }
                Arc::make_mut(&mut table.rows[row]).id = id.to_string();
                true
            });
            return Ok(());
        }
        let Some(column) = self.file.columns.get(place - 1) else {
            return Ok(());
        };
        let cell = sheet::parse_cell(&column.kind, text)
            .map_err(|why| format!("\"{}\" {why}.", text.chars().take(40).collect::<String>()))?;
        self.set_cell(at, cell);
        Ok(())
    }

    /// Sets the cell at `at` (a column's, not the ID's).
    pub(crate) fn set_cell(&mut self, at: Place, cell: Cell) -> bool {
        let (row, place) = at;
        self.change(|table| {
            let Some(r) = table.rows.get_mut(row) else {
                return false;
            };
            if place == 0 || r.cells.get(place - 1) == Some(&cell) {
                return false;
            }
            Arc::make_mut(r).cells[place - 1] = cell;
            true
        })
    }

    /// Empties the cells in the block picked (IDs stay: a row needs one).
    pub(crate) fn clear(&mut self) -> bool {
        let Some((rows, places)) = self.block() else {
            return false;
        };
        self.change(|table| {
            for r in rows.clone() {
                let Some(row) = table.rows.get_mut(r) else {
                    continue;
                };
                for p in places.clone().filter(|p| *p > 0) {
                    let Some(column) = table.columns.get(p - 1) else {
                        continue;
                    };
                    let empty = column.kind.empty();
                    if row.cells[p - 1] != empty {
                        Arc::make_mut(row).cells[p - 1] = empty;
                    }
                }
            }
            true
        })
    }

    /// The block picked as the text a spreadsheet pastes.
    pub(crate) fn copy(&self) -> Option<String> {
        let (rows, places) = self.block()?;
        Some(sheet::copy(&self.file, rows, places))
    }

    /// Pastes a spreadsheet's `text` with its top left at the cursor, as
    /// one undo step; what couldn't be taken is in [`Self::said`].
    pub(crate) fn paste(&mut self, text: &str) -> bool {
        let Some((row, place)) = self
            .cursor
            .or((self.file.rows.is_empty()).then_some((0, 0)))
        else {
            return false;
        };
        let mut said = Vec::new();
        let changed = self.change(|table| {
            let (notes, _) = sheet::paste(table, row, place, text);
            said = notes;
            true
        });
        self.said = said;
        changed
    }

    /// Takes CSV `text` in (rows matched by ID, columns by name), as one
    /// undo step; or why it can't be read.
    pub(crate) fn import_csv(&mut self, text: &str) -> Result<sheet::Imported, String> {
        let mut done = Err(String::new());
        self.change(|table| {
            done = sheet::import_csv(table, text);
            done.is_ok()
        });
        if let Ok(done) = &done {
            self.said.clone_from(&done.notes);
        }
        done
    }

    /// Puts the cursor on `at` alone.
    pub(crate) fn pick(&mut self, at: Place) {
        self.cursor = Some(at);
        self.anchor = None;
        self.reveal = true;
    }

    /// Moves the cursor `rows` down and `places` right among the rows
    /// shown, keeping within them; with `extend`, the block grows from
    /// where it was.
    pub(crate) fn step(&mut self, rows: isize, places: isize, extend: bool) {
        let max_place = self.places() - 1;
        let shown = self.shown().to_vec();
        if shown.is_empty() {
            return;
        }
        let (row, place) = self.cursor.unwrap_or((shown[0], 0));
        let at = shown.iter().position(|r| *r == row).unwrap_or(0);
        let at = at.saturating_add_signed(rows).min(shown.len() - 1);
        let place = place.saturating_add_signed(places).min(max_place);
        if extend {
            self.anchor = self.anchor.or(self.cursor);
        } else {
            self.anchor = None;
        }
        self.cursor = Some((shown[at], place));
        self.reveal = true;
    }

    /// Starts typing in the cursor's cell: with its text, or with
    /// `first` (a key typed over it).
    pub(crate) fn start_typing(&mut self, first: Option<String>) {
        let Some(at) = self.cursor else {
            return;
        };
        let text = first.unwrap_or_else(|| self.text_at(at));
        self.typing = Some(Typing {
            at,
            text,
            focused: false,
        });
    }

    /// The text of the cell at `at`, as typed.
    pub(crate) fn text_at(&self, (row, place): Place) -> String {
        let Some(r) = self.file.rows.get(row) else {
            return String::new();
        };
        match place {
            0 => r.id.clone(),
            p => r.cells.get(p - 1).map(sheet::cell_text).unwrap_or_default(),
        }
    }

    /// Keeps what's typed in the cell; or why it can't be (typing goes
    /// on).
    pub(crate) fn confirm(&mut self) -> Result<(), String> {
        let Some(typing) = self.typing.clone() else {
            return Ok(());
        };
        self.set_text(typing.at, &typing.text)?;
        self.typing = None;
        Ok(())
    }

    /// Takes in the file as it is now (changed on disk), as one undo step.
    pub(crate) fn taken_in(&mut self, file: Table) {
        self.history.push(std::mem::replace(&mut self.file, file));
        self.typing = None;
        self.keep_picks();
    }

    /// Whether there's a change to undo.
    #[cfg(test)]
    pub(crate) fn can_undo(&self) -> bool {
        self.history.can_undo()
    }
}

/// The rows of `table` holding `search` in any cell (any capitals), in
/// the file's order or sorted by `sort` (rows that are equal keep their
/// order).
fn shown_rows(table: &Table, search: &str, sort: Option<Sort>) -> Vec<usize> {
    let words = search.trim().to_lowercase();
    let mut shown: Vec<usize> = (0..table.rows.len())
        .filter(|&r| {
            words.is_empty() || {
                let row = &table.rows[r];
                row.id.to_lowercase().contains(&words)
                    || row
                        .cells
                        .iter()
                        .any(|c| sheet::cell_text(c).to_lowercase().contains(&words))
            }
        })
        .collect();
    if let Some(Sort { place, rising }) = sort {
        shown.sort_by(|&a, &b| {
            let (a, b) = (&table.rows[a], &table.rows[b]);
            let order = match place {
                0 => a.id.cmp(&b.id),
                p => compare(&a.cells[p - 1], &b.cells[p - 1]),
            };
            if rising { order } else { order.reverse() }
        });
    }
    shown
}

/// How two cells of one column sort: numbers by size, the rest by text.
fn compare(a: &Cell, b: &Cell) -> std::cmp::Ordering {
    match (a, b) {
        (Cell::Whole(x), Cell::Whole(y)) => x.cmp(y),
        (Cell::Number(x), Cell::Number(y)) => x.total_cmp(y),
        (Cell::YesNo(x), Cell::YesNo(y)) => x.cmp(y),
        _ => sheet::cell_text(a)
            .to_lowercase()
            .cmp(&sheet::cell_text(b).to_lowercase()),
    }
}

impl crate::own_files::OpenFile for TableEditor {
    type File = Table;

    fn path(&self) -> &str {
        &self.path
    }

    fn file(&self) -> &Table {
        &self.file
    }

    fn take_in(&mut self, file: Table) {
        self.taken_in(file);
    }

    fn of(held: &crate::own_files::OwnFile) -> Option<&Arc<Table>> {
        match held {
            crate::own_files::OwnFile::Table(file) => Some(file),
            _ => None,
        }
    }
}

#[cfg(test)]
#[path = "table_editor_tests.rs"]
mod tests;
