//! Game code in the editor: the program's components in the registry (so
//! Properties shows them and Add component lists them), and the program
//! running in play mode against the game's world
//! ([decision 0028](../../../docs/decisions/0028-lust-runs-as-luau-while-editing.md)).

use crate::console::LogLevel;
use crate::workbench::Workbench;
use area::Area;
use glam::Mat4;
use reflect::EntityId;
use script::{Change, CodeProblem, Failure, Game, GameWorld, ViewIndex, WorldView};
use std::collections::HashMap;
use std::sync::Arc;

impl Workbench {
    /// Takes in the game's program (the app compiles it on a worker,
    /// starting it there to read its components, and hands it over when
    /// the code has no mistakes; `None` when there's no code). Its
    /// components replace the last program's in the registry; one whose
    /// name is taken (by one of the engine's own components, or a Rust
    /// one) is a problem, and doesn't run. While playing, the running game
    /// takes the new code first: if it can't (its start fails against the
    /// game's world), that's a problem, and the last program stays,
    /// registry and all. Nothing here runs game code unless the game is
    /// playing.
    pub fn set_program(&mut self, chunk: Option<Arc<script::Compiled>>) {
        let schemas = chunk.as_ref().map_or(&[][..], |c| c.schemas.as_slice());
        let left_out: std::collections::BTreeSet<String> = schemas
            .iter()
            .filter(|s| {
                self.document.registry.get(&s.name).is_some()
                    && !self.game_components.contains(&s.name)
            })
            .map(|s| s.name.clone())
            .collect();
        let taken: Vec<CodeProblem> = left_out
            .iter()
            .map(|name| self.name_taken(chunk.as_deref(), name))
            .collect();
        if self.playing.is_some()
            && let Err(failure) = self.swap_into_game(chunk.as_ref(), &left_out)
        {
            let text = format!(
                "The running game couldn't take the new code, so it keeps the last: {failure}"
            );
            self.log(LogLevel::Error, text);
            self.code_problems.push(failure.at);
            return;
        }
        for name in std::mem::take(&mut self.game_components) {
            self.document.registry.remove(&name);
        }
        self.sources += 1;
        for schema in schemas {
            if !left_out.contains(&schema.name)
                && self.document.registry.register(schema.clone()).is_ok()
            {
                self.game_components.insert(schema.name.clone());
            }
        }
        self.code_problems.extend(taken);
        self.left_out = left_out;
        let notes: Vec<String> = chunk
            .as_ref()
            .and_then(|c| c.lust.as_ref())
            .map(|lust| lust.notes.iter().map(ToString::to_string).collect())
            .unwrap_or_default();
        if notes != self.told_code_notes {
            for note in &notes {
                self.log(LogLevel::Warning, note.clone());
            }
            self.told_code_notes = notes;
        }
        self.program = chunk;
        // The screens a Screen can show (decision 0087).
        let screens = self.screen_names();
        if self.viewport.sheets.screens != screens {
            std::sync::Arc::make_mut(&mut self.viewport.sheets).screens = screens;
        }
        self.fit_values_to_code();
    }

    /// Game code's components' values read before their schemas were
    /// known (the area opened before the code was checked: making the
    /// playground or the sample game) as the kinds the code says: an
    /// entity field is still text until then, and would read as nothing.
    /// In the area (not an undo step) and the assemblies.
    fn fit_values_to_code(&mut self) {
        let registry = &self.document.registry;
        let fits = |data: &area::EntityData| -> Vec<(String, String, reflect::Value)> {
            let mut fitted = Vec::new();
            for (name, component) in &data.components {
                let Some(schema) = registry.get(name) else {
                    continue;
                };
                for field in &schema.fields {
                    if let Some(value) = component.get(&field.name) {
                        let fit = area::text::fit_value(value.clone(), &field.kind);
                        if &fit != value {
                            fitted.push((name.clone(), field.name.clone(), fit));
                        }
                    }
                }
            }
            fitted
        };
        let mut edits = Vec::new();
        for (id, data) in self.document.area().iter() {
            for (component, field, value) in fits(data) {
                edits.push(area::Edit::SetField {
                    entity: id,
                    component,
                    field,
                    value: Some(value),
                });
            }
        }
        let mut files = Vec::new();
        for (asset, file) in &self.assemblies {
            let changes: Vec<_> = file
                .area
                .iter()
                .flat_map(|(id, data)| fits(data).into_iter().map(move |f| (id, f)))
                .collect();
            if !changes.is_empty() {
                files.push((*asset, changes));
            }
        }
        if !edits.is_empty() {
            self.document.edit_by_game(edits);
        }
        for (asset, changes) in files {
            if let Some(file) = self.assemblies.get_mut(&asset) {
                let file = std::sync::Arc::make_mut(file);
                for (id, (component, field, value)) in changes {
                    if let Some(c) = file
                        .area
                        .get_mut(id)
                        .and_then(|d| d.components.get_mut(&component))
                    {
                        c.set(&field, value);
                    }
                }
            }
        }
    }

