//! A project's 2D settings (Phase 9, D1): the size its 2D games are drawn
//! at before they're scaled up to the screen, how many of those pixels
//! make a metre, the tile size, and how the picture fills the screen.
//! Kept in the project's `pixels.solpix`, a settings file in the same
//! Lust style as areas, so it travels with the project and Git merges it.
//!
//! ```text
//! -- The project's 2D settings. Safe to edit by hand.
//! pixels version 1;
//!
//! base = [480, 270];
//! pixels_per_unit = 16;
//! tile = [16, 16];
//! scaling = "whole steps";
//! ```
//!
//! A value out of range is brought into range with a note; a setting it
//! doesn't know is skipped with a note; only a file that can't be read at
//! all (or one from a newer version) is refused, with its line.

use crate::settings;
use reflect::Value;
use std::fmt;

/// The 2D settings file's name, in the project's folder.
pub const FILE_NAME: &str = "pixels.solpix";
/// The layout version this build writes and reads.
pub const VERSION: u32 = 1;
/// The word the file starts with.
const KIND: &str = "pixels";
/// The biggest 2D settings file read.
pub const MAX_BYTES: usize = 16 << 10;

/// This file's version and upgraders (Phase 8, C1): each step turns
/// fields read at one version into the next, before they're applied.
/// Every version ever released keeps an example file in `tests/fixtures/`.
pub static FORMAT: reflect::upgrade::Format<settings::Settings> = reflect::upgrade::Format {
    name: "pixels",
    current: VERSION,
    version_of: |text| reflect::upgrade::header_version(text, &[KIND]),
    steps: &[],
};

/// The smallest and biggest base resolution, each way, in pixels.
pub const BASE_RANGE: (u32, u32) = (16, 3840);
/// The fewest and most pixels to a unit (a metre).
pub const PIXELS_PER_UNIT_RANGE: (u32, u32) = (1, 1024);
/// The smallest and biggest tile, each way, in pixels.
pub const TILE_RANGE: (u32, u32) = (1, 1024);

/// How the small picture fills the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Scaling {
    /// Every pixel scaled the same whole number of times, with bars at the
    /// edges when the screen doesn't divide evenly: always crisp.
    #[default]
    WholeSteps,
    /// Scaled up in whole steps as far as it goes, then the last bit
    /// smoothed so the picture fills the screen: no bars, edges still
    /// sharp.
    Fill,
}

impl Scaling {
    /// Both, in the order menus show them.
    pub const ALL: [Self; 2] = [Self::WholeSteps, Self::Fill];

    /// Its name, as the file and the Settings window write it.
    pub fn name(self) -> &'static str {
        match self {
            Self::WholeSteps => "whole steps",
            Self::Fill => "fill",
        }
    }

    /// Its name for a menu.
    pub fn label(self) -> &'static str {
        match self {
            Self::WholeSteps => "Whole steps",
            Self::Fill => "Fill the screen",
        }
    }

    fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.name() == name)
    }
}

/// A project's 2D settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PixelSettings {
    /// The size the game is drawn at, in pixels, before it's scaled up.
    pub base: [u32; 2],
    /// How many pixels make one unit (one metre) of the world.
    pub pixels_per_unit: u32,
    /// The tile size, in pixels: the 2D view's grid and new tile maps.
    pub tile: [u32; 2],
    /// How the picture fills the screen.
    pub scaling: Scaling,
}

impl Default for PixelSettings {
    /// 480 × 270 with 16-pixel tiles (the project lead's choice,
    /// 2026-10-28): it scales evenly to 1080p (×4) and 4K (×8).
    fn default() -> Self {
        Self {
            base: [480, 270],
            pixels_per_unit: 16,
            tile: [16, 16],
            scaling: Scaling::WholeSteps,
        }
    }
}

impl PixelSettings {
    /// The same settings with every number in range.
    #[must_use]
    pub fn sanitized(self) -> Self {
        let fit = |v: u32, (lo, hi): (u32, u32)| v.clamp(lo, hi);
        Self {
            base: self.base.map(|v| fit(v, BASE_RANGE)),
            pixels_per_unit: fit(self.pixels_per_unit, PIXELS_PER_UNIT_RANGE),
            tile: self.tile.map(|v| fit(v, TILE_RANGE)),
            scaling: self.scaling,
        }
    }

    /// One pixel's size in the world, in metres.
    pub fn pixel_size(&self) -> f32 {
        1.0 / self.pixels_per_unit.max(1) as f32
    }

