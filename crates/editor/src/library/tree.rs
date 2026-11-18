//! The browser's folder tree (board 04): the project's own folder, then
//! every folder in it, each folded or not; the open folder lit. A click
//! opens a folder; files dropped on one move into it; right-click has the
//! folder's menu. Only the rows in view are drawn.

use super::shown::{folder_name, is_engine};
use super::{Entry, Files, LibraryRequest, LibraryState, browser, menus, rename};
use crate::icons::{self, Icon};
use crate::tokens::{self, size, text};
use egui::{Rect, Response, Sense, Ui, pos2, vec2};
use std::collections::BTreeSet;

/// The browser's folder tree, filling `ui`.
pub(crate) fn folder_tree(
    ui: &mut Ui,
    files: Files<'_>,
    state: &mut LibraryState,
    requests: &mut Vec<LibraryRequest>,
) {
    let rows = visible_folders(files.folders, &state.trees.folded, "");
    ui.spacing_mut().item_spacing.y = 0.0;
    egui::ScrollArea::vertical()
        .id_salt("library folder tree")
        .auto_shrink(false)
        .show_rows(ui, size::LIBRARY_ROW_H, rows.len() + 1, |ui, range| {
            for at in range {
                let folder = at.checked_sub(1).map_or("", |i| rows[i].0);
                let depth = at.checked_sub(1).map_or(0, |i| rows[i].1 + 1);
                let has_inside = folder.is_empty()
                    || files
                        .folders
                        .iter()
                        .any(|f| super::parent_of(f) == folder && !is_engine(f));
                let open = state.folder == folder && state.search.trim().is_empty();
                let folded = state.trees.folded.contains(folder);
                let row = Row {
                    depth,
                    icon: if open {
                        Icon::FolderOpen
                    } else {
                        Icon::Library
                    },
                    colour: crate::theme::FOLDER,
                    name: folder_name(folder, &files.project.name),
                    fold: has_inside.then_some(!folded),
                    lit: open,
                    folder: true,
                };
                let (response, chevron) = draw_row(ui, &row, files.accent, |ui, rect| {
                    !folder.is_empty()
                        && rename::name_box(ui, rect, folder, (files, &mut *state), requests)
                });
                if chevron && !folder.is_empty() {
                    if !state.trees.folded.remove(folder) {
                        state.trees.folded.insert(folder.to_owned());
                    }
                } else if response.clicked() {
                    state.open_folder(folder);
                }
                browser::take_moves(&response, folder, requests);
                if !folder.is_empty() {
                    response.dnd_set_drag_payload(super::LibraryDrag::folder(folder));
                }
                response.context_menu(|ui| {
                    if folder.is_empty() {
                        menus::space_menu(ui, "", &files.project.name, requests);
                    } else {
                        menus::entry_menu(ui, Entry::Folder(folder), state, Some(files), requests);
                    }
                });
            }
        });
}

/// The folders inside `top` in the tree's order (each followed by those
/// inside it, by name), with how deep each is below `top`, leaving out
/// those inside a folded one and the engine's own.
pub(crate) fn visible_folders<'a>(
    folders: &'a [String],
    folded: &BTreeSet<String>,
    top: &str,
) -> Vec<(&'a str, usize)> {
    let below = |f: &'a str| -> Option<&'a str> {
        if top.is_empty() {
            Some(f)
        } else {
            f.strip_prefix(top).and_then(|r| r.strip_prefix('/'))
        }
    };
    let mut inside: Vec<(&str, Vec<String>)> = folders
        .iter()
        .map(String::as_str)
        .filter(|f| !is_engine(f))
        .filter_map(|f| {
            let rest = below(f)?;
            Some((f, rest.split('/').map(str::to_lowercase).collect()))
        })
        .collect();
    inside.sort_by(|a, b| a.1.cmp(&b.1));
    inside
        .into_iter()
        .filter(|(f, _)| {
            // Hidden when any folder holding it (below the top) is folded.
            let mut at = super::parent_of(f);
            while at.len() > top.len() {
                if folded.contains(at) {
                    return false;
                }
                at = super::parent_of(at);
            }
            true
        })
        .map(|(f, parts)| (f, parts.len() - 1))
        .collect()
}

/// One row of a tree.
pub(crate) struct Row<'a> {
    /// How many folders deep it is.
    pub(crate) depth: usize,
    pub(crate) icon: Icon,
    pub(crate) colour: egui::Color32,
    pub(crate) name: &'a str,
    /// Whether it's unfolded, if it can be folded.
    pub(crate) fold: Option<bool>,
    /// Whether it's the one open, or picked.
    pub(crate) lit: bool,
    /// Whether it's a folder (a screen reader says so).
    pub(crate) folder: bool,
}

/// Draws a tree's row: its fold arrow, icon and name (or what `name_box`
/// draws in the name's place, when it draws). Returns the row's response,
/// and whether the fold arrow was clicked.
pub(crate) fn draw_row(
    ui: &mut Ui,
    row: &Row<'_>,
    accent: crate::theme::Accent,
    name_box: impl FnOnce(&mut Ui, Rect) -> bool,
) -> (Response, bool) {
    let (rect, response) = ui.allocate_exact_size(
        vec2(ui.available_width(), size::LIBRARY_ROW_H),
        Sense::click_and_drag(),
    );
    if row.lit {
        ui.painter()
            .rect_filled(rect, 0.0, tokens::select_row(accent));
    } else if response.hovered() {
        ui.painter().rect_filled(rect, 0.0, tokens::hover());
    }
    let x = rect.left() + size::ROW_GAP + row.depth as f32 * size::TREE_LEVEL;
    let fold = Rect::from_min_size(pos2(x, rect.top()), vec2(size::TREE_FOLD_W, rect.height()));
    let mut chevron = false;
    if let Some(open) = row.fold {
        let arrow = if open {
            Icon::ChevronDown
        } else {
            Icon::ChevronRight
        };
        let spot = Rect::from_center_size(fold.center(), vec2(size::FOLD_ICON, size::FOLD_ICON));
        icons::icon(ui.painter(), spot, arrow, tokens::text_meta());
        chevron = response.clicked()
            && response
                .interact_pointer_pos()
                .is_some_and(|at| fold.contains(at));
    }
    let icon = Rect::from_min_size(
        pos2(
            fold.right() + size::ROW_GAP,
            rect.center().y - size::ROW_ICON / 2.0,
        ),
        vec2(size::ROW_ICON, size::ROW_ICON),
    );
    icons::icon(ui.painter(), icon, row.icon, row.colour);
    let name = Rect::from_min_max(
        pos2(icon.right() + size::GAP, rect.top()),
        pos2(rect.right() - size::ROW_GAP, rect.bottom()),
    );
    let boxed = Rect::from_center_size(name.center(), vec2(name.width(), size::NAME_BOX_H));
    if !name_box(ui, boxed) {
        let ink = if row.lit {
            tokens::text_emphasis()
        } else {
            tokens::text_primary()
        };
        crate::widgets::cut_off(ui, name, row.name, text::row(), ink);
    }
    response.widget_info(|| {
        let label = if row.folder {
            format!("Folder {}", row.name)
        } else {
            row.name.to_owned()
        };
        egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, row.lit, label)
    });
    (response, chevron)
}
