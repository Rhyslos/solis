//! The editor's fonts: Inter for the interface and JetBrains Mono for
//! numbers, paths and keys, each in the weights the design uses (400,
//! 500, 600 and 700; 400 and 700). Both are under the SIL Open Font
//! License 1.1 (`licenses/fonts`).
//!
//! egui has no notion of weight: each weight is a font file under a family
//! name (`tokens::text::MEDIUM` and the others). [`WEIGHTS`] is the one
//! place that says which file each family draws with, and which bundled
//! weight stands in while a file isn't in `assets/fonts/` yet.

use crate::tokens::text::{BOLD, MEDIUM, MONO_BOLD, SEMIBOLD};
use egui::{FontData, FontDefinitions, FontFamily};
use std::sync::Arc;

/// Inter, the interface font. Also game UI's standard fonts (decision
/// 0084): font 0 and 1.
pub const INTER_REGULAR: &[u8] = include_bytes!("../assets/fonts/Inter-Regular.ttf");
/// Inter's semibold, game UI's font 1.
pub const INTER_SEMIBOLD: &[u8] = include_bytes!("../assets/fonts/Inter-SemiBold.ttf");
const JETBRAINS_MONO: &[u8] = include_bytes!("../assets/fonts/JetBrainsMonoNL-Regular.ttf");
const JETBRAINS_MONO_BOLD: &[u8] = include_bytes!("../assets/fonts/JetBrainsMonoNL-Bold.ttf");
/// Inter Medium: not bundled yet. When `Inter-Medium.ttf` is added to
/// `assets/fonts/`, this becomes `Some(include_bytes!(…))` and nothing
/// else changes.
const INTER_MEDIUM: Option<&[u8]> = None;
/// Inter Bold: not bundled yet, as above (`Inter-Bold.ttf`).
const INTER_BOLD: Option<&[u8]> = None;

/// The font the regular interface text is drawn with.
const REGULAR_FONT: &str = "inter";
/// The font monospace text is drawn with.
const MONO_FONT: &str = "jetbrains-mono";

/// One weight beyond the regular ones.
struct Weight {
    /// Its family's name, which type styles ask for.
    family: &'static str,
    /// Its font's name inside egui.
    font: &'static str,
    /// Its file, if bundled.
    file: Option<&'static [u8]>,
    /// The bundled font it is drawn with otherwise: the nearest lighter
    /// weight, so nothing looks heavier than designed.
    stand_in: &'static str,
    /// Whether it is a monospace weight.
    mono: bool,
}

/// Every weight the design uses beyond Inter Regular and JetBrains Mono
/// Regular.
const WEIGHTS: [Weight; 4] = [
    Weight {
        family: MEDIUM,
        font: "inter-medium",
        file: INTER_MEDIUM,
        stand_in: REGULAR_FONT,
        mono: false,
    },
    Weight {
        family: SEMIBOLD,
        font: "inter-semibold",
        file: Some(INTER_SEMIBOLD),
        stand_in: REGULAR_FONT,
        mono: false,
    },
    Weight {
        family: BOLD,
        font: "inter-bold",
        file: INTER_BOLD,
        stand_in: "inter-semibold",
        mono: false,
    },
    Weight {
        family: MONO_BOLD,
        font: "jetbrains-mono-bold",
        file: Some(JETBRAINS_MONO_BOLD),
        stand_in: MONO_FONT,
        mono: true,
    },
];

/// The families whose own font file isn't bundled yet, each with the
/// font it is drawn with meanwhile. A test lists them, so adding a file
/// is noticed.
#[cfg(test)]
fn standing_in() -> Vec<(&'static str, &'static str)> {
    WEIGHTS
        .iter()
        .filter(|weight| weight.file.is_none())
        .map(|weight| (weight.family, weight.stand_in))
        .collect()
}

/// Glyphs drawn as a browser draws them, without snapping their outlines
/// to the pixel grid: softer, and closer to the design's pictures than
/// egui's default.
fn soft() -> egui::FontTweak {
    egui::FontTweak {
        hinting: Some(false),
        ..Default::default()
    }
}

/// Inter for interface text and JetBrains Mono for monospace text, in
/// front of egui's own fonts (which stay as fallbacks for symbols and
/// other scripts), and a family for each further weight.
///
/// Every symbol the editor writes must be in one of these fonts, or it
/// shows as an empty box: the `symbols_have_glyphs` test checks.
pub fn fonts() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        REGULAR_FONT.into(),
        Arc::new(FontData::from_static(INTER_REGULAR).tweak(soft())),
    );
    fonts.font_data.insert(
        MONO_FONT.into(),
        Arc::new(FontData::from_static(JETBRAINS_MONO)),
    );
    let proportional = fonts.families.entry(FontFamily::Proportional).or_default();
    proportional.insert(0, REGULAR_FONT.into());
    // Inter lacks a few symbols the editor uses (≡ ▾ ▴); egui's monospace
    // font has them.
    proportional.push("Hack".into());
    let after_inter: Vec<String> = proportional.iter().skip(1).cloned().collect();
    // Monospace text is JetBrains Mono, with egui's Hack and then Inter
    // for any symbol it lacks.
    let monospace = fonts.families.entry(FontFamily::Monospace).or_default();
    monospace.insert(0, MONO_FONT.into());
    monospace.insert(2.min(monospace.len()), REGULAR_FONT.into());
    let after_mono: Vec<String> = monospace.iter().skip(1).cloned().collect();
    for weight in &WEIGHTS {
        let first = match weight.file {
            Some(file) => {
                let data = FontData::from_static(file);
                let data = if weight.mono {
                    data
                } else {
                    data.tweak(soft())
                };
                fonts.font_data.insert(weight.font.into(), Arc::new(data));
                weight.font
            }
            None => weight.stand_in,
        };
        let rest = if weight.mono {
            &after_mono
        } else {
            &after_inter
        };
        let mut family = vec![first.to_string()];
        family.extend(rest.iter().filter(|font| *font != first).cloned());
        fonts
            .families
            .insert(FontFamily::Name(weight.family.into()), family);
    }
    fonts
}

#[cfg(test)]
#[path = "fonts_tests.rs"]
mod tests;
