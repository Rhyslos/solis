//! Named timers (stage 1, timers; [decision 0124](../../../docs/decisions/0124-named-timers-now-blocks-later.md)):
//! "open in 2 seconds", "every 5 seconds", "each day at six". A component
//! starts one by name (`startTimer(Timer.DoorOpens, 2.0)`), and hears
//! `TimerFinished(Timer which)` when it's due, in the first round of a
//! tick's events.
//!
//! **Game time, counted exactly.** A timer's seconds become whole ticks
//! once, as it starts (rounded to the nearest), and the store counts game
//! time in whole numbers: each tick adds the game's speed in 65,536ths of
//! a tick ([`UNITS_PER_TICK`]). So a timer follows `setGameSpeed`, stands
//! still while the speed is 0, and never drifts the way adding up seconds
//! in floating point would. A repeating timer's next turn is its last one
//! plus its interval, never "now plus the interval".
//!
//! **Order.** What's due in a tick fires in the order (due, entity,
//! component name, start order), however many cores run the game.
//!
//! **Kept by whose it is:** (entity, component, the timer's name), so
//! starting a running timer starts it again, and a timer belongs to the
//! component that started it. The store is ready for blocks run later
//! (`after 2.0 { … }`, a later milestone): they are pending work with the
//! same order and owners.

use reflect::EntityId;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

#[path = "timer_text.rs"]
mod text;

/// A tick, in seconds of game time at speed 1 (the world's `TICK`).
pub const TICK: f64 = 1.0 / 60.0;

/// One tick of game time in the units timers count in: a game speed that
/// is a whole number of 65,536ths counts exactly (0.5, 2, 0.25, 1.5).
pub const UNITS_PER_TICK: u64 = 1 << 16;

/// The longest timer, in seconds of game time: about 115 days.
pub const MAX_SECONDS: f64 = 10_000_000.0;

/// The most timers waiting at once, in the whole game. Past it a start is
/// left out, and the game says so once.
pub const MAX_TIMERS: usize = 1 << 20;

/// The longest name a timer may have.
pub const MAX_NAME: usize = 64;

/// The fastest game speed the timers count (the world's own limit is
/// lower).
const MAX_SPEED: f64 = 1_000.0;

/// When a timer finishes, as game code asks for it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Timing {
    /// Once, this many seconds of game time from now (`startTimer`).
    After(f64),
    /// Again and again, every this many seconds (`startTimerEvery`).
    Every(f64),
    /// Each day, as the day's clock passes this hour (`startTimerAtHour`).
    AtHour(f64),
}

/// A timer's timing as the store keeps it: seconds made whole ticks, an
/// hour within the day.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Plan {
    /// Once, this many ticks of game time from now (0: the next tick).
    After(u64),
    /// Every this many ticks (at least one).
    Every(u64),
    /// Each day at this hour (0 up to 24).
    AtHour(f64),
}

impl Timing {
    /// The timing as the store keeps it, or why it can't be: seconds made
    /// whole ticks (the nearest; a repeating timer at least one), an hour
    /// wrapped into the day as `setTimeOfDay` wraps it.
    pub fn plan(self) -> Result<Plan, String> {
        let ticks = |seconds: f64| -> Result<u64, String> {
            if !seconds.is_finite() || seconds < 0.0 {
                return Err(format!(
                    "a timer's time is a number of seconds from 0 up, and {seconds} isn't one"
                ));
            }
            if seconds > MAX_SECONDS {
                return Err(format!(
                    "a timer can wait at most {MAX_SECONDS} seconds (about 115 days), not \
                     {seconds}"
                ));
            }
            Ok((seconds / TICK).round() as u64)
        };
        Ok(match self {
            Self::After(seconds) => Plan::After(ticks(seconds)?),
            Self::Every(seconds) => Plan::Every(ticks(seconds)?.max(1)),
            Self::AtHour(hour) => {
                if !hour.is_finite() {
                    return Err(format!("a timer's hour is a number, and {hour} isn't one"));
                }
                Plan::AtHour(within_day(hour))
            }
        })
    }
}

