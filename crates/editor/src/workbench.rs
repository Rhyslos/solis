//! The whole editor window, inside the frame of
//! [decision 0099](../../../docs/decisions/0099-the-g-layout.md)
//! (`shell/`): the tab bar and ribbon, the editor groups and the shelf
//! that hold the panels, and the status bar.
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
use crate::key_owner::KeyOwner;
use crate::library::{LibraryItem, LibraryKind, ProjectInfo};
use crate::made_from::MadeFrom;
use crate::models::ModelInfo;
use crate::panel::Panel;
use crate::panels::{PanelViewer, drop_hint};
use crate::project_state::ProjectState;
use crate::theme::{self, Accent};
use crate::viewport::{Overlays, Tool, Viewport, ViewportRenderer};
use egui::Ui;
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
    /// Export the game for playtesting (decision 0093).
    ExportPlaytest(crate::export_dialog::ExportRequest),
    /// Choose the folder Build > Export exports into.
    ChooseExportFolder,
    /// Ask which model file to import (Library > Import).
    ImportModel,
    /// Bring in the built-in sample area (Add the sample area, in the search).
    AddSampleArea,
    /// Make the turret sample game as a new project, in a folder the app
    /// asks for (Make the sample game, on File > Learn).
    MakeSampleGame,
    /// Make the UI sample…, on File > Learn: a folder to make it in, then it opens.
    MakeUiSample,
    /// Make the benchmark as a new project, in a folder the app asks for
    /// (Make the benchmark, on File > Learn).
    MakeBenchmark,
    /// Make the playground (a level for the character, the cameras,
    /// physics and saves) as a new project, in a folder the app asks for
    /// (Make the playground, on File > Learn).
    MakePlayground,
    /// Make a new project in a folder the user picks (File > New project).
    NewProject,
    /// Open a project from a folder the user picks (File > Open…).
    OpenProject,
    /// Open the project in this folder (File > Recent projects, the
    /// launcher).
    OpenRecent(std::path::PathBuf),
    /// Read the projects Solis knows afresh, for the launcher.
    ListProjects,
    /// Pin (true) or unpin a project in the launcher's list.
    PinProject(std::path::PathBuf, bool),
    /// Forget a project in the launcher's list; its files stay.
    ForgetProject(std::path::PathBuf),
    /// Show a project's folder in Explorer (the launcher's ⋯).
    ShowProject(std::path::PathBuf),
    /// Give the project in this folder a new name (its folder keeps its
    /// name).
    RenameProject(std::path::PathBuf, String),
    /// Ask for an empty folder and copy the project in this folder there.
    CopyProject(std::path::PathBuf),
    /// Ask where the project known in this folder is now (its folder was
    /// moved).
    LocateProject(std::path::PathBuf),
    /// Ask for a folder and look through it for projects to add to the
    /// list.
    ScanForProjects,
    /// Place the project's asset where the camera looks (a double-click in
    /// the Library).
    PlaceAsset(AssetId),
    /// Start a new area (New area (in the search), Ctrl+N): the app calls
    /// [`Workbench::new_area`].
    NewArea,
    /// Ask which area file to open (Open area… (in the search), Ctrl+O).
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
    /// Open this file of the project's assets in the program the computer
    /// has for it, or show this folder of them in the file browser. The
    /// app opens a file only if its kind is one that's safe to
    /// ([`LibraryKind::opens_in_its_program`](crate::LibraryKind::opens_in_its_program)),
    /// showing its folder if not, and refuses a path outside the assets.
    OpenExternally(std::path::PathBuf),
    /// Show this folder, one the user chose on this computer (an export's),
    /// in the file browser. Never opens a file.
    ShowFolder(std::path::PathBuf),
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
    /// Make a new text file inside the assets: a Lust code file or an area
    /// (E1). Never replaces a file.
    NewTextFile {
        /// The file, inside the assets.
        path: std::path::PathBuf,
        /// What it holds.
        text: String,
    },
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
    /// Save a file of one of the kinds the editor changes itself (a clips
    /// file, an animator, a tileset, an atlas, a tile map; inside the
    /// project's assets), making it if it isn't there yet. The app
    /// answers every one, in order: [`Workbench::file_saved`] once it's
    /// written (and it keeps `file` as the file from then on), or
    /// [`Workbench::file_not_saved`] if it couldn't be, or wasn't tried.
    SaveFile {
        /// The file.
        path: std::path::PathBuf,
        /// What it holds.
        file: crate::OwnFile,
        /// The same, as the text to write.
        text: String,
    },
    /// Save a screen's Lust file (inside the project's assets) after a
    /// change on the UI editor's canvas (decision 0089). It must be there.
    SaveScreen {
        /// The file.
        path: std::path::PathBuf,
        /// What to save.
        text: String,
    },
    /// Save a new picture (a PNG inside the project's assets): a tileset
    /// kind's template to draw over (Phase 9, D5). Never replaces a file.
    SavePicture {
        /// The file.
        path: std::path::PathBuf,
        /// The PNG's bytes.
        bytes: Vec<u8>,
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
    /// text (File > Input).
    SaveBindings(String),
    /// Save a recording of a play's input (this text) in the project's
    /// `recordings` folder.
    SaveRecording(String),
    /// Ask which recording to play back (Debug > Replay a recording…);
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
    /// Ask which save game to load (Debug > Load save), and hand it to
    /// [`Workbench::load_game`].
    OpenSave,
    /// Check GitHub for a newer build now (File > Updates).
    CheckForUpdates,
    /// Turn checking for updates when starting on or off (File > Updates when starting); the app saves it.
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
    /// Everything that belongs to the open project (or to working without
    /// one): replaced whole when another project opens, so nothing of one
    /// project is used with another. What is declared below is the
    /// program's and the user's, and stays.
    pub(crate) open: ProjectState,
    pub(crate) accent: Accent,
    /// The interface's scale, density and switches (decision 0100). Its
    /// accent is `accent` above, which the drawing code reads; the two
    /// are set together and read together (`interface_look`).
    pub(crate) interface: theme::InterfaceLook,
    pub(crate) viewport: Viewport,
    pub(crate) console: Console,
    /// Whether files are being dragged over the window.
    pub(crate) files_hovering: bool,
    pub(crate) about_open: bool,
    /// The launcher (decision 0086).
    pub(crate) launcher: crate::launcher::Launcher,
    /// The projects Solis knows, for the launcher (the app reads them).
    pub(crate) known_projects: Vec<crate::launcher::KnownProject>,
    /// This copy's build number, for the launcher's marks.
    pub(crate) this_build: Option<u64>,
    /// Where the manual is, for the launcher's Learn page.
    pub(crate) manual_url: String,
    /// How long the game's sound took last frame (the Profiler's Sound).
    pub(crate) sound_spent: std::time::Duration,
    /// The Settings window, while it's open (File > Preferences).
    pub(crate) settings_window: Option<crate::settings_window::SettingsWindow>,
    /// The Documents folder, for exports to go in by default.
    pub(crate) documents: Option<std::path::PathBuf>,
    /// How updating is going, as the app last said.
    pub(crate) update: crate::updates::UpdateNotice,
    /// What the newer build found brings (the launcher's Updates page).
    pub(crate) release_notes: Option<crate::updates::ReleaseNotes>,
    /// Whether updates are checked when Solis starts.
    pub(crate) auto_update: bool,
    /// The update question, while it's asked (File > Updates).
    pub(crate) offer: Option<crate::splash::UpdateOffer>,
    /// Its Skip this version tick.
    pub(crate) skip_offer: bool,
    /// Whether the window is closing to install an update and open again
    /// (Restart to update), until a close is called off.
    pub(crate) restarting: bool,
    /// The tabs that were in front last frame, to underline them in the
    /// accent colour.
    pub(crate) in_front: HashSet<Panel>,
    /// An exported game (decision 0093): only the game shows, filling the
    /// window, and the window follows the player's settings.
    pub(crate) play_only: bool,
    /// Whether the window may close despite unsaved changes (the user
    /// said Don't save).
    pub(crate) may_close: bool,
    /// Counts presses of the mouse: changes to one field during one press
    /// (a drag) are one undo step.
    pub(crate) gesture: u64,
    /// Whether the Console has said which program opens Lust files.
    pub(crate) told_how_lust_opens: bool,
    /// Whether the Hints panel shows hints (Settings > Appearance).
    pub(crate) hints: bool,
    /// One search for everything (decision 0085).
    pub(crate) search: crate::search::SearchState,
    /// The frame: the tab in front, the ribbon, each tab's groups and
    /// shelf, the File page (decision 0099).
    pub(crate) shell: crate::shell::Shell,
    /// The program's logo, for the top bar (the app hands it over).
    pub(crate) logo: Option<egui::TextureId>,
    /// The panel the pointer rested over when the last frame was
    /// drawn, if any: what [`Workbench::key_owner`] works the keys' owner
    /// out from.
    pub(crate) pointer_over: Option<Panel>,
    /// Who the keys belong to this frame (`key_owner.rs`): every key
    /// handler asks this, and none decides for itself.
    pub(crate) keys: KeyOwner,
    /// Goes up whenever what Problems checks the area against changes:
    /// the models loaded, the Library, the assemblies, the components
    /// known. It only ever goes up, another project included, so nothing
    /// worked out for one count is taken for a later one's.
    pub(crate) sources: u64,
    /// Where each frame's time goes (the Profiler panel).
    pub(crate) profiler: crate::profiler::Profiler,
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
    /// A test screen the Game panel shows over the game (the Console's
    /// `ui test`, decision 0083), until screens are made in Lust.
    pub(crate) ui_test: Option<std::sync::Arc<ui::Screen>>,
    /// The player's settings (decision 0091): text size, UI size and the
    /// window, as game code last set them; kept from play to play. They
    /// are the player's, not the project's: an exported game is handed
    /// them before its project opens.
    pub(crate) player_settings: script::settings::PlayerSettings,
    /// The fonts the canvas measures with.
    pub(crate) screen_fonts: Option<ui::Fonts>,
    /// How many times Play has started a game (the app's sound starts
    /// afresh with each). It only ever goes up, another project included.
    pub(crate) plays: u64,
}

