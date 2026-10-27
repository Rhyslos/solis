//! The Library panel: the project's files
//! ([decision 0021](../../../docs/decisions/0021-asset-database-and-models.md)),
//! with a folder tree that can be put away, the files of one folder as
//! thumbnails or a list, and a search box. Drag a model or assembly onto
//! the viewport or the Area panel to place it, or onto a folder to move
//! it; double-click to place, open an area, or open a file in its own
//! program; right-click for the rest. The Code tab shows the code files
//! the same way.
//!
//! The app keeps the list up to date from the asset database and makes
//! the thumbnails; the panel only shows them and returns
//! `LibraryRequest`s.

use crate::icons::{self, Icon};
use crate::sound_view::{self, Preview, SOUND_COLOR, SoundState};
use crate::theme::{self, Accent};
use egui::{Align, Color32, Layout, RichText, Ui};
use reflect::AssetId;
use std::collections::HashMap;
use std::path::PathBuf;

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

/// What kind of file a Library item is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LibraryKind {
    /// A model.
    #[default]
    Model,
    /// An assembly.
    Assembly,
    /// An area.
    Area,
    /// A picture.
    Texture,
    /// Game code (Lust, Lua or Rust).
    Code,
    /// A sound file (WAV or Ogg Vorbis).
    Sound,
    /// Anything else.
    Other,
}

impl LibraryKind {
    /// What it's called, for tooltips.
    fn name(self) -> &'static str {
        match self {
            Self::Model => "Model",
            Self::Assembly => "Assembly",
            Self::Area => "Area",
            Self::Texture => "Picture",
            Self::Code => "Code",
            Self::Sound => "Sound",
            Self::Other => "File",
        }
    }

    /// The stripe under its thumbnail, a colour per kind (decision 0004).
    fn color(self) -> Color32 {
        match self {
            Self::Model => Color32::from_rgb(0x4a, 0x9c, 0xd9),
            Self::Assembly => theme::FOLDER,
            Self::Area => theme::axis::Y,
            Self::Texture => Color32::from_rgb(0xb4, 0x7c, 0xf0),
            Self::Code => Color32::from_rgb(0xe8, 0x87, 0x5a),
            Self::Sound => SOUND_COLOR,
            Self::Other => theme::TEXT_WEAK,
        }
    }

    /// Its icon when it has no thumbnail.
    fn icon(self) -> Icon {
        match self {
            Self::Model => Icon::Model,
            Self::Assembly => Icon::Area,
            Self::Area => Icon::AreaFile,
            Self::Texture => Icon::Viewport,
            Self::Code => Icon::Code,
            Self::Sound => Icon::Sound,
            Self::Other => Icon::Properties,
        }
    }

    /// Whether it can be placed in an area (dragged in, double-clicked).
    pub fn placeable(self) -> bool {
        matches!(self, Self::Model | Self::Assembly)
    }
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
        }
    }
}

/// Something asked for in the Library. Paths are inside the assets, with
/// `/` between folders.
#[derive(Debug, Clone, PartialEq, Eq)]
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
    /// Move a file or folder to the Recycle Bin.
    Delete(String),
    /// Import a model from outside the project.
    Import,
    /// Listen to a sound file.
    PlaySound(String),
    /// Stop listening.
    StopSound,
}

/// The sound files the app has measured, by path inside the assets, and
/// the one previewing.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Sounds<'a> {
    pub(crate) states: &'a HashMap<String, SoundState>,
    pub(crate) preview: Option<&'a Preview>,
}

impl Sounds<'_> {
    /// How far through `item` the preview is, if it's playing.
    fn played(&self, item: &LibraryItem) -> Option<f32> {
        match self.states.get(&item.path) {
            Some(SoundState::Read(info)) => sound_view::played(info, &item.path, self.preview),
            _ => None,
        }
    }
}

/// Tile and row sizes.
const TILE: egui::Vec2 = egui::vec2(88.0, 104.0);
const PICTURE: f32 = 72.0;
const ROW: f32 = 21.0;
const TREE_WIDTH: f32 = 150.0;

/// The Library panel (or, with `code`, the Code panel: code files only,
/// as a list). Returns what was asked for.
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
    menu_row(ui, state, code, project.is_some(), &mut requests);
    let Some(project) = project else {
        note(
            ui,
            "No project is open. File > New project… makes one: a folder whose models are kept \
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
            egui::Stroke::new(1.0, theme::LINE),
        );
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
                "{} has no code yet. Lust, Lua and Rust files in its assets folder show here; \
                 writing game code in Solis arrives in Phase 4.",
                project.name
            )
        } else if items.is_empty() {
            format!(
                "{} has no files yet. Import a model (File > Import model…), drop one onto the \
                 window, or put files in its assets folder.",
                project.name
            )
        } else {
            "This folder is empty.".to_string()
        };
        files.label(RichText::new(text).color(theme::TEXT_WEAK));
    } else if state.list || code {
        list(&mut files, &shown, state, accent, sounds, &mut requests);
    } else {
        grid(&mut files, &shown, state, accent, sounds, &mut requests);
    }
    ui.advance_cursor_after_rect(body);
    rename_window(ui, state, &mut requests);
    requests
}

