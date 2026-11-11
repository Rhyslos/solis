//! The frame round the editor
//! ([decision 0099](../../../../docs/decisions/0099-the-g-layout.md)): the
//! tab bar, the ribbon under it and the File page. The middle (the editor
//! groups, the shelf and the status bar) joins them next; until then the
//! docked panels are drawn under the ribbon as they were.
//!
//! - [`RibbonTab`] is one tab of the tab bar, and [`Shell`] is what the
//!   frame remembers between frames: the tab in front, whether the ribbon
//!   is collapsed, the File page shown. It is the editor's one field for
//!   all of it.
//! - `commands.rs` is the one table of what the editor can do by name
//!   ([`ShellCommand`]): the ribbon's buttons, the File page's items and
//!   the search all run its entries, and show its shortcuts.
//! - `tab_bar.rs` draws the top band; `ribbon.rs` draws a ribbon from a
//!   table, and `ribbons/` holds one small table for each tab;
//!   `play_group.rs` is the group at the ribbon's right end;
//!   `file_page.rs` is the File tab's menu column and page.

mod command_words;
pub(crate) mod commands;
mod create;
mod file_page;

pub(crate) use commands::ShellCommand;
pub(crate) use file_page::FileItem;

use crate::layout;
use crate::panel::Panel;
use crate::workbench::Workbench;
use crate::workspace::Workspace;

/// One tab of the tab bar: an area of the engine, with a ribbon of its
/// own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RibbonTab {
    /// Projects, settings and leaving: a page in place of the editor.
    File,
    /// Building areas.
    Home,
    /// The project's files.
    Library,
    /// Materials and shaders (not built yet).
    Shading,
    /// Sprite clips and animators.
    Animate,
    /// Game screens, drawn on the canvas.
    Ui,
    /// Effects (not built yet).
    Vfx,
    /// The mixer.
    Audio,
    /// The profiler, the Console and Problems, with the game big.
    Debug,
    /// Exporting the game.
    Build,
}

impl RibbonTab {
    /// Every tab, in the tab bar's order.
    pub const ALL: [Self; 10] = [
        Self::File,
        Self::Home,
        Self::Library,
        Self::Shading,
        Self::Animate,
        Self::Ui,
        Self::Vfx,
        Self::Audio,
        Self::Debug,
        Self::Build,
    ];

    /// The tab's name.
    pub fn name(self) -> &'static str {
        match self {
            Self::File => "File",
            Self::Home => "Home",
            Self::Library => "Library",
            Self::Shading => "Shading",
            Self::Animate => "Animate",
            Self::Ui => "UI",
            Self::Vfx => "VFX",
            Self::Audio => "Audio",
            Self::Debug => "Debug",
            Self::Build => "Build",
        }
    }

    /// What the tab is for, on hover.
    pub fn help(self) -> &'static str {
        match self {
            Self::File => "Projects, settings, the manual and updates",
            Self::Home => "Building areas: making entities, snapping, saving and playing",
            Self::Library => "The project's files: bringing in, making, finding and placing them",
            Self::Shading => "Materials and shaders",
            Self::Animate => "Animating sprites: a clips file's clips and frames, and animators",
            Self::Ui => "Drawing game screens on the canvas",
            Self::Vfx => "Effects",
            Self::Audio => "The mixer: the project's buses and how loud each is",
            Self::Debug => {
                "Testing the game: where each frame's time goes, the Console and Problems"
            }
            Self::Build => "Exporting the game for someone to play",
        }
    }

    /// Whether the tab's screen is built. One that isn't is drawn as an
    /// unfinished part: its tab, its ribbon and one page in the middle.
    pub fn is_built(self) -> bool {
        !matches!(self, Self::Shading | Self::Vfx)
    }

    /// The arrangement of panels the tab shows, while the panels are
    /// still docked. A tab without one keeps the arrangement in use.
    fn workspace(self) -> Option<Workspace> {
        match self {
            Self::Home | Self::Library | Self::Audio => Some(Workspace::Level),
            Self::Animate => Some(Workspace::Animate),
            Self::Ui => Some(Workspace::Ui),
            Self::Debug => Some(Workspace::Code),
            Self::File | Self::Shading | Self::Vfx | Self::Build => None,
        }
    }

    /// The panel the tab brings to the front.
    fn panel(self) -> Option<Panel> {
        match self {
            Self::Library => Some(Panel::Library),
            Self::Audio => Some(Panel::Mixer),
            Self::Debug => Some(Panel::Profiler),
            _ => None,
        }
    }

    /// The tab that shows `workspace`, for an arrangement saved before
    /// there were tabs.
    pub(crate) fn of_workspace(workspace: Workspace) -> Self {
        match workspace {
            Workspace::Level => Self::Home,
            Workspace::Animate => Self::Animate,
            Workspace::Code => Self::Debug,
            Workspace::Ui => Self::Ui,
        }
    }
}

