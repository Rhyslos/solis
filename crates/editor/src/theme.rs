//! The editor's look ([decision 0004](../../../docs/decisions/0004-editor-layout.md)):
//! Aventail's dark neutral greys, flat panels split by thin dark lines,
//! plain-text tabs with the front one underlined, small corners, and one
//! accent colour chosen in Window → Accent colour (amber unless the user
//! picks another). Text is Inter.
//!
//! The accent marks what's chosen and selected. Colours that mean
//! something keep their own whatever the accent ([decision 0060](../../../docs/decisions/0060-amber-and-colours-that-mean-something.md)):
//! Play and Build & Play are blue ([`GO`]), Stop is red while the game
//! runs, the console's levels have theirs ([`level`]), and things in the
//! Area panel and the Library share a colour per kind ([`kind`]).
//!
//! Text colours are checked for readability by the tests at the bottom
//! (WCAG contrast 4.5:1 for normal text).

use egui::{
    Color32, Context, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Frame, Margin,
    Shadow, Stroke, Style, TextStyle, Visuals, vec2,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Window background, seen in the thin lines between panels.
pub const BACKGROUND: Color32 = Color32::from_rgb(0x1b, 0x1b, 0x1b);
/// Panel bodies.
pub const PANEL: Color32 = Color32::from_rgb(0x26, 0x26, 0x26);
/// Title bars above the panels, and the top bar.
pub const HEADER: Color32 = Color32::from_rgb(0x22, 0x22, 0x22);
/// The small menu row under a panel's title ("≡ Edit View …").
pub const MENU_ROW: Color32 = Color32::from_rgb(0x2a, 0x2a, 0x2a);
/// Text and number fields.
pub const FIELD: Color32 = Color32::from_rgb(0x1c, 0x1c, 0x1c);
/// Hairlines between areas and around fields.
pub const LINE: Color32 = Color32::from_rgb(0x16, 0x16, 0x16);
/// The thin border around buttons.
pub const BUTTON_BORDER: Color32 = Color32::from_rgb(0x3a, 0x3a, 0x3a);
/// A hovered button.
pub const HOVER: Color32 = Color32::from_rgb(0x3a, 0x3a, 0x3a);
/// Normal text.
pub const TEXT: Color32 = Color32::from_rgb(0xc8, 0xc8, 0xc8);
/// Bright text: headings and the front tab.
pub const TEXT_STRONG: Color32 = Color32::from_rgb(0xf0, 0xf0, 0xf0);
/// Quieter text: labels and hints.
pub const TEXT_WEAK: Color32 = Color32::from_rgb(0xa5, 0xa5, 0xa5);
/// Console text, a touch brighter than normal text (as in the mockup).
pub const TEXT_CONSOLE: Color32 = Color32::from_rgb(0xd4, 0xd4, 0xd4);
/// Times in the console.
pub const TEXT_TIME: Color32 = Color32::from_rgb(0x9a, 0x9a, 0x9a);
/// The Library's folder icon: a warm yellow.
pub const FOLDER: Color32 = Color32::from_rgb(0xe8, 0xc3, 0x5a);
/// Text that warns: a soft amber, readable on the panels.
pub const WARNING: Color32 = Color32::from_rgb(0xe8, 0xb3, 0x4a);
/// Text that reports a failure: a soft red, readable on the panels.
pub const ERROR: Color32 = Color32::from_rgb(0xf0, 0x7a, 0x72);
/// A selected entity's collider (Phase 6): what physics bumps into.
pub const COLLIDER: Color32 = Color32::from_rgb(0x6e, 0xd8, 0x8a);
/// Play and Build & Play, whatever the accent: petrol blue, white text
/// reads on it.
pub const GO: Color32 = Color32::from_rgb(0x1f, 0x6a, 0x8f);

/// The console's levels, whatever the accent: the fill of a level's filter
/// button while that level is shown. White text reads on each.
pub mod level {
    use egui::Color32;

    /// Information: a quiet grey.
    pub const INFO: Color32 = Color32::from_rgb(0x55, 0x55, 0x55);
    /// Warnings: a deep amber.
    pub const WARNING: Color32 = Color32::from_rgb(0x8a, 0x5c, 0x00);
    /// Errors: a deep red.
    pub const ERROR: Color32 = Color32::from_rgb(0xb0, 0x3a, 0x34);
}

/// A colour per kind of thing, shared by the Library (the stripe under a
/// tile) and the Area panel (an entity's icon), so a model or a sound looks
/// the same in both.
pub mod kind {
    use egui::Color32;

    /// Models.
    pub const MODEL: Color32 = Color32::from_rgb(0x4a, 0x9c, 0xd9);
    /// Assemblies (and a placed copy of one).
    pub const ASSEMBLY: Color32 = super::FOLDER;
    /// Area files.
    pub const AREA: Color32 = super::axis::Y;
    /// Pictures.
    pub const PICTURE: Color32 = Color32::from_rgb(0xb4, 0x7c, 0xf0);
    /// Game code.
    pub const CODE: Color32 = Color32::from_rgb(0xe8, 0x87, 0x5a);
    /// Sound files.
    pub const SOUND: Color32 = Color32::from_rgb(0x1d, 0x9e, 0x75);
    /// Sound events.
    pub const SOUND_EVENT: Color32 = Color32::from_rgb(0x5d, 0xca, 0xa5);
    /// The sun (other lamps show their own light's colour).
    pub const SUN: Color32 = super::WARNING;
    /// The environment (sky and fog).
    pub const ENVIRONMENT: Color32 = Color32::from_rgb(0x4f, 0xb3, 0xa9);
    /// Cameras.
    pub const CAMERA: Color32 = super::TEXT;
    /// Anything else.
    pub const OTHER: Color32 = super::TEXT_WEAK;
}

/// The panels drawn over the viewport (stats, view buttons, tools), a
/// little see-through.
pub const OVERLAY: Color32 = Color32::from_rgba_premultiplied(0x1f, 0x1f, 0x1f, 0xeb);

/// The viewport's backdrop.
pub mod backdrop {
    use egui::Color32;

    /// The sky straight up: the darkest part, as in the mockup.
    pub const SKY_TOP: Color32 = Color32::from_rgb(0x16, 0x18, 0x1d);
    /// The sky at the horizon.
    pub const HORIZON: Color32 = Color32::from_rgb(0x1c, 0x1e, 0x23);
    /// The ground just below the horizon (it lightens towards your feet).
    pub const GROUND: Color32 = Color32::from_rgb(0x1e, 0x21, 0x26);

    /// A colour as the renderer takes it: sRGB, 0 to 1.
    pub fn rgb(c: Color32) -> [f32; 3] {
        [c.r(), c.g(), c.b()].map(|v| f32::from(v) / 255.0)
    }
}

/// Axis colours: X red, Y green, Z blue (Z is up).
pub mod axis {
    use egui::Color32;

    /// The X axis.
    pub const X: Color32 = Color32::from_rgb(0xe8, 0x54, 0x5a);
    /// The Y axis.
    pub const Y: Color32 = Color32::from_rgb(0x58, 0xc2, 0x6a);
    /// The Z axis (up).
    pub const Z: Color32 = Color32::from_rgb(0x4a, 0x7c, 0xff);
}

/// The accent colour: marks the chosen tool, the front tab and the
/// selected row. Amber unless the user picks another (Window → Accent
/// colour).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Accent {
    /// Petrol blue, the standard until decision 0060.
    PetrolBlue,
    /// A soft, warm red.
    Red,
    /// A clear, saturated blue.
    Blue,
    /// A deep blue-green ("Marine green" in the menu; saved settings call
    /// it Green).
    Green,
    /// A dusky purple-pink.
    Plum,
    /// A natural, slightly yellow green.
    Olive,
    /// A deep red-orange, like polished copper.
    Copper,
    /// A warm sunset orange.
    Orange,
    /// An orange-yellow, like amber: the standard.
    #[default]
    Amber,
    /// A deep yellow, like old gold.
    Gold,
}