/// Whether `name` can be a timer's name: a capital letter, then letters,
/// digits and `_`, at most [`MAX_NAME`] (the shape of a Lust state's
/// option, so Lua and Rust name timers as Lust does).
pub fn valid_name(name: &str) -> bool {
    name.len() <= MAX_NAME
        && name.chars().next().is_some_and(|c| c.is_ascii_uppercase())
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// What a timer does: started with a plan, or stopped.
#[derive(Debug, Clone, PartialEq)]
pub struct TimerChange {
    /// The entity whose component it is.
    pub entity: EntityId,
    /// The component that started it.
    pub component: Arc<str>,
    /// Its name.
    pub name: Arc<str>,
    /// Started (again) with this plan, or stopped (`None`).
    pub plan: Option<Plan>,
}

/// A timer that finished this tick: whose it is, and its name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fired {
    /// The entity whose component started it.
    pub entity: EntityId,
    /// The component that started it, which alone hears it.
    pub component: Arc<str>,
    /// Its name (`DoorOpens`).
    pub name: Arc<str>,
}

/// How a waiting timer finishes.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Kind {
    Once,
    /// Again every this many units.
    Every(u64),
    /// Each day at this hour.
    AtHour(f64),
}

/// One waiting timer.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Slot {
    /// Its place in the order of starts (ties in a tick go by it).
    start: u64,
    kind: Kind,
    /// When it's due, in units (unused for one at an hour).
    due: u64,
}

/// A timer's owner and name.
type Key = (EntityId, Arc<str>, Arc<str>);

/// The timers waiting for game time to pass, in due order: (due, entity,
/// component, start, name).
type Queue = BTreeSet<(u64, EntityId, Arc<str>, u64, Arc<str>)>;

/// The timers waiting for an hour: (entity, component, start) to (name,
/// hour).
type Hourly = BTreeMap<(EntityId, Arc<str>, u64), (Arc<str>, f64)>;

/// Every timer waiting in a game: game state, saved, in the checksum, the
/// same on any number of cores.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Timers {
    /// Game time in units since the first timer of the ones waiting
    /// started (0 while none waits, so a game and a loaded save of it
    /// count alike).
    now: u64,
    /// The day's hour as the last tick began (none before the first).
    hour: Option<f64>,
    /// The next start's place in the order.
    next_start: u64,
    slots: HashMap<Key, Slot>,
    queue: Queue,
    hourly: Hourly,
}

impl Timers {
    /// How many timers wait.
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// Whether none waits.
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// Makes `change`: a timer started (again, if it runs: from the new
    /// time) or stopped. False when a start is left out, past
    /// [`MAX_TIMERS`].
    pub fn apply(&mut self, change: &TimerChange) -> bool {
        let key: Key = (
            change.entity,
            Arc::clone(&change.component),
            Arc::clone(&change.name),
        );
        let Some(plan) = change.plan else {
            self.remove(&key);
            self.settle();
            return true;
        };
        if self.slots.len() >= MAX_TIMERS && !self.slots.contains_key(&key) {
            return false;
        }
        self.remove(&key);
        let start = self.next_start;
        self.next_start += 1;
        let (kind, due) = match plan {
            Plan::After(ticks) => (Kind::Once, self.now.saturating_add(units(ticks))),
            Plan::Every(ticks) => {
                let every = units(ticks.max(1));
                (Kind::Every(every), self.now.saturating_add(every))
            }
            Plan::AtHour(hour) => (Kind::AtHour(hour), 0),
        };
        self.place(key, Slot { start, kind, due });
        true
    }

