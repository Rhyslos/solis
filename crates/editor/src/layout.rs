//! Arrangements of the dockable panels: the standard one (Reset layout
//! brings it back), and checks on a saved arrangement
//! ([decision 0004](../../../docs/decisions/0004-editor-layout.md)).

use crate::panel::Panel;
use egui_dock::{DockState, NodeIndex};

/// Share of the window width given to the viewport's side (the right
/// column with Area and Properties gets the rest).
const MAIN_AREA_WIDTH: f32 = 0.77;
/// Share of the right column given to Area and Layers.
const AREA_HEIGHT: f32 = 0.45;
/// Share of the main area given to the viewport (the bottom panels get the
/// rest).
const VIEWPORT_HEIGHT: f32 = 0.74;
/// Share of the bottom given to the library (the console gets the rest).
const LIBRARY_WIDTH: f32 = 0.6;
/// The standard arrangement: what a first start shows, and what Reset
/// layout brings back. There are no other presets: every panel docks
/// freely, so any arrangement is a drag away ([decision 0061](../../../docs/decisions/0061-settings-window-and-no-layout-presets.md)).
///
/// ```text
/// ┌──────────────────┬──────┐
/// │ Viewport · Game  │Area │
/// │                  ├──────┤
/// ├──────────┬───────┤Props │
/// │ Library  │Console│      │
/// └──────────┴───────┴──────┘
/// ```
///
/// Area shares its group with Layers, Library with Code and Console
/// with Problems and the Profiler, as in the mockup.
pub fn standard() -> DockState<Panel> {
    let mut dock = DockState::new(vec![Panel::Viewport, Panel::Game]);
    let tree = dock.main_surface_mut();
    let [main_area, right_column] = tree.split_right(
        NodeIndex::root(),
        MAIN_AREA_WIDTH,
        vec![Panel::Area, Panel::Layers],
    );
    tree.split_below(right_column, AREA_HEIGHT, vec![Panel::Properties]);
    let [_, bottom] = tree.split_below(
        main_area,
        VIEWPORT_HEIGHT,
        vec![Panel::Library, Panel::Code],
    );
    tree.split_right(
        bottom,
        LIBRARY_WIDTH,
        vec![
            Panel::Console,
            Panel::Problems,
            Panel::Profiler,
            Panel::Mixer,
        ],
    );
    dock
}

/// Whether every panel appears exactly once in `dock`, in any of its
/// surfaces (the main window or floating windows).
pub fn is_complete(dock: &DockState<Panel>) -> bool {
    Panel::ALL.iter().all(|panel| {
        dock.iter_all_tabs()
            .filter(|(_, tab)| *tab == panel)
            .count()
            == 1
    }) && dock
        .iter_all_tabs()
        .filter(|(_, tab)| !tab.comes_with_a_file())
        .count()
        == Panel::ALL.len()
}

/// Opens `panel` (one that comes with a file) in the tab group of
/// `beside`, in front; already open, it's brought to the front.
pub fn open_beside(dock: &mut DockState<Panel>, panel: Panel, beside: Panel) {
    if let Some(path) = dock.find_tab(&panel) {
        let _ = dock.set_active_tab(path);
        return;
    }
    match dock.find_tab(&beside) {
        Some(path) => {
            dock[path.surface][path.node].append_tab(panel);
            if let Some(path) = dock.find_tab(&panel) {
                let _ = dock.set_active_tab(path);
            }
        }
        None => dock.push_to_first_leaf(panel),
    }
}

/// Closes `panel`, if it's open.
pub fn close(dock: &mut DockState<Panel>, panel: Panel) {
    if let Some(path) = dock.find_tab(&panel) {
        let _ = dock.remove_tab(path);
    }
}

/// The saved arrangement, with any panel it lacks (a layout saved before
/// that panel existed) added back beside the panel it shares a tab group
/// with in the default layout, as decision 0004 says. A damaged layout,
/// one with a panel twice, or `None`, gives the default one.
pub fn restore(saved: Option<DockState<Panel>>) -> DockState<Panel> {
    let Some(mut dock) = saved else {
        return standard();
    };
    // Tabs that come with a file aren't kept: no file is open yet.
    for panel in Panel::OPEN_WITH_A_FILE {
        close(&mut dock, panel);
    }
    let twice = Panel::ALL.iter().any(|panel| {
        dock.iter_all_tabs()
            .filter(|(_, tab)| *tab == panel)
            .count()
            > 1
    });
    if twice || dock.iter_all_tabs().next().is_none() {
        return standard();
    }
    for panel in Panel::ALL {
        if dock.find_tab(&panel).is_none() {
            add_beside_its_group(&mut dock, panel);
        }
    }
    if is_complete(&dock) && draws(&dock) {
        dock
    } else {
        standard()
    }
}

