//! The icon table: every drawing reads, fits its square at the sizes the
//! design uses, and comes out in one line style; and the icon sheet, as
//! an approved picture.

use super::*;
use crate::picture_support::{drawn, matches_approved, window};
use crate::tokens;
use egui::vec2;

/// The sizes the design draws icons at: rows and tabs, tools, the ribbon.
const SIZES: [f32; 3] = [12.0, 15.0, 18.0];

/// The outlines of every part of `icon`.
fn outlines(icon: Icon) -> Vec<Outline> {
    let drawing = icon.drawing();
    [drawing.lines, drawing.fills, drawing.dashed, drawing.faint]
        .iter()
        .flat_map(|paths| paths.iter())
        .flat_map(|path| {
            reader::read(path).unwrap_or_else(|e| panic!("{}: {path}: {e}", icon.name()))
        })
        .collect()
}

#[test]
fn every_path_in_the_table_reads_and_every_icon_has_something_to_draw() {
    for icon in Icon::ALL {
        let outlines = outlines(*icon);
        assert!(!outlines.is_empty(), "{} draws nothing", icon.name());
        // More than a speck: at least a third of the grid one way.
        let span = |axis: usize| {
            let all = outlines
                .iter()
                .flat_map(|o| o.points.iter().map(|p| p[axis]));
            let (low, high) = all.fold((f32::MAX, f32::MIN), |(l, h), v| (l.min(v), h.max(v)));
            high - low
        };
        assert!(
            span(0).max(span(1)) >= GRID / 3.0,
            "{} is a speck: {} × {}",
            icon.name(),
            span(0),
            span(1)
        );
    }
}

