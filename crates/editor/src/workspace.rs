//! Workspaces, one per job (the editor overhaul's O6, decision 0085): tabs
//! in the top bar switch the whole layout at once, and each workspace
//! remembers how you arranged it. **Level** is for building areas;
//! **Animate** gives sprite sheets' clips and frames and the animator room;
//! **Code** is a testing layout to keep beside Ledger, where game code is
//! written: the game big, the code files, and the Console and Problems
//! side by side. A **UI** workspace comes with the game UI editor.
//!
//! A file's panels (an animator, a clips file's Clips and Frames, a
//! tileset, a map's layers) go with you: switching moves them into the
//! workspace switched to.

use crate::layout;
use crate::panel::Panel;
use crate::workbench::Workbench;
use egui_dock::{DockState, NodeIndex};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One workspace.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
pub enum Workspace {
    /// Building areas: the standard layout.
    #[default]
    Level,
    /// Sprite sheets' clips and frames, and animators.
    Animate,
    /// Testing game code written in Ledger.
    Code,
}

impl Workspace {
    /// Every workspace, in the top bar's order.
    pub const ALL: [Self; 3] = [Self::Level, Self::Animate, Self::Code];

    /// Its tab's name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Level => "Level",
            Self::Animate => "Animate",
            Self::Code => "Code",
        }
    }

    /// What it's for, on hover.
    pub fn help(self) -> &'static str {
        match self {
            Self::Level => {
                "Building areas: the viewport big, the Area panel, Properties and the Library"
            }
            Self::Animate => {
                "Animating sprites: room for a clips file's Clips and Frames, and for an animator \
                 beside the viewport"
            }
            Self::Code => {
                "Testing game code beside Ledger: the game big, your code files, and the Console \
                 and Problems side by side"
            }
        }
    }

    /// Its symbol on the tab.
    pub fn icon(self) -> crate::icons::Icon {
        use crate::icons::Icon;
        match self {
            Self::Level => Icon::AreaFile,
            Self::Animate => Icon::Animator,
            Self::Code => Icon::Code,
        }
    }

    /// Its layout when it's new, or after Reset layout.
    pub fn standard(self) -> DockState<Panel> {
        match self {
            Self::Level => layout::standard(),
            Self::Animate => animate(),
            Self::Code => code(),
        }
    }

    fn fallback(self) -> fn() -> DockState<Panel> {
        match self {
            Self::Level => layout::standard,
            Self::Animate => animate,
            Self::Code => code,
        }
    }
}

/// Animate's standard layout: the viewport (an animator opens beside it),
/// Properties on the right (Clips opens beside it), the Library on the left
/// to pick sheets and animators from, and a taller bottom for the Frames
/// ruler.
///
/// ```text
/// ┌─────────┬────────────────────┬───────┐
/// │ Library │ Viewport · Game    │ Props │
/// │ Area    ├────────────────────┤       │
/// │         │ Console · Problems │       │
/// └─────────┴────────────────────┴───────┘
/// ```
pub fn animate() -> DockState<Panel> {
    let mut dock = DockState::new(vec![Panel::Viewport, Panel::Game]);
    let tree = dock.main_surface_mut();
    let [main, _] = tree.split_right(NodeIndex::root(), 0.765, vec![Panel::Properties]);
    let [top, _] = tree.split_below(
        main,
        0.6,
        vec![
            Panel::Console,
            Panel::Problems,
            Panel::Mixer,
            Panel::Profiler,
        ],
    );
    tree.split_left(top, 0.25, vec![Panel::Library, Panel::Area, Panel::Code]);
    dock
}

/// Code's standard layout: the game big in the middle, the code files on
/// the left, Properties on the right (game code's values, live while
/// playing), and the Console and Problems side by side along the bottom.
///
/// ```text
/// ┌──────┬────────────────────┬───────┐
/// │ Code │ Game · Viewport    │ Props │
/// │      ├─────────┬──────────┤       │
/// │      │ Console │ Problems │       │
/// └──────┴─────────┴──────────┴───────┘
/// ```
pub fn code() -> DockState<Panel> {
    let mut dock = DockState::new(vec![Panel::Game, Panel::Viewport]);
    let tree = dock.main_surface_mut();
    let [main, _] = tree.split_right(NodeIndex::root(), 0.765, vec![Panel::Properties]);
    let [top, bottom] = tree.split_below(
        main,
        0.66,
        vec![Panel::Console, Panel::Mixer, Panel::Profiler],
    );
    tree.split_right(bottom, 0.5, vec![Panel::Problems]);
    tree.split_left(top, 0.22, vec![Panel::Code, Panel::Library, Panel::Area]);
    dock
}

/// Every workspace's layout and which one is in use, as saved between
/// runs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspaces {
    /// The one in use.
    pub current: Workspace,
    /// Each one's layout (the one in use's as it is now).
    pub layouts: BTreeMap<Workspace, DockState<Panel>>,
}

impl Workbench {
    /// The workspace in use.
    pub fn workspace(&self) -> Workspace {
        self.workspace
    }

    /// Every workspace's layout, for saving.
    pub fn workspaces(&self) -> Workspaces {
        let mut layouts = self.other_layouts.clone();
        layouts.insert(self.workspace, self.dock.clone());
        Workspaces {
            current: self.workspace,
            layouts,
        }
    }

