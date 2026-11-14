//! What the editor asks the app to do ([`WorkbenchAction`]): the
//! workbench never touches the window, the GPU or files itself.

use reflect::{AssetId, EntityId};

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
    /// Open the project in this folder (File › Recent projects).
    OpenRecent(std::path::PathBuf),
    /// Read the projects Solis knows afresh, for Recent projects.
    ListProjects,
    /// Pin (true) or unpin a project in the list on Recent projects.
    PinProject(std::path::PathBuf, bool),
    /// Forget a project in the list on Recent projects; its files stay.
    ForgetProject(std::path::PathBuf),
    /// Show a project's folder in Explorer (a card on Recent projects).
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
    /// Start a new area (New area, after the area's tab, Ctrl+N): the app calls
    /// [`Workbench::new_area`](crate::Workbench::new_area).
    NewArea,
    /// Ask which area file to open (Open area…, after the area's tab,
    /// Ctrl+O).
    OpenArea,
    /// Save the area to its file, or ask where if it has none (Ctrl+S).
    SaveArea,
    /// Ask where to save the area (Ctrl+Shift+S).
    SaveAreaAs,
    /// Save this entity and everything under it as an assembly in the
    /// project, then turn it into a copy of it with
    /// [`Workbench::make_copy`](crate::Workbench::make_copy).
    CreateAssembly(EntityId),
    /// Open this area file (a double-click in the Library); the app reads
    /// it and calls [`Workbench::open_area`](crate::Workbench::open_area).
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
    /// got with [`Workbench::set_preview`](crate::Workbench::set_preview).
    PlaySound(std::path::PathBuf),
    /// Stop listening to the sound playing.
    StopSound,
    /// Make a new sound event in this folder (inside the project's
    /// assets), and show it with [`Workbench::show_in_library`](crate::Workbench::show_in_library).
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
    /// answers every one, in order: [`Workbench::file_saved`](crate::Workbench::file_saved) once it's
    /// written (and it keeps `file` as the file from then on), or
    /// [`Workbench::file_not_saved`](crate::Workbench::file_not_saved) if it couldn't be, or wasn't tried.
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
    /// [`Workbench::set_assembly`](crate::Workbench::set_assembly), which updates every copy.
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
    /// the app reads it and calls [`Workbench::replay`](crate::Workbench::replay).
    OpenRecording,
    /// Save this save game (game code's `Saves:saveTo`) as the slot's file in the
    /// project's `saves` folder.
    SaveGame {
        /// Its slot: the file's name.
        slot: String,
        /// The save's text.
        text: String,
    },
    /// Read the slot's save game from the project's `saves` folder (game
    /// code's `load`), and hand it to [`Workbench::load_game`](crate::Workbench::load_game).
    LoadGame(String),
    /// Ask which save game to load (Debug > Load save), and hand it to
    /// [`Workbench::load_game`](crate::Workbench::load_game).
    OpenSave,
    /// Check GitHub for a newer build now (File > Updates).
    CheckForUpdates,
    /// Open the user guide (Documentation in the tab bar, File › Learn
    /// and About, a component's help), at the chapter of the manual page
    /// named (`lights`), or at its start; in the program chosen in
    /// Preferences (decision 0112).
    OpenGuide(Option<String>),
    /// Ask which program the guide opens in (Preferences' A program you
    /// choose…); the app sets it with `Workbench::set_guide_browser`.
    PickGuideBrowser,
    /// Turn checking for updates when starting on or off (File > Updates when starting); the app saves it.
    SetAutoUpdate(bool),
    /// Download the newer build offered (the update question's Update
    /// now); it installs on Restart to update or when Solis closes.
    DownloadUpdate,
    /// Open the page to download the newer build by hand (the update
    /// question, for a copy that wasn't installed).
    OpenUpdatePage,
    /// Open a file the program carries (File › About and Updates): the
    /// installed copy's own, or the repository's online.
    OpenProgramFile(crate::file_pages::ProgramFile),
    /// The update question was answered Open without updating: skip this
    /// build from now on when it's given (Skip this version).
    SkipUpdate(Option<u64>),
}
