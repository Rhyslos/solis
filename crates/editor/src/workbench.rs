//! The whole editor window: the top bar and the docked panels
//! ([decision 0004](../../../docs/decisions/0004-editor-layout.md)).
//!
//! The workbench never touches the window, the GPU or files itself: it
//! returns [`WorkbenchAction`]s for the app to carry out, and draws the
//! viewport through a [`ViewportRenderer`] it is handed each frame.

use crate::area_view::AreaView;
use crate::camera::Camera;
#[cfg(test)]
use crate::console::CONSOLE_LIMIT;
use crate::console::{Console, LogLevel, LogLine};
use crate::document::Document;
use crate::layout;
use crate::library::{LibraryItem, LibraryKind, ProjectInfo};
use crate::models::ModelInfo;
use crate::panel::Panel;
use crate::panels::{PanelViewer, drop_hint};
use crate::properties::AssemblyPart;
use crate::theme::{self, Accent};
use crate::viewport::{Overlays, Tool, Viewport, ViewportRenderer};
use egui::Ui;
use egui_dock::{DockArea, DockState};
use reflect::{AssetId, EntityId};
use std::collections::{BTreeMap, HashMap, HashSet};

/// Facts about the program, shown in the window title and the About box.
/// The app passes these in, so this crate never names the program itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppInfo {
    /// The program's name, such as "Solis".
    pub name: String,
    /// The version, such as "0.0.1".
    pub version: String,
}

/// Something the app must do, asked for by the editor.
#[derive(Debug, Clone, PartialEq)]
pub enum WorkbenchAction {
    /// Close the window.
    Quit,
    /// Ask which model file to import (File > Import model…).
    ImportModel,
    /// Bring in the built-in sample area (Help > Add the sample area).
    AddSampleArea,
    /// Make the turret sample game as a new project, in a folder the app
    /// asks for (Help > Make the sample game).
    MakeSampleGame,
    /// Make the benchmark as a new project, in a folder the app asks for
    /// (Help > Make the benchmark).
    MakeBenchmark,
    /// Make the playground (a level for the character, the cameras,
    /// physics and saves) as a new project, in a folder the app asks for
    /// (Help > Make the playground).
    MakePlayground,
    /// Make a new project in a folder the user picks (File > New project…).
    NewProject,
    /// Open a project from a folder the user picks (File > Open project…).
    OpenProject,
    /// Open the project in this folder (File > Recent projects).
    OpenRecent(std::path::PathBuf),
    /// Place the project's asset where the camera looks (a double-click in
    /// the Library).
    PlaceAsset(AssetId),
    /// Start a new area (File > New area, Ctrl+N): the app calls
    /// [`Workbench::new_area`].
    NewArea,
    /// Ask which area file to open (File > Open area…, Ctrl+O).
    OpenArea,
    /// Save the area to its file, or ask where if it has none (Ctrl+S).
    SaveArea,
    /// Ask where to save the area (Ctrl+Shift+S).
    SaveAreaAs,
    /// Save this entity and everything under it as an assembly in the
    /// project, then turn it into a copy of it with
    /// [`Workbench::make_copy`].
    CreateAssembly(EntityId),
    /// Open this area file (a double-click in the Library); the app reads
    /// it and calls [`Workbench::open_area`].
    OpenAreaFile(std::path::PathBuf),
    /// Open this file or folder in the system's own program or file
    /// browser.
    OpenExternally(std::path::PathBuf),
    /// Rename or move a file or folder inside the project's assets, with
    /// its sidecar.
    MoveFile {
        /// Where it is, inside the assets.
        from: std::path::PathBuf,
        /// Where it goes, inside the assets.
        to: std::path::PathBuf,
    },
    /// Make a new folder inside this one (inside the assets).
    NewFolder(std::path::PathBuf),
    /// Move a file or folder (inside the assets) to the Recycle Bin.
    DeleteFile(std::path::PathBuf),
    /// Listen to this sound file (inside the project's assets) in the
    /// Library or Properties; the app plays it and reports how far it has
    /// got with [`Workbench::set_preview`].
    PlaySound(std::path::PathBuf),
    /// Stop listening to the sound playing.
    StopSound,
    /// Make a new sound event in this folder (inside the project's
    /// assets), and show it with [`Workbench::show_in_library`].
    NewSoundEvent(std::path::PathBuf),
    /// Save a sound event's settings to its `.solsound` file (inside the
    /// project's assets).
    SaveSoundEvent {
        /// The file.
        path: std::path::PathBuf,
        /// What to save.
        event: sound::SoundEvent,
    },
    /// Save the project's buses to its `mixer.solmix` (the Mixer).
    SaveMixer(sound::Mixer),
    /// Save a sprite's clips file (`.solframes`, inside the project's
    /// assets), making it if it isn't there yet (Phase 9, D3).
    SaveFrames {
        /// The file.
        path: std::path::PathBuf,
        /// What to save.
        text: String,
    },
    /// Save an animator file (`.solanim`, inside the project's assets),
    /// making it if it isn't there yet (Phase 9, D3b).
    SaveAnimator {
        /// The file.
        path: std::path::PathBuf,
        /// What to save.
        text: String,
    },
    /// Save a tileset (`.soltileset`, inside the project's assets), making
    /// it if it isn't there yet (Phase 9, D4).
    SaveTileset {
        /// The file.
        path: std::path::PathBuf,
        /// What to save.
        text: String,
    },
    /// Save a tile map file (`.soltilemap`, inside the project's assets),
    /// making it if it isn't there yet (Phase 9, D4).
    SaveTileMap {
        /// The file.
        path: std::path::PathBuf,
        /// What to save.
        text: String,
    },
    /// Save the project's 2D settings to its `pixels.solpix` (Settings >
    /// 2D).
    SavePixels(area::pixels::PixelSettings),
    /// Change one field of an entity inside an assembly's file (Apply to
    /// assembly); the app saves it and hands it back with
    /// [`Workbench::set_assembly`], which updates every copy.
    ApplyToAssembly {
        /// The assembly.
        assembly: AssetId,
        /// The entity inside it.
        path: Vec<EntityId>,
        /// The component's name.
        component: String,
        /// The field's name.
        field: String,
        /// Its new value.
        value: reflect::Value,
    },
    /// Save the project's input actions (`input.solinput`) with this
    /// text (Edit > Input actions…).
    SaveBindings(String),
    /// Save a recording of a play's input (this text) in the project's
    /// `recordings` folder.
    SaveRecording(String),
    /// Ask which recording to play back (Play > Replay a recording…);
    /// the app reads it and calls [`Workbench::replay`].
    OpenRecording,
    /// Save this save game (game code's `saveGame`) as the slot's file in the
    /// project's `saves` folder.
    SaveGame {
        /// Its slot: the file's name.
        slot: String,
        /// The save's text.
        text: String,
    },
    /// Read the slot's save game from the project's `saves` folder (game
    /// code's `load`), and hand it to [`Workbench::load_game`].
    LoadGame(String),
    /// Ask which save game to load (Play > Load a save…), and hand it to
    /// [`Workbench::load_game`].
    OpenSave,
    /// Check GitHub for a newer build now (Help > Check for updates).
    CheckForUpdates,
    /// Turn checking for updates when starting on or off (Help > Check for
    /// updates when starting); the app saves it.
    SetAutoUpdate(bool),
    /// Download the newer build offered (the update question's Update
    /// now); it installs on Restart to update or when Solis closes.
    DownloadUpdate,
    /// Open the page to download the newer build by hand (the update
    /// question, for a copy that wasn't installed).
    OpenUpdatePage,
    /// The update question was answered Open without updating: skip this
    /// build from now on when it's given (Skip this version).
    SkipUpdate(Option<u64>),
}

