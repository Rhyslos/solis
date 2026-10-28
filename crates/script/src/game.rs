//! Game code on every core ([decision 0033](../../../docs/decisions/0033-game-code-sees-the-start-of-the-tick.md)).
//!
//! A [`Game`] splits the entities into shards, one per core, each with a
//! [`Runtime`] of its own (its own Luau, the same code) running that
//! share's components (Lust, Lua and Rust alike). A tick runs in phases:
//! the updates, then the events in rounds. During a phase every shard
//! reads the same copy of the world, taken when the phase started
//! ([`WorldView`]), and sees what its running component itself changed;
//! what it changes (moves, turns, words in the Console, events) is kept
//! with its place in the tick's order ([`crate::ticking::Order`]), and
//! lands at the end of the phase, sorted. So the result is the same on one
//! core or sixteen.
//!
//! With one shard, its runtime runs on the caller's thread; with more,
//! each runs on a thread of its own (a Luau stays on the thread that made
//! it).

use crate::code::Compiled;
use crate::events::Event;
use crate::host::Host;
use crate::runtime::{Failure, Runtime, Swapped, Ticked, in_rounds, is_flood, pause_for};
use crate::rust_component::RustKind;
use crate::ticking::{Order, Phase};
use glam::{Mat4, Quat, Vec3};
use reflect::{ComponentData, ComponentSchema, EntityId};
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};

/// A change game code made to the world, landing when its phase ends.
#[derive(Debug, Clone, PartialEq)]
pub enum Change {
    /// `moveTo`: put the entity at this place in the world.
    Move(EntityId, Vec3),
    /// `turnTo`: turn the entity to face this way in the world.
    Turn(EntityId, Quat),
    /// `fire`: fire a projectile.
    Fire(crate::host::Shot),
    /// `spawn`: make a copy of the assembly named here, as this entity,
    /// at this place.
    Spawn(EntityId, String, Vec3),
    /// `despawn`: take this entity (and those under it) out of the game.
    Despawn(EntityId),
    /// `setSeed`: start the game's randomness again from this seed.
    Seed(u64),
    /// `push`: give the entity's body a push (kilograms times metres a
    /// second).
    Push(EntityId, Vec3),
    /// `setVelocity`: set how fast the entity's body moves.
    Velocity(EntityId, Vec3),
    /// `moveCharacter`: move the character this far (metres), as far as
    /// it can.
    Character(EntityId, Vec3),
    /// `rebind`: bind this input to this action, from the next tick.
    Rebind(String, String),
    /// `saveGame`: save the game in this slot, at the end of the tick.
    Save(String),
    /// `loadGame`: load the game saved in this slot, at the end of the
    /// tick.
    Load(String),
    /// `playSound…`: play the sound event named here, from there; and
    /// the entity whose code asked (who caused it, for "Heard by").
    Sound(String, crate::host::SoundAt, Option<EntityId>),
    /// `stopSoundsOn`: stop the sounds on this entity.
    StopSounds(EntityId),
    /// `setSoundPlaying`: turn this entity's `AudioSource` on or off.
    SoundPlaying(EntityId, bool),
    /// `setBusVolume`: this bus at this share of its fader (0 to 1) for
    /// the rest of the play.
    BusVolume(String, f32),
}

/// The entities of a [`WorldView`]: the same while the world's entities,
/// their order and their names are.
#[derive(Debug, Default)]
pub struct ViewIndex {
    /// Each entity's number in [`WorldView::places`].
    slots: reflect::IdMap<u32>,
    names: Vec<String>,
    /// Each name's first entity, in the area's tree order.
    first_named: HashMap<String, EntityId>,
}

impl ViewIndex {
    /// The index of `entities`, in the area's tree order (as
    /// `world::World::entities` gives them).
    pub fn new(entities: &[Option<(EntityId, String)>]) -> Self {
        let mut index = Self::default();
        for (slot, entity) in entities.iter().enumerate() {
            let Some((id, name)) = entity else {
                index.names.push(String::new());
                continue;
            };
            index.slots.insert(*id, slot as u32);
            index.first_named.entry(name.clone()).or_insert(*id);
            index.names.push(name.clone());
        }
        index
    }
}

/// The world as game code reads it during a phase of a tick: where every
/// entity was when the phase started.
#[derive(Debug, Clone)]
pub struct WorldView {
    /// The game's time, in seconds.
    pub time: f64,
    /// The entities.
    pub index: Arc<ViewIndex>,
    /// Where each is in the world (by its number in `index`).
    pub places: Arc<Vec<Mat4>>,
    /// How long the last frame took, in seconds of real time.
    pub frame_time: f64,
    /// The game's seed, which its random numbers are made from.
    pub seed: u64,
    /// Physics as the phase started (decision 0036), shared, to read.
    pub physics: Arc<physics::State>,
    /// The input this tick (decision 0037): the same on every core.
    pub input: Arc<input::Frame>,
    /// The actions' bindings, to say which input sets one off.
    pub bindings: Arc<input::Bindings>,
}

