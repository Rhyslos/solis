use super::file::{read, write};
use super::text::{cell_text, copy, import_csv, parse, parse_cell, paste, to_csv};
use super::*;
use proptest::prelude::*;

/// A table with a column of every kind and three rows.
fn items() -> Table {
    let mut table = Table::new("Item");
    table.columns = vec![
        Column {
            translated: true,
            ..Column::new("name", ColumnKind::Text)
        },
        Column::new("price", ColumnKind::Whole),
        Column::new("weight", ColumnKind::Number),
        Column::new("stacks", ColumnKind::YesNo),
        Column::new(
            "rarity",
            ColumnKind::Choice(vec!["Common".into(), "Rare".into()]),
        ),
        Column::new("icon", ColumnKind::Name(NameKind::Picture)),
        Column::new("tint", ColumnKind::Colour),
    ];
    let row = |id: &str, cells: Vec<Cell>| {
        Arc::new(Row {
            id: id.into(),
            cells,
        })
    };
    table.rows = vec![
        row(
            "Apple",
            vec![
                Cell::Text("Apple".into()),
                Cell::Whole(5),
                Cell::Number(0.2),
                Cell::YesNo(true),
                Cell::Option("Common".into()),
                Cell::Option("Sprites.Apple".into()),
                Cell::Colour([1.0, 0.0, 0.0, 1.0]),
            ],
        ),
        row(
            "IronSword",
            vec![
                Cell::Text("Iron sword, \"sharp\"\nand {heavy}".into()),
                Cell::Whole(-120),
                Cell::Number(3.5e10),
                Cell::YesNo(false),
                Cell::Option("Rare".into()),
                Cell::Option(String::new()),
                Cell::Colour([0.2, 0.4, 0.6, 0.5]),
            ],
        ),
        row("Potion", (0..7).map(table_empty).collect()),
    ];
    table
}

fn table_empty(column: usize) -> Cell {
    let kinds = [
        ColumnKind::Text,
        ColumnKind::Whole,
        ColumnKind::Number,
        ColumnKind::YesNo,
        ColumnKind::Choice(vec!["Common".into(), "Rare".into()]),
        ColumnKind::Name(NameKind::Picture),
        ColumnKind::Colour,
    ];
    kinds[column].empty()
}

/// Colours as the file keeps them: a byte each.
fn bytes_of(table: &Table) -> Table {
    let mut t = table.clone();
    for row in &mut t.rows {
        for cell in &mut Arc::make_mut(row).cells {
            if let Cell::Colour(c) = cell {
                *c = text::parse_colour(&text::colour_hex(*c)).unwrap_or_default();
            }
        }
    }
    t
}

#[test]
fn a_table_of_every_kind_reads_back_as_itself() {
    let table = items();
    let text = write(&table);
    let back = read(&text, "Fallback").unwrap();
    assert_eq!(back.notes, Vec::<String>::new());
    assert_eq!(back.table, bytes_of(&table));
    assert_eq!(write(&back.table), text);
    assert!(text.contains("new Row { id = \"Apple\", cells = [\"Apple\", \"5\", \"0.2\", \"yes\", \"Common\", \"Sprites.Apple\", \"#ff0000\"] },"));
}

#[test]
fn an_empty_table_reads_back() {
    let table = Table::new("Crop");
    let back = read(&write(&table), "X").unwrap();
    assert_eq!(back.table, table);
}

#[test]
fn every_kind_has_a_word_and_reads_its_empty_cell_back() {
    for kind in ColumnKind::all() {
        let options = match &kind {
            ColumnKind::Choice(o) => o.clone(),
            _ => Vec::new(),
        };
        assert_eq!(
            ColumnKind::of_word(kind.word(), options),
            Some(kind.clone())
        );
        let empty = kind.empty();
        assert_eq!(parse_cell(&kind, &cell_text(&empty)), Ok(empty), "{kind:?}");
    }
}

