//! What each ribbon tab shows in the middle of the window (decision
//! 0099; spec 148-158): the documents its editor groups hold, and the
//! tabs of its shelf with the columns each lays side by side.
//!
//! The arrangement is fixed per tab, as the design draws each screen: a
//! panel is never dragged elsewhere. What changes is what's open (a
//! running game, an animator, a tileset, a picture's crops, a clips file,
//! a tile map selected), which adds documents and shelf tabs while it is
//! open; and what each tab remembers (`Arrangement`): the shelf tab in
//! front, whether the shelf is folded, and the document in front of each
//! group. The sizes of its borders are kept apart (`sizes.rs`).
//!
//! On Home, the layout chosen in Preferences (decision 0109) takes the
//! Area, Properties and Assets lists off the shelf into columns of their
//! own (`layouts.rs`): the Entity shelf tab goes, and Place keeps only
//! its Placement column.

use super::RibbonTab;
use super::layouts::PanelLayout;
use crate::panel::Panel;
use crate::tokens::size;
use crate::unbuilt::{Entry, UnbuiltId};

/// One tab of the shelf.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ShelfTab {
    /// Placing: the area's entities, how placed things land, and the
    /// files to drag in (spec 262-276). Its being in front is the
    /// placing mode (the project lead's answer 20): the ribbon takes its
    /// build form and the tool bar goes.
    Place,
    /// The area's entities, the selected one's Properties and the
    /// Library's files (the design's Object tab, in Solis's words).
    Entity,
    /// The file picked in the Library, or the tileset open.
    Details,
    /// The project's code files, with what uses each.
    Code,
    /// A clips file's frames along a time ruler, while one is open.
    Frames,
    /// The chosen clip's playback, while a clips file is open.
    Clip,
    /// The open animator's parameters.
    Parameters,
    /// What Properties shows on the Animate tab: the animator's state or
    /// transition picked, or the selected entity.
    Properties,
    /// The open screen's pieces and the one picked.
    Screen,
    /// The project's sound events, and the one picked.
    SoundEvents,
    /// Messages from the editor, the build and the game.
    Console,
    /// Mistakes found in the game's code and areas.
    Problems,
    /// The selected tile map's layers, terrains and values, while one is
    /// selected.
    Map,
    /// The open tileset's terrains, the one chosen and its slot picked.
    Terrains,
    /// The open picture's crops and the one picked.
    Crops,
}

/// How wide a shelf column is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Width {
    /// A fixed width, in points.
    Fixed(f32),
    /// What the fixed columns leave.
    Rest,
}

/// One column of a shelf tab: a header and the panels stacked in it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Column {
    /// The header's words (a tab with one column draws no header).
    pub title: &'static str,
    /// The panels in it, top to bottom; a second one takes the lower
    /// part.
    pub panels: &'static [Panel],
    /// How wide it is.
    pub width: Width,
    /// Whether its panel draws its own header in the header's place
    /// (Properties: the entity's name, + Component and ⋯, spec 199).
    pub own_header: bool,
}

/// A column, as a constant (so a tab's columns are a constant table).
macro_rules! column {
    ($title:expr, [$($panel:expr),*], $width:expr) => {
        Column {
            title: $title,
            panels: &[$($panel),*],
            width: $width,
            own_header: false,
        }
    };
    ($title:expr, [$($panel:expr),*], $width:expr, own_header) => {
        Column {
            title: $title,
            panels: &[$($panel),*],
            width: $width,
            own_header: true,
        }
    };
}

impl ShelfTab {
    /// Every shelf tab.
    pub const ALL: [Self; 15] = [
        Self::Place,
        Self::Entity,
        Self::Details,
        Self::Code,
        Self::Frames,
        Self::Clip,
        Self::Parameters,
        Self::Properties,
        Self::Screen,
        Self::SoundEvents,
        Self::Console,
        Self::Problems,
        Self::Map,
        Self::Terrains,
        Self::Crops,
    ];