/// Whether `dock` can be drawn. egui_dock trusts a layout's insides (which
/// window is in focus, which tab is in front, how splits divide) and stops
/// the program on a damaged one, so a saved layout is drawn once out of
/// sight before it's used.
fn draws(dock: &DockState<Panel>) -> bool {
    struct Blank;
    impl egui_dock::TabViewer for Blank {
        type Tab = Panel;
        fn id(&mut self, tab: &mut Panel) -> egui::Id {
            egui::Id::new(("trial", *tab))
        }
        fn title(&mut self, tab: &mut Panel) -> egui::WidgetText {
            tab.title().into()
        }
        fn ui(&mut self, _ui: &mut egui::Ui, _tab: &mut Panel) {}
    }
    let mut copy = dock.clone();
    let ctx = egui::Context::default();
    let input = || egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1440.0, 900.0),
        )),
        ..egui::RawInput::default()
    };
    // Silenced: a damaged layout is expected here, and isn't news. (Not in
    // tests, where the hook is shared with tests running alongside.)
    let hook = (!cfg!(test)).then(|| {
        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        hook
    });
    let drawn = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        for _ in 0..2 {
            let mut output = ctx.run_ui(input(), |ui| {
                egui_dock::DockArea::new(&mut copy).show_inside(ui, &mut Blank);
            });
            // Nothing is shown, so the fonts' pictures go nowhere.
            output.textures_delta.clear();
        }
    }))
    .is_ok();
    if let Some(hook) = hook {
        std::panic::set_hook(hook);
    }
    drawn
}

/// Adds `panel` to the tab group of a panel it shares a group with in the
/// default layout; or, if none of them is there, beside the viewport (or
/// in the first group).
fn add_beside_its_group(dock: &mut DockState<Panel>, panel: Panel) {
    let default = standard();
    let companions: Vec<Panel> = default
        .find_tab(&panel)
        .and_then(|path| {
            default[path.surface][path.node]
                .tabs()
                .map(<[Panel]>::to_vec)
        })
        .unwrap_or_default();
    let home = companions
        .iter()
        .chain(std::iter::once(&Panel::Viewport))
        .find_map(|companion| dock.find_tab(companion));
    match home {
        Some(path) => {
            dock[path.surface][path.node].append_tab(panel);
            // The tab that was in front stays in front.
            let _ = dock.set_active_tab(path);
        }
        None => dock.push_to_first_leaf(panel),
    }
}

