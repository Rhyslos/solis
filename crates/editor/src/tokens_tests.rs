//! The tokens against the design they come from: every colour in the
//! specification's tables is a token, every size it gives is the token's,
//! mixing is the mock-up's arithmetic, and hairlines land on whole pixels.

use super::size::{self, Density, Scale};
use super::*;
use std::path::Path;

/// The specification, with its line ends made plain (a Windows checkout
/// ends lines with a carriage return too).
fn specification() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("docs")
        .join("design")
        .join("g-layout")
        .join("specification.md");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
        .replace("\r\n", "\n")
}

/// The part of `text` from the line `from` up to the line `to`.
fn between<'a>(text: &'a str, from: &str, to: &str) -> &'a str {
    let start = text.find(from).unwrap_or_else(|| panic!("no {from:?}"));
    let end = text[start..]
        .find(to)
        .unwrap_or_else(|| panic!("no {to:?} after {from:?}"));
    &text[start..start + end]
}

/// Every colour written as `#RRGGBB` in `text`.
fn colours_in(text: &str) -> Vec<Color32> {
    let mut found = Vec::new();
    for (at, _) in text.match_indices('#') {
        let digits: String = text[at + 1..]
            .chars()
            .take_while(char::is_ascii_hexdigit)
            .collect();
        if digits.len() == 6 {
            let value = u32::from_str_radix(&digits, 16).unwrap();
            found.push(hex(value));
        }
    }
    found
}

fn name_of(colour: Color32) -> String {
    format!("#{:02X}{:02X}{:02X}", colour.r(), colour.g(), colour.b())
}

#[test]
fn every_colour_in_the_specification_s_tables_is_a_token() {
    let spec = specification();
    let tables = between(&spec, "### Surfaces and lines", "### Type");
    let listed = colours_in(tables);
    assert!(listed.len() > 60, "found only {} colours", listed.len());
    let tokens: Vec<Color32> = sheet().into_iter().map(|(_, colour)| colour).collect();
    let missing: Vec<String> = listed
        .into_iter()
        .filter(|colour| !tokens.contains(colour))
        .map(name_of)
        .collect();
    assert!(missing.is_empty(), "not tokens: {missing:?}");
}

#[test]
fn every_neutral_token_is_a_colour_of_the_design() {
    let spec = specification();
    let written = colours_in(&spec);
    // Read from the mock-up, where the specification has no value.
    let from_the_mock_up = [("text_tile", text_tile())];
    for (name, colour) in neutrals() {
        let known = written.contains(&colour) || from_the_mock_up.contains(&(name, colour));
        assert!(
            known,
            "{name} is {}, which the design never uses",
            name_of(colour)
        );
        assert_eq!(
            colour.r(),
            colour.g(),
            "{name}: the design's greys are pure"
        );
        assert_eq!(colour.g(), colour.b(), "{name}");
        assert_ne!(colour, Color32::BLACK, "{name}: the design is never black");
    }
}

#[test]
fn surfaces_get_lighter_the_further_forward_they_sit() {
    let back_to_front = [
        app(),
        field(),
        shelf_bar(),
        code(),
        ribbon(),
        panel(),
        viewport(),
        card(),
        row_hover(),
        hover(),
        line(),
        card_border(),
        field_border(),
        pill_border(),
        strong_line(),
        stronger_line(),
    ];
    for pair in back_to_front.windows(2) {
        assert!(
            pair[0].r() < pair[1].r(),
            "{:?} then {:?}",
            pair[0],
            pair[1]
        );
    }
    let text = [
        text_separator(),
        text_line_number(),
        text_faint(),
        text_meta(),
        text_label(),
        text_body(),
        text_tile(),
        text_primary(),
        text_emphasis(),
    ];
    for pair in text.windows(2) {
        assert!(pair[0].r() < pair[1].r());
    }
}

#[test]
fn the_sheet_names_every_colour_once() {
    let all = sheet();
    let mut names: Vec<&str> = all.iter().map(|(name, _)| *name).collect();
    names.sort_unstable();
    let count = names.len();
    names.dedup();
    assert_eq!(names.len(), count, "a name is listed twice");
    assert!(count > 80, "{count} colours");
    for accent in Accent::ALL {
        assert!(all.contains(&(accent.name(), accent.colour())));
    }
}

/// The specification's row for `element` in its sizes table.
fn size_row<'a>(sizes: &'a str, element: &str) -> &'a str {
    sizes
        .lines()
        .find(|line| line.starts_with(&format!("| {element} |")))
        .unwrap_or_else(|| panic!("no row for {element:?}"))
}

