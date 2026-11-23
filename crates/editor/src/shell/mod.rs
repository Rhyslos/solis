//! The frame round the editor
//! ([decision 0099](../../../../docs/decisions/0099-the-g-layout.md),
//! [decision 0118](../../../../docs/decisions/0118-the-editor-rethink.md)):
//! the tab bar, the ribbon, then one frame on every tab (a left slot and a
//! right slot running full height, the centre between them, and the
//! bottom panel under the centre only), then the status line; or, on the
//! File tab and an unfinished screen, one page in place of the ribbon and
//! the frame. Docking is Solis's own ([decision 0104](../../../../docs/decisions/0104-docking-is-solis-s-own.md)):
//! each tab fills the same slots with its own panels.
//!
//! - [`RibbonTab`] is one tab of the tab bar, and [`Shell`] is what the
//!   frame remembers between frames: the tab in front, whether the ribbon
//!   is collapsed, each tab's arrangement of its frame. It is the
//!   editor's one field for all of it, saved as [`ShellState`]
//!   (`saved.rs`).
//! - `workspaces.rs` is the table of what each tab puts in its slots;
//!   `layouts.rs` the layouts and where everything in the middle goes,
//!   `sizes.rs` what each edge was dragged to; `groups.rs` draws the
//!   centre's editor groups and their document tabs, `bottom.rs` the
//!   bottom panel, `columns.rs` the side columns and every edge's drag,
//!   `status.rs` the status line.
//! - `commands.rs` is the one table of what the editor can do by name
//!   (`ShellCommand`): the ribbon's buttons, the File page's items and
//!   the search all run its entries, and show its shortcuts.
//! - `tab_bar.rs` draws the top band; `ribbon.rs` draws a ribbon from a
//!   table, and `ribbons/` holds one small table for each tab;
//!   `play_group.rs` is the group at the ribbon's right end;
//!   `file_page.rs` is the File tab's menu column and page.

mod bottom;
mod columns;
mod command_words;
pub(crate) mod commands;
mod controls;
mod create;
pub(crate) mod documentation;
pub(crate) mod file_page;
mod groups;
mod kind_note;
mod layout_choice;
pub mod layouts;
mod play_group;
pub(crate) mod ribbon;
mod ribbons;
pub(crate) mod saved;
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
pub use workspaces::{BottomTab, Slot};

use crate::panel::Panel;
use crate::unbuilt::Entry;
use crate::workbench::Workbench;
use workspaces::{Home, OpenNow};