/// The shades of one accent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccentColors {
    /// Fills: the chosen tool, selected rows, the main buttons. White text
    /// reads on it.
    pub fill: Color32,
    /// Lines and highlights: the front tab's underline, links.
    pub light: Color32,
}

impl Accent {
    /// Every accent, in menu order.
    /// Aventail's accents (Petrol blue is Solis's own shade, and Olive a
    /// touch darker so white text reads on it), and four sun colours of
    /// Solis's own: the standard first, then round the colour wheel from
    /// red.
    pub const ALL: [Self; 10] = [
        Self::Amber,
        Self::Red,
        Self::Copper,
        Self::Orange,
        Self::Gold,
        Self::Olive,
        Self::Green,
        Self::PetrolBlue,
        Self::Blue,
        Self::Plum,
    ];

    /// The accent's name, as the menu shows it.
    pub fn name(self) -> &'static str {
        match self {
            Self::PetrolBlue => "Petrol blue",
            Self::Red => "Red",
            Self::Blue => "Blue",
            Self::Green => "Marine green",
            Self::Plum => "Plum",
            Self::Olive => "Olive",
            Self::Copper => "Copper",
            Self::Orange => "Orange",
            Self::Amber => "Amber",
            Self::Gold => "Gold",
        }
    }

    /// The accent's shades.
    pub fn colors(self) -> AccentColors {
        let (fill, light) = match self {
            Self::PetrolBlue => ([0x1f, 0x6a, 0x8f], [0x7c, 0xc4, 0xe4]),
            Self::Red => ([0xc2, 0x43, 0x3d], [0xe5, 0x73, 0x6b]),
            Self::Blue => ([0x24, 0x68, 0xd8], [0x5a, 0x95, 0xf2]),
            Self::Green => ([0x16, 0x6a, 0x62], [0x3e, 0x9d, 0x93]),
            Self::Plum => ([0x9b, 0x3f, 0x7a], [0xcf, 0x72, 0xab]),
            // Aventail's is 5d8530: white text on it reads at 4.3 to 1,
            // just under the 4.5 the tests ask.
            Self::Olive => ([0x58, 0x7e, 0x2e], [0x90, 0xbd, 0x5a]),
            // Yellows and oranges deep enough for white text; their light
            // shades carry the brightness.
            Self::Copper => ([0xb0, 0x4a, 0x1c], [0xec, 0x8a, 0x5a]),
            Self::Orange => ([0xb8, 0x56, 0x00], [0xf5, 0x9e, 0x4c]),
            Self::Amber => ([0x9a, 0x62, 0x00], [0xf2, 0xb6, 0x40]),
            Self::Gold => ([0x86, 0x6c, 0x00], [0xe6, 0xc8, 0x3c]),
        };
        let rgb = |[r, g, b]: [u8; 3]| Color32::from_rgb(r, g, b);
        AccentColors {
            fill: rgb(fill),
            light: rgb(light),
        }
    }
}