    /// The tab's name, on the shelf bar and when saved.
    pub fn name(self) -> &'static str {
        match self {
            Self::Place => "Place",
            Self::Entity => "Entity",
            Self::Details => "Details",
            Self::Code => "Code",
            Self::Frames => "Frames",
            Self::Clip => "Clip",
            Self::Parameters => "Parameters",
            Self::Properties => "Properties",
            Self::Screen => "Screen",
            Self::SoundEvents => "Sound events",
            Self::Console => "Console",
            Self::Problems => "Problems",
            Self::Map => "Map",
            Self::Terrains => "Terrains",
            Self::Crops => "Crops",
        }
    }

    /// The tab called `name`, as saved.
    pub fn named(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|tab| tab.name() == name)
    }

    /// Its columns, left to right, in `layout`.
    pub fn columns(self, layout: PanelLayout) -> &'static [Column] {
        use Panel as P;
        use Width::{Fixed, Rest};
        match self {
            // The Area and Assets lists are columns of their own: placing
            // keeps how placed things land on the shelf.
            Self::Place if layout.is_side() => &[column!("Placement", [P::Placement], Rest)],
            Self::Place => &[
                column!("Area", [P::Area], Fixed(size::SCENE_COLUMN_W)),
                column!("Placement", [P::Placement], Fixed(size::PLACEMENT_COLUMN_W)),
                column!("Assets", [P::Assets], Rest),
            ],
            Self::Entity => &[
                column!("Area", [P::Area], Fixed(size::SCENE_COLUMN_W)),
                column!("Properties", [P::Properties], Rest, own_header),
                column!("Assets", [P::Assets], Fixed(size::ASSETS_COLUMN_W)),
            ],
            Self::Screen => &[
                column!("Elements", [P::Elements], Fixed(size::SCENE_COLUMN_W)),
                column!("Properties", [P::Properties], Rest),
                column!(
                    "Pictures",
                    [P::Pictures],
                    Fixed(size::ASSETS_COLUMN_W - 50.0)
                ),
            ],
            Self::Details | Self::Properties => &[column!("Properties", [P::Properties], Rest)],
            Self::Code => &[column!("Code", [P::Code], Rest)],
            Self::Frames => &[
                column!("Clips", [P::Clips], Fixed(size::SCENE_COLUMN_W + 60.0)),
                column!("Frames", [P::Frames], Rest),
            ],
            Self::Clip => &[column!("Clip", [P::Clip], Rest)],
            Self::Parameters => &[column!("Parameters", [P::Parameters], Rest)],
            Self::SoundEvents => &[column!("Sound events", [P::SoundEvents], Rest)],
            Self::Console => &[column!("Console", [P::Console], Rest)],
            Self::Problems => &[column!("Problems", [P::Problems], Rest)],
            Self::Map => &[column!("Map", [P::Map], Rest)],
            Self::Terrains => &[
                column!("Terrains", [P::Terrains], Fixed(size::TERRAINS_COLUMN_W)),
                column!("Slot", [P::Properties], Rest),
            ],
            Self::Crops => &[column!("Crops", [P::Crops], Rest)],
        }
    }

    /// Every panel the tab shows in `layout`.
    pub fn panels(self, layout: PanelLayout) -> impl Iterator<Item = Panel> {
        self.columns(layout)
            .iter()
            .flat_map(|c| c.panels.iter().copied())
    }

    /// Whether a close button sits on the tab: it shows a file of its own
    /// that it closes (until the Animate screen gives a clips file its
    /// own document tab).
    pub fn closes_a_file(self) -> bool {
        matches!(self, Self::Frames | Self::Clip)
    }
}

/// What is open now, which adds documents and shelf tabs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OpenNow {
    /// The game is running: Home shows a Game tab.
    pub playing: bool,
    /// An animator is open.
    pub animator: bool,
    /// A tileset is open.
    pub tileset: bool,
    /// A picture's crops are open.
    pub atlas: bool,
    /// A clips file is open.
    pub clips: bool,
    /// An entity with a tile map is selected.
    pub map: bool,
    /// The layout Home is drawn in (decision 0109): the one chosen in
    /// Preferences, or the shelf while the window is too narrow for it.
    pub layout: PanelLayout,
}

/// The documents of a tab's editor groups: one or two groups, each a
/// list of tabs. A tab with no screen of its own (File, Shading, VFX) has
/// none: a page stands there.
pub fn groups(tab: RibbonTab, open: OpenNow) -> Vec<Vec<Panel>> {
    use Panel as P;
    let groups: Vec<Vec<Panel>> = match tab {
        // Export for playtesting is Build's page (round 3).
        RibbonTab::Build => vec![vec![P::Export]],
        RibbonTab::Home => {
            let mut area = vec![P::Viewport];
            if open.playing {
                area.push(P::Game);
            }
            vec![area]
        }
        RibbonTab::Library => vec![vec![P::Library]],
        RibbonTab::Tileset if open.tileset => vec![vec![P::Tileset]],
        RibbonTab::Picture if open.atlas => vec![vec![P::AtlasCrop]],
        // With neither a clips file nor an animator open, the Sequencer's
        // page stands in the middle (round 3).
        RibbonTab::Animate if open.animator => {
            vec![vec![P::Viewport, P::Animator], vec![P::Preview]]
        }
        RibbonTab::Animate if open.clips => vec![vec![P::Viewport]],
        RibbonTab::Ui => vec![vec![P::Canvas]],
        RibbonTab::Audio => vec![vec![P::Mixer], vec![P::Bus]],
        // The Profiler fills the middle (round 3); the Game beside it is
        // the one the Debug tab looks into.
        RibbonTab::Debug => vec![vec![P::Profiler, P::Game]],
        RibbonTab::File
        | RibbonTab::Shading
        | RibbonTab::Vfx
        | RibbonTab::Animate
        | RibbonTab::Tileset
        | RibbonTab::Picture
        | RibbonTab::Model => Vec::new(),
    };
    groups
}

