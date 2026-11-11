//! The editor's look, held against the project lead's design.
//!
//! - **The design's pictures are made of the tokens:** named points on
//!   flat surfaces of the screens in `docs/design/g-layout/` are exactly
//!   the colours `tokens.rs` gives those surfaces. Needs no graphics
//!   driver.
//! - **The editor is drawn in the tokens:** the same surfaces in a real
//!   frame of today's editor are exactly those colours too, so the two
//!   agree by way of the token.
//! - **Approved pictures** of today's layout in the new look, so that
//!   what later milestones change shows up as a difference.
//!
//! The tests that draw skip, with a note, where there is no graphics
//! driver (`picture_support.rs` says how to give them one).

use crate::picture_support::{
    DESIGN_SCALE, Picture, design, drawn, editor_window, matches_approved, window,
};
use crate::test_support::{info, with_a_crate};
use crate::theme::{Accent, InterfaceLook, Scale};
use crate::tokens;
use crate::workbench::Workbench;
use egui::Color32;

/// A point on one of the design's screens (in the 1440 × 900 it was drawn
/// at), what is there, and the token it must be.
type Sample = (&'static str, [f32; 2], Color32);

fn assert_samples(file: &str, samples: &[Sample]) {
    let screen = design(file);
    for (what, point, token) in samples {
        // A surface is sampled where it is flat for some way around; a
        // line or a border is only a pixel or two thick.
        let thin = what.contains("line") || what.contains("border");
        assert!(
            thin || screen.is_flat_at(*point, DESIGN_SCALE, 3),
            "{file}: {what} at {point:?} isn't a flat surface: pick another point"
        );
        assert_eq!(
            screen.at(*point, DESIGN_SCALE),
            *token,
            "{file}: {what} at {point:?}"
        );
    }
}

#[test]
fn the_level_editor_s_picture_is_made_of_the_tokens() {
    let blue = Accent::Blue;
    assert_samples(
        "01-home-level-editor.png",
        &[
            ("the tab bar", [640.0, 17.0], tokens::app()),
            ("the ribbon's tab in front", [92.0, 28.0], tokens::ribbon()),
            ("the command search", [900.0, 17.0], tokens::ribbon()),
            ("the ribbon's body", [700.0, 60.0], tokens::ribbon()),
            ("the line under the ribbon", [1400.0, 99.0], tokens::line()),
            ("the document tab strip", [500.0, 115.0], tokens::app()),
            (
                "the document tab in front",
                [6.0, 126.0],
                tokens::viewport(),
            ),
            ("a document tab behind", [128.0, 126.0], tokens::shelf_bar()),
            ("the toolbox column", [21.0, 460.0], tokens::shelf_bar()),
            ("the active tool", [9.0, 176.0], blue.colour()),
            ("the code group", [1300.0, 250.0], tokens::code()),
            ("the line over the shelf", [700.0, 500.0], tokens::line()),
            ("the shelf bar", [700.0, 517.0], tokens::shelf_bar()),
            ("the shelf tab in front", [68.0, 509.0], tokens::line()),
            ("the scene column", [110.0, 800.0], tokens::panel()),
            ("the line between columns", [220.0, 750.0], tokens::line()),
            ("the properties column", [700.0, 800.0], tokens::panel()),
            ("the assets column", [1400.0, 800.0], tokens::panel()),
            ("a card's border", [231.0, 650.0], tokens::card_border()),
            ("a field's border", [290.0, 620.0], tokens::field_border()),
            ("a field", [355.0, 626.0], tokens::field()),
            ("the selected row", [180.0, 604.0], tokens::select_row(blue)),
            ("a ticked checkbox", [251.0, 51.0], blue.colour()),
            ("Play", [1335.0, 46.0], tokens::PLAY_FILL),
            (
                "the line over the status bar",
                [700.0, 876.0],
                tokens::line(),
            ),
            ("the status bar", [700.0, 889.0], tokens::code()),
        ],
    );
}

#[test]
fn the_other_screens_pictures_are_made_of_the_tokens_too() {
    assert_samples(
        "03-library.png",
        &[
            ("the ribbon", [900.0, 60.0], tokens::ribbon()),
            ("the browser", [700.0, 300.0], tokens::code()),
            ("the shelf bar", [700.0, 560.0], tokens::shelf_bar()),
        ],
    );
    assert_samples(
        "06-debug-profiler-and-console.png",
        &[
            ("the ribbon", [1000.0, 60.0], tokens::ribbon()),
            ("the status bar", [700.0, 889.0], tokens::code()),
        ],
    );
    assert_samples(
        "07-file-recent-projects.png",
        &[
            ("the tab bar", [640.0, 17.0], tokens::app()),
            ("the File menu column", [100.0, 500.0], tokens::ribbon()),
            ("the page", [700.0, 860.0], tokens::shelf_bar()),
            ("the status bar", [700.0, 889.0], tokens::code()),
        ],
    );
    assert_samples(
        "08-file-preferences.png",
        &[
            ("the File menu column", [100.0, 600.0], tokens::ribbon()),
            ("the page", [600.0, 860.0], tokens::shelf_bar()),
        ],
    );
    assert_samples(
        "13-project-settings-input.png",
        &[
            ("the File menu column", [100.0, 700.0], tokens::ribbon()),
            ("the page", [1300.0, 860.0], tokens::shelf_bar()),
        ],
    );
}

#[test]
fn every_screen_of_the_design_is_here_at_twice_its_size() {
    for (file, tall) in [
        ("01-home-level-editor.png", 1800),
        ("02-home-level-design-placing.png", 1800),
        ("03-library.png", 1800),
        ("04-shading-material.png", 1800),
        ("05-animate-sequencer.png", 1800),
        ("06-debug-profiler-and-console.png", 1800),
        ("07-file-recent-projects.png", 1800),
        ("08-file-preferences.png", 1800),
        ("09-library-model-viewer.png", 1856),
        ("10-audio-mixer.png", 1800),
        ("11-vfx-effect-editor.png", 1800),
        ("12-file-project-settings.png", 2018),
        ("13-project-settings-input.png", 1800),
    ] {
        let screen = design(file);
        assert_eq!(screen.size, [2880, tall], "{file}");
        // Every one has the tab bar's grey in its top left corner.
        assert_eq!(screen.pixel(4, 4), tokens::app(), "{file}");
    }
}

/// How the token sheet is laid out: swatches in rows on the window's grey.
const SWATCH: f32 = 24.0;
const SWATCH_GAP: f32 = 4.0;
const SWATCHES_ACROSS: usize = 12;

fn swatch_at(index: usize) -> egui::Rect {
    let step = SWATCH + SWATCH_GAP;
    let (column, row) = (index % SWATCHES_ACROSS, index / SWATCHES_ACROSS);
    egui::Rect::from_min_size(
        egui::pos2(
            SWATCH_GAP + column as f32 * step,
            SWATCH_GAP + row as f32 * step,
        ),
        egui::vec2(SWATCH, SWATCH),
    )
}

/// Every colour of the design as a swatch, drawn for real: each comes out
/// of the graphics driver exactly as its token says, which is what lets a
/// sampled pixel stand for a token everywhere else.
#[test]
fn the_token_sheet_draws_every_colour_exactly() {
    let sheet = tokens::sheet();
    let rows = sheet.len().div_ceil(SWATCHES_ACROSS);
    let step = SWATCH + SWATCH_GAP;
    let size = [
        SWATCH_GAP + SWATCHES_ACROSS as f32 * step,
        SWATCH_GAP + rows as f32 * step,
    ];
    let colours: Vec<Color32> = sheet.iter().map(|(_, colour)| *colour).collect();
    let Some(mut harness) = window(size, 1.0, |ui| {
        let painter = ui.painter();
        painter.rect_filled(ui.max_rect(), 0.0, tokens::app());
        for (index, colour) in colours.iter().enumerate() {
            painter.rect_filled(swatch_at(index), 0.0, *colour);
        }
    }) else {
        return;
    };
    harness.run();
    let picture = drawn(&mut harness);
    for (index, (name, colour)) in sheet.iter().enumerate() {
        let middle = swatch_at(index).center();
        assert_eq!(
            picture.at([middle.x, middle.y], 1.0),
            *colour,
            "{name} was drawn as another colour"
        );
    }
    assert_eq!(picture.at([1.0, 1.0], 1.0), tokens::app());
    matches_approved("token-sheet", &picture);
}

/// Hairlines drawn at awkward places and at each scale: every one comes
/// out as whole rows and columns of pixels in exactly its colour, with
/// exactly the surface's colour on either side. A line of one point
/// would be a pixel and a half at 150%, and blur.
#[test]
fn hairlines_are_whole_sharp_pixels_at_100_125_and_150_percent() {
    let across = [9.0_f32, 20.4, 33.7, 47.5, 61.25];
    let down = [14.0_f32, 30.6, 52.5, 77.3];
    for scale in [1.0_f32, 1.25, 1.5] {
        let Some(mut harness) = window([100.0, 72.0], scale, |ui| {
            let painter = ui.painter();
            painter.rect_filled(ui.max_rect(), 0.0, tokens::panel());
            for y in across {
                tokens::hline(painter, egui::Rangef::new(4.0, 96.0), y, tokens::line());
            }
            for x in down {
                tokens::vline(painter, x, egui::Rangef::new(4.0, 68.0), tokens::line());
            }
        }) else {
            return;
        };
        harness.run();
        let picture = drawn(&mut harness);
        let thick = (tokens::hairline(scale) * scale).round() as u32;
        assert_eq!(thick, 1, "one pixel at {scale}");
        // Clear of the lines down, a column of pixels crosses each line
        // across exactly once.
        let x = (88.0 * scale) as u32;
        for y in across {
            let row = (tokens::on_pixel(y, scale) * scale).round() as u32;
            assert_eq!(picture.pixel(x, row), tokens::line(), "{y} at {scale}");
            assert_eq!(
                picture.pixel(x, row - 1),
                tokens::panel(),
                "above {y} at {scale}"
            );
            assert_eq!(
                picture.pixel(x, row + thick),
                tokens::panel(),
                "below {y} at {scale}"
            );
        }
        // And a row of pixels between two lines across crosses each line
        // down exactly once.
        let y = (40.0 * scale) as u32;
        for x in down {
            let column = (tokens::on_pixel(x, scale) * scale).round() as u32;
            assert_eq!(picture.pixel(column, y), tokens::line(), "{x} at {scale}");
            assert_eq!(picture.pixel(column - 1, y), tokens::panel(), "left of {x}");
            assert_eq!(
                picture.pixel(column + thick, y),
                tokens::panel(),
                "right of {x}"
            );
        }
    }
}

const EDITOR_SIZE: [f32; 2] = [1440.0, 900.0];
const LAUNCHER_SIZE: [f32; 2] = [1100.0, 700.0];

/// Today's editor, drawn for real in the new look.
fn editor_picture(look: InterfaceLook) -> Option<Picture> {
    let (workbench, _) = with_a_crate();
    let mut harness = editor_window(workbench, look, EDITOR_SIZE)?;
    harness.run();
    harness.run();
    Some(drawn(&mut harness))
}

/// The launcher, drawn for real in the new look.
fn launcher_picture(look: InterfaceLook) -> Option<Picture> {
    let mut workbench = Workbench::new(info(), None);
    workbench.set_this_build(Some(57));
    workbench.launch_launcher();
    let mut harness = editor_window(workbench, look, LAUNCHER_SIZE)?;
    harness.run();
    harness.run();
    Some(drawn(&mut harness))
}

#[test]
fn the_editor_s_surfaces_are_the_design_s() {
    let Some(picture) = editor_picture(InterfaceLook::default()) else {
        return;
    };
    assert_eq!(picture.size, [1440, 900]);
    // Each surface as the design's first screen has it (the test above
    // holds that screen to the same tokens). The layout is still today's:
    // the points are where today's panels are.
    for (what, point, token) in [
        ("the top bar", [860.0, 17.0], tokens::app()),
        ("the line under the top bar", [860.0, 33.0], tokens::line()),
        ("a tab strip", [200.0, 49.0], tokens::app()),
        ("another tab strip", [800.0, 49.0], tokens::app()),
        ("a tab behind", [440.0, 42.0], tokens::shelf_bar()),
        ("the Area panel", [130.0, 400.0], tokens::panel()),
        ("Properties", [1270.0, 400.0], tokens::panel()),
        ("the Library", [500.0, 800.0], tokens::panel()),
        ("Hints", [1270.0, 830.0], tokens::panel()),
        (
            "the Library's tool row",
            [700.0, 687.0],
            tokens::shelf_bar(),
        ),
        ("the search box", [1150.0, 17.0], tokens::field()),
        ("Play", [1306.0, 9.0], tokens::PLAY_FILL),
        (
            "the line over the status bar",
            [700.0, 877.0],
            tokens::line(),
        ),
        ("the status bar", [700.0, 889.0], tokens::code()),
    ] {
        assert!(
            picture.is_flat_at(point, 1.0, 1) || what.starts_with("the line"),
            "{what} at {point:?} isn't flat"
        );
        assert_eq!(picture.at(point, 1.0), token, "{what} at {point:?}");
    }
    matches_approved("editor", &picture);
}

/// D18's menus: on the ribbon's grey. (Their border and corners are in
/// `theme_tests.rs`, where no pixel has to be found.)
#[test]
fn an_open_menu_is_on_the_ribbon_s_grey() {
    use egui_kittest::kittest::Queryable as _;
    let (workbench, _) = with_a_crate();
    let Some(mut harness) = editor_window(workbench, InterfaceLook::default(), EDITOR_SIZE) else {
        return;
    };
    harness.run();
    harness.get_all_by_label("Edit").next().unwrap().click();
    harness.run();
    let item = harness.get_by_label("Settings…").rect();
    let picture = drawn(&mut harness);
    // Just inside the left end of a row nobody points at.
    let point = [item.left() + 1.0, item.center().y];
    assert_eq!(picture.at(point, 1.0), tokens::ribbon(), "{point:?}");
}

#[test]
fn the_launcher_s_surfaces_are_the_file_page_s() {
    let Some(picture) = launcher_picture(InterfaceLook::default()) else {
        return;
    };
    assert_eq!(picture.size, [1100, 700]);
    // As the design's Recent projects screen: the menu column on the
    // ribbon's grey, the page on the shelf bar's.
    for (what, point, token) in [
        ("the sidebar", [110.0, 400.0], tokens::ribbon()),
        ("the page", [630.0, 500.0], tokens::shelf_bar()),
        (
            "the page in front's row",
            [130.0, 90.0],
            tokens::group_gap(),
        ),
        ("+ New project", [980.0, 35.0], Accent::Blue.colour()),
    ] {
        assert_eq!(picture.at(point, 1.0), token, "{what} at {point:?}");
    }
    matches_approved("launcher", &picture);
}

#[test]
fn another_accent_recolours_what_is_chosen_and_nothing_else() {
    let look = |accent| InterfaceLook {
        accent,
        ..InterfaceLook::default()
    };
    let (Some(blue), Some(teal)) = (
        launcher_picture(look(Accent::Blue)),
        launcher_picture(look(Accent::Teal)),
    ) else {
        return;
    };
    assert_eq!(teal.at([980.0, 35.0], 1.0), Accent::Teal.colour());
    // The neutral surfaces are the same pixels in both.
    for point in [
        [110.0, 400.0],
        [130.0, 90.0],
        [630.0, 500.0],
        [300.0, 650.0],
    ] {
        assert_eq!(blue.at(point, 1.0), teal.at(point, 1.0), "{point:?}");
    }
    crate::interface_look::use_look(InterfaceLook::default());
}

#[test]
fn the_interface_scale_draws_the_same_editor_with_more_pixels() {
    let look = InterfaceLook {
        scale: Scale::from(150),
        ..InterfaceLook::default()
    };
    let Some(picture) = launcher_picture(look) else {
        return;
    };
    // The window is as many pixels as before; each point is a pixel and a
    // half, so the same surfaces are where they were times 1.5 (as far
    // as the smaller room allows: the sidebar keeps its width in points).
    assert_eq!(picture.size, [1100, 700]);
    assert_eq!(picture.at([110.0, 400.0], 1.0), tokens::ribbon());
    assert_eq!(picture.at([130.0, 90.0], 1.5), tokens::group_gap());
    assert_eq!(picture.at([900.0, 600.0], 1.0), tokens::shelf_bar());
    crate::interface_look::use_look(InterfaceLook::default());
}

/// Writes the sheet a pull request carries: the design's level editor on
/// the left, today's editor on the right. Run with `--ignored` and
/// `SOLIS_PICTURE_SHEET` set to the folder to write into.
#[test]
#[ignore = "writes the side-by-side sheet; needs SOLIS_PICTURE_SHEET and a graphics driver"]
fn the_side_by_side_sheet() {
    let folder = std::env::var_os("SOLIS_PICTURE_SHEET").expect("SOLIS_PICTURE_SHEET");
    let folder = std::path::Path::new(&folder);
    let Some(editor) = editor_picture(InterfaceLook::default()) else {
        return;
    };
    let design = design("01-home-level-editor.png").halved();
    design
        .beside(&editor, 24, tokens::text_meta())
        .write(&folder.join("level-editor-design-and-editor.png"));
    let Some(launcher) = launcher_picture(InterfaceLook::default()) else {
        return;
    };
    let design = design_file_page().halved();
    design
        .beside(&launcher, 24, tokens::text_meta())
        .write(&folder.join("recent-projects-design-and-launcher.png"));
}

fn design_file_page() -> Picture {
    design("07-file-recent-projects.png")
}