/// The panel sharing a tab group with the viewport, if any (for tests and
/// checks).
pub fn beside_viewport(dock: &DockState<Panel>) -> Vec<Panel> {
    let Some(path) = dock.find_tab(&Panel::Viewport) else {
        return Vec::new();
    };
    dock[path.surface][path.node]
        .tabs()
        .map(|tabs| tabs.to_vec())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tab_group_of(dock: &DockState<Panel>, panel: Panel) -> Vec<Panel> {
        let path = dock.find_tab(&panel).unwrap();
        dock[path.surface][path.node].tabs().unwrap().to_vec()
    }

    /// The whole arrangement as text, to compare two of them.
    fn text(dock: &DockState<Panel>) -> String {
        ron::to_string(dock).unwrap()
    }

    /// A rearranged layout, as someone might drag one together: the
    /// Console moved in with the viewport.
    fn rearranged() -> DockState<Panel> {
        let mut dock = standard();
        let path = dock.find_tab(&Panel::Console).unwrap();
        dock.remove_tab(path);
        let viewport = dock.find_tab(&Panel::Viewport).unwrap();
        dock[viewport.surface][viewport.node].append_tab(Panel::Console);
        assert!(is_complete(&dock));
        dock
    }

    #[test]
    fn the_standard_layout_shows_every_panel_once() {
        assert!(is_complete(&standard()));
    }

    #[test]
    fn the_default_puts_library_and_console_side_by_side() {
        let dock = standard();
        assert_eq!(
            tab_group_of(&dock, Panel::Library),
            [Panel::Library, Panel::Code]
        );
        assert_eq!(
            tab_group_of(&dock, Panel::Console),
            [
                Panel::Console,
                Panel::Problems,
                Panel::Profiler,
                Panel::Mixer
            ]
        );
        assert_eq!(
            tab_group_of(&dock, Panel::Area),
            [Panel::Area, Panel::Layers]
        );
        assert_eq!(beside_viewport(&dock), [Panel::Viewport, Panel::Game]);
    }

    #[test]
    fn an_old_layout_gets_its_missing_panels_back_and_keeps_the_rest() {
        // A rearranged layout saved before Problems, Code and the
        // Profiler existed.
        let mut old = rearranged();
        for panel in [Panel::Problems, Panel::Code, Panel::Profiler] {
            let path = old.find_tab(&panel).unwrap();
            old.remove_tab(path);
        }
        assert!(!is_complete(&old));
        let restored = restore(Some(old));
        assert!(is_complete(&restored));
        // The rest of the arrangement is kept: still rearranged.
        assert!(beside_viewport(&restored).contains(&Panel::Console));
        assert_eq!(text(&restore(None)), text(&standard()));
    }

    #[test]
    fn a_missing_panel_goes_beside_the_viewport_when_its_group_is_gone() {
        let mut old = standard();
        for panel in [
            Panel::Console,
            Panel::Problems,
            Panel::Profiler,
            Panel::Mixer,
        ] {
            let path = old.find_tab(&panel).unwrap();
            old.remove_tab(path);
        }
        let restored = restore(Some(old));
        assert!(is_complete(&restored));
        let beside = beside_viewport(&restored);
        assert!(beside.contains(&Panel::Console), "{beside:?}");
    }

    #[test]
    fn a_complete_saved_layout_is_kept() {
        let mine = rearranged();
        assert_eq!(text(&restore(Some(mine.clone()))), text(&mine));
    }

    #[test]
    fn a_layout_with_a_panel_twice_is_not_complete() {
        let mut twice = standard();
        twice.push_to_focused_leaf(Panel::Console);
        assert!(!is_complete(&twice));
    }

    #[test]
    fn layouts_survive_saving_and_loading() {
        for dock in [standard(), rearranged()] {
            // RON is the format eframe saves settings in.
            let saved = text(&dock);
            let back: DockState<Panel> = ron::from_str(&saved).unwrap();
            assert!(is_complete(&back));
            assert_eq!(text(&back), saved);
        }
    }

    /// Random damage to a saved layout, thousands of times: reading it
    /// never crashes, and what comes back always has every panel exactly
    /// once.
    #[test]
    fn a_layout_that_cant_be_drawn_gives_the_default() {
        // Found by review: a window in focus that doesn't exist made
        // egui_dock stop the program at every start.
        let good = ron::to_string(&standard()).unwrap();
        assert!(good.contains("focused_surface:None"), "{good}");
        let bad = good.replace("focused_surface:None", "focused_surface:Some((3))");
        let parsed: DockState<Panel> = ron::from_str(&bad).unwrap();
        assert!(!draws(&parsed));
        assert_eq!(text(&restore(Some(parsed))), good);
        assert!(draws(&standard()));
    }

    #[test]
    fn random_damage_to_a_saved_layout_never_loses_panels() {
        let mut moved = rearranged();
        let path = moved.find_tab(&Panel::Console).unwrap();
        let tab = moved.remove_tab(path).unwrap();
        moved.push_to_focused_leaf(tab);
        let saved = ron::to_string(&moved).unwrap();
        let bytes = saved.as_bytes();
        let names: Vec<&str> = Panel::ALL.iter().map(|p| p.title()).collect();
        let mut seed: u64 = 11;
        let mut next = |n: usize| {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (seed >> 33) as usize % n.max(1)
        };
        let mut read_back = 0;
        for _ in 0..2_000 {
            let mut damaged = bytes.to_vec();
            for _ in 0..1 + next(3) {
                let at = next(damaged.len());
                let cut = next(6).min(damaged.len() - at);
                // Swap in another panel's name, a number, a bracket, or
                // nothing: the kinds of damage that still parse sometimes.
                let with: Vec<u8> = match next(5) {
                    0 => names[next(names.len())].as_bytes().to_vec(),
                    1 => b"99999".to_vec(),
                    2 => b"-1.5".to_vec(),
                    3 => b"]".to_vec(),
                    _ => Vec::new(),
                };
                damaged.splice(at..at + cut, with);
            }
            let text = String::from_utf8_lossy(&damaged);
            let parsed = ron::from_str::<DockState<Panel>>(&text).ok();
            read_back += usize::from(parsed.is_some());
            let restored = restore(parsed);
            assert!(is_complete(&restored), "{text}");
            assert!(draws(&restored), "{text}");
        }
        // Make sure the test really exercised damaged layouts that still
        // read back, not only ones refused outright.
        assert!(read_back > 0);
    }

    #[test]
    fn a_layout_with_a_panel_twice_is_not_used() {
        let mut dock = standard();
        dock.push_to_focused_leaf(Panel::Console);
        assert!(!is_complete(&dock));
        assert!(is_complete(&restore(Some(dock))));
    }
}
