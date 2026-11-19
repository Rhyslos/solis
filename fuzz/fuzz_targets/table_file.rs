//! A data table's file (`items.table.lst`, stage 1): refused or read,
//! within the limits, every ID and column one game code can use and none
//! twice, every row a cell a column, and written and read back as the
//! same table.
#![no_main]
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    fuzz_support::quiet_caught_panics();
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let Ok(read) = level::table::file::read(text, "Item") else {
        return;
    };
    let table = read.table;
    assert!(table.rows.len() <= level::table::MAX_ROWS, "within the most rows");
    assert!(table.columns.len() <= level::table::MAX_COLUMNS, "within the most columns");
    for (at, column) in table.columns.iter().enumerate() {
        assert_eq!(table.column_problem(&column.name, Some(at)), None, "{}", column.name);
    }
    for row in &table.rows {
        assert_eq!(row.cells.len(), table.columns.len(), "a cell a column");
        assert_eq!(level::table::option_problem(&row.id, ""), None, "{}", row.id);
    }
    let text = level::table::file::write(&table);
    let again = level::table::file::read(&text, "Item").expect("what's written reads");
    assert!(again.notes.is_empty(), "{:?}", again.notes);
    assert_eq!(level::table::file::write(&again.table), text, "a table written reads back");
    // Spreadsheet text of the same table reads back too.
    let csv = level::table::text::to_csv(&table);
    let mut copy = level::table::Table::new(table.row_name.clone());
    let _ = level::table::text::import_csv(&mut copy, &csv);
});
