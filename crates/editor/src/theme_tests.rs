//! Checks on the look: text reads on every surface it is put on, dark
//! text reads on every accent, and egui's controls wear the tokens.

use super::*;
use crate::tokens::size::radius;

/// The contrast the design promises for text (specification, line 64).
const READABLE: f32 = 4.5;

/// The surfaces text is put on, from the back of the window forward.
fn surfaces() -> Vec<(&'static str, Color32)> {
    vec![
        ("app", tokens::app()),
        ("field", tokens::field()),
        ("preview", tokens::preview()),
        ("shelf_bar", tokens::shelf_bar()),
        ("curves", tokens::curves()),
        ("code", tokens::code()),
        ("ribbon", tokens::ribbon()),
        ("panel", tokens::panel()),
        ("viewport", tokens::viewport()),
        ("card", tokens::card()),
        ("row_hover", tokens::row_hover()),
        ("hover", tokens::hover()),
        // A plain button, a keycap, the shelf tab in front.
        ("line", tokens::line()),
    ]
}

#[test]
fn every_text_grey_reads_on_every_surface_it_is_put_on() {
    let greys = [
        ("emphasis", tokens::text_emphasis()),
        ("primary", tokens::text_primary()),
        ("tile", tokens::text_tile()),
        ("body", tokens::text_body()),
        ("label", tokens::text_label()),
        ("meta", tokens::text_meta()),
    ];
    for (surface_name, surface) in surfaces() {
        for (name, grey) in greys {
            let ratio = contrast(grey, surface);
            assert!(
                ratio >= READABLE,
                "{name} on {surface_name} has contrast {ratio:.2}, below {READABLE}"
            );
        }
    }
}

/// The design calls its fourth-lowest grey "the lowest gray used for
/// readable text". The three below it are for marks that repeat what
/// something else says, and are the only text allowed under the promise.
#[test]
fn only_the_three_faint_greys_are_below_the_promise_and_each_still_shows() {
    let faint = [
        ("a log line's time", tokens::text_faint(), tokens::panel()),
        ("a log line's time", tokens::text_faint(), tokens::field()),
        ("a line number", tokens::text_line_number(), tokens::code()),
        (
            "the slash in a path",
            tokens::text_separator(),
            tokens::app(),
        ),
    ];
    for (what, grey, surface) in faint {
        let ratio = contrast(grey, surface);
        assert!(ratio < READABLE, "{what} now reads: take it off this list");
        assert!(
            ratio >= 2.5,
            "{what} has contrast {ratio:.2}: too faint to see"
        );
    }
    // Nothing a person must read is drawn in them: the names the drawing
    // code uses for readable text are all greys that pass.
    for readable in [text(), text_strong(), text_weak(), text_console()] {
        for (_, grey, _) in faint {
            assert_ne!(readable, grey);
        }
    }
    assert_eq!(text_time(), tokens::text_faint(), "the console's times");
}

#[test]
fn colours_that_mean_something_read_as_words_on_every_surface() {
    let words = [
        ("warning", WARNING),
        ("error", ERROR),
        ("success", tokens::SUCCESS_TEXT),
        ("link", tokens::LINK),
        ("link, hovered", tokens::LINK_HOVER),
        ("x", tokens::AXIS_X_TEXT),
        ("y", tokens::AXIS_Y_TEXT),
        ("z", tokens::AXIS_Z_TEXT),
        ("collider", COLLIDER),
        ("model", kind::MODEL),
        ("assembly", kind::ASSEMBLY),
        ("area", kind::AREA),
        ("picture", kind::PICTURE),
        ("code", kind::CODE),
        ("sound event", kind::SOUND_EVENT),
        ("sun", kind::SUN),
        ("environment", kind::ENVIRONMENT),
        ("camera", kind::CAMERA),
        ("other", kind::OTHER),
    ];
    for (surface_name, surface) in surfaces() {
        for (name, colour) in words {
            let ratio = contrast(colour, surface);
            assert!(
                ratio >= READABLE,
                "{name} on {surface_name} has contrast {ratio:.2}"
            );
        }
    }
}

#[test]
fn dark_text_reads_on_every_accent_and_on_play() {
    for accent in Accent::ALL {
        let ratio = contrast(on_accent(), accent.colors().fill);
        assert!(
            ratio >= READABLE,
            "dark on {} has contrast {ratio:.2}",
            accent.name()
        );
        // And white wouldn't: the rule flipped with the design.
        assert!(contrast(Color32::WHITE, accent.colors().fill) < READABLE);
    }
    assert!(contrast(on_accent(), GO) >= READABLE);
    assert!(contrast(on_accent(), ERROR) >= READABLE, "Stop's icon");
}