/// One tab of the tab bar: a level of the engine, with a ribbon of its
/// own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RibbonTab {
    /// Projects, settings and leaving: a page in place of the editor.
    File,
    /// Building levels.
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
            Self::Home => "Building levels: making entities, snapping, saving and playing",
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
    /// What each edge was dragged to, per tab and layout.
    pub(crate) sizes: sizes::Sizes,
    /// Where each panel was drawn in the last frame: its body, under its
    /// tab or header.
    pub(crate) drawn: Vec<(Panel, egui::Rect)>,
    /// Whether a tile map was selected when the last frame was drawn:
    /// selecting one brings its Map tab to the front.
    pub(crate) seen_map: bool,
    /// The entity selected when the last frame was drawn: selecting
    /// another shows Properties again, if Preferences says so.
    pub(crate) seen_selected: Option<reflect::EntityId>,
    /// The model the model viewer was opened on (its file), while its
    /// tab is in front: it goes when another tab is chosen.
    pub(crate) viewing_model: Option<String>,
    /// What the last layout chosen changed, while its note offers Undo.
    pub(crate) layout_undo: Option<layout_choice::LayoutUndo>,
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
            drawn: Vec::new(),
            seen_map: false,
            seen_selected: None,
            viewing_model: None,
            layout_undo: None,
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
        // A tab of the kind the project hides isn't there (R6).
        if !self.open.kind.shows(crate::kinds::tab(tab)) {
            return;
        }
        if tab == RibbonTab::File {
            self.show_file_page(self.file_pages.page);
            return;
        }
        self.shell.tab = tab;
    }

    /// What is open now, which adds documents, slots and bottom tabs.
    pub(crate) fn open_now(&self) -> OpenNow {
        let open = &self.open;
        OpenNow {
            playing: open.playing.is_some(),
            animator: open.animator_editor.is_some(),
            tileset: open.tilesets.tab().is_some(),
            atlas: open.atlas_editor.is_some(),
            clips: open.frames_editor.is_some(),
            map: open.map_edits.path.is_some(),
            layout: self.interface.layout,
            folders: self.open.library_state.left == crate::library::LeftShows::Folders,
            module_page: self.open.module_page.is_some(),
            table: self.open.table_editor.is_some(),
            translations: self.open.translations_tab.is_some(),
        }
    }

    /// Brings `panel` into view: in front of its group, its slot shown
    /// again, or its bottom tab in front and open; on the tab in front if
    /// it is there, or else on the first tab that has it.
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
            Home::Slot(slot) => {
                arrangement.hidden.set(slot, false);
                // Showing the Level panel or the folders switches Home's
                // left slot to it.
                let left = &mut self.open.library_state.left;
                match panel {
                    Panel::Level => *left = crate::library::LeftShows::Level,
                    Panel::Folders => *left = crate::library::LeftShows::Folders,
                    _ => {}
                }
                if panel == Panel::Placement {
                    arrangement.placing = true;
                }
            }
            Home::Bottom(bottom) => {
                arrangement.bottom_tab = bottom;
                arrangement.bottom_folded = false;
                if panel == Panel::Placement {
                    arrangement.placing = true;
                }
            }
        }
    }

    /// The panels in view now: the documents in front, the slots and the
    /// bottom tab in front, as the last frame drew them.
    pub fn panels_shown(&self) -> Vec<Panel> {
        self.shell.drawn.iter().map(|(panel, _)| *panel).collect()
    }

    /// Folds the bottom panel of the tab in front to its one line, or
    /// opens it again.
    pub fn fold_bottom(&mut self, folded: bool) {
        let tab = self.shell.tab;
        self.shell.arrangement_mut(tab).bottom_folded = folded;
    }

    /// Whether the bottom panel of the tab in front is folded.
    pub fn bottom_folded(&self) -> bool {
        self.shell.arrangement(self.shell.tab).bottom_folded
    }

    /// Whether the editor is placing: Home's Library has its Place switch
    /// on (the project lead's answer 20; decision 0118 moved it from a
    /// shelf tab of its own to the Library). The ribbon takes its build
    /// form and the tool bar goes.
    pub fn placing(&self) -> bool {
        self.shell.tab == RibbonTab::Home && self.shell.arrangement(RibbonTab::Home).placing
    }

    /// Starts placing (the Library's Place switch on, the Library in
    /// view), or stops.
    pub fn set_placing(&mut self, on: bool) {
        if on {
            self.show_tab(RibbonTab::Home);
            self.show_panel(Panel::Assets);
        }
        self.shell.arrangement_mut(RibbonTab::Home).placing = on;
    }

    /// Whether HOME TOOLS · Tile map is the tab in front: a tile map is
    /// selected and its Map tab is in front in the bottom panel (round
    /// 3). The ribbon is the map's tools then, as placing is the build
    /// form.
    pub(crate) fn map_tools_in_front(&self) -> bool {
        self.shell.tab == RibbonTab::Home && self.bottom_tab() == Some(workspaces::BottomTab::Map)
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

    /// The bottom tab in front on the tab in front, if it has a bottom
    /// panel.
    pub fn bottom_tab(&self) -> Option<workspaces::BottomTab> {
        let tabs = workspaces::built_bottom_tabs(self.shell.tab, self.open_now());
        self.shell.arrangement(self.shell.tab).bottom_tab_of(&tabs)
    }

    /// Ctrl + Space folds the bottom panel or opens it again (spec 157,
    /// once the shelf's). Works
    /// while typing too, as the Library's drawer did on the same key;
    /// not while the game has the keys, a question waits for its answer,
    /// or a key is being bound.
    pub(crate) fn bottom_keys(&mut self, ui: &egui::Ui) {
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
            self.fold_bottom(!self.bottom_folded());
        }
    }

    /// A tile map selected brings Home's Map bottom tab to the front, as
    /// the Map tab came to the front beside Properties before; with
    /// Preferences' "Shelf opens on" set to show it, something selected
    /// shows Properties again if it was hidden.
    pub(crate) fn follow_selection(&mut self) {
        let selected = self.open.document.selection.primary();
        let was = std::mem::replace(&mut self.shell.seen_selected, selected);
        if selected.is_some()
            && selected != was
            && self.interface_look().shelf_opens_on == crate::interface_look::ShelfOpensOn::Entity
            && let Some(home) =
                workspaces::home_of(RibbonTab::Home, self.open_now(), Panel::Properties)
        {
            self.show_in(RibbonTab::Home, home, Panel::Properties);
        }
        self.follow_map_selection();
        let has_map = self.open.map_edits.path.is_some();
        let had_map = std::mem::replace(&mut self.shell.seen_map, has_map);
        if has_map && !had_map {
            self.show_in(
                RibbonTab::Home,
                Home::Bottom(workspaces::BottomTab::Map),
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
        if asks.show_level_in_library {
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
        let of_kind = self.open.kind.shows(crate::kinds::tab(self.shell.tab));
        if !of_kind || (!self.shell.tab.is_built() && self.interface_look().hide_unfinished) {
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
            // Home again, from its tile map's tools: the Console in front
            // of the bottom panel.
            let console = workspaces::BottomTab::Console;
            self.show_in(tab, Home::Bottom(console), Panel::Console);
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
    /// whether a whole page was drawn in the middle: the frame isn't,
    /// then.
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
    /// its one page where the frame would be, and under it its bottom
    /// panel folded to its line, its tabs as unfinished parts.
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
        egui::Panel::bottom("unbuilt bottom panel")
            .frame(
                egui::Frame::new()
                    .fill(crate::tokens::shelf_bar())
                    .inner_margin(egui::Margin::symmetric(
                        crate::tokens::size::ROW_PAD as i8,
                        0,
                    )),
            )
            .exact_size(crate::tokens::size::BOTTOM_BAR_H)
            .show_separator_line(false)
            .show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    for entry in crate::unbuilt::filter(shelf) {
                        if let Entry::Unbuilt(part) = entry {
                            crate::unbuilt::bottom_tab(ui, part);
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

/// The frame of the tab in front, in what is left of `ui` under the
/// ribbon: its slots placed by the layout chosen, its centre and its
/// bottom panel. Notes where each panel was drawn.
pub(crate) fn middle(
    ui: &mut egui::Ui,
    shell: &mut Shell,
    open: OpenNow,
    viewer: &mut crate::panels::PanelViewer<'_>,
) -> groups::DocAsks {
    let rect = ui.available_rect_before_wrap();
    let (tab, layout) = (shell.tab, open.layout);
    shell.drawn.clear();
    ui.painter()
        .rect_filled(rect, 0.0, crate::tokens::viewport());
    let (parts, bottom_now) = frame_parts(shell, open);
    let frame = layouts::place(rect, &parts, |edge| shell.size(tab, layout, edge));
    let mut asks = groups::center(ui, frame.centre, shell, open, viewer);
    if let Some(at) = frame.bottom {
        bottom::bottom(ui, at, shell, &bottom_now, viewer);
    }
    let columns = columns::columns(ui, &frame.columns, shell, viewer);
    columns::drag_borders(ui, &frame.borders, shell, layout);
    let arrangement = shell.arrangement_mut(tab);
    if let Some(slot) = columns.hide {
        arrangement.hidden.set(slot, true);
        if slot == Slot::Library {
            arrangement.placing = false;
        }
    }
    if columns.place {
        arrangement.placing = !arrangement.placing;
    }
    asks.hide_unfinished |= columns.hide_unfinished;
    ui.advance_cursor_after_rect(rect);
    asks
}

/// What the tab in front puts where, in the layout `open` has: its side
/// columns' parts (hidden slots left out), and its bottom panel's tabs.
fn frame_parts(shell: &Shell, open: OpenNow) -> (layouts::Parts, bottom::BottomNow) {
    use workspaces::{BottomTab, LIBRARY, LIBRARY_PLACING};
    let (tab, layout) = (shell.tab, open.layout);
    let arrangement = shell.arrangement(tab);
    let mut tabs = crate::unbuilt::filter(workspaces::bottom_tabs(tab, open));
    let library = Entry::Built(BottomTab::Library);
    let library_beside = layout.library_beside() && tabs.contains(&library);
    if library_beside {
        tabs.retain(|entry| *entry != library);
    }
    let mut hidden = Vec::new();
    let mut take = |slot: Slot, column: Option<workspaces::Column>| {
        // An unfinished slot goes with the unfinished parts.
        let column = column.filter(|c| c.unbuilt.is_none() || !crate::unbuilt::hidden())?;
        if arrangement.hidden.has(slot) {
            hidden.push((slot, column));
            return None;
        }
        Some((slot, column))
    };
    let left = take(Slot::Left, workspaces::left_slot(tab, open));
    let right = take(Slot::Right, workspaces::right_slot(tab, open));
    let placing = arrangement.placing && tab == RibbonTab::Home;
    let shown_library = if placing { LIBRARY_PLACING } else { LIBRARY };
    let library = take(Slot::Library, library_beside.then_some(shown_library));
    let (mut on_left, mut on_right) = (Vec::new(), Vec::new());
    if layout.left_on_right() {
        on_right.extend(left);
    } else {
        on_left.extend(left);
    }
    on_right.extend(right);
    match layout.library_at() {
        layouts::LibraryAt::UnderLeft => on_left.extend(library),
        layouts::LibraryAt::UnderRight => on_right.extend(library),
        layouts::LibraryAt::Bottom => {}
    }
    let parts = layouts::Parts {
        left: on_left,
        right: on_right,
        bottom: (!tabs.is_empty()).then_some(arrangement.bottom_folded),
        bottom_under_left: layout.bottom_under_left(),
    };
    let now = bottom::BottomNow {
        tabs,
        hidden,
        layout,
    };
    (parts, now)
}

#[cfg(test)]
mod kind_tests;
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
