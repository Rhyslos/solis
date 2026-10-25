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
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Change {
    /// `moveTo`: put the entity at this place in the world.
    Move(EntityId, Vec3),
    /// `turnTo`: turn the entity to face this way in the world.
    Turn(EntityId, Quat),
}

/// The entities of a [`WorldView`]: the same while the world's entities,
/// their order and their names are.
#[derive(Debug, Default)]
pub struct ViewIndex {
    /// Each entity's number in [`WorldView::places`].
    slots: HashMap<EntityId, u32>,
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
}

/// The host a shard's game code reaches during a phase: it reads the
/// phase's [`WorldView`] (and what the running component changed itself),
/// and keeps what it changes.
struct ViewHost {
    view: WorldView,
    /// The component running now, and where what it does belongs.
    at: Order,
    /// What it has done so far (its order, counted on).
    next: u32,
    /// What the running component changed this phase, newest last.
    own: Vec<Change>,
    kept: Kept,
}

impl ViewHost {
    fn new(view: WorldView) -> Self {
        Self {
            view,
            at: Order::default(),
            next: 0,
            own: Vec::new(),
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
        let order = self.order();
        self.kept.said.push((order, words.to_string()));
    }

    fn time(&self) -> f64 {
        self.view.time
    }

    fn find(&self, name: &str) -> Option<EntityId> {
        self.view.index.first_named.get(name).copied()
    }

    fn position(&self, which: EntityId) -> Option<Vec3> {
        let moved = self.own.iter().rev().find_map(|c| match c {
            Change::Move(e, place) if *e == which => Some(*place),
            _ => None,
        });
        moved.or_else(|| self.view.place(which).map(|m| m.w_axis.truncate()))
    }

    fn move_to(&mut self, which: EntityId, place: Vec3) {
        if self.view.index.slots.contains_key(&which) {
            let order = self.order();
            self.kept.changes.push((order, Change::Move(which, place)));
            self.own.push(Change::Move(which, place));
        }
    }

    fn rotation(&self, which: EntityId) -> Option<Quat> {
        let turned = self.own.iter().rev().find_map(|c| match c {
            Change::Turn(e, facing) if *e == which => Some(*facing),
            _ => None,
        });
        turned.or_else(|| {
            self.view
                .place(which)
                .map(|m| m.to_scale_rotation_translation().1.normalize())
        })
    }

    fn turn_to(&mut self, which: EntityId, facing: Quat) {
        if self.view.index.slots.contains_key(&which) {
            let order = self.order();
            self.kept.changes.push((order, Change::Turn(which, facing)));
            self.own.push(Change::Turn(which, facing));
        }
    }

    fn name(&self, which: EntityId) -> Option<String> {
        let slot = *self.view.index.slots.get(&which)?;
        self.view.index.names.get(slot as usize).cloned()
    }

    fn at(&mut self, order: &Order) {
        self.at = order.clone();
        self.next = 0;
        self.own.clear();
    }
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
    StartTick,
    Updates(f64, WorldView),
    Round(Vec<(u32, Event)>, WorldView),
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
            let mut host = ViewHost::new(view);
            let loaded = runtime.load(&code, &mut host);
            Done::Loaded(loaded, host.kept)
        }
        Job::Swap(code, view) => {
            let mut host = ViewHost::new(view);
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
            let mut host = ViewHost::new(view);
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
        Job::StartTick => {
            runtime.start_tick();
            Done::Nothing
        }
        Job::Updates(dt, view) => {
            let mut host = ViewHost::new(view);
            let phase = runtime.run_updates(dt, &mut host);
            Done::Phase(phase, host.kept)
        }
        Job::Round(events, view) => {
            let mut host = ViewHost::new(view);
            let phase = runtime.run_round(&events, &mut host);
            Done::Phase(phase, host.kept)
        }
        Job::FinishTick => Done::Finished(runtime.finish_tick()),
    }
}

/// A share of the entities, and the runtime running their components.
enum Shard {
    /// On the caller's thread (the only shard).
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

impl Game {
    /// Game code on `cores` shards (at least one). Fails only if Luau
    /// can't set itself up (out of memory). If a shard's thread can't
    /// start, the game runs on fewer.
    pub fn new(cores: usize) -> Result<Self, Failure> {
        let cores = cores.clamp(1, 64);
        let shards = if cores == 1 {
            vec![Shard::Here(Box::new(Runtime::new()?))]
        } else {
            let away: Vec<Shard> = (0..cores).filter_map(Shard::away).collect();
            if away.is_empty() {
                vec![Shard::Here(Box::new(Runtime::new()?))]
            } else {
                away
            }
        };
        Ok(Self {
            shards,
            rust: BTreeSet::new(),
            code: BTreeSet::new(),
            schemas: Vec::new(),
            left_out: BTreeSet::new(),
            told: false,
        })
    }

