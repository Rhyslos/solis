//! A play's input written by hand (Fixes 4, T1 and T3): a plain-text
//! script of what the player does, by action and tick, that `solis
//! --play` plays a level with, and that a check recording can be made
//! from. The same actions game code names, so a script reads like the
//! game: `at 60 press Jump`.
//!
//! ```text
//! -- walk right for a second, jumping half way
//! at 0 hold Move 1 0
//! at 30 press Jump
//! at 60 hold Move 0 0
//! at 90 hold Sprint
//! at 120 release Sprint
//! ```
//!
//! Ticks count from 0, the first tick played. `hold` keeps a button down,
//! or an axis at two numbers (across and up), until it changes; `press`
//! is a button down for one tick; `release` lets a button go. Lines may
//! come in any order, and several may share a tick: they're taken in
//! order of tick, then as written. `--` starts a comment.

use crate::{ActionState, Frame};
use glam::Vec2;
use std::collections::BTreeMap;

/// The most lines a script holds.
pub(crate) const MAX_LINES: usize = 100_000;
/// The latest tick a script names: an hour at 60 a second.
pub(crate) const MAX_TICK: u32 = 60 * 60 * 60;
/// The longest script read, in bytes.
pub(crate) const MAX_BYTES: usize = 8 << 20;

/// What a line does to one action.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Step {
    /// A button down from now.
    Hold,
    /// A button down for this tick only.
    Press,
    /// A button let go.
    Release,
    /// An axis at these numbers from now (zero lets it rest).
    Axis(Vec2),
}

/// A script, read: what changes on each tick, in order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Script {
    /// Each tick's changes, by tick.
    steps: BTreeMap<u32, Vec<(String, Step)>>,
    /// Buttons down, and axes away from rest, as of the last frame made.
    down: BTreeMap<String, Step>,
    /// Buttons pressed for one tick, let go on the next.
    pressed_once: Vec<String>,
    /// The next tick [`Script::next_frame`] makes.
    next: u32,
}

impl Script {
    /// The script in `text`, or the first mistake with its line (from 1).
    pub fn read(text: &str) -> Result<Self, String> {
        if text.len() > MAX_BYTES {
            return Err(format!("the script is bigger than {} MB", MAX_BYTES >> 20));
        }
        let mut steps: BTreeMap<u32, Vec<(String, Step)>> = BTreeMap::new();
        let mut count = 0;
        for (n, line) in text.lines().enumerate() {
            let line = line.split("--").next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            count += 1;
            if count > MAX_LINES {
                return Err(format!("the script has more than {MAX_LINES} lines"));
            }
            let (tick, name, step) =
                read_line(line).map_err(|why| format!("line {}: {why}", n + 1))?;
            steps.entry(tick).or_default().push((name, step));
        }
        Ok(Self {
            steps,
            ..Self::default()
        })
    }

    /// Every action the script names, for checking against a project's.
    pub fn actions(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self
            .steps
            .values()
            .flatten()
            .map(|(name, _)| name.as_str())
            .collect();
        names.sort_unstable();
        names.dedup();
        names
    }

    /// The last tick the script names (0 when empty).
    pub fn last_tick(&self) -> u32 {
        self.steps.keys().next_back().copied().unwrap_or(0)
    }

    /// The input of the next tick, from the first: the actions held, those
    /// pressed and released this tick, and the axes.
    pub fn next_frame(&mut self) -> Frame {
        let tick = self.next;
        self.next = self.next.saturating_add(1);
        let mut frame = Frame::default();
        // Last tick's one-tick presses come up now.
        for name in std::mem::take(&mut self.pressed_once) {
            if !matches!(self.down.get(&name), Some(Step::Hold)) {
                frame.actions.entry(name).or_default().released = true;
            }
        }
        for (name, step) in self.steps.get(&tick).cloned().unwrap_or_default() {
            let was_down = matches!(self.down.get(&name), Some(Step::Hold));
            let state = frame.actions.entry(name.clone()).or_default();
            match step {
                Step::Hold => {
                    state.pressed |= !was_down;
                    self.down.insert(name, Step::Hold);
                }
                Step::Press => {
                    state.pressed |= !was_down;
                    self.pressed_once.push(name.clone());
                    state.held = true;
                }
                Step::Release => {
                    state.released |= was_down;
                    self.down.remove(&name);
                }
                Step::Axis(v) if v == Vec2::ZERO => {
                    self.down.remove(&name);
                }
                Step::Axis(v) => {
                    self.down.insert(name, Step::Axis(v));
                }
            }
        }
        for (name, step) in &self.down {
            let state: &mut ActionState = frame.actions.entry(name.clone()).or_default();
            match step {
                Step::Hold => state.held = true,
                Step::Axis(v) => {
                    state.axis = *v;
                    state.held = true;
                }
                _ => {}
            }
        }
        frame.actions.retain(|_, s| !s.is_idle());
        frame
    }
}

/// One line: `at <tick> <hold|press|release> <Action> [x y]`.
fn read_line(line: &str) -> Result<(u32, String, Step), String> {
    let words: Vec<&str> = line.split_whitespace().collect();
    let shape = "write it as at <tick> hold|press|release <Action>, or at <tick> hold <Action> \
                 <across> <up> for an axis: at 60 press Jump";
    let [at, tick, verb, name, rest @ ..] = words.as_slice() else {
        return Err(shape.to_string());
    };
    if *at != "at" {
        return Err(shape.to_string());
    }
    let tick: u32 = tick
        .parse()
        .ok()
        .filter(|t| *t <= MAX_TICK)
        .ok_or_else(|| format!("{tick} isn't a tick from 0 to {MAX_TICK}"))?;
    if !crate::valid_action_name(name) {
        return Err(format!(
            "{name} isn't an action's name: letters, digits and _, starting with a letter"
        ));
    }
    let number = |w: &str| {
        w.parse::<f32>()
            .ok()
            .filter(|v| v.is_finite() && (-1.0..=1.0).contains(v))
            .ok_or_else(|| format!("{w} isn't a number from -1 to 1"))
    };
    let step = match (*verb, rest) {
        ("hold", []) => Step::Hold,
        ("press", []) => Step::Press,
        ("release", []) => Step::Release,
        ("hold", [x, y]) => Step::Axis(Vec2::new(number(x)?, number(y)?)),
        ("press" | "release", [_, ..]) => {
            return Err(format!(
                "{verb} is for buttons; hold an axis at its numbers, and at 0 0 to let it rest"
            ));
        }
        _ => return Err(shape.to_string()),
    };
    Ok((tick, (*name).to_string(), step))
}

#[cfg(test)]
#[path = "script_tests.rs"]
mod tests;
