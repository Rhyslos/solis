//! The kit drawn for real, on the widget sheet: each control is in the
//! colours the design's screens have for it (the switch from
//! Preferences, the keycap from Input, a card from the level editor),
//! the ring that shows where the keyboard is is one pixel of the accent,
//! borders stay one sharp pixel at 150%, and the sheet as a whole is an
//! approved picture. These need a graphics driver, as the editor's other
//! picture tests do.

use super::sheet::{SIZE, Sheet};
use super::tests::{rect_of, sheet_in, standard_again};
use crate::picture_support::{DESIGN_SCALE, Picture, can_draw, design, drawn, matches_approved};
use crate::theme::{self, Accent, InterfaceLook};
use crate::tokens::{self, size};
use egui::{Color32, accesskit::Role};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

/// The sheet drawn for real; `None` (after a note) where nothing can.
fn sheet_picture(look: InterfaceLook) -> Option<(Harness<'static, Sheet>, Picture)> {
    if !can_draw() {
        return None;
    }
    let mut harness = sheet_in(look, true);
    harness.run();
    let picture = drawn(&mut harness);
    Some((harness, picture))
}

/// The colour at `point` of a drawn sheet, which must be a flat surface
/// there: clear of letters, corners and borders, so every driver draws
/// the same pixel.
fn flat_at(picture: &Picture, what: &str, point: egui::Pos2) -> Color32 {
    assert!(
        picture.is_flat_at([point.x, point.y], 1.0, 1),
        "{what} at {point:?} isn't a flat surface: pick another point"
    );
    picture.at([point.x, point.y], 1.0)
}

/// The colour at a point of one of the design's screens, which must be
/// a flat surface there too.
fn design_at(screen: &Picture, what: &str, point: [f32; 2]) -> Color32 {
    assert!(
        screen.is_flat_at(point, DESIGN_SCALE, 3),
        "the design's {what} at {point:?} isn't a flat surface"
    );
    screen.at(point, DESIGN_SCALE)
}

#[test]
fn the_widget_sheet_matches_its_approved_picture() {
    let Some((_, picture)) = sheet_picture(InterfaceLook::default()) else {
        return;
    };
    assert_eq!(picture.size, [SIZE[0] as u32, SIZE[1] as u32]);
    matches_approved("widget-sheet", &picture);
}

