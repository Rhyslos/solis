//! What each border remembers: standards, limits, keys, and forgetting.

use super::*;

const EDGES: [Edge; 9] = [
    Edge::ShelfHeight,
    Edge::Split,
    Edge::Left,
    Edge::Right,
    Edge::Strip,
    Edge::LeftShare,
    Edge::RightShare,
    Edge::Column(ShelfTab::Entity, 0),
    Edge::Column(ShelfTab::Frames, 0),
];

#[test]
fn every_edge_reads_back_by_its_key() {
    for edge in EDGES {
        assert_eq!(Edge::from_key(&edge.key()), Some(edge), "{edge:?}");
    }
    for unknown in [
        "",
        "middle",
        "column",
        "column Entity",
        "column Entity x",
        "column Nowhere 0",
        "column Entity 9",
        // Properties takes what's left: it has no width of its own.
        "column Entity 1",
        "column Entity 300",
    ] {
        assert_eq!(Edge::from_key(unknown), None, "{unknown:?}");
    }
}

#[test]
fn every_standard_is_inside_its_limits() {
    for tab in RibbonTab::ALL.into_iter().chain(RibbonTab::CONTEXT) {
        for layout in PanelLayout::ALL {
            for edge in EDGES {
                let standard = edge.standard(tab, layout);
                assert!(
                    edge.limits().contains(&standard),
                    "{tab:?} {layout:?} {edge:?}"
                );
            }
        }
    }
    // Home's shelf holds less beside the lists, so it starts shorter.
    let home = |layout| Edge::ShelfHeight.standard(RibbonTab::Home, layout);
    assert!(home(PanelLayout::AreaLeftAssetsBottom) < home(PanelLayout::Shelf));
}

#[test]
fn a_size_is_kept_per_tab_and_per_layout_on_home_only() {
    let mut sizes = Sizes::default();
    let (side, shelf) = (PanelLayout::AreaLeftAssetsBottom, PanelLayout::Shelf);
    sizes.set(RibbonTab::Home, side, Edge::ShelfHeight, 222.0);
    assert_eq!(sizes.get(RibbonTab::Home, side, Edge::ShelfHeight), 222.0);
    assert_eq!(
        sizes.get(RibbonTab::Home, shelf, Edge::ShelfHeight),
        size::SHELF_H,
        "the shelf layout's own"
    );
    // Another tab looks the same in every layout: one size for all.
    sizes.set(RibbonTab::Audio, side, Edge::Split, 0.3);
    for layout in PanelLayout::ALL {
        assert_eq!(sizes.get(RibbonTab::Audio, layout, Edge::Split), 0.3);
    }
    assert_eq!(sizes.get(RibbonTab::Debug, side, Edge::Split), size::SPLIT);
}

#[test]
fn a_size_is_held_to_its_limits_and_nothing_unusable_is_kept() {
    let mut sizes = Sizes::default();
    let home = RibbonTab::Home;
    let layout = PanelLayout::AreaLeftAssetsRight;
    sizes.set(home, layout, Edge::Left, 1.0);
    assert_eq!(sizes.get(home, layout, Edge::Left), size::COLUMN_MIN_W);
    sizes.set(home, layout, Edge::RightShare, 2.0);
    assert_eq!(sizes.get(home, layout, Edge::RightShare), 0.85);
    sizes.set(home, layout, Edge::Strip, f32::NAN);
    assert_eq!(sizes.get(home, layout, Edge::Strip), size::ASSETS_STRIP_H);
    for value in [f32::NAN, f32::INFINITY, -1.0, 0.0, SIZE_MAX * 2.0] {
        assert!(
            !sizes.take_saved(home, layout, Edge::Right, value),
            "{value}"
        );
    }
    assert!(sizes.take_saved(home, layout, Edge::Right, 500.0));
    assert_eq!(sizes.get(home, layout, Edge::Right), 500.0);
}

#[test]
fn forgetting_puts_the_standard_back() {
    let mut sizes = Sizes::default();
    let layout = PanelLayout::AreaAndAssetsLeft;
    sizes.set(RibbonTab::Home, layout, Edge::Left, 420.0);
    sizes.set(
        RibbonTab::Home,
        PanelLayout::Shelf,
        Edge::ShelfHeight,
        420.0,
    );
    sizes.set(RibbonTab::Ui, layout, Edge::Split, 0.7);
    sizes.forget(RibbonTab::Home, layout, Edge::Left);
    assert_eq!(
        sizes.get(RibbonTab::Home, layout, Edge::Left),
        Edge::Left.standard(RibbonTab::Home, layout)
    );
    sizes.forget_tab(RibbonTab::Home);
    assert_eq!(
        sizes.get(RibbonTab::Home, PanelLayout::Shelf, Edge::ShelfHeight),
        size::SHELF_H
    );
    assert_eq!(
        sizes.get(RibbonTab::Ui, layout, Edge::Split),
        0.7,
        "another tab's kept"
    );
    assert_eq!(sizes.kept().count(), 1);
}
