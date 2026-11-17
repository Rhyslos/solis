//! What each edge was dragged to: kept per tab and per layout, or shared
//! by every tab while tabs don't keep their own, held to its limits, and
//! forgotten again.

use super::*;

#[test]
fn every_edge_reads_back_by_its_key() {
    for edge in Edge::ALL {
        assert_eq!(Edge::from_key(edge.key()), Some(edge));
    }
    // Decision 0109's keys held points: `saved.rs` reads them, not this.
    for old in ["left", "right", "strip", "shelf", "column Place 1", ""] {
        assert_eq!(Edge::from_key(old), None, "{old}");
    }
}

#[test]
fn every_standard_is_inside_its_limits() {
    for tab in RibbonTab::ALL {
        for layout in PanelLayout::ALL {
            for edge in Edge::ALL {
                let value = edge.standard(tab, layout);
                assert!(
                    edge.limits().contains(&value),
                    "{tab:?} {layout:?} {edge:?}"
                );
            }
        }
    }
    // The board's: Properties wider in Tall properties, the timeline
    // taller on Animate.
    let tall = Edge::Right.standard(RibbonTab::Home, PanelLayout::TallProperties);
    assert!(tall > Edge::Right.standard(RibbonTab::Home, PanelLayout::Standard));
    let animate = Edge::Bottom.standard(RibbonTab::Animate, PanelLayout::Standard);
    assert!(animate > Edge::Bottom.standard(RibbonTab::Home, PanelLayout::Standard));
}

#[test]
fn a_size_is_kept_per_tab_and_per_layout() {
    let mut sizes = Sizes::default();
    let layout = PanelLayout::Standard;
    sizes.set(RibbonTab::Home, layout, Edge::Left, 0.3);
    assert_eq!(sizes.get(RibbonTab::Home, layout, Edge::Left), 0.3);
    let other_tab = sizes.get(RibbonTab::Ui, layout, Edge::Left);
    assert_eq!(other_tab, Edge::Left.standard(RibbonTab::Ui, layout));
    let other_layout = sizes.get(RibbonTab::Home, PanelLayout::WideView, Edge::Left);
    assert_eq!(
        other_layout,
        Edge::Left.standard(RibbonTab::Home, PanelLayout::WideView)
    );
}

#[test]
fn with_tabs_sharing_sizes_the_frame_s_edges_are_one_set() {
    let mut sizes = Sizes {
        per_tab: false,
        ..Sizes::default()
    };
    let layout = PanelLayout::Standard;
    sizes.set(RibbonTab::Ui, layout, Edge::Right, 0.3);
    sizes.set(RibbonTab::Debug, layout, Edge::Bottom, 0.5);
    for tab in [RibbonTab::Home, RibbonTab::Audio, RibbonTab::Ui] {
        assert_eq!(sizes.get(tab, layout, Edge::Right), 0.3, "{tab:?}");
        assert_eq!(sizes.get(tab, layout, Edge::Bottom), 0.5, "{tab:?}");
    }
    // What a tab shows keeps its own: the split between its groups.
    sizes.set(RibbonTab::Animate, layout, Edge::Split, 0.3);
    assert_eq!(
        sizes.get(RibbonTab::Debug, layout, Edge::Split),
        size::SPLIT
    );
    // On again, each tab has what it had kept before.
    sizes.per_tab = true;
    assert_eq!(
        sizes.get(RibbonTab::Ui, layout, Edge::Right),
        Edge::Right.standard(RibbonTab::Ui, layout)
    );
}

#[test]
fn a_size_is_held_to_its_limits_and_nothing_unusable_is_kept() {
    let mut sizes = Sizes::default();
    let layout = PanelLayout::Standard;
    sizes.set(RibbonTab::Home, layout, Edge::Left, 5.0);
    assert_eq!(
        sizes.get(RibbonTab::Home, layout, Edge::Left),
        *Edge::Left.limits().end()
    );
    sizes.set(RibbonTab::Home, layout, Edge::Bottom, f32::NAN);
    sizes.set(RibbonTab::Home, layout, Edge::Bottom, f32::INFINITY);
    assert_eq!(
        sizes.get(RibbonTab::Home, layout, Edge::Bottom),
        Edge::Bottom.standard(RibbonTab::Home, layout)
    );
    assert!(!sizes.take_saved(RibbonTab::Home, layout, Edge::Split, 0.99));
    assert!(!sizes.take_saved(RibbonTab::Home, layout, Edge::Placement, -3.0));
    assert!(sizes.take_saved(RibbonTab::Home, layout, Edge::Split, 0.3));
}

#[test]
fn forgetting_puts_the_standard_back() {
    let mut sizes = Sizes::default();
    let (standard, wide) = (PanelLayout::Standard, PanelLayout::WideView);
    for tab in [RibbonTab::Home, RibbonTab::Ui] {
        for layout in [standard, wide] {
            sizes.set(tab, layout, Edge::Left, 0.3);
        }
    }
    sizes.forget(RibbonTab::Home, standard, Edge::Left);
    assert_eq!(
        sizes.get(RibbonTab::Home, standard, Edge::Left),
        Edge::Left.standard(RibbonTab::Home, standard)
    );
    sizes.forget_tab(RibbonTab::Home);
    assert_eq!(sizes.kept().count(), 2);
    // Reset sizes: every tab's in the layout in use, the rest kept.
    sizes.forget_layout(wide);
    assert_eq!(sizes.kept().count(), 1);
    assert_eq!(sizes.get(RibbonTab::Ui, standard, Edge::Left), 0.3);
}
