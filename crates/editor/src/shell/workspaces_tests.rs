//! The table of what each tab puts in its slots.

use super::*;

const EVERYTHING: OpenNow = OpenNow {
    playing: true,
    animator: true,
    tileset: true,
    atlas: true,
    clips: true,
    map: true,
    layout: PanelLayout::Standard,
    folders: false,
    module_page: true,
    table: true,
    translations: true,
};

/// Every panel `tab` shows anywhere with `open` open.
fn everywhere(tab: RibbonTab, open: OpenNow) -> Vec<Panel> {
    let mut all: Vec<Panel> = groups(tab, open).concat();
    for column in [left_slot(tab, open), right_slot(tab, open)]
        .into_iter()
        .flatten()
    {
        all.extend(column.panels);
    }
    for bottom in built_bottom_tabs(tab, open) {
        all.extend(bottom.panels());
    }
    all
}

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
        for panel in everywhere(tab, EVERYTHING) {
            assert!(!seen.contains(&panel), "{panel:?} twice on {tab:?}");
            seen.push(panel);
        }
    }
}

/// Board 09: each tab drawn there fills the four slots as it lists.
#[test]
fn each_tab_fills_the_same_four_slots_as_board_09_draws_them() {
    let none = OpenNow::default();
    let title = |c: Option<Column>| c.map(|c| c.title);
    let bottoms = |tab| built_bottom_tabs(tab, none);
    // Home: Level, the view, Properties, the Library under the view.
    assert_eq!(title(left_slot(RibbonTab::Home, none)), Some("Level"));
    assert_eq!(groups(RibbonTab::Home, none), [vec![Panel::Viewport]]);
    assert_eq!(title(right_slot(RibbonTab::Home, none)), Some("Properties"));
    assert_eq!(
        bottoms(RibbonTab::Home),
        [BottomTab::Library, BottomTab::Console, BottomTab::Problems]
    );
    // Library: the left slot empty, the browser, Details.
    assert_eq!(left_slot(RibbonTab::Library, none), None);
    assert_eq!(groups(RibbonTab::Library, none), [vec![Panel::Library]]);
    assert_eq!(title(right_slot(RibbonTab::Library, none)), Some("Details"));
    // Animate with a clips file and an animator: their lists left, the
    // preview, Properties right, the timeline under it.
    let animating = OpenNow {
        clips: true,
        animator: true,
        ..none
    };
    let left = left_slot(RibbonTab::Animate, animating).unwrap();
    assert_eq!(left.panels, [Panel::Clips, Panel::Parameters]);
    assert!(
        groups(RibbonTab::Animate, animating)
            .concat()
            .contains(&Panel::Preview)
    );
    assert_eq!(
        right_slot(RibbonTab::Animate, animating).map(|c| c.panels),
        Some(&[Panel::Properties][..])
    );
    assert_eq!(
        built_bottom_tabs(RibbonTab::Animate, animating)[0],
        BottomTab::Frames
    );
    // UI: the screen's elements, the canvas, Properties, the Library.
    assert_eq!(title(left_slot(RibbonTab::Ui, none)), Some("Elements"));
    assert_eq!(groups(RibbonTab::Ui, none), [vec![Panel::Canvas]]);
    assert_eq!(title(right_slot(RibbonTab::Ui, none)), Some("Properties"));
    assert_eq!(bottoms(RibbonTab::Ui)[0], BottomTab::Library);
    // Audio: the events, the mixer, the bus, the Library.
    assert_eq!(
        title(left_slot(RibbonTab::Audio, none)),
        Some("Sound events")
    );
    assert_eq!(groups(RibbonTab::Audio, none), [vec![Panel::Mixer]]);
    assert_eq!(title(right_slot(RibbonTab::Audio, none)), Some("Bus"));
    assert_eq!(bottoms(RibbonTab::Audio)[0], BottomTab::Library);
    // Debug: what is running is still to come; the profiler (with its
    // statistics) in the centre, the Console under it.
    assert!(left_slot(RibbonTab::Debug, none).unwrap().unbuilt.is_some());
    assert!(right_slot(RibbonTab::Debug, none).is_none());
    assert_eq!(groups(RibbonTab::Debug, none)[0][0], Panel::Profiler);
    assert_eq!(bottoms(RibbonTab::Debug)[0], BottomTab::Console);
}

#[test]
fn every_built_tab_has_a_bottom_panel_with_the_console() {
    for tab in RibbonTab::ALL.into_iter().chain(RibbonTab::CONTEXT) {
        if tab == RibbonTab::File || !tab.is_built() {
            continue;
        }
        assert!(
            built_bottom_tabs(tab, EVERYTHING).contains(&BottomTab::Console),
            "{tab:?}"
        );
    }
}

