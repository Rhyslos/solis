//! The Table tab used as a person uses it: opened from the Library beside
//! the level's view, cells picked, typed in and moved through with the
//! spreadsheet keys, copied and pasted, each change saved; the tab's
//! approved pictures; and the scale case, a table of 10,000 rows and 20
//! columns drawn and scrolled within the editor's frame target.

use crate::panel::Panel;
use crate::sprite_sheets::TableState;
use crate::table_view::{HEAD_H, ROW_H};
use crate::test_support::{click, double_click, harness};
use crate::workbench::{Workbench, WorkbenchAction};
use level::table::{Cell, Column, ColumnKind, Row, Table};
use std::sync::Arc;

/// Where the items' table is in the project.
pub(crate) const PATH: &str = "data/items.table.lst";

/// The sample project with `table` read at [`PATH`] and open in the
/// Table tab.
pub(crate) fn with_table(table: Table) -> Workbench {
    let mut w = crate::picture_support::sample_project();
    Arc::make_mut(&mut w.viewport.sheets)
        .tables
        .insert(PATH.into(), TableState::Read(Arc::new(table), Vec::new()));
    w.open_table(PATH);
    w
}

/// The items as the sketch has them.
pub(crate) fn sketch_items() -> Table {
    let mut table = Table::new("Item");
    table.columns = vec![
        Column {
            translated: true,
            ..Column::new("name", ColumnKind::Text)
        },
        Column::new("price", ColumnKind::Whole),
        Column::new("icon", ColumnKind::Name(reflect::NameKind::Picture)),
        Column::new(
            "rarity",
            ColumnKind::Choice(vec!["Common".into(), "Rare".into()]),
        ),
        Column::new("stacks", ColumnKind::YesNo),
        Column::new("tint", ColumnKind::Colour),
        Column {
            translated: true,
            ..Column::new("description", ColumnKind::Text)
        },
    ];
    let row = |id: &str, name: &str, price, icon: &str, rare: bool, tint, words: &str| {
        Arc::new(Row {
            id: id.into(),
            cells: vec![
                Cell::Text(name.into()),
                Cell::Whole(price),
                Cell::Option(icon.into()),
                Cell::Option(if rare { "Rare" } else { "Common" }.into()),
                Cell::YesNo(!rare),
                Cell::Colour(tint),
                Cell::Text(words.into()),
            ],
        })
    };
    table.rows = vec![
        row(
            "Apple",
            "Apple",
            5,
            "Items.Apple",
            false,
            [0.86, 0.2, 0.2, 1.0],
            "A crisp red apple.",
        ),
        row(
            "IronSword",
            "Iron sword",
            120,
            "Items.SwordIron",
            true,
            [0.62, 0.65, 0.7, 1.0],
            "Heavy, but it holds an edge.",
        ),
        row(
            "Potion",
            "Potion",
            50,
            "Items.Potion",
            false,
            [0.3, 0.75, 0.45, 1.0],
            "Heals 20.",
        ),
    ];
    table
}

/// The text the last save of the table wrote, if one was asked for.
fn last_saved(actions: &[WorkbenchAction]) -> Option<String> {
    actions.iter().rev().find_map(|a| match a {
        WorkbenchAction::SaveFile { path, text, .. } if path.to_string_lossy() == PATH => {
            Some(text.clone())
        }
        _ => None,
    })
}

/// Where the cell at `row` (as shown) and `place` is on screen.
fn cell_at(
    h: &egui_kittest::Harness<'static, crate::test_support::State>,
    row: usize,
    place: usize,
) -> egui::Pos2 {
    let editor = h.state().0.open.table_editor.as_ref().unwrap();
    let (x, y) = editor.drawn_at.expect("the grid was drawn");
    let left: f32 = std::iter::once(160.0)
        .chain(
            editor
                .file
                .columns
                .iter()
                .map(|c| crate::table_view::width_of(&c.kind)),
        )
        .take(place)
        .sum();
    egui::pos2(
        x + left + 20.0,
        y + HEAD_H + ROW_H * row as f32 + ROW_H / 2.0,
    )
}

