//! The frame round the editor
//! ([decision 0099](../../../../docs/decisions/0099-the-g-layout.md)),
//! five bands from top to bottom: the tab bar, the ribbon, the editor
//! groups, the shelf and the status bar; or, on the File tab and an
//! unfinished screen, one page in place of the ribbon, groups and shelf.
//! Docking is Solis's own ([decision 0104](../../../../docs/decisions/0104-docking-is-solis-s-own.md)):
//! each tab's middle is fixed, as the design draws it.
//!
//! - [`RibbonTab`] is one tab of the tab bar, and [`Shell`] is what the
//!   frame remembers between frames: the tab in front, whether the ribbon
//!   is collapsed, each tab's arrangement of its middle. It is the
//!   editor's one field for all of it, saved as [`ShellState`]
//!   (`saved.rs`).
//! - `workspaces.rs` is the table of what each tab's middle holds;
//!   `layouts.rs` the layouts of Home's lists and where everything in the
//!   middle goes, `sizes.rs` what each border was dragged to;
//!   `groups.rs` draws the center's editor groups and their document
//!   tabs, `shelf.rs` the shelf, `beside.rs` the lists beside the groups
//!   and every border's drag, `status.rs` the status bar.
//! - `commands.rs` is the one table of what the editor can do by name
//!   (`ShellCommand`): the ribbon's buttons, the File page's items and
//!   the search all run its entries, and show its shortcuts.
//! - `tab_bar.rs` draws the top band; `ribbon.rs` draws a ribbon from a
//!   table, and `ribbons/` holds one small table for each tab;
//!   `play_group.rs` is the group at the ribbon's right end;
//!   `file_page.rs` is the File tab's menu column and page.

mod beside;
mod command_words;
pub(crate) mod commands;
mod controls;
mod create;
pub(crate) mod documentation;
pub(crate) mod file_page;
mod groups;
pub mod layouts;
mod play_group;
pub(crate) mod ribbon;
mod ribbons;
pub(crate) mod saved;
mod shelf;
pub mod sizes;
pub mod status;
pub(crate) mod tab_bar;
mod tab_drag;
pub mod workspaces;

pub(crate) use commands::ShellCommand;

/// The direction of index `i` in Animate's Direction list.
pub(crate) fn animate_direction(i: u8) -> Option<animation::Direction> {
    ribbons::animate::DIRECTIONS.get(usize::from(i)).copied()
}
pub use layouts::PanelLayout;
pub use saved::{ShellState, WorkspaceState};
use std::collections::BTreeMap;
use workspaces::Arrangement;
pub use workspaces::ShelfTab;

use crate::panel::Panel;
use crate::workbench::Workbench;
use workspaces::{Home, OpenNow};

/// One tab of the tab bar: an area of the engine, with a ribbon of its
/// own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RibbonTab {
    /// Projects, settings and leaving: a page in place of the editor.
    File,
    /// Building areas.
    Home,
    /// The project's files.
    Library,
    /// Materials and shaders (not built yet).
    Shading,
    /// Sprite clips and animators.
    Animate,
    /// Game screens, drawn on the canvas.
    Ui,
    /// Effects (not built yet).
    Vfx,
    /// The mixer.
    Audio,
    /// The profiler, the Console and Problems, with the game big.
    Debug,
    /// Exporting the game.
    Build,
    /// LIBRARY TOOLS · Tileset: a tileset open from the Library, there
    /// only while one is (a contextual tab, after Library).
    Tileset,
    /// LIBRARY TOOLS · Picture: a picture's crops open from the Library,
    /// there only while they are.
    Picture,
    /// LIBRARY TOOLS · Model: the model viewer (not built yet), there
    /// while a model is looked at from the Library's right-click menu.
    Model,
}

impl RibbonTab {
    /// Every tab, in the tab bar's order.
    pub const ALL: [Self; 10] = [
        Self::File,
        Self::Home,
        Self::Library,
        Self::Shading,
        Self::Animate,
        Self::Ui,
        Self::Vfx,
        Self::Audio,
        Self::Debug,
        Self::Build,
    ];

    /// The contextual tabs: each comes with a file open from its parent
    /// tab and goes with it (the approved additions, round 3).
    pub const CONTEXT: [Self; 3] = [Self::Tileset, Self::Picture, Self::Model];