#[test]
fn cells_are_typed_as_spreadsheets_write_them() {
    assert_eq!(parse_cell(&ColumnKind::Whole, " 42 "), Ok(Cell::Whole(42)));
    assert_eq!(parse_cell(&ColumnKind::Whole, "5.0"), Ok(Cell::Whole(5)));
    assert!(parse_cell(&ColumnKind::Whole, "5.5").is_err());
    assert!(parse_cell(&ColumnKind::Number, "inf").is_err());
    assert!(parse_cell(&ColumnKind::Number, "NaN").is_err());
    assert_eq!(
        parse_cell(&ColumnKind::YesNo, "TRUE"),
        Ok(Cell::YesNo(true))
    );
    assert_eq!(parse_cell(&ColumnKind::YesNo, ""), Ok(Cell::YesNo(false)));
    assert!(parse_cell(&ColumnKind::YesNo, "maybe").is_err());
    let choice = ColumnKind::Choice(vec!["Common".into(), "Rare".into()]);
    assert_eq!(parse_cell(&choice, "rare"), Ok(Cell::Option("Rare".into())));
    assert!(
        parse_cell(&choice, "Epic")
            .unwrap_err()
            .contains("Common, Rare")
    );
    let picture = ColumnKind::Name(NameKind::Picture);
    assert_eq!(
        parse_cell(&picture, "Picture.Sprites.Apple"),
        Ok(Cell::Option("Sprites.Apple".into()))
    );
    assert!(parse_cell(&picture, "sprites/apple.png").is_err());
    assert_eq!(
        parse_cell(&ColumnKind::Colour, "#f00"),
        Ok(Cell::Colour([1.0, 0.0, 0.0, 1.0]))
    );
    assert!(parse_cell(&ColumnKind::Colour, "#ff00").is_err());
    let long = "x".repeat(MAX_TEXT_CHARS + 1);
    assert!(parse_cell(&ColumnKind::Text, &long).is_err());
}

#[test]
fn names_game_code_cant_use_are_refused_in_plain_words() {
    let table = items();
    assert!(table.id_problem("Bread", None).is_none());
    assert!(table.id_problem("bread", None).unwrap().contains("Bread"));
    assert!(
        table
            .id_problem("Iron Sword", None)
            .unwrap()
            .contains("IronSword")
    );
    assert!(table.id_problem("apple", None).is_some());
    assert!(table.id_problem("APPLE", None).is_some());
    assert!(table.id_problem("ironsword", None).is_some());
    assert!(table.id_problem("Apple", Some(0)).is_none());
    assert!(table.id_problem("None", None).is_some());
    assert!(table.id_problem("Maths", None).is_some());
    assert!(table.column_problem("daysToGrow", None).is_none());
    assert!(
        table
            .column_problem("Days to grow", None)
            .unwrap()
            .contains("daysToGrow")
    );
    for taken in ["id", "end", "function", "price", "Price", "colour"] {
        assert!(table.column_problem(taken, None).is_some(), "{taken}");
    }
    assert_eq!(table.free_id("Apple"), "Apple2");
    assert_eq!(table.free_column("price"), "price2");
}

#[test]
fn rows_and_tables_are_named_from_the_file() {
    assert_eq!(row_name_for("items"), "Item");
    assert_eq!(row_name_for("berries"), "Berry");
    assert_eq!(row_name_for("glass"), "GlassRow");
    assert_eq!(row_name_for("shop prices"), "ShopPrice");
    assert_eq!(row_name_for("weather"), "WeatherRow");
    assert_eq!(object_name_for("items"), Some("Items".into()));
    assert_eq!(object_name_for("shop prices"), Some("ShopPrices".into()));
    assert_eq!(object_name_for("3d"), None);
    assert_eq!(label_of("daysToGrow"), "Days to grow");
}

#[test]
fn a_damaged_row_or_column_is_left_out_with_a_note() {
    let text = write(&items())
        .replace("name = \"price\"", "name = \"price tag\"")
        .replace("id = \"Potion\"", "id = \"potion\"")
        .replace("\"#ff0000\"", "\"red\"")
        .replace("\"yes\"", "\"perhaps\"");
    let back = read(&text, "Item").unwrap();
    assert_eq!(back.table.columns.len(), 6);
    assert_eq!(back.table.rows.len(), 2);
    assert_eq!(back.notes.len(), 4, "{:?}", back.notes);
    assert!(back.notes[0].contains("price tag"));
    // The apple's colour went white, its stack no.
    assert_eq!(back.table.rows[0].cells[5], Cell::Colour([1.0; 4]));
    assert_eq!(back.table.rows[0].cells[2], Cell::YesNo(false));
}

