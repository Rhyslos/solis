//! The kit, measured, pressed, driven from the keyboard and held against
//! the design's own pictures.
//!
//! - Each control is as large as the tokens say, to the point.
//! - Each has a name the tests (and a screen reader) can find it by.
//! - Each can be used without the mouse.
//! - On the widget sheet, drawn for real, each is in the colours the
//!   design's screens have for it; the sheet as a whole is an approved
//!   picture.

use super::sheet::{MIDDLE, PAD, SIZE, Sheet};
use crate::picture_support::{DESIGN_SCALE, Picture, can_draw, design, drawn, matches_approved};
use crate::theme::{self, Accent, Density, InterfaceLook};
use crate::tokens::{self, size};
use egui::{Color32, Key, Rect, accesskit::Role, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

/// The widget sheet in a test window, in `look`. `draws` asks for real
/// frames (a graphics driver); without, only the layout is worked out.
fn sheet_in(look: InterfaceLook, draws: bool) -> Harness<'static, Sheet> {
    let mut themed = false;
    let mut builder = Harness::builder().with_size(vec2(SIZE[0], SIZE[1]));
    if draws {
        builder = builder.renderer(egui_kittest::wgpu::WgpuTestRenderer::new());
    }
    let mut harness = builder.build_ui_state(
        move |ui, sheet: &mut Sheet| {
            if !themed {
                // The fonts are in force from the next frame: nothing
                // is drawn in this one.
                theme::apply(ui.ctx(), look);
                themed = true;
                return;
            }
            // Edge to edge: the test window insets what it draws.
            let whole = ui.ctx().content_rect();
            let mut root = ui.new_child(egui::UiBuilder::new().max_rect(whole));
            root.set_clip_rect(whole);
            sheet.show(&mut root);
        },
        Sheet::default(),
    );
    harness.run();
    harness
}

fn sheet() -> Harness<'static, Sheet> {
    sheet_in(InterfaceLook::default(), false)
}

/// Puts the standard look back for the tests that share this thread.
fn standard_again() {
    crate::interface_look::use_look(InterfaceLook::default());
}

fn rect_of(h: &Harness<'_, Sheet>, label: &str) -> Rect {
    h.get_by_label(label).rect()
}

#[test]
fn each_control_is_as_large_as_the_tokens_say() {
    let h = sheet();
    let size_of = |label: &str| {
        let rect = rect_of(&h, label);
        [rect.width(), rect.height()]
    };
    let toggle = h
        .get_by_role_and_label(Role::CheckBox, "Show ribbon labels")
        .rect();
    assert_eq!([toggle.width(), toggle.height()], size::TOGGLE);
    // Segments are 20 tall in a well with 2 of padding.
    for segment in ["Compact", "Default", "Comfortable"] {
        assert_eq!(size_of(segment)[1], size::SEGMENT_H, "{segment}");
    }
    let (first, last) = (rect_of(&h, "Compact"), rect_of(&h, "Comfortable"));
    assert_eq!(first.right(), rect_of(&h, "Default").left());
    assert!(last.right() > first.left() + 150.0);
    // Swatches: 22 square, 8 apart.
    let swatches: Vec<Rect> = Accent::ALL.iter().map(|a| rect_of(&h, a.name())).collect();
    for pair in swatches.windows(2) {
        assert_eq!(pair[0].size(), vec2(size::SWATCH, size::SWATCH));
        assert_eq!(pair[1].left() - pair[0].right(), size::SWATCH_GAP);
    }
    // The drop-down is a field: 24 tall, 200 wide on a settings page.
    let drop = h
        .get_by_role_and_label(Role::ComboBox, "Shelf opens on")
        .rect();
    assert_eq!([drop.width(), drop.height()], [size::SETTING_FIELD_W, 24.0]);
    // Buttons: 32 in a page's header, 30 elsewhere.
    assert_eq!(size_of("Open…")[1], 32.0);
    assert_eq!(size_of("New project")[1], 32.0);
    assert_eq!(size_of("Play")[1], size::PAGE_BUTTON_H);
    assert_eq!(size_of("Reset to defaults"), [260.0, size::PAGE_BUTTON_H]);
    assert_eq!(size_of("+ Component")[1], size::DASHED_BUTTON_H);
    assert_eq!(size_of("More"), [size::ICON_BUTTON, size::ICON_BUTTON]);
    // Keycaps: 22 tall and never narrower than that.
    assert_eq!(size_of("W"), [size::KEYCAP, size::KEYCAP]);
    assert_eq!(size_of("Left Shift")[1], size::KEYCAP);
    assert!(size_of("Left Shift")[0] > 60.0);
    assert_eq!(size_of("Perspective ▾")[1], size::PILL_H);
    // Chips: the three kinds the design draws.
    assert_eq!(size_of("Model")[1], size::CHIP_H);
    assert_eq!(size_of("Recent")[1], 22.0);
    assert_eq!(size_of("Gameplay")[1], 26.0);
    // A checkbox's row is a small ribbon row's height.
    assert_eq!(size_of("Casts shadows")[1], size::RIBBON_ROW_H);
    assert_eq!(size_of("Volume")[1], size::SLIDER_KNOB);
    // A tile: as wide as asked, and as tall as its square thumbnail and
    // two lines of words make it.
    let tile = size_of("Crane_Arm_02");
    assert_eq!(tile[0], size::TILE_MIN_W);
    assert_eq!(tile[1], super::tile::height(size::TILE_MIN_W));
}

