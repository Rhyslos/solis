//! The design's tokens: every colour of the editor's look, as named values
//! ([decision 0100](../../../docs/decisions/0100-one-theme-five-accents.md)).
//!
//! The source is the project lead's design, `docs/design/g-layout/`: the
//! tables of its specification (lines 31 to 125), and for what those leave
//! out, the mock-up it describes ("from the mock-up" below) or Solis's own
//! choice in the same language ("Solis's own"). Sizes and radii are in
//! [`size`], type styles in [`text`].
//!
//! There is one theme, Neutral dark. Its greys are read through functions
//! ([`panel`], [`text_body`] …), not constants, on purpose: the design
//! leaves a high-contrast option open, and a second [`Palette`] later then
//! touches no code that draws. Colours that mean something are constants:
//! they keep their value whatever the palette.
//!
//! A colour is written as numbers here and in `theme.rs` only; a test
//! (`source_rules_tests.rs`) fails on one anywhere else.

pub mod size;
pub mod text;

use egui::Color32;
use serde::{Deserialize, Serialize};

/// A neutral grey: the design's greys are pure (red, green and blue equal).
const fn grey(v: u8) -> Color32 {
    Color32::from_rgb(v, v, v)
}

/// A colour from its hex value, `0xRRGGBB`.
const fn hex(v: u32) -> Color32 {
    Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

/// The neutral colours of a theme: surfaces from the back of the window
/// forward (the further forward, the lighter), lines, and text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    /// The window's root, the tab bar, document tab strips, the wells of
    /// segmented controls.
    pub app: Color32,
    /// Text and number fields, the Console.
    pub field: Color32,
    /// Preview boxes.
    pub preview: Color32,
    /// The shelf's bar, graph backgrounds, File pages, a document tab
    /// that isn't in front.
    pub shelf_bar: Color32,
    /// Curve editors.
    pub curves: Color32,
    /// The status bar, column header strips, the code group.
    pub code: Color32,
    /// The ribbon's body and its tab in front, the command search, menus.
    pub ribbon: Color32,
    /// Shelf columns, tables, project cards.
    pub panel: Color32,
    /// The viewport's backdrop and the document tab in front.
    pub viewport: Color32,
    /// The 1 px gap between editor groups; track and lane lines.
    pub group_gap: Color32,
    /// Cards inside the shelf.
    pub card: Color32,
    /// A hovered row; the dividers between setting rows.
    pub row_hover: Color32,
    /// A hovered button; a tile's thumbnail.
    pub hover: Color32,
    /// Panel borders and 1 px separators; the shelf tab in front; plain
    /// buttons, keycaps and chips.
    pub line: Color32,
    /// Card borders.
    pub card_border: Color32,
    /// Field borders, slider tracks, the active segment.
    pub field_border: Color32,
    /// The border of pills floating over the viewport.
    pub pill_border: Color32,
    /// A toggle that is off; plain buttons' borders.
    pub strong_line: Color32,
    /// Dashed "add" borders, chip and keycap borders, the shelf's grip,
    /// a checkbox's outline.
    pub stronger_line: Color32,
    /// A graph node's border.
    pub node_border: Color32,
    /// A gamepad chip's border.
    pub pad_border: Color32,
    /// Selected rows, tabs in front, an entity's name.
    pub text_emphasis: Color32,
    /// Values, headings, what's in a field.
    pub text_primary: Color32,
    /// Tile names (from the mock-up).
    pub text_tile: Color32,
    /// Tree and list rows, ribbon button labels.
    pub text_body: Color32,
    /// Field labels, tabs that aren't in front, the status bar.
    pub text_label: Color32,
    /// Counts, kinds, hints, group labels: the lowest grey for text that
    /// must be read.
    pub text_meta: Color32,
    /// A log line's time, an empty slot. Below the contrast the design
    /// promises: only for marks that repeat what something else says.
    pub text_faint: Color32,
    /// Code line numbers (likewise below it).
    pub text_line_number: Color32,
    /// The slash in a path (likewise below it).
    pub text_separator: Color32,
    /// The viewport's sky straight up, where an area has no sky of its own.
    pub sky_top: Color32,
    /// That sky at the horizon.
    pub horizon: Color32,
    /// The floor just below the horizon.
    pub ground: Color32,
}

