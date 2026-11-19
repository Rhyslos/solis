//! A table's cells as text: how each kind is written in the file and
//! typed in the Table tab ([`cell_text`], [`parse_cell`]), and a table as
//! spreadsheet text: CSV for importing and exporting, and the
//! tab-separated text spreadsheets copy and paste ([`parse`], [`write`]).
//!
//! Both are read as RFC 4180 has them: a field in double quotes may hold
//! the separator, line breaks and `""` for a quote; `\r\n` and `\n` both
//! end a line. Reading is within limits (rows, columns, a field's length),
//! so a hostile file is refused in plain words, never read without end.

use super::{Cell, ColumnKind, MAX_CELLS, MAX_COLUMNS, MAX_ROWS, MAX_TEXT_CHARS, Table};

/// The most bytes of spreadsheet text read at once.
pub const MAX_BYTES: usize = 96 << 20;

/// A cell as text: as the file keeps it, the Table tab shows it to type
/// over, and a spreadsheet takes it.
pub fn cell_text(cell: &Cell) -> String {
    match cell {
        Cell::Whole(n) => n.to_string(),
        Cell::Number(x) => crate::lust_data::float(*x)
            .trim_end_matches(".0")
            .to_string(),
        Cell::Text(t) | Cell::Option(t) => t.clone(),
        Cell::YesNo(b) => if *b { "yes" } else { "no" }.to_string(),
        Cell::Colour(c) => colour_hex(*c),
    }
}

/// `#rrggbb`, or `#rrggbbaa` when it's see-through at all.
pub fn colour_hex(c: [f32; 4]) -> String {
    let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    let [r, g, b, a] = c.map(byte);
    if a == 255 {
        format!("#{r:02x}{g:02x}{b:02x}")
    } else {
        format!("#{r:02x}{g:02x}{b:02x}{a:02x}")
    }
}

/// The cell `text` gives in a column of `kind`, or why it can't be one
/// (in words that follow "which": "isn't a whole number").
pub fn parse_cell(kind: &ColumnKind, text: &str) -> Result<Cell, String> {
    let trimmed = text.trim();
    match kind {
        ColumnKind::Whole => trimmed
            .parse::<i64>()
            .map(Cell::Whole)
            .or_else(|_| {
                // A spreadsheet's 5.0 for 5.
                trimmed
                    .parse::<f64>()
                    .ok()
                    .filter(|x| x.fract() == 0.0 && x.abs() < 9.0e15)
                    .map(|x| Cell::Whole(x as i64))
                    .ok_or(())
            })
            .map_err(|()| "isn't a whole number".into()),
        ColumnKind::Number => match trimmed.parse::<f64>() {
            Ok(x) if x.is_finite() => Ok(Cell::Number(x)),
            _ => Err("isn't a number".into()),
        },
        ColumnKind::Text => {
            if text.chars().count() > MAX_TEXT_CHARS {
                Err(format!("is longer than {MAX_TEXT_CHARS} characters"))
            } else {
                Ok(Cell::Text(text.to_string()))
            }
        }
        ColumnKind::YesNo => match trimmed.to_ascii_lowercase().as_str() {
            "yes" | "y" | "true" | "1" | "x" | "✓" | "on" => Ok(Cell::YesNo(true)),
            "no" | "n" | "false" | "0" | "" | "off" => Ok(Cell::YesNo(false)),
            _ => Err("isn't yes or no".into()),
        },
        ColumnKind::Choice(options) => options
            .iter()
            .find(|o| o.eq_ignore_ascii_case(trimmed))
            .map(|o| Cell::Option(o.clone()))
            .ok_or_else(|| format!("isn't one of its options ({})", options.join(", "))),
        ColumnKind::Name(kind) => {
            // `Picture.Sprites.Apple` as written in game code, or the name
            // after it.
            let name = trimmed
                .strip_prefix(kind.state())
                .and_then(|r| r.strip_prefix('.'))
                .unwrap_or(trimmed);
            let fits = name.split('.').all(|part| {
                part.starts_with(|c: char| c.is_ascii_alphabetic())
                    && part.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            });
            if name.is_empty() || (fits && name.len() <= 512) {
                Ok(Cell::Option(name.to_string()))
            } else {
                Err(format!(
                    "isn't a {} as game code names it",
                    kind.state().to_ascii_lowercase()
                ))
            }
        }
        ColumnKind::Colour => parse_colour(trimmed)
            .map(Cell::Colour)
            .ok_or_else(|| "isn't a colour written #rrggbb".into()),
    }
}

