//! What each ribbon tab shows in the middle of the window (decision
//! 0099; spec 148-158): the documents its editor groups hold, and the
//! tabs of its shelf with the columns each lays side by side.
//!
//! The arrangement is fixed per tab, as the design draws each screen: a
//! panel is never dragged elsewhere. What changes is what's open (a
//! running game, an animator, a tileset, a picture's crops, a clips file,
//! a tile map selected), which adds documents and shelf tabs while it is
//! open; and what each tab remembers (`Arrangement`): the shelf tab in
//! front, the shelf's height and whether it's folded, the split between
//! its two groups and the document in front of each.

use super::RibbonTab;
use crate::panel::Panel;
use crate::tokens::size;
use crate::unbuilt::{Entry, UnbuiltId};

/// One tab of the shelf.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ShelfTab {
    /// The area's entities, the selected one's Properties and the
    /// Library's files (the design's Object tab, in Solis's words).
    Entity,
    /// The file picked in the Library, or the tileset open.
    Details,
    /// The project's code files, with what uses each.
    Code,
    /// A clips file's frames along a time ruler, while one is open.
    Frames,
    /// A clips file's clips, while one is open.
    Clips,
    /// What Properties shows on the Animate tab: the animator's state or
    /// transition picked, or the selected entity.
    Properties,
    /// The open screen's pieces and the one picked.
    Screen,
    /// The bus picked in the mixer.
    Bus,
    /// Messages from the editor, the build and the game.
    Console,
    /// Mistakes found in the game's code and areas.
    Problems,
    /// The selected tile map's layers, terrains and values, while one is
    /// selected.
    Map,
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
}

/// A column, as a constant (so a tab's columns are a constant table).
macro_rules! column {
    ($title:expr, [$($panel:expr),*], $width:expr) => {
        Column {
            title: $title,
            panels: &[$($panel),*],
            width: $width,
        }
    };
}

impl ShelfTab {
    /// Every shelf tab.
    pub const ALL: [Self; 11] = [
        Self::Entity,
        Self::Details,
        Self::Code,
        Self::Frames,
        Self::Clips,
        Self::Properties,
        Self::Screen,
        Self::Bus,
        Self::Console,
        Self::Problems,
        Self::Map,
    ];

    /// The tab's name, on the shelf bar and when saved.
    pub fn name(self) -> &'static str {
        match self {
            Self::Entity => "Entity",
            Self::Details => "Details",
            Self::Code => "Code",
            Self::Frames => "Frames",
            Self::Clips => "Clips",
            Self::Properties => "Properties",
            Self::Screen => "Screen",
            Self::Bus => "Bus",
            Self::Console => "Console",
            Self::Problems => "Problems",
            Self::Map => "Map",
        }
    }

    /// The tab called `name`, as saved.
    pub fn named(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|tab| tab.name() == name)
    }

    /// Its columns, left to right.
    pub fn columns(self) -> &'static [Column] {
        use Panel as P;
        use Width::{Fixed, Rest};
        match self {
            Self::Entity => &[
                column!("Area", [P::Area], Fixed(size::SCENE_COLUMN_W)),
                column!("Properties", [P::Properties, P::Hints], Rest),
                column!("Library", [P::Library], Fixed(size::ASSETS_COLUMN_W)),
            ],
            Self::Screen => &[
                column!("Elements", [P::Elements], Fixed(size::SCENE_COLUMN_W)),
                column!("Properties", [P::Properties], Rest),
            ],
            Self::Details | Self::Properties | Self::Bus => {
                &[column!("Properties", [P::Properties], Rest)]
            }
            Self::Code => &[column!("Code", [P::Code], Rest)],
            Self::Frames => &[column!("Frames", [P::Frames], Rest)],
            Self::Clips => &[column!("Clips", [P::Clips], Rest)],
            Self::Console => &[column!("Console", [P::Console], Rest)],
            Self::Problems => &[column!("Problems", [P::Problems], Rest)],
            Self::Map => &[column!("Map", [P::Map], Rest)],
        }
    }

    /// Every panel the tab shows.
    pub fn panels(self) -> impl Iterator<Item = Panel> {
        self.columns().iter().flat_map(|c| c.panels.iter().copied())
    }

    /// Whether a close button sits on the tab: it shows a file of its own
    /// that it closes (until the Animate screen gives a clips file its
    /// own document tab).
    pub fn closes_a_file(self) -> bool {
        matches!(self, Self::Frames | Self::Clips)
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
}