/// The one theme (specification, lines 35 to 64).
pub const NEUTRAL_DARK: Palette = Palette {
    app: grey(0x0e),
    field: grey(0x12),
    preview: grey(0x11),
    shelf_bar: grey(0x14),
    curves: grey(0x15),
    code: grey(0x16),
    ribbon: grey(0x18),
    panel: grey(0x1a),
    viewport: grey(0x1c),
    group_gap: grey(0x1e),
    card: grey(0x1f),
    row_hover: grey(0x22),
    hover: grey(0x24),
    line: grey(0x26),
    card_border: grey(0x2a),
    field_border: grey(0x2c),
    pill_border: grey(0x2e),
    strong_line: grey(0x33),
    stronger_line: grey(0x3a),
    node_border: grey(0x36),
    pad_border: grey(0x44),
    text_emphasis: grey(0xff),
    text_primary: grey(0xe6),
    text_tile: grey(0xd0),
    text_body: grey(0xc8),
    text_label: grey(0xa6),
    text_meta: grey(0x8c),
    text_faint: grey(0x6e),
    text_line_number: grey(0x5e),
    text_separator: grey(0x5a),
    sky_top: grey(0x14),
    horizon: grey(0x2a),
    ground: grey(0x20),
};

/// The palette in use.
pub fn palette() -> &'static Palette {
    &NEUTRAL_DARK
}

macro_rules! neutral {
    ($($name:ident),* $(,)?) => {
        $(
            #[doc = concat!("The palette's `", stringify!($name), "` (see [`Palette`]).")]
            pub fn $name() -> Color32 {
                palette().$name
            }
        )*
        /// Every neutral colour with its name, for the token sheet.
        fn neutrals() -> Vec<(&'static str, Color32)> {
            vec![$((stringify!($name), $name())),*]
        }
    };
}

neutral!(
    app,
    field,
    preview,
    shelf_bar,
    curves,
    code,
    ribbon,
    panel,
    viewport,
    group_gap,
    card,
    row_hover,
    hover,
    line,
    card_border,
    field_border,
    pill_border,
    strong_line,
    stronger_line,
    node_border,
    pad_border,
    text_emphasis,
    text_primary,
    text_tile,
    text_body,
    text_label,
    text_meta,
    text_faint,
    text_line_number,
    text_separator,
    sky_top,
    horizon,
    ground,
);

/// The fill of a button being pressed. Solis's own, one step further
/// forward than a hovered one.
pub fn pressed() -> Color32 {
    card_border()
}

