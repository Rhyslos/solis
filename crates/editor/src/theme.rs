//! The editor's look, made from the design's tokens
//! ([decision 0100](../../../docs/decisions/0100-one-theme-five-accents.md)):
//! one theme, Neutral dark, and one accent colour chosen in the settings
//! (blue unless the user picks another). Text is Inter.
//!
//! This file turns the tokens (`tokens.rs`) into what egui's own controls
//! wear: buttons, fields, checkboxes, menus, tooltips, scroll bars and
//! windows take the design's fills, borders, radii and text sizes. It
//! also keeps the names the editor's drawing code has always used
//! ([`panel`], [`text_weak`] …), each now one of the tokens, so that code
//! changes only when its screen is rebuilt.
//!
//! The accent marks what's chosen and selected. Colours that mean
//! something keep their own whatever the accent ([decision 0060](../../../docs/decisions/0060-amber-and-colours-that-mean-something.md)):
//! Play is the one light button ([`GO`]), the console's levels have
//! theirs ([`level`]), and things in the Area panel and the Library share
//! a colour per kind ([`kind`]).
//!
//! Text colours are checked for readability by `theme_tests.rs` (WCAG
//! contrast 4.5:1 for normal text).

pub use crate::fonts::{INTER_REGULAR, INTER_SEMIBOLD, fonts};
pub use crate::interface_look::{
    InterfaceLook, accent_fill, in_use, notice, selected, selected_border,
};
pub use crate::tokens::size::{Density, Scale};
pub use crate::tokens::text::SEMIBOLD;
use crate::tokens::{self, size, text as type_style};
pub use crate::tokens::{Accent, AccentColors};
use egui::{
    Color32, Context, CornerRadius, FontId, Frame, Margin, Shadow, Stroke, Style, TextStyle,
    Visuals, vec2,
};

/// The window behind the panels.
pub fn background() -> Color32 {
    tokens::app()
}
/// Panel bodies.
pub fn panel() -> Color32 {
    tokens::panel()
}
/// Title bars above the panels, and the status bar.
pub fn header() -> Color32 {
    tokens::code()
}
/// The small menu row under a panel's title.
pub fn menu_row() -> Color32 {
    tokens::shelf_bar()
}
/// Text and number fields.
pub fn field() -> Color32 {
    tokens::field()
}
/// Hairlines between areas and around panels.
pub fn line() -> Color32 {
    tokens::line()
}
/// The thin border around buttons.
pub fn button_border() -> Color32 {
    tokens::strong_line()
}
/// A hovered button with no fill of its own.
pub fn hover() -> Color32 {
    tokens::hover()
}
/// A hovered button's border.
pub fn hover_border() -> Color32 {
    tokens::stronger_line()
}
/// A button being pressed.
pub fn pressed() -> Color32 {
    tokens::pressed()
}
/// The thin border around a card.
pub fn card_border() -> Color32 {
    tokens::card_border()
}
/// Normal text: rows and labels.
pub fn text() -> Color32 {
    tokens::text_body()
}
/// Bright text: headings and values.
pub fn text_strong() -> Color32 {
    tokens::text_primary()
}
/// Quieter text: field labels and hints.
pub fn text_weak() -> Color32 {
    tokens::text_label()
}
/// Console text.
pub fn text_console() -> Color32 {
    tokens::text_body()
}
/// Times in the console: the design's faint grey, for a mark that only
/// repeats the order the lines are already in.
pub fn text_time() -> Color32 {
    tokens::text_faint()
}
/// The panels drawn over the viewport, a little see-through.
pub fn overlay() -> Color32 {
    tokens::pill_fill()
}
/// Text and icons on a solid accent fill, on Play, or on any other light
/// fill.
pub fn on_accent() -> Color32 {
    tokens::ON_ACCENT
}

