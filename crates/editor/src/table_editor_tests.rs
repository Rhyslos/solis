//! A table changed in the Table tab: each change one undo step, and what
//! the tab looks at (search, sort, the cursor) never changing the file.

use super::*;
use level::table::Row;
use reflect::NameKind;

/// Items: a name, a price, whether it stacks, a rarity and a picture.
pub(crate) fn items() -> Table {
    let mut table = Table::new("Item");
    table.columns = vec![
        Column::new("name", ColumnKind::Text),
        Column::new("price", ColumnKind::Whole),
        Column::new("stacks", ColumnKind::YesNo),
        Column::new(
            "rarity",
            ColumnKind::Choice(vec!["Common".into(), "Rare".into()]),
        ),
        Column::new("icon", ColumnKind::Name(NameKind::Picture)),
    ];
    let row = |id: &str, name: &str, price, rare: bool| {
        Arc::new(Row {
            id: id.into(),
            cells: vec![
                Cell::Text(name.into()),
                Cell::Whole(price),
                Cell::YesNo(!rare),
                Cell::Option(if rare { "Rare" } else { "Common" }.into()),
                Cell::Option("Apple".into()),
            ],
        })
    };
    table.rows = vec![
        row("Apple", "Apple", 5, false),
        row("IronSword", "Iron sword", 120, true),
        row("Potion", "Potion", 50, false),
    ];
    table
}

fn editor() -> TableEditor {
    TableEditor::new("data/items.table.lst".into(), items())
}

/// Makes `change` and checks it was one undo step that undo and redo
/// take back and give again exactly.
fn one_step(e: &mut TableEditor, change: impl FnOnce(&mut TableEditor)) {
    let before = e.file.clone();
    change(e);
    let after = e.file.clone();
    assert_ne!(before, after, "it changed the table");
    assert!(e.undo());
    assert_eq!(e.file, before, "undo puts it back");
    assert!(e.redo());
    assert_eq!(e.file, after, "redo makes it again");
}

#[test]
fn every_change_is_one_undo_step() {
    let mut e = editor();
    one_step(&mut e, |e| e.add_row().unwrap());
    one_step(&mut e, |e| e.add_column(ColumnKind::Colour).unwrap());
    let mut e = editor();
    one_step(&mut e, |e| e.rename_column(1, "cost").unwrap());
    one_step(&mut e, |e| assert!(e.delete_column(0)));
    let mut e = editor();
    one_step(&mut e, |e| assert!(e.move_column(0, 3)));
    one_step(&mut e, |e| assert!(e.set_translated(3, true)));
    let mut e = editor();
    one_step(&mut e, |e| assert!(e.set_kind(1, ColumnKind::Text)));
    one_step(&mut e, |e| {
        e.set_options(3, vec!["Plain".into(), "Shiny".into()])
            .unwrap();
    });
    let mut e = editor();
    one_step(&mut e, |e| e.set_row_name("Thing").unwrap());
    one_step(&mut e, |e| assert!(e.delete_rows(0..=1)));
    let mut e = editor();
    one_step(&mut e, |e| assert!(e.move_row(2, 0)));
    one_step(&mut e, |e| e.set_text((0, 2), "7").unwrap());
    let mut e = editor();
    one_step(&mut e, |e| e.set_text((1, 0), "SteelSword").unwrap());
    one_step(&mut e, |e| {
        e.pick((0, 1));
        e.anchor = Some((1, 2));
        assert!(e.clear());
    });
    let mut e = editor();
    one_step(&mut e, |e| {
        e.pick((2, 1));
        assert!(e.paste("Elixir\t99\nTonic\t3"));
    });
    one_step(&mut e, |e| {
        e.import_csv("id,price\nApple,1\nBread,2\n").unwrap();
    });
}

#[test]
fn a_change_that_changes_nothing_is_no_step() {
    let mut e = editor();
    e.set_text((0, 2), "5").unwrap();
    assert!(!e.set_cell((0, 1), Cell::Text("Apple".into())));
    e.set_row_name("Item").unwrap();
    assert!(!e.can_undo());
}