/// What the viewport's view of the area depends on: when it's the same,
/// the view is too.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ViewKey {
    revision: u64,
    sources: u64,
    models: usize,
    hidden: HashSet<reflect::EntityId>,
    locked: HashSet<reflect::EntityId>,
}

/// What the Game panel's picture depends on: when it's the same, the
/// picture is too.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GameKey {
    world: u64,
    /// How far between ticks it's drawn (its bits): it moves every frame
    /// while playing.
    between: u32,
    sources: u64,
    models: usize,
    state: crate::play::PlayState,
}

impl Workbench {
    /// A workbench with a new area, its frame as the design draws it.
    pub fn new(info: AppInfo) -> Self {
        Self {
            info,
            open: ProjectState::default(),
            accent: Accent::default(),
            interface: theme::InterfaceLook::default(),
            viewport: Viewport::default(),
            console: Console::default(),
            files_hovering: false,
            about_open: false,
            launcher: Default::default(),
            known_projects: Vec::new(),
            this_build: None,
            manual_url: String::new(),
            sound_spent: std::time::Duration::ZERO,
            settings_window: None,
            documents: None,
            update: Default::default(),
            release_notes: None,
            auto_update: true,
            offer: None,
            skip_offer: false,
            restarting: false,
            in_front: HashSet::new(),
            gesture: 0,
            told_how_lust_opens: false,
            hints: true,
            search: crate::search::SearchState::default(),
            shell: crate::shell::Shell::default(),
            logo: None,
            pointer_over: None,
            keys: KeyOwner::default(),
            sources: 0,
            profiler: crate::profiler::Profiler::default(),
            ui_test: None,
            player_settings: Default::default(),
            screen_fonts: None,
            devices: input::Devices::new(),
            game_input: false,
            release_pointer: false,
            pad_now: None,
            plays: 0,
            play_only: false,
            may_close: false,
        }
    }