/// Name of the bolder font family, for headings.
pub const SEMIBOLD: &str = "semibold";

/// Inter, the interface font (SIL Open Font License 1.1; see licenses/fonts).
const INTER_REGULAR: &[u8] = include_bytes!("../assets/fonts/Inter-Regular.ttf");
const INTER_SEMIBOLD: &[u8] = include_bytes!("../assets/fonts/Inter-SemiBold.ttf");
const JETBRAINS_MONO: &[u8] = include_bytes!("../assets/fonts/JetBrainsMonoNL-Regular.ttf");

/// Applies the theme to the whole editor with `accent`. The editor is
/// always dark, whatever the system prefers.
pub fn apply(ctx: &Context, accent: Accent) {
    ctx.set_fonts(fonts());
    set_accent(ctx, accent);
    ctx.set_theme(egui::ThemePreference::Dark);
}

/// Switches the accent colour for the whole editor.
pub fn set_accent(ctx: &Context, accent: Accent) {
    ctx.set_style_of(egui::Theme::Dark, style_with(accent));
    ctx.set_style_of(egui::Theme::Light, style_with(accent));
}

/// Glyphs drawn as a browser draws them, without snapping their outlines
/// to the pixel grid: softer and closer to the mockup than egui's default.
fn soft() -> egui::FontTweak {
    egui::FontTweak {
        hinting: Some(false),
        ..Default::default()
    }
}

