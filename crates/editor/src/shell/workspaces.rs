//! What each ribbon tab shows in the frame's four slots (decision 0118,
//! board 09): its left slot (a list of what is in it), its centre (the
//! documents its editor groups hold), its right slot (the picked thing's
//! settings) and its bottom panel (small tabs of one panel, under the
//! centre only).
//!
//! The table is fixed per tab, as the board draws each one: a panel is
//! never dragged elsewhere. What changes is what's open (a running game,
//! an animator, a tileset, a picture's crops, a clips file, a tile map
//! selected), which adds documents, slots and bottom tabs while it is
//! open; the layout chosen in Preferences (`layouts.rs`), which moves the
//! slots; and what each tab remembers (`Arrangement`): the bottom tab in
//! front, whether the bottom panel is folded, which slots are hidden, and
//! the document in front of each group. The sizes of its edges are kept
//! apart (`sizes.rs`).

use super::RibbonTab;
use super::layouts::PanelLayout;
use crate::panel::Panel;
use crate::unbuilt::{Entry, UnbuiltId};

/// One tab of the bottom panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BottomTab {
    /// The Library's open folder as tiles (the assets browser), and while
    /// placing, how placed things land beside it.
    Library,
    /// The project's code files, with what uses each.
    Code,
    /// A clips file's frames along a time ruler, while one is open: the
    /// Animate tab's timeline.
    Frames,
    /// The chosen clip's playback, while a clips file is open.
    Clip,
    /// The project's pictures for the canvas's Image tool.
    Pictures,
    /// Messages from the editor, the build and the game, with the
    /// command line.
    Console,
    /// Mistakes found in the game's code and levels.
    Problems,
    /// The selected tile map's layers, terrains and values, while one is
    /// selected.
    Map,
}

/// A slot's or a bottom tab's contents: a header and the panels in it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Column {
    /// The header's words.
    pub title: &'static str,
    /// The panels in it, top to bottom; a second one takes the lower
    /// part.
    pub panels: &'static [Panel],
    /// A part that isn't built yet, drawn as its page in place of panels
    /// (and gone with the unfinished parts).
    pub unbuilt: Option<Entry<()>>,
}

/// A column, as a constant (so a tab's slots are a constant table).
macro_rules! column {
    ($title:expr, [$($panel:expr),*]) => {
        Column {
            title: $title,
            panels: &[$($panel),*],
            unbuilt: None,
        }
    };
}

/// A slot whose part isn't built yet.
const fn unbuilt_column(title: &'static str, part: Entry<()>) -> Column {
    Column {
        title,
        panels: &[],
        unbuilt: Some(part),
    }
}

impl BottomTab {
    /// Every bottom tab.
    pub const ALL: [Self; 8] = [
        Self::Library,
        Self::Code,
        Self::Frames,
        Self::Clip,
        Self::Pictures,
        Self::Console,
        Self::Problems,
        Self::Map,
    ];

    /// The tab's name, on the bottom panel and when saved.
    pub fn name(self) -> &'static str {
        match self {
            Self::Library => "Library",
            Self::Code => "Code",
            Self::Frames => "Frames",
            Self::Clip => "Clip",
            Self::Pictures => "Pictures",
            Self::Console => "Console",
            Self::Problems => "Problems",
            Self::Map => "Map",
        }
    }

    /// The tab called `name`, as saved. The shelf's tabs from before the
    /// frame (decision 0118) that held the assets (Place, Entity) read
    /// as the Library; the rest moved into slots, and read as nothing.
    pub fn named(name: &str) -> Option<Self> {
        match name {
            "Place" | "Entity" => Some(Self::Library),
            _ => Self::ALL.into_iter().find(|tab| tab.name() == name),
        }
    }

    /// Its symbol, before its name.
    pub fn icon(self) -> crate::icons::Icon {
        match self {
            Self::Library => crate::icons::Icon::FolderOpen,
            other => other.panel().icon(),
        }
    }

    /// The panel it shows.
    pub fn panel(self) -> Panel {
        match self {
            Self::Library => Panel::Assets,
            Self::Code => Panel::Code,
            Self::Frames => Panel::Frames,
            Self::Clip => Panel::Clip,
            Self::Pictures => Panel::Pictures,
            Self::Console => Panel::Console,
            Self::Problems => Panel::Problems,
            Self::Map => Panel::Map,
        }
    }

    /// Every panel it shows: the Library's Placement too, while placing.
    pub fn panels(self) -> impl Iterator<Item = Panel> {
        let placement = (self == Self::Library).then_some(Panel::Placement);
        std::iter::once(self.panel()).chain(placement)
    }

    /// Whether a close button sits on the tab: it shows a file of its own
    /// that it closes (until the Animate screen gives a clips file its
    /// own document tab).
    pub fn closes_a_file(self) -> bool {
        matches!(self, Self::Frames | Self::Clip)
    }
}