/// Items narrow enough that every column is in view: a name, a price,
/// whether it stacks and a rarity.
fn small_items() -> Table {
    let mut table = sketch_items();
    let keep = [0, 1, 4, 3];
    table.columns = keep.iter().map(|&c| table.columns[c].clone()).collect();
    for row in &mut table.rows {
        let cells = keep.iter().map(|&c| row.cells[c].clone()).collect();
        Arc::make_mut(row).cells = cells;
    }
    table
}

#[test]
fn a_table_opens_beside_the_level_s_view_on_home() {
    let mut h = harness(with_table(small_items()));
    for _ in 0..4 {
        h.run();
    }
    let w = &h.state().0;
    assert_eq!(w.ribbon_tab(), crate::shell::RibbonTab::Home);
    assert!(
        w.shell.drawn_at(Panel::Table).is_some(),
        "the Table tab is in view"
    );
}

#[test]
fn enter_types_in_a_cell_and_enter_again_keeps_it_and_moves_down() {
    let mut h = harness(with_table(small_items()));
    for _ in 0..4 {
        h.run();
    }
    let at = cell_at(&h, 1, 2);
    click(&mut h, at, egui::Modifiers::NONE);
    assert_eq!(
        h.state().0.open.table_editor.as_ref().unwrap().cursor,
        Some((1, 2))
    );
    h.key_press(egui::Key::Enter);
    h.run();
    h.run();
    let typing = h
        .state()
        .0
        .open
        .table_editor
        .as_ref()
        .unwrap()
        .typing
        .clone();
    assert_eq!(typing.map(|t| t.text), Some("120".to_string()));
    h.key_press(egui::Key::Backspace);
    h.key_press(egui::Key::Backspace);
    h.key_press(egui::Key::Backspace);
    h.event(egui::Event::Text("150".into()));
    h.run();
    h.key_press(egui::Key::Enter);
    h.run();
    // The table's text is written on a worker: wait for it, as leaving
    // the project does.
    h.state_mut().0.poll_table_saves(true);
    h.run();
    let w = &h.state().0;
    let editor = w.open.table_editor.as_ref().unwrap();
    assert_eq!(editor.file.rows[1].cells[1], Cell::Whole(150));
    assert_eq!(editor.cursor, Some((2, 2)), "down one");
    assert!(
        last_saved(&h.state().1.actions)
            .unwrap()
            .contains("\"150\""),
        "saved at once"
    );
}

#[test]
fn a_key_typed_starts_typing_over_the_cell_and_esc_leaves_it() {
    let mut h = harness(with_table(small_items()));
    for _ in 0..4 {
        h.run();
    }
    let at = cell_at(&h, 0, 1);
    click(&mut h, at, egui::Modifiers::NONE);
    h.event(egui::Event::Text("P".into()));
    h.run();
    h.run();
    let typing = h
        .state()
        .0
        .open
        .table_editor
        .as_ref()
        .unwrap()
        .typing
        .clone();
    assert_eq!(typing.map(|t| t.text), Some("P".to_string()));
    h.key_press(egui::Key::Escape);
    h.run();
    let editor = h.state().0.open.table_editor.as_ref().unwrap();
    assert!(editor.typing.is_none());
    assert_eq!(editor.file.rows[0].cells[0], Cell::Text("Apple".into()));
}

#[test]
fn tab_and_the_arrows_move_and_delete_empties() {
    let mut h = harness(with_table(small_items()));
    for _ in 0..4 {
        h.run();
    }
    let at = cell_at(&h, 0, 0);
    click(&mut h, at, egui::Modifiers::NONE);
    h.key_press(egui::Key::Tab);
    h.run();
    h.key_press(egui::Key::Tab);
    h.run();
    h.key_press(egui::Key::ArrowDown);
    h.run();
    assert_eq!(
        h.state().0.open.table_editor.as_ref().unwrap().cursor,
        Some((1, 2))
    );
    h.key_press(egui::Key::Delete);
    h.run();
    let editor = h.state().0.open.table_editor.as_ref().unwrap();
    assert_eq!(editor.file.rows[1].cells[1], Cell::Whole(0));
    // Ctrl+Z over the tab undoes it.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    let editor = h.state().0.open.table_editor.as_ref().unwrap();
    assert_eq!(editor.file.rows[1].cells[1], Cell::Whole(120));
}