impl WorldView {
    fn place(&self, which: EntityId) -> Option<Mat4> {
        let slot = *self.index.slots.get(&which)?;
        self.places.get(slot as usize).copied()
    }
}

/// The game's world, as a [`Game`] reaches it: a copy for each phase, and
/// where the phase's changes land.
pub trait GameWorld {
    /// The world now.
    fn view(&mut self) -> WorldView;
    /// Makes `changes`, in order.
    fn apply(&mut self, changes: &[Change]);
    /// Writes words game code said in the Console.
    fn say(&mut self, words: &str);
}

/// What a shard's game code changed in one phase, each with its place in
/// the order.
#[derive(Debug, Default)]
struct Kept {
    changes: Vec<(Order, Change)>,
    said: Vec<(Order, String)>,
    /// Changes and lines left out past [`MAX_KEPT`].
    dropped: u64,
    /// Bytes of assembly names kept for spawns.
    spawn_bytes: usize,
    /// Sounds asked for.
    sounds: usize,
}

/// The most changes (moves, turns, shots, spawns, lines said) one shard
/// keeps in one phase of a tick; past it they're left out, and the Console
/// says so. Kept changes live outside Luau's memory limit, so a runaway
/// loop calling `moveTo` would otherwise fill the computer's memory before
/// the time limit stops it.
pub const MAX_KEPT: usize = 200_000;

/// The most sounds game code may ask for in one tick (decision 0056's
/// limits); past it they're left out.
pub const MAX_SOUNDS: usize = 256;

/// The longest sound event name read.
pub const MAX_SOUND_NAME: usize = 256;

/// The most bytes of assembly names one shard keeps for spawns in one
/// phase.
pub const MAX_SPAWN_BYTES: usize = 4 << 20;

impl Kept {
    /// Whether there's room for one more; counts it as left out if not.
    fn room(&mut self) -> bool {
        let room = self.changes.len() + self.said.len() < MAX_KEPT;
        if !room {
            self.dropped += 1;
        }
        room
    }
}

/// The host a shard's game code reaches during a phase: it reads the
/// phase's [`WorldView`] (and what the running component changed itself),
/// and keeps what it changes.
struct ViewHost {
    view: WorldView,
    /// Which part of the tick this is (the updates are 0, each round of
    /// events the next number; starts and syncs have their own), for the
    /// IDs of what's spawned.
    phase: u32,
    /// The component running now, and where what it does belongs.
    at: Order,
    /// What it has done so far (its order, counted on).
    next: u32,
    /// Where the running component moved entities this phase, the last
    /// of each (it sees its own changes at once).
    own_moves: reflect::IdMap<Vec3>,
    /// … turned them.
    own_turns: reflect::IdMap<Quat>,
    /// … set their velocities.
    own_velocities: reflect::IdMap<Vec3>,
    kept: Kept,
}

impl ViewHost {
    fn new(view: WorldView, phase: u32) -> Self {
        Self {
            view,
            phase,
            at: Order::default(),
            next: 0,
            own_moves: reflect::IdMap::default(),
            own_turns: reflect::IdMap::default(),
            own_velocities: reflect::IdMap::default(),
            kept: Kept::default(),
        }
    }

    fn order(&mut self) -> Order {
        let order = Order {
            seq: self.next,
            ..self.at.clone()
        };
        self.next += 1;
        order
    }
}

impl Host for ViewHost {
    fn say(&mut self, words: &str) {
        if self.kept.room() {
            let order = self.order();
            self.kept.said.push((order, words.to_string()));
        }
    }

    fn time(&self) -> f64 {
        self.view.time
    }

    fn find(&self, name: &str) -> Option<EntityId> {
        self.view.index.first_named.get(name).copied()
    }

    fn position(&self, which: EntityId) -> Option<Vec3> {
        let moved = self.own_moves.get(&which).copied();
        moved.or_else(|| self.view.place(which).map(|m| m.w_axis.truncate()))
    }

    fn move_to(&mut self, which: EntityId, place: Vec3) {
        if self.view.index.slots.contains_key(&which) && self.kept.room() {
            let order = self.order();
            self.kept.changes.push((order, Change::Move(which, place)));
            self.own_moves.insert(which, place);
        }
    }

    fn rotation(&self, which: EntityId) -> Option<Quat> {
        let turned = self.own_turns.get(&which).copied();
        turned.or_else(|| {
            self.view
                .place(which)
                .map(|m| m.to_scale_rotation_translation().1.normalize())
        })
    }

    fn turn_to(&mut self, which: EntityId, facing: Quat) {
        if self.view.index.slots.contains_key(&which) && self.kept.room() {
            let order = self.order();
            self.kept.changes.push((order, Change::Turn(which, facing)));
            self.own_turns.insert(which, facing);
        }
    }

    fn name(&self, which: EntityId) -> Option<String> {
        let slot = *self.view.index.slots.get(&which)?;
        self.view.index.names.get(slot as usize).cloned()
    }

