//! The Library panel: the project's files
//! ([decision 0021](../../../../docs/decisions/0021-asset-database-and-models.md)),
//! with a folder tree that can be put away, the files of one folder as
//! thumbnails or a list, and a search box. Drag a model or assembly onto
//! the viewport or the Area panel to place it, or onto a folder to move
//! it; double-click to open an area or a file in its own program (a file
//! that would be placed is only picked: the ribbon's **Place** places it,
//! the project lead's answer 25); right-click for the rest. The Code tab
//! shows the code files the same way. The ribbon has making files, the
//! view and the kinds' filter (milestone M7).
//!
//! The app keeps the list up to date from the asset database and makes
//! the thumbnails; the panel only shows them and returns
//! `LibraryRequest`s.

use crate::icons::{self, Icon};
use crate::sound_view::{self, EventState, Preview, SOUND_COLOR, SoundState};
use crate::theme::{self, Accent};
use crate::tokens::{self, size, text};
use egui::{Align, Color32, Layout, RichText, Ui};
use reflect::AssetId;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

mod kinds;
pub(crate) use kinds::Filter;
pub use kinds::LibraryKind;

/// The open project, as the editor shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectInfo {
    /// Its name.
    pub name: String,
    /// Its folder.
    pub root: PathBuf,
    /// Its assets folder, which the Library shows.
    pub assets: PathBuf,
}

/// One file in the Library.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LibraryItem {
    /// Its asset ID (models and assemblies).
    pub id: Option<AssetId>,
    /// What kind of file.
    pub kind: LibraryKind,
    /// Its name (the file's, without the extension).
    pub name: String,
    /// Where it is inside the project's assets, with `/` between folders.
    pub path: String,
    /// Its picture, once the app has made one.
    pub thumbnail: Option<egui::TextureId>,
}

impl LibraryItem {
    /// Its colour: its kind's (the project lead's answer 21).
    pub(crate) fn colour(&self) -> Color32 {
        self.kind.color()
    }

    /// A text file's extension, lower case, which its icon shows ("md").
    pub(crate) fn text_extension(&self) -> Option<String> {
        (self.kind == LibraryKind::Text)
            .then(|| self.path.rsplit_once('.').map(|(_, e)| e.to_lowercase()))
            .flatten()
    }

    /// The folder it's in, inside the assets ("" for the assets folder
    /// itself).
    pub fn folder(&self) -> &str {
        self.path.rsplit_once('/').map_or("", |(folder, _)| folder)
    }

    /// Its file's name, with the extension.
    fn file_name(&self) -> &str {
        self.path
            .rsplit_once('/')
            .map_or(&self.path, |(_, name)| name)
    }
}

/// What a Library item being dragged carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LibraryDrag(pub(crate) LibraryItem);

/// How the Library is shown: the folder open, whether the folder tree is
/// out, thumbnails or a list, the search, and what's selected. The
/// editor's own, never saved with the project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LibraryState {
    /// The folder shown, inside the assets ("" for the assets folder).
    pub(crate) folder: String,
    /// Whether the folder tree is out.
    pub(crate) folders_shown: bool,
    /// A list rather than thumbnails.
    pub(crate) list: bool,
    /// Only names containing this are shown, from every folder.
    pub(crate) search: String,
    /// The item or folder picked, by path.
    pub(crate) selected: Option<String>,
    /// The file or folder being renamed, and the name typed so far.
    pub(crate) renaming: Option<(String, String)>,
    /// The chip picked: every file, or one group of kinds.
    pub(crate) filter: Filter,
}

impl Default for LibraryState {
    fn default() -> Self {
        Self {
            folder: String::new(),
            folders_shown: true,
            list: false,
            search: String::new(),
            selected: None,
            renaming: None,
            filter: Filter::All,
        }
    }
}

/// Something asked for in the Library. Paths are inside the assets, with
/// `/` between folders.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum LibraryRequest {
    /// Place a model or assembly where the camera looks.
    Place(AssetId),
    /// Open an area, or a file in its own program.
    Open(LibraryKind, String),
    /// Show a folder in the system's file browser.
    Show(String),
    /// Rename or move a file or folder: from, to.
    Move(String, String),
    /// Make a new folder inside this one.
    NewFolder(String),
    /// Bring the Library tab to the front (the Place shelf tab's **Open
    /// Library**).
    ShowLibraryTab,
    /// Make a new Lust code file in this folder, and open it in the
    /// computer's editor for Lust (E1).
    NewCodeFile(String),
    /// Make a new, empty area in this folder, saved, and open it (E1).
    NewArea(String),
    /// Move a file or folder to the Recycle Bin.
    Delete(String),
    /// Import a model from outside the project.
    Import,
    /// Listen to a sound file or sound event.
    PlaySound(String),
    /// Stop listening.
    StopSound,
    /// Make a new sound event in this folder.
    NewSoundEvent(String),
    /// Save a sound event's settings to its file.
    SaveSoundEvent(String, sound::SoundEvent),
    /// Make a new animator in this folder (Phase 9, D3b).
    NewAnimator(String),
    /// Make a new tileset in this folder (Phase 9, D4).
    NewTileset(String),
    /// Make a new tile map in this folder (Phase 9, D4).
    NewTileMap(String),
    /// Change a tileset's terrains, asked for in its Properties (path, the
    /// change).
    EditTileset(String, crate::tile_properties::TerrainEdit),
    /// Change a tile map file's tileset or size in Properties (path,
    /// tileset, size).
    ShapeTileMap(String, Option<AssetId>, [u32; 2]),
    /// Make a clips file beside the Aseprite file at this path (or open
    /// the one there), from its Properties (Phase 9, D3).
    MakeClipsFile(String),
    /// Open the crops of the picture at this path in the Atlas Crop tab,
    /// making its atlas file if it has none (decision 0089).
    AtlasCrop(String),
    /// Add a sound file (by its asset ID) to a sound event's takes.
    AddTake(String, AssetId),
    /// Look at the model at this path in the model viewer (not built
    /// yet: its tab, ribbon and page; round 3, `11-unbuilt-model-viewer`).
    ViewModel(String),
}