/// The old bottom bar's Animation tab went to the Animate tab, under the
/// preview (decision 0118).
#[test]
fn the_unfinished_animation_tab_moved_from_home_to_animate() {
    use crate::unbuilt::UnbuiltId;
    let animation = Entry::Unbuilt(UnbuiltId::HomeShelfAnimation);
    assert!(!bottom_tabs(RibbonTab::Home, EVERYTHING).contains(&animation));
    assert!(bottom_tabs(RibbonTab::Animate, EVERYTHING).contains(&animation));
}

#[test]
fn documents_and_tabs_come_and_go_with_what_is_open() {
    let none = OpenNow::default();
    let playing = OpenNow {
        playing: true,
        ..none
    };
    assert_eq!(
        groups(RibbonTab::Home, playing),
        [vec![Panel::Viewport, Panel::Game]]
    );
    let tileset = OpenNow {
        tileset: true,
        ..none
    };
    // A tileset is LIBRARY TOOLS · Tileset's own document, not the
    // Library's (round 3), its terrains on the left, the slot on the
    // right.
    assert_eq!(groups(RibbonTab::Library, tileset), [vec![Panel::Library]]);
    assert_eq!(groups(RibbonTab::Tileset, tileset), [vec![Panel::Tileset]]);
    assert_eq!(
        left_slot(RibbonTab::Tileset, tileset).map(|c| c.title),
        Some("Terrains")
    );
    assert!(groups(RibbonTab::Tileset, none).is_empty());
    assert!(left_slot(RibbonTab::Tileset, none).is_none());
    assert!(groups(RibbonTab::File, EVERYTHING).is_empty());
    assert!(!built_bottom_tabs(RibbonTab::Home, none).contains(&BottomTab::Map));
    let map = OpenNow { map: true, ..none };
    assert!(built_bottom_tabs(RibbonTab::Home, map).contains(&BottomTab::Map));
    assert!(left_slot(RibbonTab::Animate, none).is_none());
}

#[test]
fn names_are_their_own_and_read_back() {
    for tab in BottomTab::ALL {
        assert_eq!(BottomTab::named(tab.name()), Some(tab));
    }
    assert_eq!(BottomTab::named("Object"), None);
    // The shelf's tabs from before the frame that held the assets read
    // as the Library; the rest moved into slots.
    assert_eq!(BottomTab::named("Place"), Some(BottomTab::Library));
    assert_eq!(BottomTab::named("Entity"), Some(BottomTab::Library));
    assert_eq!(BottomTab::named("Screen"), None);
}

#[test]
fn each_tab_starts_on_its_first_bottom_tab_open_and_nothing_hidden() {
    let home = Arrangement::standard(RibbonTab::Home);
    assert_eq!(home.bottom_tab, BottomTab::Library);
    assert!(!home.bottom_folded && !home.hidden.any() && !home.placing);
    assert_eq!(
        Arrangement::standard(RibbonTab::Animate).bottom_tab,
        BottomTab::Frames
    );
    assert_eq!(
        Arrangement::standard(RibbonTab::Debug).bottom_tab,
        BottomTab::Console
    );
    // The Library tab's browser fills it (board 09).
    assert!(Arrangement::standard(RibbonTab::Library).bottom_folded);
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
    a.bottom_tab = BottomTab::Map;
    assert_eq!(
        a.bottom_tab_of(&[BottomTab::Library, BottomTab::Console]),
        Some(BottomTab::Library)
    );
    assert_eq!(a.bottom_tab_of(&[]), None);
}

#[test]
fn a_panel_s_home_follows_the_layout() {
    let open = OpenNow::default();
    assert_eq!(
        home_of(RibbonTab::Home, open, Panel::Level),
        Some(Home::Slot(Slot::Left))
    );
    assert_eq!(
        home_of(RibbonTab::Home, open, Panel::Assets),
        Some(Home::Bottom(BottomTab::Library))
    );
    assert_eq!(
        home_of(RibbonTab::Home, open, Panel::Placement),
        Some(Home::Bottom(BottomTab::Library))
    );
    for layout in [PanelLayout::LibraryLeft, PanelLayout::LibraryRight] {
        let open = OpenNow { layout, ..open };
        assert_eq!(
            home_of(RibbonTab::Home, open, Panel::Assets),
            Some(Home::Slot(Slot::Library)),
            "{layout:?}"
        );
    }
    assert_eq!(home_of(RibbonTab::Library, open, Panel::Level), None);
}

#[test]
fn hidden_slots_are_set_and_read_one_by_one() {
    let mut hidden = Hidden::default();
    assert!(!hidden.any());
    hidden.set(Slot::Right, true);
    assert!(hidden.has(Slot::Right) && !hidden.has(Slot::Left) && hidden.any());
    hidden.set(Slot::Right, false);
    assert_eq!(hidden, Hidden::default());
    for slot in [Slot::Left, Slot::Right, Slot::Library] {
        assert!(Hidden::ALL.has(slot));
    }
}