/// A quiet note in the panel.
fn note(ui: &mut Ui, text: &str) {
    egui::Frame::new()
        .inner_margin(theme::panel_margin())
        .show(ui, |ui| {
            ui.label(RichText::new(text).color(theme::TEXT_WEAK));
        });
}

/// The items to show: the open folder's (or every folder's matches while
/// searching), code files only in the Code panel and never in the
/// Library.
fn shown_items<'a>(
    items: &'a [LibraryItem],
    state: &LibraryState,
    code: bool,
) -> Vec<&'a LibraryItem> {
    let search = state.search.trim().to_lowercase();
    items
        .iter()
        .filter(|i| (i.kind == LibraryKind::Code) == code)
        .filter(|i| {
            if search.is_empty() {
                code || i.folder() == state.folder
            } else {
                i.name.to_lowercase().contains(&search)
            }
        })
        .collect()
}

/// ≡, Import, View and the search box.
fn menu_row(
    ui: &mut Ui,
    state: &mut LibraryState,
    code: bool,
    open: bool,
    requests: &mut Vec<LibraryRequest>,
) {
    theme::menu_row_frame().show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        egui::MenuBar::new().ui(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            ui.label("≡");
            if !code {
                ui.menu_button("Import", |ui| {
                    if ui
                        .button("Import model…")
                        .on_hover_text("Copies a glTF model into the project")
                        .clicked()
                    {
                        requests.push(LibraryRequest::Import);
                        ui.close();
                    }
                    if ui
                        .add_enabled(open, egui::Button::new("New folder"))
                        .on_hover_text("A new folder inside the one shown")
                        .clicked()
                    {
                        requests.push(LibraryRequest::NewFolder(state.folder.clone()));
                        ui.close();
                    }
                });
                ui.menu_button("View", |ui| {
                    if ui
                        .add(egui::Button::new("Thumbnails").selected(!state.list))
                        .clicked()
                    {
                        state.list = false;
                        ui.close();
                    }
                    if ui
                        .add(egui::Button::new("List").selected(state.list))
                        .clicked()
                    {
                        state.list = true;
                        ui.close();
                    }
                    ui.separator();
                    ui.checkbox(&mut state.folders_shown, "Folder tree");
                });
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let search = ui.add(
                    egui::TextEdit::singleline(&mut state.search)
                        .hint_text("Search")
                        .desired_width(ui.available_width().clamp(40.0, 160.0)),
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
                        .rect_filled(rect, 3.0, accent.colors().fill.gamma_multiply(0.45));
                } else if response.hovered() {
                    ui.painter().rect_filled(rect, 3.0, theme::HOVER);
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
    if ui.button("New folder").clicked() {
        requests.push(LibraryRequest::NewFolder(folder.to_string()));
        ui.close();
    }
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
            ui.label(RichText::new("›").color(theme::TEXT_WEAK));
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

/// What happened to one item this frame.
struct Picked<'a> {
    item: &'a LibraryItem,
    response: egui::Response,
    place: bool,
    /// Play or Stop, from a sound's ▶ or ■.
    listen: Option<LibraryRequest>,
}

/// What clicks, double-clicks, drags and right-clicks on an item ask for.
fn item_actions(picked: Picked<'_>, state: &mut LibraryState, requests: &mut Vec<LibraryRequest>) {
    let Picked {
        item,
        response,
        place,
        listen,
    } = picked;
    requests.extend(listen);
    response.dnd_set_drag_payload(LibraryDrag(item.clone()));
    if response.clicked() {
        state.selected = Some(item.path.clone());
    }
    if place && let Some(id) = item.id {
        requests.push(LibraryRequest::Place(id));
    } else if response.double_clicked() {
        match (item.kind.placeable(), item.id) {
            (true, Some(id)) => requests.push(LibraryRequest::Place(id)),
            _ => requests.push(LibraryRequest::Open(item.kind, item.path.clone())),
        }
    }
    response.context_menu(|ui| {
        if item.kind == LibraryKind::Sound && ui.button("Play").clicked() {
            requests.push(LibraryRequest::PlaySound(item.path.clone()));
            ui.close();
        }
        if item.kind.placeable() {
            if ui.button("Place").clicked()
                && let Some(id) = item.id
            {
                requests.push(LibraryRequest::Place(id));
                ui.close();
            }
        } else if ui.button("Open").clicked() {
            requests.push(LibraryRequest::Open(item.kind, item.path.clone()));
            ui.close();
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

/// The hint shown when pointing at an item.
fn hint(item: &LibraryItem) -> String {
    let what = match item.kind {
        LibraryKind::Model | LibraryKind::Assembly => {
            "Drag it into the viewport or onto an entity, or double-click, to place it."
        }
        LibraryKind::Area => "Double-click to open it.",
        LibraryKind::Sound => "▶ to listen; click it to see it in Properties.",
        _ => "Double-click to open it in its own program.",
    };
    format!("{} · {}\n{what}", item.kind.name(), item.path)
}

/// Items as thumbnails, row by row; only the rows in view are drawn.
fn grid(
    ui: &mut Ui,
    items: &[&LibraryItem],
    state: &mut LibraryState,
    accent: Accent,
    sounds: Sounds<'_>,
    requests: &mut Vec<LibraryRequest>,
) {
    let gap = 6.0;
    let per_row = ((ui.available_width() + gap) / (TILE.x + gap))
        .floor()
        .max(1.0) as usize;
    let rows = items.len().div_ceil(per_row);
    egui::ScrollArea::vertical().auto_shrink(false).show_rows(
        ui,
        TILE.y + gap,
        rows,
        |ui, range| {
            for row in range {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    for item in items.iter().skip(row * per_row).take(per_row) {
                        let picked = tile(ui, item, state, accent, sounds);
                        item_actions(picked, state, requests);
                    }
                });
                ui.add_space(gap);
            }
        },
    );
}

/// One thumbnail tile: the picture (or the kind's icon), the kind's
/// stripe, the name, and a + to place it.
fn tile<'a>(
    ui: &mut Ui,
    item: &'a LibraryItem,
    state: &LibraryState,
    accent: Accent,
    sounds: Sounds<'_>,
) -> Picked<'a> {
    let (rect, response) = ui.allocate_exact_size(TILE, egui::Sense::click_and_drag());
    let selected = state.selected.as_deref() == Some(item.path.as_str());
    let painter = ui.painter();
    let picture = egui::Rect::from_min_size(rect.min, egui::vec2(TILE.x, PICTURE));
    painter.rect_filled(picture, 3.0, theme::FIELD);
    let waveform = match sounds.states.get(&item.path) {
        Some(SoundState::Read(info)) if item.kind == LibraryKind::Sound => Some(info),
        _ => None,
    };
    if let Some(info) = waveform {
        sound_view::draw_waveform(
            painter,
            picture.shrink2(egui::vec2(5.0, 10.0)),
            &info.waveform,
            sounds.played(item),
        );
    } else if let Some(texture) = item.thumbnail {
        painter.image(
            texture,
            picture.shrink(2.0),
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    } else {
        icons::icon(
            painter,
            egui::Rect::from_center_size(picture.center(), egui::vec2(28.0, 28.0)),
            item.kind.icon(),
            item.kind.color(),
        );
    }
    painter.rect_filled(
        egui::Rect::from_min_max(
            egui::pos2(picture.left(), picture.bottom() - 3.0),
            picture.max,
        ),
        0.0,
        item.kind.color(),
    );
    let name_rect = egui::Rect::from_min_max(egui::pos2(rect.left(), picture.bottom()), rect.max);
    let galley = ui.painter().layout(
        item.name.clone(),
        egui::FontId::proportional(11.5),
        theme::TEXT,
        TILE.x - 4.0,
    );
    ui.painter().galley(
        egui::pos2(name_rect.left() + 2.0, name_rect.top() + 3.0),
        galley,
        theme::TEXT,
    );
    if selected {
        ui.painter().rect_stroke(
            rect.expand(1.0),
            4.0,
            egui::Stroke::new(1.5, accent.colors().light),
            egui::StrokeKind::Outside,
        );
    } else if response.hovered() {
        ui.painter().rect_stroke(
            rect.expand(1.0),
            4.0,
            egui::Stroke::new(1.0, theme::BUTTON_BORDER),
            egui::StrokeKind::Outside,
        );
    }
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
            egui::Button::new(RichText::new("+").color(theme::TEXT_STRONG)).frame(false),
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

/// A sound's ▶ (or ■ while it plays), shown while it's pointed at or
/// playing; returns what a click asks for.
fn listen_button(
    ui: &mut Ui,
    item: &LibraryItem,
    area: egui::Rect,
    response: &egui::Response,
    sounds: Sounds<'_>,
) -> Option<LibraryRequest> {
    if item.kind != LibraryKind::Sound {
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
            theme::TEXT_STRONG
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
                        .rect_filled(rect, 3.0, accent.colors().fill.gamma_multiply(0.45));
                } else if response.hovered() {
                    ui.painter().rect_filled(rect, 3.0, theme::HOVER);
                }
                ui.painter().rect_filled(
                    egui::Rect::from_min_size(
                        egui::pos2(rect.left() + 1.0, rect.top() + 4.0),
                        egui::vec2(3.0, ROW - 8.0),
                    ),
                    1.0,
                    item.kind.color(),
                );
                icons::icon(
                    ui.painter(),
                    egui::Rect::from_min_size(
                        egui::pos2(rect.left() + 8.0, rect.center().y - 6.0),
                        egui::vec2(12.0, 12.0),
                    ),
                    item.kind.icon(),
                    item.kind.color(),
                );
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
                    .color(theme::TEXT_WEAK)
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
    fn the_library_shows_one_folder_or_every_match_and_never_code() {
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
        assert_eq!(names(&state, false), ["crate"]);
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
}