#[test]
fn the_sizes_are_the_specification_s() {
    let spec = specification();
    let sizes = between(&spec, "### Sizes and spacing", "### Icons and states");
    let px = |v: f32| format!("{v} px");
    let by = |[w, h]: [f32; 2]| format!("{w} × {h} px");
    let rows = [
        ("Tab bar", px(size::TAB_BAR_H) + " tall"),
        ("Tab bar", format!("tabs {}", px(size::RIBBON_TAB_H))),
        (
            "Tab bar",
            format!("{} side padding", px(size::RIBBON_TAB_PAD)),
        ),
        ("Ribbon", px(size::RIBBON_H) + " tall"),
        ("Ribbon", format!("large button {}", by(size::RIBBON_BIG))),
        ("Ribbon", format!("small row {}", px(size::RIBBON_ROW_H))),
        ("Editor tab strip", px(size::DOC_TAB_H)),
        ("Shelf bar", px(size::SHELF_BAR_H)),
        ("Shelf bar", format!("drag grip {}", by(size::SHELF_GRIP))),
        ("Shelf column header", px(size::COLUMN_HEADER_H)),
        ("Fields", px(Density::Default.field_h()) + " tall"),
        ("Fields", format!("radius {}", size::radius::CONTROL)),
        ("Fields", format!("{} side padding", px(size::FIELD_PAD))),
        ("Icon buttons", format!("{0} × {0} px", size::ICON_BUTTON)),
        (
            "Icon buttons",
            format!("toolbox buttons {0} × {0} px", size::TOOL_BUTTON),
        ),
        (
            "Floating pills (viewport overlays)",
            px(size::PILL_H) + " tall",
        ),
        ("Status bar", px(size::STATUS_BAR_H)),
        ("Radii", format!("{} px cards", size::radius::CARD)),
        (
            "Radii",
            format!("{} px project cards", size::radius::PROJECT_CARD),
        ),
        (
            "Radii",
            format!("{} px tab tops and tiles", size::radius::TAB),
        ),
        ("Panel padding", format!("card gap {}", px(size::CARD_GAP))),
    ];
    for (element, says) in rows {
        let row = size_row(sizes, element);
        assert!(
            row.contains(&says),
            "{element}: expected {says:?} in {row:?}"
        );
    }
    // Rows are "22-24 px": compact and the design's own.
    let rows_row = size_row(sizes, "Tree / list rows");
    assert!(
        rows_row.contains("22") && rows_row.contains("24 px"),
        "{rows_row}"
    );
    assert_eq!(Density::Compact.row_h(), 22.0);
    assert_eq!(Density::Default.row_h(), 24.0);
}

#[test]
fn the_pills_and_the_accent_s_shares_are_the_specification_s() {
    let spec = specification();
    assert!(spec.contains("background #121212 at 85%"));
    assert_eq!(PILL_OPACITY, 0.85);
    assert_eq!(pill_fill().a(), 217);
    assert!(spec.contains("accent mixed 16–24% over the surface"));
    for share in [
        share::SELECTED,
        share::SELECTED_ROW,
        share::TRACK,
        share::ACTIVE_BUTTON,
        share::LISTENING,
    ] {
        assert!((0.16..=0.24).contains(&share));
    }
}

#[test]
fn mixing_is_a_straight_blend_of_the_two_colours() {
    let blue = Accent::Blue.colour();
    // By hand: 0.16 * (0x5c, 0x9c, 0xff) + 0.84 * 0x1a, each rounded.
    assert_eq!(
        mix(blue, panel(), 0.16),
        Color32::from_rgb(0x25, 0x2f, 0x3f)
    );
    assert_eq!(
        select_item(Accent::Blue),
        Color32::from_rgb(0x25, 0x2f, 0x3f)
    );
    // The scene list's selected row, as the design's first screen has it.
    assert_eq!(
        select_row(Accent::Blue),
        Color32::from_rgb(0x29, 0x37, 0x4c)
    );
    assert_eq!(
        active_button(Accent::Blue),
        Color32::from_rgb(0x27, 0x35, 0x4b)
    );
    // The ends, and shares outside 0 to 1.
    assert_eq!(mix(blue, panel(), 0.0), panel());
    assert_eq!(mix(blue, panel(), 1.0), blue);
    assert_eq!(mix(blue, panel(), -3.0), panel());
    assert_eq!(mix(blue, panel(), 7.0), blue);
    assert_eq!(
        mix(blue, panel(), f32::NAN),
        panel(),
        "no number, no accent"
    );
    // Always solid, whatever goes in.
    for accent in Accent::ALL {
        for share in [0.08, 0.12, 0.14, 0.16, 0.2, 0.22, 0.45, 0.55, 0.6, 0.8] {
            let mixed = mix(accent.colour(), ribbon(), share);
            assert_eq!(mixed.a(), 255);
            // Between the two, channel by channel.
            for (m, (a, s)) in [mixed.r(), mixed.g(), mixed.b()].into_iter().zip([
                (accent.colour().r(), ribbon().r()),
                (accent.colour().g(), ribbon().g()),
                (accent.colour().b(), ribbon().b()),
            ]) {
                assert!(m >= a.min(s) && m <= a.max(s));
            }
        }
    }
}