#[test]
fn a_second_row_of_one_id_is_left_out() {
    let text = write(&items()).replace("id = \"Potion\"", "id = \"ApplE\"");
    let back = read(&text, "Item").unwrap();
    assert_eq!(back.table.rows.len(), 2);
    assert!(back.notes[0].contains("another row has that ID"));
}

#[test]
fn a_file_from_a_newer_version_or_not_lust_is_refused() {
    let newer = write(&items()).replace("version = 1", "version = 9");
    assert!(read(&newer, "X").unwrap_err().message.contains("newer"));
    assert!(read("hello", "X").is_err());
    assert!(read("", "X").is_err());
    let code = write(&items()).replace("lock Table", "method boom() { }\nlock Table");
    assert!(read(&code, "X").is_err());
}

#[test]
fn past_the_limits_the_first_rows_are_kept() {
    let mut table = Table::new("Thing");
    table.columns = (0..40)
        .map(|n| Column::new(format!("c{n}"), ColumnKind::Whole))
        .collect();
    let most = MAX_CELLS / 40;
    table.rows = (0..most + 3)
        .map(|n| Arc::new(table.new_row(format!("R{n}"))))
        .collect();
    let back = read(&write(&table), "Thing").unwrap();
    assert_eq!(back.table.rows.len(), most);
    assert!(back.notes[0].contains("3 more"));
    assert!(back.table.row_room().unwrap_err().contains("cells"));
}

#[test]
fn a_full_table_says_why_it_takes_no_more() {
    let mut table = Table::new("Thing");
    table.columns = (0..MAX_COLUMNS)
        .map(|n| Column::new(format!("c{n}"), ColumnKind::YesNo))
        .collect();
    assert!(table.column_room().unwrap_err().contains("64 columns"));
    table.columns.truncate(1);
    table.rows = (0..MAX_ROWS)
        .map(|n| {
            Arc::new(Row {
                id: format!("R{n}"),
                cells: vec![Cell::YesNo(false)],
            })
        })
        .collect();
    assert!(table.row_room().unwrap_err().contains("100000 rows"));
}

#[test]
fn csv_keeps_quotes_commas_and_line_breaks() {
    let table = items();
    let csv = to_csv(&table);
    assert!(csv.starts_with("id,name,price,weight,stacks,rarity,icon,tint\r\n"));
    assert!(csv.contains("\"Iron sword, \"\"sharp\"\"\nand {heavy}\""));
    let lines = parse(&csv, ',').unwrap();
    assert_eq!(lines.len(), 4);
    assert_eq!(lines[2][1], "Iron sword, \"sharp\"\nand {heavy}");
    let mut empty = Table::new("Item");
    let done = import_csv(&mut empty, &csv).unwrap();
    assert_eq!((done.added, done.columns), (3, 7));
    assert_eq!(empty.rows[1].cells[0], table.rows[1].cells[0]);
    assert_eq!(empty.columns[1].kind, ColumnKind::Whole);
    assert_eq!(empty.columns[3].kind, ColumnKind::YesNo);
    assert_eq!(empty.columns[6].kind, ColumnKind::Colour);
}

#[test]
fn importing_sets_rows_it_has_and_adds_the_rest() {
    let mut table = items();
    let csv = "ID,Price,Name,Days to grow\r\nApple,7,Red apple,3\r\nbread loaf,2,Bread,\r\nPotion,lots,Potion,1\r\n";
    let done = import_csv(&mut table, csv).unwrap();
    assert_eq!((done.added, done.changed, done.columns), (1, 2, 1));
    assert_eq!(table.rows[0].cells[1], Cell::Whole(7));
    assert_eq!(table.rows[3].id, "BreadLoaf");
    assert_eq!(table.columns[7].name, "daysToGrow");
    assert_eq!(table.columns[7].kind, ColumnKind::Whole);
    assert_eq!(done.notes.len(), 1);
    assert!(done.notes[0].contains("lots"));
}

