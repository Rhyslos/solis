//! Loading a save game (decision 0038): the save read and checked
//! against this project, the world built as it was saved, game code
//! started afresh and given its saved values, then swapped in for the
//! running game.

use super::{Save, objects_of, one_line};
use crate::console::LogLevel;
use crate::game_code::{GameEdit, PlayWorld, ViewCache};
use crate::workbench::{Workbench, WorkbenchAction};
use area::Area;
use reflect::EntityId;
use std::sync::Arc;

/// A loaded game, ready to take the running one's place.
struct Loaded {
    /// Its world, built as it was saved.
    world: world::World,
    /// What game code reads of it.
    view: ViewCache,
    /// Its game code, given its saved values.
    game: Option<script::Game>,
    /// What the starts spawned, landing before the next tick.
    pending: Vec<GameEdit>,
    /// Its input actions.
    bindings: input::Bindings,
    /// Game time, seconds.
    time: f64,
}

impl Workbench {
    /// Loads a save game (`text`, from the file called `name`): the game
    /// is put back exactly as it was saved, and carries on from there.
    /// Plays first if nothing is running. Stop still puts the area back
    /// as it was before Play. The editor gets the mouse and keys back (a
    /// load from the Play menu).
    pub fn load_game(&mut self, text: &str, name: &str) {
        self.load_game_with(text, name, false);
    }

    /// As [`Self::load_game`], for game code's `Saves:loadFrom` (a quick-load
    /// key): the game keeps the mouse and keys, so playing goes on.
    pub fn load_game_from_code(&mut self, text: &str, slot: &str) {
        self.load_game_with(text, slot, true);
    }

    fn load_game_with(&mut self, text: &str, name: &str, keep_input: bool) {
        let Some((mut save, area, values, bindings)) = self.read_for_loading(text, name) else {
            return;
        };
        // An older save loads, saying what couldn't be carried over.
        for note in &save.notes {
            self.log(LogLevel::Warning, format!("Loading {name}: {note}."));
        }
        // The loaded game sounds afresh: what played before the load fades
        // out, and nothing queued from before it plays after.
        self.plays += 1;
        self.open.game_sounds.clear();
        let mut world = world_of_save(&area, &mut save);
        if self.open.playing.is_none() {
            self.play();
        }
        if self.open.playing.is_none() {
            return;
        }
        // The area as it was saved, for the editor's panels; not undone
        // steps, and Stop still goes back to before Play.
        if keep_input {
            // What's held now isn't held in the loaded game.
            self.devices.release_all();
        } else {
            self.give_back_input();
        }
        self.open.document.replace_by_game(&area);
        let _ = self.open.document.take_changes();
        let mut view = ViewCache::default();
        // The player's settings aren't the game's: a load keeps them.
        view.settings = self.player_settings;
        // Game code starts on a world of its own, thrown away after: what
        // starts do (spawn, move, set the seed) already happened in the
        // saved game, so they mustn't happen again on top of it.
        let mut scratch = world::World::from_area_without_physics(&area);
        let mut scratch_view = ViewCache::default();
        let (mut game, _) = self.start_game_saying(&mut scratch, &mut scratch_view, false);
        drop(scratch);
        let mut pending = Vec::new();
        // The running components take their saved values.
        if let Some(game) = &mut game {
            let mut host = PlayWorld::new(&mut world, &mut view, save.time);
            self.give_saved_values(game, &mut host, &area, &values, name);
            pending.extend(std::mem::take(&mut host.edits));
            // And the game-wide objects theirs (decision 0080).
            self.give_saved_objects(game, &save.objects, name);
        }
        let loaded = Loaded {
            world,
            view,
            game,
            pending,
            bindings,
            time: save.time,
        };
        self.swap_in_loaded(loaded, name);
    }

    /// The save in `text` (from the file called `name`) read, with its
    /// area, its running components' values and its input actions; none,
    /// saying why in the Console, when it can't be loaded here.
    fn read_for_loading(
        &mut self,
        text: &str,
        name: &str,
    ) -> Option<(Save, Area, Area, input::Bindings)> {
        let save = match Save::read(text) {
            Ok(save) => save,
            Err(why) => {
                self.log(LogLevel::Error, format!("Couldn't load {name}: {why}."));
                return None;
            }
        };
        if let Some(project) = &self.open.project
            && one_line(&project.name) != save.project
        {
            self.log(
                LogLevel::Error,
                format!(
                    "Couldn't load {name}: it was saved by the project {}, not this one.",
                    save.project
                ),
            );
            return None;
        }
        let registry = &self.open.document.registry;
        let (area, values) = match (
            area::text::read(&save.area, registry),
            area::text::read(&save.values, registry),
        ) {
            (Ok(area), Ok(values)) => (area, values),
            (Err(why), _) | (_, Err(why)) => {
                self.log(LogLevel::Error, format!("Couldn't load {name}: {why}."));
                return None;
            }
        };
        let bindings = match input::Bindings::read(&save.bindings) {
            Ok(b) => b,
            Err(why) => {
                self.log(
                    LogLevel::Error,
                    format!("Couldn't load {name}: its input actions can't be read: {why}."),
                );
                return None;
            }
        };
        Some((save, area, values, bindings))
    }

