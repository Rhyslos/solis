//! Where the timers wait ([`Timers`]): each in a slot of a list, found by
//! its owner and name through a map of numbers, and filed under the game
//! time it's due in a map of buckets. Finishing a tick's timers takes the
//! buckets due off the front whole, so 10,000 timers finishing in a tick
//! cost a few lookups each, never a search; their order is put right once,
//! at the end ((due, entity, component name, start)).
//!
//! A timer started again leaves its old entry in its bucket, known as old
//! by its start (each start has its own number), and dropped when the
//! bucket comes due, or sooner, when old entries outnumber the timers.

use super::{
    Fired, MAX_SPEED, MAX_TIMERS, Plan, TICK, TimerChange, UNITS_PER_TICK, units, within_day,
};
use reflect::EntityId;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

/// How a waiting timer finishes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Kind {
    Once,
    /// Again every this many units.
    Every(u64),
    /// Each day at this hour.
    AtHour(f64),
}

/// One waiting timer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Entry {
    pub(super) entity: EntityId,
    /// Its component's name, by number ([`Words`]).
    pub(super) component: u32,
    /// Its name, by number.
    pub(super) name: u32,
    /// Its place in the order of starts: ties in a tick go by it, and no
    /// two starts share one.
    pub(super) start: u64,
    pub(super) kind: Kind,
    /// When it's due, in units (0 for one at an hour).
    pub(super) due: u64,
}

/// The names of components and timers, each kept once and known by its
/// number: comparing and hashing numbers is what keeps finishing cheap.
#[derive(Debug, Clone, Default)]
struct Words {
    list: Vec<Arc<str>>,
    numbers: HashMap<Arc<str>, u32, Quick>,
    /// The words numbered last, by where their text lives: a tick's
    /// starts mostly hand over the same few names again and again.
    recent: [(usize, u32); 8],
    /// Where the next one numbered goes in `recent`.
    turn: usize,
}

impl Words {
    /// `word`'s number, given one if it's new.
    fn number(&mut self, word: &Arc<str>) -> u32 {
        // The same text in memory is the same word; a word's own copy in
        // `list` keeps it from being reused for another while numbered.
        let at = Arc::as_ptr(word).cast::<u8>() as usize;
        if let Some(&(_, n)) = self.recent.iter().find(|(p, _)| *p == at) {
            if *self.list[n as usize] == **word {
                return n;
            }
        }
        if let Some(&n) = self.numbers.get(word) {
            self.recent[self.turn] = (at, n);
            self.turn = (self.turn + 1) % self.recent.len();
            return n;
        }
        let n = self.list.len() as u32;
        self.list.push(Arc::clone(word));
        self.numbers.insert(Arc::clone(word), n);
        n
    }

    /// `word`'s number, if it has one.
    fn find(&self, word: &str) -> Option<u32> {
        self.numbers.get(word).copied()
    }

    fn word(&self, n: u32) -> &Arc<str> {
        &self.list[n as usize]
    }
}

/// A quick hash for the store's keys (numbers and short names): Rust's
/// default, made to resist floods of chosen keys, costs more than the
/// rest of a finishing timer, and game code's own names are no such
/// threat.
#[derive(Debug, Clone, Copy, Default)]
struct QuickHash(u64);

impl std::hash::Hasher for QuickHash {
    fn write(&mut self, bytes: &[u8]) {
        for chunk in bytes.chunks(8) {
            let mut word = [0_u8; 8];
            word[..chunk.len()].copy_from_slice(chunk);
            self.mix(u64::from_le_bytes(word));
        }
    }

    fn write_u64(&mut self, n: u64) {
        self.mix(n);
    }

    fn write_u32(&mut self, n: u32) {
        self.mix(u64::from(n));
    }