    /// How many shards it runs on.
    pub fn shards(&self) -> usize {
        self.shards.len()
    }

    /// Which shard runs `entity`'s components: by its ID, so the same
    /// whatever else is running.
    fn shard_of(&self, entity: EntityId) -> usize {
        (entity.0.raw() % self.shards.len() as u64) as usize
    }

    /// Gives every shard `job` (made for each), and their answers in
    /// shard order. The shards work at the same time.
    fn all(&mut self, mut job: impl FnMut(usize) -> Job) -> Vec<Option<Done>> {
        let mut here: Vec<Option<Done>> = Vec::new();
        here.resize_with(self.shards.len(), || None);
        for (i, shard) in self.shards.iter_mut().enumerate() {
            shard.ask(job(i), &mut here[i]);
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
        let answers = self.all(|_| Job::Load(Arc::clone(code), view.clone()));
        let mut first = None;
        for answer in answers {
            match answer {
                Some(Done::Loaded(result, kept)) => {
                    result?;
                    first.get_or_insert(kept);
                }
                _ => return Err(gone()),
            }
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
        let answers = self.all(|_| Job::Swap(Arc::clone(code), view.clone()));
        let mut merged: Option<Swapped> = None;
        let mut first = None;
        for answer in answers {
            let swapped = match answer {
                Some(Done::Swapped(result, kept)) => {
                    first.get_or_insert(kept);
                    result?
                }
                _ => return Err(gone()),
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
        if let Some(kept) = first {
            land(kept, world);
        }
        self.took(code);
        Ok(merged.unwrap_or_default())
    }

    /// Notes the components `code` has.
    fn took(&mut self, code: &Compiled) {
        self.schemas.clone_from(&code.schemas);
        self.code = code.schemas.iter().map(|s| s.name.clone()).collect();
        self.told = false;
    }

    /// Stops the game's Lust and Lua (see [`Runtime::unload`]).
    pub fn unload(&mut self) {
        let _ = self.all(|_| Job::Unload);
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
        let _ = self.all(|_| Job::LeaveOut(names.clone()));
    }

    /// How many components are running.
    pub fn len(&mut self) -> usize {
        self.all(|_| Job::Len)
            .into_iter()
            .map(|a| match a {
                Some(Done::Len(n)) => n,
                _ => 0,
            })
            .sum()
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
        for (entity, name, data) in components {
            shares[self.shard_of(entity)].push((entity, name.to_string(), data.clone()));
        }
        if shares.iter().all(Vec::is_empty) {
            return Vec::new();
        }
        let view = world.view();
        let answers = self.all(|i| Job::Sync(std::mem::take(&mut shares[i]), view.clone()));
        let mut failures = Vec::new();
        let mut kept = Kept::default();
        for answer in answers {
            match answer {
                Some(Done::Synced(f, k)) => {
                    failures.extend(f);
                    kept.changes.extend(k.changes);
                    kept.said.extend(k.said);
                }
                _ => failures.push(gone()),
            }
        }
        land(kept, world);
        failures
    }

    /// Stops every running component not in `wanted`.
    pub fn retain_only(&mut self, wanted: BTreeSet<(EntityId, String)>) {
        let wanted = Arc::new(wanted);
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
        let _ = self.all(|i| Job::RetainOn(std::mem::take(&mut shares[i])));
    }

    /// Runs one tick of `dt` seconds on every shard (see [`Runtime::tick`]):
    /// the updates, then the events in rounds, each phase reading `world`
    /// as the phase before left it.
    pub fn tick(&mut self, dt: f64, world: &mut dyn GameWorld) -> Ticked {
        let _ = self.all(|_| Job::StartTick);
        let told = self.told;
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
                let shares = game.split(events);
                let mut shares = shares.into_iter();
                game.phase(
                    |_| Job::Round(shares.next().unwrap_or_default(), view.clone()),
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
                }
                Some(Done::Phase(Err(why), _)) => broke = Some(why),
                _ => broke = Some(gone().at.message),
            }
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
