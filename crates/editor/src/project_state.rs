//! What belongs to the open project, apart from the program.
//!
//! The editor's state is in two places. [`Workbench`] itself holds what is
//! the program's and the user's: the layout, the theme, the cameras, the
//! Console, the launcher, updates. [`ProjectState`] holds everything that
//! belongs to the open project (or to working without one): the area and
//! its undo history, the Library, the files open in tabs with their undo
//! steps, the running game, the questions waiting for an answer.
//! [`Workbench::set_project`] replaces it whole with a new one, so nothing
//! of one project is ever used with another: its undo steps, its open
//! files and its paths would otherwise be carried out on the files of the
//! project opened next (a copy of a project has the same paths and IDs).
//!
//! A new field goes where it belongs: here if it is the open project's,
//! in `Workbench` if it is the program's. There is no list of fields to
//! clear, so none can be left off it.

use crate::console::LogLevel;
use crate::document::Document;
use crate::library::{LibraryItem, ProjectInfo};
use crate::made_from::MadeFrom;
use crate::models::ModelInfo;
use crate::properties::AssemblyPart;
use crate::workbench::{GameKey, PartKey, ViewKey, Workbench, WorkbenchAction};
use reflect::{AssetId, EntityId};
use std::collections::{BTreeMap, HashMap};

/// Everything the editor holds that belongs to the open project (or to
/// working without one), kept between frames. Another project starts
/// with a new one ([`Workbench::set_project`]).
/// Which entities use each code file's components, with the area's
/// revision, the sources' count and the build of the game's code it was
/// worked out for.
pub(crate) type CodeUses = (
    u64,
    u64,
    Option<MadeFrom<script::Compiled>>,
    std::sync::Arc<BTreeMap<String, String>>,
);