    /// The problem of a component in `chunk` called `name` when the name
    /// is taken already.
    fn name_taken(&self, chunk: Option<&script::Compiled>, name: &str) -> CodeProblem {
        let by = if self.rust_components.iter().any(|k| k.schema.name == name) {
            "a component written in Rust"
        } else {
            "one of the engine's own components"
        };
        let (file, line) = chunk
            .and_then(|c| c.places.get(name).cloned())
            .unwrap_or_else(|| (script::code::ENTRY.to_string(), 0));
        CodeProblem {
            file,
            line,
            column: 0,
            message: format!(
                "{name} is the name of {by}, so the one in your code doesn't run; give it \
                 another name"
            ),
        }
    }

    /// Gives the running game (if playing) `chunk`: running components get
    /// the new code in place and keep their values, new ones start, and
    /// the Console says what changed. The components called one of
    /// `left_out` don't run. On a failure nothing changed.
    fn swap_into_game(
        &mut self,
        chunk: Option<&Arc<script::Compiled>>,
        left_out: &std::collections::BTreeSet<String>,
    ) -> Result<(), Failure> {
        let Some(playing) = &mut self.playing else {
            return Ok(());
        };
        if playing.game.is_none() {
            playing.game = Some(new_game(&self.rust_components)?);
        }
        let Some(game) = &mut playing.game else {
            return Ok(());
        };
        let mut lines = Vec::new();
        if playing.recording.is_some() && !playing.told_recording_edited {
            playing.told_recording_edited = true;
            lines.push(
                "The code changed while recording: a recording keeps only the input, so its \
                 replay will stop matching from here."
                    .to_string(),
            );
        }
        let time = playing.world.clock().time;
        let (tick, seed) = (playing.world.clock().ticks, playing.world.seed());
        let mut world = PlayWorld::new(&mut playing.world, &mut playing.view, time);
        match chunk {
            None => {
                game.unload();
                lines.push("The game's code is gone, so its components stopped.".to_string());
            }
            Some(chunk) => {
                let swapped = game.swap(chunk, &mut world)?;
                lines.push(swap_summary(&swapped));
                lines.extend(swapped.notes);
            }
        }
        game.leave_out(left_out.clone());
        let failures = sync_all(game, self.document.area(), &mut world);
        world.outside_a_tick();
        let said = std::mem::take(&mut world.said);
        // What starts spawned lands before the next tick.
        let edits = std::mem::take(&mut world.edits);
        let sounds = std::mem::take(&mut world.sounds);
        playing.pending.extend(edits);
        crate::game_sound::gather(&mut self.game_sounds, sounds, tick, seed);
        for line in lines {
            self.log(LogLevel::Info, line);
        }
        self.tell_game(said, &failures);
        Ok(())
    }

    /// Adds a kind of component written in Rust (a game's own): its
    /// schema joins the registry, so Properties shows it, and play mode
    /// runs it beside the Lust ones, sending and handling the same events.
    pub fn add_rust_component(
        &mut self,
        kind: script::RustKind,
    ) -> Result<(), reflect::RegistryError> {
        self.register_component(kind.schema.clone())?;
        self.rust_components.push(kind);
        Ok(())
    }

