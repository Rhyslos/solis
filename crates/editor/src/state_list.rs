//! What a recorded game should come out as, tick by tick (stage 1, the
//! same game on two PCs): each tick's checksums, and its state part by
//! part ([`world::StateParts`]), written as what changed since the tick
//! before. A replay on another PC is held to it: the first tick whose
//! checksums differ is found, and the parts that differ there are named.
//!
//! The file is text:
//!
//! ```text
//! solis states 1
//! -- comments
//! tick 0 w 9c1d… g 77ab…
//! + 5A329QT768NJB "Crate" Transform (in the world)<tab>1 0 0 2 | …
//! tick 1 w 1e2f… g 77ab…
//! - 5A329QT768NJB "Drone" 3D physics body
//! ```
//!
//! Tick 0 lists every part; each tick after it lists the parts that
//! changed (`+`, its name, a tab and what it holds now) and those gone
//! (`-` and its name). Ticks are numbered from 0, one after another.

use std::collections::BTreeMap;
use world::StateParts;

/// The first line, before the version.
const HEADER: &str = "solis states";
/// The format version this build reads and writes.
const VERSION: u32 = 1;
/// The biggest file read.
pub const MAX_STATE_LIST_BYTES: usize = 16 << 20;
/// The most ticks read: an hour at 60 a second, as recordings.
const MAX_TICKS: usize = 60 * 60 * 60;

/// One tick: its checksums, and what changed in its state.
#[derive(Debug, Clone, PartialEq, Default)]
struct Tick {
    /// The world's checksum at the end of the tick.
    world: u64,
    /// Game code's checksum at the end of the tick.
    game: u64,
    /// Each part that changed: what it holds now, or `None` when it's
    /// gone.
    changes: Vec<(String, Option<String>)>,
}

/// A recorded game's state, tick by tick.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StateList {
    ticks: Vec<Tick>,
    /// The state at the last tick pushed, for the next one's changes.
    last: StateParts,
}

/// A part whose two states differ.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartDifference {
    /// The part's name.
    pub part: String,
    /// What it should hold (`None`: it shouldn't be there).
    pub expected: Option<String>,
    /// What it holds (`None`: it isn't there).
    pub actual: Option<String>,
}

/// Why a list couldn't be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateListError {
    /// Bigger than 16 MB, or longer than an hour of ticks.
    TooBig,
    /// Not a list of states, or one from a newer Solis.
    NotAStateList,
    /// Damaged at this line (from 1).
    Damaged(usize),
}

impl std::fmt::Display for StateListError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooBig => write!(f, "it's longer than Solis reads (an hour, or 16 MB)"),
            Self::NotAStateList => write!(f, "it isn't a list of game states this Solis reads"),
            Self::Damaged(line) => write!(f, "it's damaged at line {line}"),
        }
    }
}

impl std::error::Error for StateListError {}

impl StateList {
    /// Adds the next tick: its checksums and its state.
    pub fn push(&mut self, world: u64, game: u64, state: StateParts) {
        // Kept as the file keeps them, so a list read back is the same.
        let state: StateParts = state
            .into_iter()
            .map(|(part, now)| (one_line(&part), one_line(&now)))
            .collect();
        let mut changes: Vec<(String, Option<String>)> = state
            .iter()
            .filter(|(part, now)| self.last.get(*part) != Some(*now))
            .map(|(part, now)| (part.clone(), Some(now.clone())))
            .collect();
        changes.extend(
            self.last
                .keys()
                .filter(|part| !state.contains_key(*part))
                .map(|part| (part.clone(), None)),
        );
        changes.sort();
        self.ticks.push(Tick {
            world,
            game,
            changes,
        });
        self.last = state;
    }

    /// How many ticks it holds.
    pub fn len(&self) -> usize {
        self.ticks.len()
    }

    /// Whether it holds no ticks.
    pub fn is_empty(&self) -> bool {
        self.ticks.is_empty()
    }

    /// Tick `tick`'s checksums: the world's and game code's.
    pub fn checksums(&self, tick: usize) -> Option<(u64, u64)> {
        self.ticks.get(tick).map(|t| (t.world, t.game))
    }