    /// The tab a contextual tab belongs to; any other tab is its own.
    pub fn parent(self) -> Self {
        match self {
            Self::Tileset | Self::Picture | Self::Model => Self::Library,
            other => other,
        }
    }

    /// The tab's name.
    pub fn name(self) -> &'static str {
        match self {
            Self::File => "File",
            Self::Home => "Home",
            Self::Library => "Library",
            Self::Shading => "Shading",
            Self::Animate => "Animate",
            Self::Ui => "UI",
            Self::Vfx => "VFX",
            Self::Audio => "Audio",
            Self::Debug => "Debug",
            Self::Build => "Build",
            Self::Tileset => "Tileset",
            Self::Picture => "Picture",
            Self::Model => "Model",
        }
    }

    /// What the tab is for, on hover.
    pub fn help(self) -> &'static str {
        match self {
            Self::File => "Projects, settings, the manual and updates",
            Self::Home => "Building areas: making entities, snapping, saving and playing",
            Self::Library => "The project's files: bringing in, making, finding and placing them",
            Self::Shading => "Materials and shaders",
            Self::Animate => "Animating sprites: a clips file's clips and frames, and animators",
            Self::Ui => "Drawing game screens on the canvas",
            Self::Vfx => "Effects",
            Self::Audio => "The mixer: the project's buses and how loud each is",
            Self::Debug => {
                "Testing the game: where each frame's time goes, the Console and Problems"
            }
            Self::Build => "Exporting the game for someone to play",
            Self::Tileset => "The tileset open: its picture, terrains and slots",
            Self::Picture => "The picture's crops: parts of it named for game code and screens",
            Self::Model => "A model on its own, in the model viewer",
        }
    }

    /// Whether the tab's screen is built. One that isn't is drawn as an
    /// unfinished part: its tab, its ribbon and one page in the middle.
    pub fn is_built(self) -> bool {
        !matches!(self, Self::Shading | Self::Vfx | Self::Model)
    }
}

/// What the frame remembers between frames.
#[derive(Debug, Clone, PartialEq)]
pub struct Shell {
    /// The tab in front.
    pub(crate) tab: RibbonTab,
    /// The tab that was in front before File: Back to editor returns to
    /// it.
    pub(crate) before_file: RibbonTab,
    /// Whether the ribbon is collapsed to its tabs.
    pub(crate) collapsed: bool,
    /// While collapsed: whether a click on a tab has dropped the ribbon
    /// down over the center.
    pub(crate) dropped: bool,
    /// What each tab remembers of its middle; a tab not here is as the
    /// design draws it.
    pub(crate) arrangements: BTreeMap<RibbonTab, Arrangement>,
    /// What each border was dragged to, per tab and layout.
    pub(crate) sizes: sizes::Sizes,
    /// Whether the last frame was too narrow for the layout chosen, so
    /// Home's lists went back on the shelf (decision 0109).
    pub(crate) narrow: bool,
    /// Where each panel was drawn in the last frame: its body, under its
    /// tab or header.
    pub(crate) drawn: Vec<(Panel, egui::Rect)>,
    /// Whether a tile map was selected when the last frame was drawn:
    /// selecting one brings its Map tab to the front.
    pub(crate) seen_map: bool,
    /// The entity selected when the last frame was drawn: selecting
    /// another brings the shelf tab Preferences names to the front.
    pub(crate) seen_selected: Option<reflect::EntityId>,
    /// The model the model viewer was opened on (its file), while its
    /// tab is in front: it goes when another tab is chosen.
    pub(crate) viewing_model: Option<String>,
}

impl Default for Shell {
    fn default() -> Self {
        Self {
            tab: RibbonTab::Home,
            before_file: RibbonTab::Home,
            collapsed: false,
            dropped: false,
            arrangements: BTreeMap::new(),
            sizes: sizes::Sizes::default(),
            narrow: false,
            drawn: Vec::new(),
            seen_map: false,
            seen_selected: None,
            viewing_model: None,
        }
    }
}

impl Shell {
    /// Whether a whole page stands where the docked panels would be: a
    /// File page, or an unfinished tab's.
    pub(crate) fn covers_center(&self) -> bool {
        self.tab == RibbonTab::File || !self.tab.is_built()
    }

    /// Whether the ribbon is drawn in its own band, pushing the center
    /// down.
    pub(crate) fn ribbon_in_band(&self) -> bool {
        !self.collapsed && self.tab != RibbonTab::File
    }

