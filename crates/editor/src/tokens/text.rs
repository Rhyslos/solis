//! The design's type styles (specification, lines 83 to 96, with the
//! mock-up's sizes where it leaves one out): Inter for the interface and
//! JetBrains Mono for numbers, paths and keys.
//!
//! egui has no notion of weight: each weight is a font file under a family
//! name, so a style is a size and one of the families below. The families
//! are filled in by `fonts.rs`, which also says what a weight falls back
//! to while its file isn't bundled.

use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontFamily, FontId};

/// The family of Inter Medium (weight 500): tabs in front, setting names.
pub const MEDIUM: &str = "medium";
/// The family of Inter SemiBold (weight 600): headers, card titles.
pub const SEMIBOLD: &str = "semibold";
/// The family of Inter Bold (weight 700): a File page's title.
pub const BOLD: &str = "bold";
/// The family of JetBrains Mono Bold (weight 700): the axis letters in
/// number fields.
pub const MONO_BOLD: &str = "mono-bold";

/// The size of interface text.
pub const UI_SIZE: f32 = 12.0;
/// The size of rows, labels, headers and document tabs.
pub const ROW_SIZE: f32 = 11.5;
/// The size of small text: small ribbon rows, pills, tile names.
pub const SMALL_SIZE: f32 = 11.0;
/// The size of numbers in fields, log lines and timecodes.
pub const MONO_SIZE: f32 = 11.0;
/// The size of the labels under large ribbon buttons and of group labels.
pub const RIBBON_LABEL_SIZE: f32 = 10.0;
/// The size of a details title, and of headings egui draws.
pub const HEADING_SIZE: f32 = 14.0;

/// Inter Regular at `size`.
fn regular(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}

/// A named family at `size`.
fn named(size: f32, family: &str) -> FontId {
    FontId::new(size, FontFamily::Name(family.into()))
}

/// Inter Medium (500) at `size`: a chip's or a segment's word.
pub fn medium_at(size: f32) -> FontId {
    named(size, MEDIUM)
}

/// Inter SemiBold (600) at `size`: a main button's word.
pub fn semibold_at(size: f32) -> FontId {
    named(size, SEMIBOLD)
}

/// Interface text: 12 / 400.
pub fn ui() -> FontId {
    regular(UI_SIZE)
}

/// A ribbon or shelf tab that isn't in front: 12 / 400.
pub fn tab() -> FontId {
    regular(UI_SIZE)
}

/// The ribbon or shelf tab in front: 12 / 500.
pub fn tab_active() -> FontId {
    named(UI_SIZE, MEDIUM)
}

/// The label under a large ribbon button: 10 / 400.
pub fn ribbon_label() -> FontId {
    regular(RIBBON_LABEL_SIZE)
}

/// A small ribbon row: 11 / 400.
pub fn ribbon_small() -> FontId {
    regular(SMALL_SIZE)
}

/// A column or panel header, a card's title: 11.5 / 600.
pub fn header() -> FontId {
    named(ROW_SIZE, SEMIBOLD)
}

/// A row, a field's label, a document tab: 11.5 / 400.
pub fn row() -> FontId {
    regular(ROW_SIZE)
}

/// Numbers in fields, log lines, timecodes: mono 11 / 400.
pub fn mono() -> FontId {
    FontId::new(MONO_SIZE, FontFamily::Monospace)
}

/// Paths, keycaps, shortcut hints: mono 10.5 / 400.
pub fn mono_small() -> FontId {
    FontId::new(10.5, FontFamily::Monospace)
}

/// The axis letters in number fields: mono 10 / 700.
pub fn mono_bold() -> FontId {
    named(10.0, MONO_BOLD)
}

/// Code: mono 11.5 / 400, on lines 19 points apart ([`CODE_LINE_H`]).
pub fn code() -> FontId {
    FontId::new(ROW_SIZE, FontFamily::Monospace)
}

/// The distance between lines of code.
pub const CODE_LINE_H: f32 = 19.0;