    /// The running components take their saved `values`: only values for
    /// components the saved `area` has on that entity (which are what's
    /// running), so a hand-made save can't give an entity a component
    /// this way. Values of components the code no longer has are left
    /// out, said once each; the starts' failures are told.
    fn give_saved_values(
        &mut self,
        game: &mut script::Game,
        host: &mut PlayWorld<'_>,
        area: &Area,
        values: &Area,
        name_shown: &str,
    ) {
        let components: Vec<(EntityId, String, reflect::ComponentData)> = values
            .iter()
            .flat_map(|(id, data)| {
                let has = area.get(id).map(|e| &e.components);
                data.components
                    .iter()
                    .filter(move |(n, _)| has.is_some_and(|c| c.contains_key(*n)))
                    .map(move |(n, d)| (id, n.clone(), d.clone()))
            })
            .collect();
        // Values of components the game's code no longer has (the code
        // changed since the save): left out, and said once each.
        let mut gone: Vec<&str> = components
            .iter()
            .map(|(_, n, _)| n.as_str())
            .filter(|n| !game.has_component(n))
            .collect();
        gone.sort_unstable();
        gone.dedup();
        for name in gone {
            self.console.push(
                LogLevel::Warning,
                format!(
                    "Loading {name_of_save}: the game's code has no component {name} any \
                     more, so its saved values were left out.",
                    name_of_save = name_shown
                ),
            );
        }
        let failures = game.sync_many(components.iter().map(|(e, n, d)| (*e, n.as_str(), d)), host);
        for failure in failures {
            let text = crate::game_code::failure_text(self.open.document.area(), &failure);
            self.log(LogLevel::Error, text);
        }
    }

    /// The game-wide objects take their saved values (decision 0080):
    /// those of objects the code no longer has, or of fields of another
    /// kind now, are left out and said.
    fn give_saved_objects(&mut self, game: &mut script::Game, objects: &str, name_shown: &str) {
        let (schemas, _) = game.object_info();
        match objects_of(objects, &schemas) {
            Ok(objects) => {
                let mut gone: Vec<String> = objects
                    .iter()
                    .filter(|(name, _)| !schemas.iter().any(|s| s.name == *name))
                    .map(|(name, _)| {
                        format!(
                            "the game's code has no object {name} any more, so its saved \
                             values were left out"
                        )
                    })
                    .collect();
                for (object, field) in game.set_object_values(objects) {
                    if schemas.iter().any(|s| s.name == object) {
                        gone.push(format!(
                            "{object}'s {field} is a different kind now, so it starts as the \
                             code writes it"
                        ));
                    }
                }
                for why in gone {
                    self.console.push(
                        LogLevel::Warning,
                        format!("Loading {name_shown}: {why}."),
                    );
                }
            }
            Err(why) => self.log(
                LogLevel::Warning,
                format!("Loading {name_shown}: its objects couldn't be read ({why}), so they start as the code writes them."),
            ),
        }
    }

    /// The loaded game takes the running one's place. A recording or
    /// replay ends at a load (it plays back from Play), and what was
    /// recorded up to it is kept, as Stop keeps it.
    fn swap_in_loaded(&mut self, loaded: Loaded, name: &str) {
        let Some(playing) = &mut self.open.playing else {
            return;
        };
        playing.world = loaded.world;
        playing.view = loaded.view;
        playing.game = loaded.game;
        playing.pending = loaded.pending;
        playing.bindings = Arc::new(loaded.bindings);
        let recorded = playing.recording.take();
        let stopped = recorded.is_some() | playing.replay.take().is_some();
        if let Some(recording) = recorded.filter(|r| !r.ticks.is_empty()) {
            self.open
                .outbox
                .push(WorkbenchAction::SaveRecording(recording.write()));
        }
        self.log(
            LogLevel::Info,
            format!(
                "Loaded {name}: the game is back as it was {:.1} s in.",
                loaded.time
            ),
        );
        if stopped {
            self.log(
                LogLevel::Warning,
                "Loading a save ended the recording or replay (a recording plays back from \
                 Play; what was recorded up to the load is saved).",
            );
        }
    }
}

/// The world as `save` has it, built from its `area`: each entity where
/// the game had it (noted as the game's move, as in the saved game),
/// physics built from each body's place and motion as saving built the
/// running game's, the clock, seed, speed and day, projectiles, animators
/// and tile maps. Takes those last three out of `save`.
fn world_of_save(area: &Area, save: &mut Save) -> world::World {
    let mut world = world::World::from_area_without_physics(area);
    for (id, t) in &save.places {
        world.set_transform(*id, *t);
    }
    world.settle();
    world.rebuild_physics(&save.physics, true);
    world.set_clock(save.ticks, save.time);
    world.set_seed(save.seed);
    world.set_speed(save.speed);
    world.set_day_shift(save.day_shift);
    world
        .projectiles_mut()
        .restore(std::mem::take(&mut save.projectiles));
    world.restore_animator_values(std::mem::take(&mut save.animators));
    world.restore_tile_maps(std::mem::take(&mut save.tile_maps));
    world
}