/// The Library as a column of its own, where a layout puts it beside the
/// centre.
pub const LIBRARY: Column = column!("Library", [Panel::Assets]);

/// The Library's column while placing: how placed things land over the
/// files.
pub const LIBRARY_PLACING: Column = column!("Library", [Panel::Placement, Panel::Assets]);

/// A slot of the frame beside the centre.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Slot {
    /// The left one: a list of what is in what's worked on.
    Left,
    /// The right one: the picked thing's settings.
    Right,
    /// The Library, where a layout takes it out of the bottom panel into
    /// a column.
    Library,
}

/// What `tab` puts in its left slot with `open` open, if anything.
pub fn left_slot(tab: RibbonTab, open: OpenNow) -> Option<Column> {
    use Panel as P;
    match tab {
        // The Level panel, or the project's folders (decision 0118: the
        // toggle in the slot's title switches them).
        RibbonTab::Home if open.folders => Some(column!("Folders", [P::Folders])),
        RibbonTab::Home => Some(column!("Level", [P::Level])),
        RibbonTab::Animate => match (open.clips, open.animator) {
            (true, true) => Some(column!("Clips", [P::Clips, P::Parameters])),
            (true, false) => Some(column!("Clips", [P::Clips])),
            (false, true) => Some(column!("Parameters", [P::Parameters])),
            (false, false) => None,
        },
        RibbonTab::Ui => Some(column!("Elements", [P::Elements])),
        RibbonTab::Audio => Some(column!("Sound events", [P::SoundEvents])),
        RibbonTab::Debug => Some(unbuilt_column(
            "Running",
            Entry::Unbuilt(UnbuiltId::DebugRunning),
        )),
        RibbonTab::Tileset if open.tileset => Some(column!("Terrains", [P::Terrains])),
        RibbonTab::Picture if open.atlas => Some(column!("Crops", [P::Crops])),
        _ => None,
    }
}

/// What `tab` puts in its right slot with `open` open, if anything.
pub fn right_slot(tab: RibbonTab, open: OpenNow) -> Option<Column> {
    use Panel as P;
    match tab {
        RibbonTab::Home | RibbonTab::Ui => Some(column!("Properties", [P::Properties])),
        RibbonTab::Library => Some(column!("Details", [P::Properties])),
        RibbonTab::Animate if open.animator => Some(column!("Properties", [P::Properties])),
        RibbonTab::Audio => Some(column!("Bus", [P::Bus])),
        RibbonTab::Tileset if open.tileset => Some(column!("Slot", [P::Properties])),
        _ => None,
    }
}

/// What is open now, which adds documents, slots and bottom tabs.
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
    /// The layout chosen in Preferences: where the slots sit.
    pub layout: PanelLayout,
    /// Home's left slot shows the project's folders, not the Level panel.
    pub folders: bool,
    /// A prefab's page is open: a tab beside the level's on Home.
    pub prefab_page: bool,
    /// A data table is open: a tab beside the level's on Home.
    pub table: bool,
}

