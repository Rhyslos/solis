//! The Library: the project's own files and folders
//! ([decision 0021](../../../../docs/decisions/0021-asset-database-and-models.md),
//! the rethink's board 04, decision 0118). One browser, drawn the same on
//! Home's bottom panel and on the Library tab: a folder tree, a breadcrumb
//! with back and up, the files as tiles or a list, a kind filter and a
//! search. Right-click makes, renames, duplicates and deletes; a name is
//! renamed in place (double-click it, or `F2`); tiles are dragged onto a
//! folder to move them, or into the level to place them. Home's left slot
//! can show the project as folder trees instead of the Level panel,
//! several side by side (`trees.rs`). The Code panel shows the code files
//! as a list.
//!
//! The app keeps the list up to date from the asset database and makes
//! the thumbnails; the Library only shows them and returns
//! `LibraryRequest`s.

use crate::sound_view::{self, EventState, Preview, SoundState};
use crate::theme::Accent;
use egui::{Color32, Ui};
use reflect::AssetId;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::PathBuf;

mod browser;
mod kinds;
mod menus;
mod rename;
mod shown;
mod tiles;
mod tree;
mod trees;

pub(crate) use browser::tools;
pub(crate) use kinds::Filter;
pub use kinds::LibraryKind;
#[cfg(test)]
use rename::{checked_name, rename_to};
pub(crate) use shown::Entry;
#[cfg(test)]
use shown::shown;
pub(crate) use tiles::{frames_text, paint_thumbnail};
pub(crate) use trees::{LeftShows, MOST_TREES, Trees, folder_trees};

/// The open project, as the editor shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectInfo {
    /// Its name.
    pub name: String,
    /// Its folder.
    pub root: PathBuf,
    /// The folder its files are in, which the Library shows: its own
    /// folder (decision 0118), or an older project's `assets` folder.
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
    /// Where it is inside the project, with `/` between folders.
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

    /// The folder it's in, inside the project ("" for the project's own
    /// folder).
    pub fn folder(&self) -> &str {
        parent_of(&self.path)
    }

    /// Its file's name, with the extension.
    pub(crate) fn file_name(&self) -> &str {
        last_part(&self.path)
    }
}

/// What a Library item being dragged carries: the file under the pointer
/// (which the viewport, the Level panel and a card's field take), and
/// every file and folder being moved with it, itself first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LibraryDrag(pub(crate) LibraryItem, pub(crate) Vec<String>);

impl LibraryDrag {
    /// A folder being dragged, to move it: no file is placed by it.
    pub(crate) fn folder(path: &str) -> Self {
        let item = LibraryItem {
            kind: LibraryKind::Other,
            name: last_part(path).to_owned(),
            path: path.to_owned(),
            ..LibraryItem::default()
        };
        Self(item, vec![path.to_owned()])
    }
}

/// The project's files as the Library lists them, each with its name in
/// lower case, made once when the files change: searching compares every
/// name every frame (the second review pass: it lowered each one every
/// frame). Reads as a list of [`LibraryItem`]s.
#[derive(Debug, Clone, Default)]
pub(crate) struct LibraryFiles {
    items: Vec<LibraryItem>,
    lowered: Vec<String>,
}

impl From<Vec<LibraryItem>> for LibraryFiles {
    fn from(items: Vec<LibraryItem>) -> Self {
        let lowered = items.iter().map(|i| i.name.to_lowercase()).collect();
        Self { items, lowered }
    }
}

impl std::ops::Deref for LibraryFiles {
    type Target = [LibraryItem];

    fn deref(&self) -> &[LibraryItem] {
        &self.items
    }
}

impl LibraryFiles {
    /// Each file with its name in lower case.
    pub(crate) fn with_lowered(&self) -> impl Iterator<Item = (&LibraryItem, &str)> {
        self.items
            .iter()
            .zip(self.lowered.iter().map(String::as_str))
    }

    /// The file at `path`, if there is one.
    pub(crate) fn at(&self, path: &str) -> Option<&LibraryItem> {
        self.items.iter().find(|i| i.path == path)
    }
}