/// What the frame remembers between frames.
#[derive(Debug, Clone, PartialEq)]
pub struct Shell {
    /// The tab in front.
    pub(crate) tab: RibbonTab,
    /// The tab that was in front before File: Back to editor returns to
    /// it.
    pub(crate) before_file: RibbonTab,
    /// Whether the ribbon is collapsed to its tabs.
    pub(crate) collapsed: bool,
    /// While collapsed: whether a click on a tab has dropped the ribbon
    /// down over the center.
    pub(crate) dropped: bool,
    /// The File page shown.
    pub(crate) file_item: FileItem,
}

impl Default for Shell {
    fn default() -> Self {
        Self {
            tab: RibbonTab::Home,
            before_file: RibbonTab::Home,
            collapsed: false,
            dropped: false,
            file_item: FileItem::RecentProjects,
        }
    }
}

impl Shell {
    /// Whether a whole page stands where the docked panels would be: a
    /// File page, or an unfinished tab's.
    pub(crate) fn covers_center(&self) -> bool {
        self.tab == RibbonTab::File || !self.tab.is_built()
    }

    /// Whether the ribbon is drawn in its own band, pushing the center
    /// down.
    pub(crate) fn ribbon_in_band(&self) -> bool {
        !self.collapsed && self.tab != RibbonTab::File
    }
}

impl Workbench {
    /// The tab in front.
    pub fn ribbon_tab(&self) -> RibbonTab {
        self.shell.tab
    }

    /// Brings `tab` to the front: its ribbon, and what it shows in the
    /// middle. A tab that isn't built can't be shown while unfinished
    /// parts are hidden.
    pub fn show_tab(&mut self, tab: RibbonTab) {
        if !tab.is_built() && self.interface_look().hide_unfinished {
            return;
        }
        if tab == RibbonTab::File {
            self.show_file_page(self.shell.file_item);
            return;
        }
        self.shell.tab = tab;
        if let Some(workspace) = tab.workspace() {
            self.switch_workspace(workspace);
        }
        match tab.panel() {
            // An unpinned Library is a drawer, not a docked panel.
            Some(Panel::Library) if !self.library_pinned => self.drawer_open = true,
            Some(panel) => layout::bring_to_front(&mut self.dock, panel),
            None => {}
        }
    }

    /// A tab hidden with the unfinished parts can't stay in front: Home
    /// takes its place. Called at the start of each frame.
    pub(crate) fn settle_tab(&mut self) {
        if !self.shell.tab.is_built() && self.interface_look().hide_unfinished {
            self.show_tab(RibbonTab::Home);
        }
    }

    /// Collapses the ribbon to its tabs, or brings it back.
    pub fn set_ribbon_collapsed(&mut self, collapsed: bool) {
        self.shell.collapsed = collapsed;
        self.shell.dropped = false;
    }

    /// Whether the ribbon is collapsed to its tabs.
    pub fn ribbon_collapsed(&self) -> bool {
        self.shell.collapsed
    }

    /// A tab was clicked: it comes to the front, and a collapsed ribbon
    /// drops down over the center until something on it is chosen or the
    /// pointer leaves it.
    pub(crate) fn tab_clicked(&mut self, tab: RibbonTab) {
        let again = self.shell.tab == tab;
        self.show_tab(tab);
        if self.shell.collapsed && self.shell.tab != RibbonTab::File {
            // A second click on the tab in front puts the ribbon away.
            self.shell.dropped = !(again && self.shell.dropped);
        }
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