/// The design's own pictures say what each control's colours are; the
/// sheet, drawn for real, has the same pixels in the same places of the
/// same controls.
#[test]
fn the_sheet_s_controls_are_the_colours_of_the_design_s_pictures() {
    let Some((h, picture)) = sheet_picture(InterfaceLook::default()) else {
        return;
    };
    let blue = Accent::Blue.colour();
    // The switch, from Preferences (screen 8): "Show ribbon labels", on.
    let preferences = design("08-file-preferences.png");
    let track = design_at(&preferences, "switch's track", [990.0, 498.0]);
    let knob = design_at(&preferences, "switch's knob", [1007.5, 497.5]);
    assert_eq!((track, knob), (blue, tokens::app()));
    let toggle = h
        .get_by_role_and_label(Role::CheckBox, "Show ribbon labels")
        .rect();
    let middle = toggle.center().y;
    assert_eq!(
        flat_at(
            &picture,
            "the switch's track",
            egui::pos2(toggle.left() + 5.5, middle)
        ),
        track
    );
    assert_eq!(
        flat_at(
            &picture,
            "the switch's knob",
            egui::pos2(toggle.right() - 9.0, middle)
        ),
        knob
    );
    // Off, as the design's placing screen has one: a grey track, the
    // knob at the left in the lowest readable grey.
    let off = h
        .get_by_role_and_label(Role::CheckBox, "Code beside the viewport")
        .rect();
    assert_eq!(
        flat_at(
            &picture,
            "a switch that is off",
            egui::pos2(off.right() - 5.5, middle + off.top() - toggle.top())
        ),
        tokens::strong_line()
    );
    assert_eq!(
        flat_at(
            &picture,
            "its knob",
            egui::pos2(off.left() + 9.0, off.center().y)
        ),
        tokens::text_meta()
    );
    // The segmented control and a drop-down field, from the same screen.
    let chosen = design_at(&preferences, "chosen segment", [879.0, 396.5]);
    let well = design_at(&preferences, "segments' well", [812.0, 394.5]);
    let field = design_at(&preferences, "field", [823.0, 216.5]);
    assert_eq!(
        (chosen, well, field),
        (tokens::field_border(), tokens::app(), tokens::field())
    );
    let segment = |label: &str| {
        let rect = rect_of(&h, label);
        egui::pos2(rect.left() + 3.5, rect.center().y)
    };
    assert_eq!(
        flat_at(&picture, "the chosen segment", segment("Default")),
        chosen
    );
    assert_eq!(
        flat_at(&picture, "a segment at rest", segment("Compact")),
        well
    );
    let drop = h
        .get_by_role_and_label(Role::ComboBox, "Shelf opens on")
        .rect();
    assert_eq!(
        flat_at(
            &picture,
            "the field",
            egui::pos2(drop.left() + 3.5, drop.center().y)
        ),
        field
    );
    // The chosen swatch: from the swatch outwards, 2 px of the page and
    // 2 px of its own colour, as on screen 8.
    let screen = |x: f32| preferences.at([x, 286.0], DESIGN_SCALE);
    assert_eq!(
        [screen(875.5), screen(873.5), screen(871.5), screen(869.5)],
        [blue, tokens::shelf_bar(), blue, tokens::shelf_bar()]
    );
    let swatch = rect_of(&h, "Blue");
    let sheet = |from_left: f32| picture.at([swatch.left() + from_left, swatch.center().y], 1.0);
    assert_eq!(
        [sheet(0.5), sheet(-1.5), sheet(-3.5), sheet(-5.5)],
        [blue, tokens::shelf_bar(), blue, tokens::shelf_bar()]
    );
    // A keycap, from the input page (screen 13).
    let input = design("13-project-settings-input.png");
    let cap = design_at(&input, "keycap", [590.0, 350.0]);
    assert_eq!(cap, tokens::line());
    assert_eq!(
        input.at([589.5, 380.5], DESIGN_SCALE),
        tokens::listening(Accent::Blue)
    );
    let key = rect_of(&h, "Left Shift");
    assert_eq!(
        flat_at(
            &picture,
            "the keycap",
            egui::pos2(key.left() + 4.0, key.center().y)
        ),
        cap
    );
    // Its border: one pixel at the sides, two along its foot.
    let edge = |x: f32, y: f32| picture.at([x, y], 1.0);
    assert_eq!(
        edge(key.left() + 0.5, key.center().y),
        tokens::stronger_line()
    );
    assert_eq!(
        edge(key.center().x, key.bottom() - 0.5),
        tokens::stronger_line()
    );
    assert_eq!(
        edge(key.center().x, key.bottom() - 1.5),
        tokens::stronger_line()
    );
    assert_eq!(edge(key.center().x, key.bottom() - 2.5), cap);
    assert_eq!(edge(key.center().x, key.top() + 1.5), cap);
    let waiting = rect_of(&h, "Press a key…");
    assert_eq!(
        flat_at(
            &picture,
            "a keycap waiting",
            egui::pos2(waiting.left() + 4.0, waiting.center().y)
        ),
        tokens::listening(Accent::Blue)
    );
    // A card, from the level editor (screen 1).
    let level = design("01-home-level-editor.png");
    let card = design_at(&level, "card", [236.0, 660.0]);
    assert_eq!(card, tokens::card());
    let title = rect_of(&h, "Transform");
    assert_eq!(
        flat_at(
            &picture,
            "the card",
            egui::pos2(title.left() - 5.0, title.center().y)
        ),
        card
    );
    // Its border is outside its padding of 10.
    assert_eq!(
        edge(title.left() - size::CARD_PAD - 0.5, title.center().y),
        tokens::card_border()
    );
    assert_eq!(
        edge(title.left() - size::CARD_PAD - 1.5, title.center().y),
        tokens::panel()
    );
}