/// The sound files the app has measured, by path inside the assets, and
/// the one previewing.
#[derive(Clone, Copy)]
pub(crate) struct Sounds<'a> {
    pub(crate) states: &'a HashMap<String, SoundState>,
    pub(crate) events: &'a HashMap<String, EventState>,
    pub(crate) preview: Option<&'a Preview>,
    /// The project's buses, which sound events play through.
    pub(crate) mixer: &'a sound::Mixer,
    /// The project's sprite sheets (Phase 9, D2), and each one's atlas as
    /// a picture, by path.
    pub(crate) sheets: &'a crate::sprite_sheets::SpriteSheets,
    pub(crate) sheet_pictures: &'a HashMap<String, egui::TextureHandle>,
    /// For each code file (by path), the components written in it and how
    /// many entities in the area use each (`Player · 1`), for the Code
    /// panel (decision 0085).
    pub(crate) code_uses: &'a BTreeMap<String, String>,
}

impl Sounds<'_> {
    /// How far through `item` the preview is, if it's playing.
    fn played(&self, item: &LibraryItem) -> Option<f32> {
        sound_view::played(&item.path, self.preview)
    }
}

/// A list row's height.
const ROW: f32 = 21.0;
const TREE_WIDTH: f32 = 150.0;

/// The Library panel (or, with `code`, the Code panel: code files only,
/// as a list). Returns what was asked for.
#[allow(clippy::too_many_arguments)]
pub(crate) fn library(
    ui: &mut Ui,
    project: Option<&ProjectInfo>,
    (items, folders): (&[LibraryItem], &[String]),
    state: &mut LibraryState,
    accent: Accent,
    code: bool,
    sounds: Sounds<'_>,
) -> Vec<LibraryRequest> {
    let mut requests = Vec::new();
    menu_row(ui, state, code, project.is_some(), accent, &mut requests);
    let Some(project) = project else {
        note(
            ui,
            "No project is open. File > New project makes one: a folder whose models are kept \
             here, with IDs that survive renaming.",
        );
        return requests;
    };
    let shown: Vec<&LibraryItem> = shown_items(items, state, code);
    let body = ui.available_rect_before_wrap();
    let tree_out = state.folders_shown && !code;
    let (tree_rect, files_rect) = if tree_out {
        let split = (body.left() + TREE_WIDTH).min(body.center().x);
        (
            Some(egui::Rect::from_min_max(
                body.min,
                egui::pos2(split, body.max.y),
            )),
            egui::Rect::from_min_max(egui::pos2(split + 1.0, body.min.y), body.max),
        )
    } else {
        (None, body)
    };
    if let Some(rect) = tree_rect {
        let mut tree = ui.new_child(egui::UiBuilder::new().max_rect(rect).id_salt("folders"));
        folder_tree(
            &mut tree,
            &project.name,
            folders,
            state,
            accent,
            &mut requests,
        );
        ui.painter().vline(
            rect.right() + 0.5,
            rect.y_range(),
            egui::Stroke::new(1.0, theme::line()),
        );
    }
    if !code {
        // Right-clicking where there's no file: what can be made here.
        // Files and folders drawn over it have their own menus.
        let space = ui.interact(
            files_rect,
            ui.id().with("empty space"),
            egui::Sense::click(),
        );
        space.context_menu(|ui| {
            ui.menu_button("New", |ui| new_items(ui, &state.folder, &mut requests));
            ui.separator();
            if ui.button("Import model…").clicked() {
                requests.push(LibraryRequest::Import);
                ui.close();
            }
            if ui.button("Show in Explorer").clicked() {
                requests.push(LibraryRequest::Show(state.folder.clone()));
                ui.close();
            }
        });
    }
    let mut files = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(files_rect.shrink2(egui::vec2(6.0, 2.0)))
            .id_salt("files"),
    );
    if !code && state.search.is_empty() {
        breadcrumb(&mut files, &project.name, state);
    }
    if shown.is_empty() {
        let text = if !state.search.is_empty() {
            "Nothing has that name.".to_string()
        } else if code {
            format!(
                "{} has no code yet. Lust, Lua and Rust files in its assets folder show here: \
                 right-click in the Library and choose New > Lust code file to start one.",
                project.name
            )
        } else if items.is_empty() {
            format!(
                "{} has no files yet. Right-click here to make a map, a tileset or a code file, \
                 drop files onto the window, or put them in its assets folder.",
                project.name
            )
        } else {
            "This folder is empty.".to_string()
        };
        files.label(RichText::new(text).color(theme::text_weak()));
    } else if state.list || code {
        list(&mut files, &shown, state, accent, sounds, &mut requests);
    } else {
        grid(&mut files, &shown, state, accent, sounds, &mut requests);
    }
    ui.advance_cursor_after_rect(body);
    rename_window(ui, state, &mut requests);
    requests
}

/// One thing the New menu makes: its name, what it is, and the request.
type NewItem = (&'static str, &'static str, fn(String) -> LibraryRequest);