/// Inter for interface text and JetBrains Mono for monospace text, in front
/// of egui's own fonts (which stay as
/// fallbacks for symbols and other scripts).
///
/// Every symbol the editor writes must be in one of these fonts, or it
/// shows as an empty box: the `symbols_have_glyphs` test checks.
pub fn fonts() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "inter".into(),
        Arc::new(FontData::from_static(INTER_REGULAR).tweak(soft())),
    );
    fonts.font_data.insert(
        "inter-semibold".into(),
        Arc::new(FontData::from_static(INTER_SEMIBOLD).tweak(soft())),
    );
    let proportional = fonts.families.entry(FontFamily::Proportional).or_default();
    proportional.insert(0, "inter".into());
    // Inter lacks a few symbols the editor uses (≡ ▾ ▴); egui's monospace
    // font has them.
    proportional.push("Hack".into());
    let mut semibold = vec!["inter-semibold".to_string()];
    semibold.extend(proportional.iter().skip(1).cloned());
    fonts
        .families
        .insert(FontFamily::Name(SEMIBOLD.into()), semibold);
    // Monospace text (the console, the frame rate) is JetBrains Mono, as
    // in the mockup, with egui's Hack and then Inter for any symbol it
    // lacks.
    fonts.font_data.insert(
        "jetbrains-mono".into(),
        Arc::new(FontData::from_static(JETBRAINS_MONO)),
    );
    let monospace = fonts.families.entry(FontFamily::Monospace).or_default();
    monospace.insert(0, "jetbrains-mono".into());
    monospace.insert(2.min(monospace.len()), "inter".into());
    fonts
}

/// The size of interface text, in points.
pub const TEXT_SIZE: f32 = 12.0;

/// Colours, sizes and spacing, with the given accent.
pub fn style_with(accent: Accent) -> Style {
    let mut style = Style {
        visuals: visuals(accent),
        ..Style::default()
    };
    style.text_styles = [
        (TextStyle::Small, FontId::proportional(11.0)),
        (TextStyle::Body, FontId::proportional(TEXT_SIZE)),
        (TextStyle::Button, FontId::proportional(TEXT_SIZE)),
        (
            TextStyle::Heading,
            FontId::new(16.0, FontFamily::Name(SEMIBOLD.into())),
        ),
        (TextStyle::Monospace, FontId::monospace(11.0)),
    ]
    .into();
    let spacing = &mut style.spacing;
    spacing.item_spacing = vec2(6.0, 4.0);
    spacing.button_padding = vec2(8.0, 3.0);
    spacing.interact_size.y = 22.0;
    spacing.menu_margin = Margin::same(6);
    spacing.window_margin = Margin::same(12);
    style
}

/// The colour scheme.
pub fn visuals(accent: Accent) -> Visuals {
    let accent = accent.colors();
    let mut v = Visuals::dark();
    v.override_text_color = None;
    v.panel_fill = PANEL;
    v.window_fill = PANEL;
    v.window_stroke = Stroke::new(1.0, BUTTON_BORDER);
    v.window_corner_radius = CornerRadius::same(4);
    v.menu_corner_radius = CornerRadius::same(4);
    v.window_shadow = Shadow {
        offset: [0, 6],
        blur: 18,
        spread: 0,
        color: Color32::from_black_alpha(110),
    };
    v.popup_shadow = Shadow {
        offset: [0, 4],
        blur: 12,
        spread: 0,
        color: Color32::from_black_alpha(100),
    };
    v.extreme_bg_color = FIELD;
    v.text_edit_bg_color = Some(FIELD);
    v.faint_bg_color = MENU_ROW;
    v.code_bg_color = FIELD;
    v.hyperlink_color = accent.light;
    v.selection.bg_fill = accent.fill;
    v.selection.stroke = Stroke::new(1.0, TEXT_STRONG);
    v.weak_text_color = Some(TEXT_WEAK);

    let radius = CornerRadius::same(4);
    let w = &mut v.widgets;
    // Labels, separators and frames that can't be interacted with.
    w.noninteractive.bg_fill = PANEL;
    w.noninteractive.weak_bg_fill = PANEL;
    w.noninteractive.bg_stroke = Stroke::new(1.0, LINE);
    w.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);
    w.noninteractive.corner_radius = radius;
    // Buttons and fields at rest: buttons have a thin border.
    w.inactive.bg_fill = FIELD;
    w.inactive.weak_bg_fill = MENU_ROW;
    w.inactive.bg_stroke = Stroke::new(1.0, BUTTON_BORDER);
    w.inactive.fg_stroke = Stroke::new(1.0, TEXT_STRONG);
    w.inactive.corner_radius = radius;
    // Under the mouse.
    w.hovered.bg_fill = HOVER;
    w.hovered.weak_bg_fill = HOVER;
    w.hovered.bg_stroke = Stroke::new(1.0, Color32::from_rgb(0x4a, 0x4a, 0x4a));
    w.hovered.fg_stroke = Stroke::new(1.5, TEXT_STRONG);
    w.hovered.corner_radius = radius;
    w.hovered.expansion = 0.0;
    // Being pressed or dragged.
    w.active.bg_fill = Color32::from_rgb(0x44, 0x44, 0x44);
    w.active.weak_bg_fill = Color32::from_rgb(0x44, 0x44, 0x44);
    w.active.bg_stroke = Stroke::new(1.0, accent.light);
    w.active.fg_stroke = Stroke::new(2.0, TEXT_STRONG);
    w.active.corner_radius = radius;
    w.active.expansion = 0.0;
    // An open menu.
    w.open.bg_fill = PANEL;
    w.open.weak_bg_fill = MENU_ROW;
    w.open.bg_stroke = Stroke::new(1.0, BUTTON_BORDER);
    w.open.fg_stroke = Stroke::new(1.0, TEXT_STRONG);
    w.open.corner_radius = radius;
    v
}