    /// A tile's size in the world, in metres, across and up.
    pub fn tile_size(&self) -> [f32; 2] {
        self.tile.map(|t| t as f32 * self.pixel_size())
    }
}

/// Why a 2D settings file couldn't be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PixelsError {
    /// The line it went wrong on (from 1), or 0 for the whole file.
    pub line: usize,
    /// What went wrong, in plain words.
    pub message: String,
}

impl fmt::Display for PixelsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line == 0 {
            write!(f, "{}", self.message)
        } else {
            write!(f, "line {}: {}", self.line, self.message)
        }
    }
}

impl std::error::Error for PixelsError {}

/// A 2D settings file as read: the settings, and what was put right.
#[derive(Debug, Clone, PartialEq)]
pub struct ReadPixels {
    /// The settings.
    pub settings: PixelSettings,
    /// What was put right or skipped, each starting with its line
    /// ("line 4: …").
    pub notes: Vec<String>,
}

/// Reads a 2D settings file. Read as a hostile file: within
/// [`MAX_BYTES`], refusing a newer layout or anything malformed with its
/// line; a value out of range or of the wrong kind is put right with a
/// note.
pub fn read(text: &str) -> Result<ReadPixels, PixelsError> {
    if text.len() > MAX_BYTES {
        return Err(PixelsError {
            line: 0,
            message: format!(
                "the file is {} KB; 2D settings up to {} KB are read",
                text.len() >> 10,
                MAX_BYTES >> 10
            ),
        });
    }
    let mut file = settings::read_settings(text, KIND, VERSION).map_err(|e| PixelsError {
        line: e.line,
        message: e.message,
    })?;
    let version = file.version;
    FORMAT
        .upgrade(version, &mut file)
        .map_err(|e| PixelsError {
            line: 1,
            message: e.to_string(),
        })?;
    let d = PixelSettings::default();
    let mut s = d;
    let mut notes = Vec::new();
    for field in &file.fields {
        let line = field.line;
        match field.key.as_str() {
            "base" => {
                s.base = pair(&field.value, d.base, BASE_RANGE, line, "base", &mut notes);
            }
            "pixels_per_unit" => {
                s.pixels_per_unit = whole(
                    &field.value,
                    d.pixels_per_unit,
                    PIXELS_PER_UNIT_RANGE,
                    line,
                    "pixels_per_unit",
                    &mut notes,
                );
            }
            "tile" => {
                s.tile = pair(&field.value, d.tile, TILE_RANGE, line, "tile", &mut notes);
            }
            "scaling" => {
                s.scaling = match &field.value {
                    Value::Text(name) => Scaling::from_name(name).unwrap_or_else(|| {
                        notes.push(format!(
                            "line {line}: scaling \"{name}\" isn't \"whole steps\" or \"fill\", \
                             so it's \"whole steps\""
                        ));
                        d.scaling
                    }),
                    _ => {
                        notes.push(format!(
                            "line {line}: scaling should be \"whole steps\" or \"fill\", so it's \
                             \"whole steps\""
                        ));
                        d.scaling
                    }
                };
            }
            other => notes.push(format!(
                "line {line}: \"{other}\" isn't a 2D setting, so it's skipped"
            )),
        }
    }
    Ok(ReadPixels { settings: s, notes })
}

/// One whole number in `range`, or `default` with a note.
fn whole(
    value: &Value,
    default: u32,
    (lo, hi): (u32, u32),
    line: usize,
    key: &str,
    notes: &mut Vec<String>,
) -> u32 {
    let Value::Int(n) = *value else {
        notes.push(format!(
            "line {line}: {key} should be a whole number, so it's {default}"
        ));
        return default;
    };
    let fitted = n.clamp(i64::from(lo), i64::from(hi));
    if fitted != n {
        notes.push(format!(
            "line {line}: {key} = {n} is outside {lo} to {hi}, so it's {fitted}"
        ));
    }
    u32::try_from(fitted).unwrap_or(default)
}