    fn fire(&mut self, shot: crate::host::Shot) {
        if !self.kept.room() {
            return;
        }
        let order = self.order();
        self.kept.changes.push((order, Change::Fire(shot)));
    }

    fn spawn(&mut self, assembly: &str, place: Vec3) -> Option<EntityId> {
        if !place.is_finite() || assembly.is_empty() || assembly.len() > 1024 {
            return None;
        }
        if self.kept.spawn_bytes + assembly.len() > MAX_SPAWN_BYTES {
            self.kept.dropped += 1;
            return None;
        }
        if !self.kept.room() {
            return None;
        }
        self.kept.spawn_bytes += assembly.len();
        let order = self.order();
        // Made from where it was asked for in the tick's order and the
        // time, so the same game makes the same entities on any number of
        // cores.
        let id = spawned_id(&order, self.phase, self.view.time)?;
        self.kept
            .changes
            .push((order, Change::Spawn(id, assembly.to_string(), place)));
        Some(id)
    }

    fn despawn(&mut self, which: EntityId) {
        if !self.kept.room() {
            return;
        }
        let order = self.order();
        self.kept.changes.push((order, Change::Despawn(which)));
    }

    fn frame_time(&self) -> f64 {
        self.view.frame_time
    }

    fn random(&mut self) -> f64 {
        // Made from the seed and where it was asked for in the tick's
        // order, not drawn in turn, so it's the same whichever core asks
        // first (Phase 6 plan, "Determinism rules").
        let order = self.order();
        random_from(self.view.seed, &order, self.phase, self.view.time)
    }

    fn push(&mut self, which: EntityId, impulse: Vec3) {
        if impulse.is_finite() && self.view.physics.contains(which) && self.kept.room() {
            let order = self.order();
            self.kept
                .changes
                .push((order, Change::Push(which, impulse)));
        }
    }

    fn velocity(&self, which: EntityId) -> Option<Vec3> {
        // A component sees its own new velocity at once.
        let own = self.own_velocities.get(&which).copied();
        own.or_else(|| self.view.physics.velocity(which))
    }

    fn set_velocity(&mut self, which: EntityId, velocity: Vec3) {
        if velocity.is_finite() && self.view.physics.contains(which) && self.kept.room() {
            let order = self.order();
            self.kept
                .changes
                .push((order, Change::Velocity(which, velocity)));
            self.own_velocities.insert(which, velocity);
        }
    }

    fn move_character(&mut self, which: EntityId, wanted: Vec3) {
        if wanted.is_finite() && self.view.physics.character(which).is_some() && self.kept.room() {
            let order = self.order();
            self.kept
                .changes
                .push((order, Change::Character(which, wanted)));
        }
    }

    fn on_ground(&self, which: EntityId) -> bool {
        self.view
            .physics
            .character(which)
            .is_some_and(|c| c.grounded)
    }

    fn cast_ray(&self, from: Vec3, direction: Vec3, reach: f32) -> Option<physics::Hit> {
        self.view.physics.cast_ray(from, direction, reach)
    }

    fn action(&self, name: &str) -> input::ActionState {
        self.view.input.get(name)
    }

    fn binding(&self, action: &str) -> Option<String> {
        let action = self.view.bindings.get(action)?;
        action
            .first_input("")
            .or_else(|| action.first_input("Mouse "))
            .or_else(|| action.first_input("Pad "))
            .map(str::to_string)
    }

    fn rebind(&mut self, action: &str, input: &str) {
        if input::valid_action_name(action) && input::valid_input_name(input) && self.kept.room() {
            let order = self.order();
            self.kept
                .changes
                .push((order, Change::Rebind(action.to_string(), input.to_string())));
        }
    }

    fn play_sound(&mut self, event: &str, at: crate::host::SoundAt) {
        let named =
            !event.is_empty() && event.len() <= MAX_SOUND_NAME && !event.contains(char::is_control);
        if named && self.kept.sounds < MAX_SOUNDS && self.kept.room() {
            self.kept.sounds += 1;
            let order = self.order();
            let caller = reflect::Id::from_raw(order.entity).map(EntityId);
            self.kept
                .changes
                .push((order, Change::Sound(event.to_string(), at, caller)));
        }
    }

    fn stop_sounds_on(&mut self, which: EntityId) {
        if self.kept.room() {
            let order = self.order();
            self.kept.changes.push((order, Change::StopSounds(which)));
        }
    }

    fn set_sound_playing(&mut self, which: EntityId, playing: bool) {
        if self.kept.room() {
            let order = self.order();
            self.kept
                .changes
                .push((order, Change::SoundPlaying(which, playing)));
        }
    }

    fn set_bus_volume(&mut self, bus: &str, share: f32) {
        let named =
            !bus.is_empty() && bus.len() <= MAX_SOUND_NAME && !bus.contains(char::is_control);
        // Not counted against the tick's sounds: a flood of plays never
        // drops an options slider (found by the Phase 7 review).
        if named && share.is_finite() && self.kept.room() {
            let order = self.order();
            self.kept.changes.push((
                order,
                Change::BusVolume(bus.to_string(), share.clamp(0.0, 1.0)),
            ));
        }
    }