    /// Uses `look` (as saved last time): its accent, scale and density.
    #[must_use]
    pub fn with_interface_look(mut self, look: theme::InterfaceLook) -> Self {
        self.accent = look.accent;
        self.interface = look;
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
        &self.open.models
    }

    /// The open area and its selection.
    pub fn document(&self) -> &Document {
        &self.open.document
    }

    /// The entity shown in Properties, if any.
    pub fn selection(&self) -> Option<EntityId> {
        self.open.document.selection.primary()
    }

    /// The open project, if any.
    pub fn project(&self) -> Option<&ProjectInfo> {
        self.open.project.as_ref()
    }

    /// The project's files, for the Library.
    pub fn set_library(&mut self, items: Vec<LibraryItem>) {
        self.open.library = items;
        self.open.cue_sounds = cue_sounds(&self.open.library);
        self.sources += 1;
    }

    /// The project's folders inside its assets, `/` between names, for
    /// the Library's tree.
    pub fn set_folders(&mut self, folders: Vec<String>) {
        if !folders.contains(&self.open.library_state.folder) {
            self.open.library_state.folder.clear();
        }
        self.open.folders = folders;
    }

    /// Whether the model `id` is loaded (placed or reloaded before).
    pub fn has_model(&self, id: AssetId) -> bool {
        self.open.models.contains_key(&id)
    }