/// The documents of a tab's editor groups: one or two groups, each a
/// list of tabs. A tab with no screen of its own (File, Shading, VFX) has
/// none: a page stands there.
pub fn groups(tab: RibbonTab, open: OpenNow) -> Vec<Vec<Panel>> {
    use Panel as P;
    let mut groups: Vec<Vec<Panel>> = match tab {
        RibbonTab::Home | RibbonTab::Build => {
            let mut area = vec![P::Viewport];
            if open.playing {
                area.push(P::Game);
            }
            vec![area]
        }
        RibbonTab::Library => vec![vec![P::Library]],
        RibbonTab::Animate => vec![vec![P::Viewport]],
        RibbonTab::Ui => vec![vec![P::Canvas]],
        RibbonTab::Audio => vec![vec![P::Mixer]],
        RibbonTab::Debug => vec![vec![P::Game, P::Viewport], vec![P::Profiler]],
        RibbonTab::File | RibbonTab::Shading | RibbonTab::Vfx => Vec::new(),
    };
    let second: Vec<Panel> = match tab {
        RibbonTab::Library => [(open.tileset, P::Tileset), (open.atlas, P::AtlasCrop)]
            .into_iter()
            .filter_map(|(there, panel)| there.then_some(panel))
            .collect(),
        RibbonTab::Animate if open.animator => vec![P::Animator],
        _ => Vec::new(),
    };
    if !second.is_empty() {
        groups.push(second);
    }
    groups
}