    fn save_game(&mut self, slot: &str, load: bool) {
        if crate::valid_slot(slot) && self.kept.room() {
            let order = self.order();
            let change = if load {
                Change::Load(slot.to_string())
            } else {
                Change::Save(slot.to_string())
            };
            self.kept.changes.push((order, change));
        }
    }

    fn set_seed(&mut self, seed: u64) {
        if self.kept.room() {
            let order = self.order();
            self.kept.changes.push((order, Change::Seed(seed)));
        }
    }

    fn at(&mut self, order: &Order) {
        self.at = order.clone();
        self.next = 0;
        // Most components change nothing: clearing an empty map that once
        // grew would still wipe all its room.
        if !self.own_moves.is_empty() {
            self.own_moves.clear();
        }
        if !self.own_turns.is_empty() {
            self.own_turns.clear();
        }
        if !self.own_velocities.is_empty() {
            self.own_velocities.clear();
        }
    }
}

/// A random number from 0 up to (not including) 1, made from `seed` and
/// the place `order` in `phase` of the tick at game time `time`: the same
/// hash as spawned IDs, with the seed, then mixed (splitmix64's
/// finalizer) so every bit counts.
fn random_from(seed: u64, order: &Order, phase: u32, time: f64) -> f64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |bytes: &[u8]| {
        for &b in bytes {
            hash ^= u64::from(b);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
    };
    eat(&seed.to_le_bytes());
    eat(&order.major.to_le_bytes());
    eat(order.component.as_bytes());
    eat(&order.seq.to_le_bytes());
    eat(&phase.to_le_bytes());
    eat(&time.to_bits().to_le_bytes());
    let mut x = hash;
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^= x >> 31;
    // The top 53 bits: every value a float can hold evenly.
    (x >> 11) as f64 * (1.0 / (1_u64 << 53) as f64)
}

/// The part of a tick (or not) a component's start ran in, for
/// [`ViewHost::phase`].
const PHASE_LOAD: u32 = u32::MAX;
/// A sync (components started or given changes from the editor).
const PHASE_SYNC: u32 = u32::MAX - 1;

