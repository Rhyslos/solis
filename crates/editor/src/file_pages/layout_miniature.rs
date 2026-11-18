//! A layout drawn small (decision 0118, board 06): the frame as a
//! miniature, its tab bar and ribbon as a few marks, the view with a box
//! in it, the lists as lines, Properties as cards and the Library as
//! rows of coloured tiles. On a card it is plain; in the Preview box each
//! part carries its name.
//!
//! Drawn from shares of the rectangle, not from the frame's own sizes:
//! a miniature shows where the slots sit, not how many points wide.

use crate::shell::PanelLayout;
use crate::tokens::{self, size, text};
use egui::{Align2, Color32, CornerRadius, Painter, Pos2, Rect, Stroke, StrokeKind, pos2, vec2};

/// What fills one part of a miniature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fill {
    /// A list's lines: the Level panel.
    Lines,
    /// Cards: Properties.
    Cards,
    /// Rows of tiles: the Library.
    Tiles,
    /// The view, with a box in it.
    View,
}

/// One part of a miniature, in shares of the middle (left, top, right,
/// bottom), its fill, and its name in the Preview box.
type Part = ([f32; 4], Fill, &'static str);

/// The parts of `layout`, in shares of the middle.
fn parts(layout: PanelLayout) -> Vec<Part> {
    use Fill::{Cards, Lines, Tiles, View};
    use PanelLayout as L;
    let (l, r, b) = (0.2, 0.78, 0.66);
    match layout {
        L::Standard | L::TwoViews => vec![
            ([0.0, 0.0, l, 1.0], Lines, "Level"),
            ([l, 0.0, r, b], View, "View"),
            ([l, b, r, 1.0], Tiles, "Library"),
            ([r, 0.0, 1.0, 1.0], Cards, "Properties"),
        ],
        L::WideView => vec![
            ([0.0, 0.0, 0.72, b], View, "View"),
            ([0.0, b, 0.72, 1.0], Tiles, "Library"),
            ([0.72, 0.0, 1.0, 0.45], Lines, "Level"),
            ([0.72, 0.45, 1.0, 1.0], Cards, "Properties"),
        ],
        L::LibraryLeft => vec![
            ([0.0, 0.0, 0.22, 0.48], Lines, "Level"),
            ([0.0, 0.48, 0.22, 1.0], Lines, "Library"),
            ([0.22, 0.0, r, 0.94], View, "View"),
            ([r, 0.0, 1.0, 1.0], Cards, "Properties"),
        ],
        L::LibraryRight => vec![
            ([0.0, 0.0, l, 1.0], Lines, "Level"),
            ([l, 0.0, 0.76, 0.94], View, "View"),
            ([0.76, 0.0, 1.0, 0.52], Cards, "Properties"),
            ([0.76, 0.52, 1.0, 1.0], Lines, "Library"),
        ],
        L::TallProperties => vec![
            ([0.0, 0.0, 0.18, b], Lines, "Level"),
            ([0.18, 0.0, 0.7, b], View, "View"),
            ([0.0, b, 0.7, 1.0], Tiles, "Library"),
            ([0.7, 0.0, 1.0, 1.0], Cards, "Properties"),
        ],
        L::JustTheView => vec![([0.0, 0.0, 1.0, 0.94], View, "View")],
    }
}

/// Draws `layout` small in `rect`, with each part's name when `named`.
pub(super) fn miniature(painter: &Painter, rect: Rect, layout: PanelLayout, named: bool) {
    let round = CornerRadius::same(size::radius::SMALL);
    painter.rect_filled(rect, round, tokens::app());
    let inner = rect.shrink(1.0);
    let h = inner.height();
    // The tab bar's marks, the second lit, and the ribbon's.
    let tabs = inner.top() + h * 0.04;
    for i in 0..6 {
        let x = inner.left() + 6.0 + i as f32 * 12.0;
        let colour = if i == 2 {
            tokens::text_primary()
        } else {
            tokens::stronger_line()
        };
        let mark = Rect::from_min_size(pos2(x, tabs), vec2(8.0, 2.0));
        painter.rect_filled(mark, 0.0, colour);
    }
    let ribbon = Rect::from_x_y_ranges(
        inner.x_range(),
        inner.top() + h * 0.09..=inner.top() + h * 0.15,
    );
    painter.rect_filled(ribbon, 0.0, tokens::ribbon());
    for i in 0..8 {
        let x = ribbon.left() + 6.0 + i as f32 * 10.0;
        let mark = Rect::from_min_size(pos2(x, ribbon.center().y - 1.5), vec2(6.0, 3.0));
        painter.rect_filled(mark, 1.0, tokens::stronger_line());
    }
    let middle = Rect::from_x_y_ranges(inner.x_range(), ribbon.bottom() + 1.0..=inner.bottom());
    let at = |[a, b, c, d]: [f32; 4]| {
        Rect::from_min_max(
            pos2(
                middle.left() + middle.width() * a,
                middle.top() + middle.height() * b,
            ),
            pos2(
                middle.left() + middle.width() * c,
                middle.top() + middle.height() * d,
            ),
        )
        .shrink(0.5)
    };
    let mut names = Vec::new();
    for (share, fill, name) in parts(layout) {
        let part = at(share);
        draw_part(painter, part, fill, layout.two_views());
        names.push((part, fill, name));
    }
    if layout.library_beside() || layout == PanelLayout::JustTheView {
        // The bottom line that stays under the view.
        let line = Rect::from_x_y_ranges(
            at([0.0, 0.94, 1.0, 1.0]).x_range(),
            at([0.0, 0.94, 1.0, 1.0]).y_range(),
        );
        let under_view = names
            .iter()
            .find(|(_, fill, _)| *fill == Fill::View)
            .map_or(line, |(view, ..)| {
                line.with_min_x(view.left()).with_max_x(view.right())
            });
        painter.rect_filled(under_view, 0.0, tokens::shelf_bar());
        for i in 0..3 {
            let x = under_view.left() + 4.0 + i as f32 * 12.0;
            let mark = Rect::from_min_size(pos2(x, under_view.center().y - 1.0), vec2(8.0, 2.0));
            painter.rect_filled(mark, 0.0, tokens::stronger_line());
        }
    }
    if named {
        for (part, _, name) in names {
            label(painter, part.center(), name);
        }
    }
    painter.rect_stroke(
        rect,
        round,
        Stroke::new(1.0, tokens::card_border()),
        StrokeKind::Inside,
    );
}

/// One part's fill.
fn draw_part(painter: &Painter, part: Rect, fill: Fill, two_views: bool) {
    let background = match fill {
        Fill::View => tokens::viewport(),
        Fill::Tiles | Fill::Lines | Fill::Cards => tokens::panel(),
    };
    painter.rect_filled(part, 0.0, background);
    match fill {
        Fill::Lines => lines(painter, part),
        Fill::Cards => cards(painter, part),
        Fill::Tiles => tiles(painter, part),
        Fill::View if two_views => {
            let half = part.width() / 2.0;
            let left = part.with_max_x(part.left() + half - 1.0);
            let right = part.with_min_x(part.left() + half + 1.0);
            tokens::vline(painter, part.center().x, part.y_range(), tokens::line());
            for view in [left, right] {
                cube(painter, view);
            }
        }
        Fill::View => cube(painter, part),
    }
}

/// A list's lines, the first one lit in the accent.
fn lines(painter: &Painter, part: Rect) {
    let step = 6.0;
    let mut y = part.top() + 5.0;
    let mut i = 0;
    while y + 2.0 < part.bottom() - 3.0 {
        let width = part.width() * if i % 3 == 0 { 0.6 } else { 0.45 };
        let colour = if i == 3 {
            crate::interface_look::accent_fill()
        } else {
            tokens::stronger_line()
        };
        let line = Rect::from_min_size(pos2(part.left() + 4.0, y), vec2(width, 2.0));
        painter.rect_filled(line, 0.0, colour);
        y += step;
        i += 1;
    }
}

/// Properties' cards, one under another.
fn cards(painter: &Painter, part: Rect) {
    let height = (part.height() / 3.0 - 4.0).max(4.0);
    let mut y = part.top() + 3.0;
    while y + height < part.bottom() {
        let card =
            Rect::from_min_size(pos2(part.left() + 3.0, y), vec2(part.width() - 6.0, height));
        painter.rect_filled(card, 2.0, tokens::card());
        y += height + 3.0;
    }
}

/// The Library's tiles, in the kinds' colours.
fn tiles(painter: &Painter, part: Rect) {
    let colours = [
        tokens::KIND_FOLDER,
        tokens::KIND_MODEL,
        tokens::KIND_PICTURE,
        tokens::KIND_MATERIAL,
    ];
    let side = 4.0;
    let mut y = part.top() + 6.0;
    let mut n = 0;
    while y + side < part.bottom() - 3.0 {
        let mut x = part.left() + 6.0;
        while x + side < part.right() - 4.0 {
            let tile = Rect::from_min_size(pos2(x, y), vec2(side, side));
            painter.rect_filled(tile, 1.0, colours[n % colours.len()].gamma_multiply(0.8));
            x += side * 2.5;
            n += 1;
        }
        y += side * 2.5;
    }
}

/// A box standing in the view, with its shadow.
fn cube(painter: &Painter, part: Rect) {
    let side = (part.height().min(part.width()) * 0.22).max(6.0);
    let centre = part.center();
    let top = [
        centre + vec2(0.0, -side * 0.6),
        centre + vec2(side * 0.5, -side * 0.35),
        centre + vec2(0.0, -side * 0.1),
        centre + vec2(-side * 0.5, -side * 0.35),
    ];
    let front_left = [
        top[3],
        top[2],
        top[2] + vec2(0.0, side * 0.6),
        top[3] + vec2(0.0, side * 0.6),
    ];
    let front_right = [
        top[2],
        top[1],
        top[1] + vec2(0.0, side * 0.6),
        top[2] + vec2(0.0, side * 0.6),
    ];
    let shade = |share: f32| Color32::from_gray((share * 255.0) as u8);
    for (points, colour) in [
        (front_left.to_vec(), shade(0.32)),
        (front_right.to_vec(), shade(0.26)),
        (top.to_vec(), shade(0.4)),
    ] {
        painter.add(egui::Shape::convex_polygon(points, colour, Stroke::NONE));
    }
    let shadow = Rect::from_center_size(centre + vec2(0.0, side * 0.75), vec2(side * 1.1, 2.0));
    painter.rect_filled(shadow, 1.0, tokens::stronger_line());
}

/// A part's name in a small dark label, as the Preview box has them.
fn label(painter: &Painter, at: Pos2, name: &str) {
    let galley = painter.layout_no_wrap(name.to_owned(), text::ui(), tokens::text_emphasis());
    let rect = Rect::from_center_size(at, galley.size() + vec2(2.0 * size::GAP, 4.0));
    painter.rect_filled(rect, CornerRadius::same(size::radius::SMALL), tokens::app());
    painter.rect_stroke(
        rect,
        CornerRadius::same(size::radius::SMALL),
        Stroke::new(1.0, tokens::card_border()),
        StrokeKind::Inside,
    );
    painter.text(
        at,
        Align2::CENTER_CENTER,
        name,
        text::ui(),
        tokens::text_emphasis(),
    );
}
