//! Arrangements of the dockable panels: the three presets in
//! Window → Layouts, and checks on a saved arrangement
//! ([decision 0004](../../../docs/decisions/0004-editor-layout.md)).

use crate::panel::Panel;
use egui_dock::{DockState, NodeIndex};
use serde::{Deserialize, Serialize};

/// Share of the window width given to the viewport's side (the right
/// column with Scene and Properties gets the rest).
const MAIN_AREA_WIDTH: f32 = 0.77;
/// Share of the right column given to Scene and Layers.
const SCENE_HEIGHT: f32 = 0.45;
/// Share of the main area given to the viewport (the bottom panels get the
/// rest).
const VIEWPORT_HEIGHT: f32 = 0.74;
/// Share of the bottom given to the library (the console gets the rest).
const LIBRARY_WIDTH: f32 = 0.6;
/// Share of the main area given to the library column in the wide layout.
const LIBRARY_COLUMN_WIDTH: f32 = 0.2;

/// The presets in Window → Layouts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Preset {
    /// The library beside the console, under the viewport.
    #[default]
    Default,
    /// The library in a column on the left, the console under the viewport.
    WideScreen,
    /// The library and console as tabs in one panel under the viewport.
    SmallScreen,
}

impl Preset {
    /// Every preset, in menu order.
    pub const ALL: [Self; 3] = [Self::Default, Self::WideScreen, Self::SmallScreen];

    /// The preset's name, as the menu shows it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::WideScreen => "Wide screen",
            Self::SmallScreen => "Small screen",
        }
    }

    /// The arrangement.
    ///
    /// ```text
    /// Default                       Wide screen                  Small screen
    /// ┌──────────────────┬──────┐   ┌─────┬────────────┬──────┐   ┌──────────────────┬──────┐
    /// │ Viewport · Game  │Scene │   │     │  Viewport  │Scene │   │ Viewport · Game  │Scene │
    /// │                  ├──────┤   │Libr-│            ├──────┤   │                  ├──────┤
    /// ├──────────┬───────┤Props │   │ary  ├────────────┤Props │   ├──────────────────┤Props │
    /// │ Library  │Console│      │   │     │  Console   │      │   │ Library · Console│      │
    /// └──────────┴───────┴──────┘   └─────┴────────────┴──────┘   └──────────────────┴──────┘
    /// ```
    ///
    /// Scene shares its group with Layers, Library with Code and Console
    /// with Problems, as in the mockup.
    pub fn layout(self) -> DockState<Panel> {
        let mut dock = DockState::new(vec![Panel::Viewport, Panel::Game]);
        let tree = dock.main_surface_mut();
        let [main_area, right_column] = tree.split_right(
            NodeIndex::root(),
            MAIN_AREA_WIDTH,
            vec![Panel::Scene, Panel::Layers],
        );
        tree.split_below(right_column, SCENE_HEIGHT, vec![Panel::Properties]);
        match self {
            Self::Default => {
                let [_, bottom] = tree.split_below(
                    main_area,
                    VIEWPORT_HEIGHT,
                    vec![Panel::Library, Panel::Code],
                );
                tree.split_right(bottom, LIBRARY_WIDTH, vec![Panel::Console, Panel::Problems]);
            }
            Self::WideScreen => {
                let [_, view] = tree.split_left(
                    main_area,
                    LIBRARY_COLUMN_WIDTH,
                    vec![Panel::Library, Panel::Code],
                );
                tree.split_below(view, VIEWPORT_HEIGHT, vec![Panel::Console, Panel::Problems]);
            }
            Self::SmallScreen => {
                tree.split_below(
                    main_area,
                    VIEWPORT_HEIGHT,
                    vec![Panel::Library, Panel::Code, Panel::Console, Panel::Problems],
                );
            }
        }
        dock
    }
}

/// Whether every panel appears exactly once in `dock`, in any of its
/// surfaces (the main window or floating windows).
pub fn is_complete(dock: &DockState<Panel>) -> bool {
    Panel::ALL.iter().all(|panel| {
        dock.iter_all_tabs()
            .filter(|(_, tab)| *tab == panel)
            .count()
            == 1
    }) && dock.iter_all_tabs().count() == Panel::ALL.len()
}

/// The saved arrangement, with any panel it lacks (a layout saved before
/// that panel existed) added back beside the panel it shares a tab group
/// with in the default layout, as decision 0004 says. A damaged layout,
/// one with a panel twice, or `None`, gives the default one.
pub fn restore(saved: Option<DockState<Panel>>) -> DockState<Panel> {
    let Some(mut dock) = saved else {
        return Preset::Default.layout();
    };
    let twice = Panel::ALL.iter().any(|panel| {
        dock.iter_all_tabs()
            .filter(|(_, tab)| *tab == panel)
            .count()
            > 1
    });
    if twice || dock.iter_all_tabs().next().is_none() {
        return Preset::Default.layout();
    }
    for panel in Panel::ALL {
        if dock.find_tab(&panel).is_none() {
            add_beside_its_group(&mut dock, panel);
        }
    }
    if is_complete(&dock) {
        dock
    } else {
        Preset::Default.layout()
    }
}

