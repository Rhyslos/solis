//! Where the layouts put things, as numbers: the side columns running
//! full height, the centre and the bottom panel under it, how they give
//! way when the window narrows, and how they keep their proportions when
//! it grows.

use super::*;
use crate::panel::Panel;
use crate::shell::RibbonTab;
use crate::shell::sizes::Sizes;
use crate::shell::workspaces::{LIBRARY, OpenNow, left_slot, right_slot};

/// The middle at `width` × 900: under the tab bar and ribbon, over the
/// status line.
fn middle(width: f32) -> Rect {
    let top = size::TAB_BAR_H + size::RIBBON_H;
    let bottom = 900.0 - size::STATUS_BAR_H;
    Rect::from_x_y_ranges(0.0..=width, top..=bottom)
}

fn standard(layout: PanelLayout) -> impl Fn(Edge) -> f32 {
    let sizes = Sizes::default();
    move |edge| sizes.get(RibbonTab::Home, layout, edge)
}

/// Home's parts in `layout`, nothing hidden, the bottom panel open.
fn home(layout: PanelLayout) -> Parts {
    let open = OpenNow {
        layout,
        ..OpenNow::default()
    };
    let level = (Slot::Left, left_slot(RibbonTab::Home, open).unwrap());
    let properties = (Slot::Right, right_slot(RibbonTab::Home, open).unwrap());
    let library = (Slot::Library, LIBRARY);
    let (mut left, mut right) = (Vec::new(), Vec::new());
    if layout.left_on_right() {
        right.push(level);
    } else {
        left.push(level);
    }
    right.push(properties);
    match layout.library_at() {
        LibraryAt::UnderLeft => left.push(library),
        LibraryAt::UnderRight => right.push(library),
        LibraryAt::Bottom => {}
    }
    Parts {
        left,
        right,
        bottom: Some(false),
        bottom_under_left: layout.bottom_under_left(),
    }
}

fn rect_of(frame: &Frame, panel: Panel) -> Rect {
    frame
        .columns
        .iter()
        .find(|placed| placed.column.panels.contains(&panel))
        .map(|placed| placed.rect)
        .unwrap_or_else(|| panic!("{panel:?} is placed"))
}

#[test]
fn every_layout_has_its_words_and_reads_back_by_its_key() {
    for layout in PanelLayout::ALL {
        assert_eq!(PanelLayout::from_key(layout.key()), Some(layout));
        assert!(!layout.name().is_empty() && !layout.line().is_empty());
        assert!(!layout.more().is_empty());
    }
    assert_eq!(PanelLayout::default(), PanelLayout::Standard);
    assert_eq!(PanelLayout::from_key("diagonal"), None);
    // Saved before the frame: decision 0109's, by their old words.
    for (old, layout) in [
        ("area left assets bottom", PanelLayout::Standard),
        ("level left assets bottom", PanelLayout::Standard),
        ("shelf", PanelLayout::Standard),
        ("area and assets left", PanelLayout::LibraryLeft),
        ("level and assets left", PanelLayout::LibraryLeft),
        ("area left assets right", PanelLayout::LibraryRight),
        ("level left assets right", PanelLayout::LibraryRight),
    ] {
        assert_eq!(PanelLayout::from_key(old), Some(layout), "{old}");
    }
}

/// The layout as saved in the interface's look: each of decision 0109's
/// reads as its nearest, and one a later build adds as the standard.
#[test]
fn layouts_saved_before_read_as_their_nearest() {
    for (old, layout) in [
        ("LevelLeftAssetsBottom", PanelLayout::Standard),
        ("AreaLeftAssetsBottom", PanelLayout::Standard),
        ("Shelf", PanelLayout::Standard),
        ("LevelAndAssetsLeft", PanelLayout::LibraryLeft),
        ("AreaAndAssetsLeft", PanelLayout::LibraryLeft),
        ("LevelLeftAssetsRight", PanelLayout::LibraryRight),
        ("AreaLeftAssetsRight", PanelLayout::LibraryRight),
        ("Diagonal", PanelLayout::Standard),
    ] {
        let read: PanelLayout = ron::from_str(old).unwrap();
        assert_eq!(read, layout, "{old}");
    }
    for layout in PanelLayout::ALL {
        let text = ron::to_string(&layout).unwrap();
        assert_eq!(ron::from_str::<PanelLayout>(&text).unwrap(), layout);
    }
    // The standard keeps its old name, so a build before the frame
    // still reads it.
    assert_eq!(
        ron::to_string(&PanelLayout::Standard).unwrap(),
        "LevelLeftAssetsBottom"
    );
}