    /// What `tab` remembers of its middle.
    pub(crate) fn arrangement(&self, tab: RibbonTab) -> Arrangement {
        self.arrangements
            .get(&tab)
            .copied()
            .unwrap_or_else(|| Arrangement::standard(tab))
    }

    /// What `tab` remembers of its middle, to change.
    pub(crate) fn arrangement_mut(&mut self, tab: RibbonTab) -> &mut Arrangement {
        self.arrangements
            .entry(tab)
            .or_insert_with(|| Arrangement::standard(tab))
    }

    /// `edge`'s size on `tab` in `layout`: as dragged, or its standard.
    pub(crate) fn size(&self, tab: RibbonTab, layout: PanelLayout, edge: sizes::Edge) -> f32 {
        self.sizes.get(tab, layout, edge)
    }

    /// Where `panel`'s body was drawn in the last frame, if it was.
    pub(crate) fn drawn_at(&self, panel: Panel) -> Option<egui::Rect> {
        self.drawn
            .iter()
            .find(|(p, _)| *p == panel)
            .map(|(_, rect)| *rect)
    }
}

impl Workbench {
    /// The tab in front.
    pub fn ribbon_tab(&self) -> RibbonTab {
        self.shell.tab
    }

    /// Brings `tab` to the front: its ribbon, and what it shows in the
    /// middle. A tab that isn't built can't be shown while unfinished
    /// parts are hidden.
    pub fn show_tab(&mut self, tab: RibbonTab) {
        if !tab.is_built() && self.interface_look().hide_unfinished {
            return;
        }
        if tab == RibbonTab::File {
            self.show_file_page(self.file_pages.page);
            return;
        }
        self.shell.tab = tab;
    }

    /// What is open now, which adds documents and shelf tabs.
    pub(crate) fn open_now(&self) -> OpenNow {
        let open = &self.open;
        OpenNow {
            playing: open.playing.is_some(),
            animator: open.animator_editor.is_some(),
            tileset: open.tilesets.tab().is_some(),
            atlas: open.atlas_editor.is_some(),
            clips: open.frames_editor.is_some(),
            map: open.map_edits.path.is_some(),
            layout: if self.shell.narrow {
                PanelLayout::Shelf
            } else {
                self.interface.layout
            },
        }
    }

    /// Brings `panel` into view: in front of its group or on its shelf
    /// tab, on the tab in front if it is there, or else on the first
    /// tab that has it.
    pub fn show_panel(&mut self, panel: Panel) {
        let open = self.open_now();
        let here = self.shell.tab;
        let found = workspaces::home_of(here, open, panel)
            .map(|home| (here, home))
            .or_else(|| {
                let tab = workspaces::tab_for(panel, open)?;
                Some((tab, workspaces::home_of(tab, open, panel)?))
            });
        let Some((tab, home)) = found else {
            return;
        };
        if tab != here {
            self.show_tab(tab);
        }
        self.show_in(tab, home, panel);
    }

    /// Puts `panel` in front at `home` on `tab`, whether or not `tab` is
    /// in front.
    pub(crate) fn show_in(&mut self, tab: RibbonTab, home: Home, panel: Panel) {
        let open = self.open_now();
        let arrangement = self.shell.arrangement_mut(tab);
        match home {
            Home::Group(_) => {
                // Its group as arranged now: a dragged tab may have moved
                // it to the other.
                let index = arrangement
                    .arranged(tab, open)
                    .iter()
                    .position(|docs| docs.contains(&panel))
                    .unwrap_or(0);
                if let Some(front) = arrangement.front.get_mut(index) {
                    *front = Some(panel);
                }
            }
            Home::Shelf(shelf) => {
                arrangement.shelf_tab = shelf;
                arrangement.shelf_folded = false;
            }
            // Always in view.
            Home::Beside => {}
        }
    }

    /// The panels in view now: the documents in front and the shelf tab's
    /// columns, as the last frame drew them.
    pub fn panels_shown(&self) -> Vec<Panel> {
        self.shell.drawn.iter().map(|(panel, _)| *panel).collect()
    }

    /// Folds the shelf of the tab in front to its bar, or opens it again.
    pub fn fold_shelf(&mut self, folded: bool) {
        let tab = self.shell.tab;
        self.shell.arrangement_mut(tab).shelf_folded = folded;
    }

    /// Whether the shelf of the tab in front is folded.
    pub fn shelf_folded(&self) -> bool {
        self.shell.arrangement(self.shell.tab).shelf_folded
    }