#[test]
fn white_text_reads_on_every_accent_mixed_into_a_surface() {
    for accent in Accent::ALL {
        // The launcher writes labels on its chosen row and its notices;
        // a list's selected row and the ribbon's active button carry
        // bright words only.
        let labels = [tokens::text_emphasis(), text(), text_weak()];
        for (name, fill, texts) in [
            ("a selected row", tokens::select_row(accent), &labels[..2]),
            ("a selected card", tokens::select_item(accent), &labels[..]),
            ("a notice", tokens::changes_pill(accent), &labels[..]),
            (
                "the active ribbon button",
                tokens::active_button(accent),
                &labels[..2],
            ),
        ] {
            for text in texts {
                let ratio = contrast(*text, fill);
                assert!(
                    ratio >= READABLE,
                    "{name} in {}: contrast {ratio:.2}",
                    accent.name()
                );
            }
        }
        // The accent itself shows on the surfaces it draws lines on.
        for (surface_name, surface) in surfaces() {
            assert!(
                contrast(accent.colour(), surface) >= 3.0,
                "{} on {surface_name}",
                accent.name()
            );
        }
    }
}

#[test]
fn the_standard_accent_is_the_design_s_blue() {
    assert_eq!(Accent::default(), Accent::Blue);
    assert_eq!(Accent::ALL[0], Accent::Blue);
    assert_eq!(Accent::Blue.colour(), Color32::from_rgb(0x5c, 0x9c, 0xff));
    assert_eq!(Accent::ALL.len(), 5);
}

#[test]
fn accent_names_and_colours_are_distinct() {
    let mut names: Vec<_> = Accent::ALL.iter().map(|a| a.name()).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), Accent::ALL.len());
    for a in Accent::ALL {
        for b in Accent::ALL {
            assert_eq!(a == b, a.colour() == b.colour());
        }
    }
}

#[test]
fn colours_that_mean_something_keep_theirs() {
    // Decision 0060's rule, with decision 0100's values: Play is the one
    // light button, and white text reads on each console level.
    for fill in [level::INFO, level::WARNING, level::ERROR] {
        let ratio = contrast(Color32::WHITE, fill);
        assert!(ratio >= READABLE, "white on {fill:?}: {ratio:.2}");
    }
    assert_ne!(level::INFO, level::WARNING);
    assert_ne!(level::WARNING, level::ERROR);
    // None is an accent, so a warning never looks like something selected.
    for accent in Accent::ALL {
        for meaning in [GO, WARNING, ERROR, tokens::SUCCESS, level::WARNING] {
            assert_ne!(meaning, accent.colour(), "{}", accent.name());
        }
    }
    // Play is lighter than anything else that is filled.
    assert_eq!(GO, tokens::text_primary());
}

#[test]
fn the_names_drawing_code_uses_are_the_design_s_tokens() {
    assert_eq!(background(), Color32::from_rgb(0x0e, 0x0e, 0x0e));
    assert_eq!(panel(), Color32::from_rgb(0x1a, 0x1a, 0x1a));
    assert_eq!(field(), Color32::from_rgb(0x12, 0x12, 0x12));
    assert_eq!(line(), Color32::from_rgb(0x26, 0x26, 0x26));
    assert_eq!(hover(), Color32::from_rgb(0x24, 0x24, 0x24));
    assert_eq!(text(), Color32::from_rgb(0xc8, 0xc8, 0xc8));
    assert_eq!(text_strong(), Color32::from_rgb(0xe6, 0xe6, 0xe6));
    assert_eq!(text_weak(), Color32::from_rgb(0xa6, 0xa6, 0xa6));
    assert_eq!(axis::X, Color32::from_rgb(0xe5, 0x53, 0x4b));
    assert_eq!(backdrop::sky_top(), Color32::from_rgb(0x14, 0x14, 0x14));
    assert_eq!(backdrop::horizon(), Color32::from_rgb(0x2a, 0x2a, 0x2a));
    assert_eq!(backdrop::ground(), Color32::from_rgb(0x20, 0x20, 0x20));
}