    /// Replaces the Rust components the open project uses (the app gives
    /// each project its own): the last ones leave the registry, these
    /// join it. A name already taken is a problem, and that one is left
    /// out.
    pub fn set_rust_components(&mut self, kinds: Vec<script::RustKind>) {
        for kind in std::mem::take(&mut self.rust_components) {
            self.document.registry.remove(&kind.schema.name);
        }
        self.sources += 1;
        for kind in kinds {
            if let Err(why) = self.add_rust_component(kind) {
                self.log(
                    LogLevel::Warning,
                    format!("A Rust component was left out: {why}."),
                );
            }
        }
    }

    /// The components the game's code has, by name.
    pub fn game_components(&self) -> impl Iterator<Item = &str> {
        self.game_components.iter().map(String::as_str)
    }
}

/// How many shards game code runs on: as many as the job system has
/// workers (decision 0031), so it uses every core but the window's.
fn cores() -> usize {
    rayon::current_num_threads().max(1)
}

/// New game code on every core, with the open project's Rust components.
pub(crate) fn new_game(rust: &[script::RustKind]) -> Result<Game, Failure> {
    let mut game = Game::new(cores())?;
    for kind in rust {
        game.add_rust(kind.clone());
    }
    Ok(game)
}

/// What game code reads of the world is worked out again only when the
/// world's entities or names change: the copy of who's who.
#[derive(Debug, Default)]
pub(crate) struct ViewCache {
    revision: Option<u64>,
    index: Arc<ViewIndex>,
    /// Each tile map file's path without its extension, by asset ID.
    map_paths: HashMap<reflect::AssetId, String>,
    /// The map files those paths were worked out for.
    map_set: Option<Arc<world::MapFiles>>,
    /// The maps at play game code reads, by path, and their world's
    /// maps revision; and each path's entity, for changes to land on.
    maps: (Option<u64>, Arc<HashMap<String, Arc<tiles::TileMap>>>),
    map_ids: HashMap<String, EntityId>,
    /// Every entity showing a map, with its file's path (if it has one)
    /// and its size in cells, for where maps lie (decision 0082).
    map_entities: Vec<(EntityId, Option<String>, [u32; 2])>,
}

impl ViewCache {
    /// Works out the map files' paths again when the editor's map files
    /// change.
    pub(crate) fn set_map_paths(&mut self, sheets: &crate::sprite_sheets::SpriteSheets) {
        if self
            .map_set
            .as_ref()
            .is_some_and(|m| Arc::ptr_eq(m, &sheets.map_set))
        {
            return;
        }
        self.map_set = Some(Arc::clone(&sheets.map_set));
        self.map_paths = sheets
            .map_set
            .keys()
            .filter_map(|id| {
                let path = sheets.paths.get(id)?;
                Some((*id, reflect::data_files::path_stem(path)))
            })
            .collect();
        self.maps.0 = None;
    }
}

/// Something game code asked to happen to the area: a copy of an
/// assembly made (as this entity, of the assembly named so, here), or an
/// entity taken out.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum GameEdit {
    Spawn(EntityId, String, glam::Vec3),
    Despawn(EntityId),
}

/// The game's world as game code reaches it ([decision 0033](../../../docs/decisions/0033-game-code-sees-the-start-of-the-tick.md)):
/// a copy for each phase of a tick, and where its changes land.
pub(crate) struct PlayWorld<'a> {
    world: &'a mut world::World,
    cache: &'a mut ViewCache,
    /// The game's time at this tick.
    time: f64,
    /// What game code said, for the Console.
    pub(crate) said: Vec<String>,
    /// How long the last frame took, in seconds.
    pub(crate) frame_time: f64,
    /// What game code spawned and despawned, in the order asked for, for
    /// the editor to land after the tick.
    pub(crate) edits: Vec<GameEdit>,
    /// The tick's input (decision 0037).
    pub(crate) input: Arc<input::Frame>,
    /// The input actions' bindings.
    pub(crate) bindings: Arc<input::Bindings>,
    /// Game code's rebinding (action, input), in order, for the editor to
    /// make after the tick.
    pub(crate) rebinds: Vec<(String, String)>,
    /// Game code's saves and loads (slot, whether it's a load), in order,
    /// for the editor to make after the tick.
    pub(crate) saves: Vec<(String, bool)>,
    /// Game code's sound requests, in order (decision 0056: they only go
    /// out; nothing comes back).
    pub(crate) sounds: Vec<crate::game_sound::Request>,
    /// A seed game code set (`setSeed`), made at the end of the tick so
    /// the whole tick draws from one seed.
    pub(crate) seed: Option<u64>,
}