/// What can be made in `folder` (E1): the "+ New" menu, and New in every
/// right-click menu of the Library.
fn new_items(ui: &mut Ui, folder: &str, requests: &mut Vec<LibraryRequest>) {
    let folder = folder.to_string();
    let items: [NewItem; 7] = [
        (
            "Lust code file",
            "A Lust file with a component to start from, opened in your editor for Lust",
            LibraryRequest::NewCodeFile,
        ),
        (
            "Folder",
            "A new folder inside this one",
            LibraryRequest::NewFolder,
        ),
        (
            "Area",
            "An empty area (a level), saved here and opened",
            LibraryRequest::NewArea,
        ),
        (
            "Tile map",
            "A grid of terrains, filled with its tileset's first; drag it into the viewport",
            LibraryRequest::NewTileMap,
        ),
        (
            "Tileset",
            "The terrains a tile map's cells can be, and their art",
            LibraryRequest::NewTileset,
        ),
        (
            "Animator",
            "States that choose which clip a sprite plays",
            LibraryRequest::NewAnimator,
        ),
        (
            "Sound event",
            "Takes of a sound, one picked each time it plays",
            LibraryRequest::NewSoundEvent,
        ),
    ];
    for (i, (name, hint, request)) in items.into_iter().enumerate() {
        if i == 2 {
            ui.separator();
        }
        if ui.button(name).on_hover_text(hint).clicked() {
            requests.push(request(folder.clone()));
            ui.close();
        }
    }
}

/// A quiet note in the panel.
fn note(ui: &mut Ui, text: &str) {
    egui::Frame::new()
        .inner_margin(theme::panel_margin())
        .show(ui, |ui| {
            ui.label(RichText::new(text).color(theme::text_weak()));
        });
}

/// The items to show: the open folder's (or every folder's matches while
/// searching) that pass the chip picked; code files only in the Code
/// panel.
fn shown_items<'a>(
    items: &'a [LibraryItem],
    state: &LibraryState,
    code: bool,
) -> Vec<&'a LibraryItem> {
    let search = state.search.trim().to_lowercase();
    items
        .iter()
        .filter(|i| {
            if code {
                i.kind == LibraryKind::Code
            } else {
                state.filter.passes(i.kind)
            }
        })
        .filter(|i| {
            if search.is_empty() {
                code || i.folder() == state.folder
            } else {
                i.name.to_lowercase().contains(&search)
            }
        })
        .collect()
}

/// The Library's tool row (spec 305): the kinds' chips, each with its
/// colour's dot, and the search at the right. Making files, importing,
/// the view and the folder tree are on the ribbon (Add, View, Filter).
fn menu_row(
    ui: &mut Ui,
    state: &mut LibraryState,
    code: bool,
    _open: bool,
    _accent: Accent,
    _requests: &mut Vec<LibraryRequest>,
) {
    use crate::widgets::chip::{ChipKind, chip};
    theme::menu_row_frame().show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            if !code {
                for filter in Filter::ALL {
                    let on = state.filter == filter;
                    let name = format!("{} files", filter.name());
                    let response = chip(ui, ChipKind::Filter, filter.name(), on, filter.dot());
                    response.widget_info(|| {
                        egui::WidgetInfo::selected(egui::WidgetType::Button, true, on, &name)
                    });
                    let hover = match filter {
                        Filter::All => "Every file".to_string(),
                        other => format!("Only {}", other.name().to_lowercase()),
                    };
                    if response.on_hover_text(hover).clicked() {
                        state.filter = filter;
                    }
                }
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let search = ui.add(
                    egui::TextEdit::singleline(&mut state.search)
                        .hint_text("Search the project")
                        .desired_width(ui.available_width().clamp(40.0, 260.0)),
                );
                let label = if code {
                    "Search code"
                } else {
                    "Search the Library"
                };
                search.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, label)
                });
            });
        });
    });
}

/// The folder tree: the assets folder and every folder in it.
fn folder_tree(
    ui: &mut Ui,
    project: &str,
    folders: &[String],
    state: &mut LibraryState,
    accent: Accent,
    requests: &mut Vec<LibraryRequest>,
) {
    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 1.0;
            let all = std::iter::once(String::new()).chain(folders.iter().cloned());
            for folder in all {
                let depth = if folder.is_empty() {
                    0
                } else {
                    folder.matches('/').count() + 1
                };
                let name = folder
                    .rsplit_once('/')
                    .map_or(folder.as_str(), |(_, n)| n)
                    .to_string();
                let name = if folder.is_empty() {
                    project.to_string()
                } else {
                    name
                };
                let (rect, response) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), ROW),
                    egui::Sense::click(),
                );
                let open = state.folder == folder && state.search.is_empty();
                if open {
                    ui.painter()
                        .rect_filled(rect, 3.0, crate::tokens::select_row(accent));
                } else if response.hovered() {
                    ui.painter().rect_filled(rect, 3.0, theme::hover());
                }
                let x = rect.left() + 6.0 + depth as f32 * 12.0;
                icons::icon(
                    ui.painter(),
                    egui::Rect::from_min_size(
                        egui::pos2(x, rect.center().y - 6.0),
                        egui::vec2(12.0, 12.0),
                    ),
                    Icon::Library,
                    theme::FOLDER,
                );
                ui.painter().text(
                    egui::pos2(x + 17.0, rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    &name,
                    egui::TextStyle::Body.resolve(ui.style()),
                    ui.visuals().text_color(),
                );
                response.widget_info(|| {
                    egui::WidgetInfo::selected(
                        egui::WidgetType::SelectableLabel,
                        true,
                        open,
                        format!("Folder {name}"),
                    )
                });
                if response.clicked() {
                    state.folder.clone_from(&folder);
                    state.search.clear();
                }
                drop_onto_folder(ui, &response, &folder, requests);
                response.context_menu(|ui| folder_menu(ui, &folder, state, requests));
            }
        });
}

