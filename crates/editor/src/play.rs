//! Play mode: the area running as a game, beside the editor.
//!
//! Play keeps a copy of the document (the area, its undo history and the
//! selection; the history's steps are shared, so this is cheap) and builds the game's world from the area
//! ([`world::World`]). While it runs, the editor keeps working: changes
//! reach the game, which is built again from the area when it changes.
//! Stop throws the world away and puts the copy back, so everything is
//! exactly as it was before Play; the edits the user made meanwhile wait
//! for Keep or Undo all (`play_changes.rs`). Saving and opening wait
//! until then, and quitting stops the game and waits for that answer.

use crate::console::LogLevel;
use crate::document::{Document, FieldsSet};
use crate::game_code::{
    GameEdit, PlayWorld, ViewCache, failure_text, new_game, sync_all, sync_entities,
};
use crate::panel::Panel;
use crate::workbench::{Workbench, WorkbenchAction};
use area::assembly::AssemblyError;
use area::text::AreaFile;
use area::{Edit, EntityData};
use reflect::{AssetId, EntityId};
use std::collections::BTreeMap;
use std::sync::Arc;

/// Whether the game is running.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayState {
    /// Not playing: editing the area.
    Editing,
    /// The game is running.
    Playing,
    /// The game is running but its clock is stopped.
    Paused,
}

/// The game while it runs.
pub(crate) struct Playing {
    /// The game's entities.
    pub(crate) world: world::World,
    /// Whether its clock is stopped.
    pub(crate) paused: bool,
    /// Step was pressed: the next frame runs one tick while paused.
    pub(crate) step: bool,
    /// The document as it was when Play was pressed, put back on Stop.
    /// Only its file changes meanwhile: it follows the area's file when
    /// the Library moves or deletes it (`Workbench::file_moved`).
    pub(crate) before: Box<Document>,
    /// The assemblies as they were when Play was pressed, to rebuild
    /// copies from on Stop if any changed meanwhile.
    assemblies_before: BTreeMap<AssetId, Arc<AreaFile>>,
    /// The area as its file holds it now, if the file changed on disk
    /// while playing: offered again after Stop.
    pub(crate) reload: Option<area::Area>,
    /// The game's code running, if there is code (and it started), on
    /// every core (decision 0033).
    pub(crate) game: Option<script::Game>,
    /// What game code reads of the world's entities, kept between ticks.
    pub(crate) view: crate::game_code::ViewCache,
    /// Whether the Console was told projectiles hit their limit.
    told_too_many: bool,
    /// Whether the Console was told there were too many physics bodies.
    told_too_many_bodies: bool,
    /// Whether the Console was told 2D physics stopped and was started
    /// again (said once a play, however often it happens).
    told_flat_broke: bool,
    /// Assembly names game code spawned of which no copy could be made
    /// (none by that name, or one that can't be copied), each said once.
    told_unmade: std::collections::HashSet<String>,
    /// Whether the Console was told game code spawned too much.
    told_too_many_spawns: bool,
    /// What game code spawned and despawned that hasn't landed yet (from
    /// components' starts, which run outside the ticks).
    pub(crate) pending: Vec<GameEdit>,
    /// The input actions while playing (decision 0037): the project's at
    /// Play, changed by game code's `rebind`.
    pub(crate) bindings: Arc<input::Bindings>,
    /// The seed Play chose, before game code's starts (which may set
    /// another): what a recording keeps, so its replay starts the same.
    pub(crate) start_seed: u64,
    /// What this play records (Debug > Play and record inputs), if it
    /// records.
    pub(crate) recording: Option<input::Recording>,
    /// At most how big the recording's file is so far.
    pub(crate) recording_bytes: usize,
    /// Whether the Console has said an edit spoils this recording.
    pub(crate) told_recording_edited: bool,
    /// Whether the Console has said game code asked for too many sounds
    /// in a tick (said once a play).
    pub(crate) told_too_many_sounds: bool,
    /// The recording this play plays back, and how many of its ticks
    /// have run.
    pub(crate) replay: Option<(input::Recording, usize)>,
    /// How the last playback ended: matched on every tick (their count),
    /// or stopped matching by this tick (Phase 8, C4: a project's check).
    pub(crate) replayed: Option<Result<usize, usize>>,
    /// The fields the user changed in Properties while playing, each with
    /// its value from before Play (`None` where the entity didn't have
    /// it): marked there, with a way back (`04-properties-playing.png`).
    pub(crate) yours: BackToBefore,
}

/// Fields changed while playing, by entity, component and field, each
/// with its value from before Play.
pub(crate) type BackToBefore = BTreeMap<(EntityId, String, String), Option<reflect::Value>>;

impl Playing {
    /// Notes that the user changed a field while playing, keeping its
    /// value from before Play the first time. An entity the game made
    /// has no value from before, and isn't noted.
    pub(crate) fn note_yours(&mut self, entity: EntityId, component: &str, field: &str) {
        let Some(was) = self.before.area().get(entity) else {
            return;
        };
        let key = (entity, component.to_owned(), field.to_owned());
        self.yours.entry(key).or_insert_with(|| {
            was.components
                .get(component)
                .and_then(|c| c.get(field))
                .cloned()
        });
    }
}

/// What the Console says when 2D physics stopped on something and the
/// world started it again, naming the character whose move it stopped on
/// when that's known.
fn flat_broke_note(world: &world::World, broke: world::FlatBroke) -> String {
    let on = broke
        .on
        .and_then(|id| world.get::<world::Name>(id))
        .map(|name| format!(" (moving \"{}\")", name.0))
        .unwrap_or_default();
    format!(
        "2D physics stopped on something it couldn't work out{on}, and was started again from \
         the entities and tile maps as they are now, so the game carries on. If it keeps \
         stopping, it's started again at most once a second and 2D characters stand still in \
         between: look for a Collider2D or a tile map whose size or place is far out of the \
         ordinary. (Said once.)"
    )
}

/// The most ticks one recording keeps: an hour at 60 a second.
pub const RECORDING_TICKS: usize = 60 * 60 * 60;

/// The longest one tick of physics may take before the game pauses and
/// says why, like game code's tick limit.
pub const PHYSICS_LIMIT: std::time::Duration = std::time::Duration::from_millis(250);

/// The most slots game code saves and loads in one tick; more are left
/// out with a warning (a save is the whole game, so thousands in a tick,
/// say from every unit's update, would freeze Solis).
pub const MAX_SAVES_PER_TICK: usize = 4;

