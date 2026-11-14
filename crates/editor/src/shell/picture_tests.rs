//! The frame's top, drawn for real: its colours sampled against the
//! tokens, and pictures written out to look at beside the design's.

use super::RibbonTab;
use crate::interface_look::InterfaceLook;
use crate::picture_support::{Picture, drawn, editor_window, sample_project};
use crate::tokens::{self, size};
use crate::workbench::Workbench;

const SIZE: [f32; 2] = [1440.0, 900.0];

/// The editor after `set_up`, drawn in `look`.
fn picture(look: InterfaceLook, set_up: impl FnOnce(&mut Workbench)) -> Option<Picture> {
    let mut workbench = sample_project();
    set_up(&mut workbench);
    let mut harness = editor_window(workbench, look, SIZE)?;
    harness.run();
    harness.run();
    Some(drawn(&mut harness))
}

/// Writes the frame's top on each of its screens, to look at beside the
/// design's screens and the approved additions. Run with `--ignored` and
/// `SOLIS_PICTURE_SHEET` set to the folder to write into.
#[test]
#[ignore = "writes pictures to look at; needs SOLIS_PICTURE_SHEET and a graphics driver"]
fn the_top_on_each_screen() {
    let folder = std::env::var_os("SOLIS_PICTURE_SHEET").expect("SOLIS_PICTURE_SHEET");
    let folder = std::path::Path::new(&folder);
    let shown = InterfaceLook::default();
    let hidden = InterfaceLook {
        hide_unfinished: true,
        ..shown
    };
    type SetUp = fn(&mut Workbench);
    let screens: [(&str, InterfaceLook, SetUp); 10] = [
        ("home", shown, |_| {}),
        ("home-playing", shown, |w| w.play()),
        ("home-paused", shown, |w| {
            w.play();
            w.pause();
        }),
        ("home-hidden", hidden, |_| {}),
        ("debug", shown, |w| w.show_tab(RibbonTab::Debug)),
        ("library", shown, |w| w.show_tab(RibbonTab::Library)),
        ("shading", shown, |w| w.show_tab(RibbonTab::Shading)),
        ("file", shown, |w| w.show_tab(RibbonTab::File)),
        ("collapsed", shown, |w| w.set_ribbon_collapsed(true)),
        ("collapsed-dropped", shown, |w| {
            w.set_ribbon_collapsed(true);
            w.tab_clicked(RibbonTab::Home);
        }),
    ];
    for (name, look, set_up) in screens {
        let Some(picture) = picture(look, set_up) else {
            return;
        };
        picture.write(&folder.join(format!("top-{name}.png")));
    }
}

/// A point is sampled only where the frame is one colour for three
/// pixels every way.
fn colour_at(picture: &Picture, what: &str, point: [f32; 2]) -> egui::Color32 {
    assert!(
        picture.is_flat_at(point, 1.0, 3),
        "{what} at {point:?} isn't a flat surface: pick another point"
    );
    picture.at(point, 1.0)
}

#[test]
fn the_tab_bar_and_the_ribbon_are_the_design_s_colours() {
    let Some(home) = picture(InterfaceLook::default(), |_| {}) else {
        return;
    };
    let ribbon_middle = size::TAB_BAR_H + size::RIBBON_H / 2.0;
    for (what, point, token) in [
        // Between the last tab and the search box.
        ("the tab bar", [645.0, 17.0], tokens::app()),
        // Beside the words of the tab in front.
        ("the tab in front", [96.0, 10.0], tokens::ribbon()),
        ("the search box", [1000.0, 17.0], tokens::ribbon()),
        // Between Project and the Play group.
        ("the ribbon", [1150.0, ribbon_middle], tokens::ribbon()),
        ("Play", [1336.0, 50.0], tokens::PLAY_FILL),
    ] {
        assert_eq!(colour_at(&home, what, point), token, "{what} at {point:?}");
    }
    // The line under the ribbon, and the accent's under the tab in front.
    let foot = (size::TAB_BAR_H + size::RIBBON_H) as u32 - 1;
    assert_eq!(home.pixel(1150, foot), tokens::line());
    assert_eq!(
        home.pixel(100, size::TAB_BAR_H as u32 - 1),
        crate::interface_look::accent_fill()
    );
}

#[test]
fn nothing_changes_colour_while_the_game_runs_but_play_becoming_stop() {
    let Some(editing) = picture(InterfaceLook::default(), |_| {}) else {
        return;
    };
    let Some(playing) = picture(InterfaceLook::default(), |w| w.play()) else {
        return;
    };
    // The bars that were blue while playing, and the window's edge.
    for point in [
        [700.0, 17.0],
        [700.0, 889.0],
        [1.0, 700.0],
        [1438.0, 450.0],
        [700.0, 1.0],
    ] {
        assert_eq!(
            playing.at(point, 1.0),
            editing.at(point, 1.0),
            "no tint at {point:?}"
        );
    }
}

/// Documentation is drawn as the tab bar's tabs that aren't in front
/// are: its words in the tab's grey, on the tab bar's own colour, at the
/// bar's right end between the project and the chevron (decision 0112).
#[test]
fn documentation_is_drawn_as_the_tab_bar_s_words() {
    use egui_kittest::kittest::Queryable;
    let Some(mut harness) = editor_window(sample_project(), InterfaceLook::default(), SIZE) else {
        return;
    };
    harness.run();
    harness.run();
    let rect = harness
        .get_by_label(super::documentation::DOCUMENTATION)
        .rect();
    let chevron = harness.get_by_label("Collapse the ribbon").rect();
    let picture = drawn(&mut harness);
    assert!(rect.right() <= chevron.left() && rect.left() > SIZE[0] / 2.0);
    assert_eq!(
        colour_at(
            &picture,
            "beside Documentation",
            [rect.left() + 2.0, rect.top() + 3.0]
        ),
        tokens::app()
    );
    let brightest = (rect.left() as u32..rect.right() as u32)
        .flat_map(|x| (rect.top() as u32..rect.bottom() as u32).map(move |y| (x, y)))
        .map(|(x, y)| picture.pixel(x, y))
        .max_by_key(|c| c.r())
        .unwrap();
    let ink = tokens::text_label();
    assert!(
        brightest.r().abs_diff(ink.r()) <= 6 && brightest.r() == brightest.g(),
        "Documentation's words are {brightest:?}, the tab's grey is {ink:?}"
    );
}