/// A file dropped on a folder moves there.
fn drop_onto_folder(
    ui: &Ui,
    response: &egui::Response,
    folder: &str,
    requests: &mut Vec<LibraryRequest>,
) {
    if let Some(dragged) = response.dnd_release_payload::<LibraryDrag>() {
        let to = join(folder, dragged.0.file_name());
        if to != dragged.0.path {
            requests.push(LibraryRequest::Move(dragged.0.path.clone(), to));
        }
    } else if response.dnd_hover_payload::<LibraryDrag>().is_some() {
        ui.painter().rect_stroke(
            response.rect,
            3.0,
            egui::Stroke::new(1.5, theme::FOLDER),
            egui::StrokeKind::Inside,
        );
    }
}

/// A folder's right-click menu.
fn folder_menu(
    ui: &mut Ui,
    folder: &str,
    state: &mut LibraryState,
    requests: &mut Vec<LibraryRequest>,
) {
    ui.menu_button("New", |ui| new_items(ui, folder, requests));
    ui.separator();
    if ui.button("Show in Explorer").clicked() {
        requests.push(LibraryRequest::Show(folder.to_string()));
        ui.close();
    }
    if !folder.is_empty() {
        ui.separator();
        if ui.button("Rename…").clicked() {
            let name = folder.rsplit_once('/').map_or(folder, |(_, n)| n);
            state.renaming = Some((folder.to_string(), name.to_string()));
            ui.close();
        }
        if ui
            .button("Delete")
            .on_hover_text("Moves the folder and everything in it to the Recycle Bin")
            .clicked()
        {
            requests.push(LibraryRequest::Delete(folder.to_string()));
            ui.close();
        }
    }
}

/// "Harbour › props › old", each part clickable, above the files.
fn breadcrumb(ui: &mut Ui, project: &str, state: &mut LibraryState) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        let mut go = None;
        if ui.link(project).clicked() {
            go = Some(String::new());
        }
        let mut so_far = String::new();
        for part in state.folder.split('/').filter(|p| !p.is_empty()) {
            so_far = join(&so_far, part);
            ui.label(RichText::new("›").color(theme::text_weak()));
            if ui.link(part).clicked() {
                go = Some(so_far.clone());
            }
        }
        if let Some(folder) = go {
            state.folder = folder;
        }
    });
}

/// `name` inside `folder`.
fn join(folder: &str, name: &str) -> String {
    if folder.is_empty() {
        name.to_string()
    } else {
        format!("{folder}/{name}")
    }
}

/// What an item's right-click menu offers to do with it
/// ([`LibraryKind::menu_actions`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MenuAction {
    /// Open it: in its tab in the editor, or in the computer's own
    /// program for it.
    Open,
    /// Place it in the area, where the camera looks.
    Place,
}

impl MenuAction {
    /// Its name in the menu.
    fn label(self) -> &'static str {
        match self {
            Self::Open => "Open",
            Self::Place => "Place",
        }
    }
}

/// What happened to one item this frame.
struct Picked<'a> {
    item: &'a LibraryItem,
    response: egui::Response,
    place: bool,
    /// Play or Stop, from a sound's ▶ or ■.
    listen: Option<LibraryRequest>,
    /// It is on the Library tab, where a double-click only selects a
    /// file that would be placed (the project lead's answer 25): **Place**
    /// on the ribbon places it.
    in_tab: bool,
}

/// Why Place is greyed for a file with no ID.
const NO_ID_YET: &str =
    "This file has no ID yet, and one is needed to place it. Try again in a moment.";

/// What clicks, double-clicks, drags and right-clicks on `item`'s
/// `response` ask for, where nothing else is drawn on it (the Assets
/// column's tiles).
pub(crate) fn act_on(
    item: &LibraryItem,
    response: egui::Response,
    state: &mut LibraryState,
    requests: &mut Vec<LibraryRequest>,
) {
    let picked = Picked {
        item,
        response,
        place: false,
        listen: None,
        in_tab: false,
    };
    item_actions(picked, state, requests);
}

/// What clicks, double-clicks, drags and right-clicks on an item ask for.
fn item_actions(picked: Picked<'_>, state: &mut LibraryState, requests: &mut Vec<LibraryRequest>) {
    let Picked {
        item,
        response,
        place,
        listen,
        in_tab,
    } = picked;
    requests.extend(listen);
    if item.kind == LibraryKind::SoundEvent
        && let Some(dragged) = response.dnd_release_payload::<LibraryDrag>()
    {
        requests.extend(crate::sound_event_view::add_take(&item.path, &dragged.0));
    }
    response.dnd_set_drag_payload(LibraryDrag(item.clone()));
    if response.clicked() {
        state.selected = Some(item.path.clone());
    }
    if place && let Some(id) = item.id {
        requests.push(LibraryRequest::Place(id));
    } else if response.double_clicked() {
        match (item.kind.placeable(), item.id) {
            (true, Some(id))
                if !item.kind.opens_on_double_click() && item.kind != LibraryKind::Texture =>
            {
                if in_tab {
                    state.selected = Some(item.path.clone());
                } else {
                    requests.push(LibraryRequest::Place(id));
                }
            }
            _ => requests.push(LibraryRequest::Open(item.kind, item.path.clone())),
        }
    }
    response.context_menu(|ui| {
        ui.menu_button("New", |ui| new_items(ui, item.folder(), requests));
        ui.separator();
        if matches!(item.kind, LibraryKind::Texture | LibraryKind::SpriteSheet)
            && ui
                .button("Atlas Crop…")
                .on_hover_text(
                    "Cut named pictures out of it, to use one sprite or a few on their own",
                )
                .clicked()
        {
            requests.push(LibraryRequest::AtlasCrop(item.path.clone()));
            ui.close();
        }
        if item.kind == LibraryKind::Model
            && !crate::unbuilt::hidden()
            && ui.button("Open in the model viewer").clicked()
        {
            requests.push(LibraryRequest::ViewModel(item.path.clone()));
            ui.close();
        }
        if item.kind == LibraryKind::Sound && ui.button("Play").clicked() {
            requests.push(LibraryRequest::PlaySound(item.path.clone()));
            ui.close();
        }
        for action in item.kind.menu_actions() {
            let asked = match (action, item.id) {
                (MenuAction::Open, _) => Some(LibraryRequest::Open(item.kind, item.path.clone())),
                (MenuAction::Place, Some(id)) => Some(LibraryRequest::Place(id)),
                // Only a file with an ID can be placed: the item shows
                // greyed, and says why.
                (MenuAction::Place, None) => None,
            };
            let button = ui
                .add_enabled(asked.is_some(), egui::Button::new(action.label()))
                .on_disabled_hover_text(NO_ID_YET);
            if let Some(asked) = asked.filter(|_| button.clicked()) {
                requests.push(asked);
                ui.close();
            }
        }
        if ui.button("Show in Explorer").clicked() {
            requests.push(LibraryRequest::Show(item.folder().to_string()));
            ui.close();
        }
        ui.separator();
        if ui.button("Rename…").clicked() {
            state.renaming = Some((item.path.clone(), item.file_name().to_string()));
            ui.close();
        }
        if ui
            .button("Delete")
            .on_hover_text("Moves it to the Recycle Bin, with its ID file")
            .clicked()
        {
            requests.push(LibraryRequest::Delete(item.path.clone()));
            ui.close();
        }
    });
}