#[test]
fn every_icon_stays_inside_its_square_at_12_15_and_18_px() {
    for icon in Icon::ALL {
        let drawing = icon.drawing();
        let stroked = !drawing.lines.is_empty() || !drawing.dashed.is_empty();
        for side in SIZES {
            // A line reaches half its width beyond its corners.
            let reach = if stroked {
                drawing.weight.width(side) / 2.0
            } else {
                0.0
            };
            for outline in outlines(*icon) {
                for p in outline.points {
                    for v in p {
                        assert!(
                            v - reach >= -0.01 && v + reach <= GRID + 0.01,
                            "{} leaves its square at {side} px: {p:?}",
                            icon.name()
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn every_icon_paints_shapes_at_every_size() {
    for side in SIZES {
        for icon in Icon::ALL {
            let ctx = egui::Context::default();
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                let painter = ui.ctx().layer_painter(egui::LayerId::background());
                let rect = Rect::from_min_size(pos2(4.0, 4.0), vec2(side, side));
                super::icon(&painter, rect, *icon, Color32::WHITE);
            });
            output.textures_delta.clear();
            assert!(!output.shapes.is_empty(), "{} at {side}", icon.name());
        }
    }
}

#[test]
fn an_icon_asked_for_in_no_room_draws_nothing_and_does_not_fail() {
    let ctx = egui::Context::default();
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
        let painter = ui.ctx().layer_painter(egui::LayerId::background());
        for size in [vec2(0.0, 12.0), vec2(-4.0, -4.0), vec2(f32::NAN, 12.0)] {
            let rect = Rect::from_min_size(pos2(0.0, 0.0), size);
            super::icon(&painter, rect, Icon::Save, Color32::WHITE);
        }
    });
    output.textures_delta.clear();
    assert!(output.shapes.is_empty());
}

#[test]
fn the_table_is_in_the_enum_s_order_with_every_name_once() {
    for (index, icon) in Icon::ALL.iter().enumerate() {
        assert_eq!(*icon as usize, index, "{}", icon.name());
    }
    let mut names: Vec<&str> = Icon::ALL.iter().map(|icon| icon.name()).collect();
    names.sort_unstable();
    let before = names.len();
    names.dedup();
    assert_eq!(names.len(), before, "two icons share a name");
    for name in names {
        assert!(
            !name.is_empty()
                && name
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c == '-' || c.is_ascii_digit()),
            "{name}"
        );
    }
    // The design's 58 and the 69 made for what it lacks.
    assert_eq!(Icon::ALL.len(), 127);
}

#[test]
fn filled_parts_are_shapes_a_plain_fill_can_draw() {
    // The painter fills shapes that bulge outwards everywhere: a fill
    // with a dent would come out wrong.
    for icon in Icon::ALL {
        for path in icon.drawing().fills {
            for outline in reader::read(path).unwrap() {
                let p = &outline.points;
                let n = p.len();
                let turns: Vec<f32> = (0..n)
                    .map(|i| {
                        let (a, b, c) = (p[i], p[(i + 1) % n], p[(i + 2) % n]);
                        (b[0] - a[0]) * (c[1] - b[1]) - (b[1] - a[1]) * (c[0] - b[0])
                    })
                    .filter(|turn| turn.abs() > 1e-4)
                    .collect();
                let one_way = turns.iter().all(|t| *t > 0.0) || turns.iter().all(|t| *t < 0.0);
                assert!(one_way, "{}: {path} has a dent", icon.name());
            }
        }
    }
}

#[test]
fn lines_are_as_heavy_as_the_design_draws_them() {
    // The design's own numbers (the plan's inventory, d.7): 1.3 on the
    // grid for the ribbon's 18 px icons, 1.5 for 12 to 15 px.
    assert_eq!(Weight::Line.width(18.0), 1.3);
    assert_eq!(Weight::Line.width(15.0), 1.5);
    assert_eq!(Weight::Line.width(12.0), 1.5);
    assert_eq!(Weight::Mark.width(13.0), 1.6);
    assert_eq!(Weight::Chevron.width(12.0), 1.8);
    // Larger drawings take lighter lines, as its thumbnails do.
    assert_eq!(Weight::Line.width(22.0), 1.0);
    assert_eq!(Weight::Line.width(26.0), 0.9);
    assert_eq!(Weight::Line.width(44.0), 0.8);
    assert_eq!(Weight::Chevron.width(44.0), 0.8);
    // So a line is between one and one and a half pixels wide at every
    // size the interface draws an icon at.
    for side in 12..=26 {
        let side = side as f32;
        let pixels = Weight::Line.width(side) * side / GRID;
        assert!((1.0..=1.55).contains(&pixels), "{side}: {pixels}");
    }
}

#[test]
fn the_new_icons_are_drawn_like_the_design_s_own() {
    // One style: the same weights as the design's icons, whichever part
    // of the table an icon is in. Only chevrons and small marks differ.
    let heavy: Vec<&str> = Icon::ALL
        .iter()
        .filter(|icon| icon.drawing().weight != Weight::Line)
        .map(|icon| icon.name())
        .collect();
    assert_eq!(
        heavy,
        [
            "chevron-up",
            "error",
            "warning-outline",
            "chevron-left",
            "arrow-up",
            "close",
            "warning",
            "chevron-down",
            "chevron-right"
        ]
    );
    // Square ends and sharp corners are how the painter draws a line:
    // an open line is a plain run of points, with nothing added at its
    // ends.
    let ctx = egui::Context::default();
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
        let painter = ui.ctx().layer_painter(egui::LayerId::background());
        let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(16.0, 16.0));
        super::icon(&painter, rect, Icon::ChevronDown, Color32::WHITE);
    });
    output.textures_delta.clear();
    assert_eq!(output.shapes.len(), 1);
    let egui::Shape::Path(path) = &output.shapes[0].shape else {
        panic!("a chevron is one line");
    };
    assert!(!path.closed);
    assert_eq!(path.points.len(), 3);
    assert_eq!(path.stroke.width, 1.8);
}

/// The icon sheet's cell: an icon at each of the three sizes.
const CELL: [f32; 2] = [66.0, 26.0];
const ACROSS: usize = 12;
/// How many of the table's icons are the design's own (they come first).
const THE_DESIGN_S: usize = 58;

/// Where the `index`th icon's cell is. The icons made for Solis start
/// on a new row, a gap below the design's.
fn cell(index: usize) -> Rect {
    let (index, top) = if index < THE_DESIGN_S {
        (index, 8.0)
    } else {
        let rows = THE_DESIGN_S.div_ceil(ACROSS) as f32;
        (index - THE_DESIGN_S, 8.0 + rows * CELL[1] + 12.0)
    };
    let (column, row) = (index % ACROSS, index / ACROSS);
    Rect::from_min_size(
        pos2(8.0 + column as f32 * CELL[0], top + row as f32 * CELL[1]),
        vec2(CELL[0], CELL[1]),
    )
}

/// Where in its cell an icon of the `nth` size is drawn: on whole
/// pixels, as a control places it.
fn spot(cell: Rect, nth: usize) -> Rect {
    let left = cell.left() + [0.0, 16.0, 35.0][nth];
    let side = SIZES[nth];
    Rect::from_min_size(
        pos2(left, (cell.center().y - side / 2.0).floor()),
        vec2(side, side),
    )
}