#[test]
fn a_setting_row_has_its_words_left_and_its_control_at_the_right_edge() {
    let h = sheet();
    let right_edge = MIDDLE - PAD;
    for (role, label) in [
        (Role::CheckBox, "Show ribbon labels"),
        (Role::ComboBox, "Shelf opens on"),
        (Role::RadioButton, "Comfortable"),
        (Role::RadioButton, "Purple"),
    ] {
        let control = h.get_by_role_and_label(role, label).rect();
        let at_edge = if label == "Comfortable" {
            // The well's padding is between the last segment and the edge.
            right_edge - size::SEGMENT_WELL_PAD
        } else {
            right_edge
        };
        assert_eq!(control.right(), at_edge, "{label}");
    }
    // The name is at the left edge; the toggle is level with the row's
    // middle, and the rows follow one another at the design's rhythm:
    // 12 above, the words, 12 below and a hairline.
    let name = h
        .get_by_role_and_label(Role::Label, "Show ribbon labels")
        .rect();
    assert_eq!(name.left(), PAD);
    let first = h
        .get_by_role_and_label(Role::CheckBox, "Show ribbon labels")
        .rect();
    let second = h
        .get_by_role_and_label(Role::CheckBox, "Code beside the viewport")
        .rect();
    let rhythm = second.center().y - first.center().y;
    assert!((58.0..=61.0).contains(&rhythm), "rows are {rhythm} apart");
}

#[test]
fn every_control_has_a_name_the_tests_can_find_it_by() {
    let h = sheet();
    for (role, label) in [
        (Role::CheckBox, "Show ribbon labels"),
        (Role::CheckBox, "Code beside the viewport"),
        (Role::RadioButton, "Compact"),
        (Role::RadioButton, "Teal"),
        (Role::ComboBox, "Shelf opens on"),
        (Role::Button, "Open…"),
        (Role::Button, "New project"),
        (Role::Button, "Play"),
        (Role::Link, "All templates"),
        (Role::Button, "Reset to defaults"),
        (Role::Button, "W"),
        (Role::Button, "Press a key…"),
        (Role::Button, "Perspective ▾"),
        (Role::Button, "More"),
        (Role::Button, "+ Component"),
        (Role::Button, "Model"),
        (Role::Button, "Gameplay"),
        (Role::CheckBox, "Casts shadows"),
        (Role::Slider, "Volume"),
        (Role::Button, "Crane_Arm_02"),
    ] {
        assert!(
            h.query_by_role_and_label(role, label).is_some(),
            "no {role:?} named {label}"
        );
    }
    // Words that aren't controls are there to read.
    for words in ["LAYOUT", "Properties", "Transform", "Triangles", "12,480"] {
        assert!(h.query_by_label(words).is_some(), "{words}");
    }
}