/// The documents of a tab's editor groups: one or two groups, each a
/// list of tabs. A tab with no screen of its own (File, Shading, VFX) has
/// none: a page stands there.
pub fn groups(tab: RibbonTab, open: OpenNow) -> Vec<Vec<Panel>> {
    use Panel as P;
    match tab {
        // Export for playtesting is Build's page (round 3).
        RibbonTab::Build => vec![vec![P::Export]],
        RibbonTab::Home => {
            let mut level = vec![P::Viewport];
            if open.prefab_page {
                level.push(P::PrefabPage);
            }
            if open.table {
                level.push(P::Table);
            }
            if open.playing {
                level.push(P::Game);
            }
            vec![level]
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
        RibbonTab::Audio => vec![vec![P::Mixer]],
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
    }
}

/// The bottom panel's tabs on `tab`, unfinished ones among them, in
/// their order. The Library is among them even where the layout takes it
/// into a column (`Frame::bottom_tabs` leaves it out there).
pub fn bottom_tabs(tab: RibbonTab, open: OpenNow) -> Vec<Entry<BottomTab>> {
    use BottomTab as B;
    use Entry::{Built, Unbuilt};
    let mut tabs = match tab {
        RibbonTab::Build => vec![
            Built(B::Console),
            Built(B::Problems),
            Unbuilt(UnbuiltId::BuildLog),
        ],
        RibbonTab::Home => vec![Built(B::Library), Built(B::Console), Built(B::Problems)],
        RibbonTab::Library => vec![
            Built(B::Code),
            Built(B::Console),
            Built(B::Problems),
            Unbuilt(UnbuiltId::LibShelfImportQueue),
            Unbuilt(UnbuiltId::LibSourceHistory),
        ],
        // The timeline under the preview: a clips file's frames, then
        // the unfinished animation of what's selected (once Home's), the
        // sequencer and the key.
        RibbonTab::Animate => {
            let mut tabs = Vec::new();
            if open.clips {
                tabs.extend([Built(B::Frames), Built(B::Clip)]);
            }
            tabs.extend([
                Unbuilt(UnbuiltId::HomeShelfAnimation),
                Unbuilt(UnbuiltId::AnimShelfSequencer),
                Unbuilt(UnbuiltId::AnimShelfKey),
                Built(B::Console),
            ]);
            tabs
        }
        RibbonTab::Ui => vec![
            Built(B::Library),
            Built(B::Pictures),
            Built(B::Console),
            Built(B::Problems),
        ],
        RibbonTab::Audio => vec![
            Built(B::Library),
            Unbuilt(UnbuiltId::AudioMixerSnapshot),
            Built(B::Console),
        ],
        RibbonTab::Debug => vec![
            Built(B::Console),
            Built(B::Problems),
            Unbuilt(UnbuiltId::DebugShelfHottest),
            Unbuilt(UnbuiltId::DebugShelfMemory),
        ],
        RibbonTab::Tileset if open.tileset => vec![Built(B::Console), Built(B::Problems)],
        RibbonTab::Picture if open.atlas => vec![Built(B::Console), Built(B::Problems)],
        RibbonTab::File
        | RibbonTab::Shading
        | RibbonTab::Vfx
        | RibbonTab::Tileset
        | RibbonTab::Picture
        | RibbonTab::Model => Vec::new(),
    };
    if tab == RibbonTab::Home && open.map {
        tabs.push(Built(B::Map));
    }
    tabs
}

/// The bottom tabs on `tab` that work, unfinished ones left out.
pub fn built_bottom_tabs(tab: RibbonTab, open: OpenNow) -> Vec<BottomTab> {
    bottom_tabs(tab, open)
        .into_iter()
        .filter_map(|entry| match entry {
            Entry::Built(tab) => Some(tab),
            Entry::Unbuilt(_) => None,
        })
        .collect()
}

/// Which slots are hidden, from a title's ⋯ menu or by a layout.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Hidden {
    /// The left slot.
    pub left: bool,
    /// The right slot.
    pub right: bool,
    /// The Library, where a layout puts it in a column.
    pub library: bool,
}