/// What [`Workbench::assembly_part`] was worked out for: the selected
/// entity, the area's revision and the sources' count.
pub(crate) type PartKey = (Option<reflect::EntityId>, u64, u64);

/// The editor's state, kept between frames.
pub struct Workbench {
    pub(crate) info: AppInfo,
    pub(crate) dock: DockState<Panel>,
    pub(crate) accent: Accent,
    pub(crate) viewport: Viewport,
    pub(crate) console: Console,
    /// The open area and what's selected in it.
    pub(crate) document: Document,
    /// What the editor knows about the imported models, by asset.
    pub(crate) models: BTreeMap<AssetId, ModelInfo>,
    /// Whether files are being dragged over the window.
    pub(crate) files_hovering: bool,
    pub(crate) about_open: bool,
    /// How long the game's sound took last frame (the Profiler's Sound).
    pub(crate) sound_spent: std::time::Duration,
    /// The Settings window, while it's open (Edit > Settings…).
    pub(crate) settings_window: Option<crate::settings_window::SettingsWindow>,
    /// How updating is going, as the app last said.
    pub(crate) update: crate::updates::UpdateNotice,
    /// Whether updates are checked when Solis starts.
    pub(crate) auto_update: bool,
    /// The update question, while it's asked (Help > Check for updates).
    pub(crate) offer: Option<crate::splash::UpdateOffer>,
    /// Its Skip this version tick.
    pub(crate) skip_offer: bool,
    /// Whether the window is closing to install an update and open again
    /// (Restart to update), until a close is called off.
    pub(crate) restarting: bool,
    /// The tabs that were in front last frame, to underline them in the
    /// accent colour.
    pub(crate) in_front: HashSet<Panel>,
    /// The open project, if any.
    pub(crate) project: Option<ProjectInfo>,
    /// Projects opened lately, newest first, for File > Recent projects.
    pub(crate) recent: Vec<std::path::PathBuf>,
    /// The project's assets, for the Library.
    pub(crate) library: Vec<LibraryItem>,
    /// The project's assemblies the area uses, as read from their files.
    pub(crate) assemblies: BTreeMap<AssetId, std::sync::Arc<area::text::AreaFile>>,
    /// Actions asked for outside [`Self::show`]'s own flow (menus,
    /// Properties), handed to the app with the next frame's.
    pub(crate) outbox: Vec<WorkbenchAction>,
    /// What waits for an answer about unsaved changes: Save, Don't save
    /// or Cancel.
    pub(crate) unsaved_prompt: Option<WorkbenchAction>,
    /// The area as the file now on disk holds it, waiting for Keep mine
    /// or Load the file.
    pub(crate) pending_reload: Option<area::Area>,
    /// Whether the window may close despite unsaved changes (the user
    /// said Don't save).
    pub(crate) may_close: bool,
    /// Counts presses of the mouse: changes to one field during one press
    /// (a drag) are one undo step.
    pub(crate) gesture: u64,
    /// While a gizmo is dragged: what moves, as it was when the drag
    /// began, and the drag's gesture.
    pub(crate) moving: Option<(crate::transforming::Targets, u64)>,
    /// What the Area panel searches for, folds, hides and locks.
    pub(crate) area_panel: crate::area_panel::AreaPanelState,
    /// Components' values copied in Properties, to paste.
    pub(crate) copied: Vec<(String, reflect::ComponentData)>,
    /// Whether the Console has said which program opens Lust files.
    pub(crate) told_how_lust_opens: bool,
    /// The project's folders, for the Library's tree.
    pub(crate) folders: Vec<String>,
    /// How the Library is shown.
    pub(crate) library_state: crate::library::LibraryState,
    /// The project's sound files as the app has measured them, by path
    /// inside the assets.
    pub(crate) sounds: std::collections::HashMap<String, crate::sound_view::SoundState>,
    /// The project's sound events as the app has read them, by path
    /// inside the assets.
    pub(crate) sound_events: std::collections::HashMap<String, crate::sound_view::EventState>,
    /// The sound being listened to, and how far it has got.
    pub(crate) preview: Option<crate::sound_view::Preview>,
    /// The sound file Properties shows (picked in the Library), with the
    /// area's selection as it was then: picking something else in the
    /// area shows that instead.
    pub(crate) sound_shown: Option<(String, crate::document::Selection)>,
    /// The project's buses, mute and solo, and the meters (the Mixer).
    pub(crate) mixer: crate::mixer_view::MixerState,
    /// The project's 2D settings (Settings > 2D, `pixels.solpix`).
    pub(crate) pixels: area::pixels::PixelSettings,
    /// The sound event a sprite's clip event plays, by the event's name
    /// (Phase 9, D3): each sound event by its path without `.solsound`
    /// ("sfx/footstep"), and by its file's name alone ("footstep") when
    /// no other sound event has that name.
    pub(crate) cue_sounds: HashMap<String, String>,
    /// The clips file open in the Clips and Frames tabs (Phase 9, D3).
    pub(crate) frames_editor: Option<crate::frames_editor::FramesEditor>,
    /// Whether the pointer was over the Clips or Frames tab last frame:
    /// Ctrl+Z, Ctrl+Y and Delete are the file's then.
    pub(crate) frames_hovered: bool,
    /// The animator file open in the Animator tab (Phase 9, D3b).
    pub(crate) animator_editor: Option<crate::animator_editor::AnimatorEditor>,
    /// Whether the pointer was over the Animator tab last frame: Ctrl+Z,
    /// Ctrl+Y, Delete and F are the animator's then.
    pub(crate) animator_hovered: bool,
    /// The Map tab's undo steps, the selected map's values shown over it
    /// and its hidden layers (Phase 9, D4).
    pub(crate) map_edits: crate::workbench_tiles::MapEdits,
    /// Whether the pointer was over the Map tab last frame (Ctrl+Z and
    /// Ctrl+Y are the map's then).
    pub(crate) map_hovered: bool,
    /// Each sprite sheet's atlas as an egui picture, by path, for the
    /// Library and Properties, and which sheet each was made from.
    pub(crate) sheet_pictures: HashMap<String, egui::TextureHandle>,
    pub(crate) sheet_pictures_from: HashMap<String, usize>,
    /// Where assets being placed go, when they were dropped somewhere:
    /// the point, and the entity they go under.
    pub(crate) place_targets: BTreeMap<AssetId, (glam::Vec3, Option<EntityId>)>,
    /// Mistakes found in the area, for the Problems panel.
    pub(crate) problems: crate::problems::Problems,
    /// The game, while it runs.
    pub(crate) playing: Option<crate::play::Playing>,
    /// Goes up whenever what Problems checks the area against changes:
    /// the models loaded, the Library, the assemblies, the components
    /// known.
    pub(crate) sources: u64,
    /// Mistakes in the game's code, from the last check.
    pub(crate) code_problems: Vec<script::CodeProblem>,
    /// The game's program, from the last code without mistakes.
    pub(crate) program: Option<std::sync::Arc<script::Compiled>>,
    /// The components it put in the registry, by name.
    pub(crate) game_components: std::collections::BTreeSet<String>,
    /// Where each frame's time goes (the Profiler panel).
    pub(crate) profiler: crate::profiler::Profiler,
    /// What the viewport's [`AreaView`] was last worked out for.
    editor_view: Option<ViewKey>,
    /// The project's input actions (decision 0037), from its
    /// `input.solinput` ([`Self::set_bindings`]); the ready-made ones
    /// without a project.
    pub(crate) bindings: input::Bindings,
    /// What the keyboard, mouse and gamepad are doing, while the game has
    /// them.
    pub(crate) devices: input::Devices,
    /// Whether the game has the mouse and keys (the Game panel was
    /// clicked while playing; Esc gives them back).
    pub(crate) game_input: bool,
    /// The game gave the mouse and keys back: the pointer is shown again
    /// at the next frame.
    pub(crate) release_pointer: bool,
    /// The gamepad as the app last read it (also while editing, for
    /// binding its buttons in the Input actions window).
    pub(crate) pad_now: Option<input::Pad>,
    /// Whether the Input actions window is open.
    pub(crate) input_window: Option<crate::input_window::InputWindow>,
    /// The seed every Play starts the game's random numbers from, if one
    /// was chosen (`seed` in the Console); otherwise each Play has a new
    /// one.
    pub(crate) fixed_seed: Option<u64>,
    /// What the Game panel showed last, and what it was worked out for:
    /// made again only when the world, the models or the state changed
    /// (not on a frame without a tick, or while paused).
    game_scene: Option<(GameKey, crate::game_panel::GameScene)>,
    /// Game code's sound requests since the app last took them.
    pub(crate) game_sounds: Vec<crate::game_sound::GameSound>,
    /// How many times Play has started a game (the app's sound starts
    /// afresh with each).
    pub(crate) plays: u64,
    /// Its components whose names were taken (by the engine's own, or a
    /// Rust one's): they don't run.
    pub(crate) left_out: std::collections::BTreeSet<String>,
    /// Kinds of component written in Rust, run in play mode.
    pub(crate) rust_components: Vec<script::RustKind>,
    /// The last [`Self::assembly_part`], with the selection, revision and
    /// sources it was worked out for: kept, since it expands the whole
    /// assembly.
    pub(crate) assembly_part_kept: std::cell::RefCell<Option<(PartKey, Option<AssemblyPart>)>>,
}