macro_rules! meaning {
    ($($(#[$doc:meta])* $name:ident = $value:expr;)*) => {
        $($(#[$doc])* pub const $name: Color32 = hex($value);)*
        /// Every colour that means something with its name, for the token
        /// sheet.
        fn meanings() -> Vec<(&'static str, Color32)> {
            vec![$((stringify!($name), $name)),*]
        }
    };
}

meaning! {
    /// Text and icons on a solid accent fill or on the Play button.
    ON_ACCENT = 0x0e0e0e;
    /// A link in a sentence.
    LINK = 0x7fb2ff;
    /// A link under the pointer.
    LINK_HOVER = 0xa9cbff;
    /// Went well: a dot, a level, an icon. Always beside a word.
    SUCCESS = 0x4fc28a;
    /// A warning; the dot after an unsaved document's name.
    WARNING = 0xe8b04a;
    /// An error, as a dot or an icon.
    ERROR = 0xf06a6a;
    /// An error as words; Record.
    ERROR_TEXT = 0xf08a84;
    /// "Up to date", as words.
    SUCCESS_TEXT = 0x8fd4a8;
    /// The X axis on the gizmo.
    AXIS_X = 0xe5534b;
    /// The Y axis on the gizmo.
    AXIS_Y = 0x6cc24a;
    /// The Z axis on the gizmo.
    AXIS_Z = 0x4c8dff;
    /// The small X in a number field.
    AXIS_X_TEXT = 0xf08a84;
    /// The small Y in a number field.
    AXIS_Y_TEXT = 0x8fd47a;
    /// The small Z in a number field.
    AXIS_Z_TEXT = 0x7fb2ff;
    /// A folder.
    KIND_FOLDER = 0xc9a35a;
    /// A model (the design's "mesh").
    KIND_MODEL = 0x5fc4d6;
    /// A material.
    KIND_MATERIAL = 0x6cc28a;
    /// A picture (the design's "texture").
    KIND_PICTURE = 0xe57f6a;
    /// An animation.
    KIND_ANIMATION = 0xb48cff;
    /// A sound (the design's "audio").
    KIND_SOUND = 0xe8b04a;
    /// An assembly (the design's "prefab").
    KIND_ASSEMBLY = 0x7fb2ff;
    /// A graph pin carrying one number.
    PIN_NUMBER = 0x9a9a9a;
    /// A graph pin carrying three numbers.
    PIN_VECTOR = 0xe8c95a;
    /// A graph pin carrying a colour.
    PIN_COLOUR = 0xc38cff;
    /// A graph pin carrying a picture.
    PIN_PICTURE = 0xe57f6a;
    /// Code: a keyword.
    CODE_KEYWORD = 0x8fb8f0;
    /// Code: text in quotes.
    CODE_STRING = 0xb8cf9a;
    /// Code: a comment.
    CODE_COMMENT = 0x7a7a7a;
    /// Code: a number.
    CODE_NUMBER = 0xe0b878;
    /// Code: everything else.
    CODE_PLAIN = 0xd4d4d4;
    /// The Play button: the one light button on screen.
    PLAY_FILL = 0xe6e6e6;
    /// A keycap whose key is used twice: its fill.
    CONFLICT_FILL = 0x2e2617;
    /// That keycap's border.
    CONFLICT_BORDER = 0x8a6a2a;
    /// That keycap's letters.
    CONFLICT_TEXT = 0xf2d08a;
    /// The banner that says keys are used twice: its fill.
    CONFLICT_BANNER = 0x1f1b12;
    /// That banner's line along the top.
    CONFLICT_BANNER_LINE = 0x3a3020;
    /// That banner's words.
    CONFLICT_BANNER_TEXT = 0xe8d2a2;
    /// The round badge counting problems (from the mock-up).
    PROBLEM_BADGE = 0x4a2422;
    /// A frame's bar in the frame-time graph, and the budget line.
    PROFILER_BAR = 0x4a4a4a;
    /// A frame's bar over its budget.
    PROFILER_OVER = 0xa6a6a6;
    /// A block of memory tidying in the frame timeline.
    PROFILER_PAUSE = 0x4a2a2a;
    /// The words on that block.
    PROFILER_PAUSE_TEXT = 0xffd6d3;
    /// A level meter's foot.
    METER_LOW = 0x6e6e6e;
    /// A level meter's top.
    METER_HIGH = 0x9a9a9a;
    /// A fader's cap, at its top.
    FADER_TOP = 0x5a5a5a;
    /// A fader's cap, at its foot.
    FADER_FOOT = 0x3a3a3a;
    /// A sample effect's brightest colour.
    SAMPLE_SPARK = 0xfff8e0;
    /// A sample effect's glow.
    SAMPLE_GLOW = 0xfff2c8;
    /// A sample effect's flame.
    SAMPLE_FLAME = 0xf2c46d;
    /// A crop's frame and name on its picture while it isn't picked (the
    /// picked one is in the accent). Darker than every accent, so the two
    /// differ in lightness and not by colour alone (Solis's own; the
    /// specification's grey for a curve shown for context).
    CROP = 0x6e6e6e;
    /// A piece game code made, on the screen's canvas: its frame under the
    /// pointer and the badge saying "code" (from the mock-up).
    MADE_BY_CODE = 0xffd84d;
}

/// How see-through the pills floating over the viewport are (their fill is
/// [`field`] at this share).
pub const PILL_OPACITY: f32 = 0.85;
/// How much of a tile being dragged out still shows (spec 274: 55%).
pub const DRAGGED_OPACITY: f32 = 0.55;
/// How see-through the label box is while placing (its fill is [`ribbon`]
/// at this share).
pub const LABEL_BOX_OPACITY: f32 = 0.95;
/// A control that exists but can't be used right now is drawn this
/// see-through: body text on a panel then reads as [`text_faint`]. Solis's
/// own; the design shows no disabled control.
pub const DISABLED_OPACITY: f32 = 0.48;

/// `colour`, `share` of the way from see-through to solid.
fn see_through(colour: Color32, share: f32) -> Color32 {
    let alpha = (share * 255.0).round() as u8;
    Color32::from_rgba_unmultiplied(colour.r(), colour.g(), colour.b(), alpha)
}

/// The fill of a pill floating over the viewport.
pub fn pill_fill() -> Color32 {
    see_through(field(), PILL_OPACITY)
}

/// The fill of the label box while placing.
pub fn label_box_fill() -> Color32 {
    see_through(ribbon(), LABEL_BOX_OPACITY)
}

/// The accent colour: marks what is selected, active or focused. Blue
/// unless the user picks another.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Accent {
    /// The standard: a clear blue.
    #[default]
    Blue,
    /// No colour: a light grey.
    Grey,
    /// A warm orange.
    Orange,
    /// A blue-green.
    Teal,
    /// A soft purple.
    Purple,
}

/// The shades of one accent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccentColors {
    /// Solid fills: the active tool, the main button. [`ON_ACCENT`] reads
    /// on it.
    pub fill: Color32,
    /// Lines: the underline of the tab in front, a focused field's border.
    /// The same colour as the fill: the design has one shade.
    pub light: Color32,
}

impl Accent {
    /// Every accent, in the order the settings list them.
    pub const ALL: [Self; 5] = [
        Self::Blue,
        Self::Grey,
        Self::Orange,
        Self::Teal,
        Self::Purple,
    ];

    /// The accent's name, as the settings show it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Blue => "Blue",
            Self::Grey => "Grey",
            Self::Orange => "Orange",
            Self::Teal => "Teal",
            Self::Purple => "Purple",
        }
    }

    /// The accent's colour (specification, line 70).
    pub const fn colour(self) -> Color32 {
        match self {
            Self::Blue => hex(0x5c9cff),
            Self::Grey => hex(0xd4d4d4),
            Self::Orange => hex(0xf2994a),
            Self::Teal => hex(0x4fc2a8),
            Self::Purple => hex(0xb48cff),
        }
    }

    /// The accent's shades, under the names the editor's drawing code
    /// already uses.
    pub fn colors(self) -> AccentColors {
        AccentColors {
            fill: self.colour(),
            light: self.colour(),
        }
    }
}