/// The shelf's tabs on `tab`, unfinished ones among them, in the shelf
/// bar's order.
pub fn shelf_tabs(tab: RibbonTab, open: OpenNow) -> Vec<Entry<ShelfTab>> {
    use ShelfTab as S;
    let mut tabs = match tab {
        RibbonTab::Build => vec![
            Entry::Built(S::Console),
            Entry::Built(S::Problems),
            Entry::Unbuilt(UnbuiltId::BuildLog),
        ],
        RibbonTab::Home => vec![
            Entry::Built(S::Place),
            Entry::Built(S::Entity),
            Entry::Unbuilt(UnbuiltId::HomeShelfAnimation),
            Entry::Built(S::Console),
            Entry::Built(S::Problems),
        ],
        RibbonTab::Library => vec![
            Entry::Built(S::Details),
            Entry::Built(S::Code),
            Entry::Unbuilt(UnbuiltId::LibShelfImportQueue),
            Entry::Unbuilt(UnbuiltId::LibSourceHistory),
        ],
        RibbonTab::Animate => vec![
            Entry::Unbuilt(UnbuiltId::AnimShelfSequencer),
            Entry::Unbuilt(UnbuiltId::AnimShelfKey),
            Entry::Built(S::Console),
        ],
        RibbonTab::Ui => vec![
            Entry::Built(S::Screen),
            Entry::Built(S::Console),
            Entry::Built(S::Problems),
        ],
        RibbonTab::Audio => vec![
            Entry::Built(S::SoundEvents),
            Entry::Unbuilt(UnbuiltId::AudioMixerSnapshot),
            Entry::Built(S::Console),
        ],
        RibbonTab::Debug => vec![
            Entry::Built(S::Console),
            Entry::Built(S::Problems),
            Entry::Unbuilt(UnbuiltId::DebugShelfHottest),
            Entry::Unbuilt(UnbuiltId::DebugShelfMemory),
        ],
        RibbonTab::Tileset if open.tileset => vec![
            Entry::Built(S::Terrains),
            Entry::Built(S::Console),
            Entry::Built(S::Problems),
        ],
        RibbonTab::Picture if open.atlas => vec![
            Entry::Built(S::Crops),
            Entry::Built(S::Console),
            Entry::Built(S::Problems),
        ],
        RibbonTab::File
        | RibbonTab::Shading
        | RibbonTab::Vfx
        | RibbonTab::Tileset
        | RibbonTab::Picture
        | RibbonTab::Model => Vec::new(),
    };
    if tab == RibbonTab::Home && open.map {
        tabs.push(Entry::Built(S::Map));
    }
    if tab == RibbonTab::Home && open.layout.is_side() {
        // Its three lists are columns of their own.
        tabs.retain(|entry| *entry != Entry::Built(S::Entity));
    }
    if tab == RibbonTab::Animate {
        // A clips file's tabs, then an animator's, after the unfinished
        // sequencer (round 3).
        let mut file_tabs = Vec::new();
        if open.clips {
            file_tabs.extend([Entry::Built(S::Frames), Entry::Built(S::Clip)]);
        }
        if open.animator {
            file_tabs.extend([Entry::Built(S::Parameters), Entry::Built(S::Properties)]);
        }
        tabs.splice(1..1, file_tabs);
    }
    tabs
}

/// The shelf tabs on `tab` that work, unfinished ones left out.
pub fn built_shelf_tabs(tab: RibbonTab, open: OpenNow) -> Vec<ShelfTab> {
    shelf_tabs(tab, open)
        .into_iter()
        .filter_map(|entry| match entry {
            Entry::Built(tab) => Some(tab),
            Entry::Unbuilt(_) => None,
        })
        .collect()
}

/// What a tab remembers of its middle, between frames and between runs
/// (spec 1131-1138), apart from its borders' sizes (`sizes.rs`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Arrangement {
    /// The shelf tab in front.
    pub shelf_tab: ShelfTab,
    /// Whether the shelf is folded to its bar.
    pub shelf_folded: bool,
    /// The document in front of each group, if one was chosen.
    pub front: [Option<Panel>; 2],
    /// A document dragged to one half of the center, standing alone in
    /// a group there (`true`: the right half), the rest beside it (the
    /// project lead's answer 25). Kept while the editor runs.
    pub alone: Option<(Panel, bool)>,
    /// The groups were joined by dragging a tab onto another group's
    /// strip: every document in one group.
    pub merged: bool,
}

