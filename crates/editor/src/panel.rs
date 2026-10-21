//! The dockable panels.

use serde::{Deserialize, Serialize};

/// One dockable panel. Each appears exactly once in a layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Panel {
    /// The 3D view of the scene.
    Viewport,
    /// The running game, beside the viewport (Phase 5).
    Game,
    /// The scene's entities, as a tree.
    Scene,
    /// The scene's layers.
    Layers,
    /// The selected entity's components.
    Properties,
    /// Assets and code.
    Library,
    /// Messages from the editor, the build and the game.
    Console,
}

impl Panel {
    /// Every panel, in the order the Window menu lists them.
    pub const ALL: [Self; 7] = [
        Self::Viewport,
        Self::Game,
        Self::Scene,
        Self::Layers,
        Self::Properties,
        Self::Library,
        Self::Console,
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
            Self::Console => "Console",
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