/// The shelf's tabs on `tab`, unfinished ones among them, in the shelf
/// bar's order.
pub fn shelf_tabs(tab: RibbonTab, open: OpenNow) -> Vec<Entry<ShelfTab>> {
    use ShelfTab as S;
    let mut tabs = match tab {
        RibbonTab::Home | RibbonTab::Build => vec![
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
            Entry::Built(S::Properties),
            Entry::Unbuilt(UnbuiltId::AnimShelfKey),
            Entry::Built(S::Console),
        ],
        RibbonTab::Ui => vec![Entry::Built(S::Screen), Entry::Built(S::Console)],
        RibbonTab::Audio => vec![
            Entry::Built(S::Bus),
            Entry::Unbuilt(UnbuiltId::AudioMixerSnapshot),
            Entry::Built(S::Console),
        ],
        RibbonTab::Debug => vec![
            Entry::Built(S::Console),
            Entry::Built(S::Problems),
            Entry::Unbuilt(UnbuiltId::DebugShelfHottest),
            Entry::Unbuilt(UnbuiltId::DebugShelfMemory),
        ],
        RibbonTab::File | RibbonTab::Shading | RibbonTab::Vfx => Vec::new(),
    };
    if matches!(tab, RibbonTab::Home | RibbonTab::Build) && open.map {
        tabs.push(Entry::Built(S::Map));
    }
    if tab == RibbonTab::Animate && open.clips {
        // A clips file's tabs come first, after the unfinished sequencer.
        tabs.splice(1..1, [Entry::Built(S::Frames), Entry::Built(S::Clips)]);
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
/// (spec 1131-1138).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Arrangement {
    /// The shelf tab in front.
    pub shelf_tab: ShelfTab,
    /// The shelf's content height (under its bar), in points.
    pub shelf_height: f32,
    /// Whether the shelf is folded to its bar.
    pub shelf_folded: bool,
    /// The first group's share of the room, when there are two.
    pub split: f32,
    /// The document in front of each group, if one was chosen.
    pub front: [Option<Panel>; 2],
}

/// The split's limits: neither group gets less than this share.
pub const SPLIT_LIMIT: f32 = 0.15;

impl Arrangement {
    /// The design's standard for `tab`.
    pub fn standard(tab: RibbonTab) -> Self {
        let shelf_tab = built_shelf_tabs(tab, OpenNow::default())
            .first()
            .copied()
            .unwrap_or(ShelfTab::Console);
        Self {
            shelf_tab,
            shelf_height: if tab == RibbonTab::Animate {
                size::SHELF_H_ANIMATE
            } else {
                size::SHELF_H
            },
            shelf_folded: false,
            split: match tab {
                RibbonTab::Library => size::SPLIT_LIBRARY,
                _ => size::SPLIT,
            },
            front: [None, None],
        }
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
    /// chosen if it's there, else the first.
    pub fn shelf_tab_of(&self, tabs: &[ShelfTab]) -> Option<ShelfTab> {
        tabs.iter()
            .copied()
            .find(|tab| *tab == self.shelf_tab)
            .or_else(|| tabs.first().copied())
    }
}

/// Where `panel` lives on `tab`: in a group (its index), or on a shelf
/// tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Home {
    /// A document of the group with this index.
    Group(usize),
    /// A column of this shelf tab.
    Shelf(ShelfTab),
}

/// Where `panel` is on `tab` with `open` open, if it's there.
pub fn home_of(tab: RibbonTab, open: OpenNow, panel: Panel) -> Option<Home> {
    if let Some(index) = groups(tab, open)
        .iter()
        .position(|docs| docs.contains(&panel))
    {
        return Some(Home::Group(index));
    }
    built_shelf_tabs(tab, open)
        .into_iter()
        .find(|shelf| shelf.panels().any(|p| p == panel))
        .map(Home::Shelf)
}

/// The ribbon tab that shows `panel` when the one in front doesn't: the
/// first, in this order, that has it.
pub fn tab_for(panel: Panel, open: OpenNow) -> Option<RibbonTab> {
    const ORDER: [RibbonTab; 6] = [
        RibbonTab::Home,
        RibbonTab::Library,
        RibbonTab::Animate,
        RibbonTab::Ui,
        RibbonTab::Audio,
        RibbonTab::Debug,
    ];
    ORDER
        .into_iter()
        .find(|tab| home_of(*tab, open, panel).is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    const EVERYTHING: OpenNow = OpenNow {
        playing: true,
        animator: true,
        tileset: true,
        atlas: true,
        clips: true,
        map: true,
    };

    #[test]
    fn every_panel_has_a_home_with_everything_open() {
        for panel in Panel::ALL.into_iter().chain(Panel::OPEN_WITH_A_FILE) {
            assert!(
                tab_for(panel, EVERYTHING).is_some(),
                "{panel:?} has no place"
            );
        }
    }

    #[test]
    fn a_panel_is_in_one_place_per_tab() {
        for tab in RibbonTab::ALL {
            let mut seen = Vec::new();
            for panel in groups(tab, EVERYTHING).into_iter().flatten() {
                assert!(!seen.contains(&panel), "{panel:?} twice on {tab:?}");
                seen.push(panel);
            }
            for shelf in built_shelf_tabs(tab, EVERYTHING) {
                for panel in shelf.panels() {
                    assert!(
                        !groups(tab, EVERYTHING).concat().contains(&panel),
                        "{panel:?} is both a document and on the shelf of {tab:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn documents_come_and_go_with_what_is_open() {
        let none = OpenNow::default();
        assert_eq!(groups(RibbonTab::Home, none), [vec![Panel::Viewport]]);
        let playing = OpenNow {
            playing: true,
            ..none
        };
        assert_eq!(
            groups(RibbonTab::Home, playing),
            [vec![Panel::Viewport, Panel::Game]]
        );
        assert_eq!(groups(RibbonTab::Library, none).len(), 1);
        let tileset = OpenNow {
            tileset: true,
            ..none
        };
        assert_eq!(
            groups(RibbonTab::Library, tileset),
            [vec![Panel::Library], vec![Panel::Tileset]]
        );
        assert!(groups(RibbonTab::File, EVERYTHING).is_empty());
        assert!(!built_shelf_tabs(RibbonTab::Home, none).contains(&ShelfTab::Map));
        let map = OpenNow { map: true, ..none };
        assert!(built_shelf_tabs(RibbonTab::Home, map).contains(&ShelfTab::Map));
    }

    #[test]
    fn names_are_their_own_and_read_back() {
        for tab in ShelfTab::ALL {
            assert_eq!(ShelfTab::named(tab.name()), Some(tab));
        }
        assert_eq!(ShelfTab::named("Object"), None);
    }

    #[test]
    fn each_tab_starts_on_its_first_working_shelf_tab() {
        assert_eq!(
            Arrangement::standard(RibbonTab::Home).shelf_tab,
            ShelfTab::Entity
        );
        assert_eq!(
            Arrangement::standard(RibbonTab::Library).shelf_tab,
            ShelfTab::Details
        );
        assert_eq!(
            Arrangement::standard(RibbonTab::Debug).shelf_tab,
            ShelfTab::Console
        );
    }

    #[test]
    fn the_front_falls_back_to_the_first_there_is() {
        let mut a = Arrangement::standard(RibbonTab::Home);
        a.front[0] = Some(Panel::Game);
        assert_eq!(a.front_of(0, &[Panel::Viewport]), Some(Panel::Viewport));
        assert_eq!(
            a.front_of(0, &[Panel::Viewport, Panel::Game]),
            Some(Panel::Game)
        );
        assert_eq!(a.front_of(1, &[]), None);
        a.shelf_tab = ShelfTab::Map;
        assert_eq!(
            a.shelf_tab_of(&[ShelfTab::Entity, ShelfTab::Console]),
            Some(ShelfTab::Entity)
        );
    }
}