/// A File page's title: 22 / 700.
pub fn page_title() -> FontId {
    named(22.0, BOLD)
}

/// The line under a File page's title: 12.5 / 400.
pub fn page_subtitle() -> FontId {
    regular(12.5)
}

/// A setting's name: 12.5 / 500.
pub fn setting_name() -> FontId {
    named(12.5, MEDIUM)
}

/// The line under a setting's name: 11.5 / 400.
pub fn setting_hint() -> FontId {
    regular(ROW_SIZE)
}

/// The distance between the lines under a setting's name: 1.45 of their
/// size.
pub const SETTING_HINT_LINE_H: f32 = ROW_SIZE * 1.45;

/// An item of the File page's menu: 13 / 500.
pub fn menu_item() -> FontId {
    named(13.0, MEDIUM)
}

/// A project card's name: 13 / 600.
pub fn card_name() -> FontId {
    named(13.0, SEMIBOLD)
}

/// A details title, and the headings egui draws: 14 / 600.
pub fn heading() -> FontId {
    named(HEADING_SIZE, SEMIBOLD)
}

/// A tile's name: 11 / 400.
pub fn tile_name() -> FontId {
    regular(SMALL_SIZE)
}

/// A tile's kind, a small badge: 10.5 / 400.
pub fn tile_kind() -> FontId {
    regular(10.5)
}

/// Capital letters set a little apart: how wide the gaps are, as a share
/// of the text's size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Caps {
    /// The font.
    pub size: f32,
    /// The family's name.
    pub family: &'static str,
    /// The room added between letters, as a share of the size.
    pub spacing: f32,
    /// Whether the words are written in capitals.
    pub upper: bool,
}

/// A ribbon group's label: 10 / 400, letters 0.04 of the size apart.
pub const GROUP_LABEL: Caps = Caps {
    size: RIBBON_LABEL_SIZE,
    family: "",
    spacing: 0.04,
    upper: false,
};

/// A settings section, a table's header: 10.5 / 600 in capitals, letters
/// 0.08 of the size apart.
pub const SECTION_CAPS: Caps = Caps {
    size: 10.5,
    family: SEMIBOLD,
    spacing: 0.08,
    upper: true,
};

/// The small words before a contextual tab's name: 9.5 / 600 in capitals,
/// letters 0.06 of the size apart.
pub const CONTEXT_PREFIX: Caps = Caps {
    size: 9.5,
    family: SEMIBOLD,
    spacing: 0.06,
    upper: true,
};

/// `words` laid out in a spaced style, in `colour`. egui can space
/// letters but not capitalise them, so the capitals are made here.
pub fn caps(words: &str, style: Caps, colour: Color32) -> LayoutJob {
    let font_id = if style.family.is_empty() {
        regular(style.size)
    } else {
        named(style.size, style.family)
    };
    let text = if style.upper {
        words.to_uppercase()
    } else {
        words.to_string()
    };
    let format = TextFormat {
        font_id,
        color: colour,
        extra_letter_spacing: style.size * style.spacing,
        ..TextFormat::default()
    };
    LayoutJob::single_section(text, format)
}

/// Every type style with its name, for tests that walk them all.
pub fn all() -> Vec<(&'static str, FontId)> {
    vec![
        ("ui", ui()),
        ("tab", tab()),
        ("tab_active", tab_active()),
        ("ribbon_label", ribbon_label()),
        ("ribbon_small", ribbon_small()),
        ("header", header()),
        ("row", row()),
        ("mono", mono()),
        ("mono_small", mono_small()),
        ("mono_bold", mono_bold()),
        ("code", code()),
        ("page_title", page_title()),
        ("page_subtitle", page_subtitle()),
        ("setting_name", setting_name()),
        ("setting_hint", setting_hint()),
        ("menu_item", menu_item()),
        ("card_name", card_name()),
        ("heading", heading()),
        ("tile_name", tile_name()),
        ("tile_kind", tile_kind()),
    ]
}
