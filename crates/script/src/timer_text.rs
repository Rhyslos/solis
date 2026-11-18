//! The timers as a save game keeps them (save version 11): a line saying
//! where the timers' clock is, then a line a timer, in the order they
//! fire. Nothing waiting is no text at all.
//!
//! ```text
//! at NOW HOUR NEXT
//! once ENTITY COMPONENT NAME START DUE
//! every ENTITY COMPONENT NAME START DUE INTERVAL
//! hour ENTITY COMPONENT NAME START HOUR
//! ```
//!
//! `NOW`, `DUE` and `INTERVAL` are game time in units
//! ([`super::UNITS_PER_TICK`] a tick), `HOUR` an hour's exact bits (`-`
//! before the first tick), `START` and `NEXT` places in the order of
//! starts. A save is a file a player can be handed, so every line is
//! checked before anything is kept.

use super::{Key, Kind, MAX_TIMERS, Slot, Timers, UNITS_PER_TICK, valid_name, within_day};
use reflect::EntityId;
use std::sync::Arc;

/// The longest component name read (Lust's and Lua's names are far
/// shorter).
const MAX_COMPONENT: usize = 200;

/// The latest a timer may be due, in units: the longest timer from a
/// clock that has run a long time.
const MAX_UNITS: u64 = 1 << 62;

impl Timers {
    /// The timers as a save keeps them; empty while none waits.
    pub fn write(&self) -> String {
        if self.slots.is_empty() {
            return String::new();
        }
        let hour = self
            .hour
            .map_or_else(|| "-".to_string(), |h| format!("{:x}", h.to_bits()));
        let mut text = format!("at {} {hour} {}\n", self.now, self.next_start);
        for (due, entity, component, start, name) in &self.queue {
            let key: Key = (*entity, Arc::clone(component), Arc::clone(name));
            match self.slots.get(&key).map(|s| s.kind) {
                Some(Kind::Every(every)) => text.push_str(&format!(
                    "every {entity} {component} {name} {start} {due} {every}\n"
                )),
                _ => text.push_str(&format!("once {entity} {component} {name} {start} {due}\n")),
            }
        }
        for ((entity, component, start), (name, hour)) in &self.hourly {
            text.push_str(&format!(
                "hour {entity} {component} {name} {start} {:x}\n",
                hour.to_bits()
            ));
        }
        text
    }

    /// The timers a save kept, or why they can't be read: a damaged or
    /// hand-made section is refused whole, never half kept.
    pub fn read(text: &str) -> Result<Self, String> {
        let damaged = || "its timers are damaged".to_string();
        let mut timers = Self::default();
        let mut lines = text.lines();
        let Some(first) = lines.next() else {
            return Ok(timers);
        };
        let mut words = first.split(' ').take(5);
        if words.next() != Some("at") {
            return Err(damaged());
        }
        timers.now = number(words.next())?;
        timers.hour = match words.next() {
            Some("-") => None,
            word => Some(hour_of(word)?),
        };
        timers.next_start = number(words.next())?;
        if words.next().is_some() || timers.now > MAX_UNITS {
            return Err(damaged());
        }
        for line in lines {
            if timers.slots.len() >= MAX_TIMERS {
                return Err(damaged());
            }
            let (key, slot) = timer_of(line, &timers)?;
            if timers.slots.contains_key(&key) {
                return Err(damaged());
            }
            timers.place(key, slot);
        }
        if timers.slots.is_empty() {
            return Err(damaged());
        }
        Ok(timers)
    }
}

/// One timer's line read, checked against the clock and the order.
fn timer_of(line: &str, timers: &Timers) -> Result<(Key, Slot), String> {
    let damaged = || "its timers are damaged".to_string();
    let mut words = line.split(' ').take(8);
    let what = words.next();
    let entity = words
        .next()
        .and_then(|w| w.parse::<reflect::Id>().ok())
        .map(EntityId)
        .ok_or_else(damaged)?;
    let component = words
        .next()
        .filter(|c| {
            !c.is_empty()
                && c.len() <= MAX_COMPONENT
                && c.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        })
        .ok_or_else(damaged)?;
    let name = words.next().filter(|n| valid_name(n)).ok_or_else(damaged)?;
    let start = number(words.next())?;
    if start >= timers.next_start {
        return Err(damaged());
    }
    let kind_and_due = match what {
        Some("once") => (Kind::Once, number(words.next())?),
        Some("every") => {
            let due = number(words.next())?;
            let every = number(words.next())?;
            if every == 0 || every % UNITS_PER_TICK != 0 {
                return Err(damaged());
            }
            (Kind::Every(every), due)
        }
        Some("hour") => (Kind::AtHour(hour_of(words.next())?), 0),
        _ => return Err(damaged()),
    };
    let (kind, due) = kind_and_due;
    if words.next().is_some() || due > MAX_UNITS {
        return Err(damaged());
    }
    Ok((
        (entity, Arc::from(component), Arc::from(name)),
        Slot { start, kind, due },
    ))
}

/// A whole number from a save's word.
fn number(word: Option<&str>) -> Result<u64, String> {
    word.and_then(|w| w.parse::<u64>().ok())
        .ok_or_else(|| "its timers are damaged".to_string())
}

/// An hour from its exact bits: within the day, or the save is damaged.
fn hour_of(word: Option<&str>) -> Result<f64, String> {
    let hour = word
        .and_then(|w| u64::from_str_radix(w, 16).ok())
        .map(f64::from_bits)
        .ok_or_else(|| "its timers are damaged".to_string())?;
    if hour.is_finite() && within_day(hour) == hour {
        Ok(hour)
    } else {
        Err("its timers are damaged".to_string())
    }
}