/// What the viewport's view of the area depends on: when it's the same,
/// the view is too.
#[derive(Debug, Clone, PartialEq)]
struct ViewKey {
    revision: u64,
    sources: u64,
    models: usize,
    hidden: HashSet<reflect::EntityId>,
    locked: HashSet<reflect::EntityId>,
}

/// What the Game panel's picture depends on: when it's the same, the
/// picture is too.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GameKey {
    world: u64,
    /// How far between ticks it's drawn (its bits): it moves every frame
    /// while playing.
    between: u32,
    sources: u64,
    models: usize,
    state: crate::play::PlayState,
}

impl Workbench {
    /// A workbench with a saved panel layout, if it is usable (otherwise the
    /// default one), and a new area.
    pub fn new(info: AppInfo, saved_layout: Option<DockState<Panel>>) -> Self {
        Self {
            info,
            dock: layout::restore(saved_layout),
            accent: Accent::default(),
            viewport: Viewport::default(),
            console: Console::default(),
            document: Document::default(),
            models: BTreeMap::new(),
            files_hovering: false,
            about_open: false,
            sound_spent: std::time::Duration::ZERO,
            settings_window: None,
            update: Default::default(),
            auto_update: true,
            offer: None,
            skip_offer: false,
            restarting: false,
            in_front: HashSet::new(),
            gesture: 0,
            moving: None,
            area_panel: Default::default(),
            copied: Vec::new(),
            told_how_lust_opens: false,
            folders: Vec::new(),
            library_state: Default::default(),
            sounds: std::collections::HashMap::new(),
            sound_events: std::collections::HashMap::new(),
            preview: None,
            sound_shown: None,
            mixer: Default::default(),
            pixels: area::pixels::PixelSettings::default(),
            cue_sounds: HashMap::new(),
            frames_editor: None,
            frames_hovered: false,
            animator_editor: None,
            animator_hovered: false,
            map_edits: Default::default(),
            map_hovered: false,
            sheet_pictures: HashMap::new(),
            sheet_pictures_from: HashMap::new(),
            place_targets: BTreeMap::new(),
            problems: Default::default(),
            playing: None,
            sources: 0,
            code_problems: Vec::new(),
            program: None,
            game_components: std::collections::BTreeSet::new(),
            profiler: crate::profiler::Profiler::default(),
            editor_view: None,
            fixed_seed: None,
            bindings: input::Bindings::default(),
            devices: input::Devices::new(),
            game_input: false,
            release_pointer: false,
            pad_now: None,
            input_window: None,
            game_scene: None,
            game_sounds: Vec::new(),
            plays: 0,
            left_out: std::collections::BTreeSet::new(),
            rust_components: Vec::new(),
            assembly_part_kept: std::cell::RefCell::new(None),
            assemblies: BTreeMap::new(),
            outbox: Vec::new(),
            unsaved_prompt: None,
            pending_reload: None,
            may_close: false,
            project: None,
            recent: Vec::new(),
            library: Vec::new(),
        }
    }

    /// Uses `accent` (as saved last time).
    #[must_use]
    pub fn with_accent(mut self, accent: Accent) -> Self {
        self.accent = accent;
        self
    }