/// A tick's saves and loads (slot, whether it's a load) as they're made:
/// each slot and kind once, where it was last asked for, at most
/// [`MAX_SAVES_PER_TICK`]; and how many were left out past that.
pub(crate) fn collapse_saves(saves: Vec<(String, bool)>) -> (Vec<(String, bool)>, usize) {
    let mut seen = std::collections::HashSet::new();
    let mut kept: Vec<(String, bool)> = saves
        .into_iter()
        .rev()
        .filter(|s| seen.insert(s.clone()))
        .collect();
    kept.reverse();
    let dropped = kept.len().saturating_sub(MAX_SAVES_PER_TICK);
    kept.truncate(MAX_SAVES_PER_TICK);
    (kept, dropped)
}

/// Why a spawn made nothing.
enum Unmade {
    /// No assembly in the project has the name.
    NoSuchAssembly,
    /// This assembly can't be copied.
    Broken(AssetId, AssemblyError),
    /// The entity it was to be is in the area already.
    IdTaken,
}

/// The most copies of assemblies game code may make in one tick; more
/// aren't made, and the Console says so (a component spawning every tick by
/// mistake would otherwise grow the area until the computer runs out of
/// memory).
pub const MAX_SPAWNS_PER_TICK: usize = 10_000;

impl std::fmt::Debug for Playing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Playing")
            .field("world", &self.world)
            .field("paused", &self.paused)
            .finish_non_exhaustive()
    }
}

/// Whether `action` must wait until the game stops: it saves or replaces
/// the area (or writes an assembly), which would save the game's changes
/// or lose the copy kept for Stop.
pub(crate) fn waits_for_stop(action: &WorkbenchAction) -> bool {
    matches!(
        action,
        WorkbenchAction::NewArea
            | WorkbenchAction::OpenArea
            | WorkbenchAction::OpenAreaFile(_)
            | WorkbenchAction::SaveArea
            | WorkbenchAction::SaveAreaAs
            | WorkbenchAction::NewProject
            | WorkbenchAction::OpenProject
            | WorkbenchAction::OpenRecent(_)
            | WorkbenchAction::AddSampleArea
            | WorkbenchAction::MakeSampleGame
            | WorkbenchAction::MakeUiSample
            | WorkbenchAction::MakeBenchmark
            | WorkbenchAction::MakePlayground
            | WorkbenchAction::CreateAssembly(_)
            | WorkbenchAction::ApplyToAssembly { .. }
    )
}

impl Workbench {
    /// Whether the game is running, and paused.
    pub fn play_state(&self) -> PlayState {
        match &self.open.playing {
            None => PlayState::Editing,
            Some(p) if p.paused => PlayState::Paused,
            Some(_) => PlayState::Playing,
        }
    }

    /// The running game's world, if it's running.
    pub fn world(&self) -> Option<&world::World> {
        self.open.playing.as_ref().map(|p| &p.world)
    }

