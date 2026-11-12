//! The frame's middle and bottom, drawn for real: each tab's screen at
//! the design's size against its approved picture, the bands' colours
//! sampled against the tokens (and the design's own first screen), and
//! pictures written out at two sizes to look at beside the design's.

use super::RibbonTab;
use crate::interface_look::InterfaceLook;
use crate::picture_support::{
    DESIGN_SCALE, Picture, design, drawn, editor_window, matches_approved, sample_project,
};
use crate::tokens::{self, size};

/// The design's size, and a smaller window the editor must still fit.
const SIZES: [[f32; 2]; 2] = [[1440.0, 900.0], [1100.0, 700.0]];

/// The tabs with a screen of their own, each with the design's picture
/// of it where the design has one.
const TABS: [(RibbonTab, &str, Option<&str>); 7] = [
    (RibbonTab::Home, "home", Some("01-home-level-editor.png")),
    (RibbonTab::Library, "library", Some("03-library.png")),
    (
        RibbonTab::Animate,
        "animate",
        Some("05-animate-sequencer.png"),
    ),
    (RibbonTab::Ui, "ui", None),
    (RibbonTab::Audio, "audio", Some("10-audio-mixer.png")),
    (
        RibbonTab::Debug,
        "debug",
        Some("06-debug-profiler-and-console.png"),
    ),
    (RibbonTab::Build, "build", None),
];

/// The sample project with `tab` in front, drawn at `size`.
fn tab_picture(tab: RibbonTab, size: [f32; 2]) -> Option<Picture> {
    let mut workbench = sample_project();
    workbench.show_tab(tab);
    let mut harness = editor_window(workbench, InterfaceLook::default(), size)?;
    harness.run();
    harness.run();
    Some(drawn(&mut harness))
}

/// A point is sampled only where the frame is one colour for three
/// pixels every way.
fn colour_at(picture: &Picture, what: &str, point: [f32; 2], scale: f32) -> egui::Color32 {
    assert!(
        picture.is_flat_at(point, scale, 3),
        "{what} at {point:?} isn't a flat surface: pick another point"
    );
    picture.at(point, scale)
}

/// Where the bands end, top to bottom, in a window `height` tall with the
/// standard shelf: the ribbon's foot, the shelf bar's top and foot, and
/// the status bar's top.
fn bands(height: f32) -> [f32; 4] {
    let ribbon = size::TAB_BAR_H + size::RIBBON_H;
    let status = height - size::STATUS_BAR_H;
    let shelf = status - size::SHELF_H - size::SHELF_BAR_H;
    [ribbon, shelf, shelf + size::SHELF_BAR_H, status]
}

#[test]
fn the_shelf_and_the_status_bar_are_the_design_s_colours() {
    let Some(home) = tab_picture(RibbonTab::Home, SIZES[0]) else {
        return;
    };
    let [_, shelf, columns, status] = bands(SIZES[0][1]);
    // The same surfaces in the design's first screen (its bands sit where
    // its own shelf is, at its own height).
    let design = design("01-home-level-editor.png");
    for (what, point, token) in [
        (
            "the shelf bar",
            [600.0, 500.0 + size::SHELF_BAR_H / 2.0],
            tokens::shelf_bar(),
        ),
        ("a column", [200.0, 680.0], tokens::panel()),
        ("the status bar", [700.0, 889.0], tokens::code()),
    ] {
        assert_eq!(
            colour_at(&design, what, point, DESIGN_SCALE),
            token,
            "the design's {what}"
        );
    }
    for (what, point, token) in [
        // Between the last shelf tab and the tools at the right.
        (
            "the shelf bar",
            [800.0, shelf + size::SHELF_BAR_H / 2.0],
            tokens::shelf_bar(),
        ),
        // A column's header, right of its title.
        (
            "a column's header",
            [100.0, columns + size::COLUMN_HEADER_H / 2.0],
            tokens::panel(),
        ),
        // Between the version and the context, and left of the counts.
        (
            "the status bar",
            [700.0, status + size::STATUS_BAR_H / 2.0],
            tokens::code(),
        ),
    ] {
        assert_eq!(
            colour_at(&home, what, point, 1.0),
            token,
            "{what} at {point:?}"
        );
    }
    // The lines: the shelf bar's top and foot, and the status bar's top.
    assert_eq!(home.pixel(800, shelf as u32), tokens::line());
    assert_eq!(home.pixel(800, columns as u32 - 1), tokens::line());
    assert_eq!(home.pixel(800, status as u32), tokens::line());
    // The tab strip's ground right of the area's tab and its +.
    let strip = bands(SIZES[0][1])[0] + size::DOC_TAB_H / 2.0;
    assert_eq!(
        colour_at(&home, "the tab strip", [800.0, strip], 1.0),
        tokens::app()
    );
}

#[test]
fn every_tab_s_screen_is_its_approved_picture() {
    // Home is `editor.png` (`picture_tests.rs`). Debug's center shows the
    // Profiler's times, which are this machine's and this moment's: it is
    // looked at, not compared; Build's Console in front shows the times
    // of day its lines were written.
    let compared =
        |tab: &RibbonTab| !matches!(tab, RibbonTab::Home | RibbonTab::Debug | RibbonTab::Build);
    for (tab, name, _) in TABS.into_iter().filter(|(tab, ..)| compared(tab)) {
        let Some(picture) = tab_picture(tab, SIZES[0]) else {
            return;
        };
        matches_approved(&format!("tab-{name}"), &picture);
    }
}

/// Writes each tab's screen at both sizes, the design's own picture of
/// it beside the larger one where there is one. Run with `--ignored` and
/// `SOLIS_PICTURE_SHEET` set to the folder to write into.
#[test]
#[ignore = "writes pictures to look at; needs SOLIS_PICTURE_SHEET and a graphics driver"]
fn each_tab_at_two_sizes_beside_the_design() {
    let folder = std::env::var_os("SOLIS_PICTURE_SHEET").expect("SOLIS_PICTURE_SHEET");
    let folder = std::path::Path::new(&folder);
    for (tab, name, screen) in TABS {
        for size in SIZES {
            let Some(picture) = tab_picture(tab, size) else {
                return;
            };
            let file = format!("middle-{name}-{}.png", size[0]);
            match screen.filter(|_| size == SIZES[0]) {
                Some(screen) => design(screen)
                    .halved()
                    .beside(&picture, 24, tokens::text_meta())
                    .write(&folder.join(file)),
                None => picture.write(&folder.join(file)),
            }
        }
    }
}

#[test]
fn the_grip_and_a_fold_change_only_the_shelf() {
    let Some(open) = tab_picture(RibbonTab::Home, SIZES[0]) else {
        return;
    };
    let mut workbench = sample_project();
    workbench.fold_shelf(true);
    let Some(mut harness) = editor_window(workbench, InterfaceLook::default(), SIZES[0]) else {
        return;
    };
    harness.run();
    harness.run();
    let folded = drawn(&mut harness);
    // Folded, the bar sits on the status bar, and the center is taller.
    let [_, _, _, status] = bands(SIZES[0][1]);
    let bar = status - size::SHELF_BAR_H / 2.0;
    assert_eq!(
        colour_at(&folded, "the folded bar", [800.0, bar], 1.0),
        tokens::shelf_bar()
    );
    assert_ne!(open.at([800.0, bar], 1.0), tokens::shelf_bar());
    // The status bar is where it was.
    assert_eq!(
        folded.at([700.0, status + size::STATUS_BAR_H / 2.0], 1.0),
        open.at([700.0, status + size::STATUS_BAR_H / 2.0], 1.0)
    );
}