    /// Uses `overlays` (as saved last time).
    #[must_use]
    pub fn with_overlays(mut self, overlays: Overlays) -> Self {
        self.viewport.overlays = overlays;
        self
    }

    /// Uses `snapping` (as saved last time), with anything out of range
    /// put back.
    #[must_use]
    pub fn with_snapping(mut self, snapping: crate::Snapping) -> Self {
        self.viewport.snapping = snapping.sanitized();
        self
    }

    /// Snapping, axis locks and world or object axes, for saving.
    pub fn snapping(&self) -> crate::Snapping {
        self.viewport.snapping
    }

    /// Tells the workbench whether files are being dragged over the window,
    /// so it can show where to drop them.
    pub fn set_files_hovering(&mut self, hovering: bool) {
        self.files_hovering = hovering;
    }

    /// What the editor knows about the imported models, by asset.
    pub fn models(&self) -> &BTreeMap<AssetId, ModelInfo> {
        &self.models
    }

    /// The open area and its selection.
    pub fn document(&self) -> &Document {
        &self.document
    }

    /// The entity shown in Properties, if any.
    pub fn selection(&self) -> Option<EntityId> {
        self.document.selection.primary()
    }

    /// Shows `project` as the open one (or none). A different project
    /// starts a new area, as the old one's models belong to the old
    /// project.
    pub fn set_project(&mut self, project: Option<ProjectInfo>) {
        if project != self.project {
            self.replace_document(Document::default());
            self.models.clear();
            self.library.clear();
            self.sources += 1;
            // Nothing of the old project's is used with the new one: its
            // assemblies (a copy of a project shares IDs with it), where
            // things were dropped, a question about its file, the
            // Library's place in its folders, its code's mistakes.
            self.assemblies.clear();
            self.place_targets.clear();
            self.code_problems.clear();
            self.set_program(None);
            self.set_rust_components(Vec::new());
            self.pending_reload = None;
            self.unsaved_prompt = None;
            self.library_state = Default::default();
            if let Some(p) = &project {
                self.log(LogLevel::Info, format!("Opened the project {}.", p.name));
            }
        }
        self.project = project;
    }

    /// The open project, if any.
    pub fn project(&self) -> Option<&ProjectInfo> {
        self.project.as_ref()
    }

    /// The projects for File > Recent projects, newest first.
    pub fn set_recent(&mut self, recent: Vec<std::path::PathBuf>) {
        self.recent = recent;
    }

    /// The project's files, for the Library.
    pub fn set_library(&mut self, items: Vec<LibraryItem>) {
        self.library = items;
        self.cue_sounds = cue_sounds(&self.library);
        self.sources += 1;
    }

    /// The project's folders inside its assets, `/` between names, for
    /// the Library's tree.
    pub fn set_folders(&mut self, folders: Vec<String>) {
        if !folders.contains(&self.library_state.folder) {
            self.library_state.folder.clear();
        }
        self.folders = folders;
    }

    /// Whether the model `id` is loaded (placed or reloaded before).
    pub fn has_model(&self, id: AssetId) -> bool {
        self.models.contains_key(&id)
    }

    /// Swaps in a model that changed on disk (or loads one an opened area
    /// uses): everything using it now draws it. Not an undo step: the area
    /// didn't change.
    pub fn reload_model(&mut self, info: ModelInfo, warnings: &[String]) {
        let (name, triangles) = (info.name.clone(), info.triangles);
        self.sources += 1;
        if self.models.insert(info.asset, info).is_none() {
            // First loaded for an opened area: nothing to announce.
            return;
        }
        self.log(
            LogLevel::Info,
            format!(
                "Reloaded {name}: {} triangles.",
                crate::models::grouped(triangles)
            ),
        );
        for warning in warnings {
            self.log(LogLevel::Warning, format!("{name}: {warning}"));
        }
    }

    /// Starts a new, unsaved area (File > New area). Doesn't ask about
    /// unsaved changes: asking comes before the action.
    pub fn new_area(&mut self) {
        self.replace_document(Document::default());
        self.pending_reload = None;
    }

    /// Shows `area`, read from `file`, in place of the open one.
    pub fn open_area(&mut self, area: area::Area, file: std::path::PathBuf) {
        self.replace_document(Document::opened(area, file));
        self.pending_reload = None;
        self.log(LogLevel::Info, format!("Opened {}.", self.document.name()));
    }

    /// Uses `document` in place of the open one, keeping the components
    /// registered so far (game code registers its own once, not once per
    /// area).
    pub(crate) fn replace_document(&mut self, mut document: Document) {
        document.registry = std::mem::take(&mut self.document.registry);
        self.document = document;
    }

    /// A file or folder was moved from `from` to `to` (full paths), or
    /// deleted (`to` is `None`). If it was the open area's file, or a
    /// folder holding it, the area follows it; returns its new file then
    /// (`None` inside: it was deleted).
    pub fn file_moved(
        &mut self,
        from: &std::path::Path,
        to: Option<&std::path::Path>,
    ) -> Option<Option<std::path::PathBuf>> {
        let file = self.document.file()?.to_path_buf();
        let rest = file.strip_prefix(from).ok()?;
        let moved = to.map(|to| {
            if rest.as_os_str().is_empty() {
                to.to_path_buf()
            } else {
                to.join(rest)
            }
        });
        self.document.file_moved(moved.clone());
        match &moved {
            Some(path) => self.log(
                LogLevel::Info,
                format!("The open area is now {}.", path.display()),
            ),
            None => self.log(
                LogLevel::Warning,
                "The open area's file was deleted; saving asks where to put it.",
            ),
        }
        Some(moved)
    }

    /// Notes that the area was saved to `file`.
    pub fn mark_saved(&mut self, file: std::path::PathBuf) {
        self.document.mark_saved(file);
        self.log(LogLevel::Info, format!("Saved {}.", self.document.name()));
    }

    /// The open area's file changed on disk and now holds `area`: it's
    /// loaded as one undoable step, or, with unsaved changes, the user is
    /// asked which to keep.
    pub fn offer_reload(&mut self, area: area::Area) {
        if let Some(playing) = &mut self.playing {
            // The game's copy isn't the file's; offered again after Stop.
            playing.reload = Some(area);
            return;
        }
        if self.document.is_dirty() {
            self.pending_reload = Some(area);
        } else {
            self.reload_area(area);
        }
    }

    fn reload_area(&mut self, area: area::Area) {
        let file = self
            .document
            .file()
            .and_then(|f| f.file_name())
            .map_or_else(
                || self.document.name(),
                |n| n.to_string_lossy().into_owned(),
            );
        let label = format!("Reloaded {file} from disk");
        match self.document.reload(area, &label) {
            Ok(()) => self.log(LogLevel::Info, format!("{label}.")),
            Err(why) => self.log(LogLevel::Error, format!("Couldn't reload {file}: {why}.")),
        }
    }