fn sheet_size() -> [f32; 2] {
    let last = cell(Icon::ALL.len() - 1);
    [16.0 + ACROSS as f32 * CELL[0], last.bottom() + 8.0]
}

/// Every icon at 12, 15 and 18 px on the ribbon's grey: the design's own
/// first, then the ones made for Solis, so each new one can be looked at
/// beside a design icon of its weight.
#[test]
fn the_icon_sheet_matches_its_approved_picture() {
    let Some(mut harness) = window(sheet_size(), 1.0, |ui| {
        let painter = ui.painter();
        painter.rect_filled(ui.max_rect(), 0.0, tokens::ribbon());
        for (index, icon) in Icon::ALL.iter().enumerate() {
            for nth in 0..SIZES.len() {
                super::icon(painter, spot(cell(index), nth), *icon, tokens::text_body());
            }
        }
    }) else {
        return;
    };
    harness.run();
    let picture = drawn(&mut harness);
    matches_approved("icon-sheet", &picture);
    // Every icon at every size left ink in its spot, and none outside it
    // (a pixel's soft edge aside).
    for (index, icon) in Icon::ALL.iter().enumerate() {
        for (nth, side) in SIZES.iter().enumerate() {
            let spot = spot(cell(index), nth);
            let mut inked = 0;
            for y in spot.top() as u32..spot.bottom() as u32 {
                for x in spot.left() as u32..spot.right() as u32 {
                    if picture.pixel(x, y) != tokens::ribbon() {
                        inked += 1;
                    }
                }
            }
            assert!(inked >= 8, "{} at {} px: {inked} pixels", icon.name(), side);
            let ring = spot.expand(1.0);
            for x in ring.left() as u32..ring.right() as u32 {
                for y in [ring.top() as u32 - 1, ring.bottom() as u32] {
                    assert_eq!(
                        picture.pixel(x, y),
                        tokens::ribbon(),
                        "{} at {} px spills over",
                        icon.name(),
                        side
                    );
                }
            }
        }
    }
}

#[test]
fn a_line_that_turns_back_on_itself_is_cut_there() {
    // Down, then straight back up: two lines that end at the turn.
    let hairpin = Outline {
        points: vec![[2.0, 2.0], [2.0, 10.0], [2.5, 2.0]],
        closed: false,
    };
    let pieces = super::cut_at_cusps(hairpin);
    assert_eq!(pieces.len(), 2);
    assert_eq!(pieces[0].points, vec![[2.0, 2.0], [2.0, 10.0]]);
    assert_eq!(pieces[1].points, vec![[2.0, 10.0], [2.5, 2.0]]);
    // A right angle is a corner like any other, and a closed square
    // stays closed.
    let square = Outline {
        points: vec![[2.0, 2.0], [10.0, 2.0], [10.0, 10.0], [2.0, 10.0]],
        closed: true,
    };
    assert_eq!(super::cut_at_cusps(square.clone()), vec![square]);
    // A closed sliver is opened at its sharp end, and still drawn whole.
    let sliver = Outline {
        points: vec![[2.0, 8.0], [8.0, 7.0], [14.0, 8.0], [8.0, 9.0]],
        closed: true,
    };
    let pieces = super::cut_at_cusps(sliver);
    assert!(pieces.iter().all(|piece| !piece.closed));
    let drawn: usize = pieces.iter().map(|piece| piece.points.len() - 1).sum();
    assert_eq!(drawn, 4, "every side is still there: {pieces:?}");
}

#[test]
fn no_icon_is_drawn_with_a_corner_sharp_enough_to_spike() {
    // The open book's spine was: its page's edge curls straight back up
    // from the foot of the spine, and the corner shot out below the icon.
    for icon in Icon::ALL {
        let read = super::read_of(*icon).unwrap();
        for outline in read.lines.iter().chain(&read.faint) {
            let p = &outline.points;
            let n = p.len();
            let joints = if outline.closed {
                n
            } else {
                n.saturating_sub(2)
            };
            for i in 0..joints {
                let (a, b, c) = if outline.closed {
                    (p[i], p[(i + 1) % n], p[(i + 2) % n])
                } else {
                    (p[i], p[i + 1], p[i + 2])
                };
                assert!(!super::is_cusp(a, b, c), "{} at {b:?}", icon.name());
            }
        }
    }
}