    /// Whether the editor is placing: Home's Place shelf tab is in front
    /// (the project lead's answer 20). The ribbon takes its build form
    /// and the tool bar goes.
    pub fn placing(&self) -> bool {
        self.shell.tab == RibbonTab::Home && self.shelf_tab() == Some(workspaces::ShelfTab::Place)
    }

    /// Whether HOME TOOLS · Tile map is the tab in front: a tile map is
    /// selected and its Map shelf tab is in front (round 3). The ribbon
    /// is the map's tools then, as Place in front is the build form.
    pub(crate) fn map_tools_in_front(&self) -> bool {
        self.shell.tab == RibbonTab::Home && self.shelf_tab() == Some(workspaces::ShelfTab::Map)
    }

    /// Whether HOME TOOLS · Brush is the tab in front: a brush's insides
    /// are being edited and Home's own ribbon hasn't been asked for
    /// (Tools 1a, B3).
    pub(crate) fn brush_tools_in_front(&self) -> bool {
        self.shell.tab == RibbonTab::Home
            && self
                .viewport
                .brushing
                .edit
                .as_ref()
                .is_some_and(|edit| !edit.home_ribbon)
    }

    /// Brings HOME TOOLS · Brush to the front, with the view.
    pub(crate) fn show_brush_tools(&mut self) {
        if let Some(edit) = self.viewport.brushing.edit.as_mut() {
            edit.home_ribbon = false;
        }
        self.show_panel(Panel::Viewport);
        self.show_tab(RibbonTab::Home);
    }

    /// The shelf tab in front on the tab in front, if it has a shelf.
    pub fn shelf_tab(&self) -> Option<workspaces::ShelfTab> {
        let tabs = workspaces::built_shelf_tabs(self.shell.tab, self.open_now());
        self.shell.arrangement(self.shell.tab).shelf_tab_of(&tabs)
    }

    /// Ctrl + Space folds the shelf or opens it again (spec 157). Works
    /// while typing too, as the Library's drawer did on the same key;
    /// not while the game has the keys, a question waits for its answer,
    /// or a key is being bound.
    pub(crate) fn shelf_keys(&mut self, ui: &egui::Ui) {
        if !self.keys.is_editor() || self.shell.covers_center() {
            return;
        }
        let pressed = ui.input_mut(|i| {
            i.consume_shortcut(&egui::KeyboardShortcut::new(
                egui::Modifiers::COMMAND,
                egui::Key::Space,
            ))
        });
        if pressed {
            self.fold_shelf(!self.shelf_folded());
        }
    }

    /// A tile map selected brings Home's Map shelf tab to the front, as
    /// the Map tab came to the front beside Properties before. (The
    /// design's "Shelf opens on", which opens a chosen tab whenever
    /// something is selected, waits for its setting in Preferences: on
    /// its own it would take the Console away from a command that
    /// selects.)
    pub(crate) fn follow_selection(&mut self) {
        let selected = self.open.document.selection.primary();
        let was = std::mem::replace(&mut self.shell.seen_selected, selected);
        if selected.is_some()
            && selected != was
            && self.interface_look().shelf_opens_on == crate::interface_look::ShelfOpensOn::Entity
            && let Some(home) =
                workspaces::home_of(RibbonTab::Home, self.open_now(), Panel::Properties)
        {
            // In a side layout, Properties is always in view.
            self.show_in(RibbonTab::Home, home, Panel::Properties);
        }
        self.follow_map_selection();
        let has_map = self.open.map_edits.path.is_some();
        let had_map = std::mem::replace(&mut self.shell.seen_map, has_map);
        if has_map && !had_map {
            self.show_in(
                RibbonTab::Home,
                Home::Shelf(workspaces::ShelfTab::Map),
                Panel::Map,
            );
        }
    }

    /// Does what the document tabs' menus asked for.
    pub(crate) fn doc_asks(
        &mut self,
        asks: groups::DocAsks,
        ctx: &egui::Context,
        actions: &mut Vec<crate::WorkbenchAction>,
    ) {
        for command in asks.commands {
            command.run(self, ctx, actions);
        }
        if asks.hide_unfinished {
            self.hide_unfinished(ctx, true);
        }
        if asks.show_area_in_library {
            let root = self.open.project.as_ref().map(|p| p.root.clone());
            let file = self.open.document.file().map(std::path::Path::to_path_buf);
            if let (Some(root), Some(file)) = (root, file)
                && let Ok(inside) = file.strip_prefix(&root)
            {
                let path = inside.to_string_lossy().replace('\\', "/");
                self.pick_in_library(&path);
            }
            self.show_tab(RibbonTab::Library);
        }
    }