/// The entity a spawn at `order`, in `phase` of the tick, at game time
/// `time`, makes: a hash of all three (FNV-1a), made odd so it's never
/// zero.
fn spawned_id(order: &Order, phase: u32, time: f64) -> Option<EntityId> {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |bytes: &[u8]| {
        for &b in bytes {
            hash ^= u64::from(b);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
    };
    eat(&order.major.to_le_bytes());
    eat(order.component.as_bytes());
    eat(&order.seq.to_le_bytes());
    eat(&phase.to_le_bytes());
    eat(&time.to_bits().to_le_bytes());
    reflect::Id::from_raw(hash | 1).map(EntityId)
}

/// What a shard is asked to do.
enum Job {
    AddRust(RustKind),
    Load(Arc<Compiled>, WorldView),
    Swap(Arc<Compiled>, WorldView),
    Unload,
    LeaveOut(BTreeSet<String>),
    Sync(Vec<(EntityId, String, ComponentData)>, WorldView),
    RetainOnly(Arc<BTreeSet<(EntityId, String)>>),
    RetainOn(Vec<(EntityId, Vec<String>)>),
    Values(EntityId, String),
    Len,
    Checksum,
    AllValues,
    Updates(f64, WorldView),
    Round(Vec<(u32, Event)>, WorldView, u32),
    FinishTick,
}

/// A shard's answer.
enum Done {
    Nothing,
    Loaded(Result<(), Failure>, Kept),
    Swapped(Result<Swapped, Failure>, Kept),
    Synced(Vec<Failure>, Kept),
    Values(Option<ComponentData>),
    Len(usize),
    Checksum(u64),
    AllValues(Vec<(EntityId, String, ComponentData)>),
    Phase(Result<Phase, String>, Kept),
    Finished(bool),
}

/// Does `job` on `runtime`.
fn work(runtime: &mut Runtime, job: Job) -> Done {
    match job {
        Job::AddRust(kind) => {
            runtime.add_rust(kind);
            Done::Nothing
        }
        Job::Load(code, view) => {
            let mut host = ViewHost::new(view, PHASE_LOAD);
            let loaded = runtime.load(&code, &mut host);
            Done::Loaded(loaded, host.kept)
        }
        Job::Swap(code, view) => {
            let mut host = ViewHost::new(view, PHASE_LOAD);
            let swapped = runtime.swap(&code, &mut host);
            Done::Swapped(swapped, host.kept)
        }
        Job::Unload => {
            runtime.unload();
            Done::Nothing
        }
        Job::LeaveOut(names) => {
            runtime.leave_out(names);
            Done::Nothing
        }
        Job::Sync(components, view) => {
            let mut host = ViewHost::new(view, PHASE_SYNC);
            let failures = runtime.sync_many(
                components.iter().map(|(e, n, d)| (*e, n.as_str(), d)),
                &mut host,
            );
            Done::Synced(failures, host.kept)
        }
        Job::RetainOnly(wanted) => {
            runtime.retain(|e, name| wanted.contains(&(e, name.to_string())));
            Done::Nothing
        }
        Job::RetainOn(entities) => {
            for (entity, names) in entities {
                runtime.retain_on(entity, |name| names.iter().any(|n| n == name));
            }
            Done::Nothing
        }
        Job::Values(entity, name) => Done::Values(runtime.values(entity, &name)),
        Job::Len => Done::Len(runtime.len()),
        Job::Checksum => Done::Checksum(runtime.checksum()),
        Job::AllValues => Done::AllValues(runtime.all_values()),
        Job::Updates(dt, view) => {
            // The tick starts with its updates: one hand-over, not two.
            runtime.start_tick();
            let mut host = ViewHost::new(view, 0);
            let phase = runtime.run_updates(dt, &mut host);
            Done::Phase(phase, host.kept)
        }
        Job::Round(events, view, round) => {
            let mut host = ViewHost::new(view, round);
            let phase = runtime.run_round(&events, &mut host);
            Done::Phase(phase, host.kept)
        }
        Job::FinishTick => Done::Finished(runtime.finish_tick()),
    }
}

/// A share of the entities, and the runtime running their components.
enum Shard {
    /// On the caller's thread (the first shard).
    Here(Box<Runtime>),
    /// On a thread of its own.
    Away {
        jobs: Sender<Job>,
        done: Receiver<Done>,
    },
}

impl Shard {
    /// A shard on a thread of its own, or `None` if the thread or its
    /// Luau couldn't start.
    fn away(number: usize) -> Option<Self> {
        let (jobs, inbox) = channel::<Job>();
        let (outbox, done) = channel::<Done>();
        let (ready, started) = channel::<bool>();
        std::thread::Builder::new()
            .name(format!("game code {number}"))
            .spawn(move || {
                let Ok(mut runtime) = Runtime::new() else {
                    let _ = ready.send(false);
                    return;
                };
                let _ = ready.send(true);
                for job in inbox {
                    if outbox.send(work(&mut runtime, job)).is_err() {
                        break;
                    }
                }
            })
            .ok()?;
        started
            .recv()
            .ok()
            .filter(|&ok| ok)
            .map(|_| Self::Away { jobs, done })
    }

    /// Hands `job` over (a shard here does it at once, and keeps the
    /// answer for [`Self::answer`]).
    fn ask(&mut self, job: Job, here: &mut Option<Done>) {
        match self {
            Self::Here(runtime) => *here = Some(work(runtime, job)),
            Self::Away { jobs, .. } => {
                let _ = jobs.send(job);
            }
        }
    }

    /// The answer to the last job (`None`: the shard's thread is gone).
    fn answer(&mut self, here: &mut Option<Done>) -> Option<Done> {
        match self {
            Self::Here(_) => here.take(),
            Self::Away { done, .. } => done.recv().ok(),
        }
    }
}

/// Game code running on several cores. See the module's notes.
pub struct Game {
    shards: Vec<Shard>,
    /// The Rust components' kinds, by name (every shard has them).
    rust: BTreeSet<String>,
    /// The code's own components (Lust and Lua), by name.
    code: BTreeSet<String>,
    /// The code's components' schemas, sorted by name.
    schemas: Vec<ComponentSchema>,
    left_out: BTreeSet<String>,
    /// Whether too many events in a tick was said, since the code changed.
    told: bool,
    /// Whether the Console was told game code changed too much at once
    /// (until the code changes).
    told_too_much: bool,
    /// The code every shard runs, if any: what a failed swap goes back to.
    running: Option<Arc<Compiled>>,
    /// How many components run, once counted (the Profiler asks every
    /// frame; only adding, removing or new code changes it).
    count: Option<usize>,
}

impl std::fmt::Debug for Game {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Game")
            .field("shards", &self.shards.len())
            .field("code", &self.code)
            .finish_non_exhaustive()
    }
}

/// The failure when a shard's thread stopped answering.
fn gone() -> Failure {
    Failure {
        entity: None,
        component: String::new(),
        at: crate::ticking::plain(
            "part of the game's code stopped unexpectedly; press Stop and Play to start again"
                .to_string(),
        ),
        stopped: true,
    }
}

/// The most shards: each is a whole Luau (up to 64 MB) with its own copy
/// of the game's code, and past this many, more cores add little.
pub const MAX_SHARDS: usize = 16;

