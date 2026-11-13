//! What the frame keeps between runs (spec 1131-1138; plan 4.2, 4.3):
//! the tab in front, whether the ribbon is collapsed, and for each tab
//! its shelf tab, the shelf's fold, and the size of every border dragged,
//! for each layout (decision 0109).
//!
//! It is a saved format (rules 9 and 23). Tabs are kept by name, so a
//! name this build doesn't know (one a later build adds, or damage) is
//! passed over rather than refusing the whole; every field has a default,
//! so a value missing is the standard; and [`Workbench::restore_shell`]
//! repairs what it can: an unknown shelf tab becomes the tab's first, a
//! height that doesn't fit becomes the standard, a split outside its
//! limits becomes the standard, and a size of a layout or border this
//! build doesn't know, or one outside its limits, is passed over.
//!
//! The shelf's height and the split in the shelf layout keep the keys
//! they had before the layouts (`shelf_height`, `split`), so an earlier
//! build still reads them; every other size is under `sizes`, by the
//! layout's name and then the border's.
//!
//! The panel arrangements saved before (the dock's trees under
//! `panel_layout2` and `workspaces`) mean nothing to this frame and are
//! not carried over: the Console says so once.

use super::layouts::PanelLayout;
use super::sizes::{Edge, Sizes};
use super::workspaces::{Arrangement, ShelfTab};
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
    /// The first group's share of the room, in the shelf layout.
    pub split: f32,
    /// Every other border dragged: by the layout's name, then the
    /// border's (`PanelLayout::key`, `Edge::key`).
    pub sizes: BTreeMap<String, BTreeMap<String, f32>>,
}

impl Default for WorkspaceState {
    fn default() -> Self {
        Self {
            shelf_tab: String::new(),
            shelf_height: size::SHELF_H,
            shelf_folded: false,
            split: size::SPLIT,
            sizes: BTreeMap::new(),
        }
    }
}

/// The two sizes kept under their keys from before the layouts.
const OLD_KEYS: [(PanelLayout, Edge); 2] = [
    (PanelLayout::Shelf, Edge::ShelfHeight),
    (PanelLayout::Shelf, Edge::Split),
];

impl WorkspaceState {
    /// What to save of `tab`.
    fn of(tab: RibbonTab, arrangement: &Arrangement, sizes: &Sizes) -> Self {
        let mut kept: BTreeMap<String, BTreeMap<String, f32>> = BTreeMap::new();
        for ((t, layout, edge), value) in sizes.kept() {
            if t == tab && !OLD_KEYS.contains(&(layout, edge)) {
                kept.entry(layout.key().to_owned())
                    .or_default()
                    .insert(edge.key(), value);
            }
        }
        let shelf = PanelLayout::Shelf;
        Self {
            shelf_tab: arrangement.shelf_tab.name().to_owned(),
            shelf_height: sizes.get(tab, shelf, Edge::ShelfHeight),
            shelf_folded: arrangement.shelf_folded,
            split: sizes.get(tab, shelf, Edge::Split),
            sizes: kept,
        }
    }

    /// The arrangement this stands for on `tab`, repaired where it can't
    /// be used as saved, and its sizes taken into `sizes`: those this
    /// build knows, inside their limits, and not just the standard.
    fn restore(&self, tab: RibbonTab, sizes: &mut Sizes) -> Arrangement {
        let standard = Arrangement::standard(tab);
        let shelf_tab = ShelfTab::named(&self.shelf_tab).unwrap_or(standard.shelf_tab);
        let old = [self.shelf_height, self.split];
        let named = self.sizes.iter().flat_map(|(layout, edges)| {
            let layout = PanelLayout::from_key(layout);
            edges
                .iter()
                .filter_map(move |(edge, value)| Some((layout?, Edge::from_key(edge)?, *value)))
        });
        let all = OLD_KEYS
            .into_iter()
            .zip(old)
            .map(|((layout, edge), value)| (layout, edge, value))
            .chain(named);
        for (layout, edge, value) in all {
            if value != edge.standard(tab, layout) {
                sizes.take_saved(tab, layout, edge, value);
            }
        }
        Arrangement {
            shelf_tab,
            shelf_folded: self.shelf_folded,
            ..standard
        }
    }
}

impl Shell {
    /// What to save: every tab whose middle was changed.
    pub fn saved(&self) -> ShellState {
        let mut changed: Vec<RibbonTab> = self.arrangements.keys().copied().collect();
        changed.extend(self.sizes.kept().map(|((tab, _, _), _)| tab));
        changed.sort();
        changed.dedup();
        ShellState {
            tab: self.tab_to_save().name().to_owned(),
            ribbon_collapsed: self.collapsed,
            workspaces: changed
                .into_iter()
                .map(|tab| {
                    let state = WorkspaceState::of(tab, &self.arrangement(tab), &self.sizes);
                    (tab.name().to_owned(), state)
                })
                .collect(),
        }
    }

    /// The tab a later run opens on: File's pages and unfinished screens
    /// are places to visit, so the tab before them is kept.
    fn tab_to_save(&self) -> RibbonTab {
        if self.tab == RibbonTab::File {
            self.before_file
        } else if self.tab.is_built() {
            // A contextual tab's file isn't open again at the next start.
            self.tab.parent()
        } else {
            RibbonTab::Home
        }
    }

    /// The frame as `saved` left it, repaired where it can't be used.
    pub fn restored(saved: &ShellState) -> Self {
        // The contextual tabs' middles are saved too (a Tileset's shelf
        // tab), and were never put back when only the main tabs were
        // looked in (the second review pass's N-22).
        let named = |name: &str| {
            RibbonTab::ALL
                .into_iter()
                .chain(RibbonTab::CONTEXT)
                .find(|t| t.name() == name)
        };
        // Not a contextual tab in front: its file isn't open at the start.
        let tab = named(&saved.tab)
            .filter(|t| *t != RibbonTab::File && t.is_built() && t.parent() == *t)
            .unwrap_or(RibbonTab::Home);
        let mut sizes = Sizes::default();
        let arrangements = saved
            .workspaces
            .iter()
            .filter_map(|(name, state)| named(name).map(|t| (t, state.restore(t, &mut sizes))))
            .collect();
        Self {
            tab,
            before_file: tab,
            collapsed: saved.ribbon_collapsed,
            arrangements,
            sizes,
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