/// `#rgb`, `#rrggbb` or `#rrggbbaa` (the `#` may be left out).
pub fn parse_colour(text: &str) -> Option<[f32; 4]> {
    let hex = text.strip_prefix('#').unwrap_or(text);
    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let pair = |at: usize| u8::from_str_radix(hex.get(at..at + 2)?, 16).ok();
    let [r, g, b, a] = match hex.len() {
        3 => {
            let one = |at: usize| {
                u8::from_str_radix(hex.get(at..=at)?, 16)
                    .ok()
                    .map(|v| v * 17)
            };
            [one(0)?, one(1)?, one(2)?, 255]
        }
        6 => [pair(0)?, pair(2)?, pair(4)?, 255],
        8 => [pair(0)?, pair(2)?, pair(4)?, pair(6)?],
        _ => return None,
    };
    Some([r, g, b, a].map(|v| f32::from(v) / 255.0))
}

/// Spreadsheet text read into rows of fields, split by `separator`
/// (`,` for CSV, `\t` for what a spreadsheet copies). A line break inside
/// quotes stays in its field; a last empty line is no row. Refused, in
/// plain words, past [`MAX_BYTES`], [`MAX_ROWS`] (and a header) rows,
/// more fields than a table has columns (and its IDs), or a field longer
/// than a cell's text.
pub fn parse(text: &str, separator: char) -> Result<Vec<Vec<String>>, String> {
    if text.len() > MAX_BYTES {
        return Err(format!("it's bigger than {} MB", MAX_BYTES >> 20));
    }
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = text.chars().peekable();
    let mut cells = 0usize;
    let mut started = false;
    let end_field = |row: &mut Vec<String>, field: &mut String, cells: &mut usize| {
        if row.len() > MAX_COLUMNS {
            return Err(format!(
                "a line has more than {} fields, more than a table's IDs and {MAX_COLUMNS} columns",
                MAX_COLUMNS + 1
            ));
        }
        *cells += 1;
        if *cells > MAX_CELLS + MAX_ROWS + MAX_COLUMNS + 1 {
            return Err(format!("it has more than {MAX_CELLS} cells"));
        }
        row.push(std::mem::take(field));
        Ok(())
    };
    while let Some(c) = chars.next() {
        started = true;
        if field.len() > MAX_TEXT_CHARS * 4 {
            return Err(format!(
                "a field is longer than {MAX_TEXT_CHARS} characters"
            ));
        }
        if quoted {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    field.push('"');
                } else {
                    quoted = false;
                }
            } else {
                field.push(c);
            }
            continue;
        }
        match c {
            '"' if field.is_empty() => quoted = true,
            c if c == separator => end_field(&mut row, &mut field, &mut cells)?,
            '\r' if chars.peek() == Some(&'\n') => {}
            '\n' | '\r' => {
                end_field(&mut row, &mut field, &mut cells)?;
                rows.push(std::mem::take(&mut row));
                started = false;
                if rows.len() > MAX_ROWS + 1 {
                    return Err(format!("it has more than {MAX_ROWS} rows"));
                }
            }
            c => field.push(c),
        }
    }
    if started {
        end_field(&mut row, &mut field, &mut cells)?;
        rows.push(row);
    }
    Ok(rows)
}

/// One field written for spreadsheet text, in quotes when it holds the
/// separator, a quote or a line break (or starts or ends with a space).
pub fn field(text: &str, separator: char) -> String {
    let needs =
        text.contains([separator, '"', '\n', '\r']) || text.starts_with(' ') || text.ends_with(' ');
    if needs {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_string()
    }
}

/// Rows of fields as spreadsheet text, each line ended with `\r\n` as
/// RFC 4180 has it.
pub fn write(rows: &[Vec<String>], separator: char) -> String {
    let mut out = String::new();
    for row in rows {
        let line: Vec<String> = row.iter().map(|f| field(f, separator)).collect();
        out.push_str(&line.join(&separator.to_string()));
        out.push_str("\r\n");
    }
    out
}

/// `table` as CSV: a header (`id` and each column's name), then a line a
/// row.
pub fn to_csv(table: &Table) -> String {
    let mut rows = Vec::with_capacity(table.rows.len() + 1);
    rows.push(
        std::iter::once(super::ID_FIELD.to_string())
            .chain(table.columns.iter().map(|c| c.name.clone()))
            .collect(),
    );
    for row in &table.rows {
        rows.push(
            std::iter::once(row.id.clone())
                .chain(row.cells.iter().map(cell_text))
                .collect(),
        );
    }
    write(&rows, ',')
}

