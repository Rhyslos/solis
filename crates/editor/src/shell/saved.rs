//! What the frame keeps between runs (spec 1131-1138; plan 4.2, 4.3):
//! the tab in front, whether the ribbon is collapsed, and for each tab
//! its shelf tab, the shelf's height and fold, and its split.
//!
//! It is a saved format (rules 9 and 23). Tabs are kept by name, so a
//! name this build doesn't know (one a later build adds, or damage) is
//! passed over rather than refusing the whole; every field has a default,
//! so a value missing is the standard; and [`Workbench::restore_shell`]
//! repairs what it can: an unknown shelf tab becomes the tab's first, a
//! height that doesn't fit becomes the standard, a split outside its
//! limits becomes the standard.
//!
//! The panel arrangements saved before (the dock's trees under
//! `panel_layout2` and `workspaces`) mean nothing to this frame and are
//! not carried over: the Console says so once.

use super::workspaces::{Arrangement, SPLIT_LIMIT, ShelfTab};
use super::{RibbonTab, Shell};
use crate::tokens::size;
use crate::workbench::Workbench;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The frame as saved.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ShellState {
    /// The tab in front, by name.
    pub tab: String,
    /// Whether the ribbon is collapsed to its tabs.
    pub ribbon_collapsed: bool,
    /// Each tab's middle, by the tab's name.
    pub workspaces: BTreeMap<String, WorkspaceState>,
}

/// One tab's middle, as saved.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkspaceState {
    /// The shelf tab in front, by name.
    pub shelf_tab: String,
    /// The shelf's height under its bar, in points.
    pub shelf_height: f32,
    /// Whether the shelf is folded to its bar.
    pub shelf_folded: bool,
    /// The first group's share of the room.
    pub split: f32,
}

impl Default for WorkspaceState {
    fn default() -> Self {
        Self {
            shelf_tab: String::new(),
            shelf_height: size::SHELF_H,
            shelf_folded: false,
            split: size::SPLIT,
        }
    }
}

/// The most the shelf's content may take, in points: the window has room
/// for it and a center at least [`size::CENTER_MIN_H`] tall.
pub(crate) const SHELF_MAX: f32 = 2_000.0;

impl WorkspaceState {
    fn of(arrangement: &Arrangement) -> Self {
        Self {
            shelf_tab: arrangement.shelf_tab.name().to_owned(),
            shelf_height: arrangement.shelf_height,
            shelf_folded: arrangement.shelf_folded,
            split: arrangement.split,
        }
    }

    /// The arrangement this stands for on `tab`, repaired where it can't
    /// be used as saved.
    fn restore(&self, tab: RibbonTab) -> Arrangement {
        let standard = Arrangement::standard(tab);
        let shelf_tab = ShelfTab::named(&self.shelf_tab).unwrap_or(standard.shelf_tab);
        let height = self.shelf_height;
        let shelf_height =
            if height.is_finite() && (size::SHELF_MIN_H..=SHELF_MAX).contains(&height) {
                height
            } else {
                standard.shelf_height
            };
        let split =
            if self.split.is_finite() && (SPLIT_LIMIT..=1.0 - SPLIT_LIMIT).contains(&self.split) {
                self.split
            } else {
                standard.split
            };
        Arrangement {
            shelf_tab,
            shelf_height,
            shelf_folded: self.shelf_folded,
            split,
            front: [None, None],
        }
    }
}

impl Shell {
    /// What to save.
    pub fn saved(&self) -> ShellState {
        ShellState {
            tab: self.tab_to_save().name().to_owned(),
            ribbon_collapsed: self.collapsed,
            workspaces: self
                .arrangements
                .iter()
                .map(|(tab, a)| (tab.name().to_owned(), WorkspaceState::of(a)))
                .collect(),
        }
    }

    /// The tab a later run opens on: File's pages and unfinished screens
    /// are places to visit, so the tab before them is kept.
    fn tab_to_save(&self) -> RibbonTab {
        if self.tab == RibbonTab::File {
            self.before_file
        } else if self.tab.is_built() {
            self.tab
        } else {
            RibbonTab::Home
        }
    }

    /// The frame as `saved` left it, repaired where it can't be used.
    pub fn restored(saved: &ShellState) -> Self {
        let named = |name: &str| RibbonTab::ALL.into_iter().find(|t| t.name() == name);
        let tab = named(&saved.tab)
            .filter(|t| *t != RibbonTab::File && t.is_built())
            .unwrap_or(RibbonTab::Home);
        let arrangements = saved
            .workspaces
            .iter()
            .filter_map(|(name, state)| named(name).map(|t| (t, state.restore(t))))
            .collect();
        Self {
            tab,
            before_file: tab,
            collapsed: saved.ribbon_collapsed,
            arrangements,
            ..Self::default()
        }
    }
}

/// What the Console says once, the first time this frame opens on
/// settings that had a panel arrangement from before it.
pub(crate) const NOT_CARRIED_OVER: &str = "The interface is new: your panel arrangement from before \
                                    wasn't carried over. Each tab now remembers its own shelf \
                                    and split.";

impl Workbench {
    /// The frame, for saving.
    pub fn shell_state(&self) -> ShellState {
        self.shell.saved()
    }

    /// Takes in the frame saved last time, if any. With none, and an
    /// arrangement saved by an earlier build (`had_old_layout`), the
    /// Console says once that it wasn't carried over: every tab starts
    /// from the design's standard.
    pub fn restore_shell(&mut self, saved: Option<&ShellState>, had_old_layout: bool) {
        match saved {
            Some(saved) => self.shell = Shell::restored(saved),
            None if had_old_layout => {
                self.log(crate::console::LogLevel::Info, NOT_CARRIED_OVER);
            }
            None => {}
        }
    }
}

#[cfg(test)]
#[path = "saved_tests.rs"]
mod tests;
