//! The editor's two themes and the rounded-panels switch
//! ([decision 0096](../../../docs/decisions/0096-graphite-theme-and-rounded-panels.md)).
//!
//! A theme is a [`Palette`] of neutral colours: the panels, lines, fields
//! and text. Colours that mean something (the accent, Play, warnings, the
//! kinds of things) stay the same in every theme. The editor draws on one
//! thread, so the chosen look is kept there and read by the colour
//! functions ([`panel`], [`text_weak`] …), which `theme` re-exports.

use egui::Color32;
use serde::{Deserialize, Serialize};
use std::cell::Cell;

/// The editor's colours, chosen in Edit > Settings… > Appearance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Theme {
    /// Dark neutral greys with thin dark lines: the look Solis started with.
    #[default]
    Standard,
    /// Near-black panels with lighter hairlines and crisp white text.
    Graphite,
}

impl Theme {
    /// Every theme, in the order the menu lists them.
    pub const ALL: [Self; 2] = [Self::Standard, Self::Graphite];

    /// The theme's name in the menu.
    pub fn name(self) -> &'static str {
        match self {
            Self::Standard => "Standard",
            Self::Graphite => "Graphite",
        }
    }

    /// The theme's colours.
    pub fn palette(self) -> &'static Palette {
        match self {
            Self::Standard => &STANDARD,
            Self::Graphite => &GRAPHITE,
        }
    }
}

/// How the editor looks: its theme, and whether panels are rounded cards
/// with gaps between them (a separate switch, so each can be tried alone).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Appearance {
    /// The colours.
    pub theme: Theme,
    /// Panels as rounded cards, and the workspaces as one group of pills.
    pub rounded: bool,
}

/// A theme's neutral colours.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    /// The window behind the panels: the lines or gaps between them.
    pub background: Color32,
    /// Panel bodies.
    pub panel: Color32,
    /// Title bars above the panels, the top bar and the status bar.
    pub header: Color32,
    /// The small menu row under a panel's title.
    pub menu_row: Color32,
    /// Text and number fields.
    pub field: Color32,
    /// Hairlines between areas and around fields.
    pub line: Color32,
    /// The thin border around buttons.
    pub button_border: Color32,
    /// A hovered button.
    pub hover: Color32,
    /// A hovered button's border.
    pub hover_border: Color32,
    /// A button being pressed.
    pub pressed: Color32,
    /// The thin border around a card on the launcher's pages.
    pub card_border: Color32,
    /// The chosen row or page on the launcher: the panel with a cool tint.
    pub selected: Color32,
    /// The border round the chosen row and round a notice.
    pub selected_border: Color32,
    /// A notice on the launcher (what was just done, a newer build): a
    /// quieter tint than the chosen row's.
    pub notice: Color32,
    /// Normal text.
    pub text: Color32,
    /// Bright text: headings and the front tab.
    pub text_strong: Color32,
    /// Quieter text: labels and hints.
    pub text_weak: Color32,
    /// Console text.
    pub text_console: Color32,
    /// Times in the console.
    pub text_time: Color32,
    /// The panels drawn over the viewport, a little see-through.
    pub overlay: Color32,
    /// The viewport's sky straight up.
    pub sky_top: Color32,
    /// The viewport's sky at the horizon.
    pub horizon: Color32,
    /// The viewport's ground just below the horizon.
    pub ground: Color32,
}

const fn grey(v: u8) -> Color32 {
    Color32::from_rgb(v, v, v)
}

/// The look Solis started with (decision 0004).
pub(crate) const STANDARD: Palette = Palette {
    background: grey(0x1b),
    panel: grey(0x26),
    header: grey(0x22),
    menu_row: grey(0x2a),
    field: grey(0x1c),
    line: grey(0x16),
    button_border: grey(0x3a),
    hover: grey(0x3a),
    hover_border: grey(0x4a),
    pressed: grey(0x44),
    card_border: grey(0x30),
    selected: Color32::from_rgb(0x2c, 0x3a, 0x4d),
    selected_border: Color32::from_rgb(0x3f, 0x5a, 0x7a),
    notice: Color32::from_rgb(0x23, 0x30, 0x3f),
    text: grey(0xc8),
    text_strong: grey(0xf0),
    text_weak: grey(0xa5),
    text_console: grey(0xd4),
    text_time: grey(0x9a),
    overlay: Color32::from_rgba_premultiplied(0x1f, 0x1f, 0x1f, 0xeb),
    sky_top: Color32::from_rgb(0x16, 0x18, 0x1d),
    horizon: Color32::from_rgb(0x1c, 0x1e, 0x23),
    ground: Color32::from_rgb(0x1e, 0x21, 0x26),
};