/// What a column of fields most likely holds, for a CSV's column the
/// table doesn't have: whole numbers, numbers, yes or no, colours, or text.
pub fn guess_kind<'a>(fields: impl Iterator<Item = &'a str> + Clone) -> ColumnKind {
    let filled = || fields.clone().map(str::trim).filter(|f| !f.is_empty());
    if filled().next().is_none() {
        return ColumnKind::Text;
    }
    if filled().all(|f| f.parse::<i64>().is_ok()) {
        ColumnKind::Whole
    } else if filled().all(|f| f.parse::<f64>().is_ok_and(f64::is_finite)) {
        ColumnKind::Number
    } else if filled().all(|f| {
        matches!(
            f.to_ascii_lowercase().as_str(),
            "yes" | "no" | "true" | "false"
        )
    }) {
        ColumnKind::YesNo
    } else if filled().all(|f| f.starts_with('#') && parse_colour(f).is_some()) {
        ColumnKind::Colour
    } else {
        ColumnKind::Text
    }
}

/// What [`import_csv`] did, in counts and notes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Imported {
    /// Rows added at the end.
    pub added: usize,
    /// Rows already there whose cells were set.
    pub changed: usize,
    /// Columns added for a header the table didn't have.
    pub columns: usize,
    /// What couldn't be taken in, and why.
    pub notes: Vec<String>,
}

/// The most notes an import or a paste keeps; the rest are counted.
const MAX_NOTES: usize = 20;

fn note(notes: &mut Vec<String>, said: &mut usize, text: impl FnOnce() -> String) {
    *said += 1;
    if notes.len() < MAX_NOTES {
        notes.push(text());
    }
}

/// Takes CSV `text` into `table`: its first line names the columns (the
/// first is the rows' IDs), matched to the table's by name or heading;
/// a column the table lacks is added, its kind guessed from its fields. A
/// row whose ID the table has gets its cells set; any other is added at
/// the end. An ID that can't be one is made one (`iron sword` is
/// `IronSword`); a field that can't be its column's kind is left as it
/// was, with a note.
pub fn import_csv(table: &mut Table, text: &str) -> Result<Imported, String> {
    let mut lines = parse(text, ',')?;
    if lines.is_empty() {
        return Err("it's empty".into());
    }
    let header = lines.remove(0);
    let mut done = Imported::default();
    let mut said = 0usize;
    let mut targets: Vec<Option<usize>> = Vec::with_capacity(header.len());
    for (at, name) in header.iter().enumerate().skip(1) {
        let wanted = name.trim();
        let found = table.columns.iter().position(|c| {
            c.name.eq_ignore_ascii_case(wanted) || c.label().eq_ignore_ascii_case(wanted)
        });
        let column = match found {
            Some(c) => Some(c),
            None => match table.column_room() {
                Ok(()) => {
                    let fields = lines.iter().map(|l| l.get(at).map_or("", String::as_str));
                    let kind = guess_kind(fields);
                    let name = table.free_column(&super::field_suggestion(wanted));
                    table.columns.push(super::Column::new(name, kind.clone()));
                    for row in &mut table.rows {
                        std::sync::Arc::make_mut(row).cells.push(kind.empty());
                    }
                    done.columns += 1;
                    Some(table.columns.len() - 1)
                }
                Err(why) => {
                    note(&mut done.notes, &mut said, || {
                        format!("The column \"{wanted}\" is left out: {why}")
                    });
                    None
                }
            },
        };
        targets.push(column);
    }
    let mut index: std::collections::HashMap<String, usize> = table
        .rows
        .iter()
        .enumerate()
        .map(|(at, r)| (r.id.to_ascii_lowercase(), at))
        .collect();
    for line in lines {
        let Some(raw) = line.first() else { continue };
        let raw = raw.trim();
        if raw.is_empty() && line.iter().all(|f| f.trim().is_empty()) {
            continue;
        }
        let id = if super::option_problem(raw, "").is_none() && raw != super::NO_ROW {
            raw.to_string()
        } else {
            super::suggestion(raw)
        };
        let at = match index.get(&id.to_ascii_lowercase()) {
            Some(&at) => {
                done.changed += 1;
                at
            }
            None => {
                if let Err(why) = table.row_room() {
                    note(&mut done.notes, &mut said, || why.clone());
                    break;
                }
                let id = table.free_id(&id);
                index.insert(id.to_ascii_lowercase(), table.rows.len());
                table.rows.push(std::sync::Arc::new(table.new_row(id)));
                done.added += 1;
                table.rows.len() - 1
            }
        };
        set_fields(
            table,
            at,
            &targets,
            line.iter().skip(1),
            &mut done.notes,
            &mut said,
        );
    }
    if said > done.notes.len() {
        done.notes
            .push(format!("…and {} more like these.", said - done.notes.len()));
    }
    Ok(done)
}

