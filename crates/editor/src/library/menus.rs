//! The Library's right-click menus (board 04, decision 0118): on empty
//! space, what can be made in the folder shown, importing and showing the
//! folder; on a file or folder, opening, renaming, duplicating, deleting
//! and showing it, with what its kind can do; on several picked, what
//! applies to all of them.

use super::shown::folder_name;
use super::{Entry, Files, LibraryItem, LibraryKind, LibraryRequest, LibraryState, rename};
use crate::icons::Icon;
use crate::unbuilt::{self, Entry as Part, UnbuiltId};
use crate::widgets::menu;
use egui::Ui;

/// What an item's right-click menu offers to do with it besides the
/// rest ([`LibraryKind::menu_actions`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MenuAction {
    /// Open it: in its tab in the editor, or in the computer's own
    /// program for it.
    Open,
    /// Place it in the level, where the camera looks.
    Place,
}

impl MenuAction {
    /// Its name in the menu.
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Open => "Open",
            Self::Place => "Place",
        }
    }
}

/// Holds a menu to the width its longest of `labels` needs: its rows
/// fill what room they're given.
pub(crate) fn fit(ui: &mut Ui, labels: &[&str]) {
    let width = menu::width_for(ui, labels);
    ui.set_min_width(width);
    ui.set_max_width(width);
}

/// Why Place is greyed for a file with no ID.
const NO_ID_YET: &str =
    "This file has no ID yet, and one is needed to place it. Try again in a moment.";

/// A row of the menu that asks for `request` when clicked, and closes it.
fn ask(
    ui: &mut Ui,
    (icon, label): (Icon, &str),
    request: LibraryRequest,
    requests: &mut Vec<LibraryRequest>,
) {
    if menu::icon_item(ui, icon, label, "").clicked() {
        requests.push(request);
        ui.close();
    }
}

/// The menu on empty space in `folder` (board 04).
pub(crate) fn space_menu(
    ui: &mut Ui,
    folder: &str,
    project: &str,
    requests: &mut Vec<LibraryRequest>,
) {
    let header = format!("New in \u{201c}{}\u{201d}", folder_name(folder, project));
    fit(ui, &["New sound event…", "Show in Explorer", &header]);
    menu::caps(ui, &header);
    new_rows(ui, folder, requests);
    menu::line(ui);
    ask(
        ui,
        (Icon::Import, "Import…"),
        LibraryRequest::Import,
        requests,
    );
    for part in unbuilt::filter(vec![Part::<()>::Unbuilt(UnbuiltId::LibPaste)]) {
        if let Part::Unbuilt(id) = part {
            unbuilt::context_item(ui, id, Icon::Copy);
        }
    }
    menu::line(ui);
    let show = LibraryRequest::Show(folder.to_owned());
    ask(ui, (Icon::Reveal, "Show in Explorer"), show, requests);
}

