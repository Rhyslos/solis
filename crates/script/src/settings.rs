//! The player's settings (decision 0091, M6): text size and UI size for
//! every screen (`Screens`, decision 0089), and the game's window
//! (`Window`: fullscreen, its size). Game code reads and sets them (a
//! pause menu offers them); screens read them. They're the player's, not the game's: they're not
//! in saves, recordings or the checksum, so game code should only offer
//! and show them, never let the game itself depend on them.

/// The player's settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerSettings {
    /// Every screen's text, times this (0.5 to 3).
    pub text_size: f64,
    /// Every screen, times this (0.5 to 2).
    pub ui_size: f64,
    /// Whether the game fills the screen.
    pub fullscreen: bool,
    /// The window's size in pixels when it doesn't (each 320 to 16384).
    pub resolution: [u32; 2],
}

impl Default for PlayerSettings {
    fn default() -> Self {
        Self {
            text_size: 1.0,
            ui_size: 1.0,
            fullscreen: false,
            resolution: [1280, 720],
        }
    }
}

/// The smallest and largest text size.
pub const TEXT_SIZES: (f64, f64) = (0.5, 3.0);
/// The smallest and largest UI size.
pub const UI_SIZES: (f64, f64) = (0.5, 2.0);
/// The smallest and largest window side, in pixels.
pub const WINDOW_SIDES: (u32, u32) = (320, 16384);

/// The window sizes `nextResolution` steps through: the usual ones.
pub const RESOLUTIONS: [[u32; 2]; 6] = [
    [1280, 720],
    [1600, 900],
    [1920, 1080],
    [2560, 1440],
    [3440, 1440],
    [3840, 2160],
];

/// One change game code asks for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Setting {
    /// `Screens:setTextSize(size)`.
    TextSize(f64),
    /// `Screens:setUiSize(size)`.
    UiSize(f64),
    /// `Window:setFullscreen(on)`.
    Fullscreen(bool),
    /// `Window:setResolution(width, height)`.
    Resolution(u32, u32),
}

impl PlayerSettings {
    /// These settings with `change` made, kept in their ranges (a number
    /// that isn't one changes nothing).
    pub fn with(mut self, change: Setting) -> Self {
        match change {
            Setting::TextSize(s) if s.is_finite() => {
                self.text_size = s.clamp(TEXT_SIZES.0, TEXT_SIZES.1);
            }
            Setting::UiSize(s) if s.is_finite() => self.ui_size = s.clamp(UI_SIZES.0, UI_SIZES.1),
            Setting::Fullscreen(on) => self.fullscreen = on,
            Setting::Resolution(w, h) => {
                self.resolution = [
                    w.clamp(WINDOW_SIDES.0, WINDOW_SIDES.1),
                    h.clamp(WINDOW_SIDES.0, WINDOW_SIDES.1),
                ];
            }
            _ => {}
        }
        self
    }

    /// Kept in their ranges, as read from a file that may be anything.
    pub fn sanitized(self) -> Self {
        let d = Self::default();
        Self {
            text_size: d.text_size,
            ui_size: d.ui_size,
            fullscreen: self.fullscreen,
            resolution: d.resolution,
        }
        .with(Setting::TextSize(self.text_size))
        .with(Setting::UiSize(self.ui_size))
        .with(Setting::Resolution(self.resolution[0], self.resolution[1]))
    }

    /// The text size in whole percent, as screens lay text out.
    pub fn text_percent(&self) -> u16 {
        (self.text_size * 100.0).round().clamp(50.0, 300.0) as u16
    }

    /// The usual window size after this one (the first after the last),
    /// for a menu that steps through them.
    pub fn next_resolution(&self) -> [u32; 2] {
        RESOLUTIONS
            .iter()
            .position(|r| *r == self.resolution)
            .map_or(RESOLUTIONS[0], |i| RESOLUTIONS[(i + 1) % RESOLUTIONS.len()])
    }
}

#[cfg(test)]
#[path = "settings_tests.rs"]
mod tests;
