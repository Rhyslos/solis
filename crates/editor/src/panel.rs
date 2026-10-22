//! The dockable panels.

use crate::icons::Icon;
use serde::{Deserialize, Serialize};

/// One dockable panel. Each appears exactly once in a layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Panel {
    /// The 3D view of the scene.
    Viewport,
    /// The running game, beside the viewport (Phase 3).
    Game,
    /// The scene's entities, as a tree.
    Scene,
    /// The scene's layers.
    Layers,
    /// The selected entity's components.
    Properties,
    /// Assets: models, textures, sounds, prefabs.
    Library,
    /// Game code in Lust, Rust or Lua (Phase 4).
    Code,
    /// Messages from the editor, the build and the game.
    Console,
    /// Mistakes found in the game's code and scenes.
    Problems,
}

impl Panel {
    /// Every panel, in the order the Window menu lists them.
    pub const ALL: [Self; 9] = [
        Self::Viewport,
        Self::Game,
        Self::Scene,
        Self::Layers,
        Self::Properties,
        Self::Library,
        Self::Code,
        Self::Console,
        Self::Problems,
    ];

    /// The name on the panel's tab.
    pub fn title(self) -> &'static str {
        match self {
            Self::Viewport => "Viewport",
            Self::Game => "Game",
            Self::Scene => "Scene",
            Self::Layers => "Layers",
            Self::Properties => "Properties",
            Self::Library => "Library",
            Self::Code => "Code",
            Self::Console => "Console",
            Self::Problems => "Problems",
        }
    }

    /// The symbol before the name on the panel's tab.
    pub fn icon(self) -> Icon {
        match self {
            Self::Viewport => Icon::Viewport,
            Self::Game => Icon::Game,
            Self::Scene => Icon::Scene,
            Self::Layers => Icon::Layers,
            Self::Properties => Icon::Properties,
            Self::Library => Icon::Library,
            Self::Code => Icon::Code,
            Self::Console => Icon::Console,
            Self::Problems => Icon::Problems,
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
}
