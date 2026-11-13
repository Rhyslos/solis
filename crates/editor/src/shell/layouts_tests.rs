//! Where the layouts put things, as numbers: the side columns, the center
//! and the strip at their standard sizes, how they give way when the
//! window narrows, and the shelf's columns with widths of their own.

use super::*;
use crate::shell::RibbonTab;
use crate::shell::sizes::Sizes;
use crate::shell::workspaces::ShelfTab;

/// The middle above the shelf at 1440 × 900 in a side layout: under the
/// tab bar and ribbon, over the shelf at its standard height.
fn upper(width: f32) -> Rect {
    let top = size::TAB_BAR_H + size::RIBBON_H;
    let bottom = 900.0 - size::STATUS_BAR_H - size::SHELF_BAR_H - size::SHELF_H_SIDE;
    Rect::from_x_y_ranges(0.0..=width, top..=bottom)
}

fn standard(layout: PanelLayout) -> impl Fn(Edge) -> f32 {
    let sizes = Sizes::default();
    move |edge| sizes.get(RibbonTab::Home, layout, edge)
}

fn rect_of(side: &Side, panel: Panel) -> Rect {
    side.lists
        .iter()
        .find(|placed| placed.column.panels.contains(&panel))
        .map(|placed| placed.rect)
        .unwrap_or_else(|| panic!("{panel:?} is placed"))
}

#[test]
fn every_layout_has_its_names_and_reads_back_by_its_key() {
    for layout in PanelLayout::ALL {
        assert_eq!(PanelLayout::from_key(layout.key()), Some(layout));
        assert!(!layout.name().is_empty());
    }
    assert_eq!(PanelLayout::default(), PanelLayout::AreaLeftAssetsBottom);
    assert_eq!(PanelLayout::from_key("diagonal"), None);
    assert!(!PanelLayout::Shelf.is_side());
}

#[test]
fn a_layout_a_later_build_adds_reads_as_the_standard() {
    let read: PanelLayout = ron::from_str("Diagonal").unwrap();
    assert_eq!(read, PanelLayout::default());
    for layout in PanelLayout::ALL {
        let text = ron::to_string(&layout).unwrap();
        assert_eq!(ron::from_str::<PanelLayout>(&text).unwrap(), layout);
    }
}

#[test]
fn each_side_layout_has_area_properties_and_assets_once() {
    for layout in PanelLayout::ALL.into_iter().filter(|l| l.is_side()) {
        let side = side(upper(1440.0), layout, standard(layout));
        let panels: Vec<Panel> = side
            .lists
            .iter()
            .flat_map(|p| p.column.panels.iter().copied())
            .collect();
        let want = [Panel::Area, Panel::Properties, Panel::Assets];
        assert_eq!(panels.len(), 3, "{layout:?}");
        for panel in want {
            assert!(panels.contains(&panel), "{layout:?} {panel:?}");
        }
        for panel in want {
            assert!(layout.has_beside(panel));
        }
    }
    assert!(!PanelLayout::Shelf.has_beside(Panel::Area));
}

#[test]
fn area_left_assets_bottom_at_its_standard_sizes() {
    let up = upper(1440.0);
    let side = side(
        up,
        PanelLayout::AreaLeftAssetsBottom,
        standard(PanelLayout::AreaLeftAssetsBottom),
    );
    let area = rect_of(&side, Panel::Area);
    let properties = rect_of(&side, Panel::Properties);
    let assets = rect_of(&side, Panel::Assets);
    // Tall columns at the sides, from top to bottom.
    assert_eq!(area.width(), size::SCENE_COLUMN_W);
    assert_eq!(area.y_range(), up.y_range());
    assert_eq!(properties.width(), size::PROPERTIES_COLUMN_W);
    assert_eq!(properties.right(), up.right());
    assert_eq!(properties.y_range(), up.y_range());
    // The center between them, 1 px from each, and the strip under it.
    assert_eq!(side.center.left(), area.right() + 1.0);
    assert_eq!(side.center.right(), properties.left() - 1.0);
    assert_eq!(assets.x_range(), side.center.x_range());
    assert_eq!(assets.height(), size::ASSETS_STRIP_H);
    assert_eq!(assets.top(), side.center.bottom() + 1.0);
    assert_eq!(assets.bottom(), up.bottom());
    let edges: Vec<Edge> = side.borders.iter().map(|b| b.edge).collect();
    assert_eq!(edges, [Edge::Left, Edge::Right, Edge::Strip]);
}