/// Sets row `at`'s cells from `fields`, each to the column `targets` says
/// (none: left out), leaving a cell whose field can't be its kind.
fn set_fields<'a>(
    table: &mut Table,
    at: usize,
    targets: &[Option<usize>],
    fields: impl Iterator<Item = &'a String>,
    notes: &mut Vec<String>,
    said: &mut usize,
) {
    let columns = &table.columns;
    let row = std::sync::Arc::make_mut(&mut table.rows[at]);
    for (field, target) in fields.zip(targets) {
        let Some(c) = *target else { continue };
        // An empty field leaves a number, a choice or a colour as it is.
        if field.trim().is_empty()
            && !matches!(columns[c].kind, ColumnKind::Text | ColumnKind::Name(_))
        {
            continue;
        }
        match parse_cell(&columns[c].kind, field) {
            Ok(cell) => row.cells[c] = cell,
            Err(why) => note(notes, said, || {
                format!(
                    "Row {}'s {} was \"{}\", which {why}; it's left as it was.",
                    row.id,
                    columns[c].name,
                    field.chars().take(40).collect::<String>()
                )
            }),
        }
    }
}

/// The cells in rows `rows` and places `places` (0 is the IDs, 1 the
/// first column) as the tab-separated text a spreadsheet pastes.
pub fn copy(
    table: &Table,
    rows: std::ops::RangeInclusive<usize>,
    places: std::ops::RangeInclusive<usize>,
) -> String {
    let lines: Vec<Vec<String>> = table
        .rows
        .get(rows)
        .unwrap_or_default()
        .iter()
        .map(|row| {
            places
                .clone()
                .filter_map(|p| match p {
                    0 => Some(row.id.clone()),
                    p => row.cells.get(p - 1).map(cell_text),
                })
                .collect()
        })
        .collect();
    let mut text = write(&lines, '\t');
    // One cell copied is its text alone, as spreadsheets give it.
    if lines.len() == 1 && lines[0].len() == 1 {
        text = lines[0][0].clone();
    }
    text
}

/// Pastes tab-separated `text` into `table` with its top left at row
/// `row` and place `place` (0 is the IDs): rows past the end are added
/// with IDs of their own, fields past the last column are left out, an
/// ID pasted is a rename, and a field that can't be its column's kind
/// leaves its cell, with a note. Gives back the notes and how many rows
/// were added.
pub fn paste(table: &mut Table, row: usize, place: usize, text: &str) -> (Vec<String>, usize) {
    let mut notes = Vec::new();
    let mut said = 0usize;
    let block = match parse(text.strip_suffix('\n').unwrap_or(text), '\t') {
        Ok(block) => block,
        Err(why) => return (vec![format!("Couldn't paste: {why}.")], 0),
    };
    let mut added = 0;
    for (n, line) in block.iter().enumerate() {
        let at = row + n;
        if at >= table.rows.len() {
            if let Err(why) = table.row_room() {
                notes.push(why);
                break;
            }
            let id = table.free_id(&table.row_name.clone());
            table.rows.push(std::sync::Arc::new(table.new_row(id)));
            added += 1;
        }
        let mut fields = line.iter();
        if place == 0 {
            if let Some(id) = fields.next() {
                let id = id.trim();
                if table.rows[at].id != id {
                    match table.id_problem(id, Some(at)) {
                        None => std::sync::Arc::make_mut(&mut table.rows[at]).id = id.to_string(),
                        Some(why) => note(&mut notes, &mut said, || why),
                    }
                }
            }
        }
        let first = place.max(1) - 1;
        let targets: Vec<Option<usize>> = (first..table.columns.len()).map(Some).collect();
        set_fields(table, at, &targets, fields, &mut notes, &mut said);
    }
    if said > notes.len() {
        notes.push(format!("…and {} more like these.", said - notes.len()));
    }
    (notes, added)
}