    /// The panel layout, for saving.
    pub fn layout(&self) -> &DockState<Panel> {
        &self.dock
    }

    /// The accent colour, for saving.
    pub fn accent(&self) -> Accent {
        self.accent
    }

    /// Uses `camera` (as saved last time), with anything unusable in it
    /// replaced (see [`Camera::sanitized`]).
    #[must_use]
    pub fn with_camera(mut self, camera: Camera) -> Self {
        self.viewport.camera = camera.sanitized();
        self
    }

    /// Uses `look` (as saved last time), with anything out of range put
    /// back in range.
    #[must_use]
    pub fn with_look(mut self, look: crate::look::Look) -> Self {
        self.viewport.look = look.sanitized();
        self
    }

    /// How models are shaded, for saving.
    pub fn look(&self) -> crate::look::Look {
        self.viewport.look
    }

    /// The viewport camera of the view in use.
    pub fn camera(&self) -> Camera {
        self.viewport.camera
    }

    /// Uses `views` (the layout and every view's camera and look, as
    /// saved last time), with anything unusable put back.
    #[must_use]
    pub fn with_views(mut self, views: &crate::viewport::Views) -> Self {
        self.viewport.set_views(views);
        self
    }

    /// The layout and every view's camera and look, for saving.
    pub fn views(&self) -> crate::viewport::Views {
        self.viewport.views()
    }

    /// Which viewport overlays are shown, for saving.
    pub fn overlays(&self) -> Overlays {
        self.viewport.overlays
    }

    /// The chosen tool.
    pub fn tool(&self) -> Tool {
        self.viewport.tool
    }

    /// The console's messages, oldest first.
    pub fn log_lines(&self) -> &[LogLine] {
        self.console.lines()
    }

    /// Adds a message to the console.
    pub fn log(&mut self, level: LogLevel, text: impl Into<String>) {
        self.console.push(level, text.into());
    }

    /// The window title: the program, its version and the open area.
    pub fn title(&self) -> String {
        let mut area = self.document.name();
        if self.document.is_dirty() {
            area.push_str(" •");
        }
        match &self.project {
            Some(project) => format!(
                "{} {} · {} · {area}",
                self.info.name, self.info.version, project.name
            ),
            None => format!("{} {} · {area}", self.info.name, self.info.version),
        }
    }

    /// Whether the editor needs the next frame straight away: the game is
    /// running, the camera flies, something is being dragged or a key or
    /// button is held, or a pick's answer is coming. Otherwise nothing on
    /// screen moves, and the app draws a frame only when something
    /// happens ([decision 0026](../../docs/decisions/0026-draw-only-when-something-changes.md)).
    pub fn wants_frames(&self, ctx: &egui::Context) -> bool {
        self.playing.as_ref().is_some_and(|p| !p.paused)
            || self.frames_editor.as_ref().is_some_and(|f| f.playing)
            || self.moving.is_some()
            || self.viewport.needs_frames()
            || ctx.input(|i| i.pointer.any_down() || !i.keys_down.is_empty())
    }

