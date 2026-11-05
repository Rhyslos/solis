//! The screen sizes the UI editor previews ([decision 0083](../../../docs/decisions/0083-game-ui-on-any-screen.md)):
//! the common ones, by name, so a screen can be checked on each while it's
//! designed. When playing, the game uses the player's real screen.

/// A screen size to preview: its size in pixels, its name and its shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Preset {
    /// Width and height in pixels.
    pub size: [u32; 2],
    /// What it's called (`Full HD`).
    pub name: &'static str,
    /// Its shape (`16:9`).
    pub shape: &'static str,
}

/// The screens previewed, in the order the list shows them.
pub const PRESETS: &[Preset] = &[
    Preset {
        size: [1920, 1080],
        name: "Full HD",
        shape: "16:9",
    },
    Preset {
        size: [2560, 1440],
        name: "QHD",
        shape: "16:9",
    },
    Preset {
        size: [3840, 2160],
        name: "4K",
        shape: "16:9",
    },
    Preset {
        size: [1280, 720],
        name: "HD",
        shape: "16:9",
    },
    Preset {
        size: [2560, 1080],
        name: "Ultrawide",
        shape: "21:9",
    },
    Preset {
        size: [3440, 1440],
        name: "Ultrawide",
        shape: "21:9",
    },
    Preset {
        size: [1280, 800],
        name: "Steam Deck",
        shape: "16:10",
    },
    Preset {
        size: [1024, 768],
        name: "Old monitor",
        shape: "4:3",
    },
];

/// The smallest and biggest custom screen, each way, in pixels.
pub const CUSTOM_RANGE: (u32, u32) = (64, 8192);

/// A custom size, kept in [`CUSTOM_RANGE`].
pub fn custom(size: [u32; 2]) -> [u32; 2] {
    size.map(|v| v.clamp(CUSTOM_RANGE.0, CUSTOM_RANGE.1))
}
