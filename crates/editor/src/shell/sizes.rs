//! What each tab remembers of its borders (decision 0109): every edge a
//! drag moves (the shelf's top, the line between the groups, the side
//! columns' edges, the Assets strip's top, the line between two lists
//! stacked in a column, the lines between the shelf's columns), one plain
//! number each, kept per tab and per layout.
//!
//! Only what was dragged is kept; anything else is its standard, and a
//! double-click on a border forgets it again. Looking one up is a map
//! lookup, so the frame works out its layout from a handful of numbers
//! and never from what the panels hold.

use super::RibbonTab;
use super::layouts::PanelLayout;
use super::workspaces::{ShelfTab, Width};
use crate::tokens::size;
use std::collections::BTreeMap;
use std::ops::RangeInclusive;

/// A border a drag moves, by what it sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Edge {
    /// The shelf's content height under its bar, in points.
    ShelfHeight,
    /// The first editor group's share of the center, when there are two.
    Split,
    /// The left column's width, in points.
    Left,
    /// The right column's width, in points.
    Right,
    /// The Assets strip's height under the center, header and all.
    Strip,
    /// The upper list's share of the left column, when two are stacked.
    LeftShare,
    /// The upper list's share of the right column, when two are stacked.
    RightShare,
    /// The width of a shelf tab's column (by its place, from the left),
    /// one of those with a width of their own.
    Column(ShelfTab, u8),
}

/// The largest any size in points may be saved as: far more than any
/// screen, so a value past it can only be damage.
pub const SIZE_MAX: f32 = 8_000.0;

impl Edge {
    /// What it is, as saved.
    pub fn key(self) -> String {
        match self {
            Self::ShelfHeight => "shelf".into(),
            Self::Split => "split".into(),
            Self::Left => "left".into(),
            Self::Right => "right".into(),
            Self::Strip => "strip".into(),
            Self::LeftShare => "left share".into(),
            Self::RightShare => "right share".into(),
            Self::Column(tab, index) => format!("column {} {index}", tab.name()),
        }
    }

    /// The edge saved as `key`, if this build knows it.
    pub fn from_key(key: &str) -> Option<Self> {
        let fixed = [
            Self::ShelfHeight,
            Self::Split,
            Self::Left,
            Self::Right,
            Self::Strip,
            Self::LeftShare,
            Self::RightShare,
        ];
        if let Some(edge) = fixed.into_iter().find(|e| e.key() == key) {
            return Some(edge);
        }
        let rest = key.strip_prefix("column ")?;
        let (name, index) = rest.rsplit_once(' ')?;
        let tab = ShelfTab::named(name)?;
        let index: u8 = index.parse().ok()?;
        let fixed_width = matches!(
            tab.columns(PanelLayout::Shelf).get(usize::from(index)),
            Some(column) if matches!(column.width, Width::Fixed(_))
        );
        fixed_width.then_some(Self::Column(tab, index))
    }

    /// Its standard value on `tab` in `layout`: the design's, or Solis's
    /// own where the design has none.
    pub fn standard(self, tab: RibbonTab, layout: PanelLayout) -> f32 {
        match self {
            Self::ShelfHeight => match tab {
                RibbonTab::Animate => size::SHELF_H_ANIMATE,
                RibbonTab::Home if layout.is_side() => size::SHELF_H_SIDE,
                _ => size::SHELF_H,
            },
            Self::Split => match tab {
                RibbonTab::Library => size::SPLIT_LIBRARY,
                RibbonTab::Audio => size::SPLIT_AUDIO,
                _ => size::SPLIT,
            },
            Self::Left if layout == PanelLayout::AreaAndAssetsLeft => size::ASSETS_COLUMN_W,
            Self::Left => size::SCENE_COLUMN_W,
            Self::Right => size::PROPERTIES_COLUMN_W,
            Self::Strip => size::ASSETS_STRIP_H,
            Self::LeftShare | Self::RightShare => size::SIDE_SHARE,
            Self::Column(shelf, index) => match shelf.columns(layout).get(usize::from(index)) {
                Some(column) => match column.width {
                    Width::Fixed(w) => w,
                    Width::Rest => size::COLUMN_MIN_W,
                },
                None => size::COLUMN_MIN_W,
            },
        }
    }

    /// The values it may take at all; a border is held tighter still by
    /// the room there is when it's dragged.
    pub fn limits(self) -> RangeInclusive<f32> {
        match self {
            Self::ShelfHeight => size::SHELF_MIN_H..=SIZE_MAX,
            Self::Split | Self::LeftShare | Self::RightShare => {
                super::workspaces::SPLIT_LIMIT..=1.0 - super::workspaces::SPLIT_LIMIT
            }
            Self::Left | Self::Right | Self::Column(..) => size::COLUMN_MIN_W..=SIZE_MAX,
            Self::Strip => size::PART_MIN_H..=SIZE_MAX,
        }
    }

    /// Whether it is a share of the room rather than a size in points.
    pub fn is_share(self) -> bool {
        matches!(self, Self::Split | Self::LeftShare | Self::RightShare)
    }
}

/// The layout a tab's sizes are kept under: Home's own middle changes
/// with the layout, so it keeps a set for each; every other tab looks the
/// same in all four, so it keeps one.
pub fn kept_under(tab: RibbonTab, layout: PanelLayout) -> PanelLayout {
    if tab == RibbonTab::Home {
        layout
    } else {
        PanelLayout::Shelf
    }
}

/// Every border that was dragged, by tab, layout and edge.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Sizes(BTreeMap<(RibbonTab, PanelLayout, Edge), f32>);

impl Sizes {
    /// `edge`'s value on `tab` in `layout`: as dragged, or its standard.
    pub fn get(&self, tab: RibbonTab, layout: PanelLayout, edge: Edge) -> f32 {
        let layout = kept_under(tab, layout);
        self.0
            .get(&(tab, layout, edge))
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
        self.0.insert((tab, kept_under(tab, layout), edge), value);
    }

    /// Forgets what `edge` was dragged to: it is its standard again.
    pub fn forget(&mut self, tab: RibbonTab, layout: PanelLayout, edge: Edge) {
        self.0.remove(&(tab, kept_under(tab, layout), edge));
    }

    /// Forgets every border of `tab`, in every layout.
    pub fn forget_tab(&mut self, tab: RibbonTab) {
        self.0.retain(|(t, _, _), _| *t != tab);
    }

    /// Every border kept, with its value.
    pub fn kept(&self) -> impl Iterator<Item = ((RibbonTab, PanelLayout, Edge), f32)> + '_ {
        self.0.iter().map(|(key, value)| (*key, *value))
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
            self.0.insert((tab, kept_under(tab, layout), edge), value);
        }
        usable
    }
}

#[cfg(test)]
#[path = "sizes_tests.rs"]
mod tests;