    /// Draws the editor filling `ui`, and returns what the app must do.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        renderer: &mut dyn ViewportRenderer,
    ) -> Vec<WorkbenchAction> {
        self.profiler.begin();
        self.profiler.add(
            crate::profiler::Stage::Sound,
            std::mem::take(&mut self.sound_spent),
        );
        let mut actions = std::mem::take(&mut self.outbox);
        self.refresh_sheet_pictures(ui.ctx());
        self.viewport.idle = !self.wants_frames(ui.ctx());
        if ui.input(|i| i.pointer.any_pressed()) {
            self.gesture += 1;
        }
        // While the game has the mouse and keys, they're the game's: the
        // editor's own keys are off (decision 0037).
        self.feed_input(ui.ctx());
        if !self.game_input {
            self.tool_keys(ui);
            self.tool_extra_keys(ui);
            self.undo_keys(ui);
            self.editing_keys(ui);
            self.file_keys(ui, &mut actions);
            self.console_key(ui);
        }
        let seconds = ui.input(|i| i.stable_dt);
        self.run_game(seconds);
        egui::Panel::top("top_bar")
            .frame(theme::top_bar_frame())
            .exact_size(34.0)
            .show(ui, |ui| self.top_bar(ui, &mut actions));
        let style = theme::dock_style(ui.style().as_ref(), self.accent);
        self.play_frames(ui.input(|i| f64::from(i.stable_dt)));
        self.play_animator(ui.input(|i| f64::from(i.stable_dt)));
        let sheets = std::sync::Arc::clone(&self.viewport.sheets);
        let mut shown = HashSet::new();
        let mut area_requests = Vec::new();
        let mut library_requests = Vec::new();
        let mut mixer_requests = Vec::new();
        let picked_before = self.library_state.selected.clone();
        self.drop_stale_sound();
        let mut console_requests = Vec::new();
        let part = self.assembly_part();
        let mut requests = Vec::new();
        let preparing = std::time::Instant::now();
        let (hidden, locked) = self.area_panel.in_effect(&self.document);
        // Worked out again only when something it shows changed: while
        // playing, the area usually doesn't.
        let key = ViewKey {
            revision: self.document.revision(),
            sources: self.sources,
            models: self.models.len(),
            hidden: hidden.clone(),
            locked: locked.clone(),
        };
        if self.editor_view.as_ref() != Some(&key) {
            self.viewport.area = AreaView::of_placed(
                self.document.area(),
                &self.document.tree(),
                &self.document.world_matrices(),
                &self.models,
                &hidden,
                &locked,
            );
            self.editor_view = Some(key);
        }
        self.viewport.entities = self.document.area().len();
        self.follow_map_selection();
        self.set_tile_overlay();
        self.prepare_tools();
        self.find_problems();
        let menu = crate::menu_items::MenuState::of(self);
        let copied: Vec<String> = self.copied.iter().map(|(name, _)| name.clone()).collect();
        // Worked out only while the Game panel is in front.
        let state = self.play_state();
        let game = match &mut self.playing {
            Some(playing) if self.in_front.contains(&crate::panel::Panel::Game) => {
                let between = match state {
                    crate::play::PlayState::Playing => playing.world.progress().to_bits(),
                    _ => 1.0_f32.to_bits(),
                };
                let key = GameKey {
                    world: playing.world.changes(),
                    between,
                    sources: self.sources,
                    models: self.models.len(),
                    state,
                };
                match self.game_scene.take() {
                    Some((was, scene)) if was == key => Some((key, scene)),
                    _ => Some((
                        key,
                        crate::game_panel::GameScene::of(&playing.world, &self.models, state),
                    )),
                }
            }
            _ => None,
        };
        self.profiler
            .add(crate::profiler::Stage::Prepare, preparing.elapsed());
        // While playing, the selected entity's game code values as they
        // are now, for Properties (in front) to show instead of the
        // values it started with.
        let running = match (&mut self.playing, self.document.selection.primary()) {
            (Some(playing), Some(id))
                if self.in_front.contains(&crate::panel::Panel::Properties) =>
            {
                let names: Vec<String> = self
                    .document
                    .area()
                    .get(id)
                    .map(|e| e.components.keys().cloned().collect())
                    .unwrap_or_default();
                playing.game.as_mut().map(|game| {
                    names
                        .into_iter()
                        .filter_map(|name| game.values(id, &name).map(|v| (name, v)))
                        .collect::<BTreeMap<String, reflect::ComponentData>>()
                })
            }
            _ => None,
        };
        let playing = self.playing.is_some();
        let mut game_clicked = false;
        let mut renderer = crate::profiler::Timed {
            inner: renderer,
            spent: std::time::Duration::ZERO,
        };
        let mut frames_out = crate::panels::FramesOut::default();
        let live = self.live_animator();
        let mut map_requests = Vec::new();
        let map_shown = self.map_shown();
        let map_name = self
            .map_edits
            .path
            .as_deref()
            .map(crate::sprite_sheets::name_of)
            .unwrap_or_default();
        let map = map_shown
            .as_ref()
            .map(|(map, tileset)| crate::map_tab::MapUi {
                name: map_name.clone(),
                map,
                tileset: tileset.as_deref(),
                hovered: self.viewport.tile_overlay.hovered,
                hidden: &self.map_edits.hidden,
                playing,
            });
        let frames = self.frames_editor.as_mut().map(|editor| {
            crate::workbench_frames::frames_ui(
                editor,
                &sheets,
                &self.sheet_pictures,
                &self.cue_sounds,
            )
        });
        let animator = self.animator_editor.as_mut().map(|editor| {
            crate::workbench_animator::animator_ui(
                editor,
                &sheets,
                &self.sheet_pictures,
                live,
                self.accent.colors().fill,
            )
        });
        theme::dock_frame().show(ui, |ui| {
            DockArea::new(&mut self.dock)
                .style(style)
                .tab_context_menus(false)
                .show_close_buttons(true)
                .show_add_buttons(false)
                .show_leaf_close_all_buttons(false)
                .show_leaf_collapse_buttons(true)
                .show_inside(
                    ui,
                    &mut PanelViewer {
                        area_state: &mut self.area_panel,
                        copied: &copied,
                        menu: &menu,
                        viewport: &mut self.viewport,
                        renderer: &mut renderer,
                        console: &mut self.console,
                        console_requests: &mut console_requests,
                        problems: &self.problems.list,
                        code_problems: &self.code_problems,
                        game: game.as_ref().map(|(_, scene)| scene),
                        playing,
                        game_input: self.game_input,
                        pixels: self.pixels,
                        game_clicked: &mut game_clicked,
                        document: &self.document,
                        models: &self.models,
                        area_requests: &mut area_requests,
                        project: self.project.as_ref(),
                        library: &self.library,
                        folders: &self.folders,
                        library_state: &mut self.library_state,
                        library_requests: &mut library_requests,
                        sounds: crate::library::Sounds {
                            states: &self.sounds,
                            events: &self.sound_events,
                            preview: self.preview.as_ref(),
                            mixer: &self.mixer.mixer,
                            sheets: &sheets,
                            sheet_pictures: &self.sheet_pictures,
                        },
                        mixer: &self.mixer,
                        mixer_requests: &mut mixer_requests,
                        sound_shown: self.sound_shown.as_ref().map(|(path, _)| path.as_str()),
                        part: part.as_ref(),
                        running: running.as_ref(),
                        requests: &mut requests,
                        accent: self.accent,
                        in_front: &self.in_front,
                        shown: &mut shown,
                        profiler: &self.profiler,
                        frames,
                        frames_out: &mut frames_out,
                        animator,
                        map,
                        map_requests: &mut map_requests,
                    },
                );
        });
        self.in_front = shown;
        self.map_hovered = frames_out.map_hovered;
        self.frames_tabs_done(frames_out, &mut actions);
        self.map_requests_done(map_requests);
        self.tile_overlay_done();
        if game_clicked {
            self.give_game_input(ui.ctx());
        }
        self.tool_events();
        for request in requests {
            self.apply_property(request, self.gesture);
        }
        actions.append(&mut self.outbox);
        for request in library_requests {
            self.library_request(request, &mut actions);
        }
        for request in mixer_requests {
            self.mixer_request(request, &mut actions);
        }
        self.follow_sound_selection(&picked_before);
        for request in console_requests {
            self.console_request(request, ui.ctx(), &mut actions);
        }
        for request in area_requests {
            match request {
                crate::area_panel::AreaRequest::Command(command) => self.run(command, ui.ctx()),
                other => self.apply_area_request(other),
            }
        }
        // Kept for the next frame, which may show the same.
        self.game_scene = game;
        if self.files_hovering {
            drop_hint(ui, self.accent);
        }
        self.about(ui);
        self.input_actions_window(ui.ctx());
        self.settings(ui.ctx(), &mut actions);
        self.prompts(ui, &mut actions);
        self.update_question(ui.ctx(), &mut actions);
        self.hold_while_playing(&mut actions);
        self.profiler
            .add(crate::profiler::Stage::Draw, renderer.spent);
        let (components, entities) = match &mut self.playing {
            Some(playing) => (
                playing.game.as_mut().map_or(0, script::Game::len),
                playing.world.len(),
            ),
            None => (0, self.document.area().len()),
        };
        let drawn = renderer.inner.take_report();
        self.profiler.end(drawn, components, entities);
        actions
    }

    /// Q, W, R, S and T pick the tools, unless typing in a field.
    pub(crate) fn tool_keys(&mut self, ui: &Ui) {
        // While flying, W A S D, Q and E move the camera instead.
        if ui.ctx().egui_wants_keyboard_input() || self.viewport.is_flying() {
            return;
        }
        let pressed = ui.input(|i| {
            Tool::ALL
                .into_iter()
                .find(|tool| i.key_pressed(tool.key()) && i.modifiers.is_none())
        });
        if let Some(tool) = pressed {
            self.viewport.tool = tool;
        }
    }

    /// The `'` key (beside Æ on a Norwegian keyboard) jumps to the
    /// console's command line, bringing the Console to the front (decided
    /// with the project lead, 2026-10-23). Not while typing.
    pub(crate) fn console_key(&mut self, ui: &Ui) {
        if ui.ctx().egui_wants_keyboard_input() || self.viewport.is_flying() {
            return;
        }
        let pressed = ui.input_mut(|i| {
            let pressed = i.events.iter().any(|e| {
                matches!(
                    e,
                    egui::Event::Key {
                        key: egui::Key::Quote,
                        pressed: true,
                        modifiers,
                        ..
                    } if !modifiers.shift && !modifiers.command && !modifiers.alt
                )
            });
            // The ' itself isn't typed into the command line: dropped now,
            // or, if it comes a frame later, by the command line.
            let typed = pressed
                && i.events
                    .iter()
                    .any(|e| matches!(e, egui::Event::Text(t) if t == "'"));
            if typed {
                i.events
                    .retain(|e| !matches!(e, egui::Event::Text(t) if t == "'"));
            }
            (pressed, pressed && !typed)
        });
        let (pressed, later) = pressed;
        if pressed {
            self.open_command_line();
            self.console.swallow_quote = later;
        }
    }

    /// Brings the Console to the front and puts the keyboard in its
    /// command line.
    pub(crate) fn open_command_line(&mut self) {
        if let Some(found) = self.dock.find_tab(&crate::panel::Panel::Console) {
            let _ = self.dock.set_active_tab(found);
        }
        self.console.focus = true;
    }

    /// Commands typed in the console, and Tab.
    pub(crate) fn console_request(
        &mut self,
        request: crate::console::ConsoleRequest,
        ctx: &egui::Context,
        actions: &mut Vec<WorkbenchAction>,
    ) {
        match request {
            crate::console::ConsoleRequest::Run(line) => self.run_command(&line, ctx, actions),
            crate::console::ConsoleRequest::Complete => {
                let done = self.complete(&self.console.input);
                self.console.input = done.line;
                self.console.choices = done.choices;
                self.console.cursor_to_end = true;
                self.console.focus = true;
            }
        }
    }

    /// Uses `history` as the console's earlier commands (as saved last
    /// time).
    #[must_use]
    pub fn with_console_history(mut self, history: Vec<String>) -> Self {
        self.console.set_history(history);
        self
    }

    /// Looks for problems in the area again, if anything they depend on
    /// changed.
    pub(crate) fn find_problems(&mut self) {
        use crate::problems::{Fingerprint, Sources};
        let print = Fingerprint::of(&self.document, self.sources);
        self.problems.refresh_between(
            self.document.revision(),
            self.pixels.pixels_per_unit,
            self.document.area(),
            &self.viewport.area.sprites,
        );
        if self.problems.is_from(&print) {
            return;
        }
        let known: HashSet<AssetId> = self.library.iter().filter_map(|i| i.id).collect();
        let assemblies: HashSet<AssetId> = self.assemblies.keys().copied().collect();
        self.problems.refresh(
            print,
            &Sources {
                area: self.document.area(),
                registry: &self.document.registry,
                models: &self.models,
                known: &known,
                assemblies: &assemblies,
            },
        );
    }

    /// Takes in the result of checking the game's code (the app checks it
    /// on a worker when a project opens and when a code file changes): its
    /// mistakes go to the Problems panel, and the Console says how it went.
    /// `notes` are files left out (too big, not text).
    pub fn set_code_problems(&mut self, problems: Vec<script::CodeProblem>, notes: &[String]) {
        for note in notes {
            self.log(LogLevel::Warning, note.clone());
        }
        let text = match problems.len() {
            0 if !self.code_problems.is_empty() => Some((
                LogLevel::Info,
                "The game's code has no mistakes now.".to_string(),
            )),
            0 => None,
            1 => Some((
                LogLevel::Warning,
                format!("A mistake in the game's code: {}", problems[0]),
            )),
            n => Some((
                LogLevel::Warning,
                format!("{n} mistakes in the game's code; they're listed in Problems."),
            )),
        };
        if let Some((level, text)) = text {
            self.log(level, text);
        }
        self.code_problems = problems;
    }

    /// Mistakes in the game's code, from the last check.
    pub fn code_problems(&self) -> &[script::CodeProblem] {
        &self.code_problems
    }

    /// The problems found in the area, for the app and tests.
    pub fn problems(&self) -> &[crate::problems::Problem] {
        &self.problems.list
    }

    /// The console's earlier commands, oldest first, for saving.
    pub fn console_history(&self) -> &[String] {
        self.console.history()
    }

    /// Ctrl+Z undoes; Ctrl+Y or Ctrl+Shift+Z redoes. Not while typing in a
    /// field, where they undo the typing instead.
    pub(crate) fn undo_keys(&mut self, ui: &Ui) {
        if ui.ctx().egui_wants_keyboard_input() {
            return;
        }
        let frames = self.frames_hovered && self.frames_editor.is_some();
        let (redo, undo) = ui.input_mut(|i| {
            let shift_z = egui::KeyboardShortcut::new(
                egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                egui::Key::Z,
            );
            let y = egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::Y);
            let z = egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::Z);
            // Shift+Z first: a plain Ctrl+Z shortcut would take it too.
            let redo = i.consume_shortcut(&shift_z) || i.consume_shortcut(&y);
            (redo, i.consume_shortcut(&z))
        });
        let animator = self.animator_hovered && self.animator_editor.is_some();
        if frames {
            // Over the Clips or Frames tab: the clips file's changes.
            self.frames_undo(redo, undo);
        } else if animator {
            // Over the Animator tab: the animator's changes.
            self.animator_undo(redo, undo);
        } else if self.map_hovered && self.map_edits.path.is_some() {
            // Over the Map tab: the map's changes.
            self.map_undo(redo, undo);
        } else if redo {
            self.redo();
        } else if undo {
            self.undo();
        }
    }

    /// Ctrl+N, Ctrl+O, Ctrl+S and Ctrl+Shift+S: new, open, save and save
    /// as (decided with the project lead, 2026-10-23). Not while typing.
    pub(crate) fn file_keys(&mut self, ui: &Ui, actions: &mut Vec<WorkbenchAction>) {
        if ui.ctx().egui_wants_keyboard_input() {
            return;
        }
        let command = egui::Modifiers::COMMAND;
        let pressed = ui.input_mut(|i| {
            // Shift+S first: a plain Ctrl+S shortcut would take it too.
            let save_as =
                egui::KeyboardShortcut::new(command | egui::Modifiers::SHIFT, egui::Key::S);
            [
                (save_as, WorkbenchAction::SaveAreaAs),
                (
                    egui::KeyboardShortcut::new(command, egui::Key::S),
                    WorkbenchAction::SaveArea,
                ),
                (
                    egui::KeyboardShortcut::new(command, egui::Key::N),
                    WorkbenchAction::NewArea,
                ),
                (
                    egui::KeyboardShortcut::new(command, egui::Key::O),
                    WorkbenchAction::OpenArea,
                ),
            ]
            .into_iter()
            .find(|(keys, _)| i.consume_shortcut(keys))
            .map(|(_, action)| action)
        });
        if let Some(action) = pressed {
            self.request(action, actions);
        }
    }

    /// Asks for `action`, first asking about unsaved changes if it would
    /// lose them (a new or another area, another project, quitting).
    pub(crate) fn request(&mut self, action: WorkbenchAction, actions: &mut Vec<WorkbenchAction>) {
        if self.playing.is_some() && crate::play::waits_for_stop(&action) {
            // Refused, with a word in the Console, at the end of the frame.
            actions.push(action);
            return;
        }
        if self.playing.is_some() && action == WorkbenchAction::Quit {
            self.stop();
        }
        let loses_changes = matches!(
            action,
            WorkbenchAction::NewArea
                | WorkbenchAction::OpenArea
                | WorkbenchAction::OpenAreaFile(_)
                | WorkbenchAction::OpenProject
                | WorkbenchAction::NewProject
                | WorkbenchAction::OpenRecent(_)
                | WorkbenchAction::Quit
        );
        if loses_changes && self.document.is_dirty() {
            self.unsaved_prompt = Some(action);
        } else {
            if action == WorkbenchAction::Quit {
                self.may_close = true;
            }
            actions.push(action);
        }
    }

    /// The app called off a queued quit (the save before it was cancelled
    /// or failed): a later close asks about unsaved changes again.
    pub fn quit_cancelled(&mut self) {
        self.may_close = false;
    }

    /// Whether the window may close now. If the area has unsaved changes,
    /// asks first (and the app should cancel this close).
    pub fn close_requested(&mut self) -> bool {
        // While playing, the area that matters is the one from before Play.
        let dirty = match &self.playing {
            Some(playing) => playing.before.is_dirty(),
            None => self.document.is_dirty(),
        };
        if self.may_close || !dirty {
            if self.playing.is_some() {
                self.stop();
            }
            return true;
        }
        // Asked first: Cancel leaves the game running.
        self.unsaved_prompt = Some(WorkbenchAction::Quit);
        false
    }

    /// The questions about unsaved changes and files changed on disk.
    fn prompts(&mut self, ui: &mut Ui, actions: &mut Vec<WorkbenchAction>) {
        if let Some(waiting) = self.unsaved_prompt.clone() {
            let name = self.document.name();
            let mut answer = None;
            egui::Window::new("Unsaved changes")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ui.ctx(), |ui| {
                    ui.label(format!("{name} has changes that aren't saved."));
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {
                            answer = Some(true);
                        }
                        if ui.button("Don't save").clicked() {
                            answer = Some(false);
                        }
                        if ui.button("Cancel").clicked() {
                            self.unsaved_prompt = None;
                            // A cancelled restart is no restart.
                            self.restarting = false;
                        }
                    });
                });
            if let Some(save) = answer {
                self.unsaved_prompt = None;
                // Saving or leaving waits for Stop, which puts the area
                // back as it was before Play.
                if self.playing.is_some() {
                    self.stop();
                }
                if save {
                    actions.push(WorkbenchAction::SaveArea);
                }
                if waiting == WorkbenchAction::Quit {
                    self.may_close = true;
                }
                actions.push(waiting);
            }
        }
        // Asked after the game stops: loading it now would only be undone.
        if self.pending_reload.is_some() && self.playing.is_none() {
            let name = self.document.name();
            let mut answer = None;
            egui::Window::new("Changed on disk")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ui.ctx(), |ui| {
                    ui.label(format!(
                        "{name}'s file changed on disk (perhaps a Git pull), and you have \
                         changes that aren't saved."
                    ));
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("Keep mine").clicked() {
                            answer = Some(false);
                        }
                        if ui
                            .button("Load the file")
                            .on_hover_text("Ctrl+Z brings yours back")
                            .clicked()
                        {
                            answer = Some(true);
                        }
                    });
                });
            if let Some(load) = answer
                && let Some(area) = self.pending_reload.take()
                && load
            {
                self.reload_area(area);
            }
        }
    }

    /// Delete deletes the selection, Ctrl+D duplicates it, F2 renames it
    /// and F frames it (decided with the project lead, 2026-10-23). Not
    /// while typing, nor while flying.
    pub(crate) fn editing_keys(&mut self, ui: &Ui) {
        if ui.ctx().egui_wants_keyboard_input() || self.viewport.is_flying() {
            return;
        }
        let (delete, duplicate, rename, frame) = ui.input_mut(|i| {
            let plain = |i: &egui::InputState, key| i.key_pressed(key) && i.modifiers.is_none();
            (
                plain(i, egui::Key::Delete),
                i.consume_shortcut(&egui::KeyboardShortcut::new(
                    egui::Modifiers::COMMAND,
                    egui::Key::D,
                )),
                plain(i, egui::Key::F2),
                plain(i, egui::Key::F),
            )
        });
        // Over the Frames tab, Delete takes out frames instead.
        if delete && !self.frames_hovered && !self.animator_hovered {
            self.delete_selection();
        }
        if duplicate {
            self.duplicate_selection();
        }
        if rename
            && let Some(id) = self.document.selection.primary()
            && let Some(entity) = self.document.area().get(id)
        {
            crate::area_panel::begin_rename(ui.ctx(), id, &entity.name);
        }
        if frame && !self.animator_hovered {
            self.frame_selection();
        }
    }

    fn about(&mut self, ui: &mut Ui) {
        let mut open = self.about_open;
        egui::Window::new(format!("About {}", self.info.name))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ui.ctx(), |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading(&self.info.name);
                    ui.label(format!("Version {}", self.info.version));
                    ui.add_space(8.0);
                    ui.label("A free, open-source 3D game engine.");
                    ui.label("Free software under the MIT licence.");
                });
            });
        self.about_open = open;
    }
}

/// The sound events clip events can play, by name: each by its path
/// without `.solsound`, and by its file's name alone unless two share it.
fn cue_sounds(library: &[LibraryItem]) -> HashMap<String, String> {
    let mut by_name: HashMap<String, String> = HashMap::new();
    let mut shared = std::collections::HashSet::new();
    let mut map = HashMap::new();
    for item in library.iter().filter(|i| i.kind == LibraryKind::SoundEvent) {
        let path = item
            .path
            .strip_suffix(".solsound")
            .unwrap_or(&item.path)
            .to_owned();
        if by_name.insert(item.name.clone(), path.clone()).is_some() {
            shared.insert(item.name.clone());
        }
        map.insert(path.clone(), path);
    }
    for (name, path) in by_name {
        if !shared.contains(&name) {
            map.entry(name).or_insert(path);
        }
    }
    map
}

#[cfg(test)]
#[path = "workbench_tests.rs"]
mod tests;