#[test]
fn a_click_does_what_each_control_is_for() {
    let mut h = sheet();
    h.get_by_role_and_label(Role::CheckBox, "Show ribbon labels")
        .click();
    h.get_by_role_and_label(Role::CheckBox, "Code beside the viewport")
        .click();
    h.get_by_label("Comfortable").click();
    h.get_by_label("Teal").click();
    h.get_by_label("Show the grid").click();
    h.get_by_label("Picture").click();
    h.run();
    let s = h.state();
    assert!(!s.labels, "the first switch");
    assert!(s.split, "the second switch");
    assert!(s.grid, "the checkbox");
    assert_eq!(s.density, Density::Comfortable);
    assert_eq!(s.accent, Accent::Teal);
    assert_eq!(s.kind, 2);
    for button in [
        "Open…",
        "New project",
        "Play",
        "All templates",
        "Reset to defaults",
        "W",
        "Perspective ▾",
        "More",
        "+ Component",
        "Crane_Arm_02",
    ] {
        h.get_by_label(button).click();
        h.run();
        assert_eq!(h.state().pressed, Some(button));
    }
}

#[test]
fn the_drop_down_opens_a_list_and_a_row_of_it_chooses() {
    let mut h = sheet();
    h.get_by_role_and_label(Role::ComboBox, "Shelf opens on")
        .click();
    h.run();
    // The list holds every choice, the one in force marked.
    for choice in ["Console", "Problems"] {
        assert!(
            h.query_by_role_and_label(Role::RadioButton, choice)
                .is_some()
        );
    }
    h.get_by_role_and_label(Role::RadioButton, "Problems")
        .click();
    h.run();
    assert_eq!(h.state().opens_on, 2);
    // The list closed behind the choice.
    assert!(
        h.query_by_role_and_label(Role::RadioButton, "Console")
            .is_none()
    );
}

#[test]
fn space_and_enter_do_what_a_click_does() {
    let mut h = sheet();
    h.get_by_role_and_label(Role::CheckBox, "Show ribbon labels")
        .focus();
    h.run();
    h.key_press(Key::Space);
    h.run();
    assert!(!h.state().labels);
    h.key_press(Key::Enter);
    h.run();
    assert!(h.state().labels);
    h.get_by_label("Casts shadows").focus();
    h.run();
    h.key_press(Key::Space);
    h.run();
    assert!(!h.state().shadows);
    for button in ["Reset to defaults", "+ Component", "More", "All templates"] {
        h.get_by_label(button).focus();
        h.run();
        h.key_press(Key::Enter);
        h.run();
        assert_eq!(h.state().pressed, Some(button));
    }
    h.get_by_label("Compact").focus();
    h.run();
    h.key_press(Key::Space);
    h.run();
    assert_eq!(h.state().density, Density::Compact);
}

#[test]
fn tab_walks_from_control_to_control() {
    let mut h = sheet();
    // From nothing, Tab reaches the first control, then the next.
    h.key_press(Key::Tab);
    h.run();
    let first = h.get_by_role_and_label(Role::CheckBox, "Show ribbon labels");
    assert!(first.is_focused());
    h.key_press(Key::Tab);
    h.run();
    let second = h.get_by_role_and_label(Role::CheckBox, "Code beside the viewport");
    assert!(second.is_focused());
}