impl Game {
    /// Game code on `cores` shards (at least one, at most [`MAX_SHARDS`]).
    /// Fails only if Luau can't set itself up (out of memory). If a
    /// shard's thread can't start, the game runs on fewer.
    pub fn new(cores: usize) -> Result<Self, Failure> {
        let cores = cores.clamp(1, MAX_SHARDS);
        // The first shard runs on the caller's thread, which would
        // otherwise only wait for the others.
        let mut shards = vec![Shard::Here(Box::new(Runtime::new()?))];
        shards.extend((1..cores).filter_map(Shard::away));
        Ok(Self {
            shards,
            rust: BTreeSet::new(),
            code: BTreeSet::new(),
            schemas: Vec::new(),
            left_out: BTreeSet::new(),
            told: false,
            told_too_much: false,
            running: None,
            count: None,
        })
    }

    /// How many shards it runs on.
    pub fn shards(&self) -> usize {
        self.shards.len()
    }

    /// Which shard runs `entity`'s components: by its ID, so the same
    /// whatever else is running. The ID is mixed first: spawned IDs are
    /// all odd, and on two shards they'd all land on one.
    fn shard_of(&self, entity: EntityId) -> usize {
        // splitmix64's finalizer: every bit of the ID moves every bit.
        let mut x = entity.0.raw();
        x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        x ^= x >> 31;
        (x % self.shards.len() as u64) as usize
    }

    /// Gives every shard `job` (made for each by its number, in no set
    /// order), and their answers in shard order. The shards work at the
    /// same time.
    fn all(&mut self, mut job: impl FnMut(usize) -> Job) -> Vec<Option<Done>> {
        let mut here: Vec<Option<Done>> = Vec::new();
        here.resize_with(self.shards.len(), || None);
        // The other threads start first; then the caller does its own
        // shard meanwhile, instead of waiting idle.
        for (i, shard) in self.shards.iter_mut().enumerate() {
            if matches!(shard, Shard::Away { .. }) {
                shard.ask(job(i), &mut here[i]);
            }
        }
        for (i, shard) in self.shards.iter_mut().enumerate() {
            if matches!(shard, Shard::Here(_)) {
                shard.ask(job(i), &mut here[i]);
            }
        }
        self.shards
            .iter_mut()
            .zip(here.iter_mut())
            .map(|(shard, here)| shard.answer(here))
            .collect()
    }

    /// Adds a kind of Rust component, kept across loads.
    pub fn add_rust(&mut self, kind: RustKind) {
        self.rust.insert(kind.schema.name.clone());
        let _ = self.all(|_| Job::AddRust(kind.clone()));
    }

    /// Loads the game's code in every shard (see [`Runtime::load`]), its
    /// start reading `world`. What the start says is said once (each
    /// shard runs it).
    pub fn load(&mut self, code: &Arc<Compiled>, world: &mut dyn GameWorld) -> Result<(), Failure> {
        let view = world.view();
        self.count = None;
        let answers = self.all(|_| Job::Load(Arc::clone(code), view.clone()));
        let mut first = None;
        let mut failed = None;
        for answer in answers {
            match answer {
                Some(Done::Loaded(Ok(()), kept)) => {
                    first.get_or_insert(kept);
                }
                Some(Done::Loaded(Err(failure), _)) => {
                    failed.get_or_insert(failure);
                }
                _ => {
                    failed.get_or_insert_with(gone);
                }
            }
        }
        // All or none: a shard whose start failed (out of time, say) while
        // the others' didn't would leave half the game's entities running.
        if let Some(failure) = failed {
            self.unload();
            return Err(failure);
        }
        if let Some(kept) = first {
            land(kept, world);
        }
        self.took(code);
        Ok(())
    }

    /// Swaps new code into every shard's running components (see
    /// [`Runtime::swap`]). What changed is said once; notes once each.
    pub fn swap(
        &mut self,
        code: &Arc<Compiled>,
        world: &mut dyn GameWorld,
    ) -> Result<Swapped, Failure> {
        let view = world.view();
        self.count = None;
        let answers = self.all(|_| Job::Swap(Arc::clone(code), view.clone()));
        let mut merged: Option<Swapped> = None;
        let mut first = None;
        let mut failed = None;
        for answer in answers {
            let swapped = match answer {
                Some(Done::Swapped(Ok(swapped), kept)) => {
                    first.get_or_insert(kept);
                    swapped
                }
                Some(Done::Swapped(Err(failure), _)) => {
                    failed.get_or_insert(failure);
                    continue;
                }
                _ => {
                    failed.get_or_insert_with(gone);
                    continue;
                }
            };
            match &mut merged {
                None => merged = Some(swapped),
                Some(all) => {
                    for note in swapped.notes {
                        if !all.notes.contains(&note) {
                            all.notes.push(note);
                        }
                    }
                }
            }
        }
        // All or none: the shards that took the new code go back to the
        // last, keeping where things are, so every entity runs the same.
        if let Some(failure) = failed {
            match self.running.clone() {
                // What going back asks for is dropped: it's undoing, not
                // playing.
                Some(old) => {
                    self.count = None;
                    let _ = self.all(|_| Job::Swap(Arc::clone(&old), view.clone()));
                }
                None => self.unload(),
            }
            return Err(failure);
        }
        if let Some(kept) = first {
            land(kept, world);
        }
        self.took(code);
        // The same notes however many shards there are.
        let mut merged = merged.unwrap_or_default();
        merged.notes.sort();
        Ok(merged)
    }

