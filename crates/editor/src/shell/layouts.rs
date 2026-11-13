//! The layouts of Home's middle (decision 0109), and where everything in
//! the middle goes, worked out from the room there is and the sizes the
//! tab remembers: plain arithmetic, never anything the panels hold.
//!
//! - [`PanelLayout`] is the choice in File › Preferences: the Area,
//!   Properties and Assets lists as columns of their own beside and under
//!   the editor groups (three ways), or on the shelf as the design draws
//!   them.
//! - `side` places the side columns, the center and the Assets strip,
//!   and `shelf_columns` the shelf's columns; each also gives the
//!   `Border`s between what it placed, which a drag moves.
//!
//! When the window is too narrow, things give way in this order: first
//! the center (down to its least width), then the right column, then the
//! left one (each down to its least); narrower still, the lists go back
//! on the shelf until there is room again (`least_width`). On the
//! shelf the column that takes what's left gives way first, then the
//! others from the right; narrower still, they go under each other.

use super::sizes::Edge;
use super::workspaces::{Column, Width};
use crate::panel::Panel;
use crate::tokens::size;
use egui::{Pos2, Rect, pos2};
use serde::{Deserialize, Serialize};
use std::ops::RangeInclusive;

/// Where Home's Area, Properties and Assets lists sit.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
pub enum PanelLayout {
    /// The Area list over Assets in one column at the left, Properties
    /// at the right.
    AreaAndAssetsLeft,
    /// The Area list at the left, Properties over Assets at the right.
    AreaLeftAssetsRight,
    /// All three side by side on the shelf's Entity tab, as the design
    /// draws them.
    Shelf,
    /// The Area list a tall column at the left, Properties one at the
    /// right, and Assets a wide strip under the groups: the standard.
    /// Last, so that a layout a later build adds reads as this one.
    #[default]
    #[serde(other)]
    AreaLeftAssetsBottom,
}

impl PanelLayout {
    /// Every layout, in the list's order.
    pub const ALL: [Self; 4] = [
        Self::AreaLeftAssetsBottom,
        Self::AreaAndAssetsLeft,
        Self::AreaLeftAssetsRight,
        Self::Shelf,
    ];

    /// Its words in the list.
    pub fn name(self) -> &'static str {
        match self {
            Self::AreaLeftAssetsBottom => "Area left, Assets bottom",
            Self::AreaAndAssetsLeft => "Area and Assets on the left",
            Self::AreaLeftAssetsRight => "Area left, Assets right",
            Self::Shelf => "The shelf (as designed)",
        }
    }

    /// What it is, as saved among the sizes.
    pub fn key(self) -> &'static str {
        match self {
            Self::AreaLeftAssetsBottom => "area left assets bottom",
            Self::AreaAndAssetsLeft => "area and assets left",
            Self::AreaLeftAssetsRight => "area left assets right",
            Self::Shelf => "shelf",
        }
    }

    /// The layout saved as `key`, if this build knows it.
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|layout| layout.key() == key)
    }

    /// Whether the lists are columns of their own, off the shelf.
    pub fn is_side(self) -> bool {
        self != Self::Shelf
    }

    /// The lists in the left column, top to bottom.
    pub fn left(self) -> &'static [Column] {
        match self {
            Self::AreaAndAssetsLeft => &[AREA, ASSETS],
            Self::Shelf => &[],
            _ => &[AREA],
        }
    }

    /// The lists in the right column, top to bottom.
    pub fn right(self) -> &'static [Column] {
        match self {
            Self::AreaLeftAssetsRight => &[PROPERTIES, ASSETS],
            Self::Shelf => &[],
            _ => &[PROPERTIES],
        }
    }

    /// The list in the strip under the groups, if there is one.
    pub fn strip(self) -> Option<Column> {
        (self == Self::AreaLeftAssetsBottom).then_some(ASSETS)
    }

    /// Whether `panel` is in a column of its own (or the strip) here.
    pub fn has_beside(self, panel: Panel) -> bool {
        self.left()
            .iter()
            .chain(self.right())
            .chain(self.strip().as_ref())
            .any(|column| column.panels.contains(&panel))
    }
}

/// The Area list as a column of its own.
const AREA: Column = Column {
    title: "Area",
    panels: &[Panel::Area],
    width: Width::Rest,
    own_header: false,
};

/// Properties as a column of its own: it draws its own header.
const PROPERTIES: Column = Column {
    title: "Properties",
    panels: &[Panel::Properties],
    width: Width::Rest,
    own_header: true,
};

/// The Assets list as a column of its own, or the strip.
const ASSETS: Column = Column {
    title: "Assets",
    panels: &[Panel::Assets],
    width: Width::Rest,
    own_header: false,
};