#[test]
fn stacked_lists_share_their_column() {
    let up = upper(1440.0);
    let left = side(
        up,
        PanelLayout::AreaAndAssetsLeft,
        standard(PanelLayout::AreaAndAssetsLeft),
    );
    let (area, assets) = (rect_of(&left, Panel::Area), rect_of(&left, Panel::Assets));
    assert_eq!(area.width(), size::ASSETS_COLUMN_W);
    assert_eq!(assets.top(), area.bottom() + 1.0);
    assert_eq!(assets.x_range(), area.x_range());
    assert!((area.height() - up.height() * size::SIDE_SHARE).abs() <= 1.0);
    assert_eq!(left.center.bottom(), up.bottom(), "no strip");
    let right = side(
        up,
        PanelLayout::AreaLeftAssetsRight,
        standard(PanelLayout::AreaLeftAssetsRight),
    );
    let (properties, assets) = (
        rect_of(&right, Panel::Properties),
        rect_of(&right, Panel::Assets),
    );
    assert_eq!(assets.top(), properties.bottom() + 1.0);
    assert_eq!(assets.right(), up.right());
    assert!(right.borders.iter().any(|b| b.edge == Edge::RightShare));
}

#[test]
fn a_dragged_size_is_used_and_held_to_the_room() {
    let up = upper(1440.0);
    let layout = PanelLayout::AreaLeftAssetsBottom;
    let side = side(up, layout, |edge| match edge {
        Edge::Left => 500.0,
        Edge::Strip => 5_000.0,
        other => standard(layout)(other),
    });
    assert_eq!(rect_of(&side, Panel::Area).width(), 500.0);
    // The strip can't take the center below its least.
    assert!(side.center.height() >= size::CENTER_MIN_H);
    let strip = side.borders.iter().find(|b| b.edge == Edge::Strip).unwrap();
    assert!(strip.range.end() + size::CENTER_MIN_H + 1.0 <= up.height() + 0.5);
}

#[test]
fn when_narrow_the_center_gives_way_then_the_right_column_then_the_left() {
    let layout = PanelLayout::AreaLeftAssetsBottom;
    let at = |width: f32| side(upper(width), layout, standard(layout));
    // Wide enough: both columns at their standard, the center smaller.
    let roomy = at(1100.0);
    assert_eq!(
        rect_of(&roomy, Panel::Properties).width(),
        size::PROPERTIES_COLUMN_W
    );
    assert_eq!(rect_of(&roomy, Panel::Area).width(), size::SCENE_COLUMN_W);
    // The center at its least: the right column narrows first.
    let width = size::SCENE_COLUMN_W + size::PROPERTIES_COLUMN_W - 100.0 + 2.0 + size::CENTER_MIN_W;
    let squeezed = at(width);
    assert_eq!(squeezed.center.width(), size::CENTER_MIN_W);
    assert_eq!(
        rect_of(&squeezed, Panel::Area).width(),
        size::SCENE_COLUMN_W
    );
    assert_eq!(
        rect_of(&squeezed, Panel::Properties).width(),
        size::PROPERTIES_COLUMN_W - 100.0
    );
    // Then the left, once the right is at its least.
    let least = at(least_width(layout));
    assert_eq!(rect_of(&least, Panel::Area).width(), size::COLUMN_MIN_W);
    assert_eq!(
        rect_of(&least, Panel::Properties).width(),
        size::COLUMN_MIN_W
    );
    assert_eq!(least.center.width(), size::CENTER_MIN_W);
    assert_eq!(least_width(PanelLayout::Shelf), 0.0);
}