#[test]
fn a_hairline_is_whole_pixels_at_every_scale() {
    for scale in Scale::ALL {
        let ppp = scale.factor();
        let pixels = hairline(ppp) * ppp;
        assert!(
            (pixels - pixels.round()).abs() < 1e-4 && pixels >= 1.0,
            "{}: a hairline is {pixels} pixels",
            scale.name()
        );
    }
    // One pixel until 200%, as a browser draws a 1 px border.
    for (ppp, pixels) in [(1.0, 1.0), (1.25, 1.0), (1.5, 1.0), (1.75, 1.0), (2.0, 2.0)] {
        assert!((hairline(ppp) * ppp - pixels).abs() < 1e-4, "{ppp}");
    }
    // On a screen that is itself scaled (a laptop at 150% showing the
    // interface at 125%).
    let ppp = 1.5 * 1.25;
    assert!((hairline(ppp) * ppp - 1.0).abs() < 1e-4);
    // Nonsense in, a plain point out: never zero, never a panic.
    for odd in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        assert_eq!(hairline(odd), 1.0);
        assert_eq!(on_pixel(10.3, odd), 10.3);
    }
}

#[test]
fn hairlines_land_on_whole_pixels_at_100_125_and_150_percent() {
    for ppp in [1.0_f32, 1.25, 1.5] {
        // Every place the frame's parts put a line: 34, 100, 130 and the
        // odd ones a wrapped column lands on.
        for at in [0.0_f32, 34.0, 66.0, 100.0, 130.4, 299.7, 611.5, 876.0] {
            let top = on_pixel(at, ppp) * ppp;
            let bottom = (on_pixel(at, ppp) + hairline(ppp)) * ppp;
            assert!(
                (top - top.round()).abs() < 1e-3,
                "{at} at {ppp}: starts at {top}"
            );
            assert!(
                (bottom - bottom.round()).abs() < 1e-3,
                "{at} at {ppp}: ends at {bottom}"
            );
            // And it didn't move further than half a pixel to get there.
            assert!((on_pixel(at, ppp) - at).abs() <= 0.5 / ppp + 1e-4);
        }
    }
}

#[test]
fn the_scale_has_five_steps_and_takes_any_number_to_the_nearest() {
    assert_eq!(Scale::STEPS, [100, 125, 150, 175, 200]);
    assert_eq!(Scale::default().percent(), 100);
    assert_eq!(Scale::default().factor(), 1.0);
    for (asked, given) in [
        (0, 100),
        (99, 100),
        (112, 100),
        (113, 125),
        (125, 125),
        (140, 150),
        (190, 200),
        (201, 200),
        (u16::MAX, 200),
    ] {
        assert_eq!(Scale::from(asked).percent(), given, "{asked}");
    }
    let names: Vec<String> = Scale::ALL.into_iter().map(Scale::name).collect();
    assert_eq!(names, ["100%", "125%", "150%", "175%", "200%"]);
    for (scale, step) in Scale::ALL.into_iter().zip(Scale::STEPS) {
        assert_eq!(scale.percent(), step);
    }
}

#[test]
fn density_changes_rows_fields_and_shelf_tabs() {
    let table: Vec<[f32; 3]> = Density::ALL
        .into_iter()
        .map(|d| [d.row_h(), d.field_h(), d.shelf_tab_h()])
        .collect();
    assert_eq!(
        table,
        [[22.0, 22.0, 24.0], [24.0, 24.0, 26.0], [28.0, 28.0, 30.0]]
    );
    assert_eq!(Density::default(), Density::Default);
    // The frame's fixed parts aren't in the table at all.
    assert_eq!(size::TAB_BAR_H + size::RIBBON_H, 100.0);
}

#[test]
fn spaced_capitals_are_made_here() {
    let job = text::caps("Library tools", text::CONTEXT_PREFIX, text_meta());
    assert_eq!(job.text, "LIBRARY TOOLS");
    let format = &job.sections[0].format;
    assert_eq!(format.font_id.size, 9.5);
    assert!((format.extra_letter_spacing - 9.5 * 0.06).abs() < 1e-5);
    let job = text::caps("Snapping", text::GROUP_LABEL, text_meta());
    assert_eq!(job.text, "Snapping", "a group's label keeps its letters");
    assert!((job.sections[0].format.extra_letter_spacing - 0.4).abs() < 1e-5);
    assert_eq!(job.sections[0].format.color, text_meta());
}

/// The second review pass's N-24: crops not picked were the old look's
/// amber, which with the orange accent looked like the picked crop.
/// Whichever accent is chosen, the picked crop (in the accent) and the
/// others differ clearly in lightness, so colour never tells them apart
/// alone.
#[test]
fn crops_not_picked_differ_in_lightness_from_every_accent() {
    let old_amber = hex(0xffd84d);
    assert!(crate::theme::contrast(old_amber, Accent::Orange.colour()) < 1.8);
    for accent in Accent::ALL {
        let ratio = crate::theme::contrast(CROP, accent.colour());
        assert!(ratio >= 1.8, "{}: {ratio}", accent.name());
    }
    // And the frame shows against the grey behind a picture.
    assert!(crate::theme::contrast(CROP, Color32::from_gray(60)) >= 2.0);
}
