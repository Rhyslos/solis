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
    /// The project's buses: a fader, meter, mute and solo each (Phase 7).
    Mixer,
    /// The clips of the clips file open (Phase 9, D3): beside Properties
    /// while a `.solframes` file is open, and gone when it's closed.
    Clips,
    /// The chosen clip's frames along a time ruler (Phase 9, D3): at the
    /// bottom while a `.solframes` file is open.
    Frames,
    /// The animator file open (Phase 9, D3b): its graph of states and
    /// transitions, beside the viewport while a `.solanim` file is open.
    Animator,
    /// The selected tile map's layers, terrains and values (Phase 9, D4):
    /// beside Properties while a map is selected.
    Map,
    /// The tileset open (Phase 9, D5): its terrains, picture, slots and
    /// rules, beside the viewport while a `.tileset.lst` file is open.
    Tileset,
    /// A picture's crops (decision 0089): beside the viewport while its
    /// atlas file is open.
    AtlasCrop,
    /// The Library's open folder as tiles, on Home's Entity shelf tab
    /// (spec 200).
    Assets,
    /// How placed things land (spec 270): on Home's Place shelf tab.
    Placement,
    /// The UI editor's canvas (decision 0089): a screen drawn to pick and
    /// change, in the UI workspace while a screen is open.
    Canvas,
    /// The open screen's pieces as a tree, beside the canvas.
    Elements,
    /// The open tileset's terrains, and the one chosen: on LIBRARY TOOLS
    /// · Tileset's Terrains shelf tab (round 3).
    Terrains,
    /// The open picture's crops, and the one picked: on LIBRARY TOOLS ·
    /// Picture's Crops shelf tab (round 3).
    Crops,
    /// The chosen clip's playback: Animate's Clip shelf tab (round 3).
    Clip,
    /// The open animator's parameters: Animate's Parameters shelf tab.
    Parameters,
    /// The open animator's preview: a group of its own beside the graph.
    Preview,
    /// The project's pictures for the canvas's Image tool: on the UI
    /// tab's Screen shelf tab (round 3).
    Pictures,
}

impl Panel {
    /// Every panel, in the order the Window menu lists them.
    pub const ALL: [Self; 12] = [
        Self::Viewport,
        Self::Game,
        Self::Area,
        Self::Properties,
        Self::Library,
        Self::Code,
        Self::Console,
        Self::Problems,
        Self::Profiler,
        Self::Mixer,
        Self::Assets,
        Self::Placement,
    ];

    /// Panels that come and go with what's open (a clips file), never in
    /// [`Self::ALL`] or the Window menu, and never saved in a layout.
    pub const OPEN_WITH_A_FILE: [Self; 14] = [
        Self::Clips,
        Self::Frames,
        Self::Animator,
        Self::Map,
        Self::Tileset,
        Self::AtlasCrop,
        Self::Canvas,
        Self::Elements,
        Self::Terrains,
        Self::Crops,
        Self::Clip,
        Self::Parameters,
        Self::Preview,
        Self::Pictures,
    ];

    /// The tabs a clips file opens (and closes) together.
    pub const FRAMES_TABS: [Self; 3] = [Self::Clips, Self::Frames, Self::Clip];

    /// The tabs a screen opens (and closes) together, in the UI workspace.
    pub const SCREEN_TABS: [Self; 3] = [Self::Canvas, Self::Elements, Self::Pictures];

    /// Whether it comes and goes with what's open (and can be closed).
    pub fn comes_with_a_file(self) -> bool {
        Self::OPEN_WITH_A_FILE.contains(&self)
    }

    /// The name on the panel's tab.
    pub fn title(self) -> &'static str {
        match self {
            Self::Viewport => "Viewport",
            Self::Game => "Game",
            Self::Area => "Area",
            Self::Properties => "Properties",
            Self::Library => "Library",
            Self::Code => "Code",
            Self::Console => "Console",
            Self::Problems => "Problems",
            Self::Profiler => "Profiler",
            Self::Mixer => "Mixer",
            Self::Clips => "Clips",
            Self::Frames => "Frames",
            Self::Animator => "Animator",
            Self::Map => "Map",
            Self::Tileset => "Tileset",
            Self::AtlasCrop => "Atlas Crop",
            Self::Canvas => "Canvas",
            Self::Elements => "Elements",
            Self::Assets => "Assets",
            Self::Placement => "Placement",
            Self::Terrains => "Terrains",
            Self::Crops => "Crops",
            Self::Clip => "Clip",
            Self::Parameters => "Parameters",
            Self::Preview => "Preview",
            Self::Pictures => "Pictures",
        }
    }

    /// The symbol before the name on the panel's tab.
    pub fn icon(self) -> Icon {
        match self {
            Self::Viewport => Icon::Viewport,
            Self::Game => Icon::Game,
            Self::Area => Icon::Area,
            Self::Properties => Icon::Properties,
            Self::Library => Icon::Library,
            Self::Code => Icon::Code,
            Self::Console => Icon::Console,
            Self::Problems => Icon::Warning,
            Self::Profiler => Icon::Profiler,
            Self::Mixer => Icon::Sound,
            Self::Clips => Icon::Clips,
            Self::Frames => Icon::Clips,
            Self::Animator => Icon::Animator,
            Self::Map => Icon::TileMap,
            Self::Tileset => Icon::Tileset,
            Self::AtlasCrop => Icon::Crops,
            Self::Canvas => Icon::Screen,
            Self::Elements => Icon::Area,
            Self::Assets => Icon::Library,
            Self::Placement => Icon::Move,
            Self::Terrains => Icon::Tileset,
            Self::Crops => Icon::Crops,
            Self::Clip => Icon::Clips,
            Self::Parameters => Icon::Sliders,
            Self::Preview => Icon::Play,
            Self::Pictures => Icon::Picture,
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