#[test]
fn the_standard_frame_is_board_03_s() {
    let m = middle(1440.0);
    let frame = place(
        m,
        &home(PanelLayout::Standard),
        standard(PanelLayout::Standard),
    );
    let level = rect_of(&frame, Panel::Level);
    let properties = rect_of(&frame, Panel::Properties);
    // Level and Properties run full height, down to the status line.
    assert_eq!(level.y_range(), m.y_range());
    assert_eq!(properties.y_range(), m.y_range());
    assert!((level.width() - 250.0).abs() <= 1.0, "{}", level.width());
    assert!(
        (properties.width() - 320.0).abs() <= 1.0,
        "{}",
        properties.width()
    );
    // The bottom panel under the centre only.
    let bottom = frame.bottom.unwrap();
    assert_eq!(bottom.x_range(), frame.centre.x_range());
    assert_eq!(bottom.bottom(), m.bottom());
    assert_eq!(frame.centre.bottom(), bottom.top());
    // Three accent edges, each with its grip.
    let edges: Vec<Edge> = frame.borders.iter().map(|b| b.edge).collect();
    assert_eq!(edges, [Edge::Left, Edge::Right, Edge::Bottom]);
    assert!(frame.borders.iter().all(|b| b.grip));
}

#[test]
fn each_layout_moves_the_slots_where_its_card_draws_them() {
    let m = middle(1440.0);
    let place_in = |layout| place(m, &home(layout), standard(layout));
    // Wide view: Level over Properties on the right.
    let wide = place_in(PanelLayout::WideView);
    let (level, properties) = (
        rect_of(&wide, Panel::Level),
        rect_of(&wide, Panel::Properties),
    );
    assert_eq!(level.x_range(), properties.x_range());
    assert!(level.bottom() < properties.top());
    assert_eq!(wide.centre.left(), m.left());
    // Library on the left: under Level; the view runs to the foot.
    let left = place_in(PanelLayout::LibraryLeft);
    let (level, library) = (rect_of(&left, Panel::Level), rect_of(&left, Panel::Assets));
    assert_eq!(level.x_range(), library.x_range());
    assert!(level.bottom() < library.top() && level.left() == m.left());
    // Library on the right: under Properties.
    let right = place_in(PanelLayout::LibraryRight);
    let (properties, library) = (
        rect_of(&right, Panel::Properties),
        rect_of(&right, Panel::Assets),
    );
    assert_eq!(properties.x_range(), library.x_range());
    assert_eq!(library.right(), m.right());
    // Tall properties: wider, full height; the bottom panel under Level.
    let tall = place_in(PanelLayout::TallProperties);
    let standard_frame = place_in(PanelLayout::Standard);
    let properties = rect_of(&tall, Panel::Properties);
    assert!(properties.width() > rect_of(&standard_frame, Panel::Properties).width());
    assert_eq!(properties.y_range(), m.y_range());
    let bottom = tall.bottom.unwrap();
    assert_eq!(bottom.left(), m.left());
    assert!(rect_of(&tall, Panel::Level).bottom() < bottom.top());
}

#[test]
fn a_folded_bottom_panel_is_one_line_with_no_edge_to_drag() {
    let m = middle(1440.0);
    let mut parts = home(PanelLayout::Standard);
    parts.bottom = Some(true);
    let frame = place(m, &parts, standard(PanelLayout::Standard));
    assert_eq!(frame.bottom.unwrap().height(), size::BOTTOM_BAR_H);
    assert!(frame.borders.iter().all(|b| b.edge != Edge::Bottom));
    parts.bottom = None;
    let frame = place(m, &parts, standard(PanelLayout::Standard));
    assert_eq!(frame.bottom, None);
    assert_eq!(frame.centre.bottom(), m.bottom());
}