#[test]
fn the_ring_that_shows_where_the_keyboard_is_is_one_pixel_of_the_accent_outside() {
    if !can_draw() {
        return;
    }
    let look = InterfaceLook {
        accent: Accent::Orange,
        ..InterfaceLook::default()
    };
    let mut h = sheet_in(look, true);
    let name = "Code beside the viewport";
    let rect = h.get_by_role_and_label(Role::CheckBox, name).rect();
    // Above the middle of its top edge, where the ring runs straight.
    let ring_at = [rect.center().x, rect.top() - 0.5];
    let before = drawn(&mut h);
    assert_eq!(before.at(ring_at, 1.0), tokens::shelf_bar());
    h.get_by_role_and_label(Role::CheckBox, name).focus();
    h.run();
    let after = drawn(&mut h);
    assert_eq!(after.at(ring_at, 1.0), Accent::Orange.colour());
    // One pixel, and outside: the pixel beyond it is still the page, and
    // the control itself is as it was.
    assert_eq!(
        after.at([rect.center().x, rect.top() - 1.5], 1.0),
        tokens::shelf_bar()
    );
    let inside = [rect.center().x, rect.top() + 0.5];
    assert_eq!(after.at(inside, 1.0), before.at(inside, 1.0));
    assert_eq!(after.at(inside, 1.0), tokens::strong_line());
    standard_again();
}

#[test]
fn borders_stay_one_sharp_pixel_at_150_percent() {
    if !can_draw() {
        return;
    }
    let look = InterfaceLook {
        scale: theme::Scale::from(150),
        ..InterfaceLook::default()
    };
    let mut h = sheet_in(look, true);
    // In points nothing changed size.
    let drop = h
        .get_by_role_and_label(Role::ComboBox, "Shelf opens on")
        .rect();
    assert_eq!(drop.height(), 24.0);
    let picture = drawn(&mut h);
    // Along a row of pixels through the field's left edge: page, exactly
    // one pixel of border, then the field.
    let y = (drop.center().y * 1.5) as u32;
    let edge = (drop.left() * 1.5).round() as u32;
    let row: Vec<Color32> = (edge - 2..edge + 3).map(|x| picture.pixel(x, y)).collect();
    let border = row.iter().filter(|c| **c == tokens::field_border()).count();
    assert_eq!(border, 1, "{row:?}");
    assert_eq!(row[0], tokens::shelf_bar());
    assert_eq!(row[4], tokens::field());
    standard_again();
}

/// An open drop-down's list is a menu like the editor's others: on the
/// ribbon's grey, as wide as its field, its rows 24 tall.
#[test]
fn an_open_drop_down_lists_its_choices_as_a_menu_does() {
    if !can_draw() {
        return;
    }
    let mut h = sheet_in(InterfaceLook::default(), true);
    let field = h
        .get_by_role_and_label(Role::ComboBox, "Shelf opens on")
        .rect();
    h.get_by_role_and_label(Role::ComboBox, "Shelf opens on")
        .click();
    h.run();
    let rows: Vec<egui::Rect> = ["Entity", "Console", "Problems"]
        .iter()
        .map(|choice| h.get_by_role_and_label(Role::RadioButton, choice).rect())
        .collect();
    for pair in rows.windows(2) {
        assert_eq!(pair[0].height(), size::MENU_ROW_H);
        assert_eq!(pair[1].top() - pair[0].bottom(), 0.0, "rows touch");
    }
    // Under the field, and no wider than it.
    assert!(rows[0].top() >= field.bottom());
    assert!(rows[0].left() >= field.left() && rows[0].right() <= field.right());
    let picture = drawn(&mut h);
    if let Some(folder) = std::env::var_os("SOLIS_PICTURE_SHEET") {
        picture.write(&std::path::Path::new(&folder).join("widget-sheet-drop-down.png"));
    }
    // Inside a row that isn't under the pointer, clear of its word.
    let quiet = egui::pos2(rows[2].right() - 6.0, rows[2].center().y);
    assert_eq!(flat_at(&picture, "the open list", quiet), tokens::ribbon());
    standard_again();
}