/// Adds `panel` to the tab group of a panel it shares a group with in the
/// default layout; or, if none of them is there, beside the viewport (or
/// in the first group).
fn add_beside_its_group(dock: &mut DockState<Panel>, panel: Panel) {
    let default = Preset::Default.layout();
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

    #[test]
    fn every_preset_shows_every_panel_once() {
        for preset in Preset::ALL {
            assert!(is_complete(&preset.layout()), "{}", preset.name());
        }
    }

    #[test]
    fn the_default_puts_library_and_console_side_by_side() {
        let dock = Preset::Default.layout();
        assert_eq!(
            tab_group_of(&dock, Panel::Library),
            [Panel::Library, Panel::Code]
        );
        assert_eq!(
            tab_group_of(&dock, Panel::Console),
            [Panel::Console, Panel::Problems]
        );
        assert_eq!(
            tab_group_of(&dock, Panel::Scene),
            [Panel::Scene, Panel::Layers]
        );
        assert_eq!(beside_viewport(&dock), [Panel::Viewport, Panel::Game]);
    }

    #[test]
    fn the_small_screen_layout_tabs_library_and_console_together() {
        let dock = Preset::SmallScreen.layout();
        assert_eq!(
            tab_group_of(&dock, Panel::Library),
            [Panel::Library, Panel::Code, Panel::Console, Panel::Problems]
        );
    }

    #[test]
    fn the_wide_screen_layout_keeps_library_and_console_apart() {
        let dock = Preset::WideScreen.layout();
        assert_eq!(
            tab_group_of(&dock, Panel::Library),
            [Panel::Library, Panel::Code]
        );
        assert_eq!(
            tab_group_of(&dock, Panel::Console),
            [Panel::Console, Panel::Problems]
        );
        assert_ne!(text(&dock), text(&Preset::Default.layout()));
    }

    #[test]
    fn an_old_layout_gets_its_missing_panels_back_and_keeps_the_rest() {
        // A wide-screen layout saved before Problems and Code existed.
        let mut old = Preset::WideScreen.layout();
        for panel in [Panel::Problems, Panel::Code] {
            let path = old.find_tab(&panel).unwrap();
            old.remove_tab(path);
        }
        assert!(!is_complete(&old));
        let restored = restore(Some(old));
        assert!(is_complete(&restored));
        // The rest of the arrangement is kept: still wide screen.
        assert_eq!(text(&restored), text(&Preset::WideScreen.layout()));
        assert_eq!(text(&restore(None)), text(&Preset::Default.layout()));
    }

    #[test]
    fn a_missing_panel_goes_beside_the_viewport_when_its_group_is_gone() {
        let mut old = Preset::Default.layout();
        for panel in [Panel::Console, Panel::Problems] {
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
        let wide = Preset::WideScreen.layout();
        assert_eq!(text(&restore(Some(wide.clone()))), text(&wide));
    }

    #[test]
    fn a_layout_with_a_panel_twice_is_not_complete() {
        let mut twice = Preset::Default.layout();
        twice.push_to_focused_leaf(Panel::Console);
        assert!(!is_complete(&twice));
    }

    #[test]
    fn layouts_survive_saving_and_loading() {
        for preset in Preset::ALL {
            // RON is the format eframe saves settings in.
            let saved = text(&preset.layout());
            let back: DockState<Panel> = ron::from_str(&saved).unwrap();
            assert!(is_complete(&back));
            assert_eq!(text(&back), saved);
        }
    }

    /// Random damage to a saved layout, thousands of times: reading it
    /// never crashes, and what comes back always has every panel exactly
    /// once.
    #[test]
    fn random_damage_to_a_saved_layout_never_loses_panels() {
        let mut moved = Preset::WideScreen.layout();
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
            assert!(is_complete(&restore(parsed)), "{text}");
        }
        // Make sure the test really exercised damaged layouts that still
        // read back, not only ones refused outright.
        assert!(read_back > 0);
    }

    #[test]
    fn a_layout_with_a_panel_twice_is_not_used() {
        let mut dock = Preset::Default.layout();
        dock.push_to_focused_leaf(Panel::Console);
        assert!(!is_complete(&dock));
        assert!(is_complete(&restore(Some(dock))));
    }
}