    /// Why Play can't be pressed now, if it can't: for its buttons to be
    /// greyed out saying so.
    pub(crate) fn why_not_play(&self) -> Option<&'static str> {
        if self.open.play_changes.is_some() {
            Some("Keep or undo the last play's changes first")
        } else if self.play_state() == PlayState::Playing {
            Some("Already playing")
        } else {
            None
        }
    }

    /// Play: runs the area as a game (or carries on, if paused). The
    /// Game panel comes to the front.
    pub fn play(&mut self) {
        // Changes from the last play wait for Keep or Undo all first.
        if self.open.play_changes.is_some() {
            self.log(
                LogLevel::Info,
                "Keep or undo the last play's changes first, then press Play.",
            );
            return;
        }
        if let Some(playing) = &mut self.open.playing {
            if playing.paused {
                playing.paused = false;
                self.log(LogLevel::Info, "Carrying on.");
            }
            return;
        }
        // A drag or a half-typed number ends here, so it isn't part of
        // both the editor's history and the game's.
        self.open.moving = None;
        self.open.document.close_step();
        // A new play: its sounds start afresh, and nothing of the last
        // play's waits to be heard in it.
        self.plays += 1;
        self.open.game_sounds.clear();
        self.open.quit_asked = false;
        let mut world = world::World::from_area(self.open.document.area());
        // A new seed each Play, unless one was chosen (`seed` in the
        // Console), so random numbers differ between plays but a play can
        // be repeated.
        let seed = self.open.fixed_seed.unwrap_or_else(fresh_seed);
        world.set_seed(seed);
        let _ = self.open.document.take_changes();
        let cameras = world.placed::<area::builtin::Camera>().len();
        let mut view = ViewCache::default();
        view.settings = self.player_settings;
        let (game, pending) = self.start_game(&mut world, &mut view);
        self.open.playing = Some(Playing {
            world,
            paused: false,
            step: false,
            before: Box::new(self.open.document.clone()),
            assemblies_before: self.open.assemblies.clone(),
            reload: None,
            game,
            view,
            told_too_many: false,
            told_too_many_bodies: false,
            told_flat_broke: false,
            told_unmade: Default::default(),
            told_too_many_spawns: false,
            pending,
            // The menu actions even in a project whose input actions file
            // is from before them, so its screens' buttons can be reached
            // with the keys (decision 0088).
            bindings: Arc::new(self.open.bindings.clone().with_menu_actions()),
            start_seed: seed,
            recording: None,
            recording_bytes: 0,
            told_recording_edited: false,
            told_too_many_sounds: false,
            replay: None,
            replayed: None,
            yours: BTreeMap::new(),
        });
        // Every edit made from here is kept as an undo step until Stop,
        // which makes them again on the copy just kept, for Keep.
        self.open.document.hold_steps();
        // Nothing held over from before Play.
        self.devices = input::Devices::new();
        // The Game tab comes to the front of the area's group (the
        // project lead's answer 10), on Home and on Debug.
        use crate::shell::RibbonTab;
        for tab in [RibbonTab::Home, RibbonTab::Build, RibbonTab::Debug] {
            self.show_in(tab, crate::shell::workspaces::Home::Group(0), Panel::Game);
        }
        let note = if cameras == 0 {
            " There's no camera in the area, so the Game panel shows the editor's view \
             (Home > Camera adds one)."
        } else {
            ""
        };
        self.log(
            LogLevel::Info,
            format!(
                "Playing. Stop puts the area back as it is now, and asks whether to keep the \
                 changes you make meanwhile.{note}"
            ),
        );
    }

    /// The game's code started on `world`, with every entity's game
    /// components (Lust, Lua and Rust) made from the area; `None` with no
    /// code. If the code's start fails (said in the Console), the Rust
    /// components run on their own.
    pub(crate) fn start_game(
        &mut self,
        world: &mut world::World,
        view: &mut ViewCache,
    ) -> (Option<script::Game>, Vec<GameEdit>) {
        self.start_game_saying(world, view, true)
    }

    /// As [`Self::start_game`]; `say` false keeps what the starts say out
    /// of the Console (their failures are still told): for starts run on a
    /// world thrown away after, as loading a save does.
    pub(crate) fn start_game_saying(
        &mut self,
        world: &mut world::World,
        view: &mut ViewCache,
        say: bool,
    ) -> (Option<script::Game>, Vec<GameEdit>) {
        if self.open.program.is_none() && self.open.rust_components.is_empty() {
            return (None, Vec::new());
        }
        let mut game = match new_game(&self.open.rust_components) {
            Ok(game) => game,
            Err(failure) => {
                self.tell_game(Vec::new(), &[failure]);
                return (None, Vec::new());
            }
        };
        game.leave_out(self.open.left_out.clone());
        let mut host = PlayWorld::new(world, view, 0.0);
        let mut failures = Vec::new();
        // If the code's start fails, the rest of the game (its Rust
        // components) still runs.
        if let Some(chunk) = self.open.program.clone()
            && let Err(mut failure) = game.load(&chunk, &mut host)
        {
            failure.at.message = format!(
                "the game's code stopped while starting, so its Lust and Lua components don't \
                 run: {}",
                failure.at.message
            );
            failures.push(failure);
        }
        failures.extend(sync_all(
            &mut game,
            self.open.document.area(),
            &FieldsSet::new(),
            &mut host,
        ));
        host.outside_a_tick();
        let said = std::mem::take(&mut host.said);
        // What the starts spawned lands before the first tick.
        let pending = std::mem::take(&mut host.edits);
        let sounds = std::mem::take(&mut host.sounds);
        drop(host);
        // Sounds the starts asked for (music, say) play from the start; a
        // world thrown away after (loading a save) plays none.
        if say {
            let (tick, seed) = (world.clock().ticks, world.seed());
            crate::game_sound::gather(&mut self.open.game_sounds, sounds, tick, seed);
        }
        let said = if say { said } else { Vec::new() };
        self.tell_game(said, &failures);
        if say {
            self.tell_unkept_object_fields(&mut game);
        }
        (Some(game), pending)
    }

    /// Says, once a play, which fields of game-wide objects aren't kept in
    /// saves or shared between cores: lists, maps and things (decision
    /// 0080).
    fn tell_unkept_object_fields(&mut self, game: &mut script::Game) {
        let (_, unkept) = game.object_info();
        if unkept.is_empty() {
            return;
        }
        let fields: Vec<String> = unkept.iter().map(|(o, f)| format!("{o}.{f}")).collect();
        self.log(
            LogLevel::Warning,
            format!(
                "{} can't be kept in save games yet, and each of the computer's cores keeps its \
                 own copy (only plain fields, like numbers, text and states, are copied to every \
                 core). Keep what every part of the game must see in plain fields.",
                fields.join(", ")
            ),
        );
    }

    /// Writes what game code said, and its failures, in the Console.
    pub(crate) fn tell_game(&mut self, said: Vec<String>, failures: &[script::Failure]) {
        for words in said {
            self.log(LogLevel::Info, words);
        }
        for failure in failures {
            let text = failure_text(self.open.document.area(), failure);
            self.log(LogLevel::Error, text);
        }
    }

    /// Pause: stops the game's clock, or starts it again.
    pub fn pause(&mut self) {
        let Some(playing) = &mut self.open.playing else {
            self.log(LogLevel::Info, "Nothing is running.");
            return;
        };
        playing.paused = !playing.paused;
        let text = if playing.paused {
            "Paused."
        } else {
            "Carrying on."
        };
        self.log(LogLevel::Info, text);
    }

    /// Step: while paused, runs exactly one tick (game code, projectiles
    /// and all), then stays paused.
    pub fn step(&mut self) {
        match &mut self.open.playing {
            Some(playing) if playing.paused => playing.step = true,
            Some(_) => self.log(
                LogLevel::Info,
                "Pause first, then Step runs one tick at a time.",
            ),
            None => self.log(LogLevel::Info, "Nothing is running."),
        }
    }

    /// Stop: throws the game away and puts the area, its undo history
    /// and the selection back as they were when Play was pressed.
    /// Assemblies changed on disk meanwhile update their copies again,
    /// and an area file changed on disk is offered again.
    pub fn stop(&mut self) {
        let Some(playing) = self.open.playing.take() else {
            self.log(LogLevel::Info, "Nothing is running.");
            return;
        };
        let seconds = playing.world.clock().time;
        self.open.game_sounds.clear();
        self.open.game_screens.clear();
        self.open.moving = None;
        self.give_back_input();
        if let Some(recording) = playing.recording.filter(|r| !r.ticks.is_empty()) {
            self.log(
                LogLevel::Info,
                format!(
                    "Recorded {} ticks ({:.1} s) of input; saving it in the project's recordings \
                     folder.",
                    recording.ticks.len(),
                    recording.ticks.len() as f64 * world::TICK
                ),
            );
            self.open
                .outbox
                .push(crate::workbench::WorkbenchAction::SaveRecording(
                    recording.write(),
                ));
        }
        // The user's changes to the area while playing wait for Keep or
        // Undo all (decision 0085): never thrown away silently. Not the
        // game's: what it spawned, took out or loaded goes with it.
        let asked = self.note_play_changes(&playing.before, &playing.assemblies_before);
        // The area as it was, but the components known now: code that
        // changed while playing keeps its components in the registry.
        self.replace_document(*playing.before);
        self.open.document.touch();
        let now = std::mem::replace(&mut self.open.assemblies, playing.assemblies_before);
        self.log(
            LogLevel::Info,
            if asked {
                format!("Stopped after {seconds:.1} s; keep the changes made while playing?")
            } else {
                format!("Stopped after {seconds:.1} s; the area is back as it was.")
            },
        );
        self.update_copies(now);
        if let Some(area) = playing.reload {
            self.offer_reload(area);
        }
    }

    /// Makes the assemblies `now` (as their files are now) known, in
    /// place of the ones known (as they were at Play, which the open
    /// area's copies were built from): the copies of each that changed
    /// meanwhile are rebuilt.
    pub(crate) fn update_copies(&mut self, now: BTreeMap<AssetId, Arc<AreaFile>>) {
        for (id, file) in now {
            let changed = self
                .open
                .assemblies
                .get(&id)
                .is_none_or(|old| !Arc::ptr_eq(old, &file));
            if changed {
                self.set_assembly(id, file);
            }
        }
    }

    /// Moves the running game on by `seconds` (each frame does this; for
    /// tests and tools that run the game without a window).
    pub fn play_for(&mut self, seconds: f32) {
        self.run_game(seconds);
    }

    /// Moves the game on by `seconds`, taking in what changed in the
    /// area: only the entities changed, or, when entities were added,
    /// deleted or moved, everything. Called once a frame.
    pub(crate) fn run_game(&mut self, seconds: f32) {
        let started = std::time::Instant::now();
        // Game code's share, timed apart; the rest is the world's.
        let mut code = std::time::Duration::ZERO;
        let changes = self.open.document.take_changes();
        let Some(playing) = &mut self.open.playing else {
            return;
        };
        let area = self.open.document.area();
        if changes.everything {
            playing.world.rebuild(area);
        } else if !changes.entities.is_empty() {
            playing.world.update(
                area,
                changes
                    .entities
                    .iter()
                    .map(|&id| (id, changes.moved.contains(&id))),
            );
        }
        // A field set by hand takes the area's value even where that's
        // what the area held all along and game code had set its own
        // meanwhile: for Solis's own components here, as for game code's
        // below. Everything else game code set on the entity stays.
        for ((id, component), fields) in &changes.set {
            for field in fields {
                playing.world.set_by_hand(*id, component, field);
            }
        }
        let mut said = Vec::new();
        let mut failures = Vec::new();
        let mut dropped_sounds = 0usize;
        // An edit isn't in the recording, so its replay won't match from
        // here: said once.
        if !changes.is_empty() && playing.recording.is_some() && !playing.told_recording_edited {
            playing.told_recording_edited = true;
            said.push(
                "The area changed while recording: a recording keeps only the input, so its \
                 replay will stop matching from here."
                    .to_string(),
            );
        }
        // Changes made in the editor reach the running components.
        if let Some(game) = &mut playing.game
            && !changes.is_empty()
        {
            let time = playing.world.clock().time;
            let (tick, seed) = (playing.world.clock().ticks, playing.world.seed());
            let mut host = PlayWorld::new(&mut playing.world, &mut playing.view, time);
            host.frame_time = f64::from(seconds);
            let syncing = std::time::Instant::now();
            // With the fields set by hand, which a running component
            // takes whatever the area held (Properties shows the game's
            // value, so setting it to the area's is a change too).
            if changes.everything {
                failures.extend(sync_all(game, area, &changes.set, &mut host));
            } else {
                failures.extend(sync_entities(
                    game,
                    area,
                    changes.entities.iter().copied(),
                    &changes.set,
                    &mut host,
                ));
            }
            code += syncing.elapsed();
            host.outside_a_tick();
            said.append(&mut host.said);
            // Sounds asked for as components start (music, say) play
            // with the first tick's.
            let sounds = std::mem::take(&mut host.sounds);
            dropped_sounds +=
                crate::game_sound::gather(&mut self.open.game_sounds, sounds, tick, seed);
            // What their starts spawned lands before the next tick.
            let edits = std::mem::take(&mut host.edits);
            playing.pending.extend(edits);
        }
        let mut pause = None;
        let mut flat_broke = None;
        let ran = if playing.paused {
            // Step: one tick, then paused again.
            u64::from(std::mem::take(&mut playing.step))
        } else {
            playing.world.due(f64::from(seconds))
        };
        if ran == 0 && playing.paused {
            // Presses and mouse movement while paused don't wait for the
            // game to go on (the camera would jump).
            self.devices.forget_changes();
        }
        for _ in 0..ran {
            // The clock moves on as the tick begins, so a frame that stops
            // early never counts ticks that didn't run.
            let Some(playing) = &mut self.open.playing else {
                break;
            };
            playing.world.begin_tick();
            let time = playing.world.clock().time;
            // What earlier starts spawned joins first; then projectiles
            // fly on, then game code sees where they are and fires more.
            let landing = std::time::Instant::now();
            failures.extend(self.land_edits(time, seconds));
            code += landing.elapsed();
            // Game code showed or hid a screen last tick: the screens run
            // again now, so whether a menu has the keys this tick follows
            // the game itself, however many ticks a frame runs (the phase
            // review: a replay could otherwise stop matching).
            if self.screens_out_of_step() {
                self.refresh_game_screens();
            }
            let Some(playing) = &mut self.open.playing else {
                break;
            };
            // Where everything is as the tick starts: what drawing between
            // ticks moves from.
            playing.world.remember();
            let dt = playing.world.tick_time() as f32;
            playing.world.projectiles_mut().tick(dt);
            // Sprites' clips move on (Phase 9, D3): the events they pass
            // play their sound events and reach game code first this tick.
            playing
                .world
                .set_clip_sets(Arc::clone(&self.viewport.sheets.clip_sets));
            // Tile maps (D4): the project's files, and a map for each new
            // TileMap (one game code spawned, say).
            playing
                .world
                .set_tile_maps(Arc::clone(&self.viewport.sheets.map_set));
            playing.world.start_maps();
            // Game code names maps by their files' paths (decision 0079).
            playing.view.set_map_paths(&self.viewport.sheets);
            // 2D physics (decision 0071): pixels to metres, and which
            // terrains are solid, as they are now.
            playing.world.set_pixels(self.viewport.pixels);
            playing
                .world
                .set_solid_sets(Arc::new(self.viewport.sheets.solid_sets()));
            // Animators first (D3b): each takes the transition the
            // parameters set last tick call for, and sets its sprite's clip.
            playing
                .world
                .set_animators(Arc::clone(&self.viewport.sheets.animator_set));
            playing.world.step_animators();
            let cues = playing.world.step_sprites(playing.world.tick_time());
            let (cue_sounds, engine_events) = cues_of(
                &playing.world,
                cues,
                &self.open.cue_sounds,
                &self.viewport.sheets.paths,
            );
            // The tick's input: played back, or sampled from the devices
            // once, so every core sees the same (decision 0037).
            let sampled = self.devices.sample(&playing.bindings, world::TICK as f32);
            let frame = match &playing.replay {
                // The devices are still sampled (and the sample dropped),
                // so nothing pressed during a replay piles up for after it.
                Some((recording, at)) => recording
                    .ticks
                    .get(*at)
                    .map(|t| t.frame.clone())
                    .unwrap_or_default(),
                None => sampled,
            };
            // What screens' buttons sent since the last tick is part of its
            // input (decision 0088), so a replay sends it again; during a
            // replay, the recording's own.
            let mut frame = frame;
            // While a screen with buttons is shown, the menu keys move
            // its focus and choose, and the game's own actions wait; what
            // game code sees (and a recording keeps) is that frame.
            let chosen = self.open.game_screens.take_menu_keys(&mut frame);
            let sent = std::mem::take(&mut self.open.game_screens.sent);
            if playing.replay.is_none() {
                frame.sent = sent;
            }
            let mut engine_events = engine_events;
            engine_events.extend(frame.sent.iter().map(|s| script::Event {
                to: s.to,
                name: s.name.clone(),
                values: s.values.clone(),
            }));
            let frame = Arc::new(frame);
            let mut slow = None;
            let mut saves = Vec::new();
            let (sound_tick, sound_seed) = (playing.world.clock().ticks, playing.world.seed());
            dropped_sounds += crate::game_sound::gather(
                &mut self.open.game_sounds,
                cue_sounds,
                sound_tick,
                sound_seed,
            );
            // Game code's dt at the game's speed (0 paused, M3c).
            let dt = playing.world.tick_time();
            if let Some(game) = &mut playing.game {
                let mut host = PlayWorld::new(&mut playing.world, &mut playing.view, time);
                host.frame_time = f64::from(seconds);
                host.input = Arc::clone(&frame);
                host.bindings = Arc::clone(&playing.bindings);
                let ticking = std::time::Instant::now();
                let ticked = game.tick_with_events(dt, &mut host, engine_events);
                code += ticking.elapsed();
                failures.extend(ticked.failures);
                said.append(&mut host.said);
                // What this tick spawned and despawned joins the world now,
                // in the order it was asked for (decision 0035), so the next
                // tick sees it however many ticks this frame runs.
                let edits = std::mem::take(&mut host.edits);
                playing.pending.extend(edits);
                saves = std::mem::take(&mut host.saves);
                let sounds = std::mem::take(&mut host.sounds);
                dropped_sounds += crate::game_sound::gather(
                    &mut self.open.game_sounds,
                    sounds,
                    sound_tick,
                    sound_seed,
                );
                let seed = host.seed.take();
                // Rebinding takes effect from the next tick. The tick's
                // own share of the bindings goes first, so they're changed
                // in place rather than copied.
                let rebinds = std::mem::take(&mut host.rebinds);
                drop(host);
                for (action, input) in rebinds {
                    Arc::make_mut(&mut playing.bindings).rebind(&action, &input);
                }
                slow = ticked.pause;
                let landing = std::time::Instant::now();
                failures.extend(self.land_edits(time, seconds));
                code += landing.elapsed();
                // A new seed holds from the next tick (after the starts of
                // what this tick spawned, which belong to this one).
                if let (Some(seed), Some(playing)) = (seed, &mut self.open.playing) {
                    playing.world.set_seed(seed);
                }
            }
            // A button the menu keys chose runs now; what it sends is the
            // next tick's.
            if let Some(button) = chosen {
                self.press(button);
            }
            // Physics moves on last, with game code's moves and pushes
            // (decision 0036).
            let Some(playing) = &mut self.open.playing else {
                break;
            };
            let stepping = std::time::Instant::now();
            playing.world.step_physics();
            if playing.world.take_physics_broke() && slow.is_none() {
                slow = Some(
                    "Physics stopped working: something in it is out of the range physics can \
                     work with (a collider or body thousands of kilometres big or away), so the \
                     game has no physics now. Fix its size or place, then Stop and play again."
                        .to_string(),
                );
            }
            // 2D physics stopped too, and the world started it again: the
            // game carries on, and the Console says why once a play.
            if let Some(broke) = playing.world.take_flat_broke()
                && !playing.told_flat_broke
            {
                playing.told_flat_broke = true;
                flat_broke = Some(flat_broke_note(&playing.world, broke));
            }
            if slow.is_none() && stepping.elapsed() > PHYSICS_LIMIT {
                slow = Some(format!(
                    "Physics took {:.0} ms for one tick (more than {} ms), so the game is paused \
                     rather than falling further behind. Fewer bodies, or simpler colliders, make \
                     it quicker.",
                    stepping.elapsed().as_secs_f64() * 1000.0,
                    PHYSICS_LIMIT.as_millis()
                ));
            }
            // Recording and playing back: the tick's input and checksums.
            if playing.recording.is_some() || playing.replay.is_some() {
                let ticks = playing.world.clock().ticks;
                let tick = input::Tick {
                    frame: (*frame).clone(),
                    world: playing.world.checksum(),
                    // Game code's every second: asking every component
                    // costs.
                    game: (ticks % 60 == 0)
                        .then(|| playing.game.as_mut().map_or(0, script::Game::checksum)),
                };
                if let Some(recording) = &mut playing.recording {
                    let bytes = playing.recording_bytes + tick.written_len();
                    // Within what Solis reads back (header and all).
                    if recording.ticks.len() < RECORDING_TICKS
                        && bytes + 256 < input::MAX_RECORDING_BYTES
                    {
                        playing.recording_bytes = bytes;
                        recording.ticks.push(tick);
                    } else if slow.is_none() {
                        slow = Some(
                            "The recording reached the most Solis keeps (an hour, or 64 MB), so \
                             the game is paused; Stop saves what was recorded."
                                .to_string(),
                        );
                    }
                } else if let Some((recording, at)) = &mut playing.replay {
                    let n = *at;
                    *at += 1;
                    let ended = *at >= recording.ticks.len();
                    let count = recording.ticks.len();
                    let mismatch = recording.ticks.get(n).is_some_and(|expected| {
                        expected.world != tick.world
                            || matches!((expected.game, tick.game), (Some(a), Some(b)) if a != b)
                    });
                    let outcome = if mismatch {
                        Some(format!(
                            "The replay stopped matching the recording by tick {n} ({:.2} s in; \
                             game code's own values are compared once a second, so it may have \
                             started a little before): the game came out differently from the \
                             same input. Something in it \
                             isn't deterministic (frameTime(), which is real time, is a usual \
                             one), or the area or code changed since recording.",
                            n as f64 * world::TICK
                        ))
                    } else if ended {
                        Some(format!(
                            "The replay matched the recording on all {count} ticks: the same \
                             input gave exactly the same game."
                        ))
                    } else {
                        None
                    };
                    if outcome.is_some() {
                        playing.replayed = Some(if mismatch { Err(n) } else { Ok(count) });
                        playing.replay = None;
                        slow = slow.or(outcome);
                    }
                }
            }
            // Too slow: no catching up, which would only be slower; the
            // game waits for the person instead (after this tick's saves).
            if slow.is_some() {
                playing.paused = true;
                pause = slow;
            }
            // Saves and loads, at the end of the tick, in the order asked
            // (decision 0038); the same slot asked for again in a tick is
            // done once, as last asked. A load replaces the game, so no
            // more ticks run before it.
            let (saves, dropped) = collapse_saves(saves);
            // What this tick's spawned components' starts spawned in turn
            // would join next tick: it joins now, so a save holds it (a
            // few rounds at most; past that, it's left for the next tick).
            if saves.iter().any(|(_, load)| !load) {
                for _ in 0..8 {
                    if self
                        .open
                        .playing
                        .as_ref()
                        .is_none_or(|p| p.pending.is_empty())
                    {
                        break;
                    }
                    failures.extend(self.land_edits(time, seconds));
                }
            }
            if dropped > 0 {
                self.log(
                    LogLevel::Warning,
                    format!(
                        "Game code asked to save or load in more than {MAX_SAVES_PER_TICK} slots \
                         in one tick; the other {dropped} were left out."
                    ),
                );
            }
            let mut loading = false;
            let replaying = self
                .open
                .playing
                .as_ref()
                .is_some_and(|p| p.replay.is_some());
            let mut saved_text: Option<Option<String>> = None;
            for (slot, load) in saves {
                if load {
                    self.open.outbox.push(WorkbenchAction::LoadGame(slot));
                    loading = true;
                } else {
                    // Every slot saved this tick holds the same moment:
                    // made once.
                    let text = match &saved_text {
                        Some(text) => text.clone(),
                        None => {
                            let text = self.save_text();
                            saved_text = Some(text.clone());
                            text
                        }
                    };
                    // A replay makes the save as the recorded play did (it
                    // builds physics again, so the game goes on alike),
                    // but doesn't write over the player's saves.
                    if let Some(text) = text
                        && !replaying
                    {
                        self.open
                            .outbox
                            .push(WorkbenchAction::SaveGame { slot, text });
                    }
                }
            }
            // `Window:quit()`: no more ticks.
            let quit = self.open.playing.as_ref().is_some_and(|p| p.view.quit);
            if loading || pause.is_some() || quit {
                break;
            }
        }
        let Some(playing) = &mut self.open.playing else {
            return;
        };
        let refused = playing.world.projectiles().refused() > 0 && !playing.told_too_many;
        if refused {
            playing.told_too_many = true;
        }
        // The player's settings game code changed (decision 0091): kept
        // for the next play; the window's only said, since the editor's
        // window isn't the game's.
        self.player_settings = playing.view.settings;
        let asked = std::mem::take(&mut playing.view.asked);
        // An exported game's window does change (the app follows the
        // settings), so there's nothing to say.
        for asked in asked.into_iter().filter(|_| !self.play_only) {
            use script::settings::Setting;
            match asked {
                Setting::Fullscreen(on) => said.push(format!(
                    "Window:setFullscreen({on}): the exported game would {}. While playing in the \
                     editor, the game stays in the Game panel.",
                    if on {
                        "fill the screen"
                    } else {
                        "sit in a window"
                    }
                )),
                Setting::Resolution(..) => {
                    let [w, h] = self.player_settings.resolution;
                    said.push(format!(
                        "Window:setResolution: the exported game's window would be {w} × {h}. \
                         While playing in the editor, the game stays in the Game panel."
                    ));
                }
                Setting::TextSize(_) | Setting::UiSize(_) => {}
            }
        }
        let bodies_refused = playing.world.physics_refused() > 0 && !playing.told_too_many_bodies;
        if bodies_refused {
            playing.told_too_many_bodies = true;
        }
        self.profiler.add(crate::profiler::Stage::GameCode, code);
        self.profiler.add(
            crate::profiler::Stage::World,
            started.elapsed().saturating_sub(code),
        );
        let quit = std::mem::take(&mut playing.view.quit);
        if !said.is_empty() || !failures.is_empty() {
            self.tell_game(said, &failures);
        }
        // `Window:quit()` (decision 0093): the exported game closes; in
        // the editor, play stops.
        if quit {
            if !self.play_only {
                self.log(
                    LogLevel::Info,
                    "Window:quit(): the exported game would close here, so play stops.",
                );
            }
            self.open.quit_asked = true;
            self.stop();
            return;
        }
        // The game's screens read what this frame's ticks left (decision
        // 0087), and start again when the code changes.
        if ran > 0
            || self
                .open
                .game_screens
                .out_of_date(self.open.program.as_ref())
        {
            self.refresh_game_screens();
        }
        if let Some(why) = pause {
            // A replay that matched is good news, not a warning.
            let level = if why.starts_with("The replay matched") {
                LogLevel::Info
            } else {
                LogLevel::Warning
            };
            self.log(level, format!("{why} Press Pause or Play to carry on."));
        }
        if dropped_sounds > 0
            && let Some(playing) = &mut self.open.playing
            && !playing.told_too_many_sounds
        {
            playing.told_too_many_sounds = true;
            self.log(
                LogLevel::Warning,
                format!(
                    "Game code asked for more than {} sounds in one tick; the rest were left out \
                     (said once).",
                    crate::game_sound::MAX_PER_TICK
                ),
            );
        }
        if refused {
            self.log(
                LogLevel::Warning,
                format!(
                    "More than {} projectiles were flying at once, so game code's next shots \
                     weren't fired until some ran out of time.",
                    crate::models::grouped(world::MAX_PROJECTILES)
                ),
            );
        }
        if let Some(note) = flat_broke {
            self.log(LogLevel::Warning, note);
        }
        if bodies_refused {
            self.log(
                LogLevel::Warning,
                format!(
                    "The game has the most physics bodies it can ({}), so the rest have no physics: \
                     they don't fall or collide.",
                    crate::models::grouped(world::physics::MAX_BODIES)
                ),
            );
        }
    }

    /// Lands what game code spawned and despawned since it was last
    /// landed (`Playing::pending`), in the order it was asked for: copies
    /// of assemblies join the area at the end of its tree and the world
    /// at once, despawned entities leave both, and the running game's
    /// components start and stop with them. Changes to the area that
    /// aren't undo steps (Stop puts the area back; decision 0035). A spawn
    /// that can't be made (no assembly by that name, or one that can't be
    /// copied) says why in the Console, once a play for each name, never
    /// once a frame (the review, BUG-51). The failures of starting the
    /// new components come back.
    fn land_edits(&mut self, time: f64, seconds: f32) -> Vec<script::Failure> {
        let edits = match &mut self.open.playing {
            Some(playing) if !playing.pending.is_empty() => std::mem::take(&mut playing.pending),
            _ => return Vec::new(),
        };
        // Why nothing was made, for each name asked for in vain: the first
        // reason this tick, however many times it was asked.
        let mut unmade: BTreeMap<String, Unmade> = BTreeMap::new();
        // Each name looked up once, not once a spawn.
        let mut named: std::collections::HashMap<String, Option<(AssetId, String)>> =
            std::collections::HashMap::new();
        let mut room = area::text::MAX_ENTITIES.saturating_sub(self.open.document.area().len());
        let mut refused = 0usize;
        let mut spawned = 0usize;
        // At the end of the area's tree, one after another.
        let mut next_order = self.open.document.area().order_at_end(None);
        let mut added: Vec<EntityId> = Vec::new();
        let mut removed: Vec<EntityId> = Vec::new();
        let mut removed_roots: Vec<EntityId> = Vec::new();
        // One expander for the tick: each assembly is expanded once, and
        // its size known before a copy is made, so thousands of spawns of
        // one too big to fit cost nothing.
        let assemblies = &self.open.assemblies;
        let lookup = |a| assemblies.get(&a).cloned();
        let mut expander = area::assembly::Expander::new(&lookup);
        for edit in edits {
            match edit {
                GameEdit::Spawn(id, name, place) => {
                    if spawned >= MAX_SPAWNS_PER_TICK || room == 0 {
                        refused += 1;
                        continue;
                    }
                    let found = named
                        .entry(name.clone())
                        .or_insert_with(|| self.assembly_named(&name))
                        .clone();
                    let Some((asset, shown)) = found else {
                        unmade.entry(name).or_insert(Unmade::NoSuchAssembly);
                        continue;
                    };
                    let size = match expander.size_of(asset) {
                        Ok(size) => size,
                        Err(why) => {
                            unmade.entry(name).or_insert(Unmade::Broken(asset, why));
                            continue;
                        }
                    };
                    if size > room {
                        refused += 1;
                        continue;
                    }
                    let data = EntityData::new(&shown);
                    let mut entities = match expander.place(asset, id, &data) {
                        Ok(entities) => entities,
                        Err(why) => {
                            unmade.entry(name).or_insert(Unmade::Broken(asset, why));
                            continue;
                        }
                    };
                    // The copy stands where it was spawned, turned and sized
                    // as the assembly made it, after everything else.
                    if let Some((_, root)) = entities.iter_mut().find(|(e, _)| *e == id) {
                        let made = root.get::<area::builtin::Transform>().unwrap_or_default();
                        root.set(&area::builtin::Transform {
                            position: place,
                            ..made
                        });
                        root.parent = None;
                        root.order = next_order.clone();
                    }
                    if entities.len() > room {
                        refused += 1;
                        continue;
                    }
                    let failed = self.open.document.edit_by_game(vec![Edit::Spawn {
                        entities: entities.clone(),
                    }]);
                    if failed > 0 {
                        unmade.entry(name).or_insert(Unmade::IdTaken);
                        continue;
                    }
                    room -= entities.len();
                    spawned += 1;
                    next_order = area::order_after(Some(&next_order));
                    if let Some(playing) = &mut self.open.playing {
                        added.extend(playing.world.add_subtree(&entities));
                    }
                }
                GameEdit::Despawn(id) => {
                    if !self.open.document.area().contains(id) {
                        continue;
                    }
                    let under = match &self.open.playing {
                        Some(playing) => playing.world.subtree(id),
                        None => Vec::new(),
                    };
                    if self
                        .open
                        .document
                        .edit_by_game(vec![Edit::Despawn { root: id }])
                        == 0
                    {
                        removed.extend(under);
                        removed_roots.push(id);
                    }
                }
            }
        }
        let mut failures = Vec::new();
        let mut told = Vec::new();
        if let Some(playing) = &mut self.open.playing {
            playing.world.remove_subtrees(&removed_roots);
            let gone: std::collections::HashSet<EntityId> = removed.iter().copied().collect();
            added.retain(|id| !gone.contains(id));
            if let Some(game) = &mut playing.game {
                game.retain_on(removed.iter().map(|&id| (id, Vec::new())));
                if !added.is_empty() {
                    let (tick, seed) = (playing.world.clock().ticks, playing.world.seed());
                    let mut host = PlayWorld::new(&mut playing.world, &mut playing.view, time);
                    host.frame_time = f64::from(seconds);
                    failures = sync_entities(
                        game,
                        self.open.document.area(),
                        added,
                        &FieldsSet::new(),
                        &mut host,
                    );
                    host.outside_a_tick();
                    told = std::mem::take(&mut host.said);
                    // Sounds the new components' starts asked for.
                    let sounds = std::mem::take(&mut host.sounds);
                    crate::game_sound::gather(&mut self.open.game_sounds, sounds, tick, seed);
                    // What the new components' starts spawn waits for the
                    // next landing.
                    let more = std::mem::take(&mut host.edits);
                    playing.pending.extend(more);
                }
            }
        }
        for words in told {
            self.log(LogLevel::Info, words);
        }
        if refused > 0
            && let Some(playing) = &mut self.open.playing
            && !playing.told_too_many_spawns
        {
            playing.told_too_many_spawns = true;
            self.log(
                LogLevel::Warning,
                format!(
                    "Game code spawned more than {} copies in one tick, or the area reached \
                     its most entities ({}), so {refused} weren't made (is something spawning \
                     every tick?).",
                    crate::models::grouped(MAX_SPAWNS_PER_TICK),
                    crate::models::grouped(area::text::MAX_ENTITIES)
                ),
            );
        }
        for (name, why) in unmade {
            let Some(playing) = &mut self.open.playing else {
                break;
            };
            // A few names are remembered; past that, nothing more is said.
            if playing.told_unmade.len() < 100 && playing.told_unmade.insert(name.clone()) {
                let why = self.why_unmade(&why);
                self.log(
                    LogLevel::Warning,
                    format!("Game code spawned \"{name}\", but {why}, so nothing was made."),
                );
            }
        }
        failures
    }

    /// Why a spawn made nothing, in words for the Console: files by their
    /// names in the Library, where it knows them.
    fn why_unmade(&self, why: &Unmade) -> String {
        let named = |id: AssetId| {
            let listed = self.open.library.iter().find(|item| item.id == Some(id));
            listed.map(|item| format!("\"{}\"", item.name))
        };
        let (asset, why) = match why {
            Unmade::NoSuchAssembly => {
                return "there's no assembly by that name in the project's assets".into();
            }
            Unmade::IdTaken => return "the entity it was to be is already in the area".into(),
            Unmade::Broken(asset, why) => (*asset, why),
        };
        match why {
            AssemblyError::Missing(id) => match named(*id) {
                Some(name) if *id == asset => format!("{name}'s file can't be read"),
                Some(name) => format!("it places {name}, whose file can't be read"),
                None => format!("it places an assembly ({id}) that isn't in the project"),
            },
            AssemblyError::NotAnAssembly(id) if *id == asset => {
                "its file is an area, not an assembly".into()
            }
            AssemblyError::NotAnAssembly(id) => format!(
                "it places {}, which is an area, not an assembly",
                named(*id).unwrap_or_else(|| id.to_string())
            ),
            AssemblyError::Loop(chain) => {
                let chain: Vec<String> = chain
                    .iter()
                    .map(|&id| named(id).unwrap_or_else(|| id.to_string()))
                    .collect();
                format!(
                    "its assemblies place each other in a loop ({})",
                    chain.join(" → ")
                )
            }
            AssemblyError::TooDeep => format!(
                "its assemblies are placed more than {} deep, one inside another",
                area::text::MAX_NESTING
            ),
            AssemblyError::TooBig => format!(
                "a copy would be more than {} entities",
                crate::models::grouped(area::text::MAX_ENTITIES)
            ),
            AssemblyError::Clash(id) => {
                format!("two of its entities would share an ID ({id})")
            }
        }
    }

    /// The assembly game code names `name`: by its file in the assets
    /// (`ships/fighter.solasm`), that without the extension, or just its
    /// name (`fighter`, when only one has it); its asset and name.
    fn assembly_named(&self, name: &str) -> Option<(AssetId, String)> {
        let assemblies = || {
            self.open
                .library
                .iter()
                .filter(|i| i.kind == crate::library::LibraryKind::Assembly)
                .filter_map(|i| i.id.map(|id| (id, i)))
                .filter(|(id, _)| self.open.assemblies.contains_key(id))
        };
        let wanted = name.trim_start_matches('/');
        let by_path = assemblies().find(|(_, i)| {
            i.path.eq_ignore_ascii_case(wanted)
                || i.path
                    .rsplit_once('.')
                    .is_some_and(|(stem, _)| stem.eq_ignore_ascii_case(wanted))
        });
        let found = by_path.or_else(|| {
            let mut named = assemblies().filter(|(_, i)| i.name.eq_ignore_ascii_case(wanted));
            let first = named.next();
            first.filter(|_| named.next().is_none())
        });
        found.map(|(id, i)| (id, i.name.clone()))
    }

    /// Whether the user changed the area since Play was pressed: Stop
    /// would ask Keep or Undo all.
    pub(crate) fn edited_while_playing(&self) -> bool {
        self.open.playing.as_ref().is_some_and(|playing| {
            self.open.document.history().state() != playing.before.history().state()
        })
    }

    /// Takes out of `actions` what must wait until the game stops, saying
    /// so in the Console. Quitting stops the game first, and waits for
    /// Keep or Undo all if the area was edited meanwhile.
    pub(crate) fn hold_while_playing(&mut self, actions: &mut Vec<WorkbenchAction>) {
        if self.open.playing.is_none() {
            return;
        }
        if actions.contains(&WorkbenchAction::Quit) {
            self.stop();
            if let Some(changes) = &mut self.open.play_changes {
                actions.retain(|a| *a != WorkbenchAction::Quit);
                changes.then = Some(WorkbenchAction::Quit);
            }
            return;
        }
        let before = actions.len();
        actions.retain(|a| !waits_for_stop(a));
        if actions.len() < before {
            self.log(
                LogLevel::Warning,
                "Stop playing first: saving, opening and making assemblies wait until the \
                 game stops, so its changes are never saved by mistake.",
            );
        }
    }
}

