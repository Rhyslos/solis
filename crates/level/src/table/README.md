# level/src/table

Data tables (stage 1; [decision 0128](../../../../docs/decisions/0128-data-tables.md)): a game's data as rows and typed columns, kept in one Lust data file and read by game code with its names checked.

| File | What it does |
| --- | --- |
| `mod.rs` | The table itself: `Table` (what each row is called, its columns, its rows, shared so an undo step is cheap), `Column` and `ColumnKind` (whole number, number, text, yes or no, a choice, one of the project's names, colour), `Cell`; the limits (`MAX_ROWS`, `MAX_COLUMNS`, `MAX_CELLS`…), and the names game code reads: an ID or option must be a state's option (`option_problem`), a column a field (`field_problem`), each with a suggestion; the names a table's file gives (`row_name_for`, `object_name_for`); and the number standing for a table (`content_hash`). |
| `file.rs` | The file, `items.table.lst`: a Lust data file (decision 0081), version 1, a row a line, every cell as text; read with notes for what can't be used (left out, or made empty), the first rows kept past the limits. `FORMAT`, `CLASSES` (written into `solis/data/table.lst`). |
| `text.rs` | Cells as text (`cell_text`, `parse_cell`: what's typed in the Table tab), and a table as spreadsheet text read and written as RFC 4180 has it, within limits: CSV out (`to_csv`) and in (`import_csv`, rows matched by ID and columns by name), and the tab-separated text copied between spreadsheets (`copy`, `paste`). |
| `tests.rs` | Round trips of every kind, damaged files and limits, names refused in plain words, CSV with quotes, commas and line breaks, copy and paste, and property tests of damaged files and any spreadsheet text. The reader is fuzzed by `fuzz/fuzz_targets/table_file.rs`. |