impl<'a> PlayWorld<'a> {
    pub(crate) fn new(world: &'a mut world::World, cache: &'a mut ViewCache, time: f64) -> Self {
        Self {
            world,
            cache,
            time,
            said: Vec::new(),
            frame_time: 1.0 / 60.0,
            edits: Vec::new(),
            input: Arc::default(),
            bindings: Arc::default(),
            rebinds: Vec::new(),
            saves: Vec::new(),
            sounds: Vec::new(),
            seed: None,
        }
    }

    /// For a host lent outside a tick (a component starting, or syncing
    /// an edit): saves, loads and rebinding asked for there aren't made,
    /// so say so in what game code said instead of dropping them quietly.
    pub(crate) fn outside_a_tick(&mut self) {
        if let Some(seed) = self.seed.take() {
            self.world.set_seed(seed);
        }
        if !self.saves.is_empty() || !self.rebinds.is_empty() {
            self.saves.clear();
            self.rebinds.clear();
            self.said.push(
                "saveGame, loadGame and rebind work from update and events, not while a \
                 component starts: they were left out."
                    .to_string(),
            );
        }
    }
}

impl PlayWorld<'_> {
    /// The maps at play by their files' paths, made again only when one
    /// changed (the first entity showing each file, by ID).
    fn maps(&mut self) -> Arc<HashMap<String, Arc<tiles::TileMap>>> {
        let revision = self.world.maps_revision();
        if self.cache.maps.0 != Some(revision) {
            let mut maps = HashMap::new();
            let mut ids = HashMap::new();
            let mut entities = Vec::new();
            for (id, asset, map) in self.world.tile_maps_at_play() {
                let path = self.cache.map_paths.get(&asset).cloned();
                entities.push((id, path.clone(), map.size()));
                if let Some(path) = path
                    && !maps.contains_key(&path)
                {
                    maps.insert(path.clone(), Arc::new(map));
                    ids.insert(path, id);
                }
            }
            self.cache.maps = (Some(revision), Arc::new(maps));
            self.cache.map_ids = ids;
            self.cache.map_entities = entities;
        }
        Arc::clone(&self.cache.maps.1)
    }

    /// Where each map at play lies, by its file's path (the first entity
    /// showing it), and the box each entity's map covers: worked out each
    /// tick, since game code may move a map's entity.
    fn map_places(
        &self,
    ) -> (
        HashMap<String, script::MapPlace>,
        HashMap<EntityId, (glam::Vec3, glam::Vec3)>,
    ) {
        let cell = self.world.pixels().tile_size();
        let mut places = HashMap::new();
        let mut bounds = HashMap::new();
        for (id, path, size) in &self.cache.map_entities {
            let Some(transform) = self.world.global(*id) else {
                continue;
            };
            let place = script::MapPlace {
                transform,
                cell,
                size: *size,
            };
            bounds.insert(*id, place.bounds());
            if let Some(path) = path
                && self.cache.map_ids.get(path) == Some(id)
            {
                places.insert(path.clone(), place);
            }
        }
        (places, bounds)
    }
}