/// "1 clip", "6 clips".
pub(crate) fn clips_text(n: usize) -> String {
    if n == 1 {
        "1 clip".into()
    } else {
        format!("{n} clips")
    }
}

/// "1 frame", "24 frames".
pub(crate) fn frames_text(n: usize) -> String {
    if n == 1 {
        "1 frame".into()
    } else {
        format!("{n} frames")
    }
}

/// The files as tiles that fill the width, each at least 108 px (spec
/// 306): only the rows in view are drawn.
fn grid(
    ui: &mut Ui,
    items: &[&LibraryItem],
    state: &mut LibraryState,
    accent: Accent,
    sounds: Sounds<'_>,
    requests: &mut Vec<LibraryRequest>,
) {
    let gap = size::TILE_GAP;
    let per_row = ((ui.available_width() + gap) / (size::TILE_MIN_W + gap))
        .floor()
        .max(1.0) as usize;
    let width = ((ui.available_width() - (per_row - 1) as f32 * gap) / per_row as f32).floor();
    let rows = items.len().div_ceil(per_row);
    egui::ScrollArea::vertical().auto_shrink(false).show_rows(
        ui,
        tile_height(width) + gap,
        rows,
        |ui, range| {
            for row in range {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    for item in items.iter().skip(row * per_row).take(per_row) {
                        let picked = tile(ui, item, state, (accent, width), sounds);
                        item_actions(picked, state, requests);
                    }
                });
                ui.add_space(gap);
            }
        },
    );
}

/// How tall a tile `width` wide is: its square, its name and its kind.
fn tile_height(width: f32) -> f32 {
    width + size::GAP + 2.0 * size::ASSET_NAME_H
}

/// One tile (spec 306-309): the square thumbnail (an accent outline
/// inside it when picked), the name cut off with "…", and the kind's dot
/// and word; a + to place it.
fn tile<'a>(
    ui: &mut Ui,
    item: &'a LibraryItem,
    state: &LibraryState,
    (accent, width): (Accent, f32),
    sounds: Sounds<'_>,
) -> Picked<'a> {
    use egui::{CornerRadius, Stroke, StrokeKind, pos2, vec2};
    let (rect, response) = ui.allocate_exact_size(
        vec2(width, tile_height(width)),
        egui::Sense::click_and_drag(),
    );
    let selected = state.selected.as_deref() == Some(item.path.as_str());
    let round = CornerRadius::same(size::radius::TAB);
    if selected {
        ui.painter()
            .rect_filled(rect.expand(2.0), round, tokens::select_item(accent));
    }
    let picture = egui::Rect::from_min_size(rect.min, vec2(width, width));
    let fill = if response.hovered() {
        tokens::card_border()
    } else {
        tokens::hover()
    };
    ui.painter().rect(
        picture,
        round,
        fill,
        Stroke::new(size::LINE, tokens::field_border()),
        StrokeKind::Inside,
    );
    paint_thumbnail(ui.painter(), picture, item, sounds);
    if item.kind == LibraryKind::SoundEvent {
        event_badge(ui, item, picture, &response, sounds);
    }
    if selected {
        ui.painter().rect_stroke(
            picture,
            round,
            Stroke::new(size::TILE_OUTLINE, accent.colors().fill),
            StrokeKind::Inside,
        );
    }
    let name = egui::Rect::from_min_size(
        pos2(rect.left(), picture.bottom() + size::GAP),
        vec2(width, size::ASSET_NAME_H),
    );
    crate::widgets::cut_off(ui, name, &item.name, text::tile_name(), tokens::text_tile());
    let kind_line = name.translate(vec2(0.0, size::ASSET_NAME_H));
    let dot = pos2(
        kind_line.left() + size::KIND_DOT / 2.0,
        kind_line.center().y,
    );
    ui.painter()
        .circle_filled(dot, size::KIND_DOT / 2.0, item.colour());
    let words = egui::Rect::from_min_max(
        pos2(
            dot.x + size::KIND_DOT / 2.0 + size::ROW_GAP,
            kind_line.top(),
        ),
        kind_line.max,
    );
    crate::widgets::cut_off(
        ui,
        words,
        item.kind.name(),
        text::tile_kind(),
        tokens::text_meta(),
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Button, true, selected, &item.name)
    });
    let response = response.on_hover_text(hint(item));
    let place = item.kind.placeable() && place_button(ui, item, picture);
    let listen = listen_button(ui, item, picture, &response, sounds);
    Picked {
        item,
        response,
        place,
        listen,
        in_tab: true,
    }
}