#[derive(Default)]
pub(crate) struct ProjectState {
    /// The open project, if any.
    pub(crate) project: Option<ProjectInfo>,
    /// The open area and what's selected in it.
    pub(crate) document: Document,
    /// What the editor knows about the imported models, by asset.
    pub(crate) models: BTreeMap<AssetId, ModelInfo>,
    /// Build > Export, while it's open (decision 0093).
    pub(crate) export: Option<crate::export_dialog::ExportForm>,
    /// How the open project was last exported, to start from.
    pub(crate) last_export: Option<crate::export_dialog::Playtest>,
    /// Where the open project was last exported on this computer, if the
    /// app remembers.
    pub(crate) last_export_folder: Option<std::path::PathBuf>,
    /// The project's assets, for the Library.
    pub(crate) library: Vec<LibraryItem>,
    /// The project's assemblies the area uses, as read from their files.
    pub(crate) assemblies: BTreeMap<AssetId, std::sync::Arc<area::text::AreaFile>>,
    /// Actions asked for outside `Workbench::show`'s own flow (menus,
    /// Properties), handed to the app with the next frame's. One the app
    /// hasn't taken when another project opens is let go: it names a file
    /// of the old project, and would be carried out in the new one.
    pub(crate) outbox: Vec<WorkbenchAction>,
    /// What waits for an answer about unsaved changes: Save, Don't save
    /// or Cancel.
    pub(crate) unsaved_prompt: Option<WorkbenchAction>,
    /// The area as the file now on disk holds it, waiting for Keep mine
    /// or Load the file.
    pub(crate) pending_reload: Option<area::Area>,
    /// While a gizmo is dragged: what moves, as it was when the drag
    /// began, and the drag's gesture.
    pub(crate) moving: Option<(crate::transforming::Targets, u64)>,
    /// What the Area panel searches for, folds, hides and locks.
    pub(crate) area_panel: crate::area_panel::AreaPanelState,
    /// Components' values copied in Properties, to paste. They go with
    /// their project: they are shaped by its game code and name its
    /// assets and entities, which mean nothing in another.
    pub(crate) copied: Vec<(String, reflect::ComponentData)>,
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
    /// The text file shown in Properties, as the app last read it (by
    /// path inside the assets), made ready to show.
    pub(crate) text_file: Option<(String, crate::text_view::TextShown)>,
    /// The project's buses, mute and solo, and the meters (the Mixer).
    pub(crate) mixer: crate::mixer_view::MixerState,
    /// The project's 2D settings (Settings > 2D, `pixels.solpix`).
    pub(crate) pixels: area::pixels::PixelSettings,
    /// When the game's code was last checked (the local time, for the
    /// status bar); none before the first check.
    pub(crate) code_checked: Option<String>,
    /// Changes made while playing, waiting for Keep or Undo all.
    pub(crate) play_changes: Option<crate::play_changes::PlayChanges>,
    /// The sound event a sprite's clip event plays, by the event's name
    /// (Phase 9, D3): each sound event by its path without `.solsound`
    /// ("sfx/footstep"), and by its file's name alone ("footstep") when
    /// no other sound event has that name.
    pub(crate) cue_sounds: HashMap<String, String>,
    /// Saves of the project's files the app hasn't answered yet: what the
    /// editor has that's newer than the app's (`own_files.rs`).
    pub(crate) unanswered: crate::own_files::Unanswered,
    /// The clips file open in the Clips and Frames tabs (Phase 9, D3).
    pub(crate) frames_editor: Option<crate::frames_editor::FramesEditor>,
    /// The animator file open in the Animator tab (Phase 9, D3b).
    pub(crate) animator_editor: Option<crate::animator_editor::AnimatorEditor>,
    /// The project's tilesets being changed, each with its file's undo
    /// steps, and the one open in the Tileset tab (Phase 9, D5).
    pub(crate) tilesets: crate::workbench_tileset::TilesetEdits,
    /// The atlas open in the Atlas Crop tab (decision 0089).
    pub(crate) atlas_editor: Option<crate::atlas_editor::AtlasEditor>,
    /// The open atlas's picture as a texture.
    pub(crate) atlas_texture: crate::workbench_atlas::AtlasTexture,
    /// The open tileset's picture as a texture.
    pub(crate) tile_textures: crate::workbench_tileset::TileTextures,
    /// The Map tab's undo steps, the selected map's values shown over it
    /// and its hidden layers (Phase 9, D4).
    pub(crate) map_edits: crate::workbench_tiles::MapEdits,
    /// Each sprite sheet's atlas as an egui picture, by path, for the
    /// Library and Properties, and which sheet each was made from.
    pub(crate) sheet_pictures: HashMap<String, egui::TextureHandle>,
    pub(crate) sheet_pictures_from: HashMap<String, MadeFrom<assets::sprite_sheet::SpriteSheet>>,
    /// Where assets being placed go, when they were dropped somewhere:
    /// the point, and the entity they go under.
    pub(crate) place_targets: BTreeMap<AssetId, (glam::Vec3, Option<EntityId>)>,
    /// Pictures being placed as sprites, waiting to be read as sheets:
    /// where each goes (`None`: where the camera looks).
    pub(crate) pictures_placing: BTreeMap<AssetId, Option<(glam::Vec3, Option<EntityId>)>>,
    /// Mistakes found in the area, for the Problems panel.
    pub(crate) problems: crate::problems::Problems,
    /// The game, while it runs.
    pub(crate) playing: Option<crate::play::Playing>,
    /// Mistakes in the game's code, from the last check.
    pub(crate) code_problems: Vec<script::CodeProblem>,
    /// The game's program, from the last code without mistakes.
    pub(crate) program: Option<std::sync::Arc<script::Compiled>>,
    /// Whether a check of the game's code has handed its program over
    /// (`Workbench::set_program`), none when there's no code.
    pub(crate) program_given: bool,
    /// The components it put in the registry, by name.
    pub(crate) game_components: std::collections::BTreeSet<String>,
    /// What the viewport's `AreaView` was last worked out for.
    pub(crate) editor_view: Option<ViewKey>,
    /// The project's input actions (decision 0037), from its
    /// `input.solinput` (`Workbench::set_bindings`); the ready-made ones
    /// without a project.
    pub(crate) bindings: input::Bindings,
    /// Whether the Input actions window is open.
    pub(crate) input_page: Option<crate::file_pages::InputPage>,
    /// The seed every Play starts the game's random numbers from, if one
    /// was chosen (`seed` in the Console); otherwise each Play has a new
    /// one. Chosen for one game, so another project starts without it.
    pub(crate) fixed_seed: Option<u64>,
    /// The game's screens while it plays (decision 0087).
    pub(crate) game_screens: crate::game_screens::GameScreens,
    /// The pictures the game's screens show (decision 0089).
    pub(crate) screen_pictures: crate::screen_pictures::ScreenPictures,
    /// The screen open on the UI editor's canvas (decision 0089).
    pub(crate) screen_doc: Option<crate::screen_doc::ScreenDoc>,
    /// The build of the game's code its view was last tried with,
    /// whatever came of it (`Some(None)`: with none, the code having
    /// mistakes); `None` until it's tried.
    pub(crate) screen_from: Option<Option<MadeFrom<script::Compiled>>>,
    /// What the open screen can be tied to and send, and the build of the
    /// game's code it's for.
    /// Which entities use each code file's components (the Library's
    /// Code view), and the area's revision, the sources' count and the
    /// build of the game's code it was worked out for.
    pub(crate) code_uses: Option<CodeUses>,
    pub(crate) screen_game: Option<(
        Option<MadeFrom<script::Compiled>>,
        std::sync::Arc<crate::screen_ties::ScreenGame>,
    )>,
    /// What the Game panel showed last, and what it was worked out for:
    /// made again only when the world, the models or the state changed
    /// (not on a frame without a tick, or while paused).
    pub(crate) game_scene: Option<(GameKey, crate::game_panel::GameScene)>,
    /// Game code's sound requests since the app last took them.
    pub(crate) game_sounds: Vec<crate::game_sound::GameSound>,
    /// Its components whose names were taken (by the engine's own, or a
    /// Rust one's): they don't run.
    pub(crate) left_out: std::collections::BTreeSet<String>,
    /// The Lust compiler's notes on the game code last checked (older ways of writing that
    /// still work), told in the Console once each time they change.
    pub(crate) told_code_notes: Vec<String>,
    /// Kinds of component written in Rust, run in play mode.
    pub(crate) rust_components: Vec<script::RustKind>,
    /// The last `Workbench::assembly_part`, with the selection, revision and
    /// sources it was worked out for: kept, since it expands the whole
    /// assembly.
    pub(crate) assembly_part_kept: std::cell::RefCell<Option<(PartKey, Option<AssemblyPart>)>>,
    /// Whether game code asked to close the game (`Window:quit()`).
    pub(crate) quit_asked: bool,
    /// Whether the last project's Preview time (the Environment's, kept
    /// in egui's memory) has been dropped yet: it is at the next frame.
    pub(crate) preview_forgotten: bool,
}

