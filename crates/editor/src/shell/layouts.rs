//! The layouts of the frame (decision 0118, boards 06 and 09), and where
//! everything in the middle goes, worked out from the room there is and
//! the sizes the tab remembers: plain arithmetic, never anything the
//! panels hold.
//!
//! - [`PanelLayout`] is the choice in File › Preferences › Layout: where
//!   the slots sit round the centre, the same on every tab.
//! - `place` puts the side columns, the centre and the bottom panel in
//!   the middle, and gives the `Border`s between them, which a drag moves.
//!
//! The columns' and the bottom panel's sizes are shares of the middle, so
//! a window made larger or smaller keeps them in proportion. Each has a
//! least size in points: when the window is too narrow, the centre gives
//! way first (down to its least width), then the right column, then the
//! left one (each down to its least); narrower still, the right column
//! goes, then the left, until there is room again.

use super::sizes::Edge;
use super::workspaces::{Column, Slot};
use crate::tokens::size;
use egui::{Pos2, Rect};
use serde::{Deserialize, Serialize};
use std::ops::RangeInclusive;

/// Where the slots sit round the centre.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
pub enum PanelLayout {
    /// The left and right slots share the right; the view gets the width.
    WideView,
    /// The standard, with two views side by side in the centre.
    TwoViews,
    /// The Library under the left slot, in a tall list; the view runs to
    /// the foot. Decision 0109's "Level and Assets on the left".
    #[serde(rename = "LevelAndAssetsLeft", alias = "AreaAndAssetsLeft")]
    LibraryLeft,
    /// The Library under the right slot. Decision 0109's "Level left,
    /// Assets right".
    #[serde(rename = "LevelLeftAssetsRight", alias = "AreaLeftAssetsRight")]
    LibraryRight,
    /// The right slot full height and wider; the bottom panel runs under
    /// the left slot too.
    TallProperties,
    /// Every slot hidden; the bottom line stays.
    JustTheView,
    /// The left slot at the left, the right slot at the right, the
    /// Library under the view: the standard. Last, as
    /// what reads anything unknown must be. Saved under its name from
    /// decision 0109, so builds before the frame read it; any layout
    /// this build doesn't know (one a later build adds, or the shelf of
    /// decision 0109) reads as this one.
    #[default]
    #[serde(
        rename = "LevelLeftAssetsBottom",
        alias = "AreaLeftAssetsBottom",
        other
    )]
    Standard,
}

/// Where a layout puts the Library, on the tabs that have it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryAt {
    /// A tab of the bottom panel.
    Bottom,
    /// Under the left slot, in the left column.
    UnderLeft,
    /// Under the right slot, in the right column.
    UnderRight,
}

impl PanelLayout {
    /// Every layout, in the order Preferences shows them.
    pub const ALL: [Self; 7] = [
        Self::Standard,
        Self::WideView,
        Self::TwoViews,
        Self::LibraryLeft,
        Self::TallProperties,
        Self::JustTheView,
        Self::LibraryRight,
    ];

