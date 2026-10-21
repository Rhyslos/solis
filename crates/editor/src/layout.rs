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

/// The saved arrangement if it is usable, otherwise the default one. A
/// layout saved by an older version that lacks a panel, or one that is
/// damaged, falls back to the default rather than hiding panels.
pub fn restore(saved: Option<DockState<Panel>>) -> DockState<Panel> {
    match saved {
        Some(dock) if is_complete(&dock) => dock,
        _ => Preset::Default.layout(),
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
    fn a_damaged_or_old_saved_layout_falls_back_to_the_default() {
        let mut missing = Preset::Default.layout();
        let path = missing.find_tab(&Panel::Console).unwrap();
        missing.remove_tab(path);
        assert!(!is_complete(&missing));
        let default = text(&Preset::Default.layout());
        assert_eq!(text(&restore(Some(missing))), default);
        assert_eq!(text(&restore(None)), default);
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
}