    /// Takes in the workspaces saved last time: each layout checked as a
    /// saved layout is (a damaged one becomes its standard), and the one in
    /// use shown.
    pub fn set_workspaces(&mut self, saved: Workspaces) {
        let mut layouts = BTreeMap::new();
        for (workspace, dock) in saved.layouts {
            layouts.insert(
                workspace,
                layout::restore_with(Some(dock), workspace.fallback()),
            );
        }
        self.workspace = saved.current;
        if let Some(dock) = layouts.remove(&saved.current) {
            self.dock = dock;
        }
        self.other_layouts = layouts;
        self.fit_library_pin();
    }

    /// Switches to `workspace`: its layout (as you left it, or its
    /// standard the first time), with the panels of files open carried
    /// over.
    pub fn switch_workspace(&mut self, workspace: Workspace) {
        if workspace == self.workspace {
            return;
        }
        // The files' panels go with you.
        let open: Vec<Panel> = Panel::OPEN_WITH_A_FILE
            .into_iter()
            .filter(|p| self.dock.find_tab(p).is_some())
            .collect();
        let mut leaving = self.dock.clone();
        for panel in &open {
            layout::close(&mut leaving, *panel);
        }
        self.other_layouts.insert(self.workspace, leaving);
        self.dock = self
            .other_layouts
            .remove(&workspace)
            .unwrap_or_else(|| workspace.standard());
        self.workspace = workspace;
        for panel in open {
            let beside = match panel {
                Panel::Frames => Panel::Console,
                Panel::Clips | Panel::Map => Panel::Properties,
                _ => Panel::Viewport,
            };
            layout::open_beside(&mut self.dock, panel, beside);
        }
        self.fit_library_pin();
    }

    /// For each code file, the components written in it and how many
    /// entities in the area use each: `Player · 1, Crop · 12`, or the
    /// components alone when none is used. For the Code panel, so the Code
    /// workspace shows what each file does in this area.
    pub(crate) fn code_uses(&self) -> BTreeMap<String, String> {
        let Some(program) = &self.program else {
            return BTreeMap::new();
        };
        let mut by_file: BTreeMap<String, Vec<(String, usize)>> = BTreeMap::new();
        for (component, (file, _)) in &program.places {
            let users = self
                .document
                .area()
                .iter()
                .filter(|(_, e)| e.has(component))
                .count();
            by_file
                .entry(file.clone())
                .or_default()
                .push((component.clone(), users));
        }
        by_file
            .into_iter()
            .map(|(file, components)| {
                let words: Vec<String> = components
                    .into_iter()
                    .map(|(name, users)| format!("{name} · {users}"))
                    .collect();
                (file, words.join(", "))
            })
            .collect()
    }

    /// Puts the workspace in use back as it started.
    pub fn reset_workspace(&mut self) {
        self.dock = self.workspace.standard();
        self.fit_library_pin();
    }

    /// An unpinned Library stays out of every workspace's layout.
    fn fit_library_pin(&mut self) {
        if !self.library_pinned {
            layout::close(&mut self.dock, Panel::Library);
        }
    }

    /// The workspace tabs in the top bar: each its icon and name, the one
    /// in use lit with the accent's underline.
    pub(crate) fn workspace_tabs(&mut self, ui: &mut egui::Ui) {
        use crate::theme;
        let mut picked = None;
        for workspace in Workspace::ALL {
            let on = workspace == self.workspace;
            let font = egui::FontId::proportional(13.0);
            let width = ui.fonts_mut(|f| {
                f.layout_no_wrap(workspace.name().to_string(), font.clone(), theme::TEXT)
                    .size()
                    .x
            }) + 34.0;
            let (rect, tab) = ui.allocate_exact_size(egui::vec2(width, 26.0), egui::Sense::click());
            let colour = if on || tab.hovered() {
                theme::TEXT_STRONG
            } else {
                theme::TEXT_WEAK
            };
            let painter = ui.painter();
            if on {
                painter.rect_filled(rect, 4.0, theme::FIELD);
                painter.hline(
                    rect.x_range().shrink(6.0),
                    rect.bottom() - 1.0,
                    egui::Stroke::new(2.0, self.accent.colors().fill),
                );
            }
            let icon = egui::Rect::from_center_size(
                rect.left_center() + egui::vec2(14.0, 0.0),
                egui::vec2(12.0, 12.0),
            );
            crate::icons::icon(painter, icon, workspace.icon(), colour);
            painter.text(
                rect.left_center() + egui::vec2(26.0, 0.0),
                egui::Align2::LEFT_CENTER,
                workspace.name(),
                font,
                colour,
            );
            let name = format!("{} workspace", workspace.name());
            tab.widget_info(|| {
                egui::WidgetInfo::selected(egui::WidgetType::Button, true, on, &name)
            });
            if tab.on_hover_text(workspace.help()).clicked() {
                picked = Some(workspace);
            }
        }
        if let Some(workspace) = picked {
            self.switch_workspace(workspace);
        }
    }
}

#[cfg(test)]
#[path = "workspace_tests.rs"]
mod tests;
