//! Named timers (stage 1, timers; [decision 0125](../../../docs/decisions/0125-named-timers-now-blocks-later.md)):
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
use std::sync::Arc;

#[path = "timer_store.rs"]
mod store;
#[path = "timer_text.rs"]
mod text;

pub use store::Timers;

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