    /// Notes the components `code` has.
    fn took(&mut self, code: &Arc<Compiled>) {
        self.running = Some(Arc::clone(code));
        self.schemas.clone_from(&code.schemas);
        self.code = code.schemas.iter().map(|s| s.name.clone()).collect();
        self.told = false;
        self.told_too_much = false;
    }

    /// Stops the game's Lust and Lua (see [`Runtime::unload`]).
    pub fn unload(&mut self) {
        self.count = None;
        let _ = self.all(|_| Job::Unload);
        self.running = None;
        self.code.clear();
        self.schemas.clear();
    }

    /// The code's components (Lust and Lua), as the editor shows them.
    pub fn schemas(&self) -> &[ComponentSchema] {
        &self.schemas
    }

    /// Whether there's a component called `name` to run.
    pub fn has_component(&self, name: &str) -> bool {
        self.rust.contains(name) || (self.code.contains(name) && !self.left_out.contains(name))
    }

    /// See [`Runtime::leave_out`].
    pub fn leave_out(&mut self, names: BTreeSet<String>) {
        self.left_out.clone_from(&names);
        self.count = None;
        let _ = self.all(|_| Job::LeaveOut(names.clone()));
    }

    /// How many components are running.
    pub fn len(&mut self) -> usize {
        if let Some(count) = self.count {
            return count;
        }
        let count = self
            .all(|_| Job::Len)
            .into_iter()
            .map(|a| match a {
                Some(Done::Len(n)) => n,
                _ => 0,
            })
            .sum();
        self.count = Some(count);
        count
    }

    /// A checksum of every running component's values (Phase 6,
    /// determinism): the same on any number of shards, as long as the
    /// game is in the same state. Asks every component, so it's for
    /// recordings and tests, not every frame.
    pub fn checksum(&mut self) -> u64 {
        // Each shard's components are its own, so their sums add up in
        // any order.
        self.all(|_| Job::Checksum)
            .into_iter()
            .fold(0_u64, |sum, a| match a {
                Some(Done::Checksum(c)) => sum.wrapping_add(c),
                _ => sum,
            })
    }

    /// Every running component's values (Lust's, Lua's and Rust's), by
    /// entity then name: what a save game keeps (decision 0038).
    pub fn all_values(&mut self) -> Vec<(EntityId, String, ComponentData)> {
        let mut all: Vec<(EntityId, String, ComponentData)> = self
            .all(|_| Job::AllValues)
            .into_iter()
            .flat_map(|a| match a {
                Some(Done::AllValues(v)) => v,
                _ => Vec::new(),
            })
            .collect();
        all.sort_by(|a, b| (a.0, &a.1).cmp(&(b.0, &b.1)));
        all
    }

    /// Whether no components are running.
    pub fn is_empty(&mut self) -> bool {
        self.len() == 0
    }

    /// The fields of `entity`'s running component `name`, as they are now.
    pub fn values(&mut self, entity: EntityId, name: &str) -> Option<ComponentData> {
        let shard = self.shard_of(entity);
        let mut here = None;
        self.shards[shard].ask(Job::Values(entity, name.to_string()), &mut here);
        match self.shards[shard].answer(&mut here) {
            Some(Done::Values(values)) => values,
            _ => None,
        }
    }

