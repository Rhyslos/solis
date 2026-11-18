//! Home's left slot as the project's folders (decision 0118: the left
//! side switches between the Level panel and the folder tree, with a
//! small toggle in the slot's title). One tree or several side by side
//! ("Open another tree", from a tree's ⋯ menu), each showing its own
//! folder with the folders and files in it, so files are dragged from
//! one to another. Each can be closed but the last; the edges between
//! them drag. The trees remember their folders while the editor runs.

use super::shown::{folder_name, folders_in, is_engine};
use super::tree::{Row, draw_row};
use super::{
    Entry, Files, LibraryDrag, LibraryRequest, LibraryState, browser, menus, rename, tiles,
};
use crate::icons::Icon;
use crate::tokens::{self, size, text};
use egui::{Align, Id, Layout, Rect, Sense, Ui, UiBuilder, vec2};
use std::collections::BTreeSet;

/// What Home's left slot shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum LeftShows {
    /// The level's entities.
    #[default]
    Level,
    /// The project's folders, as trees.
    Folders,
}

/// The most trees side by side.
pub(crate) const MOST_TREES: usize = 4;

/// One tree: the folder it shows, and the folders in it unfolded.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct OneTree {
    /// Its folder, inside the project ("" for the project's own).
    pub(crate) top: String,
    /// The folders unfolded in it.
    pub(crate) open: BTreeSet<String>,
}

/// Home's folder trees, and the browser's own tree's folded folders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Trees {
    /// The trees side by side, left to right: always at least one.
    pub(crate) list: Vec<OneTree>,
    /// Where each edge between them is, as thousandths of the slot's
    /// width from its left (one fewer than the trees).
    pub(crate) edges: Vec<u16>,
    /// The browser's own tree's folded folders.
    pub(crate) folded: BTreeSet<String>,
}

impl Default for Trees {
    fn default() -> Self {
        Self {
            list: vec![OneTree::default()],
            edges: Vec::new(),
            folded: BTreeSet::new(),
        }
    }
}

impl Trees {
    /// Opens another tree beside the one at `after`, showing the same
    /// folder, and shares the room out evenly. None past
    /// [`MOST_TREES`].
    pub(crate) fn open_another(&mut self, after: usize) {
        if self.list.len() >= MOST_TREES {
            return;
        }
        let at = after.min(self.list.len() - 1);
        let tree = OneTree {
            top: self.list[at].top.clone(),
            open: BTreeSet::new(),
        };
        self.list.insert(at + 1, tree);
        self.even_out();
    }

    /// Closes the tree at `at`, unless it's the last.
    pub(crate) fn close(&mut self, at: usize) {
        if self.list.len() > 1 && at < self.list.len() {
            self.list.remove(at);
            self.even_out();
        }
    }

    /// Every tree the same width.
    fn even_out(&mut self) {
        let n = self.list.len();
        self.edges = (1..n).map(|i| (i * 1000 / n) as u16).collect();
    }

    /// The trees' rooms side by side in `rect`.
    fn rooms(&self, rect: Rect) -> Vec<Rect> {
        let mut lefts = vec![rect.left()];
        lefts.extend(
            self.edges
                .iter()
                .map(|e| (rect.left() + rect.width() * f32::from(*e) / 1000.0).round()),
        );
        lefts.push(rect.right());
        lefts
            .windows(2)
            .map(|w| Rect::from_x_y_ranges(w[0]..=w[1] - size::LINE, rect.y_range()))
            .collect()
    }

    /// Moves edge `edge` to `share` of the width, keeping every tree at
    /// least [`size::TREE_SHARE_LIMIT`] of it.
    fn drag_edge(&mut self, edge: usize, share: f32) {
        let limit = (size::TREE_SHARE_LIMIT * 1000.0) as u16;
        let low = if edge == 0 { 0 } else { self.edges[edge - 1] } + limit;
        let high = self.edges.get(edge + 1).copied().unwrap_or(1000) - limit;
        if low <= high {
            self.edges[edge] = ((share * 1000.0) as u16).clamp(low, high);
        }
    }
}