/// The hint shown when pointing at an item.
fn hint(item: &LibraryItem) -> String {
    let what = match item.kind {
        LibraryKind::Model | LibraryKind::Assembly => {
            "Drag it into the viewport or onto an entity, or pick it and press Place, to place it."
        }
        LibraryKind::Area => "Double-click to open it.",
        LibraryKind::Sound => "▶ to listen; click it to see it in Properties.",
        LibraryKind::SpriteSheet => {
            "Drag it into the viewport, or pick it and press Place, to place it as a sprite; click \
             it to see its frames in Properties."
        }
        LibraryKind::SpriteClips => {
            "Double-click to open its clips; drag it into the viewport to place a sprite that \
             plays them."
        }
        LibraryKind::Tileset => {
            "Double-click to open it in the Tileset tab: its terrains (what a tile map's cells can \
             be), their art and how it joins up."
        }
        LibraryKind::TileMap => {
            "Drag it into the viewport, or pick it and press Place, to place it; click it to \
             change its tileset and size in Properties."
        }
        LibraryKind::Animator => {
            "Double-click to open it in the Animator tab: its states, and how it moves between \
             them."
        }
        LibraryKind::SoundEvent => {
            "▶ to listen (a different take each time); drop sound files on it to add takes; \
             click it to change it in Properties."
        }
        LibraryKind::Texture => {
            "Drag it into the viewport to place it as a sprite; double-click to open it in its \
             own program."
        }
        _ => "Double-click to open it in its own program.",
    };
    format!("{} · {}\n{what}", item.kind.name(), item.path)
}

/// What a file looks like in a tile's square `picture`: a sound's
/// waveform, a sheet's (or its clips') first frame with how many frames
/// or clips, a model's or picture's thumbnail, a text file's extension,
/// or its kind's icon.
pub(crate) fn paint_thumbnail(
    painter: &egui::Painter,
    picture: egui::Rect,
    item: &LibraryItem,
    sounds: Sounds<'_>,
) {
    let waveform = match sounds.states.get(&item.path) {
        Some(SoundState::Read(info)) if item.kind == LibraryKind::Sound => Some(info),
        _ => None,
    };
    // A clips file shows its sheet's first frame, and how many clips.
    let sheet_path = match item.kind {
        LibraryKind::SpriteSheet => Some(item.path.as_str()),
        LibraryKind::SpriteClips => item
            .id
            .and_then(|id| sounds.sheets.frames_file(id))
            .and_then(|f| sounds.sheets.paths.get(&f.sheet?))
            .map(String::as_str),
        _ => None,
    };
    let badge = match item.kind {
        LibraryKind::SpriteClips => item
            .id
            .and_then(|id| sounds.sheets.clips(id))
            .map(|set| clips_text(set.clips.len())),
        _ => None,
    };
    let sheet = sheet_path
        .and_then(|path| Some((sounds.sheets.at(path)?, sounds.sheet_pictures.get(path)?)));
    if let Some(info) = waveform {
        sound_view::draw_waveform(
            painter,
            picture.shrink2(egui::vec2(5.0, 10.0)),
            &info.waveform,
            sounds.played(item),
        );
    } else if let Some((sheet, atlas)) = sheet {
        crate::sheet_view::draw_frame(painter, picture.shrink(4.0), sheet, atlas, 0);
        painter.text(
            picture.right_top() + egui::vec2(-4.0, 3.0),
            egui::Align2::RIGHT_TOP,
            badge.unwrap_or_else(|| frames_text(sheet.frames.len())),
            egui::FontId::proportional(10.5),
            theme::text(),
        );
    } else if let Some(texture) = item.thumbnail {
        painter.image(
            texture,
            picture.shrink(2.0),
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    } else if let Some(extension) = item.text_extension() {
        // A text file's icon is its extension, in its colour.
        painter.text(
            picture.center(),
            egui::Align2::CENTER_CENTER,
            extension,
            egui::FontId::monospace(20.0),
            item.colour(),
        );
    } else {
        icons::icon(
            painter,
            egui::Rect::from_center_size(picture.center(), egui::vec2(28.0, 28.0)),
            item.kind.icon(),
            item.kind.color(),
        );
    }
}

/// The small + on a placeable item; returns whether it was clicked.
fn place_button(ui: &mut Ui, item: &LibraryItem, area: egui::Rect) -> bool {
    let plus = egui::Rect::from_min_size(
        egui::pos2(area.right() - 20.0, area.top() + 1.0),
        egui::vec2(19.0, 19.0),
    );
    let button = ui
        .put(
            plus,
            egui::Button::new(RichText::new("+").color(theme::text_strong())).frame(false),
        )
        .on_hover_text("Place it where the camera looks");
    button.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            true,
            format!("Place {}", item.name),
        )
    });
    button.clicked()
}