/// Two whole numbers (`[across, up]`) in `range`, or `default` with a note.
fn pair(
    value: &Value,
    default: [u32; 2],
    range: (u32, u32),
    line: usize,
    key: &str,
    notes: &mut Vec<String>,
) -> [u32; 2] {
    let Value::List(items) = value else {
        notes.push(format!(
            "line {line}: {key} should be two whole numbers, [across, up], so it's [{}, {}]",
            default[0], default[1]
        ));
        return default;
    };
    if items.len() != 2 {
        notes.push(format!(
            "line {line}: {key} should be two whole numbers, [across, up], so it's [{}, {}]",
            default[0], default[1]
        ));
        return default;
    }
    [
        whole(&items[0], default[0], range, line, key, notes),
        whole(&items[1], default[1], range, line, key, notes),
    ]
}

/// Writes `settings` as a 2D settings file that [`read`] reads back.
pub fn write(settings: &PixelSettings) -> String {
    let s = settings.sanitized();
    let pair =
        |[a, b]: [u32; 2]| Value::List(vec![Value::Int(i64::from(a)), Value::Int(i64::from(b))]);
    settings::write_settings(
        KIND,
        VERSION,
        "The project's 2D settings. Safe to edit by hand.\n\
         base: the size 2D games are drawn at, in pixels, before they're scaled up.\n\
         pixels_per_unit: how many pixels make a metre. tile: the tile size, in pixels.\n\
         scaling: \"whole steps\" (crisp, with bars if need be) or \"fill\".",
        &[
            ("base", pair(s.base)),
            ("pixels_per_unit", Value::Int(i64::from(s.pixels_per_unit))),
            ("tile", pair(s.tile)),
            ("scaling", Value::Text(s.scaling.name().to_owned())),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_standard_settings_round_trip() {
        let text = write(&PixelSettings::default());
        assert!(text.contains("pixels version 1;"));
        assert!(text.contains("base = [480, 270];"));
        let read = read(&text).unwrap();
        assert_eq!(read.settings, PixelSettings::default());
        assert!(read.notes.is_empty(), "{:?}", read.notes);
    }

    #[test]
    fn other_settings_round_trip() {
        let s = PixelSettings {
            base: [320, 180],
            pixels_per_unit: 32,
            tile: [32, 16],
            scaling: Scaling::Fill,
        };
        assert_eq!(read(&write(&s)).unwrap().settings, s);
    }

    #[test]
    fn values_out_of_range_are_brought_in_with_a_note() {
        let text =
            "pixels version 1;\nbase = [2, 99999];\npixels_per_unit = 0;\ntile = [16, 5000];\n";
        let read = read(text).unwrap();
        assert_eq!(read.settings.base, [BASE_RANGE.0, BASE_RANGE.1]);
        assert_eq!(read.settings.pixels_per_unit, 1);
        assert_eq!(read.settings.tile, [16, TILE_RANGE.1]);
        assert_eq!(read.notes.len(), 4, "{:?}", read.notes);
        assert!(read.notes[0].starts_with("line 2:"));
    }

    #[test]
    fn values_of_the_wrong_kind_fall_back_with_a_note() {
        let text = "pixels version 1;\nbase = 480;\npixels_per_unit = \"lots\";\ntile = [1, 2, 3];\nscaling = \"stretchy\";\nzoom = 3;\n";
        let read = read(text).unwrap();
        assert_eq!(read.settings, PixelSettings::default());
        assert_eq!(read.notes.len(), 5, "{:?}", read.notes);
        assert!(read.notes[4].contains("isn't a 2D setting"));
    }

    #[test]
    fn missing_settings_are_the_standard_ones() {
        let read = read("pixels version 1;\nscaling = \"fill\";\n").unwrap();
        assert_eq!(read.settings.base, [480, 270]);
        assert_eq!(read.settings.scaling, Scaling::Fill);
    }

    #[test]
    fn damaged_newer_and_huge_files_are_refused() {
        assert!(read("").is_err());
        assert!(read("mixer version 1;").is_err());
        let newer = read("pixels version 2;").unwrap_err();
        assert!(newer.message.contains("newer"), "{newer}");
        assert!(read("pixels version 1;\nbase = [480, 270]").is_err());
        assert!(read(&"x".repeat(MAX_BYTES + 1)).unwrap_err().line == 0);
    }

    #[test]
    fn sizes_in_the_world() {
        let s = PixelSettings::default();
        assert_eq!(s.pixel_size(), 1.0 / 16.0);
        assert_eq!(s.tile_size(), [1.0, 1.0]);
        let odd = PixelSettings {
            pixels_per_unit: 0,
            ..s
        };
        assert_eq!(odd.pixel_size(), 1.0);
    }

    #[test]
    fn the_format_is_complete() {
        assert!(FORMAT.is_complete());
    }
}