    /// Its name, on its card.
    pub fn name(self) -> &'static str {
        match self {
            Self::Standard => "Standard",
            Self::WideView => "Wide view",
            Self::TwoViews => "Two views",
            Self::LibraryLeft => "Library on the left",
            Self::LibraryRight => "Library on the right",
            Self::TallProperties => "Tall properties",
            Self::JustTheView => "Just the view",
        }
    }

    /// One line saying what it does, under its name.
    pub fn line(self) -> &'static str {
        match self {
            Self::Standard => "Level left, Properties right, Library under the view.",
            Self::WideView => "Level and Properties share the right; the view gets the width.",
            Self::TwoViews => "Two views side by side, say from above and in perspective.",
            Self::LibraryLeft => "Library under Level, in a tall list; the view runs to the foot.",
            Self::LibraryRight => "Library under Properties; the view runs to the foot.",
            Self::TallProperties => {
                "Properties full height and wider; Library runs under Level too."
            }
            Self::JustTheView => "Every panel hidden; the bottom line stays.",
        }
    }

    /// What its preview says it does, at more length.
    pub fn more(self) -> &'static str {
        match self {
            Self::Standard => {
                "Level sits at the left and Properties at the right, both full height; the \
                 Library runs under the view. Every edge still drags."
            }
            Self::WideView => {
                "The view gets the whole width. Level sits over Properties on the right; the \
                 Library runs under the view. Every edge still drags."
            }
            Self::TwoViews => {
                "The standard, with the view split in two side by side, each with its own \
                 camera. The view's menu goes back to one."
            }
            Self::LibraryLeft => {
                "The Library joins Level in the left column, as a tall list; the view runs \
                 down to the bottom line. Every edge still drags."
            }
            Self::LibraryRight => {
                "The Library joins Properties in the right column; the view runs down to the \
                 bottom line. Every edge still drags."
            }
            Self::TallProperties => {
                "Properties runs full height and wider at the right; the bottom panel runs \
                 under Level as well as the view."
            }
            Self::JustTheView => {
                "Every panel is hidden and the bottom panel folded to its line. A panel's \
                 name in the bottom line brings it back."
            }
        }
    }

    /// What it is, as saved among the sizes. The three from decision 0109
    /// keep their keys, so their sizes carry over.
    pub fn key(self) -> &'static str {
        match self {
            Self::Standard => "level left assets bottom",
            Self::WideView => "wide view",
            Self::TwoViews => "two views",
            Self::LibraryLeft => "level and assets left",
            Self::LibraryRight => "level left assets right",
            Self::TallProperties => "tall properties",
            Self::JustTheView => "just the view",
        }
    }

    /// The layout saved as `key`, if this build knows it: its own key, or
    /// one from before (decision 0109's area and shelf layouts read as
    /// their nearest).
    pub fn from_key(key: &str) -> Option<Self> {
        let old = match key {
            "area left assets bottom" | "shelf" => Some(Self::Standard),
            "area and assets left" => Some(Self::LibraryLeft),
            "area left assets right" => Some(Self::LibraryRight),
            _ => None,
        };
        old.or_else(|| Self::ALL.into_iter().find(|layout| layout.key() == key))
    }

    /// Where the Library goes.
    pub fn library_at(self) -> LibraryAt {
        match self {
            Self::LibraryLeft => LibraryAt::UnderLeft,
            Self::LibraryRight => LibraryAt::UnderRight,
            _ => LibraryAt::Bottom,
        }
    }

    /// Whether the Library is a column of its own beside the centre.
    pub fn library_beside(self) -> bool {
        self.library_at() != LibraryAt::Bottom
    }

    /// Whether the left slot sits over the right slot, in the right
    /// column.
    pub fn left_on_right(self) -> bool {
        self == Self::WideView
    }

    /// Whether the bottom panel runs under the left column as well as
    /// the centre.
    pub fn bottom_under_left(self) -> bool {
        self == Self::TallProperties
    }

    /// Whether choosing it hides every slot and folds every bottom panel.
    pub fn hides_everything(self) -> bool {
        self == Self::JustTheView
    }

    /// Whether choosing it puts two views side by side in the centre.
    pub fn two_views(self) -> bool {
        self == Self::TwoViews
    }
}

/// How a pointer's place becomes an edge's value: a distance in points,
/// over `span` (the room it is a share of, or 1 for points).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Measure {
    /// From this x to the pointer: a width growing right.
    FromLeft { x: f32, span: f32 },
    /// From the pointer to this x: a width growing left.
    FromRight { x: f32, span: f32 },
    /// From the pointer to this y: a height growing up.
    FromBottom { y: f32, span: f32 },
    /// The pointer's share of the way down from `top`, `height` tall.
    ShareDown { top: f32, height: f32 },
}

impl Measure {
    /// The value at `at`.
    pub(crate) fn value(self, at: Pos2) -> f32 {
        match self {
            Self::FromLeft { x, span } => (at.x - x) / span.max(1.0),
            Self::FromRight { x, span } => (x - at.x) / span.max(1.0),
            Self::FromBottom { y, span } => (y - at.y) / span.max(1.0),
            Self::ShareDown { top, height } => (at.y - top) / height.max(1.0),
        }
    }

    /// Whether the border runs up and down (dragged sideways).
    pub(crate) fn upright(self) -> bool {
        matches!(self, Self::FromLeft { .. } | Self::FromRight { .. })
    }
}

/// A line between two things that a drag moves.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Border {
    /// What it sets.
    pub(crate) edge: Edge,
    /// The 1 px line, between the two things.
    pub(crate) line: Rect,
    /// How the pointer's place becomes the edge's value.
    pub(crate) measure: Measure,
    /// What it may be dragged to here: its limits, held tighter by the
    /// room the rest needs.
    pub(crate) range: RangeInclusive<f32>,
    /// What it is between, for its name (`Border::name`): made into
    /// words only when a screen reader or a test asks.
    pub(crate) between: (&'static str, &'static str),
    /// Whether it carries a grip at its middle (the frame's three accent
    /// edges, board 03).
    pub(crate) grip: bool,
}

