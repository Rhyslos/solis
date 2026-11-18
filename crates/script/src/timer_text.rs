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
//! From save version 12 (stage 1, states), a timer started in an option
//! of its component's own state ends its line with `in OPTION`
//! (`in Combat.Melee`): it ends when the component leaves that option.
//!
//! `NOW`, `DUE` and `INTERVAL` are game time in units
//! ([`super::UNITS_PER_TICK`] a tick), `HOUR` an hour's exact bits (`-`
//! before the first tick), `START` and `NEXT` places in the order of
//! starts. A save is a file a player can be handed, so every line is
//! checked before anything is kept.

use super::store::Kind;
use super::{MAX_TIMERS, Timers, UNITS_PER_TICK, valid_name, within_day};
use reflect::EntityId;

/// The longest component name read (Lust's and Lua's names are far
/// shorter).
const MAX_COMPONENT: usize = 200;

/// The latest a timer may be due, in units: the longest timer from a
/// clock that has run a long time.
const MAX_UNITS: u64 = 1 << 62;

/// The longest option a timer may belong to (`Combat.Melee`).
const MAX_OPTION: usize = 200;

/// A timer's line as read: whose and which, how it waits, and the option
/// it belongs to.
struct Line<'a> {
    entity: EntityId,
    component: &'a str,
    name: &'a str,
    start: u64,
    kind: Kind,
    due: u64,
    during: Option<&'a str>,
}

impl Timers {
    /// The timers as a save keeps them; empty while none waits.
    pub fn write(&self) -> String {
        if self.is_empty() {
            return String::new();
        }
        let hour = self
            .hour
            .map_or_else(|| "-".to_string(), |h| format!("{:x}", h.to_bits()));
        let mut text = format!("at {} {hour} {}\n", self.now, self.next_start);
        for (entity, component, name, entry) in self.entries() {
            let (start, due) = (entry.start, entry.due);
            text.push_str(&match entry.kind {
                Kind::Once => format!("once {entity} {component} {name} {start} {due}"),
                Kind::Every(every) => {
                    format!("every {entity} {component} {name} {start} {due} {every}")
                }
                Kind::AtHour(at) => {
                    format!(
                        "hour {entity} {component} {name} {start} {:x}",
                        at.to_bits()
                    )
                }
            });
            if let Some(during) = entry.during {
                text.push_str(" in ");
                text.push_str(self.option(during));
            }
            text.push('\n');
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
        let now = number(words.next())?;
        let hour = match words.next() {
            Some("-") => None,
            word => Some(hour_of(word)?),
        };
        let next_start = number(words.next())?;
        if words.next().is_some() || now > MAX_UNITS {
            return Err(damaged());
        }
        for line in lines {
            if timers.len() >= MAX_TIMERS {
                return Err(damaged());
            }
            let Line {
                entity,
                component,
                name,
                start,
                kind,
                due,
                during,
            } = timer_of(line, next_start)?;
            if !timers.restore((entity, component, name), (start, kind, due), during) {
                return Err(damaged());
            }
        }
        if timers.is_empty() {
            return Err(damaged());
        }
        timers.now = now;
        timers.hour = hour;
        timers.next_start = next_start;
        Ok(timers)
    }
}

/// One timer's line read, checked against the order of starts.
fn timer_of(line: &str, next_start: u64) -> Result<Line<'_>, String> {
    let damaged = || "its timers are damaged".to_string();
    let mut words = line.split(' ').take(10);
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
    if start >= next_start {
        return Err(damaged());
    }
    let (kind, due) = match what {
        Some("once") => (Kind::Once, number(words.next())?),
        Some("every") => {
            let due = number(words.next())?;
            let every = number(words.next())?;
            if every == 0 || every % UNITS_PER_TICK != 0 || every > MAX_UNITS {
                return Err(damaged());
            }
            (Kind::Every(every), due)
        }
        Some("hour") => (Kind::AtHour(hour_of(words.next())?), 0),
        _ => return Err(damaged()),
    };
    let during = match words.next() {
        None => None,
        Some("in") => Some(
            words
                .next()
                .filter(|o| valid_option(o))
                .ok_or_else(damaged)?,
        ),
        Some(_) => return Err(damaged()),
    };
    if words.next().is_some() || due > MAX_UNITS {
        return Err(damaged());
    }
    Ok(Line {
        entity,
        component,
        name,
        start,
        kind,
        due,
        during,
    })
}

/// Whether `option` can be a state's option, as Lust writes it inside
/// others (`Combat.Melee`): names joined by dots.
fn valid_option(option: &str) -> bool {
    option.len() <= MAX_OPTION
        && option.split('.').all(|part| {
            part.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
                && part.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        })
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