    fn write_usize(&mut self, n: usize) {
        self.mix(n as u64);
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

impl QuickHash {
    fn mix(&mut self, word: u64) {
        self.0 = (self.0.rotate_left(5) ^ word).wrapping_mul(0x517c_c1b7_2722_0a95);
    }
}

type Quick = std::hash::BuildHasherDefault<QuickHash>;

/// A timer's owner and name, by numbers.
type Key = (EntityId, u32, u32);

/// Every timer waiting in a game: game state, saved, in the checksum, the
/// same on any number of cores.
#[derive(Debug, Clone, Default)]
pub struct Timers {
    /// Game time in units since the first of the timers waiting started
    /// (0 while none waits, so a game and a loaded save of it count
    /// alike).
    pub(super) now: u64,
    /// The day's hour as the last tick began (none before the first).
    pub(super) hour: Option<f64>,
    /// The next start's place in the order.
    pub(super) next_start: u64,
    words: Words,
    /// Each timer's slot (empty slots are reused).
    slots: Vec<Option<Entry>>,
    /// Each owner and name's slot: kept when its timer ends, so starting
    /// it again (a handler restarting it) finds its slot without a new
    /// one; slots of ended timers are let go when they outnumber the rest.
    by_key: HashMap<Key, u32, Quick>,
    /// How many slots hold a timer.
    live: usize,
    /// Slots filed under the game time they're due, with their start
    /// (an entry whose slot has another start is old).
    buckets: BTreeMap<u64, Vec<(u32, u64)>>,
    /// Entries in the buckets, old ones too.
    filed: usize,
    /// Emptied buckets' room, kept for the next ones.
    spare: Vec<Vec<(u32, u64)>>,
    /// The slots of the timers at an hour.
    hourly: Vec<u32>,
}

impl PartialEq for Timers {
    /// Alike when they'd write the same save and stand at the same hour.
    fn eq(&self, other: &Self) -> bool {
        self.now == other.now
            && self.hour.map(f64::to_bits) == other.hour.map(f64::to_bits)
            && self.next_start == other.next_start
            && self.write() == other.write()
    }
}

impl Timers {
    /// How many timers wait.
    pub fn len(&self) -> usize {
        self.live
    }

    /// Whether none waits.
    pub fn is_empty(&self) -> bool {
        self.live == 0
    }

    /// Makes `change`: a timer started (again, if it runs: from the new
    /// time) or stopped. False when a start is left out, past
    /// [`MAX_TIMERS`].
    pub fn apply(&mut self, change: &TimerChange) -> bool {
        let component = self.words.number(&change.component);
        let name = self.words.number(&change.name);
        let key = (change.entity, component, name);
        let Some(plan) = change.plan else {
            self.remove(key);
            self.settle();
            self.tidy();
            return true;
        };
        let slot = self.by_key.get(&key).copied();
        let running = slot.and_then(|slot| self.slots[slot as usize]);
        if self.live >= MAX_TIMERS && running.is_none() {
            return false;
        }
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
        let entry = Entry {
            entity: change.entity,
            component,
            name,
            start,
            kind,
            due,
        };
        // Started again: the same slot, filed anew (its old entry is old
        // by its start).
        if let Some(slot) =
            slot.filter(|_| running.is_some_and(|w| matches!(w.kind, Kind::AtHour(_))))
        {
            self.remove_slot(slot);
        }
        self.place(entry, slot);
        self.tidy();
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
        if self.live == 0 {
            self.settle();
            return Vec::new();
        }
        let step = if speed.is_finite() && speed > 0.0 {
            (speed.min(MAX_SPEED) * UNITS_PER_TICK as f64).round() as u64
        } else {
            0
        };
        self.now = self.now.saturating_add(step);
        let mut fired: Vec<(u64, Entry)> = Vec::new();
        while self
            .buckets
            .first_key_value()
            .is_some_and(|(due, _)| *due <= self.now)
        {
            let Some((due, mut filed)) = self.buckets.pop_first() else {
                break;
            };
            self.filed -= filed.len();
            fired.reserve(filed.len());
            for &(slot, start) in &filed {
                if let Some(entry) = self.current(slot, start, due) {
                    fired.push((due, entry));
                    self.finished(slot, entry);
                }
            }
            filed.clear();
            self.spare.push(filed);
        }
        if let Some(before) = before {
            self.passed_hours(before, hour, &mut fired);
        }
        let words = &self.words;
        fired.sort_unstable_by(|(due_a, a), (due_b, b)| {
            (due_a, a.entity, words.word(a.component), a.start).cmp(&(
                due_b,
                b.entity,
                words.word(b.component),
                b.start,
            ))
        });
        let fired = fired
            .into_iter()
            .map(|(_, entry)| Fired {
                entity: entry.entity,
                component: Arc::clone(words.word(entry.component)),
                name: Arc::clone(words.word(entry.name)),
            })
            .collect();
        self.settle();
        self.tidy();
        fired
    }

    /// Whether `component` on `entity` has a timer called `name` waiting.
    pub fn running(&self, entity: EntityId, component: &str, name: &str) -> bool {
        self.entry(entity, component, name).is_some()
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
        match self.entry(entity, component, name) {
            None => 0.0,
            Some(Entry {
                kind: Kind::AtHour(at),
                ..
            }) => {
                let mut to = (at - within_day(hour)).rem_euclid(24.0);
                if to <= 0.0 {
                    to = 24.0;
                }
                to / 24.0 * day_length.max(0.0)
            }
            Some(entry) => entry.due.saturating_sub(self.now) as f64 / UNITS_PER_TICK as f64 * TICK,
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
        let mut ended: Vec<(EntityId, Arc<str>, Arc<str>)> = Vec::new();
        let mut keys: Vec<Key> = Vec::new();
        for (&(entity, component, name), &slot) in &self.by_key {
            if self.slots[slot as usize].is_none() {
                continue;
            }
            let (c, n) = (self.words.word(component), self.words.word(name));
            if gone(entity, c, n) {
                ended.push((entity, Arc::clone(c), Arc::clone(n)));
                keys.push((entity, component, name));
            }
        }
        for key in keys {
            self.remove(key);
        }
        ended.sort();
        self.settle();
        self.tidy();
        ended
    }

    /// A checksum of every waiting timer, or none while none waits (so a
    /// game from before timers keeps its checksums). Each timer's sum is
    /// added up, so the order they're kept in doesn't count.
    pub fn checksum(&self) -> Option<u64> {
        if self.live == 0 {
            return None;
        }
        let mut total = reflect::Checksum::new();
        total.u64(self.now);
        total.f64(self.hour.unwrap_or(-1.0));
        total.u64(self.next_start);
        let mut sum = total.finish();
        for entry in self.slots.iter().flatten() {
            let mut one = reflect::Checksum::new();
            one.u64(entry.entity.0.raw());
            one.bytes(self.words.word(entry.component).as_bytes());
            one.bytes(self.words.word(entry.name).as_bytes());
            one.u64(entry.start);
            one.u64(entry.due);
            match entry.kind {
                Kind::Once => one.u64(0),
                Kind::Every(every) => one.u64(every),
                Kind::AtHour(hour) => one.f64(hour),
            }
            sum = sum.wrapping_add(one.finish());
        }
        Some(sum)
    }

    /// Every timer waiting, as (entity, component, name, entry), in the
    /// order a save writes them: due first, the ones at an hour last.
    pub(super) fn entries(&self) -> Vec<(EntityId, &str, &str, Entry)> {
        let mut all: Vec<(EntityId, &str, &str, Entry)> = self
            .slots
            .iter()
            .flatten()
            .map(|e| {
                (
                    e.entity,
                    &**self.words.word(e.component),
                    &**self.words.word(e.name),
                    *e,
                )
            })
            .collect();
        all.sort_by(|a, b| {
            let hourly = |e: &Entry| matches!(e.kind, Kind::AtHour(_));
            (hourly(&a.3), a.3.due, a.0, a.1, a.3.start).cmp(&(
                hourly(&b.3),
                b.3.due,
                b.0,
                b.1,
                b.3.start,
            ))
        });
        all
    }

    /// Puts a timer a save kept in its place; false when its owner has a
    /// timer of that name already.
    pub(super) fn restore(
        &mut self,
        (entity, component, name): (EntityId, &str, &str),
        start: u64,
        kind: Kind,
        due: u64,
    ) -> bool {
        let component = self.words.number(&Arc::from(component));
        let name = self.words.number(&Arc::from(name));
        let key = (entity, component, name);
        if self.entry_of(key).is_some() {
            return false;
        }
        let slot = self.by_key.get(&key).copied();
        self.place(
            Entry {
                entity,
                component,
                name,
                start,
                kind,
                due,
            },
            slot,
        );
        true
    }

    fn entry(&self, entity: EntityId, component: &str, name: &str) -> Option<Entry> {
        self.entry_of((entity, self.words.find(component)?, self.words.find(name)?))
    }

    fn entry_of(&self, key: Key) -> Option<Entry> {
        let slot = *self.by_key.get(&key)?;
        self.slots[slot as usize]
    }

    /// The timer in `slot`, if it's still the one started at `start` and
    /// due at `due` (not stopped, started again or moved on since).
    fn current(&self, slot: u32, start: u64, due: u64) -> Option<Entry> {
        self.slots[slot as usize]
            .filter(|e| e.start == start && e.due == due && !matches!(e.kind, Kind::AtHour(_)))
    }

    /// A timer in `slot` has finished: a repeating one is filed again,
    /// from its last turn and moved on by whole turns past now; any other
    /// ends.
    fn finished(&mut self, slot: u32, entry: Entry) {
        let Kind::Every(every) = entry.kind else {
            self.remove_slot(slot);
            return;
        };
        let mut due = entry.due.saturating_add(every);
        if due <= self.now {
            due = due.saturating_add((self.now - due) / every * every + every);
        }
        if let Some(e) = self.slots[slot as usize].as_mut() {
            e.due = due;
        }
        self.file(slot, entry.start, due);
    }

    /// The timers at an hour the day passed, going from `before` to
    /// `hour`, added to `fired` as due now.
    fn passed_hours(&self, before: f64, hour: f64, fired: &mut Vec<(u64, Entry)>) {
        let passed = (hour - before).rem_euclid(24.0);
        if passed <= 0.0 {
            return;
        }
        for &slot in &self.hourly {
            let Some(entry) = self.slots[slot as usize] else {
                continue;
            };
            if let Kind::AtHour(at) = entry.kind {
                let to = (at - before).rem_euclid(24.0);
                if to > 0.0 && to <= passed {
                    fired.push((self.now, entry));
                }
            }
        }
    }

    /// Puts a timer in its slot (`slot`, its owner and name's own, or a
    /// new one), and files it.
    fn place(&mut self, entry: Entry, slot: Option<u32>) {
        let slot = slot.unwrap_or_else(|| {
            self.slots.push(None);
            let slot = (self.slots.len() - 1) as u32;
            self.by_key
                .insert((entry.entity, entry.component, entry.name), slot);
            slot
        });
        if self.slots[slot as usize].replace(entry).is_none() {
            self.live += 1;
        }
        match entry.kind {
            Kind::AtHour(_) => self.hourly.push(slot),
            Kind::Once | Kind::Every(_) => self.file(slot, entry.start, entry.due),
        }
    }

    /// Files `slot`'s timer under `due`; when old entries outnumber the
    /// timers, the buckets are made again from the slots.
    fn file(&mut self, slot: u32, start: u64, due: u64) {
        let spare = &mut self.spare;
        self.buckets
            .entry(due)
            .or_insert_with(|| spare.pop().unwrap_or_default())
            .push((slot, start));
        self.filed += 1;
        if self.filed > 2 * self.live + 4_096 {
            self.refile();
        }
    }

    /// The buckets made again from the slots, old entries left out.
    fn refile(&mut self) {
        self.buckets.clear();
        self.filed = 0;
        for (slot, entry) in self.slots.iter().enumerate() {
            if let Some(e) = entry
                && !matches!(e.kind, Kind::AtHour(_))
            {
                self.buckets
                    .entry(e.due)
                    .or_default()
                    .push((slot as u32, e.start));
                self.filed += 1;
            }
        }
    }

    /// Takes a timer out, if it waits (its bucket's entry is left, old,
    /// and its slot kept for its owner and name); when the slots of ended
    /// timers outnumber the rest, they're let go.
    fn remove(&mut self, key: Key) {
        if let Some(&slot) = self.by_key.get(&key) {
            self.remove_slot(slot);
        }
    }

    /// Takes the timer in `slot` out, if there is one.
    fn remove_slot(&mut self, slot: u32) {
        let Some(was) = self.slots[slot as usize].take() else {
            return;
        };
        self.live -= 1;
        if matches!(was.kind, Kind::AtHour(_)) {
            self.hourly.retain(|s| *s != slot);
        }
    }

    /// Lets the slots of ended timers go when they outnumber the rest:
    /// only between changes, never while slots are being read by number.
    fn tidy(&mut self) {
        if self.slots.len() > 2 * self.live + 4_096 {
            self.let_go();
        }
    }

    /// The slots made again from the timers waiting, those of ended ones
    /// let go, and everything filed anew.
    fn let_go(&mut self) {
        let entries: Vec<Entry> = self.slots.iter().flatten().copied().collect();
        self.slots.clear();
        self.by_key.clear();
        self.hourly.clear();
        self.live = 0;
        self.buckets.clear();
        self.filed = 0;
        for entry in entries {
            self.place(entry, None);
        }
    }

    /// With nothing waiting, the clock and the order of starts begin
    /// again from 0, and the slots and buckets are let go: a game, and the
    /// same game saved and loaded while nothing waited, then count alike.
    fn settle(&mut self) {
        if self.live == 0 {
            self.now = 0;
            self.next_start = 0;
            self.slots.clear();
            self.by_key.clear();
            self.buckets.clear();
            self.filed = 0;
            self.hourly.clear();
        }
    }
}
