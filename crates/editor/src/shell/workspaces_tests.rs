//! The table of what each tab's middle holds.

use super::*;

const EVERYTHING: OpenNow = OpenNow {
    playing: true,
    animator: true,
    tileset: true,
    atlas: true,
    clips: true,
    map: true,
    layout: PanelLayout::Shelf,
};

#[test]
fn every_panel_has_a_home_with_everything_open() {
    for panel in Panel::ALL.into_iter().chain(Panel::OPEN_WITH_A_FILE) {
        assert!(
            tab_for(panel, EVERYTHING).is_some(),
            "{panel:?} has no place"
        );
    }
}

#[test]
fn a_panel_is_in_one_place_per_tab() {
    for tab in RibbonTab::ALL.into_iter().chain(RibbonTab::CONTEXT) {
        let mut seen = Vec::new();
        for panel in groups(tab, EVERYTHING).into_iter().flatten() {
            assert!(!seen.contains(&panel), "{panel:?} twice on {tab:?}");
            seen.push(panel);
        }
        for shelf in built_shelf_tabs(tab, EVERYTHING) {
            for panel in shelf.panels(EVERYTHING.layout) {
                assert!(
                    !groups(tab, EVERYTHING).concat().contains(&panel),
                    "{panel:?} is both a document and on the shelf of {tab:?}"
                );
            }
        }
    }
}

#[test]
fn documents_come_and_go_with_what_is_open() {
    let none = OpenNow::default();
    assert_eq!(groups(RibbonTab::Home, none), [vec![Panel::Viewport]]);
    let playing = OpenNow {
        playing: true,
        ..none
    };
    assert_eq!(
        groups(RibbonTab::Home, playing),
        [vec![Panel::Viewport, Panel::Game]]
    );
    assert_eq!(groups(RibbonTab::Library, none).len(), 1);
    let tileset = OpenNow {
        tileset: true,
        ..none
    };
    // A tileset is LIBRARY TOOLS · Tileset's own document, not the
    // Library's (round 3).
    assert_eq!(groups(RibbonTab::Library, tileset), [vec![Panel::Library]]);
    assert_eq!(groups(RibbonTab::Tileset, tileset), [vec![Panel::Tileset]]);
    assert!(groups(RibbonTab::Tileset, none).is_empty());
    assert!(groups(RibbonTab::File, EVERYTHING).is_empty());
    assert!(!built_shelf_tabs(RibbonTab::Home, none).contains(&ShelfTab::Map));
    let map = OpenNow { map: true, ..none };
    assert!(built_shelf_tabs(RibbonTab::Home, map).contains(&ShelfTab::Map));
}

#[test]
fn names_are_their_own_and_read_back() {
    for tab in ShelfTab::ALL {
        assert_eq!(ShelfTab::named(tab.name()), Some(tab));
    }
    assert_eq!(ShelfTab::named("Object"), None);
}

#[test]
fn each_tab_starts_on_its_first_working_shelf_tab() {
    assert_eq!(
        Arrangement::standard(RibbonTab::Home).shelf_tab,
        ShelfTab::Entity
    );
    assert_eq!(
        Arrangement::standard(RibbonTab::Library).shelf_tab,
        ShelfTab::Details
    );
    assert_eq!(
        Arrangement::standard(RibbonTab::Debug).shelf_tab,
        ShelfTab::Console
    );
}

#[test]
fn the_front_falls_back_to_the_first_there_is() {
    let mut a = Arrangement::standard(RibbonTab::Home);
    a.front[0] = Some(Panel::Game);
    assert_eq!(a.front_of(0, &[Panel::Viewport]), Some(Panel::Viewport));
    assert_eq!(
        a.front_of(0, &[Panel::Viewport, Panel::Game]),
        Some(Panel::Game)
    );
    assert_eq!(a.front_of(1, &[]), None);
    a.shelf_tab = ShelfTab::Map;
    assert_eq!(
        a.shelf_tab_of(&[ShelfTab::Entity, ShelfTab::Console]),
        Some(ShelfTab::Entity)
    );
}