    /// A tick begins at game speed `speed` with the day at `hour`: game
    /// time moves on, and what's due fires, in order. A repeating timer
    /// fires at most once a tick (if it fell behind, as a game speed far
    /// above its interval makes it, it fires once and keeps its rhythm);
    /// one at an hour fires once as the day passes its hour, even when
    /// `setTimeOfDay` jumped over it.
    pub fn advance(&mut self, speed: f64, hour: f64) -> Vec<Fired> {
        let hour = if hour.is_finite() {
            within_day(hour)
        } else {
            self.hour.unwrap_or(0.0)
        };
        let before = self.hour.replace(hour);
        if self.slots.is_empty() {
            self.settle();
            return Vec::new();
        }
        let step = if speed.is_finite() && speed > 0.0 {
            (speed.min(MAX_SPEED) * UNITS_PER_TICK as f64).round() as u64
        } else {
            0
        };
        self.now = self.now.saturating_add(step);
        let mut fired: Vec<(u64, EntityId, Arc<str>, u64, Arc<str>)> = Vec::new();
        while self.queue.first().is_some_and(|first| first.0 <= self.now) {
            let Some(entry) = self.queue.pop_first() else {
                break;
            };
            let key: Key = (entry.1, Arc::clone(&entry.2), Arc::clone(&entry.4));
            match self.slots.get(&key).map(|slot| slot.kind) {
                Some(Kind::Every(every)) => {
                    // From its last turn, moved on by whole turns past now.
                    let mut due = entry.0.saturating_add(every);
                    if due <= self.now {
                        due = due.saturating_add((self.now - due) / every * every + every);
                    }
                    if let Some(slot) = self.slots.get_mut(&key) {
                        slot.due = due;
                    }
                    self.queue.insert((
                        due,
                        entry.1,
                        Arc::clone(&entry.2),
                        entry.3,
                        Arc::clone(&entry.4),
                    ));
                }
                _ => {
                    self.slots.remove(&key);
                }
            }
            fired.push(entry);
        }
        if let Some(before) = before {
            let passed = (hour - before).rem_euclid(24.0);
            if passed > 0.0 {
                for ((entity, component, start), (name, at)) in &self.hourly {
                    let to = (at - before).rem_euclid(24.0);
                    if to > 0.0 && to <= passed {
                        fired.push((
                            self.now,
                            *entity,
                            Arc::clone(component),
                            *start,
                            Arc::clone(name),
                        ));
                    }
                }
            }
        }
        fired.sort_unstable_by(|a, b| (a.0, a.1, &a.2, a.3).cmp(&(b.0, b.1, &b.2, b.3)));
        self.settle();
        fired
            .into_iter()
            .map(|(_, entity, component, _, name)| Fired {
                entity,
                component,
                name,
            })
            .collect()
    }

    /// Whether `component` on `entity` has a timer called `name` waiting.
    pub fn running(&self, entity: EntityId, component: &str, name: &str) -> bool {
        self.slot(entity, component, name).is_some()
    }

    /// The seconds of game time left before `component` on `entity`'s
    /// timer `name` finishes, with the day at `hour` and lasting
    /// `day_length` seconds of game time; 0 when it isn't running.
    pub fn left(
        &self,
        (entity, component, name): (EntityId, &str, &str),
        hour: f64,
        day_length: f64,
    ) -> f64 {
        match self.slot(entity, component, name) {
            None => 0.0,
            Some(Slot {
                kind: Kind::AtHour(at),
                ..
            }) => {
                let mut to = (at - within_day(hour)).rem_euclid(24.0);
                if to <= 0.0 {
                    to = 24.0;
                }
                to / 24.0 * day_length.max(0.0)
            }
            Some(slot) => slot.due.saturating_sub(self.now) as f64 / UNITS_PER_TICK as f64 * TICK,
        }
    }

    /// Ends every timer whose entity and component `keep` says no to
    /// (despawned, or the component taken off), and gives back how many.
    pub fn retain(&mut self, mut keep: impl FnMut(EntityId, &str) -> bool) -> usize {
        self.end_where(|entity, component, _| !keep(entity, component))
            .len()
    }