/// On a sound event's tile: how many takes it has, and a frame while a
/// sound file is dragged over it (dropping adds it as a take).
fn event_badge(
    ui: &Ui,
    item: &LibraryItem,
    picture: egui::Rect,
    response: &egui::Response,
    sounds: Sounds<'_>,
) {
    let painter = ui.painter();
    if let Some(EventState::Read { event, .. }) = sounds.events.get(&item.path) {
        painter.text(
            picture.right_bottom() + egui::vec2(-6.0, -7.0),
            egui::Align2::RIGHT_BOTTOM,
            format!("×{}", event.takes.len()),
            egui::FontId::proportional(11.0),
            SOUND_COLOR,
        );
    }
    let sound_over = response
        .dnd_hover_payload::<LibraryDrag>()
        .is_some_and(|d| d.0.kind == LibraryKind::Sound);
    if sound_over {
        painter.rect_stroke(
            picture,
            3.0,
            egui::Stroke::new(1.5, SOUND_COLOR),
            egui::StrokeKind::Inside,
        );
    }
}

/// A sound's ▶ (or ■ while it plays), shown while it's pointed at or
/// playing; returns what a click asks for.
fn listen_button(
    ui: &mut Ui,
    item: &LibraryItem,
    area: egui::Rect,
    response: &egui::Response,
    sounds: Sounds<'_>,
) -> Option<LibraryRequest> {
    if !matches!(item.kind, LibraryKind::Sound | LibraryKind::SoundEvent) {
        return None;
    }
    let playing = sounds.played(item).is_some();
    let spot = egui::Rect::from_min_size(
        egui::pos2(area.right() - 20.0, area.top() + 1.0),
        egui::vec2(19.0, 19.0),
    );
    let pointed = response.hovered() || ui.rect_contains_pointer(spot);
    if !playing && !pointed {
        return None;
    }
    let button = ui
        .put(spot, egui::Button::new("").frame(false))
        .on_hover_text(if playing { "Stop" } else { "Listen" });
    let shape = if playing { Icon::Stop } else { Icon::Play };
    icons::icon(
        ui.painter(),
        spot.shrink(4.5),
        shape,
        if button.hovered() {
            theme::text_strong()
        } else {
            SOUND_COLOR
        },
    );
    button.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            true,
            format!("{} {}", if playing { "Stop" } else { "Play" }, item.name),
        )
    });
    button.clicked().then(|| {
        if playing {
            LibraryRequest::StopSound
        } else {
            LibraryRequest::PlaySound(item.path.clone())
        }
    })
}

/// Items as a list, one row each; only the rows in view are drawn.
fn list(
    ui: &mut Ui,
    items: &[&LibraryItem],
    state: &mut LibraryState,
    accent: Accent,
    sounds: Sounds<'_>,
    requests: &mut Vec<LibraryRequest>,
) {
    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .show_rows(ui, ROW, items.len(), |ui, range| {
            ui.spacing_mut().item_spacing.y = 1.0;
            for item in &items[range] {
                let (rect, response) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), ROW),
                    egui::Sense::click_and_drag(),
                );
                let selected = state.selected.as_deref() == Some(item.path.as_str());
                if selected {
                    ui.painter()
                        .rect_filled(rect, 3.0, crate::tokens::select_row(accent));
                } else if response.hovered() {
                    ui.painter().rect_filled(rect, 3.0, theme::hover());
                }
                ui.painter().rect_filled(
                    egui::Rect::from_min_size(
                        egui::pos2(rect.left() + 1.0, rect.top() + 4.0),
                        egui::vec2(3.0, ROW - 8.0),
                    ),
                    1.0,
                    item.colour(),
                );
                let icon = egui::Rect::from_min_size(
                    egui::pos2(rect.left() + 8.0, rect.center().y - 6.0),
                    egui::vec2(12.0, 12.0),
                );
                match item.text_extension() {
                    Some(extension) => {
                        ui.painter().text(
                            icon.center(),
                            egui::Align2::CENTER_CENTER,
                            extension,
                            egui::FontId::monospace(8.5),
                            item.colour(),
                        );
                    }
                    None => icons::icon(ui.painter(), icon, item.kind.icon(), item.kind.color()),
                }
                let text = if state.search.is_empty() {
                    item.name.clone()
                } else {
                    format!("{}  ·  {}", item.name, item.folder())
                };
                ui.painter().text(
                    egui::pos2(rect.left() + 26.0, rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    text,
                    egui::TextStyle::Body.resolve(ui.style()),
                    ui.visuals().text_color(),
                );
                response.widget_info(|| {
                    egui::WidgetInfo::selected(egui::WidgetType::Button, true, selected, &item.name)
                });
                if let Some(uses) = sounds.code_uses.get(&item.path) {
                    ui.painter().text(
                        egui::pos2(rect.right() - 8.0, rect.center().y),
                        egui::Align2::RIGHT_CENTER,
                        uses,
                        egui::FontId::proportional(11.5),
                        theme::text_weak(),
                    );
                }
                let response = response.on_hover_text(hint(item));
                let plus_area =
                    egui::Rect::from_min_max(egui::pos2(rect.right() - 22.0, rect.top()), rect.max);
                let place = item.kind.placeable() && place_button(ui, item, plus_area);
                let listen = listen_button(ui, item, plus_area, &response, sounds);
                item_actions(
                    Picked {
                        item,
                        response,
                        place,
                        listen,
                        in_tab: true,
                    },
                    state,
                    requests,
                );
            }
        });
}

