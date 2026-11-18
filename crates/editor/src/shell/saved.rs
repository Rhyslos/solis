//! What the frame keeps between runs (spec 1131-1138; decisions 0109 and
//! 0118): the tab in front, whether the ribbon is collapsed, and for each
//! tab its bottom tab, whether its bottom panel is folded, which slots
//! are hidden, and the size of every edge dragged, for each layout.
//!
//! It is a saved format (rules 9 and 23). Tabs are kept by name, so a
//! name this build doesn't know (one a later build adds, or damage) is
//! passed over rather than refusing the whole; every field has a default,
//! so a value missing is the standard; and [`Workbench::restore_shell`]
//! repairs what it can: an unknown bottom tab becomes the tab's first, a
//! size of a layout or edge this build doesn't know, or one outside its
//! limits, is passed over.
//!
//! **Before the frame** (decision 0109's shelf and side columns, saved by
//! builds up to the rethink's R2) the same keys held sizes in points, by
//! old names: the shelf's `shelf_height` and `split` at the top, and
//! under `sizes` the side columns' `left` and `right`, the Assets
//! strip's `strip`, the shelf's `shelf` and its columns' widths. Nothing
//! is laid out differently, so the format keeps its shape: the new sizes
//! are shares under new names (`Edge::key`), and the old names are still
//! read, each turned into its nearest new edge as the 1440 × 900 window
//! they were made for would have had it, and never written again. The
//! layouts saved then read as their nearest (`PanelLayout::from_key`).
//!
//! The docked arrangements saved before the frame (the dock's trees
//! under `panel_layout2` and `workspaces`) mean nothing to it and are not
//! carried over: the Console says so once.

use super::layouts::PanelLayout;
use super::sizes::{Edge, Sizes};
use super::workspaces::{Arrangement, BottomTab, Hidden, Slot};
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
    /// Each tab's frame, by the tab's name.
    pub workspaces: BTreeMap<String, WorkspaceState>,
}

/// One tab's frame, as saved.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkspaceState {
    /// The bottom tab in front, by name (the shelf's tab before the
    /// frame, whose key it keeps).
    #[serde(rename = "shelf_tab")]
    pub bottom_tab: String,
    /// The shelf's height under its bar, in points: written before the
    /// frame, read once into the bottom panel's height (`NOT_SAVED` when
    /// there is none).
    #[serde(skip_serializing)]
    pub shelf_height: f32,
    /// Whether the bottom panel is folded to its line (the shelf's fold
    /// before the frame, whose key it keeps).
    #[serde(rename = "shelf_folded")]
    pub bottom_folded: bool,
    /// The first group's share of the room, in the shelf layout: written
    /// before the frame, read once into the standard layout's split (`NOT_SAVED`
    /// when there is none).
    #[serde(skip_serializing)]
    pub split: f32,
    /// The slots hidden: `left`, `right`, `library`.
    pub hidden: Vec<String>,
    /// Every edge dragged: by the layout's name, then the edge's
    /// (`PanelLayout::key`, `Edge::key`).
    pub sizes: BTreeMap<String, BTreeMap<String, f32>>,
}

/// What an old size that wasn't saved reads as.
const NOT_SAVED: f32 = -1.0;

impl Default for WorkspaceState {
    fn default() -> Self {
        Self {
            bottom_tab: String::new(),
            shelf_height: NOT_SAVED,
            bottom_folded: false,
            split: NOT_SAVED,
            hidden: Vec::new(),
            sizes: BTreeMap::new(),
        }
    }
}

/// The window the sizes saved in points before the frame were made in:
/// the editor's first size, 1440 × 900, less the tab bar, the ribbon and
/// the status line.
const OLD_MIDDLE: [f32; 2] = [
    1440.0,
    900.0 - size::TAB_BAR_H - size::RIBBON_H - size::STATUS_BAR_H,
];

/// The shelf's standard height under its bar before the frame, and on
/// the Animate tab.
const OLD_SHELF_H: f32 = 300.0;
const OLD_SHELF_H_ANIMATE: f32 = 360.0;

/// A size saved before the frame, by its old name, as the edge it became
/// and its value now (a share where the edge is one). `layout` is the
/// layout's old key, which says what the shelf held then.
fn old_size(old_layout: &str, key: &str, value: f32) -> Option<(Edge, f32)> {
    let [w, h] = OLD_MIDDLE;
    let on_shelf = old_layout == "shelf";
    match key {
        "left" => Some((Edge::Left, value / w)),
        "right" => Some((Edge::Right, value / w)),
        // The Assets strip, header and all, became the bottom panel.
        "strip" => Some((Edge::Bottom, (value - size::BOTTOM_BAR_H).max(0.0) / h)),
        // The shelf's height under its bar: the bottom panel's, where the
        // shelf held what the bottom panel holds now.
        "shelf" if on_shelf => Some((Edge::Bottom, value / h)),
        // The Place tab's Placement column.
        "column Place 1" => Some((Edge::Placement, value)),
        _ => None,
    }
}

