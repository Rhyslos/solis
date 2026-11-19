//! The scale check's case for the Table tab (stage 1, data tables): a
//! table of 10,000 rows and 20 columns opened, scrolled from top to
//! bottom with the mouse wheel and typed in cell by cell (each change
//! saved at once), with no frame over the editor's target. Only the rows
//! in view are drawn.

use crate::library::ProjectInfo;
use crate::sprite_sheets::TableState;
use crate::test_support::info;
use crate::viewport::NoRenderer;
use crate::workbench::Workbench;
use level::table::{Cell, Column, ColumnKind, Row, Table};
use std::sync::Arc;

/// The editor frame's target (as `layout_scale_tests`).
const TARGET_MS: f64 = 8.0;

/// A table of `rows` rows and 20 columns of every kind.
pub(crate) fn big_table(rows: usize) -> Table {
    let kinds = [
        ColumnKind::Text,
        ColumnKind::Whole,
        ColumnKind::Number,
        ColumnKind::YesNo,
        ColumnKind::Choice(vec!["Common".into(), "Rare".into()]),
    ];
    let mut table = Table::new("Thing");
    table.columns = (0..20)
        .map(|n| Column::new(format!("c{n}"), kinds[n % kinds.len()].clone()))
        .collect();
    table.rows = (0..rows)
        .map(|r| {
            let cells = table
                .columns
                .iter()
                .enumerate()
                .map(|(n, c)| match &c.kind {
                    ColumnKind::Text => Cell::Text(format!("words {r} {n}")),
                    ColumnKind::Whole => Cell::Whole((r * n) as i64),
                    ColumnKind::Number => Cell::Number(r as f64 / 4.0),
                    ColumnKind::YesNo => Cell::YesNo(r % 2 == 0),
                    _ => Cell::Option("Rare".into()),
                })
                .collect();
            Arc::new(Row {
                id: format!("T{r}"),
                cells,
            })
        })
        .collect();
    table
}

/// The editor with `table` open in the Table tab.
fn with(table: Table) -> Workbench {
    let mut w = Workbench::new(info());
    w.set_project(Some(ProjectInfo {
        name: "Big".into(),
        root: "big".into(),
        assets: "big".into(),
    }));
    Arc::make_mut(&mut w.viewport.sheets).tables.insert(
        "things.table.lst".into(),
        TableState::Read(Arc::new(table), Vec::new()),
    );
    w.open_table("things.table.lst");
    w
}

/// The slowest frames, in milliseconds, of opening the table, scrolling
/// it from top to bottom, and typing in 20 cells one after another.
fn slowest(w: Workbench) -> (f64, f64, f64) {
    let mut renderer = NoRenderer {
        reason: "testing".into(),
    };
    let mut h = egui_kittest::Harness::builder()
        .with_size(egui::vec2(1440.0, 900.0))
        .build_ui_state(
            move |ui, workbench: &mut Workbench| {
                if crate::test_support::fonts_ready(ui.ctx()) {
                    let _ = workbench.show(ui, &mut renderer);
                    workbench.open.outbox.clear();
                }
            },
            w,
        );
    h.run();
    let start = std::time::Instant::now();
    h.step();
    let open = start.elapsed().as_secs_f64() * 1000.0;
    h.hover_at(egui::pos2(700.0, 400.0));
    h.run();
    let mut scroll = 0.0_f64;
    for _ in 0..60 {
        h.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Page,
            delta: egui::vec2(0.0, -6.0),
            modifiers: egui::Modifiers::NONE,
            phase: egui::TouchPhase::Move,
        });
        let start = std::time::Instant::now();
        h.step();
        scroll = scroll.max(start.elapsed().as_secs_f64() * 1000.0);
    }
    // Each cell as a person types in it: picked, typed over, Enter.
    let mut edit = 0.0_f64;
    let mut timed = |h: &mut egui_kittest::Harness<'_, Workbench>| {
        let start = std::time::Instant::now();
        h.step();
        let ms = start.elapsed().as_secs_f64() * 1000.0;
        if std::env::var_os("SOLIS_TABLE_FRAMES").is_some() {
            eprintln!("frame {ms:.2} ms");
        }
        edit = edit.max(ms);
    };
    for n in 0..20 {
        // A person takes a moment to type each value: by then the last
        // save's text, written on a worker, is done (it isn't timed).
        std::thread::sleep(std::time::Duration::from_millis(60));
        if let Some(e) = h.state_mut().open.table_editor.as_mut() {
            e.pick((n * 400, 2));
        }
        timed(&mut h);
        if let Some(e) = h.state_mut().open.table_editor.as_mut() {
            e.start_typing(Some(format!("{n}")));
        }
        timed(&mut h);
        h.key_press(egui::Key::Enter);
        timed(&mut h);
        timed(&mut h);
    }
    let editor = h.state().open.table_editor.as_ref().expect("still open");
    assert_eq!(editor.file.rows[19 * 400].cells[1], Cell::Whole(19));
    (open, scroll, edit)
}

#[test]
fn a_big_table_scrolls_and_takes_typing() {
    // Small enough for an ordinary run: the case itself is below.
    let (open, scroll, edit) = slowest(with(big_table(8_000)));
    assert!(open.max(scroll).max(edit) < 2_000.0);
}

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
#[allow(clippy::print_stdout)]
fn scale_check_a_table_of_10000_rows_and_20_columns() {
    let (open, scroll, edit) = slowest(with(big_table(10_000)));
    for (what, ms) in [("opened", open), ("scrolled", scroll), ("typed in", edit)] {
        let verdict = if ms <= TARGET_MS { "ok" } else { "SLOW" };
        println!(
            "Scale check: a table of 10,000 rows and 20 columns {what}, slowest frame {ms:.2} ms \
             (target {TARGET_MS} ms) {verdict}"
        );
    }
    assert!(open.max(scroll).max(edit) <= TARGET_MS);
}