#[test]
fn pasted_cells_land_at_the_cursor() {
    let mut h = harness(with_table(small_items()));
    for _ in 0..4 {
        h.run();
    }
    let at = cell_at(&h, 2, 1);
    click(&mut h, at, egui::Modifiers::NONE);
    h.event(egui::Event::Paste("Elixir\t75\nTonic\t3".into()));
    h.run();
    let editor = h.state().0.open.table_editor.as_ref().unwrap();
    assert_eq!(editor.file.rows.len(), 4);
    assert_eq!(editor.file.rows[2].cells[0], Cell::Text("Elixir".into()));
    assert_eq!(editor.file.rows[3].cells[1], Cell::Whole(3));
}

#[test]
fn a_click_on_a_yes_or_no_cell_flips_it() {
    let mut h = harness(with_table(small_items()));
    for _ in 0..4 {
        h.run();
    }
    let at = cell_at(&h, 0, 3);
    click(&mut h, at, egui::Modifiers::NONE);
    let editor = h.state().0.open.table_editor.as_ref().unwrap();
    assert_eq!(editor.file.rows[0].cells[2], Cell::YesNo(false));
}

#[test]
fn a_double_click_types_and_a_choice_lists_its_options() {
    let mut h = harness(with_table(small_items()));
    for _ in 0..4 {
        h.run();
    }
    let at = cell_at(&h, 0, 4);
    double_click(&mut h, at);
    h.run();
    h.key_press(egui::Key::Backspace);
    h.key_press(egui::Key::Backspace);
    h.key_press(egui::Key::Backspace);
    h.key_press(egui::Key::Backspace);
    h.key_press(egui::Key::Backspace);
    h.key_press(egui::Key::Backspace);
    h.event(egui::Event::Text("rare".into()));
    h.run();
    h.key_press(egui::Key::Enter);
    h.run();
    let editor = h.state().0.open.table_editor.as_ref().unwrap();
    assert_eq!(editor.file.rows[0].cells[3], Cell::Option("Rare".into()));
}

#[test]
fn new_table_makes_a_file_with_a_free_name_and_opens_it() {
    let mut w = crate::picture_support::sample_project();
    let mut actions = Vec::new();
    w.library_request(
        crate::library::LibraryRequest::NewTable("data".into()),
        &mut actions,
    );
    let (path, text) = w
        .open
        .outbox
        .iter()
        .find_map(|a| match a {
            WorkbenchAction::SaveFile { path, text, .. } => Some((path.clone(), text.clone())),
            _ => None,
        })
        .unwrap();
    assert_eq!(path.to_string_lossy(), "data/table.table.lst");
    let read = level::table::file::read(&text, "X").unwrap();
    assert_eq!(read.table.row_name, "TableRow");
    assert_eq!(read.table.rows.len(), 1);
    assert_eq!(
        w.open.table_editor.as_ref().unwrap().path,
        "data/table.table.lst"
    );
}

#[test]
fn csv_goes_out_beside_the_table_and_comes_back_as_one_step() {
    let mut w = with_table(sketch_items());
    w.export_table_csv();
    let (path, text) = w
        .open
        .outbox
        .iter()
        .find_map(|a| match a {
            WorkbenchAction::ExportCsv { path, text } => Some((path.clone(), text.clone())),
            _ => None,
        })
        .unwrap();
    assert_eq!(path.to_string_lossy(), "data/items.csv");
    w.ask_table_csv();
    assert!(
        w.open
            .outbox
            .iter()
            .any(|a| matches!(a, WorkbenchAction::ReadCsv { .. }))
    );
    let changed = text.replace("Iron sword", "Steel sword");
    w.import_table_csv("data/items.csv", Ok(changed));
    let editor = w.open.table_editor.as_ref().unwrap();
    assert_eq!(
        editor.file.rows[1].cells[0],
        Cell::Text("Steel sword".into())
    );
    w.table_undo(false, true);
    let editor = w.open.table_editor.as_ref().unwrap();
    assert_eq!(
        editor.file.rows[1].cells[0],
        Cell::Text("Iron sword".into())
    );
    w.import_table_csv("data/items.csv", Err("there's no data/items.csv".into()));
    assert!(
        w.log_lines()
            .last()
            .unwrap()
            .text
            .contains("Couldn't take in")
    );
}