#[test]
fn borders_measure_from_where_their_size_starts() {
    let at = egui::pos2(300.0, 400.0);
    assert_eq!(Measure::FromLeft(100.0).value(at), 200.0);
    assert_eq!(Measure::FromRight(1000.0).value(at), 700.0);
    assert_eq!(Measure::FromBottom(600.0).value(at), 200.0);
    let share = Measure::ShareDown {
        top: 200.0,
        height: 400.0,
    };
    assert_eq!(share.value(at), 0.5);
    assert!(Measure::FromLeft(0.0).upright() && !share.upright());
    // A border with no room can't divide by nothing.
    let flat = Measure::ShareDown {
        top: 0.0,
        height: 0.0,
    };
    assert!(flat.value(at).is_finite());
}

fn entity_columns(layout: PanelLayout) -> &'static [Column] {
    ShelfTab::Entity.columns(layout)
}

#[test]
fn shelf_columns_take_their_widths_and_the_rest_takes_what_is_left() {
    let content = Rect::from_x_y_ranges(0.0..=1440.0, 600.0..=900.0);
    let columns = entity_columns(PanelLayout::Shelf);
    let edge = |i: usize| Edge::Column(ShelfTab::Entity, i as u8);
    let (rects, borders) = shelf_columns(content, columns, &[400.0, 0.0, 300.0], edge);
    assert_eq!(rects[0].width(), 400.0);
    assert_eq!(rects[1].left(), rects[0].right() + 1.0);
    assert_eq!(rects[2].right(), content.right());
    assert!((rects[2].width() - 300.0).abs() <= 1.0);
    // A border between each pair, setting the column with a width of
    // its own beside it.
    let edges: Vec<Edge> = borders.iter().map(|b| b.edge).collect();
    assert_eq!(edges, [edge(0), edge(2)]);
    assert_eq!(borders[0].measure, Measure::FromLeft(rects[0].left()));
    assert_eq!(borders[1].measure, Measure::FromRight(rects[2].right()));
    assert_eq!(borders[0].name(), "Between Area and Properties");
}

#[test]
fn shelf_columns_give_way_from_the_right_then_wrap() {
    let columns = entity_columns(PanelLayout::Shelf);
    let edge = |i: usize| Edge::Column(ShelfTab::Entity, i as u8);
    let widths = [size::SCENE_COLUMN_W, 0.0, size::ASSETS_COLUMN_W];
    let room = |w: f32| Rect::from_x_y_ranges(0.0..=w, 600.0..=900.0);
    // Properties at its least, then Assets narrows, Area kept.
    let w = size::SCENE_COLUMN_W + size::COLUMN_MIN_W + size::ASSETS_COLUMN_W - 50.0 + 2.0;
    let (rects, _) = shelf_columns(room(w), columns, &widths, edge);
    assert_eq!(rects[0].width(), size::SCENE_COLUMN_W);
    assert!((rects[1].width() - size::COLUMN_MIN_W).abs() <= 1.0);
    assert!((rects[2].width() - (size::ASSETS_COLUMN_W - 50.0)).abs() <= 1.0);
    // Too narrow for all three at their least: under each other.
    let (rects, borders) = shelf_columns(
        room(3.0 * size::COLUMN_MIN_W - 20.0),
        columns,
        &widths,
        edge,
    );
    assert_eq!(rects[0].left(), rects[1].left());
    assert!(rects[1].top() > rects[0].bottom());
    assert!(
        borders.is_empty(),
        "nothing to drag between columns under each other"
    );
}

#[test]
fn place_keeps_only_placement_in_a_side_layout() {
    let placing = ShelfTab::Place.columns(PanelLayout::AreaLeftAssetsBottom);
    assert_eq!(placing.len(), 1);
    assert_eq!(placing[0].panels, [Panel::Placement]);
    assert_eq!(ShelfTab::Place.columns(PanelLayout::Shelf).len(), 3);
}
