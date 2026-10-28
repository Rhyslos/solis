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
        self.program = chunk;
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

impl GameWorld for PlayWorld<'_> {
    fn view(&mut self) -> WorldView {
        let revision = self.world.revision();
        if self.cache.revision != Some(revision) {
            self.cache.index = Arc::new(ViewIndex::new(&self.world.entities()));
            self.cache.revision = Some(revision);
        }
        let mut places: Vec<Mat4> = Vec::new();
        self.world.globals(&mut places);
        WorldView {
            time: self.time,
            index: Arc::clone(&self.cache.index),
            places: Arc::new(places),
            frame_time: self.frame_time,
            seed: self.world.seed(),
            physics: Arc::clone(self.world.physics().state()),
            input: Arc::clone(&self.input),
            bindings: Arc::clone(&self.bindings),
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
                Change::Sound(ref event, at) => self
                    .sounds
                    .push(crate::game_sound::Request::Play(event.clone(), at)),
                Change::StopSounds(which) => {
                    self.sounds.push(crate::game_sound::Request::StopOn(which));
                }
                Change::SoundPlaying(which, playing) => {
                    self.world.set_sound_playing(which, playing);
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