/// Home's folder trees filling `ui`, side by side.
pub(crate) fn folder_trees(
    ui: &mut Ui,
    files: Files<'_>,
    state: &mut LibraryState,
    requests: &mut Vec<LibraryRequest>,
) {
    let rect = ui.available_rect_before_wrap();
    if state.trees.edges.len() + 1 != state.trees.list.len() {
        state.trees.even_out();
    }
    let rooms = state.trees.rooms(rect);
    for (at, room) in rooms.iter().enumerate() {
        let mut tree_ui = ui.new_child(UiBuilder::new().max_rect(*room).id_salt(("tree", at)));
        tree_ui.set_clip_rect(room.intersect(ui.clip_rect()));
        one_tree(&mut tree_ui, at, files, state, requests);
    }
    for edge in 0..state.trees.edges.len() {
        let x = rooms[edge].right();
        let line = Rect::from_x_y_ranges(x..=x + size::LINE, rect.y_range());
        let reach = line.expand2(vec2(size::DIVIDER_REACH, 0.0));
        let response = ui.interact(reach, Id::new(("tree edge", edge)), Sense::drag());
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Other, true, "Edge between trees")
        });
        let held = response.hovered() || response.dragged();
        if held {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }
        let ink = if held {
            crate::interface_look::accent_fill()
        } else {
            tokens::line()
        };
        ui.painter().rect_filled(line, 0.0, ink);
        if let Some(at) = response
            .interact_pointer_pos()
            .filter(|_| response.dragged())
        {
            state
                .trees
                .drag_edge(edge, (at.x - rect.left()) / rect.width().max(1.0));
        }
    }
    ui.advance_cursor_after_rect(rect);
}

/// One tree: its header (up, its folder's name, ⋯), then its rows.
fn one_tree(
    ui: &mut Ui,
    at: usize,
    files: Files<'_>,
    state: &mut LibraryState,
    requests: &mut Vec<LibraryRequest>,
) {
    let rect = ui.available_rect_before_wrap();
    let head = rect.with_max_y(rect.top() + size::LIBRARY_ROW_H);
    tree_header(ui, head, at, files, state, requests);
    let body = rect.with_min_y(head.bottom());
    let mut rows_ui = ui.new_child(UiBuilder::new().max_rect(body).id_salt("rows"));
    // Dropped on the tree's empty space: into its folder.
    let top = state.trees.list[at].top.clone();
    let space = rows_ui.interact(body, rows_ui.id().with("space"), Sense::click());
    browser::take_moves(&space, &top, requests);
    space.context_menu(|ui| menus::space_menu(ui, &top, &files.project.name, requests));
    let rows = tree_rows(files, &state.trees.list[at]);
    let order: Vec<&str> = rows.iter().map(|(e, _)| e.path()).collect();
    rows_ui.spacing_mut().item_spacing.y = 0.0;
    egui::ScrollArea::vertical()
        .id_salt("tree rows")
        .auto_shrink(false)
        .show_rows(
            &mut rows_ui,
            size::LIBRARY_ROW_H,
            rows.len(),
            |ui, range| {
                for (entry, depth) in &rows[range] {
                    tree_row(ui, (at, *entry, *depth), files, state, &order, requests);
                }
            },
        );
}

/// A tree's header: up one folder, the folder's name, and the ⋯ menu
/// (Open another tree, the project's folder, Close this tree).
fn tree_header(
    ui: &mut Ui,
    head: Rect,
    at: usize,
    files: Files<'_>,
    state: &mut LibraryState,
    requests: &mut Vec<LibraryRequest>,
) {
    ui.painter().rect_filled(head, 0.0, tokens::code());
    let room = head.shrink2(vec2(size::ROW_GAP, 0.0));
    // The ⋯ first, at the right, so a long name never pushes it out.
    let more_rect = room.with_min_x(room.right() - size::ICON_BUTTON);
    let mut more_ui = ui.new_child(
        UiBuilder::new()
            .max_rect(more_rect)
            .layout(Layout::right_to_left(Align::Center))
            .id_salt("tree more"),
    );
    let more = crate::widgets::button::icon(&mut more_ui, Icon::More, "More of this tree");
    egui::Popup::menu(&more).show(|ui| tree_menu(ui, at, state));
    let mut row = ui.new_child(
        UiBuilder::new()
            .max_rect(room.with_max_x(more_rect.left()))
            .layout(Layout::left_to_right(Align::Center))
            .id_salt("tree header"),
    );
    row.spacing_mut().item_spacing.x = 0.0;
    let top = state.trees.list[at].top.clone();
    if !top.is_empty()
        && crate::widgets::button::icon(&mut row, Icon::ChevronUp, "Up one folder").clicked()
    {
        state.trees.list[at].top = super::parent_of(&top).to_owned();
    }
    let name = row.add(
        egui::Label::new(
            egui::RichText::new(folder_name(&top, &files.project.name))
                .font(text::row())
                .color(tokens::text_emphasis()),
        )
        .truncate()
        .selectable(false)
        .sense(Sense::click()),
    );
    browser::take_moves(&name, &top, requests);
}