impl GameWorld for PlayWorld<'_> {
    fn view(&mut self) -> WorldView {
        let revision = self.world.revision();
        if self.cache.revision != Some(revision) {
            self.cache.index = Arc::new(ViewIndex::new(&self.world.entities()));
            self.cache.revision = Some(revision);
        }
        let mut places: Vec<Mat4> = Vec::new();
        self.world.globals(&mut places);
        let maps = self.maps();
        let places_of_maps = self.map_places();
        WorldView {
            time: self.time,
            index: Arc::clone(&self.cache.index),
            places: Arc::new(places),
            frame_time: self.frame_time,
            seed: self.world.seed(),
            physics: Arc::clone(self.world.physics().state()),
            flat: Arc::clone(self.world.flat().state()),
            input: Arc::clone(&self.input),
            bindings: Arc::clone(&self.bindings),
            animator_states: Arc::new(self.world.animator_state_names()),
            screens: Arc::new(self.world.screens_shown()),
            maps,
            map_places: Arc::new(places_of_maps.0),
            map_bounds: Arc::new(places_of_maps.1),
            view_size: {
                let pixels = self.world.pixels();
                glam::Vec3::new(
                    pixels.base[0] as f32 * pixels.pixel_size(),
                    pixels.base[1] as f32 * pixels.pixel_size(),
                    0.0,
                )
            },
        }
    }

    fn apply(&mut self, changes: &[Change]) {
        let mut i = 0;
        while i < changes.len() {
            match changes[i] {
                Change::Spawn(id, ref assembly, place) => {
                    self.edits
                        .push(GameEdit::Spawn(id, assembly.clone(), place));
                }
                Change::Despawn(id) => self.edits.push(GameEdit::Despawn(id)),
                Change::Seed(seed) => self.seed = Some(seed),
                Change::Push(which, impulse) => self.world.push(which, impulse),
                Change::Velocity(which, velocity) => self.world.set_velocity(which, velocity),
                Change::Character(which, wanted) => self.world.move_character(which, wanted),
                Change::Rebind(ref action, ref input) => {
                    self.rebinds.push((action.clone(), input.clone()));
                }
                Change::Save(ref slot) => self.saves.push((slot.clone(), false)),
                Change::Load(ref slot) => self.saves.push((slot.clone(), true)),
                Change::Sound(ref event, at, caller) => self
                    .sounds
                    .push(crate::game_sound::Request::Play(event.clone(), at, caller)),
                Change::StopSounds(which) => {
                    self.sounds.push(crate::game_sound::Request::StopOn(which));
                }
                Change::SoundPlaying(which, playing) => {
                    self.world.set_sound_playing(which, playing);
                }
                Change::ShowScreen(which, ref screen, shown) => {
                    self.world.set_screen(which, screen, shown);
                }
                Change::PlayClip(which, ref clip, again) => {
                    self.world.play_clip(which, clip, again);
                }
                Change::StopClip(which) => self.world.stop_clip(which),
                Change::AnimatorSet(which, ref name, value) => {
                    let value = match value {
                        script::AnimatorValue::Number(n) => animation::ParamValue::Number(n),
                        script::AnimatorValue::Switch(on) => animation::ParamValue::Switch(on),
                        script::AnimatorValue::Fire => animation::ParamValue::Trigger(true),
                        script::AnimatorValue::Direction(way) => {
                            let flat = glam::Vec2::new(way.x, way.y).normalize_or_zero();
                            animation::ParamValue::Direction([flat.x, flat.y])
                        }
                    };
                    // A name the animator doesn't have (its file changed
                    // since the code was checked) sets nothing.
                    let _ = self.world.set_animator_value(which, name, value);
                }
                Change::AnimatorJump(which, ref state) => {
                    self.world.jump_to_animator_state(which, state);
                }
                Change::AnimatorSpeed(which, scale) => self.world.set_animator_speed(which, scale),
                // A cell or value off the map, or a map no entity shows,
                // changes nothing.
                Change::MapTile(ref file, layer, cell, code) => {
                    if let Some(&id) = self.cache.map_ids.get(file) {
                        let _ = self.world.set_tile(id, layer, cell, code);
                    }
                }
                Change::MapValue(ref file, value, cell, to) => {
                    if let Some(&id) = self.cache.map_ids.get(file) {
                        let _ = self.world.set_cell_value(id, value, cell, to);
                    }
                }
                Change::BusVolume(ref bus, share) => self
                    .sounds
                    .push(crate::game_sound::Request::BusVolume(bus.clone(), share)),
                Change::Move(which, _) | Change::Turn(which, _) => {
                    // Every move and turn of one entity in a row (one
                    // component's are side by side) in one lookup; the
                    // last of each wins, as one after another would.
                    let mut place = None;
                    let mut facing = None;
                    while let Some(change) = changes.get(i) {
                        match *change {
                            Change::Move(w, p) if w == which => place = Some(p),
                            Change::Turn(w, f) if w == which => facing = Some(f),
                            _ => break,
                        }
                        i += 1;
                    }
                    // A transform is measured from the parent; most
                    // entities stand alone, and need no working out.
                    self.world.change_transform(which, |parent, mut t| {
                        let alone = parent == glam::Mat4::IDENTITY;
                        let mut changed = false;
                        if let Some(place) = place {
                            let p = if alone {
                                place
                            } else {
                                parent.inverse().transform_point3(place)
                            };
                            if p.is_finite() {
                                t.position = p;
                                changed = true;
                            }
                        }
                        if let Some(facing) = facing {
                            let r = if alone {
                                facing.normalize()
                            } else {
                                let parent = parent.to_scale_rotation_translation().1;
                                (parent.inverse() * facing).normalize()
                            };
                            if r.is_finite() {
                                t.rotation = r;
                                changed = true;
                            }
                        }
                        changed.then_some(t)
                    });
                    continue;
                }
                Change::Fire(shot) => {
                    self.world.projectiles_mut().fire(world::Shot {
                        from: shot.from,
                        direction: shot.direction,
                        speed: shot.speed,
                        life: shot.life,
                        colour: shot.colour,
                    });
                }
            }
            i += 1;
        }
        // Those under what moved follow before the next phase reads.
        self.world.settle();
    }

    fn say(&mut self, words: &str) {
        self.said.push(words.to_string());
    }
}

