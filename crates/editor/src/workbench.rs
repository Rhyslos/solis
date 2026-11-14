//! The whole editor window, inside the frame of
//! [decision 0099](../../../docs/decisions/0099-the-g-layout.md)
//! (`shell/`): the tab bar and ribbon, the editor groups and the shelf
//! that hold the panels, and the status bar.
//!
//! The workbench never touches the window, the GPU or files itself: it
//! returns [`WorkbenchAction`]s for the app to carry out, and draws the
//! viewport through a [`ViewportRenderer`](crate::viewport::ViewportRenderer)
//! it is handed each frame.

use crate::camera::Camera;
#[cfg(test)]
use crate::console::CONSOLE_LIMIT;
use crate::console::{Console, LogLevel, LogLine};
use crate::document::Document;
use crate::key_owner::KeyOwner;
use crate::library::{LibraryItem, LibraryKind, ProjectInfo};
use crate::models::ModelInfo;
use crate::panel::Panel;
use crate::project_state::ProjectState;
use crate::theme::{self, Accent};
use crate::viewport::{Overlays, Tool, Viewport};
use reflect::{AssetId, EntityId};
use std::collections::{BTreeMap, HashMap, HashSet};

#[path = "workbench_action.rs"]
mod action;
#[path = "workbench_area_files.rs"]
mod area_files;
#[path = "workbench_frame.rs"]
mod frame;
#[path = "workbench_keys.rs"]
mod keys;
#[path = "workbench_leaving.rs"]
mod leaving;
#[path = "workbench_problems.rs"]
mod problems;

pub use action::WorkbenchAction;

/// Facts about the program, shown in the window title and the About box.
/// The app passes these in, so this crate never names the program itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppInfo {
    /// The program's name, such as "Solis".
    pub name: String,
    /// The version, such as "0.0.1".
    pub version: String,
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
    /// The File tab's own pages: which is in front, and how Recent
    /// projects is searched and ordered.
    pub(crate) file_pages: crate::file_pages::FilePages,
    /// The projects Solis knows, for Recent projects (the app reads them).
    pub(crate) known_projects: Vec<crate::file_pages::KnownProject>,
    /// This copy's build number, for the project cards and About.
    pub(crate) this_build: Option<u64>,
    /// Which program the user guide opens in, and the browsers found.
    pub(crate) guide: crate::guide::GuideSettings,
    /// How long the game's sound took last frame (the Profiler's Sound).
    pub(crate) sound_spent: std::time::Duration,
    /// The Documents folder, for exports to go in by default.
    pub(crate) documents: Option<std::path::PathBuf>,
    /// How updating is going, as the app last said.
    pub(crate) update: crate::updates::UpdateNotice,
    /// What the newer build found brings (File › Updates).
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
    /// Which real-time limits this test's games keep (TEST-02): a test
    /// that compares one run with another plays unhurried.
    #[cfg(test)]
    pub(crate) unhurried: crate::game_code::Unhurried,
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
    /// The document's brush revision the brushes were drawn at.
    brushes: u64,
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
            file_pages: Default::default(),
            known_projects: Vec::new(),
            this_build: None,
            guide: Default::default(),
            sound_spent: std::time::Duration::ZERO,
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
            #[cfg(test)]
            unhurried: crate::game_code::Unhurried::default(),
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
        self.open.library = items.into();
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
    /// happens ([decision 0026](../../../docs/decisions/0026-draw-only-when-something-changes.md)).
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
        self.open.brushes.refresh(&mut self.open.document);
        let brushes = self.open.document.brushes();
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
            brushes,
            state,
        };
        Some(match self.open.game_scene.take() {
            Some((was, scene)) if was == key => (key, scene),
            _ => (key, {
                let mut scene = crate::game_panel::GameScene::of(
                    &playing.world,
                    (&self.open.models, &self.open.brushes),
                    state,
                );
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

    /// Hands over the program's logo, for the top bar.
    pub fn set_logo(&mut self, logo: Option<egui::TextureId>) {
        self.logo = logo;
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

    /// The console's earlier commands, oldest first, for saving.
    pub fn console_history(&self) -> &[String] {
        self.console.history()
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