#[test]
fn egui_s_controls_wear_the_tokens() {
    let style = style_with(InterfaceLook {
        accent: Accent::Orange,
        ..InterfaceLook::default()
    });
    let v = &style.visuals;
    assert_eq!(v.panel_fill, tokens::panel());
    // Menus and windows.
    assert_eq!(v.window_fill, tokens::ribbon());
    assert_eq!(v.window_stroke.color, tokens::strong_line());
    assert_eq!(v.menu_corner_radius, CornerRadius::same(radius::MENU));
    assert_eq!(v.popup_shadow, Shadow::NONE);
    assert_eq!(v.window_shadow, Shadow::NONE);
    // Fields.
    assert_eq!(v.text_edit_bg_color, Some(tokens::field()));
    // A plain button: at rest, hovered, and focused with the accent.
    assert_eq!(v.widgets.inactive.weak_bg_fill, tokens::line());
    assert_eq!(v.widgets.inactive.bg_stroke.color, tokens::strong_line());
    assert_eq!(v.widgets.hovered.weak_bg_fill, tokens::hover());
    assert_eq!(v.widgets.active.bg_stroke.color, Accent::Orange.colour());
    for state in [
        &v.widgets.inactive,
        &v.widgets.hovered,
        &v.widgets.active,
        &v.widgets.open,
    ] {
        assert_eq!(state.corner_radius, CornerRadius::same(radius::CONTROL));
        assert_eq!(state.bg_stroke.width, 1.0, "every border is 1 px");
    }
    // A selected row is the accent mixed in, with white text.
    assert_eq!(v.selection.bg_fill, tokens::select_row(Accent::Orange));
    assert_eq!(v.selection.stroke.color, tokens::text_emphasis());
    assert_eq!(v.hyperlink_color, tokens::LINK);
    // Text sizes.
    assert_eq!(style.text_styles[&TextStyle::Body].size, 12.0);
    assert_eq!(style.text_styles[&TextStyle::Monospace].size, 11.0);
    // Scroll bars: 8 wide, on nothing.
    assert_eq!(style.spacing.scroll.bar_width, 8.0);
    assert_eq!(style.spacing.scroll.dormant_background_opacity, 0.0);
    assert_eq!(v.widgets.inactive.bg_fill, tokens::strong_line());
    assert_eq!(v.widgets.hovered.bg_fill, tokens::stronger_line());
}

#[test]
fn a_disabled_control_reads_as_the_faint_grey() {
    // Body text on a panel, drawn as see-through as a disabled control.
    let style = style_with(InterfaceLook::default());
    let alpha = style.visuals.disabled_alpha;
    let faded = |text: u8, surface: u8| {
        (f32::from(surface) + alpha * (f32::from(text) - f32::from(surface))).round() as u8
    };
    let shown = faded(tokens::text_body().r(), tokens::panel().r());
    assert_eq!(shown, tokens::text_faint().r());
}

#[test]
fn density_sets_how_tall_egui_s_controls_are_and_nothing_else() {
    let heights: Vec<f32> = Density::ALL
        .into_iter()
        .map(|density| {
            let style = style_with(InterfaceLook {
                density,
                ..InterfaceLook::default()
            });
            let padded = 15.0 + 2.0 * style.spacing.button_padding.y;
            assert!(
                padded <= style.spacing.interact_size.y,
                "{}: padding makes a button taller than a field",
                density.name()
            );
            style.spacing.interact_size.y
        })
        .collect();
    assert_eq!(heights, [22.0, 24.0, 28.0]);
    let compact = style_with(InterfaceLook {
        density: Density::Compact,
        ..InterfaceLook::default()
    });
    let standard = style_with(InterfaceLook::default());
    assert_eq!(compact.text_styles, standard.text_styles);
    assert_eq!(compact.visuals, standard.visuals);
}

#[test]
fn restyling_sets_the_scale_and_the_look_in_use() {
    let ctx = Context::default();
    let look = InterfaceLook {
        accent: Accent::Teal,
        scale: Scale::from(150),
        density: Density::Comfortable,
        ..InterfaceLook::default()
    };
    apply(&ctx, look);
    let mut output = ctx.run_ui(egui::RawInput::default(), |_| {});
    output.textures_delta.clear();
    assert_eq!(ctx.zoom_factor(), 1.5);
    assert_eq!(in_use(), look);
    assert_eq!(accent_fill(), Accent::Teal.colour());
    assert_eq!(size::field_h(), 28.0);
    assert_eq!(selected(), tokens::select_item(Accent::Teal));
    // Another thread still has the standard look.
    std::thread::spawn(|| assert_eq!(in_use(), InterfaceLook::default()))
        .join()
        .unwrap();
    crate::interface_look::use_look(InterfaceLook::default());
}

#[test]
fn the_tab_in_front_stands_out_in_the_dock() {
    let base = style_with(InterfaceLook::default());
    for accent in Accent::ALL {
        let dock = dock_style(&base, accent);
        // Its name is bright and the others are quieter, and its fill is
        // lighter than theirs, so colour is never the only cue.
        let front = contrast(dock.tab.active.text_color, dock.tab.active.bg_fill);
        let rest = contrast(dock.tab.inactive.text_color, dock.tab.inactive.bg_fill);
        assert!(front > rest + 3.0, "{front:.2} against {rest:.2}");
        assert!(rest >= READABLE);
        assert_ne!(dock.tab.active.bg_fill, dock.tab.inactive.bg_fill);
        assert_eq!(dock.tab_bar.height, size::DOC_TAB_H);
        assert_eq!(dock.separator.width, 1.0);
        assert_eq!(dock.separator.color_dragged, accent.colour());
    }
}