    /// A tab hidden with the unfinished parts can't stay in front: Home
    /// takes its place. Called at the start of each frame.
    pub(crate) fn settle_tab(&mut self) {
        if !self.shell.tab.is_built() && self.interface_look().hide_unfinished {
            self.show_tab(RibbonTab::Home);
        }
        // A contextual tab goes with its file: its parent takes its place.
        let open = self.open_now();
        let gone = match self.shell.tab {
            RibbonTab::Tileset => !open.tileset,
            RibbonTab::Picture => !open.atlas,
            RibbonTab::Model => self.shell.viewing_model.is_none(),
            _ => false,
        };
        // The model viewer is looked into, not kept open: it goes once
        // another tab is in front.
        if self.shell.tab != RibbonTab::Model {
            self.shell.viewing_model = None;
        }
        if gone {
            self.shell.tab = self.shell.tab.parent();
        }
    }

    /// Collapses the ribbon to its tabs, or brings it back.
    pub fn set_ribbon_collapsed(&mut self, collapsed: bool) {
        self.shell.collapsed = collapsed;
        self.shell.dropped = false;
    }

    /// Whether the ribbon is collapsed to its tabs.
    pub fn ribbon_collapsed(&self) -> bool {
        self.shell.collapsed
    }

    /// A tab was clicked: it comes to the front, and a collapsed ribbon
    /// drops down over the center until something on it is chosen or the
    /// pointer leaves it.
    pub(crate) fn tab_clicked(&mut self, tab: RibbonTab) {
        if tab == self.shell.tab && self.brush_tools_in_front() {
            // Home's own ribbon, while the brush is still edited: its
            // tools tab brings them back.
            if let Some(edit) = self.viewport.brushing.edit.as_mut() {
                edit.home_ribbon = true;
            }
            return;
        }
        if tab == self.shell.tab && self.map_tools_in_front() {
            // Home again, from its tile map's tools: the Entity tab, or in
            // a side layout (which has none) the Console.
            let open = self.open_now();
            let shelf = workspaces::built_shelf_tabs(tab, open)
                .into_iter()
                .find(|s| *s == workspaces::ShelfTab::Entity)
                .unwrap_or(workspaces::ShelfTab::Console);
            self.show_in(tab, Home::Shelf(shelf), Panel::Properties);
            return;
        }
        let again = self.shell.tab == tab;
        self.show_tab(tab);
        if self.shell.collapsed && self.shell.tab != RibbonTab::File {
            // A second click on the tab in front puts the ribbon away.
            self.shell.dropped = !(again && self.shell.dropped);
        }
    }

    /// The tab bar, and under it the ribbon or the File page. Returns
    /// whether a whole page was drawn in the middle: the groups and the
    /// shelf aren't, then.
    pub(crate) fn top(
        &mut self,
        ui: &mut egui::Ui,
        actions: &mut Vec<crate::WorkbenchAction>,
    ) -> bool {
        self.tab_bar(ui);
        self.ribbon_band(ui, actions);
        self.search_list(ui, actions);
        match self.shell.tab {
            RibbonTab::File => self.file_page(ui, actions),
            RibbonTab::Shading => self.unbuilt_screen(ui, RibbonTab::Shading),
            RibbonTab::Vfx => self.unbuilt_screen(ui, RibbonTab::Vfx),
            RibbonTab::Model => self.unbuilt_screen(ui, RibbonTab::Model),
            _ => return false,
        }
        true
    }