/// `share` (0 to 1) of `colour` over `surface`: a straight blend of the
/// two stored values, which is how the mock-up mixes. Mixed here against
/// a known surface, never by drawing a see-through colour, so the result
/// is the same on every graphics card.
pub fn mix(colour: Color32, surface: Color32, share: f32) -> Color32 {
    let share = if share.is_nan() {
        0.0
    } else {
        share.clamp(0.0, 1.0)
    };
    let channel =
        |c: u8, s: u8| (f32::from(c) * share + f32::from(s) * (1.0 - share)).round() as u8;
    Color32::from_rgb(
        channel(colour.r(), surface.r()),
        channel(colour.g(), surface.g()),
        channel(colour.b(), surface.b()),
    )
}

/// How much accent goes into each mixed fill. The specification gives
/// "16 to 24%" (line 71); the exact shares are from the mock-up.
pub mod share {
    /// A selected row in a tree or a list (the scene list).
    pub const SELECTED_ROW: f32 = 0.22;
    /// A selected tile, table row or header.
    pub const SELECTED: f32 = 0.16;
    /// A selected track.
    pub const TRACK: f32 = 0.18;
    /// The active large ribbon button.
    pub const ACTIVE_BUTTON: f32 = 0.22;
    /// The selected lane or module.
    pub const LANE: f32 = 0.08;
    /// The selected row in the input table.
    pub const INPUT_ROW: f32 = 0.12;
    /// A keycap waiting for a key.
    pub const LISTENING: f32 = 0.20;
    /// The "changed from defaults" pill; a selected band card.
    pub const CHANGES: f32 = 0.14;
    /// The border of the open project's card.
    pub const OPEN_CARD: f32 = 0.60;
    /// The picked block in the frame timeline.
    pub const CULPRIT: f32 = 0.45;
    /// The line inside a contextual tab's top edge.
    pub const CONTEXT_TAB: f32 = 0.55;
    /// The ghost's fill while placing.
    pub const GHOST: f32 = 0.22;
    /// The ghost's outline and its bounds box.
    pub const GHOST_OUTLINE: f32 = 0.80;
    /// A warning's row in the Console (of the warning colour).
    pub const WARNING_ROW: f32 = 0.06;
    /// An error's row in the Console (of the error colour).
    pub const ERROR_ROW: f32 = 0.08;
    /// How much lighter a filled button is under the pointer (of
    /// white). Solis's own.
    pub const LIFT: f32 = 0.12;
}