/// How a pointer's place becomes a border's value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Measure {
    /// The distance from this x to the pointer: a width growing right.
    FromLeft(f32),
    /// The distance from the pointer to this x: a width growing left.
    FromRight(f32),
    /// The distance from the pointer to this y: a height growing up.
    FromBottom(f32),
    /// The pointer's share of the way down from `top`, `height` tall.
    ShareDown { top: f32, height: f32 },
}

impl Measure {
    /// The value at `at`.
    pub(crate) fn value(self, at: Pos2) -> f32 {
        match self {
            Self::FromLeft(x) => at.x - x,
            Self::FromRight(x) => x - at.x,
            Self::FromBottom(y) => y - at.y,
            Self::ShareDown { top, height } => (at.y - top) / height.max(1.0),
        }
    }

    /// Whether the border runs up and down (dragged sideways).
    pub(crate) fn upright(self) -> bool {
        matches!(self, Self::FromLeft(_) | Self::FromRight(_))
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
}

impl Border {
    /// Its name, for screen readers and tests: "Between Area and
    /// Properties", or an edge's own name ("Left column's edge").
    pub(crate) fn name(&self) -> String {
        match self.between {
            (edge, "") => edge.to_owned(),
            (first, second) => format!("Between {first} and {second}"),
        }
    }
}

/// One list placed: the column it's drawn as, and where.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Placed {
    /// The column (its header and panel).
    pub(crate) column: Column,
    /// Where, header and all.
    pub(crate) rect: Rect,
}

/// Where everything goes above the shelf in a side layout.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Side {
    /// The editor groups.
    pub(crate) center: Rect,
    /// The lists, in the side columns and the strip.
    pub(crate) lists: Vec<Placed>,
    /// The borders between them.
    pub(crate) borders: Vec<Border>,
}

/// The least width a side layout needs: the center and both columns at
/// their least, with the lines between. Narrower, the lists go back on
/// the shelf.
pub(crate) fn least_width(layout: PanelLayout) -> f32 {
    if layout.is_side() {
        2.0 * size::COLUMN_MIN_W + 2.0 + size::CENTER_MIN_W
    } else {
        0.0
    }
}

/// The room the Assets strip needs under the center at least, line and
/// all, which the shelf leaves when it is dragged up.
pub(crate) fn strip_reserve(layout: PanelLayout) -> f32 {
    if layout.strip().is_some() {
        size::PART_MIN_H + 1.0
    } else {
        0.0
    }
}

/// Places `layout`'s columns, center and strip in `upper` (the middle
/// above the shelf), with the sizes `size` gives for each edge.
pub(crate) fn side(upper: Rect, layout: PanelLayout, size: impl Fn(Edge) -> f32) -> Side {
    let least = size::COLUMN_MIN_W;
    let room = (upper.width() - 2.0 - size::CENTER_MIN_W).max(2.0 * least);
    let (mut left, mut right) = (size(Edge::Left).max(least), size(Edge::Right).max(least));
    // The center gave way first; now the right column, then the left.
    let mut excess = left + right - room;
    for width in [&mut right, &mut left] {
        let cut = excess.min(*width - least).max(0.0);
        *width -= cut;
        excess -= cut;
    }
    let (left, right) = (left.round(), right.round());
    let left_rect = upper.with_max_x(upper.left() + left);
    let right_rect = upper.with_min_x(upper.right() - right);
    let middle = Rect::from_x_y_ranges(
        left_rect.right() + 1.0..=right_rect.left() - 1.0,
        upper.y_range(),
    );
    let mut borders = vec![
        Border {
            edge: Edge::Left,
            line: Rect::from_x_y_ranges(left_rect.right()..=middle.left(), upper.y_range()),
            measure: Measure::FromLeft(upper.left()),
            range: least..=(room - right).max(least),
            between: ("Left column's edge", ""),
        },
        Border {
            edge: Edge::Right,
            line: Rect::from_x_y_ranges(middle.right()..=right_rect.left(), upper.y_range()),
            measure: Measure::FromRight(upper.right()),
            range: least..=(room - left).max(least),
            between: ("Right column's edge", ""),
        },
    ];
    let mut lists = Vec::new();
    for (rect, columns, edge) in [
        (left_rect, layout.left(), Edge::LeftShare),
        (right_rect, layout.right(), Edge::RightShare),
    ] {
        stack(rect, columns, (edge, size(edge)), &mut lists, &mut borders);
    }
    let mut center = middle;
    if let Some(strip) = layout.strip() {
        let most = (middle.height() - 1.0 - size::CENTER_MIN_H).max(0.0);
        let height = size(Edge::Strip).min(most).round();
        let rect = middle.with_min_y(middle.bottom() - height);
        center = middle.with_max_y(rect.top() - 1.0);
        borders.push(Border {
            edge: Edge::Strip,
            line: Rect::from_x_y_ranges(middle.x_range(), center.bottom()..=rect.top()),
            measure: Measure::FromBottom(middle.bottom()),
            range: size::PART_MIN_H..=most.max(size::PART_MIN_H),
            between: ("Assets strip's edge", ""),
        });
        lists.push(Placed {
            column: strip,
            rect,
        });
    }
    Side {
        center,
        lists,
        borders,
    }
}