/// The Library's folder icon.
pub const FOLDER: Color32 = tokens::KIND_FOLDER;
/// Text that warns.
pub const WARNING: Color32 = tokens::WARNING;
/// Text that reports a failure.
pub const ERROR: Color32 = tokens::ERROR_TEXT;
/// A selected entity's collider (Phase 6): what physics bumps into.
pub const COLLIDER: Color32 = Color32::from_rgb(0x6e, 0xd8, 0x8a);
/// Play, whatever the accent: the one light button. [`on_accent`] reads
/// on it.
pub const GO: Color32 = tokens::PLAY_FILL;
/// The launcher's bar that says a new build is ready: a card, which the
/// launcher borders with the accent.
pub const UPDATE: Color32 = tokens::NEUTRAL_DARK.card;

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

/// A colour per kind of thing, shared by the Library (a tile's dot) and
/// the Area panel (an entity's icon), so a model or a sound looks the
/// same in both: the design's seven where a kind maps onto one (the
/// project lead's answer 21; round 2, `07-library-kinds-a.png`), and
/// today's own colours for sound events, code files and areas
/// (`07-library-kinds-b.png`).
pub mod kind {
    use crate::tokens;
    use egui::Color32;

    /// Models (the design's mesh).
    pub const MODEL: Color32 = tokens::KIND_MODEL;
    /// Assemblies, and a placed copy of one (the design's prefab).
    pub const ASSEMBLY: Color32 = tokens::KIND_ASSEMBLY;
    /// Area files: Solis's own green.
    pub const AREA: Color32 = Color32::from_rgb(0x58, 0xc2, 0x6a);
    /// Pictures, and a picture's crops (the design's texture).
    pub const PICTURE: Color32 = tokens::KIND_PICTURE;
    /// Game code: Solis's own orange.
    pub const CODE: Color32 = Color32::from_rgb(0xe8, 0x87, 0x5a);
    /// Sound files (the design's audio).
    pub const SOUND: Color32 = tokens::KIND_SOUND;
    /// Sound events: Solis's own teal.
    pub const SOUND_EVENT: Color32 = Color32::from_rgb(0x5d, 0xca, 0xa5);
    /// Sprite sheets (Aseprite files): a picture's colour.
    pub const SPRITE: Color32 = tokens::KIND_PICTURE;
    /// A sprite's clips and animators (the design's animation).
    pub const CLIPS: Color32 = tokens::KIND_ANIMATION;
    /// Tilesets and tile maps (the design's material green).
    pub const TILES: Color32 = tokens::KIND_MATERIAL;
    /// The sun (other lamps show their own light's colour).
    pub const SUN: Color32 = super::WARNING;
    /// The environment (sky and fog).
    pub const ENVIRONMENT: Color32 = Color32::from_rgb(0x4f, 0xb3, 0xa9);
    /// Cameras.
    pub const CAMERA: Color32 = Color32::from_rgb(0xc8, 0xc8, 0xc8);
    /// Text: notes (`.md`, `.txt`) and data (`.json`, `.csv`, `.toml`,
    /// `.ron`).
    pub const TEXT: Color32 = Color32::from_rgb(0x8f, 0xb8, 0xf0);
    /// Anything else.
    pub const OTHER: Color32 = Color32::from_rgb(0x8c, 0x8c, 0x8c);
}

/// The viewport's backdrop, where an area has no sky of its own.
pub mod backdrop {
    use crate::tokens;
    use egui::Color32;

    /// The sky straight up: the darkest part.
    pub fn sky_top() -> Color32 {
        tokens::sky_top()
    }
    /// The sky at the horizon.
    pub fn horizon() -> Color32 {
        tokens::horizon()
    }
    /// The ground just below the horizon (it lightens towards your feet).
    pub fn ground() -> Color32 {
        tokens::ground()
    }

    /// A colour as the renderer takes it: sRGB, 0 to 1.
    pub fn rgb(c: Color32) -> [f32; 3] {
        [c.r(), c.g(), c.b()].map(|v| f32::from(v) / 255.0)
    }
}

/// Axis colours: X red, Y green, Z blue (Z is up).
pub mod axis {
    use crate::tokens;
    use egui::Color32;