/// A row of the New menu's More: its icon, words and request.
type MoreRow = (Icon, &'static str, fn(String) -> LibraryRequest);

/// What can be made in `folder`: a folder, a level, a prefab, a code file
/// and a sound event, as board 04 has them, then the 2D files under More.
fn new_rows(ui: &mut Ui, folder: &str, requests: &mut Vec<LibraryRequest>) {
    let at = folder.to_owned();
    ask(
        ui,
        (Icon::Library, "New folder"),
        LibraryRequest::NewFolder(at.clone()),
        requests,
    );
    ask(
        ui,
        (Icon::LevelFile, "New level"),
        LibraryRequest::NewLevel(at.clone()),
        requests,
    );
    ask(
        ui,
        (Icon::Prefab, "New prefab"),
        LibraryRequest::NewPrefab(at.clone()),
        requests,
    );
    ask(
        ui,
        (Icon::CodeFile, "New code file"),
        LibraryRequest::NewCodeFile(at.clone()),
        requests,
    );
    ask(
        ui,
        (Icon::SoundEvent, "New sound event…"),
        LibraryRequest::NewSoundEvent(at.clone()),
        requests,
    );
    ui.menu_button("More", |ui| {
        fit(ui, &["New tile map"]);
        use crate::shell::commands::NewFile;
        let more: [(MoreRow, NewFile); 4] = [
            (
                (Icon::TileMap, "New tile map", LibraryRequest::NewTileMap),
                NewFile::TileMap,
            ),
            (
                (Icon::Tileset, "New tileset", LibraryRequest::NewTileset),
                NewFile::Tileset,
            ),
            (
                (Icon::Animator, "New animator", LibraryRequest::NewAnimator),
                NewFile::Animator,
            ),
            (
                (
                    Icon::Ground,
                    "New ground\u{2026}",
                    LibraryRequest::NewGround,
                ),
                NewFile::Ground,
            ),
        ];
        // Only what the project's kind keeps (the kind table).
        for ((icon, label, request), new) in more {
            let command = crate::shell::ShellCommand::NewInLibrary(new);
            if crate::kinds::shown(crate::kinds::command(command)) {
                ask(ui, (icon, label), request(at.clone()), requests);
            }
        }
    });
}

/// The menu on a file or folder, or on several picked when it's one of
/// them.
pub(crate) fn entry_menu(
    ui: &mut Ui,
    entry: Entry<'_>,
    state: &mut LibraryState,
    files: Option<Files<'_>>,
    requests: &mut Vec<LibraryRequest>,
) {
    fit(ui, &["Open in the model viewer", "Show in Explorer"]);
    let picked = state.picked();
    if picked.len() > 1 && state.is_picked(entry.path()) {
        several_menu(ui, &picked, files, state, requests);
        return;
    }
    match entry {
        Entry::Folder(path) => {
            if menu::icon_item(ui, Icon::FolderOpen, "Open", "").clicked() {
                state.open_folder(path);
                ui.close();
            }
            if let Some(files) = files {
                let name = folder_name(path, &files.project.name).to_owned();
                ui.menu_button(format!("New in \u{201c}{name}\u{201d}"), |ui| {
                    fit(ui, &["New sound event…"]);
                    new_rows(ui, path, requests);
                });
            }
        }
        Entry::File(item) => kind_rows(ui, item, requests),
    }
    menu::line(ui);
    if menu::icon_item(ui, Icon::Pen, "Rename", "F2").clicked() {
        match files {
            Some(files) => rename::begin(state, files, entry.path()),
            None => state.start_renaming(entry.path(), entry.name()),
        }
        ui.close();
    }
    if let Entry::File(item) = entry {
        let copy = LibraryRequest::Duplicate(item.path.clone());
        ask(ui, (Icon::Copy, "Duplicate"), copy, requests);
    }
    delete_row(ui, &[entry.path().to_owned()], state, requests);
    menu::line(ui);
    let folder = match entry {
        Entry::Folder(path) => path,
        Entry::File(item) => item.folder(),
    };
    let show = LibraryRequest::Show(folder.to_owned());
    ask(ui, (Icon::Reveal, "Show in Explorer"), show, requests);
}

/// What a file's kind offers: Open and Place, cutting a picture's crops,
/// the model viewer, listening.
fn kind_rows(ui: &mut Ui, item: &LibraryItem, requests: &mut Vec<LibraryRequest>) {
    for action in item.kind.menu_actions() {
        let asked = match (action, item.id) {
            (MenuAction::Open, _) => Some(LibraryRequest::Open(item.kind, item.path.clone())),
            (MenuAction::Place, Some(id)) => Some(LibraryRequest::Place(id)),
            // Only a file with an ID can be placed: the row shows greyed,
            // and says why.
            (MenuAction::Place, None) => None,
        };
        let icon = match action {
            MenuAction::Open => Icon::External,
            MenuAction::Place => Icon::Move,
        };
        let row = ui
            .add_enabled_ui(asked.is_some(), |ui| {
                menu::icon_item(ui, icon, action.label(), "")
            })
            .inner
            .on_disabled_hover_text(NO_ID_YET);
        if let Some(asked) = asked.filter(|_| row.clicked()) {
            requests.push(asked);
            ui.close();
        }
    }
    if matches!(item.kind, LibraryKind::Texture | LibraryKind::SpriteSheet) {
        let crop = LibraryRequest::AtlasCrop(item.path.clone());
        ask(ui, (Icon::Crops, "Atlas Crop…"), crop, requests);
    }
    let viewer = crate::kinds::tab(crate::shell::RibbonTab::Model);
    if item.kind == LibraryKind::Model && !unbuilt::hidden() && crate::kinds::shown(viewer) {
        let view = LibraryRequest::ViewModel(item.path.clone());
        ask(
            ui,
            (Icon::Model, "Open in the model viewer"),
            view,
            requests,
        );
    }
    if item.kind == LibraryKind::Sound {
        let play = LibraryRequest::PlaySound(item.path.clone());
        ask(ui, (Icon::Play, "Play"), play, requests);
    }
}

/// Delete for `paths`: each to the Recycle Bin, with its ID file.
fn delete_row(
    ui: &mut Ui,
    paths: &[String],
    state: &mut LibraryState,
    requests: &mut Vec<LibraryRequest>,
) {
    let row = menu::icon_item(ui, Icon::Trash, "Delete", "Delete")
        .on_hover_text("Moves it to the Recycle Bin, with its ID file, so it can be brought back");
    if row.clicked() {
        requests.extend(paths.iter().cloned().map(LibraryRequest::Delete));
        state.selected = None;
        state.also.clear();
        ui.close();
    }
}

/// The menu on several picked: what applies to every one of them.
fn several_menu(
    ui: &mut Ui,
    picked: &[String],
    files: Option<Files<'_>>,
    state: &mut LibraryState,
    requests: &mut Vec<LibraryRequest>,
) {
    menu::caps(ui, &format!("{} picked", picked.len()));
    let all_files = files.is_some_and(|f| picked.iter().all(|p| f.items.at(p).is_some()));
    if all_files && menu::icon_item(ui, Icon::Copy, "Duplicate", "").clicked() {
        requests.extend(picked.iter().cloned().map(LibraryRequest::Duplicate));
        ui.close();
    }
    delete_row(ui, picked, state, requests);
    let folder = super::parent_of(&picked[0]);
    if picked.iter().all(|p| super::parent_of(p) == folder) {
        menu::line(ui);
        let show = LibraryRequest::Show(folder.to_owned());
        ask(ui, (Icon::Reveal, "Show in Explorer"), show, requests);
    }
}