impl Border {
    /// Its name, for screen readers and tests: "Between Level and
    /// Properties", or an edge's own name ("Left column's edge").
    pub(crate) fn name(&self) -> String {
        match self.between {
            (edge, "") => edge.to_owned(),
            (first, second) => format!("Between {first} and {second}"),
        }
    }
}

/// One part of a side column: the slot it fills and what it shows.
pub(crate) type Part = (Slot, Column);

/// One column placed: what it shows, and where.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Placed {
    /// The slot it fills.
    pub(crate) slot: Slot,
    /// The column (its header and panels).
    pub(crate) column: Column,
    /// Where, header and all.
    pub(crate) rect: Rect,
}

/// What the frame holds on the tab in front, for `place`.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Parts {
    /// The left column's parts, top to bottom (one or two).
    pub(crate) left: Vec<Part>,
    /// The right column's parts, top to bottom (one or two).
    pub(crate) right: Vec<Part>,
    /// The bottom panel, if the tab has one: whether it is folded.
    pub(crate) bottom: Option<bool>,
    /// Whether the bottom panel runs under the left column too.
    pub(crate) bottom_under_left: bool,
}

/// Where everything goes in the middle.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Frame {
    /// The editor groups.
    pub(crate) centre: Rect,
    /// The bottom panel, its bar and all (only its bar while folded).
    pub(crate) bottom: Option<Rect>,
    /// The side columns' parts.
    pub(crate) columns: Vec<Placed>,
    /// The borders between them.
    pub(crate) borders: Vec<Border>,
}

/// The columns' widths in a middle `width` wide, from what each wants
/// (`None`: the column isn't there), given way as the module says.
fn fit_columns(width: f32, mut wanted: [Option<f32>; 2]) -> [Option<f32>; 2] {
    let least = size::COLUMN_MIN_W;
    loop {
        let lines = wanted.iter().flatten().count() as f32;
        let room = width - size::CENTER_MIN_W - lines;
        let mut excess = wanted.iter().flatten().sum::<f32>() - room;
        // The centre gave way first; now the right column, then the left.
        for at in [1, 0] {
            if let Some(w) = wanted[at].as_mut() {
                let cut = excess.min(*w - least).max(0.0);
                *w -= cut;
                excess -= cut;
            }
        }
        if excess <= 0.5 {
            return wanted;
        }
        // Narrower still: the right column goes, then the left.
        match wanted {
            [_, Some(_)] => wanted[1] = None,
            [Some(_), None] => wanted[0] = None,
            [None, None] => return wanted,
        }
    }
}