    /// [`Runtime::sync_many`], each component on its entity's shard, all
    /// at once. What their starts change lands in `world`, in order.
    pub fn sync_many<'a>(
        &mut self,
        components: impl IntoIterator<Item = (EntityId, &'a str, &'a ComponentData)>,
        world: &mut dyn GameWorld,
    ) -> Vec<Failure> {
        let mut shares: Vec<Vec<(EntityId, String, ComponentData)>> =
            (0..self.shards.len()).map(|_| Vec::new()).collect();
        // Where each was asked for, so failures are told in that order
        // whichever shard ran them.
        let mut asked: HashMap<(EntityId, String), usize> = HashMap::new();
        for (entity, name, data) in components {
            asked.insert((entity, name.to_string()), asked.len());
            shares[self.shard_of(entity)].push((entity, name.to_string(), data.clone()));
        }
        if shares.iter().all(Vec::is_empty) {
            return Vec::new();
        }
        let view = world.view();
        self.count = None;
        let answers = self.all(|i| Job::Sync(std::mem::take(&mut shares[i]), view.clone()));
        let mut failures = Vec::new();
        let mut kept = Kept::default();
        for answer in answers {
            match answer {
                Some(Done::Synced(f, k)) => {
                    failures.extend(f);
                    kept.changes.extend(k.changes);
                    kept.said.extend(k.said);
                    kept.dropped += k.dropped;
                }
                _ => failures.push(gone()),
            }
        }
        land(kept, world);
        failures.sort_by_key(|f| {
            f.entity
                .and_then(|e| asked.get(&(e, f.component.clone())).copied())
                .unwrap_or(usize::MAX)
        });
        failures
    }

    /// Stops every running component not in `wanted`.
    pub fn retain_only(&mut self, wanted: BTreeSet<(EntityId, String)>) {
        let wanted = Arc::new(wanted);
        self.count = None;
        let _ = self.all(|_| Job::RetainOnly(Arc::clone(&wanted)));
    }

    /// Stops each entity's running components not named with it, one
    /// trip to each shard for them all.
    pub fn retain_on(&mut self, entities: impl IntoIterator<Item = (EntityId, Vec<String>)>) {
        let mut shares: Vec<Vec<(EntityId, Vec<String>)>> =
            (0..self.shards.len()).map(|_| Vec::new()).collect();
        for (entity, keep) in entities {
            shares[self.shard_of(entity)].push((entity, keep));
        }
        self.count = None;
        let _ = self.all(|i| Job::RetainOn(std::mem::take(&mut shares[i])));
    }

    /// Runs one tick of `dt` seconds on every shard (see [`Runtime::tick`]):
    /// the updates, then the events in rounds, each phase reading `world`
    /// as the phase before left it.
    pub fn tick(&mut self, dt: f64, world: &mut dyn GameWorld) -> Ticked {
        let told = self.told;
        // Which round of events: spawns in different rounds may share
        // everything else about where they were asked for.
        let mut rounds = 0_u32;
        let this = std::cell::RefCell::new(&mut *self);
        let world = std::cell::RefCell::new(world);
        let (failures, broke) = in_rounds(
            || {
                let view = world.borrow_mut().view();
                this.borrow_mut().phase(
                    |_| Job::Updates(dt, view.clone()),
                    &mut **world.borrow_mut(),
                )
            },
            |events| {
                let view = world.borrow_mut().view();
                let mut game = this.borrow_mut();
                let mut shares = game.split(events);
                rounds += 1;
                let round = rounds;
                game.phase(
                    |i| Job::Round(std::mem::take(&mut shares[i]), view.clone(), round),
                    &mut **world.borrow_mut(),
                )
            },
            told,
        );
        let over = self
            .all(|_| Job::FinishTick)
            .into_iter()
            .any(|a| matches!(a, Some(Done::Finished(true))));
        if failures.iter().any(is_flood) {
            self.told = true;
        }
        Ticked {
            failures,
            pause: pause_for(over, broke),
        }
    }

    /// `events` split by the shard of the entity each is for, keeping
    /// their numbers.
    fn split(&self, events: &[(u32, Event)]) -> Vec<Vec<(u32, Event)>> {
        let mut shares: Vec<Vec<(u32, Event)>> =
            (0..self.shards.len()).map(|_| Vec::new()).collect();
        for (number, event) in events {
            shares[self.shard_of(event.to)].push((*number, event.clone()));
        }
        shares
    }

    /// One phase on every shard, the answers put together, and what they
    /// changed landed in `world`, in order.
    fn phase(
        &mut self,
        job: impl FnMut(usize) -> Job,
        world: &mut dyn GameWorld,
    ) -> Result<Phase, String> {
        let mut phase = Phase::default();
        let mut kept = Kept::default();
        let mut broke = None;
        for answer in self.all(job) {
            match answer {
                Some(Done::Phase(Ok(p), k)) => {
                    phase.sent.extend(p.sent);
                    phase.failures.extend(p.failures);
                    kept.changes.extend(k.changes);
                    kept.said.extend(k.said);
                    kept.dropped += k.dropped;
                }
                Some(Done::Phase(Err(why), _)) => broke = Some(why),
                _ => broke = Some(gone().at.message),
            }
        }
        if kept.dropped > 0 && !self.told_too_much {
            self.told_too_much = true;
            phase.failures.push((
                Order::default(),
                Failure {
                    entity: None,
                    component: String::new(),
                    at: crate::code::CodeProblem {
                        file: String::new(),
                        line: 0,
                        column: 0,
                        message: format!(
                            "Game code changed things more than {MAX_KEPT} times in one go, so \
                             {} changes were left out (is something calling moveTo, fire or \
                             spawn in a loop?)",
                            kept.dropped
                        ),
                    },
                    stopped: false,
                },
            ));
        }
        land(kept, world);
        match broke {
            Some(why) => Err(why),
            None => Ok(phase),
        }
    }
}

/// Puts what game code changed and said into `world`, in the tick's order.
fn land(mut kept: Kept, world: &mut dyn GameWorld) {
    kept.changes.sort_by(|a, b| a.0.cmp(&b.0));
    kept.said.sort_by(|a, b| a.0.cmp(&b.0));
    let changes: Vec<Change> = kept.changes.into_iter().map(|(_, c)| c).collect();
    if !changes.is_empty() {
        world.apply(&changes);
    }
    for (_, words) in kept.said {
        world.say(&words);
    }
}

#[cfg(test)]
#[path = "game_tests.rs"]
mod tests;