/// The frame of the top bar.
pub fn top_bar_frame() -> Frame {
    Frame::new()
        .fill(HEADER)
        .inner_margin(Margin::symmetric(6, 0))
        .stroke(Stroke::new(1.0, LINE))
}

/// The frame of the small menu row under a panel's title.
pub fn menu_row_frame() -> Frame {
    Frame::new()
        .fill(MENU_ROW)
        .inner_margin(Margin::symmetric(8, 3))
}

/// The frame of the panels drawn over the viewport.
pub fn overlay_frame() -> Frame {
    Frame::new()
        .fill(OVERLAY)
        .stroke(Stroke::new(1.0, BUTTON_BORDER))
        .corner_radius(CornerRadius::same(4))
        .inner_margin(Margin::same(3))
}

/// The frame of the area the panels are docked in.
pub fn dock_frame() -> Frame {
    Frame::new().fill(BACKGROUND).inner_margin(Margin::same(0))
}

/// Padding inside the panels other than the viewport.
pub fn panel_margin() -> Margin {
    Margin::symmetric(8, 6)
}

/// The docked panels' look: plain-text tabs on the title bar, thin dark
/// lines between panels, and the drop overlay in the accent colour. The
/// front tab's underline is drawn by the workbench, which knows which tab
/// is in front.
pub fn dock_style(base: &Style, accent: Accent) -> egui_dock::Style {
    let accent = accent.colors();
    let mut s = egui_dock::Style::from_egui(base);
    s.dock_area_padding = None;
    s.main_surface_border_stroke = Stroke::NONE;
    s.separator.width = 1.0;
    s.separator.extra = MIN_PANEL_SIZE;
    s.separator.color_idle = LINE;
    s.separator.color_hovered = HOVER;
    s.separator.color_dragged = accent.light;
    s.tab_bar.bg_fill = HEADER;
    s.tab_bar.height = 28.0;
    s.tab_bar.hline_color = HEADER;
    s.tab_bar.corner_radius = CornerRadius::ZERO;
    s.tab.tab_body.bg_fill = PANEL;
    // No padding around tab bodies, so the viewport fills its panel edge to
    // edge; the other panels add their own (see `panel_margin`).
    s.tab.tab_body.inner_margin = Margin::ZERO;
    s.tab.tab_body.stroke = Stroke::NONE;
    s.tab.tab_body.corner_radius = CornerRadius::ZERO;
    s.tab.hline_below_active_tab_name = false;
    for tab in [
        &mut s.tab.active,
        &mut s.tab.focused,
        &mut s.tab.active_with_kb_focus,
        &mut s.tab.focused_with_kb_focus,
    ] {
        tab.bg_fill = HEADER;
        tab.text_color = TEXT_STRONG;
        tab.outline_color = Color32::TRANSPARENT;
        tab.corner_radius = CornerRadius::ZERO;
    }
    for tab in [&mut s.tab.inactive, &mut s.tab.inactive_with_kb_focus] {
        tab.bg_fill = HEADER;
        tab.text_color = TEXT_WEAK;
        tab.outline_color = Color32::TRANSPARENT;
        tab.corner_radius = CornerRadius::ZERO;
    }
    s.tab.hovered.bg_fill = HEADER;
    s.tab.hovered.text_color = TEXT_STRONG;
    s.tab.hovered.outline_color = Color32::TRANSPARENT;
    s.overlay.selection_color = accent.light.gamma_multiply(0.35);
    s.overlay.button_color = TEXT_WEAK;
    s
}

