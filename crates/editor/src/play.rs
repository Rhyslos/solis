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
use crate::game_code::{GameEdit, PlayWorld, ViewCache, failure_text, new_game, sync_all};
use crate::panel::Panel;
use crate::workbench::{Workbench, WorkbenchAction};
use area::text::AreaFile;
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
        // project lead's answer 10). Debug keeps its Profiler in front.
        use crate::shell::RibbonTab;
        self.show_in(
            RibbonTab::Home,
            crate::shell::workspaces::Home::Group(0),
            Panel::Game,
        );
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
        #[cfg(test)]
        self.unhurried.give(&mut game);
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

    /// The longest one tick of physics may take: [`PHYSICS_LIMIT`], or
    /// no limit for a test that plays unhurried (TEST-02).
    fn physics_limit(&self) -> std::time::Duration {
        #[cfg(test)]
        if self.unhurried == crate::game_code::Unhurried::Everything {
            return std::time::Duration::MAX;
        }
        PHYSICS_LIMIT
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

#[path = "play_tick.rs"]
mod tick;

#[path = "play_landing.rs"]
mod landing;

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
