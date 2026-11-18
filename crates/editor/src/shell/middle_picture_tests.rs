//! The frame's middle and bottom, drawn for real: each tab's screen at
//! the design's size against its approved picture, the bands' colours
//! sampled against the tokens (and the design's own first screen), and
//! pictures written out at two sizes to look at beside the design's.

use super::{PanelLayout, RibbonTab};
use crate::interface_look::InterfaceLook;
use crate::picture_support::{
    Picture, design, drawn, editor_window, matches_approved, sample_project,
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

/// The sample project with `tab` in front, drawn at `size` in the
/// standard layout.
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

/// Where the bottom panel's bar starts and ends, and the status line's
/// top, in the sample project's Home `height` tall, as drawn.
fn foot(harness_bar: f32, height: f32) -> [f32; 3] {
    let status = height - size::STATUS_BAR_H;
    [harness_bar, harness_bar + size::BOTTOM_BAR_H, status]
}

#[test]
fn the_bottom_panel_and_the_status_line_are_the_design_s_colours() {
    let mut workbench = sample_project();
    workbench.show_panel(crate::panel::Panel::Problems);
    let Some(mut harness) = editor_window(workbench, InterfaceLook::default(), SIZES[0]) else {
        return;
    };
    harness.run();
    harness.run();
    let view = harness
        .state()
        .shell
        .drawn_at(crate::panel::Panel::Viewport);
    let level = harness.state().shell.drawn_at(crate::panel::Panel::Level);
    let home = drawn(&mut harness);
    let (view, level) = (view.expect("the view"), level.expect("Level"));
    let [bar, under, status] = foot(view.bottom(), SIZES[0][1]);
    for (what, point, token) in [
        // Right of the tabs, left of the tools.
        (
            "the bottom panel's bar",
            [view.center().x, bar + size::BOTTOM_BAR_H / 2.0],
            tokens::shelf_bar(),
        ),
        // Under Level's rows.
        (
            "the Level panel",
            [level.center().x, level.bottom() - 20.0],
            tokens::panel(),
        ),
        // Between the left's words and the right's.
        (
            "the status line",
            [view.center().x, status + size::STATUS_BAR_H / 2.0],
            tokens::code(),
        ),
    ] {
        assert_eq!(
            colour_at(&home, what, point, 1.0),
            token,
            "{what} at {point:?}"
        );
    }
    // The lines: the bar's top and foot, and the status line's top (left
    // of the edge's grip at the middle).
    let x = (view.left() + 40.0) as u32;
    assert_eq!(home.pixel(x, bar as u32), tokens::line());
    assert_eq!(home.pixel(x, under as u32 - 1), tokens::line());
    assert_eq!(home.pixel(x, status as u32), tokens::line());
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

/// Board 05: the bottom panel folded to one line and open, each against
/// its approved picture; folded, the view runs down to the line and the
/// status line stays where it was.
#[test]
fn the_bottom_panel_folded_and_open_are_their_approved_pictures() {
    let picture = |folded: bool| {
        let mut workbench = sample_project();
        workbench.show_panel(crate::panel::Panel::Problems);
        workbench.log(
            crate::console::LogLevel::Warning,
            "No sound: couldn't open the sound card. Everything else works.",
        );
        workbench.console.fix_time("16:02:42");
        workbench.fold_bottom(folded);
        let mut harness = editor_window(workbench, InterfaceLook::default(), SIZES[0])?;
        harness.run();
        harness.run();
        let view = harness
            .state()
            .shell
            .drawn_at(crate::panel::Panel::Viewport)?;
        Some((view, drawn(&mut harness)))
    };
    let (Some((open_view, open)), Some((folded_view, folded))) = (picture(false), picture(true))
    else {
        return;
    };
    let status = SIZES[0][1] - size::STATUS_BAR_H;
    assert_eq!(folded_view.bottom(), status - size::BOTTOM_BAR_H);
    assert!(folded_view.height() > open_view.height());
    let line = [folded_view.center().x, status + size::STATUS_BAR_H / 2.0];
    assert_eq!(
        folded.at(line, 1.0),
        open.at(line, 1.0),
        "the status line stays"
    );
    matches_approved("bottom-panel-open", &open);
    matches_approved("bottom-panel-folded", &folded);
}

/// Home in each layout (board 06) against its approved picture; the
/// standard one is `editor.png` (`picture_tests.rs`). Just the view is
/// drawn as chosen in Preferences, every panel hidden.
#[test]
fn home_in_each_layout_is_its_approved_picture() {
    for layout in PanelLayout::ALL
        .into_iter()
        .filter(|l| *l != PanelLayout::Standard)
    {
        // Problems in front in the bottom panel: the Console's lines
        // carry the time of day they were written.
        let mut workbench = sample_project();
        workbench.show_panel(crate::panel::Panel::Problems);
        workbench.choose_layout(&egui::Context::default(), layout);
        workbench.close_layout_note();
        let look = InterfaceLook {
            layout,
            ..InterfaceLook::default()
        };
        let Some(mut harness) = editor_window(workbench, look, SIZES[0]) else {
            return;
        };
        harness.run();
        harness.run();
        let picture = drawn(&mut harness);
        let name = format!("home-{}", layout.key().replace(' ', "-"));
        matches_approved(&name, &picture);
    }
}