/// What sprites' clips did in a tick, as sounds to play (a cue named like
/// a sound event plays it on the entity) and as events for game code
/// (`CueReached`, `ClipFinished`), sent only to entities with game code.
/// They carry the project file's names (decision 0077): `Cue.Footstep`,
/// and the clip by its sheet's path in `paths` (`Clip.Sprites.Farmer.Walk`);
/// one that has no such name isn't sent.
fn cues_of(
    world: &world::World,
    cues: Vec<world::SpriteCue>,
    sounds: &std::collections::HashMap<String, String>,
    paths: &std::collections::HashMap<reflect::AssetId, String>,
) -> (Vec<crate::game_sound::Request>, Vec<script::Event>) {
    let mut plays = Vec::new();
    let mut events = Vec::new();
    for cue in cues {
        let (id, name, value) = match cue {
            world::SpriteCue::Event(id, cue) => {
                if let Some(event) = sounds.get(&cue) {
                    plays.push(crate::game_sound::Request::Play(
                        event.clone(),
                        script::SoundAt::On(id),
                        Some(id),
                    ));
                }
                (id, "CueReached", script::project_names::option_of(&cue))
            }
            world::SpriteCue::Finished(id, clip) => {
                let sheet = world.sprite_sheet(id).and_then(|s| paths.get(&s));
                let option = sheet.and_then(|path| {
                    let stem = path
                        .rsplit_once('.')
                        .map_or(path.as_str(), |(stem, _)| stem);
                    script::project_names::option_of(&format!("{stem}/{clip}"))
                });
                (id, "ClipFinished", option)
            }
        };
        if let Some(value) = value
            && world.has_game_components(id)
        {
            events.push(script::Event {
                to: id,
                name: name.to_owned(),
                values: vec![reflect::Value::Text(value)],
            });
        }
    }
    (plays, events)
}

#[cfg(test)]
#[path = "play_tests.rs"]
mod tests;

/// A seed for a new play, from the clock.
fn fresh_seed() -> u64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos() as u64);
    // Mixed (splitmix64's finalizer), and kept to whole numbers Lust and
    // Lua can hold exactly, so `setSeed` can repeat it.
    let mut x = nanos;
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^= x >> 31;
    x % 1_000_000_000
}
