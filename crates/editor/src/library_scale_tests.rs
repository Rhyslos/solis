//! The scale check's case for the Library browser (R4, decision 0108):
//! a folder of 5,000 files scrolled from top to bottom, on Home's bottom
//! panel and on the Library tab, as tiles and as a list, and in two
//! folder trees side by side, with no frame over the editor's target.

use crate::library::{LeftShows, LibraryItem, LibraryKind, ProjectInfo};
use crate::shell::RibbonTab;
use crate::test_support::info;
use crate::viewport::NoRenderer;
use crate::workbench::Workbench;

/// The editor frame's target (as `layout_scale_tests`).
const TARGET_MS: f64 = 8.0;

/// The editor with `count` files in one folder, the Library shown there.
fn a_full_folder(count: usize) -> Workbench {
    let mut w = Workbench::new(info());
    w.set_project(Some(ProjectInfo {
        name: "Big".into(),
        root: "big".into(),
        assets: "big".into(),
    }));
    w.set_folders(vec!["farm".into(), "other".into()]);
    let kinds = [
        LibraryKind::Model,
        LibraryKind::Texture,
        LibraryKind::Sound,
        LibraryKind::Level,
    ];
    w.set_library(
        (0..count)
            .map(|i| LibraryItem {
                id: None,
                kind: kinds[i % kinds.len()],
                name: format!("file {i:05}"),
                path: format!("farm/file {i:05}.png"),
                thumbnail: None,
            })
            .collect(),
    );
    w.open.library_state.open_folder("farm");
    w
}

/// The slowest frame of `w` at 1440 × 900 while the mouse wheel scrolls
/// over `at` from the top of the files to the bottom, in milliseconds.
fn slowest_while_scrolling(w: Workbench, at: egui::Pos2) -> f64 {
    let mut renderer = NoRenderer {
        reason: "testing".into(),
    };
    let mut h = egui_kittest::Harness::builder()
        .with_size(egui::vec2(1440.0, 900.0))
        .build_ui_state(
            move |ui, workbench: &mut Workbench| {
                if crate::test_support::fonts_ready(ui.ctx()) {
                    let _ = workbench.show(ui, &mut renderer);
                }
            },
            w,
        );
    h.run();
    h.run();
    h.hover_at(at);
    h.run();
    let mut slowest = 0.0_f64;
    for _ in 0..60 {
        h.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Page,
            delta: egui::vec2(0.0, -1.0),
            modifiers: egui::Modifiers::NONE,
            phase: egui::TouchPhase::Move,
        });
        let start = std::time::Instant::now();
        h.step();
        slowest = slowest.max(start.elapsed().as_secs_f64() * 1000.0);
    }
    slowest
}

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
#[allow(clippy::print_stdout)]
fn scale_check_a_folder_of_5000_files_scrolls_without_a_late_frame() {
    let mut cases: Vec<(&str, Workbench, egui::Pos2)> = Vec::new();
    for list in [false, true] {
        let mut home = a_full_folder(5000);
        home.open.library_state.list = list;
        cases.push((
            if list { "Home, list" } else { "Home, tiles" },
            home,
            egui::pos2(700.0, 780.0),
        ));
        let mut tab = a_full_folder(5000);
        tab.open.library_state.list = list;
        tab.show_tab(RibbonTab::Library);
        cases.push((
            if list {
                "Library tab, list"
            } else {
                "Library tab, tiles"
            },
            tab,
            egui::pos2(700.0, 500.0),
        ));
    }
    let mut trees = a_full_folder(5000);
    trees.open.library_state.left = LeftShows::Folders;
    trees.open.library_state.trees.list[0]
        .open
        .insert("farm".into());
    trees.open.library_state.trees.open_another(0);
    cases.push(("two folder trees", trees, egui::pos2(60.0, 500.0)));
    let mut slowest = 0.0_f64;
    for (what, w, at) in cases {
        let ms = slowest_while_scrolling(w, at);
        let verdict = if ms <= TARGET_MS { "ok" } else { "SLOW" };
        println!(
            "Scale check: the Library ({what}) scrolled through 5,000 files, slowest frame \
             {ms:.2} ms (target {TARGET_MS} ms) {verdict}"
        );
        slowest = slowest.max(ms);
    }
    assert!(slowest <= TARGET_MS, "a frame took {slowest:.2} ms");
}