    /// The X axis.
    pub const X: Color32 = tokens::AXIS_X;
    /// The Y axis.
    pub const Y: Color32 = tokens::AXIS_Y;
    /// The Z axis (up).
    pub const Z: Color32 = tokens::AXIS_Z;
}

/// Applies the look to the whole editor: its fonts, its style and its
/// scale. The editor is always dark, whatever the system prefers.
pub fn apply(ctx: &Context, look: InterfaceLook) {
    ctx.set_fonts(fonts());
    restyle(ctx, look);
    ctx.set_theme(egui::ThemePreference::Dark);
}

/// Switches the whole editor to `look`.
pub fn restyle(ctx: &Context, look: InterfaceLook) {
    crate::interface_look::use_look(look);
    ctx.set_style_of(egui::Theme::Dark, style_with(look));
    ctx.set_style_of(egui::Theme::Light, style_with(look));
    ctx.set_zoom_factor(look.scale.factor());
}

/// The size of interface text, in points.
pub const TEXT_SIZE: f32 = type_style::UI_SIZE;

/// Colours, sizes and spacing for `look`.
pub fn style_with(look: InterfaceLook) -> Style {
    let mut style = Style {
        visuals: visuals(look.accent),
        ..Style::default()
    };
    style.text_styles = [
        (
            TextStyle::Small,
            FontId::proportional(type_style::SMALL_SIZE),
        ),
        (TextStyle::Body, type_style::ui()),
        (TextStyle::Button, type_style::ui()),
        (TextStyle::Heading, type_style::heading()),
        (TextStyle::Monospace, type_style::mono()),
    ]
    .into();
    let spacing = &mut style.spacing;
    spacing.item_spacing = vec2(size::GAP, size::ROW_GAP);
    // Buttons, fields and drop-downs are as tall as the density's field.
    let field_h = look.density.field_h();
    spacing.interact_size.y = field_h;
    let body = type_style::UI_SIZE + 3.0;
    spacing.button_padding = vec2(size::BUTTON_PAD, ((field_h - body) / 2.0).floor().max(1.0));
    spacing.menu_margin = Margin::same(size::MENU_PAD as i8);
    spacing.window_margin = Margin::same(size::PANEL_PAD as i8);
    spacing.icon_width = size::CHECKBOX + 2.0;
    spacing.icon_width_inner = size::CHECKBOX - 4.0;
    spacing.icon_spacing = size::GAP;
    // Scroll bars: thin, their handle on nothing, lighter under the
    // pointer, and out of sight until the pointer is over what scrolls
    // (Solis's own; the design's pictures show none).
    spacing.scroll = egui::style::ScrollStyle {
        floating: true,
        bar_width: size::SCROLL_BAR,
        floating_width: size::SCROLL_BAR,
        floating_allocated_width: 0.0,
        foreground_color: false,
        dormant_background_opacity: 0.0,
        active_background_opacity: 0.0,
        interact_background_opacity: 0.0,
        dormant_handle_opacity: 0.0,
        active_handle_opacity: 1.0,
        interact_handle_opacity: 1.0,
        ..egui::style::ScrollStyle::solid()
    };
    style
}