/// A tree's ⋯ menu.
fn tree_menu(ui: &mut Ui, at: usize, state: &mut LibraryState) {
    use crate::widgets::menu;
    super::menus::fit(ui, &["Show the project's folder"]);
    let room = state.trees.list.len() < MOST_TREES;
    let another = ui
        .add_enabled_ui(room, |ui| {
            menu::icon_item(ui, Icon::Library, "Open another tree", "")
        })
        .inner
        .on_disabled_hover_text(format!("{MOST_TREES} trees side by side is the most"));
    if another.clicked() {
        state.trees.open_another(at);
        ui.close();
    }
    if menu::icon_item(ui, Icon::FolderOpen, "Show the project's folder", "").clicked() {
        state.trees.list[at].top.clear();
        ui.close();
    }
    if state.trees.list.len() > 1
        && menu::icon_item(ui, Icon::Close, "Close this tree", "").clicked()
    {
        state.trees.close(at);
        ui.close();
    }
}

/// The rows of `tree`: what's in its folder, folders first, each
/// unfolded folder followed by what's in it, with how deep each is.
fn tree_rows<'a>(files: Files<'a>, tree: &OneTree) -> Vec<(Entry<'a>, usize)> {
    let mut rows = Vec::new();
    add_rows(files, tree, &tree.top, 0, &mut rows);
    rows
}

fn add_rows<'a>(
    files: Files<'a>,
    tree: &OneTree,
    folder: &str,
    depth: usize,
    rows: &mut Vec<(Entry<'a>, usize)>,
) {
    for inside in folders_in(files.folders, folder) {
        rows.push((Entry::Folder(inside), depth));
        if tree.open.contains(inside) {
            add_rows(files, tree, inside, depth + 1, rows);
        }
    }
    rows.extend(
        files
            .items
            .iter()
            .filter(|i| i.folder() == folder && !is_engine(&i.path))
            .map(|i| (Entry::File(i), depth)),
    );
}

/// One row of a tree. A folder's click folds or unfolds it and a
/// double-click makes it the tree's folder; a file's click picks it and a
/// double-click opens it; both drag, and folders take what's dropped.
fn tree_row(
    ui: &mut Ui,
    (at, entry, depth): (usize, Entry<'_>, usize),
    files: Files<'_>,
    state: &mut LibraryState,
    order: &[&str],
    requests: &mut Vec<LibraryRequest>,
) {
    let path = entry.path();
    let (icon, colour, fold) = match entry {
        Entry::Folder(folder) => {
            let open = state.trees.list[at].open.contains(folder);
            let icon = if open {
                Icon::FolderOpen
            } else {
                Icon::Library
            };
            (icon, crate::theme::FOLDER, Some(open))
        }
        Entry::File(item) => (item.kind.icon(), item.colour(), None),
    };
    let row = Row {
        depth,
        icon,
        colour,
        name: entry.name(),
        fold,
        lit: state.is_picked(path),
        folder: matches!(entry, Entry::Folder(_)),
    };
    let (response, _) = draw_row(ui, &row, files.accent, |ui, rect| {
        rename::name_box(ui, rect, path, (files, &mut *state), requests)
    });
    match entry {
        Entry::Folder(folder) => {
            if response.double_clicked() {
                state.trees.list[at].top = folder.to_owned();
            } else if response.clicked() {
                let open = &mut state.trees.list[at].open;
                if !open.remove(folder) {
                    open.insert(folder.to_owned());
                }
            }
            browser::take_moves(&response, folder, requests);
            response.dnd_set_drag_payload(LibraryDrag::folder(folder));
            response.context_menu(|ui| menus::entry_menu(ui, entry, state, Some(files), requests));
        }
        Entry::File(_) => {
            let picked = tiles::Picked {
                entry,
                response: response.on_hover_text(path),
                name: None,
                place: false,
                listen: None,
                placing: false,
            };
            tiles::item_actions(picked, state, Some(files), order, requests);
        }
    }
}