impl Workbench {
    /// Shows `project` as the open one (or none). A different project
    /// starts from nothing: a new area, and nothing of the old project's
    /// (everything in `ProjectState`). What is the program's stays: the
    /// layout without the tabs of the old project's files, the cameras,
    /// the Console, and the components the program itself registered.
    pub fn set_project(&mut self, project: Option<ProjectInfo>) {
        if project.is_some() {
            // A project is open: Recent projects has done its job.
            self.back_to_editor();
        }
        if project == self.open.project {
            return;
        }
        // A game still running goes with its project (opening one waits
        // for Stop, so there is none unless the app skips that), and the
        // mouse and keys come back from it.
        self.open.playing = None;
        self.give_back_input();
        // The old project's own components leave the registry; the rest
        // of it (components the program registered) is kept.
        self.set_program(None);
        self.set_rust_components(Vec::new());
        let registry = std::mem::take(&mut self.open.document.registry);
        self.open = ProjectState {
            project,
            ..ProjectState::default()
        };
        self.open.document.registry = registry;
        self.viewport.forget_project();
        // The old project's files closed with it: their documents and
        // shelf tabs go, as they come with what is open.
        // Whatever was worked out for the old project is worked out again.
        self.sources += 1;
        if let Some(name) = self.open.project.as_ref().map(|p| p.name.clone()) {
            self.log(LogLevel::Info, format!("Opened the project {name}."));
        }
    }
}

#[cfg(test)]
#[path = "project_state_tests.rs"]
mod tests;