#[test]
fn what_cant_be_is_refused_in_plain_words() {
    let mut e = editor();
    assert!(
        e.set_text((0, 2), "lots")
            .unwrap_err()
            .contains("whole number")
    );
    assert!(
        e.set_text((0, 0), "IronSword")
            .unwrap_err()
            .contains("already")
    );
    assert!(
        e.set_text((0, 0), "iron sword")
            .unwrap_err()
            .contains("IronSword")
    );
    assert!(e.rename_column(0, "price").unwrap_err().contains("already"));
    assert!(e.rename_column(0, "id").is_err());
    assert!(e.set_options(3, Vec::new()).is_err());
    assert!(e.set_options(3, vec!["A".into(), "a".into()]).is_err());
    assert!(e.set_row_name("item").is_err());
    assert!(!e.can_undo());
}

#[test]
fn renamed_options_carry_their_cells() {
    let mut e = editor();
    e.set_options(3, vec!["Plain".into(), "Shiny".into()])
        .unwrap();
    assert_eq!(e.file.rows[1].cells[3], Cell::Option("Shiny".into()));
    // One taken out: its cells become the first.
    e.set_options(3, vec!["Plain".into()]).unwrap();
    assert_eq!(e.file.rows[1].cells[3], Cell::Option("Plain".into()));
}

#[test]
fn a_column_changing_kind_keeps_what_it_can() {
    let mut e = editor();
    e.set_kind(1, ColumnKind::Number);
    assert_eq!(e.file.rows[1].cells[1], Cell::Number(120.0));
    e.set_kind(0, ColumnKind::Whole);
    assert_eq!(
        e.file.rows[0].cells[0],
        Cell::Whole(0),
        "Apple isn't a number"
    );
}

#[test]
fn search_and_sort_show_rows_without_changing_the_file() {
    let mut e = editor();
    e.search = "sword".into();
    assert_eq!(e.shown(), [1]);
    e.search = "RARE".into();
    assert_eq!(e.shown(), [1]);
    e.search.clear();
    e.sort = Some(Sort {
        place: 2,
        rising: false,
    });
    assert_eq!(e.shown(), [1, 2, 0]);
    e.sort = Some(Sort {
        place: 0,
        rising: true,
    });
    assert_eq!(e.shown(), [0, 1, 2]);
    assert!(!e.can_undo());
    // The cursor moves among the rows as shown.
    e.sort = Some(Sort {
        place: 2,
        rising: false,
    });
    e.pick((1, 0));
    e.step(1, 0, false);
    assert_eq!(e.cursor, Some((2, 0)));
    e.step(5, 9, true);
    assert_eq!(e.cursor, Some((0, 5)));
    assert_eq!(e.anchor, Some((2, 0)));
}

#[test]
fn typing_is_kept_or_left() {
    let mut e = editor();
    e.pick((1, 2));
    e.start_typing(None);
    assert_eq!(e.typing.as_ref().unwrap().text, "120");
    e.typing.as_mut().unwrap().text = "150".into();
    e.confirm().unwrap();
    assert_eq!(e.file.rows[1].cells[1], Cell::Whole(150));
    e.start_typing(Some("x".into()));
    assert!(e.confirm().is_err(), "x isn't a number");
    assert!(e.typing.is_some(), "typing goes on");
    e.typing = None;
    assert_eq!(e.file.rows[1].cells[1], Cell::Whole(150));
}

#[test]
fn copying_gives_tab_separated_text() {
    let mut e = editor();
    e.pick((0, 0));
    e.anchor = Some((1, 2));
    assert_eq!(
        e.copy().unwrap(),
        "Apple\tApple\t5\r\nIronSword\tIron sword\t120\r\n"
    );
}

#[test]
fn a_full_table_says_why() {
    let mut table = Table::new("Thing");
    table.columns = (0..level::table::MAX_COLUMNS)
        .map(|n| Column::new(format!("c{n}"), ColumnKind::YesNo))
        .collect();
    let mut e = TableEditor::new("t.table.lst".into(), table);
    assert!(e.add_column(ColumnKind::Text).unwrap_err().contains("64"));
}

#[test]
fn a_file_changed_on_disk_is_one_step() {
    let mut e = editor();
    let mut other = items();
    other.rows.pop();
    crate::own_files::OpenFile::take_in(&mut e, other.clone());
    assert_eq!(e.file, other);
    assert!(e.undo());
    assert_eq!(e.file.rows.len(), 3);
}