/// The split's limits: neither group gets less than this share.
pub const SPLIT_LIMIT: f32 = 0.15;

impl Arrangement {
    /// The design's standard for `tab`.
    pub fn standard(tab: RibbonTab) -> Self {
        // Home opens on Entity, the level editor; Place is first on its
        // shelf bar but is a mode chosen by bringing it to the front.
        // In a side layout, where Home has no Entity tab, the first after
        // Place stands in for it (`shelf_tab_of`).
        let everything = OpenNow {
            tileset: true,
            atlas: true,
            layout: PanelLayout::Shelf,
            ..OpenNow::default()
        };
        let shelf_tab =
            first_not_placing(&built_shelf_tabs(tab, everything)).unwrap_or(ShelfTab::Console);
        Self {
            shelf_tab,
            shelf_folded: false,
            front: [None, None],
            alone: None,
            merged: false,
        }
    }

    /// The groups of `tab` with `open` open, as this arrangement has
    /// them: the table's, joined into one, or with one document alone in
    /// a half of its own.
    pub fn arranged(&self, tab: RibbonTab, open: OpenNow) -> Vec<Vec<Panel>> {
        let groups = groups(tab, open);
        let all: Vec<Panel> = groups.concat();
        if all.len() < 2 {
            return groups;
        }
        if let Some((panel, right)) = self.alone.filter(|(p, _)| all.contains(p)) {
            let rest: Vec<Panel> = all.into_iter().filter(|p| *p != panel).collect();
            return if right {
                vec![rest, vec![panel]]
            } else {
                vec![vec![panel], rest]
            };
        }
        if self.merged {
            return vec![all];
        }
        groups
    }

    /// The document in front of group `index` of `documents`: the one
    /// chosen if it's there, else the first.
    pub fn front_of(&self, index: usize, documents: &[Panel]) -> Option<Panel> {
        let chosen = self.front.get(index).copied().flatten();
        chosen
            .filter(|panel| documents.contains(panel))
            .or_else(|| documents.first().copied())
    }

    /// The shelf tab in front, of those `tabs` there are now: the one
    /// chosen if it's there, else the first that isn't Place (which is a
    /// mode, chosen only by bringing it to the front), else Place.
    pub fn shelf_tab_of(&self, tabs: &[ShelfTab]) -> Option<ShelfTab> {
        tabs.iter()
            .copied()
            .find(|tab| *tab == self.shelf_tab)
            .or_else(|| first_not_placing(tabs))
            .or_else(|| tabs.first().copied())
    }
}

/// The first of `tabs` that isn't Place.
fn first_not_placing(tabs: &[ShelfTab]) -> Option<ShelfTab> {
    tabs.iter().copied().find(|tab| *tab != ShelfTab::Place)
}

/// Where `panel` lives on `tab`: in a group (its index), or on a shelf
/// tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Home {
    /// A document of the group with this index.
    Group(usize),
    /// A column of this shelf tab.
    Shelf(ShelfTab),
    /// A column of its own beside the groups, or the strip under them
    /// (decision 0109): always in view.
    Beside,
}

/// Where `panel` is on `tab` with `open` open, if it's there.
pub fn home_of(tab: RibbonTab, open: OpenNow, panel: Panel) -> Option<Home> {
    if let Some(index) = groups(tab, open)
        .iter()
        .position(|docs| docs.contains(&panel))
    {
        return Some(Home::Group(index));
    }
    if tab == RibbonTab::Home && open.layout.has_beside(panel) {
        return Some(Home::Beside);
    }
    // The Area and Assets columns are on Place too, but their home is
    // Entity: showing one doesn't start placing.
    built_shelf_tabs(tab, open)
        .into_iter()
        .filter(|shelf| *shelf != ShelfTab::Place || panel == Panel::Placement)
        .find(|shelf| shelf.panels(open.layout).any(|p| p == panel))
        .map(Home::Shelf)
}

/// The ribbon tab that shows `panel` when the one in front doesn't: the
/// first, in this order, that has it.
pub fn tab_for(panel: Panel, open: OpenNow) -> Option<RibbonTab> {
    const ORDER: [RibbonTab; 9] = [
        RibbonTab::Home,
        RibbonTab::Library,
        RibbonTab::Tileset,
        RibbonTab::Picture,
        RibbonTab::Animate,
        RibbonTab::Ui,
        RibbonTab::Audio,
        RibbonTab::Debug,
        RibbonTab::Build,
    ];
    ORDER
        .into_iter()
        .find(|tab| home_of(*tab, open, panel).is_some())
}

#[cfg(test)]
#[path = "workspaces_tests.rs"]
mod tests;