/// Makes the game components of each of `ids` in `area` run (or gives
/// them their changed fields), and stops those an entity no longer has.
pub(crate) fn sync_entities(
    game: &mut Game,
    area: &Area,
    ids: impl IntoIterator<Item = EntityId> + Clone,
    world: &mut dyn GameWorld,
) -> Vec<Failure> {
    let components = ids.clone().into_iter().flat_map(|id| {
        area.get(id).into_iter().flat_map(move |entity| {
            entity
                .components
                .iter()
                .map(move |(name, data)| (id, name.as_str(), data))
        })
    });
    let wanted: Vec<_> = components
        .filter(|(_, name, _)| game.has_component(name))
        .collect();
    let failures = game.sync_many(wanted, world);
    game.retain_on(ids.into_iter().map(|id| {
        let keep = area
            .get(id)
            .map(|e| e.components.keys().cloned().collect())
            .unwrap_or_default();
        (id, keep)
    }));
    failures
}

/// [`sync_entities`] for every entity in `area`, stopping components of
/// entities that are gone.
pub(crate) fn sync_all(game: &mut Game, area: &Area, world: &mut dyn GameWorld) -> Vec<Failure> {
    let wanted: Vec<_> = area
        .iter()
        .flat_map(|(id, entity)| {
            entity
                .components
                .iter()
                .map(move |(name, data)| (id, name.as_str(), data))
        })
        .filter(|(_, name, _)| game.has_component(name))
        .collect();
    let kept = wanted
        .iter()
        .map(|(id, name, _)| (*id, (*name).to_string()))
        .collect();
    let failures = game.sync_many(wanted, world);
    game.retain_only(kept);
    failures
}

/// What a swap changed, in one line for the Console.
fn swap_summary(swapped: &script::Swapped) -> String {
    let mut parts = Vec::new();
    for (names, what) in [
        (&swapped.changed, "changed"),
        (&swapped.added, "new"),
        (&swapped.removed, "gone"),
    ] {
        if !names.is_empty() {
            parts.push(format!("{} {what}", names.join(", ")));
        }
    }
    if parts.is_empty() {
        "The running game took the new code; every component keeps its values.".to_string()
    } else {
        format!(
            "The running game took the new code ({}); components keep their values.",
            parts.join("; ")
        )
    }
}

/// A failure as the Console says it, naming the entity, and whether the
/// component stopped.
pub(crate) fn failure_text(area: &Area, failure: &Failure) -> String {
    match failure.entity.and_then(|id| area.get(id)) {
        Some(entity) if failure.stopped => format!(
            "{} on {} stopped: {}",
            failure.component, entity.name, failure.at
        ),
        Some(entity) => format!("{} on {}: {}", failure.component, entity.name, failure.at),
        None => failure.to_string(),
    }
}

#[cfg(test)]
#[path = "game_code_tests.rs"]
mod tests;
