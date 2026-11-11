//! How the editor's interface looks, as the user chose it: the accent
//! colour, the interface scale, the density, and two switches the new
//! frame reads as it is built
//! ([decision 0100](../../../docs/decisions/0100-one-theme-five-accents.md)).
//! Not to be confused with [`crate::Look`], how models are shaded in the
//! viewport.
//!
//! Saved as one value under one settings key. Every field has a standard
//! value, so a file from a later build with more fields, or an earlier
//! one with fewer, still reads. An accent chosen before the design had
//! five is mapped once ([`InterfaceLook::from_before`]).
//!
//! The editor draws on one thread, so the look in use is kept there
//! ([`in_use`]): sizes that follow the density and the launcher's tints
//! read it without being handed it.

use crate::tokens::size::{Density, Scale};
use crate::tokens::{self, Accent};
use egui::Color32;
use serde::{Deserialize, Serialize};
use std::cell::Cell;

/// The editor's look, chosen in the settings and saved between runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct InterfaceLook {
    /// The accent colour.
    pub accent: Accent,
    /// How large everything is drawn.
    pub scale: Scale,
    /// How tall rows, fields and shelf tabs are.
    pub density: Density,
    /// Whether the ribbon's large buttons show their names. Read by the
    /// ribbon when it is built; saved from now so the key never changes.
    pub ribbon_labels: bool,
    /// Whether parts that aren't built yet are hidden
    /// ([decision 0101](../../../docs/decisions/0101-unfinished-parts-as-placeholders.md)).
    /// Read by the placeholders when they are built; saved from now.
    pub hide_unfinished: bool,
}

impl Default for InterfaceLook {
    fn default() -> Self {
        Self {
            accent: Accent::default(),
            scale: Scale::default(),
            density: Density::default(),
            ribbon_labels: true,
            hide_unfinished: false,
        }
    }
}

/// An accent as builds before the design's five saved one. Kept only to
/// read those settings: "Green" was shown as Marine green.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum AccentBefore {
    /// Petrol blue, the standard at first.
    PetrolBlue,
    /// A soft red.
    Red,
    /// A saturated blue, the standard at the end.
    Blue,
    /// Marine green.
    Green,
    /// A purple-pink.
    Plum,
    /// A yellow green.
    Olive,
    /// A red-orange.
    Copper,
    /// A sunset orange.
    Orange,
    /// An orange-yellow, the standard for a while.
    Amber,
    /// A deep yellow.
    Gold,
}

impl AccentBefore {
    /// Every earlier accent.
    pub const ALL: [Self; 10] = [
        Self::PetrolBlue,
        Self::Red,
        Self::Blue,
        Self::Green,
        Self::Plum,
        Self::Olive,
        Self::Copper,
        Self::Orange,
        Self::Amber,
        Self::Gold,
    ];

    /// The accent it becomes: the nearest of the five by hue. Nothing
    /// becomes Grey, which is new.
    pub fn mapped(self) -> Accent {
        match self {
            Self::Blue => Accent::Blue,
            Self::PetrolBlue | Self::Green | Self::Olive => Accent::Teal,
            Self::Red | Self::Copper | Self::Orange | Self::Amber | Self::Gold => Accent::Orange,
            Self::Plum => Accent::Purple,
        }
    }
}

impl InterfaceLook {
    /// The look for the first start after the design's five accents
    /// arrived, from what an earlier build saved: `chosen` is the accent
    /// saved since blue became the standard, `oldest` the one saved before
    /// that, when amber was the standard (so amber there means "never
    /// chose", and becomes blue). Everything else starts as standard.
    pub fn from_before(chosen: Option<AccentBefore>, oldest: Option<AccentBefore>) -> Self {
        let accent = match (chosen, oldest) {
            (Some(chosen), _) => chosen.mapped(),
            (None, Some(AccentBefore::Amber)) | (None, None) => Accent::default(),
            (None, Some(oldest)) => oldest.mapped(),
        };
        Self {
            accent,
            ..Self::default()
        }
    }
}

thread_local! {
    static IN_USE: Cell<Option<InterfaceLook>> = const { Cell::new(None) };
}

/// The look in use on this thread (the standard one until one is chosen).
pub fn in_use() -> InterfaceLook {
    IN_USE.with(Cell::get).unwrap_or_default()
}

/// Uses `look` for everything drawn on this thread from now on.
pub(crate) fn use_look(look: InterfaceLook) {
    IN_USE.with(|cell| cell.set(Some(look)));
}

/// The solid accent colour in use, for the few places that mark something
/// with it and aren't handed the accent.
pub fn accent_fill() -> Color32 {
    in_use().accent.colour()
}

/// The chosen row or page on the launcher: the accent in use mixed into a
/// panel, lightly enough that the row's quieter words still read.
pub fn selected() -> Color32 {
    tokens::select_item(in_use().accent)
}

/// The border round the chosen row and round a notice.
pub fn selected_border() -> Color32 {
    tokens::open_card_border(in_use().accent)
}

/// A notice (what was just done, a newer build): a quieter tint than the
/// chosen row's.
pub fn notice() -> Color32 {
    tokens::changes_pill(in_use().accent)
}

#[cfg(test)]
#[path = "interface_look_tests.rs"]
mod tests;