    /// The state at the end of tick `tick`, whole.
    pub fn state_at(&self, tick: usize) -> Option<StateParts> {
        let upto = self.ticks.get(..=tick)?;
        let mut state = StateParts::new();
        for t in upto {
            for (part, now) in &t.changes {
                match now {
                    Some(value) => state.insert(part.clone(), value.clone()),
                    None => state.remove(part),
                };
            }
        }
        Some(state)
    }

    /// The file's text, with `comment` (any lines) at its top.
    pub fn write(&self, comment: &str) -> String {
        let mut text = format!("{HEADER} {VERSION}\n");
        for line in comment.lines() {
            text.push_str("-- ");
            text.push_str(line);
            text.push('\n');
        }
        for (n, tick) in self.ticks.iter().enumerate() {
            text.push_str(&format!(
                "tick {n} w {:016x} g {:016x}\n",
                tick.world, tick.game
            ));
            for (part, now) in &tick.changes {
                let part = one_line(part);
                match now {
                    Some(value) => text.push_str(&format!("+ {part}\t{}\n", one_line(value))),
                    None => text.push_str(&format!("- {part}\n")),
                }
            }
        }
        text
    }

    /// Reads a list from its file's text.
    pub fn read(text: &str) -> Result<Self, StateListError> {
        if text.len() > MAX_STATE_LIST_BYTES {
            return Err(StateListError::TooBig);
        }
        let mut lines = text.lines().enumerate();
        let version = lines
            .next()
            .and_then(|(_, line)| line.trim_end().strip_prefix(HEADER))
            .and_then(|rest| rest.trim().parse::<u32>().ok());
        if version != Some(VERSION) {
            return Err(StateListError::NotAStateList);
        }
        let mut list = Self::default();
        for (at, line) in lines {
            let damaged = || StateListError::Damaged(at + 1);
            if line.is_empty() || line.starts_with("--") {
                continue;
            }
            if let Some(rest) = line.strip_prefix("tick ") {
                if list.ticks.len() >= MAX_TICKS {
                    return Err(StateListError::TooBig);
                }
                let tick = read_tick(rest, list.ticks.len()).ok_or_else(damaged)?;
                list.ticks.push(tick);
                continue;
            }
            let tick = list.ticks.last_mut().ok_or_else(damaged)?;
            let change = if let Some(rest) = line.strip_prefix("+ ") {
                let (part, value) = rest.split_once('\t').ok_or_else(damaged)?;
                (part.to_string(), Some(value.to_string()))
            } else if let Some(part) = line.strip_prefix("- ") {
                (part.to_string(), None)
            } else {
                return Err(damaged());
            };
            if change.0.is_empty() || change.0.contains('\t') {
                return Err(damaged());
            }
            tick.changes.push(change);
        }
        if let Some(last) = list.ticks.len().checked_sub(1) {
            list.last = list.state_at(last).unwrap_or_default();
        }
        Ok(list)
    }
}

/// A tick's line after `tick `: its number (which must be `expected`)
/// and its two checksums.
fn read_tick(rest: &str, expected: usize) -> Option<Tick> {
    let words: Vec<&str> = rest.split(' ').collect();
    let [n, "w", world, "g", game] = words.as_slice() else {
        return None;
    };
    let hex = |word: &str| {
        (word.len() == 16)
            .then(|| u64::from_str_radix(word, 16).ok())
            .flatten()
    };
    (n.parse::<usize>().ok()? == expected).then_some(Tick {
        world: hex(world)?,
        game: hex(game)?,
        changes: Vec::new(),
    })
}

/// Text kept to one line with no tab, so it can't break the file's lines.
fn one_line(text: &str) -> String {
    text.replace(['\n', '\r', '\t'], " ")
}

/// The parts whose two states differ, by name.
pub fn state_differences(expected: &StateParts, actual: &StateParts) -> Vec<PartDifference> {
    let mut names: BTreeMap<&String, ()> = BTreeMap::new();
    names.extend(expected.keys().map(|k| (k, ())));
    names.extend(actual.keys().map(|k| (k, ())));
    names
        .into_keys()
        .filter(|name| expected.get(*name) != actual.get(*name))
        .map(|name| PartDifference {
            part: name.clone(),
            expected: expected.get(name).cloned(),
            actual: actual.get(name).cloned(),
        })
        .collect()
}

#[cfg(test)]
#[path = "state_list_tests.rs"]
mod tests;