    /// Ends every timer `gone` says yes to, by (entity, component, name),
    /// and gives back which, in order.
    pub fn end_where(
        &mut self,
        mut gone: impl FnMut(EntityId, &str, &str) -> bool,
    ) -> Vec<(EntityId, Arc<str>, Arc<str>)> {
        let mut ended: Vec<Key> = self
            .slots
            .keys()
            .filter(|(entity, component, name)| gone(*entity, component, name))
            .cloned()
            .collect();
        ended.sort();
        for key in &ended {
            self.remove(key);
        }
        self.settle();
        ended
    }

    /// A checksum of every waiting timer, or none while none waits (so a
    /// game from before timers keeps its checksums).
    pub fn checksum(&self) -> Option<u64> {
        if self.slots.is_empty() {
            return None;
        }
        let mut sum = reflect::Checksum::new();
        sum.u64(self.now);
        sum.f64(self.hour.unwrap_or(-1.0));
        sum.u64(self.next_start);
        for (due, entity, component, start, name) in &self.queue {
            sum.u64(*due);
            sum.u64(entity.0.raw());
            sum.bytes(component.as_bytes());
            sum.u64(*start);
            sum.bytes(name.as_bytes());
            if let Some(Slot {
                kind: Kind::Every(every),
                ..
            }) = self
                .slots
                .get(&(*entity, Arc::clone(component), Arc::clone(name)))
            {
                sum.u64(*every);
            }
        }
        for ((entity, component, start), (name, hour)) in &self.hourly {
            sum.u64(entity.0.raw());
            sum.bytes(component.as_bytes());
            sum.u64(*start);
            sum.bytes(name.as_bytes());
            sum.f64(*hour);
        }
        Some(sum.finish())
    }

    fn slot(&self, entity: EntityId, component: &str, name: &str) -> Option<&Slot> {
        // Most components have few timers: a key made of the names, looked
        // up once.
        self.slots
            .get(&(entity, Arc::from(component), Arc::from(name)))
    }

    /// Puts a timer in its place in the order.
    fn place(&mut self, key: Key, slot: Slot) {
        match slot.kind {
            Kind::AtHour(hour) => {
                self.hourly.insert(
                    (key.0, Arc::clone(&key.1), slot.start),
                    (Arc::clone(&key.2), hour),
                );
            }
            Kind::Once | Kind::Every(_) => {
                self.queue.insert((
                    slot.due,
                    key.0,
                    Arc::clone(&key.1),
                    slot.start,
                    Arc::clone(&key.2),
                ));
            }
        }
        self.slots.insert(key, slot);
    }

    /// Takes a timer out, if it waits.
    fn remove(&mut self, key: &Key) {
        let Some(slot) = self.slots.remove(key) else {
            return;
        };
        match slot.kind {
            Kind::AtHour(_) => {
                self.hourly.remove(&(key.0, Arc::clone(&key.1), slot.start));
            }
            Kind::Once | Kind::Every(_) => {
                self.queue.remove(&(
                    slot.due,
                    key.0,
                    Arc::clone(&key.1),
                    slot.start,
                    Arc::clone(&key.2),
                ));
            }
        }
    }

    /// With nothing waiting, the clock and the order of starts begin
    /// again from 0: a game, and the same game saved and loaded while
    /// nothing waited, then count alike.
    fn settle(&mut self) {
        if self.slots.is_empty() {
            self.now = 0;
            self.next_start = 0;
        }
    }
}

/// `ticks` of game time in units.
fn units(ticks: u64) -> u64 {
    ticks.saturating_mul(UNITS_PER_TICK)
}

/// `hours` wrapped into 0 up to (not including) 24, as the world wraps
/// the time of day.
fn within_day(hours: f64) -> f64 {
    let h = hours.rem_euclid(24.0);
    if h >= 24.0 { 0.0 } else { h }
}

#[cfg(test)]
#[path = "timers_tests.rs"]
mod tests;
