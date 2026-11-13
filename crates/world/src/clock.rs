//! The game's clock (whole ticks, its speed, how far between ticks), the
//! time of day ([decision 0092](../../../docs/decisions/0092-day-and-night-and-lamps-on-sprites.md))
//! and the random seed.

use crate::{Clock, Global, MAX_SPEED, MAX_TICKS_PER_STEP, Previous, TICK, World};

impl World {
    /// Moves the clock on by `seconds` (ignored if not a sensible number),
    /// running as many whole ticks as fit, at most
    /// [`MAX_TICKS_PER_STEP`]. Returns how many ran. ([`Self::due`] and
    /// [`Self::begin_tick`] do the same a tick at a time.)
    pub fn step(&mut self, seconds: f64) -> u64 {
        let due = self.due(seconds);
        for _ in 0..due {
            self.begin_tick();
        }
        due
    }

    /// How many whole ticks `seconds` more (ignored if not a sensible
    /// number) make due, at most [`MAX_TICKS_PER_STEP`]; the rest waits
    /// for the next frame (or, behind by more than that, is dropped). The
    /// clock itself moves on as each tick begins ([`Self::begin_tick`]),
    /// so a frame that stops early (a pause, a load) never counts ticks
    /// that didn't run.
    pub fn due(&mut self, seconds: f64) -> u64 {
        if !seconds.is_finite() || seconds <= 0.0 {
            return 0;
        }
        self.clock.carry += seconds;
        let mut due = 0;
        while self.clock.carry >= TICK && due < MAX_TICKS_PER_STEP {
            self.clock.carry -= TICK;
            due += 1;
        }
        // Behind by more than the most ticks: the rest is dropped.
        if self.clock.carry >= TICK {
            self.clock.carry = 0.0;
        }
        due
    }

    /// A tick begins: the clock moves on by one, and game time by a tick
    /// at the game's speed.
    pub fn begin_tick(&mut self) {
        self.clock.ticks = self.clock.ticks.saturating_add(1);
        self.clock.time += self.tick_time();
        self.touched = true;
    }

    /// How fast the game runs: 1 as it starts, 0 paused.
    pub fn speed(&self) -> f64 {
        self.speed
    }

    /// Sets how fast the game runs (game code's `World:setGameSpeed`),
    /// from 0 (paused: time, physics, projectiles and sprites stop; game
    /// code still runs) to [`MAX_SPEED`]. Not a number changes nothing.
    pub fn set_speed(&mut self, speed: f64) {
        if speed.is_finite() {
            self.speed = speed.clamp(0.0, MAX_SPEED);
            self.touched = true;
        }
    }

    /// The game's hour (decision 0092): 0 up to 24, from the area's
    /// Environment's day (its start and length) and the game's time,
    /// moved on by game code's `setTimeOfDay`. With no Environment, the
    /// default day's.
    pub fn time_of_day(&self) -> f64 {
        let env = self.environment().unwrap_or_default();
        env.hour_at(self.clock.time, self.day_shift)
    }

    /// Sets the game's hour (game code's `World:setTimeOfDay`): the day
    /// carries on from there. Not a number changes nothing.
    pub fn set_time_of_day(&mut self, hour: f64) {
        if !hour.is_finite() {
            return;
        }
        let env = self.environment().unwrap_or_default();
        let now = env.hour_at(self.clock.time, 0.0);
        self.day_shift = within_day(hour.rem_euclid(24.0) - now);
        self.touched = true;
    }

    /// The hours game code moved the day on by, for saves: always 0 up to
    /// (not including) 24.
    pub fn day_shift(&self) -> f64 {
        self.day_shift
    }

    /// Puts back the hours a save moved the day on by.
    pub fn set_day_shift(&mut self, shift: f64) {
        if shift.is_finite() {
            self.day_shift = within_day(shift);
        }
    }

    /// A tick's seconds of game time at the game's speed.
    pub fn tick_time(&self) -> f64 {
        TICK * self.speed
    }

    /// The game's clock.
    pub fn clock(&self) -> Clock {
        self.clock
    }

    /// Remembers where every entity is, as the start of a tick: what
    /// drawing between ticks moves from. The editor calls it before each
    /// tick.
    pub fn remember(&mut self) {
        for (global, previous) in self.ecs.query_mut::<(&Global, &mut Previous)>() {
            previous.0 = Some(global.0);
        }
    }

    /// How far the clock is into the next tick, from 0 to 1: how far
    /// drawing should be between the last tick and the next.
    pub fn progress(&self) -> f32 {
        ((self.clock.carry / TICK) as f32).clamp(0.0, 1.0)
    }

    /// Sets the clock to `ticks` run, at game time `time` (a save game
    /// loaded, decision 0038), with nothing waiting for the next tick.
    pub fn set_clock(&mut self, ticks: u64, time: f64) {
        self.clock = Clock {
            ticks,
            time: if time.is_finite() { time } else { 0.0 },
            carry: 0.0,
        };
        self.touched = true;
    }

    /// The seed the game's random numbers are made from.
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Starts the game's random numbers again from `seed` (Play chooses
    /// one; game code's `setSeed` changes it).
    pub fn set_seed(&mut self, seed: u64) {
        self.seed = seed;
        self.touched = true;
    }
}

/// `hours` wrapped into 0 up to (not including) 24. `rem_euclid` can round
/// a tiny negative up to exactly 24.0, which saves would refuse (the phase
/// review).
fn within_day(hours: f64) -> f64 {
    let h = hours.rem_euclid(24.0);
    if h >= 24.0 { 0.0 } else { h }
}