/// The smallest a panel can be dragged to, in logical pixels: a title bar
/// and a little room.
pub const MIN_PANEL_SIZE: f32 = 60.0;

/// WCAG contrast ratio between two opaque colours (1 to 21).
pub fn contrast(a: Color32, b: Color32) -> f32 {
    fn luminance(c: Color32) -> f32 {
        let channel = |v: u8| {
            let s = f32::from(v) / 255.0;
            if s <= 0.040_45 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(c.r()) + 0.7152 * channel(c.g()) + 0.0722 * channel(c.b())
    }
    let (la, lb) = (luminance(a), luminance(b));
    let (light, dark) = if la > lb { (la, lb) } else { (lb, la) };
    (light + 0.05) / (dark + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

    const READABLE: f32 = 4.5;

    #[test]
    fn text_reads_on_every_surface() {
        for surface in [PANEL, HEADER, MENU_ROW, FIELD, BACKGROUND] {
            for text in [
                TEXT,
                TEXT_STRONG,
                TEXT_WEAK,
                TEXT_CONSOLE,
                TEXT_TIME,
                WARNING,
                ERROR,
                kind::MODEL,
                kind::ASSEMBLY,
                kind::AREA,
                kind::PICTURE,
                kind::CODE,
                kind::SOUND_EVENT,
                kind::SUN,
                kind::ENVIRONMENT,
                kind::CAMERA,
                kind::OTHER,
            ] {
                let ratio = contrast(text, surface);
                assert!(
                    ratio >= READABLE,
                    "{text:?} on {surface:?} has contrast {ratio:.2}, below {READABLE}"
                );
            }
        }
    }

    #[test]
    fn white_text_reads_on_every_accent() {
        for accent in Accent::ALL {
            let ratio = contrast(Color32::WHITE, accent.colors().fill);
            assert!(
                ratio >= READABLE,
                "white on {} has contrast {ratio:.2}",
                accent.name()
            );
        }
    }

    #[test]
    fn the_front_tab_stands_out() {
        // The front tab's name is bright and the others are quieter, so the
        // accent underline (the mockup's deep accent) is never the only cue.
        assert!(contrast(TEXT_STRONG, HEADER) > contrast(TEXT_WEAK, HEADER) + 3.0);
        for accent in Accent::ALL {
            assert!(
                contrast(accent.colors().fill, HEADER) >= 1.8,
                "{}",
                accent.name()
            );
            assert!(
                contrast(accent.colors().light, HEADER) >= 3.0,
                "{}",
                accent.name()
            );
        }
    }

    #[test]
    fn the_standard_accent_is_amber() {
        assert_eq!(Accent::default(), Accent::Amber);
        assert_eq!(Accent::ALL[0], Accent::Amber);
    }

    #[test]
    fn colours_that_mean_something_keep_theirs() {
        // Decision 0060: Play is blue, and white text reads on it and on
        // each console level, whatever the accent.
        for fill in [GO, level::INFO, level::WARNING, level::ERROR] {
            let ratio = contrast(Color32::WHITE, fill);
            assert!(
                ratio >= READABLE,
                "white on {fill:?} has contrast {ratio:.2}"
            );
        }
        // None of them is the standard accent, so a warning never looks
        // like something selected.
        for fill in [GO, level::INFO, level::ERROR] {
            assert_ne!(fill, Accent::default().colors().fill);
        }
        assert_ne!(level::WARNING, Accent::Amber.colors().fill);
        // Three levels, three different colours.
        assert_ne!(level::INFO, level::WARNING);
        assert_ne!(level::WARNING, level::ERROR);
    }

    #[test]
    fn a_saved_accent_still_reads_after_more_were_added() {
        // Settings saved before Blue and Olive (and before Green was
        // called Marine green in the menu) keep their choice.
        for (saved, accent) in [
            ("PetrolBlue", Accent::PetrolBlue),
            ("Red", Accent::Red),
            ("Green", Accent::Green),
            ("Plum", Accent::Plum),
        ] {
            assert_eq!(ron::from_str::<Accent>(saved).ok(), Some(accent), "{saved}");
        }
        for accent in Accent::ALL {
            let text = ron::to_string(&accent).unwrap();
            assert_eq!(ron::from_str::<Accent>(&text).unwrap(), accent);
        }
    }

    #[test]
    fn the_accents_are_aventail_s() {
        // Decision 0007: Aventail's look may be copied. These are its
        // shades; Petrol blue is Solis's own.
        let fill = |a: Accent| a.colors().fill;
        assert_eq!(fill(Accent::Blue), Color32::from_rgb(0x24, 0x68, 0xd8));
        assert_eq!(fill(Accent::Red), Color32::from_rgb(0xc2, 0x43, 0x3d));
        assert_eq!(fill(Accent::Plum), Color32::from_rgb(0x9b, 0x3f, 0x7a));
        assert_eq!(Accent::Green.name(), "Marine green");
        assert_eq!(Accent::ALL.len(), 10);
        let mut all = Accent::ALL.to_vec();
        all.sort_unstable_by_key(|a| *a as u8);
        all.dedup();
        assert_eq!(all.len(), 10, "each listed once");
    }

    #[test]
    fn accent_names_are_distinct() {
        let mut names: Vec<_> = Accent::ALL.iter().map(|a| a.name()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), Accent::ALL.len());
    }

    #[test]
    fn style_uses_the_chosen_accent() {
        let style = style_with(Accent::Red);
        assert_eq!(style.visuals.selection.bg_fill, Accent::Red.colors().fill);
        assert_eq!(style.visuals.panel_fill, PANEL);
    }

    #[test]
    fn fonts_put_inter_first() {
        let fonts = fonts();
        assert_eq!(
            fonts.families[&FontFamily::Proportional]
                .first()
                .map(String::as_str),
            Some("inter")
        );
        assert!(
            fonts
                .families
                .contains_key(&FontFamily::Name(SEMIBOLD.into()))
        );
    }

    /// What egui draws for a character no font has.
    const REPLACEMENT: char = '\u{25FB}';

    /// Where in the font image a character's picture is: two characters
    /// share one only if both fall back to the replacement box.
    fn glyph_image(ctx: &Context, c: char, family: &FontFamily) -> Option<([u16; 2], [u16; 2])> {
        let font = egui::FontId::new(13.0, family.clone());
        let galley = ctx.fonts_mut(|f| f.layout_no_wrap(c.to_string(), font, Color32::WHITE));
        galley
            .rows
            .first()
            .and_then(|row| row.row.glyphs.first())
            .map(|glyph| (glyph.uv_rect.min, glyph.uv_rect.max))
    }

    /// Every non-ASCII character in the editor's code (outside comments)
    /// can be drawn in both text families, so nothing shows as a box.
    #[test]
    fn symbols_have_glyphs() {
        let ctx = Context::default();
        ctx.set_fonts(fonts());
        let mut output = ctx.run_ui(egui::RawInput::default(), |_| {});
        output.textures_delta.clear();
        let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut checked = 0;
        for entry in std::fs::read_dir(source).into_iter().flatten().flatten() {
            let text = std::fs::read_to_string(entry.path()).unwrap_or_default();
            for line in text.lines() {
                let code = line.split("//").next().unwrap_or_default();
                for c in code.chars().filter(|c| !c.is_ascii()) {
                    for family in [FontFamily::Proportional, FontFamily::Monospace] {
                        assert_ne!(
                            glyph_image(&ctx, c, &family),
                            glyph_image(&ctx, REPLACEMENT, &family),
                            "{c:?} in {:?} has no {family:?} glyph",
                            entry.path()
                        );
                    }
                    checked += 1;
                }
            }
        }
        assert!(
            checked > 0,
            "found no symbols, so the test looked in the wrong place"
        );
    }
}