    /// A whole screen that isn't built (D2; round 3's `11-unbuilt-*`):
    /// its one page where the groups would be, and under it its shelf
    /// folded to its bar, the shelf's tabs as unfinished parts.
    fn unbuilt_screen(&mut self, ui: &mut egui::Ui, tab: RibbonTab) {
        use crate::icons::Icon;
        use crate::unbuilt::{Entry, UnbuiltId};
        let shelf: Vec<Entry<()>> = match tab {
            RibbonTab::Shading => vec![
                Entry::Unbuilt(UnbuiltId::ShadingShelfParameters),
                Entry::Unbuilt(UnbuiltId::ShadingShelfNode),
                Entry::Unbuilt(UnbuiltId::ShadingShelfPalette),
                Entry::Unbuilt(UnbuiltId::ShadingShelfInstances),
            ],
            RibbonTab::Vfx => vec![
                Entry::Unbuilt(UnbuiltId::VfxShelfModule),
                Entry::Unbuilt(UnbuiltId::VfxShelfParameters),
                Entry::Unbuilt(UnbuiltId::VfxShelfPerformance),
            ],
            _ => vec![
                Entry::Unbuilt(UnbuiltId::MeshShelfOverview),
                Entry::Unbuilt(UnbuiltId::MeshShelfLods),
                Entry::Unbuilt(UnbuiltId::MeshShelfMaterials),
                Entry::Unbuilt(UnbuiltId::MeshShelfCollision),
                Entry::Unbuilt(UnbuiltId::MeshShelfSockets),
                Entry::Unbuilt(UnbuiltId::MeshShelfImport),
            ],
        };
        egui::Panel::bottom("unbuilt shelf")
            .frame(
                egui::Frame::new()
                    .fill(crate::tokens::shelf_bar())
                    .inner_margin(egui::Margin::symmetric(
                        crate::tokens::size::ROW_PAD as i8,
                        0,
                    )),
            )
            .exact_size(crate::tokens::size::SHELF_BAR_H)
            .show_separator_line(false)
            .show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    for entry in crate::unbuilt::filter(shelf) {
                        if let Entry::Unbuilt(part) = entry {
                            crate::unbuilt::shelf_tab(ui, part);
                        }
                    }
                });
            });
        let answer = egui::CentralPanel::default()
            .frame(egui::Frame::new())
            .show(ui, |ui| match tab {
                RibbonTab::Shading => {
                    crate::unbuilt::page(ui, UnbuiltId::ShadingGraph, Icon::Material)
                }
                RibbonTab::Vfx => crate::unbuilt::page(ui, UnbuiltId::VfxPreview, Icon::Sun),
                _ => crate::unbuilt::page(ui, UnbuiltId::MeshTab, Icon::Model),
            })
            .inner;
        if answer.hide {
            self.hide_unfinished(ui.ctx(), true);
        }
    }
}

/// The groups and the shelf of the tab in front, in what is left of
/// `ui` under the ribbon, with Home's lists beside them in `chosen`, the
/// layout chosen in Preferences, when the window has room for it (else
/// on the shelf). Notes where each panel was drawn.
pub(crate) fn middle(
    ui: &mut egui::Ui,
    shell: &mut Shell,
    open: OpenNow,
    chosen: PanelLayout,
    viewer: &mut crate::panels::PanelViewer<'_>,
) -> groups::DocAsks {
    let rect = ui.available_rect_before_wrap();
    shell.narrow = rect.width() < layouts::least_width(chosen);
    let layout = if shell.narrow {
        PanelLayout::Shelf
    } else {
        chosen
    };
    let open = OpenNow { layout, ..open };
    let beside = shell.tab == RibbonTab::Home && layout.is_side();
    let reserve = if beside {
        layouts::strip_reserve(layout)
    } else {
        0.0
    };
    let shelf_h = shelf::height(shell, rect.height(), layout, reserve);
    let upper = rect.with_max_y(rect.bottom() - shelf_h);
    shell.drawn.clear();
    ui.painter()
        .rect_filled(rect, 0.0, crate::tokens::viewport());
    let place = beside.then(|| {
        let tab = shell.tab;
        layouts::side(upper, layout, |edge| shell.size(tab, layout, edge))
    });
    let center = place.as_ref().map_or(upper, |p| p.center);
    let asks = groups::center(ui, center, shell, open, viewer);
    // The shelf before the lists: Assets takes its placing form beside
    // Placement drawn this frame.
    shelf::shelf(ui, rect.with_min_y(upper.bottom()), shell, open, viewer);
    if let Some(place) = place {
        beside::lists(ui, upper, &place, shell, layout, viewer);
    }
    ui.advance_cursor_after_rect(rect);
    asks
}

#[cfg(test)]
pub(crate) mod layout_scale_tests;
#[cfg(test)]
mod layout_tests;
#[cfg(test)]
mod middle_picture_tests;
#[cfg(test)]
mod middle_tests;
#[cfg(test)]
mod picture_tests;
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