#[test]
fn a_changed_table_warns_once_when_a_recording_plays_back() {
    let mut w = with_table(sketch_items());
    let now = w.tables_now().hash();
    w.warn_if_tables_changed(Some(now), "it was recorded");
    w.warn_if_tables_changed(None, "it was recorded");
    let before = w.log_lines().len();
    w.warn_if_tables_changed(Some(now ^ 1), "play.solrec was recorded");
    assert_eq!(w.log_lines().len(), before + 1);
    assert!(
        w.log_lines()
            .last()
            .unwrap()
            .text
            .contains("data tables have changed since play.solrec was recorded")
    );
}

#[test]
fn a_file_changed_on_disk_is_shown_and_one_gone_closes_the_tab() {
    let mut w = with_table(sketch_items());
    let mut sheets = (*w.viewport.sheets).clone();
    let mut other = sketch_items();
    other.rows.pop();
    sheets
        .tables
        .insert(PATH.into(), TableState::Read(Arc::new(other), Vec::new()));
    w.set_sprite_sheets(sheets.clone());
    assert_eq!(w.open.table_editor.as_ref().unwrap().file.rows.len(), 2);
    sheets.tables.clear();
    w.set_sprite_sheets(sheets);
    assert!(w.open.table_editor.is_none());
}

/// Writes the window with the Table tab to `SOLIS_SHOTS` (a folder), to
/// look at beside the mock-up.
#[test]
#[ignore = "writes a picture to look at"]
fn shot_of_the_table_tab() {
    let Some(folder) = std::env::var_os("SOLIS_SHOTS") else {
        return;
    };
    let w = with_table(sketch_items());
    let Some(mut h) = crate::picture_support::editor_window(
        w,
        crate::interface_look::InterfaceLook::default(),
        [1440.0, 900.0],
    ) else {
        return;
    };
    for _ in 0..5 {
        h.run();
    }
    crate::picture_support::drawn(&mut h)
        .write(&std::path::Path::new(&folder).join("table-tab.png"));
}

/// The Table tab as the sketch has it: the items, their kinds under each
/// heading, the cursor's row picked, and how game code reads it.
#[test]
fn the_table_tab_matches_its_approved_picture() {
    let w = with_table(sketch_items());
    let Some(mut h) = crate::picture_support::editor_window(
        w,
        crate::interface_look::InterfaceLook::default(),
        [1440.0, 900.0],
    ) else {
        return;
    };
    for _ in 0..5 {
        h.run();
    }
    let picture = crate::picture_support::drawn(&mut h).cropped([252, 100], [868, 540]);
    crate::picture_support::matches_approved("table-tab", &picture);
}

/// A cell being typed in, with a choice's options listed under it.
#[test]
fn a_choice_being_typed_matches_its_approved_picture() {
    let mut w = with_table(sketch_items());
    if let Some(editor) = w.open.table_editor.as_mut() {
        editor.pick((1, 4));
        editor.start_typing(Some("R".into()));
    }
    let Some(mut h) = crate::picture_support::editor_window(
        w,
        crate::interface_look::InterfaceLook::default(),
        [1440.0, 900.0],
    ) else {
        return;
    };
    for _ in 0..5 {
        h.run();
    }
    let picture = crate::picture_support::drawn(&mut h).cropped([252, 100], [868, 540]);
    crate::picture_support::matches_approved("table-typing", &picture);
}