#[test]
fn the_arrow_keys_move_along_a_row_of_choices() {
    let mut h = sheet();
    h.get_by_label("Default").focus();
    h.run();
    h.key_press(Key::ArrowRight);
    h.run();
    assert_eq!(h.state().density, Density::Comfortable);
    // The keyboard went with the choice, so the next arrow carries on
    // from there; at the end it stays.
    assert!(h.get_by_label("Comfortable").is_focused());
    h.key_press(Key::ArrowRight);
    h.run();
    assert_eq!(h.state().density, Density::Comfortable);
    h.key_press(Key::ArrowLeft);
    h.run();
    h.key_press(Key::ArrowLeft);
    h.run();
    assert_eq!(h.state().density, Density::Compact);
    // Swatches likewise.
    h.get_by_label("Blue").focus();
    h.run();
    h.key_press(Key::ArrowRight);
    h.run();
    assert_eq!(h.state().accent, Accent::Grey);
    h.key_press(Key::ArrowLeft);
    h.run();
    h.key_press(Key::ArrowLeft);
    h.run();
    assert_eq!(h.state().accent, Accent::Blue);
}

#[test]
fn the_arrow_keys_step_a_closed_drop_down_and_slide_a_slider() {
    let mut h = sheet();
    h.get_by_role_and_label(Role::ComboBox, "Shelf opens on")
        .focus();
    h.run();
    h.key_press(Key::ArrowDown);
    h.run();
    assert_eq!(h.state().opens_on, 1);
    h.key_press(Key::ArrowUp);
    h.run();
    h.key_press(Key::ArrowUp);
    h.run();
    assert_eq!(h.state().opens_on, 0);
    h.get_by_label("Volume").focus();
    h.run();
    let before = h.state().volume;
    h.key_press(Key::ArrowRight);
    h.run();
    assert!(h.state().volume > before);
    for _ in 0..80 {
        h.key_press(Key::ArrowLeft);
        h.run();
    }
    assert_eq!(h.state().volume, 0.0);
}

#[test]
fn a_slider_goes_where_it_is_pressed_and_stays_within_its_range() {
    let mut h = sheet();
    let rect = rect_of(&h, "Volume");
    h.drag_at(egui::pos2(rect.right() - 6.0, rect.center().y));
    h.run();
    h.drop_at(egui::pos2(rect.right() + 300.0, rect.center().y));
    h.run();
    assert_eq!(h.state().volume, 1.0);
    // A value that isn't a number becomes the range's start.
    h.state_mut().volume = f32::NAN;
    h.run();
    assert_eq!(h.state().volume, 0.0);
}

#[test]
fn fields_follow_the_density() {
    for (density, tall) in [
        (Density::Compact, 22.0),
        (Density::Default, 24.0),
        (Density::Comfortable, 28.0),
    ] {
        let look = InterfaceLook {
            density,
            ..InterfaceLook::default()
        };
        let h = sheet_in(look, false);
        let drop = h
            .get_by_role_and_label(Role::ComboBox, "Shelf opens on")
            .rect();
        assert_eq!(drop.height(), tall, "{density:?}");
        // The design's fixed sizes don't change.
        let toggle = h
            .get_by_role_and_label(Role::CheckBox, "Show ribbon labels")
            .rect();
        assert_eq!(toggle.height(), size::TOGGLE[1]);
    }
    standard_again();
}

#[test]
fn a_dashed_outline_stays_inside_its_rectangle_and_closes() {
    let rect = Rect::from_min_size(egui::pos2(10.0, 20.0), vec2(80.0, 22.0));
    for radius in [0.0, 5.0, 11.0, 40.0] {
        let points = super::dashed::outline(rect, radius, 1.0);
        assert_eq!(points.first(), points.last(), "{radius}");
        for p in &points {
            assert!(rect.expand(0.001).contains(*p), "{radius}: {p:?}");
        }
        // It reaches every side, half a line in.
        let near = |a: f32, b: f32| (a - b).abs() < 0.01;
        assert!(points.iter().any(|p| near(p.x, rect.left() + 0.5)));
        assert!(points.iter().any(|p| near(p.x, rect.right() - 0.5)));
        assert!(points.iter().any(|p| near(p.y, rect.top() + 0.5)));
        assert!(points.iter().any(|p| near(p.y, rect.bottom() - 0.5)));
    }
}

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