    /// Swaps in a model that changed on disk (or loads one an opened area
    /// uses): everything using it now draws it. Not an undo step: the area
    /// didn't change.
    pub fn reload_model(&mut self, info: ModelInfo, warnings: &[String]) {
        let (name, triangles) = (info.name.clone(), info.triangles);
        self.sources += 1;
        if self.open.models.insert(info.asset, info).is_none() {
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

    /// Starts a new, unsaved area (New area (in the search)). Doesn't ask about
    /// unsaved changes: asking comes before the action.
    pub fn new_area(&mut self) {
        self.leave_for(Document::default());
    }

    /// Shows `area`, read from `file`, in place of the open one.
    pub fn open_area(&mut self, area: area::Area, file: std::path::PathBuf) {
        self.leave_for(Document::opened(area, file));
        self.log(
            LogLevel::Info,
            format!("Opened {}.", self.open.document.name()),
        );
    }

    /// Leaves the open area for `document`. The questions about the old
    /// one go with it: answering them would put it back over the new one.
    /// Asking comes before this ([`Self::request`]).
    fn leave_for(&mut self, document: Document) {
        self.replace_document(document);
        self.open.pending_reload = None;
        self.open.play_changes = None;
    }

    /// Uses `document` in place of the open one, keeping the components
    /// registered so far (game code registers its own once, not once per
    /// area).
    pub(crate) fn replace_document(&mut self, mut document: Document) {
        document.registry = std::mem::take(&mut self.open.document.registry);
        self.open.document = document;
    }

    /// A file or folder was moved from `from` to `to` (full paths), or
    /// deleted (`to` is `None`). If it was the open area's file, or a
    /// folder holding it, the area follows it; returns its new file then
    /// (`None` inside: it was deleted). While playing, and while Keep or
    /// Undo all is asked, the area that Stop or Keep would put back
    /// follows too, so the move isn't undone by them.
    pub fn file_moved(
        &mut self,
        from: &std::path::Path,
        to: Option<&std::path::Path>,
    ) -> Option<Option<std::path::PathBuf>> {
        let file = self.open.document.file()?.to_path_buf();
        let rest = file.strip_prefix(from).ok()?;
        let moved = to.map(|to| {
            if rest.as_os_str().is_empty() {
                to.to_path_buf()
            } else {
                to.join(rest)
            }
        });
        self.open.document.file_moved(moved.clone());
        // Every other copy of the document that can become the open one
        // again is of the same file: the one kept at Play, which Stop
        // puts back, and the one Keep would put in its place. Left where
        // they were, the next save would make the old file again beside
        // the moved one, or a deleted file's area would count as saved.
        if let Some(playing) = &mut self.open.playing {
            playing.before.file_moved(moved.clone());
        }
        if let Some(changes) = &mut self.open.play_changes {
            changes.document.file_moved(moved.clone());
        }
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
        self.open.document.mark_saved(file);
        self.log(
            LogLevel::Info,
            format!("Saved {}.", self.open.document.name()),
        );
    }

    /// The open area's file changed on disk and now holds `area`: it's
    /// loaded as one undoable step, or, with unsaved changes, the user is
    /// asked which to keep.
    pub fn offer_reload(&mut self, area: area::Area) {
        if let Some(playing) = &mut self.open.playing {
            // The game's copy isn't the file's; offered again after Stop.
            playing.reload = Some(area);
            return;
        }
        if let Some(changes) = &mut self.open.play_changes {
            // Keep would put the area from before this change over it:
            // offered again once Keep or Undo all is answered.
            changes.reload = Some(area);
            return;
        }
        if self.open.document.is_dirty() {
            self.open.pending_reload = Some(area);
        } else {
            self.reload_area(area);
        }
    }

    fn reload_area(&mut self, area: area::Area) {
        let file = self
            .open
            .document
            .file()
            .and_then(|f| f.file_name())
            .map_or_else(
                || self.open.document.name(),
                |n| n.to_string_lossy().into_owned(),
            );
        let label = format!("Reloaded {file} from disk");
        match self.open.document.reload(area, &label) {
            Ok(()) => self.log(LogLevel::Info, format!("{label}.")),
            Err(why) => self.log(LogLevel::Error, format!("Couldn't reload {file}: {why}.")),
        }
    }

    /// The accent colour, for saving.
    pub fn accent(&self) -> Accent {
        self.accent
    }

    /// The interface's look (accent, scale, density), for saving.
    pub fn interface_look(&self) -> theme::InterfaceLook {
        theme::InterfaceLook {
            accent: self.accent,
            ..self.interface
        }
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

    /// The messages added since `seen` messages had been (counting every
    /// one ever added, so old ones dropped from the Console don't shift
    /// it), and the count to pass next time.
    pub fn log_since(&self, seen: u64) -> (&[LogLine], u64) {
        let pushed = self.console.pushed();
        let lines = self.console.lines();
        let new = usize::try_from(pushed.saturating_sub(seen))
            .unwrap_or(usize::MAX)
            .min(lines.len());
        (&lines[lines.len() - new..], pushed)
    }

    /// Adds a message to the console.
    pub fn log(&mut self, level: LogLevel, text: impl Into<String>) {
        self.console.push(level, text.into());
    }

    /// The window title: the program, its version and the open area.
    pub fn title(&self) -> String {
        let mut area = self.open.document.name();
        if self.open.document.is_dirty() {
            area.push_str(" •");
        }
        match &self.open.project {
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
        self.open.playing.as_ref().is_some_and(|p| !p.paused)
            || self.open.frames_editor.as_ref().is_some_and(|f| f.playing)
            || self.open.moving.is_some()
            || self.viewport.needs_frames()
            || ctx.input(|i| i.pointer.any_down() || !i.keys_down.is_empty())
    }

    /// What the game looks like now, while playing: last frame's when
    /// nothing it shows changed.
    pub(crate) fn game_scene_now(&mut self) -> Option<(GameKey, crate::game_panel::GameScene)> {
        let state = self.play_state();
        let playing = self.open.playing.as_mut()?;
        let between = match state {
            crate::play::PlayState::Playing => playing.world.progress().to_bits(),
            _ => 1.0_f32.to_bits(),
        };
        let key = GameKey {
            world: playing.world.changes(),
            between,
            sources: self.sources,
            models: self.open.models.len(),
            state,
        };
        Some(match self.open.game_scene.take() {
            Some((was, scene)) if was == key => (key, scene),
            _ => (key, {
                let mut scene =
                    crate::game_panel::GameScene::of(&playing.world, &self.open.models, state);
                scene.camera_start =
                    crate::game_panel::GameScene::camera_start_in(playing.before.area());
                scene
            }),
        })
    }

    /// The Environment's Preview time (decision 0092), set in Properties:
    /// the hour the viewport shows the area at. It's forgotten with its
    /// project, and never used while playing, when the viewport shows the
    /// game's own hour.
    pub(crate) fn preview_hour(&mut self, ctx: &egui::Context) -> Option<f64> {
        if !std::mem::replace(&mut self.open.preview_forgotten, true) {
            ctx.data_mut(|d| d.remove::<Option<f64>>(crate::day_colours::preview_id()));
        }
        if self.open.playing.is_some() {
            return None;
        }
        ctx.data(|d| d.get_temp::<Option<f64>>(crate::day_colours::preview_id()))
            .flatten()
    }

    /// Draws the editor filling `ui`, and returns what the app must do.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        renderer: &mut dyn ViewportRenderer,
    ) -> Vec<WorkbenchAction> {
        self.profiler.begin();
        // The look the editor holds is the one in use: the sizes that
        // follow the density read it there, and so does everything that
        // draws an unfinished part or leaves it out.
        crate::interface_look::use_look(self.interface_look());
        self.settle_tab();
        self.profiler.add(
            crate::profiler::Stage::Sound,
            std::mem::take(&mut self.sound_spent),
        );
        let mut actions = std::mem::take(&mut self.open.outbox);
        // The Environment's Preview time (decision 0092), set in
        // Properties: the viewport shows the area at that hour.
        self.viewport.preview_hour = self.preview_hour(ui.ctx());
        // The launcher, while it's up, is all there is.
        if self.launcher(ui, &mut actions) {
            // Asked over the launcher too: a game stopped by going to it
            // never leaves its question unseen behind it.
            self.play_changes_window(ui);
            self.prompts(ui, &mut actions);
            self.about(ui);
            self.update_question(ui.ctx(), &mut actions);
            actions.append(&mut self.open.outbox);
            return actions;
        }
        self.refresh_sheet_pictures(ui.ctx());
        self.viewport.idle = !self.wants_frames(ui.ctx());
        if ui.input(|i| i.pointer.any_pressed()) {
            self.gesture += 1;
        }
        // While the game has the mouse and keys, they're the game's: the
        // editor's own keys are off (decision 0037).
        self.feed_input(ui.ctx());
        // Who has the keys this frame is decided once, here: each handler
        // below, and each tab drawn after, acts only on what is its own.
        self.keys = self.key_owner(ui.ctx());
        self.tool_keys(ui);
        self.paint_keys(ui);
        self.tool_extra_keys(ui);
        self.undo_keys(ui);
        self.editing_keys(ui);
        self.file_keys(ui, &mut actions);
        self.console_key(ui);
        self.shelf_keys(ui);
        self.search_key(ui);
        let seconds = ui.input(|i| i.stable_dt);
        self.run_game(seconds);
        self.status_bar(ui, &mut actions);
        // The tab bar and the ribbon; a File page or an unfinished
        // screen stands where the panels would be.
        let page = self.top(ui, &mut actions);
        self.follow_selection();
        self.play_frames(ui.input(|i| f64::from(i.stable_dt)));
        self.play_animator(ui.input(|i| f64::from(i.stable_dt)));
        let sheets = std::sync::Arc::clone(&self.viewport.sheets);
        let mut shown = HashSet::new();
        let mut area_requests = Vec::new();
        let mut library_requests = Vec::new();
        let mut mixer_requests = Vec::new();
        let picked_before = self.open.library_state.selected.clone();
        self.drop_stale_sound();
        let mut console_requests = Vec::new();
        let part = self.assembly_part();
        let mut requests = Vec::new();
        let preparing = std::time::Instant::now();
        // Worked out again only when something it shows changed: while
        // playing, the area usually doesn't. What's hidden or locked
        // (with everything inside it) is only worked out then too (the
        // phase review, speed): the key holds what was picked, which is
        // small.
        let key = ViewKey {
            revision: self.open.document.revision(),
            sources: self.sources,
            models: self.open.models.len(),
            hidden: self.open.area_panel.hidden.clone(),
            locked: self.open.area_panel.locked.clone(),
        };
        if self.open.editor_view.as_ref() != Some(&key) {
            let (hidden, locked) = self.open.area_panel.in_effect(&self.open.document);
            self.viewport.area = AreaView::of_placed(
                self.open.document.area(),
                &self.open.document.tree(),
                &self.open.document.world_matrices(),
                &self.open.models,
                &hidden,
                &locked,
            );
            self.open.editor_view = Some(key);
        }
        self.viewport.entities = self.open.document.area().len();
        self.set_tile_overlay();
        self.prepare_tools();
        self.find_problems();
        self.refresh_screen_pictures();
        self.refresh_screen();
        let code_problems = self.all_code_problems().into_owned();
        let copied: Vec<String> = self
            .open
            .copied
            .iter()
            .map(|(name, _)| name.clone())
            .collect();
        // Worked out only while the Game panel is in front.
        let game = if self.in_front.contains(&crate::panel::Panel::Game) {
            self.game_scene_now()
        } else {
            None
        };
        self.profiler
            .add(crate::profiler::Stage::Prepare, preparing.elapsed());
        // While playing, the selected entity's game code values as they
        // are now, for Properties (in front) to show instead of the
        // values it started with.
        let running = match (
            &mut self.open.playing,
            self.open.document.selection.primary(),
        ) {
            (Some(playing), Some(id))
                if self.in_front.contains(&crate::panel::Panel::Properties) =>
            {
                let names: Vec<String> = self
                    .open
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
        let playing = self.open.playing.is_some();
        let mut game_out = crate::game_panel::GamePanelOut::default();
        let mut renderer = crate::profiler::Timed {
            inner: renderer,
            spent: std::time::Duration::ZERO,
        };
        let mut frames_out = crate::panels::FramesOut::default();
        self.tileset_texture(ui.ctx());
        self.atlas_texture_now(ui.ctx());
        let live = self.live_animator();
        // Which entities use each code file's components, for the Code
        // panel and the Library (decision 0085): only worked out while
        // either is in view, and then only when the area changed.
        let code_uses =
            if self.in_front.contains(&Panel::Code) || self.in_front.contains(&Panel::Library) {
                self.code_uses_now()
            } else {
                std::sync::Arc::default()
            };
        let open_now = self.open_now();
        let mut map_requests = Vec::new();
        let map_shown = self.map_shown();
        let map_ms = (ui.input(|i| i.time).max(0.0) * 1000.0) as u64;
        let map_name = self
            .open
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
                hidden: &self.open.map_edits.hidden,
                playing,
                paint: crate::map_tab::PaintShown {
                    code: self.viewport.tile_overlay.paint.code,
                    exact: self.viewport.tile_overlay.paint.exact,
                    piece: self.viewport.tile_overlay.paint.piece,
                    stamp: self
                        .viewport
                        .tile_overlay
                        .paint
                        .stamp
                        .as_ref()
                        .map(|s| s.size),
                    picture: tileset
                        .as_ref()
                        .and_then(|t| t.picture)
                        .and_then(|p| sheets.tile_art(p))
                        .and_then(|art| {
                            let (texture, atlas) = crate::workbench_tileset::texture_of(
                                &self.open.tile_textures,
                                art,
                            )?;
                            Some(crate::tileset_view::Picture {
                                texture,
                                atlas,
                                size: art.size,
                                durations: &art.durations,
                                art,
                            })
                        }),
                    ms: map_ms,
                    accent: self.accent.colors().fill,
                },
            });
        // The hints for what's selected (decision 0085), while they show.
        let hints =
            (self.hints && self.in_front.contains(&Panel::Properties)).then(|| self.hints());
        let hints_off = !self.hints;
        let atlas_problem = self.atlas_picture().1;
        let frames = self.open.frames_editor.as_mut().map(|editor| {
            crate::workbench_frames::frames_ui(
                editor,
                &sheets,
                &self.open.sheet_pictures,
                &self.open.cue_sounds,
            )
        });
        let tile_ms = (ui.input(|i| i.time).max(0.0) * 1000.0) as u64;
        if self.open.tilesets.tab().is_some() && sheets.tiles_animate() {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(40));
        }
        let tileset = self.open.tilesets.tab_mut().map(|editor| {
            crate::workbench_tileset::tileset_ui(
                editor,
                &sheets,
                &self.open.tile_textures,
                self.open.pixels.tile,
                tile_ms,
                self.accent.colors().fill,
            )
        });
        let screen_pictures = self.open.screen_pictures.pictures.clone();
        let screen_fonts = self
            .screen_fonts
            .get_or_insert_with(crate::game_screens::editor_fonts);
        // Only worked out while a screen is open: what it can be tied to
        // once per build of the game's code, the pictures only while the
        // Image tool is in use.
        let (picture_files, screen_game) = match &self.open.screen_doc {
            Some(doc) => {
                let program = self.open.program.as_ref();
                let game = match &self.open.screen_game {
                    Some((built, game)) if MadeFrom::same(built.as_ref(), program) => {
                        std::sync::Arc::clone(game)
                    }
                    _ => {
                        let game = std::sync::Arc::new(crate::screen_ties::screen_game(
                            program.map(|p| &**p),
                            &self.open.document.registry,
                        ));
                        let built = program.map(MadeFrom::new);
                        self.open.screen_game = Some((built, std::sync::Arc::clone(&game)));
                        game
                    }
                };
                let files: Vec<String> = if doc.tool == crate::canvas_tools::CanvasTool::Image {
                    self.open
                        .library
                        .iter()
                        .filter(|i| {
                            matches!(
                                i.kind,
                                crate::library::LibraryKind::Texture
                                    | crate::library::LibraryKind::SpriteSheet
                            )
                        })
                        .map(|i| i.path.clone())
                        .collect()
                } else {
                    Vec::new()
                };
                (files, game)
            }
            None => (Vec::new(), std::sync::Arc::default()),
        };
        let screen = self
            .open
            .screen_doc
            .as_mut()
            .map(|doc| crate::panels::ScreenUi {
                doc,
                fonts: screen_fonts,
                pictures: screen_pictures,
                picture_files,
                game: screen_game,
            });
        let atlas = self.open.atlas_editor.as_mut().map(|editor| {
            let picture = self.open.atlas_texture.as_ref().and_then(|(art, texture)| {
                Some(crate::atlas_view::AtlasPicture {
                    texture,
                    size: art.size,
                    rgba: art.frames.first()?,
                })
            });
            (editor, picture, atlas_problem)
        });
        let animator = self.open.animator_editor.as_mut().map(|editor| {
            crate::workbench_animator::animator_ui(
                editor,
                &sheets,
                &self.open.sheet_pictures,
                live,
                self.accent.colors().fill,
            )
        });
        let mut viewer = PanelViewer {
            area_state: &mut self.open.area_panel,
            copied: &copied,
            viewport: &mut self.viewport,
            renderer: &mut renderer,
            console: &mut self.console,
            console_requests: &mut console_requests,
            problems: &self.open.problems.list,
            code_problems: &code_problems,
            game: game.as_ref().map(|(_, scene)| scene),
            ui_test: self.ui_test.clone(),
            game_screens: &self.open.game_screens.pictures,
            playing,
            game_input: self.game_input,
            pixels: self.open.pixels,
            game_out: &mut game_out,
            document: &self.open.document,
            models: &self.open.models,
            area_requests: &mut area_requests,
            project: self.open.project.as_ref(),
            library: &self.open.library,
            folders: &self.open.folders,
            library_state: &mut self.open.library_state,
            library_requests: &mut library_requests,
            hints: if hints_off { None } else { hints.as_deref() },
            sounds: crate::library::Sounds {
                states: &self.open.sounds,
                events: &self.open.sound_events,
                preview: self.open.preview.as_ref(),
                mixer: &self.open.mixer.mixer,
                sheets: &sheets,
                sheet_pictures: &self.open.sheet_pictures,
                code_uses: &code_uses,
            },
            mixer: &self.open.mixer,
            mixer_requests: &mut mixer_requests,
            sound_shown: self
                .open
                .sound_shown
                .as_ref()
                .map(|(path, _)| path.as_str()),
            text_file: self.open.text_file.as_ref(),
            part: part.as_ref(),
            running: running.as_ref(),
            requests: &mut requests,
            help: crate::properties::Help {
                manual: &self.manual_url,
                places: self.open.program.as_ref().map(|p| &p.places),
            },
            accent: self.accent,
            in_front: &self.in_front,
            shown: &mut shown,
            profiler: &self.profiler,
            frames,
            frames_out: &mut frames_out,
            animator,
            tileset,
            atlas,
            map,
            map_requests: &mut map_requests,
            screen,
            keys: self.keys,
        };
        let asks = if page {
            self.shell.drawn.clear();
            Default::default()
        } else {
            crate::shell::middle(ui, &mut self.shell, open_now, &mut viewer)
        };
        self.in_front = shown;
        self.pointer_over = frames_out.pointer_over;
        self.doc_asks(asks, ui.ctx(), &mut actions);
        self.frames_tabs_done(frames_out, &mut actions);
        self.map_requests_done(map_requests);
        self.tile_overlay_done();
        self.paint_frame();
        self.game_panel_done(ui.ctx(), game_out);
        self.tool_events();
        for request in requests {
            self.apply_property(request, self.gesture);
        }
        actions.append(&mut self.open.outbox);
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
        self.open.game_scene = game;
        if self.files_hovering {
            drop_hint(ui, self.accent);
        }
        self.about(ui);
        self.play_changes_window(ui);
        // What waited for Keep or Undo all (a quit, say) goes on now.
        actions.append(&mut self.open.outbox);
        self.input_actions_window(ui.ctx());
        self.settings(ui.ctx(), &mut actions);
        self.export_window(ui.ctx(), &mut actions);
        self.prompts(ui, &mut actions);
        self.update_question(ui.ctx(), &mut actions);
        self.hold_while_playing(&mut actions);
        self.profiler
            .add(crate::profiler::Stage::Draw, renderer.spent);
        let (components, entities) = match &mut self.open.playing {
            Some(playing) => (
                playing.game.as_mut().map_or(0, script::Game::len),
                playing.world.len(),
            ),
            None => (0, self.open.document.area().len()),
        };
        let drawn = renderer.inner.take_report();
        self.profiler.end(drawn, components, entities);
        actions
    }

    /// Q, W, R, S and T pick the tools, where the keys are the area's.
    pub(crate) fn tool_keys(&mut self, ui: &Ui) {
        // While flying, W A S D, Q and E move the camera instead.
        if !self.keys.is_area() || self.viewport.is_flying() {
            return;
        }
        let pressed = ui.input(|i| {
            Tool::ALL
                .into_iter()
                .find(|tool| i.key_pressed(tool.key()) && i.modifiers.is_none())
        });
        if let Some(tool) = pressed {
            self.viewport.tool = tool;
            self.viewport.tile_overlay.paint.tool = None;
        }
    }

    /// The `'` key (beside Æ on a Norwegian keyboard) jumps to the
    /// console's command line, bringing the Console to the front (decided
    /// with the project lead, 2026-10-23). Not while typing.
    pub(crate) fn console_key(&mut self, ui: &Ui) {
        if !self.keys.takes_shortcuts() || self.viewport.is_flying() {
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
        self.show_panel(Panel::Console);
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
        let print = Fingerprint::of(&self.open.document, self.sources);
        self.open.problems.refresh_between(
            self.open.document.revision(),
            self.open.pixels.pixels_per_unit,
            self.open.document.area(),
            &self.viewport.area.sprites,
        );
        if self.open.problems.is_from(&print) {
            return;
        }
        let known: HashSet<AssetId> = self.open.library.iter().filter_map(|i| i.id).collect();
        let assemblies: HashSet<AssetId> = self.open.assemblies.keys().copied().collect();
        let tile_gaps = self.viewport.sheets.tile_gaps();
        self.open.problems.refresh(
            print,
            &Sources {
                area: self.open.document.area(),
                registry: &self.open.document.registry,
                models: &self.open.models,
                known: &known,
                assemblies: &assemblies,
                tile_gaps: &tile_gaps,
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
            0 if !self.open.code_problems.is_empty() => Some((
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
        self.open.code_problems = problems;
        self.open.code_checked = Some(chrono::Local::now().format("%H:%M:%S").to_string());
    }

    /// Hands over the program's logo, for the top bar.
    pub fn set_logo(&mut self, logo: Option<egui::TextureId>) {
        self.logo = logo;
    }

    /// Mistakes in the game's code, from the last check.
    pub fn code_problems(&self) -> &[script::CodeProblem] {
        &self.open.code_problems
    }

    /// Entity `id`'s component `name` as it is now: while the game plays,
    /// as game code (or, for Solis's own components, the running world)
    /// has it; otherwise as the area has it. For the Console's `get` and
    /// `values`, and for tests reading a game as it plays.
    pub fn values_now(&mut self, id: EntityId, name: &str) -> Option<reflect::ComponentData> {
        if let Some(playing) = &mut self.open.playing {
            if let Some(found) = playing.game.as_mut().and_then(|g| g.values(id, name)) {
                return Some(found);
            }
            if let Some(found) = playing.world.component_data(id, name) {
                return Some(found);
            }
        }
        self.open
            .document
            .area()
            .get(id)?
            .components
            .get(name)
            .cloned()
    }

    /// The code's mistakes, then pictures its screens name that can't be
    /// shown (decision 0089): everything Problems lists from the code.
    pub(crate) fn all_code_problems(&self) -> std::borrow::Cow<'_, [script::CodeProblem]> {
        let pictures = self.picture_problems();
        if pictures.is_empty() {
            std::borrow::Cow::Borrowed(&self.open.code_problems)
        } else {
            std::borrow::Cow::Owned([&self.open.code_problems[..], pictures].concat())
        }
    }

    /// The problems found in the area, for the app and tests.
    pub fn problems(&self) -> &[crate::problems::Problem] {
        &self.open.problems.list
    }

    /// The console's earlier commands, oldest first, for saving.
    pub fn console_history(&self) -> &[String] {
        self.console.history()
    }

    /// Ctrl+Z undoes; Ctrl+Y or Ctrl+Shift+Z redoes: the undo steps of
    /// whoever has the keys. Not while typing in a field, where they undo
    /// the typing instead.
    pub(crate) fn undo_keys(&mut self, ui: &Ui) {
        if !self.keys.takes_shortcuts() {
            return;
        }
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
        if !redo && !undo {
            return;
        }
        match self.keys {
            // Left before the keys were read.
            KeyOwner::Game
            | KeyOwner::Question
            | KeyOwner::Binding
            | KeyOwner::Field
            | KeyOwner::Page => {}
            KeyOwner::Area if redo => self.redo(),
            KeyOwner::Area => self.undo(),
            // Over the Map tab, or painting: the map's changes.
            KeyOwner::Map => self.map_undo(redo, undo),
            KeyOwner::Clips | KeyOwner::Frames => self.frames_undo(redo, undo),
            KeyOwner::Animator => self.animator_undo(redo, undo),
            KeyOwner::Tileset | KeyOwner::TilesetProperties => {
                if let Some(path) = self.open.tilesets.undone_by(self.keys) {
                    let path = path.to_owned();
                    self.tileset_undo(&path, redo, undo);
                }
            }
            KeyOwner::Atlas => self.atlas_undo(redo),
            KeyOwner::Canvas | KeyOwner::Elements => self.screen_undo(redo),
        }
    }

    /// Ctrl+N, Ctrl+O, Ctrl+S and Ctrl+Shift+S: new, open, save and save
    /// as (decided with the project lead, 2026-10-23). Not while typing.
    pub(crate) fn file_keys(&mut self, ui: &Ui, actions: &mut Vec<WorkbenchAction>) {
        if !self.keys.takes_shortcuts() {
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

    /// Asks for `action`. This is the one check before the open area is
    /// left ([`leaves_the_area`]: another area or project in its place, or
    /// the editor closed): first Keep or Undo all, if edits made while
    /// playing wait for it, then the question about unsaved changes. The
    /// action waits for each answer, so nothing is lost without a word.
    pub(crate) fn request(&mut self, action: WorkbenchAction, actions: &mut Vec<WorkbenchAction>) {
        if self.open.playing.is_some() && crate::play::waits_for_stop(&action) {
            // Refused, with a word in the Console, at the end of the frame.
            actions.push(action);
            return;
        }
        if leaves_the_area(&action) {
            // Only quitting gets here while playing. The game stops
            // first, which asks about the edits made meanwhile.
            if self.open.playing.is_some() {
                self.stop();
            }
            if let Some(changes) = &mut self.open.play_changes {
                // Asked for again once Keep or Undo all is chosen.
                changes.then = Some(action);
                return;
            }
            if self.open.document.is_dirty() {
                self.open.unsaved_prompt = Some(action);
                return;
            }
        }
        if action == WorkbenchAction::Quit {
            self.may_close = true;
        }
        actions.push(action);
    }

    /// The app called off a queued quit (the save before it was cancelled
    /// or failed): a later close asks about unsaved changes again.
    pub fn quit_cancelled(&mut self) {
        self.may_close = false;
    }

    /// Whether the window may close now. If edits made while playing wait
    /// for Keep or Undo all, or the area has unsaved changes, asks first
    /// (and the app should cancel this close).
    pub fn close_requested(&mut self) -> bool {
        // Edits made while playing are asked about at Stop, so the game
        // stops for the question, and the close waits for its answer.
        if self.edited_while_playing() {
            self.stop();
        }
        if let Some(changes) = &mut self.open.play_changes {
            changes.then = Some(WorkbenchAction::Quit);
            return false;
        }
        // While playing, the area that matters is the one from before Play.
        let dirty = match &self.open.playing {
            Some(playing) => playing.before.is_dirty(),
            None => self.open.document.is_dirty(),
        };
        if self.may_close || !dirty {
            if self.open.playing.is_some() {
                self.stop();
            }
            return true;
        }
        // Asked first: Cancel leaves the game running.
        self.open.unsaved_prompt = Some(WorkbenchAction::Quit);
        false
    }

    /// The questions about unsaved changes and files changed on disk.
    fn prompts(&mut self, ui: &mut Ui, actions: &mut Vec<WorkbenchAction>) {
        if let Some(waiting) = self.open.unsaved_prompt.clone() {
            let name = self.open.document.name();
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
                            self.open.unsaved_prompt = None;
                            // A cancelled restart is no restart.
                            self.restarting = false;
                        }
                    });
                });
            if let Some(save) = answer {
                self.open.unsaved_prompt = None;
                // Saving or leaving waits for Stop, which puts the area
                // back as it was before Play.
                if self.open.playing.is_some() {
                    self.stop();
                }
                if let Some(changes) = &mut self.open.play_changes {
                    // Edits made while this question was up (the game
                    // played on): Keep or Undo all comes first, and this
                    // is asked again after, about the area as it is then.
                    changes.then = Some(waiting);
                } else {
                    if save {
                        actions.push(WorkbenchAction::SaveArea);
                    }
                    if waiting == WorkbenchAction::Quit {
                        self.may_close = true;
                    }
                    actions.push(waiting);
                }
            }
        }
        // Asked after the game stops: loading it now would only be undone.
        if self.open.pending_reload.is_some() && self.open.playing.is_none() {
            let name = self.open.document.name();
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
                && let Some(area) = self.open.pending_reload.take()
                && load
            {
                self.reload_area(area);
            }
        }
    }

    /// Delete deletes the selection, Ctrl+D duplicates it, F2 renames it
    /// and F frames it (decided with the project lead, 2026-10-23), where
    /// the keys are the area's: over a tab with a file of its own they are
    /// that tab's, and never reach the area as well. Not while flying.
    pub(crate) fn editing_keys(&mut self, ui: &Ui) {
        if !self.keys.is_area() || self.viewport.is_flying() {
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
        if delete {
            self.delete_selection();
        }
        if duplicate {
            self.duplicate_selection();
        }
        if rename
            && let Some(id) = self.open.document.selection.primary()
            && let Some(entity) = self.open.document.area().get(id)
        {
            crate::area_panel::begin_rename(ui.ctx(), id, &entity.name);
        }
        if frame {
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

/// Whether `action` leaves the open area: puts another area or project
/// in its place, or closes the editor. [`Workbench::request`] asks about
/// what would be lost before any of these goes ahead.
pub(crate) fn leaves_the_area(action: &WorkbenchAction) -> bool {
    matches!(
        action,
        WorkbenchAction::NewArea
            | WorkbenchAction::OpenArea
            | WorkbenchAction::OpenAreaFile(_)
            | WorkbenchAction::OpenProject
            | WorkbenchAction::NewProject
            | WorkbenchAction::OpenRecent(_)
            | WorkbenchAction::MakeSampleGame
            | WorkbenchAction::MakeUiSample
            | WorkbenchAction::MakeBenchmark
            | WorkbenchAction::MakePlayground
            | WorkbenchAction::Quit
    )
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