#[test]
fn a_larger_window_keeps_the_panels_in_proportion() {
    let layout = PanelLayout::Standard;
    let small = place(middle(1100.0), &home(layout), standard(layout));
    let large = place(middle(2200.0), &home(layout), standard(layout));
    let share = |frame: &Frame, panel, width: f32| rect_of(frame, panel).width() / width;
    for panel in [Panel::Level, Panel::Properties] {
        let (a, b) = (share(&small, panel, 1100.0), share(&large, panel, 2200.0));
        assert!((a - b).abs() < 0.01, "{panel:?}: {a} and {b}");
    }
    // Growing by a point never makes anything jump.
    let mut before = place(middle(1000.0), &home(layout), standard(layout));
    for width in 1001..1400 {
        let now = place(middle(width as f32), &home(layout), standard(layout));
        let grew = rect_of(&now, Panel::Level).width() - rect_of(&before, Panel::Level).width();
        assert!((0.0..=1.0).contains(&grew), "at {width}: {grew}");
        before = now;
    }
}

#[test]
fn narrower_the_centre_gives_way_then_the_right_then_the_left() {
    let layout = PanelLayout::Standard;
    let wide_sizes = |edge| match edge {
        Edge::Left | Edge::Right => 0.4,
        other => standard(layout)(other),
    };
    // Columns asked wide keep the centre at its least.
    let frame = place(middle(1440.0), &home(layout), wide_sizes);
    assert!(frame.centre.width() >= size::CENTER_MIN_W - 1.0);
    // At their least, the right column goes first, then the left.
    let least = 2.0 * size::COLUMN_MIN_W + size::CENTER_MIN_W + 2.0;
    let just = place(middle(least), &home(layout), standard(layout));
    assert_eq!(just.columns.len(), 2);
    let narrower = place(middle(least - 10.0), &home(layout), standard(layout));
    assert!(narrower.columns.iter().all(|c| c.slot == Slot::Left));
    let narrowest = place(middle(size::CENTER_MIN_W), &home(layout), standard(layout));
    assert!(narrowest.columns.is_empty());
    assert_eq!(narrowest.centre.width(), size::CENTER_MIN_W);
}

#[test]
fn an_edge_s_range_keeps_the_centre_and_every_column_at_their_least() {
    let m = middle(1440.0);
    let frame = place(
        m,
        &home(PanelLayout::Standard),
        standard(PanelLayout::Standard),
    );
    for border in &frame.borders {
        let (low, high) = (*border.range.start(), *border.range.end());
        assert!(low <= high, "{:?}", border.edge);
        if border.edge == Edge::Left || border.edge == Edge::Right {
            assert!((low * m.width() - size::COLUMN_MIN_W).abs() < 0.5);
            let others = size::COLUMN_MIN_W.max(320.0) + size::CENTER_MIN_W;
            assert!(
                high * m.width() <= m.width() - size::CENTER_MIN_W,
                "{others}"
            );
        }
    }
}

#[test]
fn a_pointer_becomes_a_share_of_the_middle() {
    let at = egui::pos2(360.0, 600.0);
    assert_eq!(
        Measure::FromLeft {
            x: 0.0,
            span: 1440.0
        }
        .value(at),
        0.25
    );
    assert_eq!(
        Measure::FromRight {
            x: 1440.0,
            span: 1440.0
        }
        .value(at),
        0.75
    );
    assert_eq!(
        Measure::FromBottom {
            y: 800.0,
            span: 400.0
        }
        .value(at),
        0.5
    );
    assert_eq!(
        Measure::ShareDown {
            top: 400.0,
            height: 400.0
        }
        .value(at),
        0.5
    );
    // A middle of no size never divides by nothing.
    assert!(
        Measure::FromLeft { x: 0.0, span: 0.0 }
            .value(at)
            .is_finite()
    );
}

#[test]
fn any_window_and_any_sizes_place_without_overlap() {
    let mut n = 1u64;
    for layout in PanelLayout::ALL {
        for _ in 0..200 {
            n = n.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            let width = 120.0 + (n >> 40) as f32 % 3000.0;
            let height = 80.0 + (n >> 20) as f32 % 1600.0;
            let share = ((n >> 8) % 1000) as f32 / 1000.0;
            let m = Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, height));
            let frame = place(m, &home(layout), |_| share);
            let c = frame.centre;
            assert!(
                c.left() >= m.left() - 0.01
                    && c.top() >= m.top() - 0.01
                    && c.right() <= m.right() + 0.01
                    && c.bottom() <= m.bottom() + 0.01,
                "{layout:?}: {c:?} in {m:?}"
            );
            for placed in &frame.columns {
                assert!(
                    !placed.rect.intersects(frame.centre.shrink(0.5)),
                    "{layout:?}"
                );
            }
        }
    }
}