/// How the Library is shown: the folder open and those opened before it,
/// thumbnails or a list, the search, the kind filter, what's picked, the
/// name being typed, and Home's folder trees. The editor's own, never
/// saved with the project: Home and the Library tab share it, so the two
/// always show the same.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LibraryState {
    /// The folder shown, inside the project ("" for its own folder).
    pub(crate) folder: String,
    /// The folders shown before, newest last, for back.
    pub(crate) back: Vec<String>,
    /// Whether the browser's folder tree is out.
    pub(crate) folders_shown: bool,
    /// A list rather than thumbnails.
    pub(crate) list: bool,
    /// Only names containing this are shown, from every folder.
    pub(crate) search: String,
    /// The file or folder picked last, by path: the one Details shows.
    pub(crate) selected: Option<String>,
    /// The others picked with it (`Ctrl` or `Shift` and a click).
    pub(crate) also: BTreeSet<String>,
    /// The file or folder being renamed, and the name typed so far.
    pub(crate) renaming: Option<(String, String)>,
    /// Whether the box to rename in has had the keyboard yet.
    pub(crate) rename_focused: bool,
    /// The frame the box to rename in was last drawn: where the same file
    /// shows in more than one place (the browser and a tree), only the
    /// first drawn each frame has the box.
    pub(crate) rename_frame: u64,
    /// A file or folder just made: renamed as soon as it's listed.
    pub(crate) rename_new: Option<String>,
    /// The kind filter: every file, or one group of kinds.
    pub(crate) filter: Filter,
    /// What Home's left slot shows, and its folder trees.
    pub(crate) left: LeftShows,
    pub(crate) trees: Trees,
}

impl Default for LibraryState {
    fn default() -> Self {
        Self {
            folder: String::new(),
            back: Vec::new(),
            folders_shown: true,
            list: false,
            search: String::new(),
            selected: None,
            also: BTreeSet::new(),
            renaming: None,
            rename_focused: false,
            rename_frame: u64::MAX,
            rename_new: None,
            filter: Filter::All,
            left: LeftShows::Level,
            trees: Trees::default(),
        }
    }
}

/// The most folders kept for back.
const BACK_KEPT: usize = 64;

impl LibraryState {
    /// Opens `folder`, remembering the one shown for back. The search is
    /// cleared, so the folder's files are in view, and what was picked is
    /// let go, as it's out of view.
    pub(crate) fn open_folder(&mut self, folder: &str) {
        if folder != self.folder {
            let was = std::mem::replace(&mut self.folder, folder.to_owned());
            self.back.push(was);
            if self.back.len() > BACK_KEPT {
                self.back.remove(0);
            }
            self.let_go();
        }
        self.search.clear();
    }

    /// Lets go of everything picked.
    fn let_go(&mut self) {
        self.selected = None;
        self.also.clear();
    }

    /// Goes back to the folder shown before, if there was one.
    pub(crate) fn go_back(&mut self) {
        if let Some(folder) = self.back.pop() {
            self.folder = folder;
            self.search.clear();
            self.let_go();
        }
    }

    /// Opens the folder holding the one shown (nothing at the top).
    pub(crate) fn go_up(&mut self) {
        if !self.folder.is_empty() {
            let up = parent_of(&self.folder).to_owned();
            self.open_folder(&up);
        }
    }

    /// Whether `path` is picked.
    pub(crate) fn is_picked(&self, path: &str) -> bool {
        self.selected.as_deref() == Some(path) || self.also.contains(path)
    }

    /// Everything picked, the last one picked first.
    pub(crate) fn picked(&self) -> Vec<String> {
        self.selected
            .iter()
            .cloned()
            .chain(
                self.also
                    .iter()
                    .filter(|p| Some(p.as_str()) != self.selected.as_deref())
                    .cloned(),
            )
            .collect()
    }

    /// Picks `path` alone.
    pub(crate) fn pick(&mut self, path: &str) {
        self.selected = Some(path.to_owned());
        self.also.clear();
    }

    /// `Ctrl` and a click: adds `path` to what's picked, or takes it out.
    pub(crate) fn pick_also(&mut self, path: &str) {
        if self.selected.as_deref() == Some(path) {
            self.selected = self.also.pop_first();
        } else if !self.also.remove(path) {
            if let Some(was) = self.selected.replace(path.to_owned()) {
                self.also.insert(was);
            }
        }
    }

    /// `Shift` and a click: picks every entry from the one picked last to
    /// `path`, in the order `order` shows them.
    pub(crate) fn pick_range(&mut self, path: &str, order: &[&str]) {
        let from = self
            .selected
            .as_deref()
            .and_then(|p| order.iter().position(|o| *o == p));
        let to = order.iter().position(|o| *o == path);
        match (from, to) {
            (Some(from), Some(to)) => {
                let (low, high) = (from.min(to), from.max(to));
                self.also = order[low..=high]
                    .iter()
                    .map(|p| (*p).to_owned())
                    .filter(|p| Some(p.as_str()) != self.selected.as_deref())
                    .collect();
            }
            _ => self.pick(path),
        }
    }

    /// Starts renaming `path`, whose name is `name`.
    pub(crate) fn start_renaming(&mut self, path: &str, name: &str) {
        self.renaming = Some((path.to_owned(), name.to_owned()));
        self.rename_focused = false;
    }
}

