//! The dockable panels.

use crate::icons::Icon;
use serde::{Deserialize, Serialize};

/// One dockable panel. Each appears exactly once in a layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Panel {
    /// The 3D view of the area.
    Viewport,
    /// The running game, beside the viewport (Phase 3).
    Game,
    /// The area's entities, as a tree. Layouts saved before it was
    /// renamed call it "Scene".
    #[serde(alias = "Scene")]
    Area,
    /// The area's layers.
    Layers,
    /// The selected entity's components.
    Properties,
    /// Assets: models, textures, sounds, assemblies.
    Library,
    /// Game code in Lust, Rust or Lua (Phase 4).
    Code,
    /// Messages from the editor, the build and the game.
    Console,
    /// Mistakes found in the game's code and areas.
    Problems,
    /// Where each frame's time goes (Phase 5).
    Profiler,
}

impl Panel {
    /// Every panel, in the order the Window menu lists them.
    pub const ALL: [Self; 10] = [
        Self::Viewport,
        Self::Game,
        Self::Area,
        Self::Layers,
        Self::Properties,
        Self::Library,
        Self::Code,
        Self::Console,
        Self::Problems,
        Self::Profiler,
    ];

    /// The name on the panel's tab.
    pub fn title(self) -> &'static str {
        match self {
            Self::Viewport => "Viewport",
            Self::Game => "Game",
            Self::Area => "Area",
            Self::Layers => "Layers",
            Self::Properties => "Properties",
            Self::Library => "Library",
            Self::Code => "Code",
            Self::Console => "Console",
            Self::Problems => "Problems",
            Self::Profiler => "Profiler",
        }
    }

    /// The symbol before the name on the panel's tab.
    pub fn icon(self) -> Icon {
        match self {
            Self::Viewport => Icon::Viewport,
            Self::Game => Icon::Game,
            Self::Area => Icon::Area,
            Self::Layers => Icon::Layers,
            Self::Properties => Icon::Properties,
            Self::Library => Icon::Library,
            Self::Code => Icon::Code,
            Self::Console => Icon::Console,
            Self::Problems => Icon::Problems,
            Self::Profiler => Icon::Profiler,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_panel_has_its_own_title() {
        let mut titles: Vec<_> = Panel::ALL.iter().map(|p| p.title()).collect();
        titles.sort_unstable();
        titles.dedup();
        assert_eq!(titles.len(), Panel::ALL.len());
    }

    #[test]
    fn a_layout_saved_before_the_rename_still_finds_the_area_panel() {
        let old: Panel = ron::from_str("Scene").unwrap();
        assert_eq!(old, Panel::Area);
        assert_eq!(ron::to_string(&Panel::Area).unwrap(), "Area");
    }
}