impl Hidden {
    /// Every slot hidden.
    pub const ALL: Self = Self {
        left: true,
        right: true,
        library: true,
    };

    /// Whether `slot` is hidden.
    pub fn has(self, slot: Slot) -> bool {
        match slot {
            Slot::Left => self.left,
            Slot::Right => self.right,
            Slot::Library => self.library,
        }
    }

    /// Hides `slot`, or shows it again.
    pub fn set(&mut self, slot: Slot, hidden: bool) {
        match slot {
            Slot::Left => self.left = hidden,
            Slot::Right => self.right = hidden,
            Slot::Library => self.library = hidden,
        }
    }

    /// Whether anything is hidden.
    pub fn any(self) -> bool {
        self.left || self.right || self.library
    }
}

/// What a tab remembers of its frame, between frames and between runs
/// (spec 1131-1138), apart from its edges' sizes (`sizes.rs`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Arrangement {
    /// The bottom tab in front.
    pub bottom_tab: BottomTab,
    /// Whether the bottom panel is folded to its one line.
    pub bottom_folded: bool,
    /// Which slots are hidden.
    pub hidden: Hidden,
    /// Whether the Library is placing (Home only): its Place switch is
    /// on, the ribbon takes its build form and the tool bar goes.
    pub placing: bool,
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
    /// The standard for `tab`: its first bottom tab in front (the
    /// timeline on Animate), open, except on the Library tab, whose
    /// browser fills the tab (board 09); nothing hidden.
    pub fn standard(tab: RibbonTab) -> Self {
        let everything = OpenNow {
            tileset: true,
            atlas: true,
            clips: true,
            ..OpenNow::default()
        };
        let bottom_tab = built_bottom_tabs(tab, everything)
            .first()
            .copied()
            .unwrap_or(BottomTab::Console);
        Self {
            bottom_tab,
            bottom_folded: tab == RibbonTab::Library,
            hidden: Hidden::default(),
            placing: false,
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

    /// The bottom tab in front, of those `tabs` there are now: the one
    /// chosen if it's there, else the first.
    pub fn bottom_tab_of(&self, tabs: &[BottomTab]) -> Option<BottomTab> {
        tabs.iter()
            .copied()
            .find(|tab| *tab == self.bottom_tab)
            .or_else(|| tabs.first().copied())
    }
}

/// Where `panel` lives on `tab`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Home {
    /// A document of the group with this index.
    Group(usize),
    /// A slot beside the centre (or the Library's column).
    Slot(Slot),
    /// A tab of the bottom panel.
    Bottom(BottomTab),
}

/// Where `panel` is on `tab` with `open` open, if it's there.
pub fn home_of(tab: RibbonTab, open: OpenNow, panel: Panel) -> Option<Home> {
    if let Some(index) = groups(tab, open)
        .iter()
        .position(|docs| docs.contains(&panel))
    {
        return Some(Home::Group(index));
    }
    // Home's left slot holds either of its two: showing one switches.
    if tab == RibbonTab::Home && matches!(panel, Panel::Level | Panel::Folders) {
        return Some(Home::Slot(Slot::Left));
    }
    for (slot, column) in [
        (Slot::Left, left_slot(tab, open)),
        (Slot::Right, right_slot(tab, open)),
    ] {
        if column.is_some_and(|c| c.panels.contains(&panel)) {
            return Some(Home::Slot(slot));
        }
    }
    // Only Home's Library places.
    if panel == Panel::Placement && tab != RibbonTab::Home {
        return None;
    }
    let bottom = built_bottom_tabs(tab, open)
        .into_iter()
        .find(|bottom| bottom.panels().any(|p| p == panel))?;
    if bottom == BottomTab::Library && open.layout.library_beside() {
        return Some(Home::Slot(Slot::Library));
    }
    Some(Home::Bottom(bottom))
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