impl WorkspaceState {
    /// What to save of `tab`.
    fn of(tab: RibbonTab, arrangement: &Arrangement, sizes: &Sizes) -> Self {
        let mut kept: BTreeMap<String, BTreeMap<String, f32>> = BTreeMap::new();
        for ((t, layout, edge), value) in sizes.kept() {
            if t == tab {
                kept.entry(layout.key().to_owned())
                    .or_default()
                    .insert(edge.key().to_owned(), value);
            }
        }
        let hidden = [Slot::Left, Slot::Right, Slot::Library]
            .into_iter()
            .filter(|slot| arrangement.hidden.has(*slot))
            .map(|slot| slot_key(slot).to_owned())
            .collect();
        Self {
            bottom_tab: arrangement.bottom_tab.name().to_owned(),
            shelf_height: NOT_SAVED,
            bottom_folded: arrangement.bottom_folded,
            split: NOT_SAVED,
            hidden,
            sizes: kept,
        }
    }

    /// The arrangement this stands for on `tab`, repaired where it can't
    /// be used as saved, and its sizes taken into `sizes`: those this
    /// build knows, inside their limits, and not just the standard. A
    /// size saved before the frame is taken only where no size saved
    /// since says otherwise.
    fn restore(&self, tab: RibbonTab, sizes: &mut Sizes) -> Arrangement {
        let standard = Arrangement::standard(tab);
        let bottom_tab = BottomTab::named(&self.bottom_tab).unwrap_or(standard.bottom_tab);
        let mut found: BTreeMap<(PanelLayout, Edge), f32> = BTreeMap::new();
        let mut take_old = |layout: PanelLayout, (edge, value): (Edge, f32)| {
            found.entry((layout, edge)).or_insert(value);
        };
        // The side layouts' sizes before the shelf's: on Home both became
        // the standard layout's, and the side layout was the standard.
        let (shelf, sides): (Vec<_>, Vec<_>) = self
            .sizes
            .iter()
            .partition(|(layout, _)| *layout == "shelf");
        for (old_layout, edges) in sides.into_iter().chain(shelf) {
            let Some(layout) = PanelLayout::from_key(old_layout) else {
                continue;
            };
            for (key, value) in edges {
                if Edge::from_key(key).is_none()
                    && let Some(old) = old_size(old_layout, key, *value)
                {
                    take_old(layout, old);
                }
            }
        }
        // Written whether dragged or not: only a height that isn't the
        // shelf's standard then was dragged. Home's shelf held its lists.
        let [_, h] = OLD_MIDDLE;
        let old_standard = if tab == RibbonTab::Animate {
            OLD_SHELF_H_ANIMATE
        } else {
            OLD_SHELF_H
        };
        let height = self.shelf_height;
        if height > 0.0 && tab != RibbonTab::Home && height != old_standard {
            take_old(PanelLayout::Standard, (Edge::Bottom, height / h));
        }
        if self.split > 0.0 {
            take_old(PanelLayout::Standard, (Edge::Split, self.split));
        }
        // Sizes saved since the frame, by their own names, over any old.
        for (layout, edges) in &self.sizes {
            let Some(layout) = PanelLayout::from_key(layout) else {
                continue;
            };
            for (key, value) in edges {
                if let Some(edge) = Edge::from_key(key) {
                    found.insert((layout, edge), *value);
                }
            }
        }
        for ((layout, edge), value) in found {
            if value != edge.standard(tab, layout) {
                sizes.take_saved(tab, layout, edge, value);
            }
        }
        let mut hidden = Hidden::default();
        for name in &self.hidden {
            if let Some(slot) = [Slot::Left, Slot::Right, Slot::Library]
                .into_iter()
                .find(|slot| slot_key(*slot) == name)
            {
                hidden.set(slot, true);
            }
        }
        Arrangement {
            bottom_tab,
            bottom_folded: self.bottom_folded,
            hidden,
            ..standard
        }
    }
}

/// A slot's name, as saved among the hidden ones.
fn slot_key(slot: Slot) -> &'static str {
    match slot {
        Slot::Left => "left",
        Slot::Right => "right",
        Slot::Library => "library",
    }
}

impl Shell {
    /// What to save: every tab whose frame was changed.
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
        // The contextual tabs' frames are saved too (a Tileset's bottom
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
                                    wasn't carried over. Each tab now remembers its own panels \
                                    and sizes.";

impl Workbench {
    /// The frame, for saving.
    pub fn shell_state(&self) -> ShellState {
        self.shell.saved()
    }

    /// Takes in the frame saved last time, if any. With none, and an
    /// arrangement saved by an earlier build (`had_old_layout`), the
    /// Console says once that it wasn't carried over: every tab starts
    /// from its standard.
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