/// An error's row in the Console: the error colour mixed faintly into
/// the column.
pub fn error_row() -> Color32 {
    mix(ERROR, panel(), share::ERROR_ROW)
}

/// A warning's row in the Console.
pub fn warning_row() -> Color32 {
    mix(WARNING, panel(), share::WARNING_ROW)
}

/// A selected row of a tree or a list: the accent mixed into a shelf
/// column.
pub fn select_row(accent: Accent) -> Color32 {
    mix(accent.colour(), panel(), share::SELECTED_ROW)
}

/// A selected tile, table row or card: a lighter touch of the accent, on
/// which field labels still read.
pub fn select_item(accent: Accent) -> Color32 {
    mix(accent.colour(), panel(), share::SELECTED)
}

/// A line across, one whole pixel thick at any interface scale, with its
/// top edge on the pixel edge nearest `y`.
pub fn hline(painter: &egui::Painter, x: egui::Rangef, y: f32, colour: Color32) {
    let scale = painter.ctx().pixels_per_point();
    let top = on_pixel(y, scale);
    let rect = egui::Rect::from_x_y_ranges(
        on_pixel(x.min, scale)..=on_pixel(x.max, scale),
        top..=top + hairline(scale),
    );
    painter.rect_filled(rect, 0.0, colour);
}

/// A line down, one whole pixel thick at any interface scale, with its
/// left edge on the pixel edge nearest `x`.
pub fn vline(painter: &egui::Painter, x: f32, y: egui::Rangef, colour: Color32) {
    let scale = painter.ctx().pixels_per_point();
    let left = on_pixel(x, scale);
    let rect = egui::Rect::from_x_y_ranges(
        left..=left + hairline(scale),
        on_pixel(y.min, scale)..=on_pixel(y.max, scale),
    );
    painter.rect_filled(rect, 0.0, colour);
}

/// The active large ribbon button, or a selected header on the ribbon's
/// grey.
pub fn active_button(accent: Accent) -> Color32 {
    mix(accent.colour(), ribbon(), share::ACTIVE_BUTTON)
}

/// A keycap waiting for a key: the accent mixed into a panel.
pub fn listening(accent: Accent) -> Color32 {
    mix(accent.colour(), panel(), share::LISTENING)
}

/// The "changed from defaults" pill; a notice on a File page.
pub fn changes_pill(accent: Accent) -> Color32 {
    mix(accent.colour(), shelf_bar(), share::CHANGES)
}

/// The border of the open project's card, and of a selected card.
pub fn open_card_border(accent: Accent) -> Color32 {
    mix(accent.colour(), card_border(), share::OPEN_CARD)
}

/// A line one whole pixel wide, in points: sharp at every interface
/// scale. At 125% and 150% a line of one point would be a pixel and a
/// bit, and blur; the pixels are counted down to a whole number, as a
/// browser does with a border, so the line is one pixel until 200%.
pub fn hairline(pixels_per_point: f32) -> f32 {
    if !pixels_per_point.is_finite() || pixels_per_point <= 0.0 {
        return 1.0;
    }
    pixels_per_point.floor().max(1.0) / pixels_per_point
}

/// `at` (a position in points) moved onto the nearest pixel edge, so a
/// hairline starting there covers whole pixels and no row or column of
/// pixels is half lit.
pub fn on_pixel(at: f32, pixels_per_point: f32) -> f32 {
    if !pixels_per_point.is_finite() || pixels_per_point <= 0.0 {
        return at;
    }
    (at * pixels_per_point).round() / pixels_per_point
}

/// Every colour of the design with its name: the neutrals, the colours
/// that mean something and the five accents. Drawn as swatches by the
/// picture tests and checked against the specification's tables.
pub fn sheet() -> Vec<(&'static str, Color32)> {
    let mut all = neutrals();
    all.extend(meanings());
    all.extend(Accent::ALL.map(|accent| (accent.name(), accent.colour())));
    all
}

#[cfg(test)]
#[path = "tokens_tests.rs"]
mod tests;