/// The project lead's near-black palette (decision 0096). A card's border
/// is its hairline, and the launcher's tints are the standard ones laid on
/// its darker panel: each as far from the panel as it is in Standard.
pub(crate) const GRAPHITE: Palette = Palette {
    background: grey(0x09),
    panel: grey(0x14),
    header: grey(0x0e),
    menu_row: grey(0x18),
    field: grey(0x1c),
    line: grey(0x26),
    button_border: grey(0x30),
    hover: grey(0x2a),
    hover_border: grey(0x3a),
    pressed: grey(0x33),
    card_border: grey(0x26),
    selected: Color32::from_rgb(0x1a, 0x28, 0x3b),
    selected_border: Color32::from_rgb(0x2d, 0x48, 0x68),
    notice: Color32::from_rgb(0x11, 0x1e, 0x2d),
    text: grey(0xd6),
    text_strong: grey(0xfa),
    text_weak: grey(0x8e),
    text_console: grey(0xdc),
    text_time: grey(0x8e),
    overlay: Color32::from_rgba_premultiplied(0x16, 0x16, 0x16, 0xeb),
    sky_top: Color32::from_rgb(0x0c, 0x0c, 0x0d),
    horizon: Color32::from_rgb(0x12, 0x12, 0x14),
    ground: Color32::from_rgb(0x15, 0x15, 0x17),
};

thread_local! {
    static CHOSEN: Cell<Appearance> = const { Cell::new(Appearance { theme: Theme::Standard, rounded: false }) };
}

/// The look in use on this thread.
pub fn appearance() -> Appearance {
    CHOSEN.with(Cell::get)
}

/// Uses `appearance` for everything drawn on this thread from now on.
pub(crate) fn use_appearance(appearance: Appearance) {
    CHOSEN.with(|c| c.set(appearance));
}

/// The colours in use.
pub fn palette() -> &'static Palette {
    appearance().theme.palette()
}

/// Whether panels are rounded cards.
pub fn rounded() -> bool {
    appearance().rounded
}

macro_rules! colour {
    ($($name:ident),* $(,)?) => {$(
        #[doc = concat!("The theme's `", stringify!($name), "` colour (see [`Palette`]).")]
        pub fn $name() -> Color32 {
            palette().$name
        }
    )*};
}

colour!(
    background,
    panel,
    header,
    menu_row,
    field,
    line,
    button_border,
    hover,
    hover_border,
    pressed,
    card_border,
    selected,
    selected_border,
    notice,
    text,
    text_strong,
    text_weak,
    text_console,
    text_time,
    overlay,
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_is_the_default_and_both_are_listed() {
        assert_eq!(Appearance::default().theme, Theme::Standard);
        assert!(!Appearance::default().rounded);
        assert_eq!(Theme::ALL, [Theme::Standard, Theme::Graphite]);
        assert_ne!(Theme::Standard.name(), Theme::Graphite.name());
    }

    #[test]
    fn choosing_a_look_changes_the_colours_on_this_thread_only() {
        use_appearance(Appearance {
            theme: Theme::Graphite,
            rounded: true,
        });
        assert_eq!(panel(), GRAPHITE.panel);
        assert!(rounded());
        std::thread::spawn(|| assert_eq!(panel(), STANDARD.panel))
            .join()
            .unwrap();
        use_appearance(Appearance::default());
        assert_eq!(panel(), STANDARD.panel);
    }

    #[test]
    fn a_saved_look_reads_back_and_damaged_ones_fall_back() {
        for appearance in [
            Appearance::default(),
            Appearance {
                theme: Theme::Graphite,
                rounded: true,
            },
        ] {
            let text = ron::to_string(&appearance).unwrap();
            assert_eq!(ron::from_str::<Appearance>(&text).unwrap(), appearance);
        }
        // Missing fields take their defaults; an unknown theme is an error
        // the app answers with the default.
        assert_eq!(
            ron::from_str::<Appearance>("(rounded: true)").unwrap(),
            Appearance {
                theme: Theme::Standard,
                rounded: true
            }
        );
        assert!(ron::from_str::<Appearance>("(theme: Neon)").is_err());
    }

    #[test]
    fn damaged_saved_looks_never_crash() {
        // A sweep of damaged settings: each reads or fails, never panics.
        let good = ron::to_string(&Appearance {
            theme: Theme::Graphite,
            rounded: true,
        })
        .unwrap();
        let mut seed = 0x2545_f491_u32;
        for _ in 0..4000 {
            let mut bytes = good.clone().into_bytes();
            for _ in 0..3 {
                seed ^= seed << 13;
                seed ^= seed >> 17;
                seed ^= seed << 5;
                let at = seed as usize % bytes.len();
                match seed % 3 {
                    0 => bytes[at] = (seed >> 8) as u8,
                    1 => bytes.truncate(at),
                    _ => bytes.insert(at, b'('),
                }
                if bytes.is_empty() {
                    break;
                }
            }
            let text = String::from_utf8_lossy(&bytes);
            let _ = ron::from_str::<Appearance>(&text);
        }
    }
}