/// The colour scheme: what egui's own controls wear.
pub fn visuals(accent: Accent) -> Visuals {
    let line = size::LINE;
    let mut v = Visuals::dark();
    v.override_text_color = None;
    v.panel_fill = tokens::panel();
    // Menus, drop-down lists and windows that ask something: the ribbon's
    // grey with a strong border. Nothing has a shadow: depth comes from
    // lighter surfaces.
    v.window_fill = tokens::ribbon();
    v.window_stroke = Stroke::new(line, tokens::strong_line());
    v.window_corner_radius = CornerRadius::same(size::radius::CARD);
    v.menu_corner_radius = CornerRadius::same(size::radius::MENU);
    v.window_shadow = Shadow::NONE;
    v.popup_shadow = Shadow::NONE;
    v.extreme_bg_color = tokens::field();
    v.text_edit_bg_color = Some(tokens::field());
    v.faint_bg_color = tokens::card();
    v.code_bg_color = tokens::field();
    v.hyperlink_color = tokens::LINK;
    v.warn_fg_color = tokens::WARNING;
    v.error_fg_color = tokens::ERROR_TEXT;
    // A selected row: the accent mixed into the panel, with white text.
    // egui uses the same pair for selected text and for a focused field's
    // border, so those are white-edged until the kit's own field.
    v.selection.bg_fill = tokens::select_row(accent);
    v.selection.stroke = Stroke::new(line, tokens::text_emphasis());
    v.weak_text_color = Some(tokens::text_label());
    v.disabled_alpha = tokens::DISABLED_OPACITY;
    v.striped = false;

    let radius = CornerRadius::same(size::radius::CONTROL);
    let w = &mut v.widgets;
    // Labels, separators and frames that can't be interacted with.
    w.noninteractive.bg_fill = tokens::panel();
    w.noninteractive.weak_bg_fill = tokens::panel();
    w.noninteractive.bg_stroke = Stroke::new(line, tokens::line());
    w.noninteractive.fg_stroke = Stroke::new(line, tokens::text_body());
    w.noninteractive.corner_radius = radius;
    // A plain button at rest. `bg_fill` is what a scroll bar's handle and
    // a checkbox's box are filled with.
    w.inactive.bg_fill = tokens::strong_line();
    w.inactive.weak_bg_fill = tokens::line();
    w.inactive.bg_stroke = Stroke::new(line, tokens::strong_line());
    w.inactive.fg_stroke = Stroke::new(line, tokens::text_primary());
    w.inactive.corner_radius = radius;
    // Under the pointer: menu rows and buttons with no fill of their own
    // step to the hover grey.
    w.hovered.bg_fill = tokens::stronger_line();
    w.hovered.weak_bg_fill = tokens::hover();
    w.hovered.bg_stroke = Stroke::new(line, tokens::stronger_line());
    w.hovered.fg_stroke = Stroke::new(1.5, tokens::text_primary());
    w.hovered.corner_radius = radius;
    w.hovered.expansion = 0.0;
    // Being pressed or dragged, or holding the keyboard's focus: the
    // accent as a ring.
    w.active.bg_fill = tokens::stronger_line();
    w.active.weak_bg_fill = tokens::pressed();
    w.active.bg_stroke = Stroke::new(line, accent.colour());
    w.active.fg_stroke = Stroke::new(2.0, tokens::text_emphasis());
    w.active.corner_radius = radius;
    w.active.expansion = 0.0;
    // An open menu's button.
    w.open.bg_fill = tokens::ribbon();
    w.open.weak_bg_fill = tokens::line();
    w.open.bg_stroke = Stroke::new(line, tokens::strong_line());
    w.open.fg_stroke = Stroke::new(line, tokens::text_primary());
    w.open.corner_radius = radius;
    v
}

/// The frame of the top bar.
pub fn top_bar_frame() -> Frame {
    Frame::new()
        .fill(tokens::app())
        .inner_margin(Margin::symmetric(6, 0))
        .stroke(Stroke::new(size::LINE, line()))
}

/// The frame of the status bar along the bottom (spec 160): its fill and
/// its padding. Its one line, along the top, is drawn by the status bar.
pub fn status_bar_frame() -> Frame {
    Frame::new()
        .fill(tokens::code())
        .inner_margin(Margin::symmetric(size::ROW_PAD as i8, 0))
}

/// The frame of the small menu row under a panel's title.
pub fn menu_row_frame() -> Frame {
    Frame::new()
        .fill(menu_row())
        .inner_margin(Margin::symmetric(8, 3))
}

/// The frame of the panels drawn over the viewport: the design's pill.
pub fn overlay_frame() -> Frame {
    Frame::new()
        .fill(overlay())
        .stroke(Stroke::new(size::LINE, tokens::pill_border()))
        .corner_radius(CornerRadius::same(size::radius::CONTROL))
        .inner_margin(Margin::same(3))
}

/// Padding inside the panels other than the viewport.
pub fn panel_margin() -> Margin {
    Margin::symmetric(8, 6)
}

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
#[path = "theme_tests.rs"]
mod tests;