/// The small window for typing a new name.
fn rename_window(ui: &Ui, state: &mut LibraryState, requests: &mut Vec<LibraryRequest>) {
    let Some((path, mut name)) = state.renaming.take() else {
        return;
    };
    let mut keep = true;
    egui::Window::new("Rename")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ui.ctx(), |ui| {
            let edit = ui.add(egui::TextEdit::singleline(&mut name).desired_width(240.0));
            edit.request_focus();
            let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
            ui.horizontal(|ui| {
                let ok = ui.button("Rename").clicked() || enter;
                if ok {
                    let name = name.trim();
                    let folder = path.rsplit_once('/').map_or("", |(f, _)| f);
                    // The app checks the name fully when it moves the
                    // file (asset_db::valid_file_name).
                    let bad =
                        name.is_empty() || name.contains(['/', '\\']) || name.starts_with('.');
                    if !bad {
                        let to = join(folder, name);
                        if to != path {
                            requests.push(LibraryRequest::Move(path.clone(), to));
                        }
                    }
                    keep = false;
                }
                if ui.button("Cancel").clicked() {
                    keep = false;
                }
            });
            ui.label(
                RichText::new("Its ID file goes with it, so nothing that uses it breaks.")
                    .color(theme::text_weak())
                    .size(11.0),
            );
        });
    if keep && !ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        state.renaming = Some((path, name));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(path: &str, kind: LibraryKind) -> LibraryItem {
        LibraryItem {
            id: kind.placeable().then(AssetId::random),
            kind,
            name: path
                .rsplit('/')
                .next()
                .and_then(|n| n.split('.').next())
                .unwrap_or_default()
                .to_string(),
            path: path.to_string(),
            thumbnail: None,
        }
    }

    #[test]
    fn an_item_knows_its_folder() {
        assert_eq!(
            item("props/old/crate.glb", LibraryKind::Model).folder(),
            "props/old"
        );
        assert_eq!(item("crate.glb", LibraryKind::Model).folder(), "");
        assert_eq!(
            item("props/crate.glb", LibraryKind::Model).file_name(),
            "crate.glb"
        );
        assert_eq!(join("", "a"), "a");
        assert_eq!(join("x/y", "a"), "x/y/a");
    }

    #[test]
    fn the_library_shows_one_folder_or_every_match() {
        let items = [
            item("crate.glb", LibraryKind::Model),
            item("props/rock.glb", LibraryKind::Model),
            item("props/rocket.solassembly", LibraryKind::Assembly),
            item("turret.lst", LibraryKind::Code),
        ];
        let mut state = LibraryState::default();
        let names = |state: &LibraryState, code| -> Vec<String> {
            shown_items(&items, state, code)
                .iter()
                .map(|i| i.name.clone())
                .collect()
        };
        assert_eq!(names(&state, false), ["crate", "turret"]);
        state.folder = "props".into();
        assert_eq!(names(&state, false), ["rock", "rocket"]);
        state.search = "ROCK".into();
        state.folder.clear();
        assert_eq!(
            names(&state, false),
            ["rock", "rocket"],
            "from every folder"
        );
        state.search.clear();
        assert_eq!(names(&state, true), ["turret"], "the Code panel");
    }

    /// The review, BUG-30: no kind's menu names an action twice, and
    /// every kind's offers something.
    #[test]
    fn a_kind_s_menu_offers_open_or_place_or_both_each_once() {
        use LibraryKind as K;
        use MenuAction::{Open, Place};
        for (kind, offered) in [
            (K::Model, &[Place][..]),
            (K::Assembly, &[Place]),
            (K::Area, &[Open]),
            (K::Texture, &[Open, Place]),
            (K::Code, &[Open]),
            (K::Sound, &[Open]),
            (K::SoundEvent, &[Open]),
            (K::SpriteSheet, &[Place]),
            (K::SpriteClips, &[Open, Place]),
            (K::Animator, &[Open]),
            (K::Tileset, &[Open]),
            (K::TileMap, &[Place]),
            (K::Atlas, &[Open]),
            (K::Text, &[Open]),
            (K::Other, &[Open]),
        ] {
            assert_eq!(kind.menu_actions(), offered, "{kind:?}");
            let mut labels: Vec<&str> = offered.iter().map(|a| a.label()).collect();
            labels.sort_unstable();
            labels.dedup();
            assert_eq!(labels.len(), offered.len(), "{kind:?}");
            assert!(!labels.is_empty(), "{kind:?}");
            // What it offers follows what the kind can do.
            assert_eq!(offered.contains(&Place), kind.placeable(), "{kind:?}");
            if kind.opens_on_double_click() {
                assert!(offered.contains(&Open), "{kind:?}");
            }
        }
    }

    #[test]
    fn a_chip_shows_one_kind_of_file() {
        let items = [
            item("crate.glb", LibraryKind::Model),
            item("farmer.aseprite", LibraryKind::SpriteSheet),
            item("farm.tilemap.lst", LibraryKind::TileMap),
            item("birdsong.ogg", LibraryKind::Sound),
            item("main.lst", LibraryKind::Code),
        ];
        let mut state = LibraryState::default();
        let mut names = |filter| -> Vec<String> {
            state.filter = filter;
            shown_items(&items, &state, false)
                .iter()
                .map(|i| i.name.clone())
                .collect()
        };
        assert_eq!(names(Filter::All).len(), 5);
        assert_eq!(names(Filter::Models), ["crate"]);
        assert_eq!(names(Filter::Sprites), ["farmer"]);
        assert_eq!(names(Filter::Maps), ["farm"]);
        assert_eq!(names(Filter::Sounds), ["birdsong"]);
        assert_eq!(names(Filter::Code), ["main"]);
        // Every kind passes at least one chip besides All.
        for kind in [
            LibraryKind::Model,
            LibraryKind::Assembly,
            LibraryKind::Area,
            LibraryKind::Texture,
            LibraryKind::Code,
            LibraryKind::Sound,
            LibraryKind::SoundEvent,
            LibraryKind::SpriteSheet,
            LibraryKind::SpriteClips,
            LibraryKind::Animator,
            LibraryKind::Tileset,
            LibraryKind::TileMap,
        ] {
            assert!(Filter::ALL[1..].iter().any(|f| f.passes(kind)), "{kind:?}");
        }
    }
}