/// Places `columns` in `rect`, one over the other: the first takes
/// `share` of the height (each keeping its least where there is room).
fn stack(
    rect: Rect,
    columns: &[Column],
    (edge, share): (Edge, f32),
    lists: &mut Vec<Placed>,
    borders: &mut Vec<Border>,
) {
    match columns {
        [one] => lists.push(Placed { column: *one, rect }),
        [upper, lower] => {
            let height = rect.height();
            let part = size::PART_MIN_H;
            let limits = edge.limits();
            let (mut low, mut high) = (*limits.start(), *limits.end());
            if height >= 2.0 * part + 1.0 {
                low = low.max(part / height);
                high = high.min(1.0 - (part + 1.0) / height);
            }
            let y = (rect.top() + height * share.max(low).min(high)).round();
            lists.push(Placed {
                column: *upper,
                rect: rect.with_max_y(y),
            });
            lists.push(Placed {
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
            });
        }
        _ => {}
    }
}

/// Places a shelf tab's `columns` in `content`: side by side 1 px apart,
/// each with a width of its own at `widths[i]` (what the tab remembers)
/// and the rest sharing what's left; or, when even their least widths
/// don't fit, each under the one before. Gives the borders a drag moves,
/// for `edge_of(i)` naming column `i`'s width.
pub(crate) fn shelf_columns(
    content: Rect,
    columns: &[Column],
    widths: &[f32],
    edge_of: impl Fn(usize) -> Edge,
) -> (Vec<Rect>, Vec<Border>) {
    let least = size::COLUMN_MIN_W;
    let n = columns.len();
    let gaps = n.saturating_sub(1) as f32;
    let fixed = |i: usize| matches!(columns[i].width, Width::Fixed(_));
    let rests = (0..n).filter(|i| !fixed(*i)).count();
    let mut width: Vec<f32> = (0..n)
        .map(|i| {
            if fixed(i) {
                widths.get(i).copied().unwrap_or(least).max(least)
            } else {
                0.0
            }
        })
        .collect();
    let room = content.width() - gaps - least * rests as f32;
    let mut excess: f32 = width.iter().sum::<f32>() - room;
    // What's left gave way first; now the fixed ones, from the right.
    for i in (0..n).rev().filter(|i| fixed(*i)) {
        let cut = excess.min(width[i] - least).max(0.0);
        width[i] -= cut;
        excess -= cut;
    }
    if excess > 0.5 && n > 1 {
        let each = ((content.height() - gaps) / n as f32).floor();
        let rects = (0..n)
            .map(|i| {
                let top = content.top() + i as f32 * (each + 1.0);
                Rect::from_min_size(pos2(content.left(), top), egui::vec2(content.width(), each))
            })
            .collect();
        return (rects, Vec::new());
    }
    let fixed_sum: f32 = width.iter().sum();
    let rest = ((content.width() - gaps - fixed_sum) / rests.max(1) as f32).max(0.0);
    let mut x = content.left();
    let mut rects = Vec::with_capacity(n);
    for (i, own) in width.iter().enumerate() {
        let w = if fixed(i) { *own } else { rest };
        let right = if i + 1 == n {
            content.right()
        } else {
            (x + w).round()
        };
        rects.push(Rect::from_x_y_ranges(x..=right, content.y_range()));
        x = right + 1.0;
    }
    let mut borders = Vec::new();
    for i in 0..n.saturating_sub(1) {
        let (left, right) = (rects[i], rects[i + 1]);
        let line = Rect::from_x_y_ranges(left.right()..=right.left(), content.y_range());
        // The fixed column beside the line is the one it sizes; it may
        // grow until what's left is at its least.
        let (at, measure) = if fixed(i) {
            (i, Measure::FromLeft(left.left()))
        } else if fixed(i + 1) {
            (i + 1, Measure::FromRight(right.right()))
        } else {
            continue;
        };
        let spare = (rest - least).max(0.0) * if rests > 0 { 1.0 } else { 0.0 };
        borders.push(Border {
            edge: edge_of(at),
            line,
            measure,
            range: least..=(width[at] + spare).max(least),
            between: (columns[i].title, columns[i + 1].title),
        });
    }
    (rects, borders)
}

#[cfg(test)]
#[path = "layouts_tests.rs"]
mod tests;
