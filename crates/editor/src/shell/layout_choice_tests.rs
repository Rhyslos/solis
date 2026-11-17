//! Choosing a layout: at once, on every tab, with Undo; and Reset sizes.

use super::*;
use crate::shell::sizes::Edge;
use crate::shell::workspaces::Slot;

fn editor() -> (Workbench, egui::Context) {
    (
        Workbench::new(crate::test_support::info()),
        egui::Context::default(),
    )
}

#[test]
fn a_layout_applies_at_once_and_undo_puts_back_what_it_changed() {
    let (mut w, ctx) = editor();
    w.shell
        .arrangement_mut(RibbonTab::Home)
        .hidden
        .set(Slot::Right, true);
    w.shell.arrangement_mut(RibbonTab::Debug).bottom_folded = true;
    w.choose_layout(&ctx, PanelLayout::WideView);
    assert_eq!(w.layout(), PanelLayout::WideView);
    assert_eq!(w.layout_note(), Some(PanelLayout::WideView));
    // A layout shows every panel again; folds are the user's own.
    assert!(!w.shell.arrangement(RibbonTab::Home).hidden.any());
    assert!(w.shell.arrangement(RibbonTab::Debug).bottom_folded);
    w.undo_layout(&ctx);
    assert_eq!(w.layout(), PanelLayout::Standard);
    assert!(w.shell.arrangement(RibbonTab::Home).hidden.has(Slot::Right));
    assert_eq!(w.layout_note(), None);
    // A second Undo has nothing to do.
    w.undo_layout(&ctx);
    assert_eq!(w.layout(), PanelLayout::Standard);
}

#[test]
fn just_the_view_hides_every_panel_on_every_tab_and_folds_the_bottom() {
    let (mut w, ctx) = editor();
    w.shell.arrangement_mut(RibbonTab::Home).placing = true;
    w.choose_layout(&ctx, PanelLayout::JustTheView);
    for tab in [RibbonTab::Home, RibbonTab::Ui, RibbonTab::Tileset] {
        let arrangement = w.shell.arrangement(tab);
        assert_eq!(
            arrangement.hidden,
            crate::shell::workspaces::Hidden::ALL,
            "{tab:?}"
        );
        assert!(arrangement.bottom_folded, "{tab:?}");
    }
    assert!(!w.placing());
    // Another layout after it shows them again, opened as they start.
    w.choose_layout(&ctx, PanelLayout::Standard);
    let home = w.shell.arrangement(RibbonTab::Home);
    assert!(!home.hidden.any() && !home.bottom_folded);
    assert!(
        w.shell.arrangement(RibbonTab::Library).bottom_folded,
        "as it starts"
    );
}

#[test]
fn two_views_splits_the_centre_and_leaving_it_puts_one_back() {
    let (mut w, ctx) = editor();
    w.choose_layout(&ctx, PanelLayout::TwoViews);
    assert_eq!(w.viewport.layout, ViewLayout::SideBySide);
    w.choose_layout(&ctx, PanelLayout::LibraryLeft);
    assert_eq!(w.viewport.layout, ViewLayout::One);
    // Undo goes back to the two views.
    w.undo_layout(&ctx);
    assert_eq!(w.viewport.layout, ViewLayout::SideBySide);
    assert_eq!(w.layout(), PanelLayout::TwoViews);
    // Views chosen in the view's own menu are left alone by another
    // layout.
    w.choose_layout(&ctx, PanelLayout::Standard);
    w.viewport.layout = ViewLayout::Four;
    w.choose_layout(&ctx, PanelLayout::WideView);
    assert_eq!(w.viewport.layout, ViewLayout::Four);
}

#[test]
fn reset_sizes_puts_every_edge_of_the_layout_in_use_back() {
    let (mut w, ctx) = editor();
    for tab in [RibbonTab::Home, RibbonTab::Audio] {
        w.shell
            .sizes
            .set(tab, PanelLayout::Standard, Edge::Left, 0.3);
        w.shell
            .sizes
            .set(tab, PanelLayout::WideView, Edge::Left, 0.3);
    }
    w.reset_sizes();
    for tab in [RibbonTab::Home, RibbonTab::Audio] {
        assert_eq!(
            w.shell.size(tab, PanelLayout::Standard, Edge::Left),
            Edge::Left.standard(tab, PanelLayout::Standard)
        );
        assert_eq!(w.shell.size(tab, PanelLayout::WideView, Edge::Left), 0.3);
    }
    w.choose_layout(&ctx, PanelLayout::WideView);
    w.reset_sizes();
    assert_eq!(w.shell.sizes.kept().count(), 0);
}

#[test]
fn the_note_closes_and_keeps_the_layout() {
    let (mut w, ctx) = editor();
    w.choose_layout(&ctx, PanelLayout::TallProperties);
    w.close_layout_note();
    assert_eq!(w.layout_note(), None);
    w.undo_layout(&ctx);
    assert_eq!(w.layout(), PanelLayout::TallProperties);
}