#[test]
fn spreadsheet_text_is_read_within_limits() {
    assert_eq!(
        parse("a,b\nc,d", ',').unwrap(),
        vec![vec!["a", "b"], vec!["c", "d"]]
    );
    assert_eq!(parse("a\r\n\r\nb\r\n", ',').unwrap().len(), 3);
    assert_eq!(
        parse("\"a\"\"b\",\"\"", ',').unwrap(),
        vec![vec!["a\"b", ""]]
    );
    assert_eq!(parse("", ',').unwrap(), Vec::<Vec<String>>::new());
    let wide = vec!["x"; MAX_COLUMNS + 3].join(",");
    assert!(parse(&wide, ',').unwrap_err().contains("fields"));
    let long = format!("\"{}\"", "x".repeat(MAX_TEXT_CHARS * 5));
    assert!(parse(&long, ',').is_err());
}

#[test]
fn copying_and_pasting_cells_goes_through_tab_separated_text() {
    let mut table = items();
    let copied = copy(&table, 0..=1, 1..=2);
    assert_eq!(
        copied,
        "Apple\t5\r\n\"Iron sword, \"\"sharp\"\"\nand {heavy}\"\t-120\r\n"
    );
    assert_eq!(copy(&table, 0..=0, 0..=0), "Apple");
    let (notes, added) = paste(&mut table, 2, 1, &copied);
    assert!(notes.is_empty(), "{notes:?}");
    assert_eq!(added, 1);
    assert_eq!(table.rows[2].cells[0], Cell::Text("Apple".into()));
    assert_eq!(table.rows[3].cells[1], Cell::Whole(-120));
    assert_eq!(table.rows[3].id, "Item");
    let (notes, _) = paste(&mut table, 0, 2, "lots\t9");
    assert_eq!(notes.len(), 1);
    assert_eq!(table.rows[0].cells[1], Cell::Whole(5));
    assert_eq!(table.rows[0].cells[2], Cell::Number(9.0));
    let (notes, _) = paste(&mut table, 0, 0, "Pear\nPear");
    assert_eq!(table.rows[0].id, "Pear");
    assert!(notes[0].contains("already"));
}

#[test]
fn a_change_changes_the_hash_and_nothing_else_does() {
    let table = items();
    let same = read(&write(&table), "Item").unwrap().table;
    assert_eq!(bytes_of(&table).content_hash(), same.content_hash());
    let mut changed = same.clone();
    Arc::make_mut(&mut changed.rows[0]).cells[1] = Cell::Whole(6);
    assert_ne!(changed.content_hash(), same.content_hash());
}

#[test]
fn equal_tables_share_rows_and_compare_quickly() {
    let table = items();
    let copy = table.clone();
    assert!(Arc::ptr_eq(&table.rows[0], &copy.rows[0]));
    assert_eq!(table, copy);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(400))]

    /// Damaged files never crash the reader: each is read or refused.
    #[test]
    fn damaged_files_are_read_or_refused(cut in 0usize..2000, byte in any::<u8>(), at in 0usize..2000) {
        let mut bytes = write(&items()).into_bytes();
        let at = at % bytes.len();
        bytes[at] = byte;
        bytes.truncate(cut.max(1).min(bytes.len()));
        if let Ok(text) = String::from_utf8(bytes) {
            let _ = read(&text, "Item");
        }
    }

    /// Any text is read as spreadsheet text or refused, never a crash.
    #[test]
    fn any_spreadsheet_text_is_read_or_refused(text in "[a-z,\"\t\r\n ]{0,200}") {
        let rows = parse(&text, ',');
        let mut table = items();
        let _ = paste(&mut table, 1, 3, &text);
        let _ = import_csv(&mut table, &text);
        if let Ok(rows) = rows {
            prop_assert!(rows.iter().all(|r| r.len() <= MAX_COLUMNS + 1));
        }
    }

    /// Text cells round trip through the file whatever they hold.
    #[test]
    fn any_text_round_trips(text in "\\PC{0,60}") {
        let mut table = Table::new("Item");
        table.columns.push(Column::new("words", ColumnKind::Text));
        table.rows.push(Arc::new(Row { id: "A".into(), cells: vec![Cell::Text(text.clone())] }));
        let back = read(&write(&table), "Item").unwrap();
        prop_assert_eq!(back.table, table);
    }
}