/// Places `parts` in `middle` (all of the window between the ribbon and
/// the status line), with the sizes `size` gives for each edge.
pub(crate) fn place(middle: Rect, parts: &Parts, size: impl Fn(Edge) -> f32) -> Frame {
    let (w, h) = (middle.width().max(1.0), middle.height().max(1.0));
    let wants = |columns: &[Part], edge| {
        (!columns.is_empty()).then(|| (size(edge) * w).round().max(size::COLUMN_MIN_W))
    };
    let [left, right] = fit_columns(
        w,
        [
            wants(&parts.left, Edge::Left),
            wants(&parts.right, Edge::Right),
        ],
    );
    let (lw, rw) = (left.unwrap_or(0.0).round(), right.unwrap_or(0.0).round());
    let inner_left = middle.left() + if left.is_some() { lw + 1.0 } else { 0.0 };
    let inner_right = middle.right() - if right.is_some() { rw + 1.0 } else { 0.0 };
    let bottom_h = parts.bottom.map(|folded| {
        let height = if folded {
            size::BOTTOM_BAR_H
        } else {
            let most = (h - size::BOTTOM_BAR_H - size::CENTER_MIN_H).max(size::PART_MIN_H);
            size::BOTTOM_BAR_H
                + (size(Edge::Bottom) * h)
                    .clamp(size::PART_MIN_H, most)
                    .round()
        };
        // Never more than there is: a window too short for it all keeps
        // the bar.
        height.min(h)
    });
    let bottom_top = middle.bottom() - bottom_h.unwrap_or(0.0);
    let under_left = parts.bottom_under_left && bottom_h.is_some();
    let left_rect = Rect::from_x_y_ranges(
        middle.left()..=middle.left() + lw,
        middle.top()..=if under_left {
            bottom_top - 1.0
        } else {
            middle.bottom()
        },
    );
    let right_rect = Rect::from_x_y_ranges(middle.right() - rw..=middle.right(), middle.y_range());
    let centre = Rect::from_x_y_ranges(inner_left..=inner_right, middle.top()..=bottom_top);
    let bottom = bottom_h.map(|_| {
        let from = if under_left {
            middle.left()
        } else {
            inner_left
        };
        Rect::from_x_y_ranges(from..=inner_right, bottom_top..=middle.bottom())
    });
    let least = size::COLUMN_MIN_W;
    let room = w - size::CENTER_MIN_W - 2.0;
    let mut borders = Vec::new();
    if left.is_some() {
        borders.push(Border {
            edge: Edge::Left,
            line: Rect::from_x_y_ranges(left_rect.right()..=inner_left, left_rect.y_range()),
            measure: Measure::FromLeft {
                x: middle.left(),
                span: w,
            },
            range: least / w..=((room - rw).max(least)) / w,
            between: ("Left column's edge", ""),
            grip: true,
        });
    }
    if right.is_some() {
        borders.push(Border {
            edge: Edge::Right,
            line: Rect::from_x_y_ranges(inner_right..=right_rect.left(), middle.y_range()),
            measure: Measure::FromRight {
                x: middle.right(),
                span: w,
            },
            range: least / w..=((room - lw).max(least)) / w,
            between: ("Right column's edge", ""),
            grip: true,
        });
    }
    if let (Some(rect), Some(false)) = (bottom, parts.bottom) {
        let most = (h - size::BOTTOM_BAR_H - size::CENTER_MIN_H).max(size::PART_MIN_H);
        borders.push(Border {
            edge: Edge::Bottom,
            line: Rect::from_x_y_ranges(rect.x_range(), rect.top()..=rect.top() + 1.0),
            measure: Measure::FromBottom {
                y: middle.bottom() - size::BOTTOM_BAR_H,
                span: h,
            },
            range: size::PART_MIN_H / h..=most / h,
            between: ("Bottom panel's edge", ""),
            grip: true,
        });
    }
    let mut columns = Vec::new();
    for (rect, parts, edge, there) in [
        (left_rect, &parts.left, Edge::LeftShare, left.is_some()),
        (right_rect, &parts.right, Edge::RightShare, right.is_some()),
    ] {
        if there {
            stack(rect, parts, (edge, size(edge)), &mut columns, &mut borders);
        }
    }
    Frame {
        centre,
        bottom,
        columns,
        borders,
    }
}

/// Places `parts` in `rect`, one over the other: the first takes
/// `share` of the height (each keeping its least where there is room).
fn stack(
    rect: Rect,
    parts: &[Part],
    (edge, share): (Edge, f32),
    placed: &mut Vec<Placed>,
    borders: &mut Vec<Border>,
) {
    match parts {
        [(slot, column)] => placed.push(Placed {
            slot: *slot,
            column: *column,
            rect,
        }),
        [(upper_slot, upper), (lower_slot, lower), ..] => {
            let height = rect.height();
            let part = size::PART_MIN_H;
            let limits = edge.limits();
            let (mut low, mut high) = (*limits.start(), *limits.end());
            if height >= 2.0 * part + 1.0 {
                low = low.max(part / height);
                high = high.min(1.0 - (part + 1.0) / height);
            }
            let y = (rect.top() + height * share.max(low).min(high)).round();
            placed.push(Placed {
                slot: *upper_slot,
                column: *upper,
                rect: rect.with_max_y(y),
            });
            placed.push(Placed {
                slot: *lower_slot,
                column: *lower,
                rect: rect.with_min_y(y + 1.0),
            });
            borders.push(Border {
                edge,
                line: Rect::from_x_y_ranges(rect.x_range(), y..=y + 1.0),
                measure: Measure::ShareDown {
                    top: rect.top(),
                    height,
                },
                range: low..=high,
                between: (upper.title, lower.title),
                grip: false,
            });
        }
        [] => {}
    }
}

#[cfg(test)]
#[path = "layouts_tests.rs"]
mod tests;