/// Something asked for in the Library. Paths are inside the project,
/// with `/` between folders.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum LibraryRequest {
    /// Place a model or assembly where the camera looks.
    Place(AssetId),
    /// Open a level, or a file in its own program.
    Open(LibraryKind, String),
    /// Show a folder in the system's file browser.
    Show(String),
    /// Rename or move a file or folder: from, to.
    Move(String, String),
    /// Make a copy of a file beside it, under a free name.
    Duplicate(String),
    /// Make a new folder inside this one.
    NewFolder(String),
    /// Bring the Library tab to the front (the placing form's **Open
    /// Library**).
    ShowLibraryTab,
    /// Make a new Lust code file in this folder (E1).
    NewCodeFile(String),
    /// Make a new, empty level in this folder, saved (E1).
    NewLevel(String),
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
    /// Say in the Console why something couldn't be done (a name that
    /// won't do).
    Say(String),
}

/// The sound files the app has measured, by path inside the project, and
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
    /// many entities in the level use each (`Player · 1`), for the Code
    /// panel (decision 0085).
    pub(crate) code_uses: &'a BTreeMap<String, String>,
}

impl Sounds<'_> {
    /// How far through `item` the preview is, if it's playing.
    fn played(&self, item: &LibraryItem) -> Option<f32> {
        sound_view::played(&item.path, self.preview)
    }
}

/// What the Library draws from: the project, its files and folders, the
/// accent, and what the app has read of them.
#[derive(Clone, Copy)]
pub(crate) struct Files<'a> {
    pub(crate) project: &'a ProjectInfo,
    pub(crate) items: &'a LibraryFiles,
    pub(crate) folders: &'a [String],
    pub(crate) accent: Accent,
    pub(crate) sounds: Sounds<'a>,
}

/// Where the browser is drawn: its tools are in its own header on the
/// Library tab, and in the bar or header above it elsewhere ([`tools`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Place {
    /// The Library tab's centre: the browser's own header, "Library" and
    /// the project's name, with its tools.
    Tab,
    /// Home's bottom panel, or a column beside the view.
    Under,
}

/// The Library panel (or, with `code`, the Code panel: code files only,
/// as a list). Returns what was asked for.
pub(crate) fn library(
    ui: &mut Ui,
    project: Option<&ProjectInfo>,
    (items, folders): (&LibraryFiles, &[String]),
    state: &mut LibraryState,
    (accent, place, code): (Accent, Place, bool),
    sounds: Sounds<'_>,
) -> Vec<LibraryRequest> {
    let mut requests = Vec::new();
    let Some(project) = project else {
        crate::panels::empty_note(
            ui,
            "No project is open. Its files show here, with IDs that survive renaming.",
        );
        return requests;
    };
    let files = Files {
        project,
        items,
        folders,
        accent,
        sounds,
    };
    if code {
        tiles::code_list(ui, files, state, &mut requests);
    } else {
        browser::browser(ui, files, state, place, &mut requests);
    }
    requests
}

/// What clicks, double-clicks, drags and right-clicks on `item`'s
/// `response` ask for, where nothing else is drawn on it (the placing
/// form's tiles).
pub(crate) fn act_on(
    item: &LibraryItem,
    response: egui::Response,
    state: &mut LibraryState,
    requests: &mut Vec<LibraryRequest>,
) {
    let picked = tiles::Picked {
        entry: Entry::File(item),
        response,
        name: None,
        place: false,
        listen: None,
        placing: true,
    };
    tiles::item_actions(picked, state, None, &[], requests);
}

/// `name` inside `folder`.
pub(crate) fn join(folder: &str, name: &str) -> String {
    if folder.is_empty() {
        name.to_string()
    } else {
        format!("{folder}/{name}")
    }
}

/// The folder holding `path` ("" at the top).
pub(crate) fn parent_of(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(folder, _)| folder)
}

/// The last part of `path`: a file's or folder's own name.
pub(crate) fn last_part(path: &str) -> &str {
    path.rsplit_once('/').map_or(path, |(_, name)| name)
}

/// What moving each of `paths` into `folder` asks for: nothing for one
/// already there, or a folder into itself or a folder inside it.
pub(crate) fn moves_into(folder: &str, paths: &[String]) -> Vec<LibraryRequest> {
    paths
        .iter()
        .filter(|path| parent_of(path) != folder)
        .filter(|path| folder != path.as_str() && !folder.starts_with(&format!("{path}/")))
        .map(|path| LibraryRequest::Move(path.clone(), join(folder, last_part(path))))
        .collect()
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
