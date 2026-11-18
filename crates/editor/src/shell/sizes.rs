//! What each tab remembers of its edges (decisions 0109 and 0118): every
//! edge a drag moves (the side columns' edges, the bottom panel's top,
//! the line between two parts stacked in a column, the line between two
//! groups, the placing column's edge), one plain number each, kept per
//! tab and per layout, or, with **Each tab keeps its own panel sizes**
//! off, the frame's edges shared by every tab.
//!
//! Only what was dragged is kept; anything else is its standard, and a
//! double-click on an edge forgets it again. Looking one up is a map
//! lookup, so the frame works out its layout from a handful of numbers
//! and never from what the panels hold.

use super::RibbonTab;
use super::layouts::PanelLayout;
use crate::tokens::size;
use std::collections::BTreeMap;
use std::ops::RangeInclusive;

/// An edge a drag moves, by what it sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Edge {
    /// The left column's width, as a share of the middle's width.
    Left,
    /// The right column's width, as a share of the middle's width.
    Right,
    /// The bottom panel's height under its bar, as a share of the
    /// middle's height.
    Bottom,
    /// The upper part's share of the left column, when two are stacked.
    LeftShare,
    /// The upper part's share of the right column, when two are stacked.
    RightShare,
    /// The first editor group's share of the centre, when there are two.
    Split,
    /// The placing column's width beside the Library, in points.
    Placement,
}

/// The largest any size in points may be saved as: far more than any
/// screen, so a value past it can only be damage.
pub const SIZE_MAX: f32 = 8_000.0;

impl Edge {
    /// Every edge.
    pub const ALL: [Self; 7] = [
        Self::Left,
        Self::Right,
        Self::Bottom,
        Self::LeftShare,
        Self::RightShare,
        Self::Split,
        Self::Placement,
    ];

    /// What it is, as saved. Not the keys of decision 0109, whose sizes
    /// were points (`saved.rs` reads those).
    pub fn key(self) -> &'static str {
        match self {
            Self::Left => "left width",
            Self::Right => "right width",
            Self::Bottom => "bottom height",
            Self::LeftShare => "left share",
            Self::RightShare => "right share",
            Self::Split => "split",
            Self::Placement => "placement",
        }
    }

    /// The edge saved as `key`, if this build knows it.
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|e| e.key() == key)
    }

    /// Whether it is one of the frame's own edges, which every tab shares
    /// while tabs don't keep their own sizes. The split and the placing
    /// column belong to what a tab shows, so each keeps its own.
    pub fn is_frame(self) -> bool {
        !matches!(self, Self::Split | Self::Placement)
    }

    /// Its standard value on `tab` in `layout`: the board's, or Solis's
    /// own where the board has none.
    pub fn standard(self, tab: RibbonTab, layout: PanelLayout) -> f32 {
        use PanelLayout as L;
        match self {
            Self::Left if layout == L::LibraryLeft => size::LEFT_SHARE_LIBRARY,
            Self::Left => size::LEFT_SHARE,
            Self::Right if layout == L::TallProperties => size::RIGHT_SHARE_TALL,
            Self::Right if layout == L::WideView => size::RIGHT_SHARE_WIDE,
            Self::Right => size::RIGHT_SHARE,
            Self::Bottom if tab == RibbonTab::Animate => size::BOTTOM_SHARE_ANIMATE,
            Self::Bottom => size::BOTTOM_SHARE,
            Self::LeftShare | Self::RightShare => size::SIDE_SHARE,
            Self::Split => match tab {
                RibbonTab::Library => size::SPLIT_LIBRARY,
                _ => size::SPLIT,
            },
            Self::Placement => size::PLACEMENT_COLUMN_W,
        }
    }

    /// The values it may take at all; an edge is held tighter still by
    /// the room there is when it's dragged.
    pub fn limits(self) -> RangeInclusive<f32> {
        match self {
            Self::Left | Self::Right => size::COLUMN_SHARE_LIMITS,
            Self::Bottom => size::BOTTOM_SHARE_LIMITS,
            Self::Split | Self::LeftShare | Self::RightShare => {
                super::workspaces::SPLIT_LIMIT..=1.0 - super::workspaces::SPLIT_LIMIT
            }
            Self::Placement => size::COLUMN_MIN_W..=SIZE_MAX,
        }
    }
}

/// Every edge that was dragged, by tab, layout and edge; and whether each
/// tab keeps its own (Preferences › Layout).
#[derive(Debug, Clone, PartialEq)]
pub struct Sizes {
    kept: BTreeMap<(RibbonTab, PanelLayout, Edge), f32>,
    /// Whether each tab keeps its own frame edges; off, they are kept
    /// once, under Home, for every tab.
    pub per_tab: bool,
}

impl Default for Sizes {
    fn default() -> Self {
        Self {
            kept: BTreeMap::new(),
            per_tab: true,
        }
    }
}

impl Sizes {
    /// The tab an edge of `tab` is kept under.
    fn kept_under(&self, tab: RibbonTab, edge: Edge) -> RibbonTab {
        if self.per_tab || !edge.is_frame() {
            tab
        } else {
            RibbonTab::Home
        }
    }

    /// `edge`'s value on `tab` in `layout`: as dragged, or its standard.
    pub fn get(&self, tab: RibbonTab, layout: PanelLayout, edge: Edge) -> f32 {
        let under = self.kept_under(tab, edge);
        self.kept
            .get(&(under, layout, edge))
            .copied()
            .unwrap_or_else(|| edge.standard(tab, layout))
    }

    /// Keeps `value` for `edge`, held to its limits.
    pub fn set(&mut self, tab: RibbonTab, layout: PanelLayout, edge: Edge, value: f32) {
        if !value.is_finite() {
            return;
        }
        let limits = edge.limits();
        let value = value.clamp(*limits.start(), *limits.end());
        let under = self.kept_under(tab, edge);
        self.kept.insert((under, layout, edge), value);
    }

    /// Forgets what `edge` was dragged to: it is its standard again.
    pub fn forget(&mut self, tab: RibbonTab, layout: PanelLayout, edge: Edge) {
        let under = self.kept_under(tab, edge);
        self.kept.remove(&(under, layout, edge));
    }

    /// Forgets every edge of `tab`, in every layout.
    pub fn forget_tab(&mut self, tab: RibbonTab) {
        self.kept.retain(|(t, _, _), _| *t != tab);
    }

    /// Forgets every edge of every tab in `layout` (Reset sizes).
    pub fn forget_layout(&mut self, layout: PanelLayout) {
        self.kept.retain(|(_, l, _), _| *l != layout);
    }

    /// Every edge kept, with its value.
    pub fn kept(&self) -> impl Iterator<Item = ((RibbonTab, PanelLayout, Edge), f32)> + '_ {
        self.kept.iter().map(|(key, value)| (*key, *value))
    }

    /// Takes in a value read from saved settings, if it is one this build
    /// can use: finite and inside its edge's limits. Returns whether it
    /// was taken.
    pub fn take_saved(
        &mut self,
        tab: RibbonTab,
        layout: PanelLayout,
        edge: Edge,
        value: f32,
    ) -> bool {
        let usable = value.is_finite() && edge.limits().contains(&value);
        if usable {
            self.kept.insert((tab, layout, edge), value);
        }
        usable
    }
}

#[cfg(test)]
#[path = "sizes_tests.rs"]
mod tests;
