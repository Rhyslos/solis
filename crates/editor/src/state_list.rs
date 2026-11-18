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
//! + 0 5A329QT768NJB "Crate" Transform (in the world)<tab>1 0 0 2 | …
//! + 1 game seed<tab>12345
//! tick 1 w 1e2f… g 77ab…
//! = 0<tab>#3f0c9a7e
//! - 1
//! ```
//!
//! Tick 0 lists every part; each tick after it lists the parts that
//! changed and those gone. A part is numbered the first time it comes
//! (`+`, its number, its name, a tab and what it holds); after that it's
//! named by its number: `=` with what it holds now, `-` when it's gone.
//! Ticks are numbered from 0, one after another.
//!
//! What a part holds is kept as written when it's short, and as a
//! checksum of it (`#` and 8 hex digits) when it's longer than
//! [`KEPT_LENGTH`]: a body's place and motion change every tick, and the
//! list stays small. A part that differs is still named; what it holds
//! on the PC that made the list is shown in full when the check runs
//! there.

use std::collections::{BTreeMap, HashMap};
use world::StateParts;

/// The first line, before the version.
const HEADER: &str = "solis states";
/// The format version this build reads and writes.
const VERSION: u32 = 1;
/// The biggest file read.
pub const MAX_STATE_LIST_BYTES: usize = 16 << 20;
/// The most ticks read: an hour at 60 a second, as recordings.
const MAX_TICKS: usize = 60 * 60 * 60;
/// The longest a part's value is kept as written; longer, it's kept as
/// its checksum.
pub const KEPT_LENGTH: usize = 32;

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
        let state = kept(&state);
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

    /// The parts in which `actual` (a state as the game has it, in full)
    /// differs from the state at the end of tick `tick`, compared as the
    /// list keeps them; `None` past the list's end.
    pub fn differences(&self, tick: usize, actual: &StateParts) -> Option<Vec<PartDifference>> {
        let expected = self.state_at(tick)?;
        let kept_actual = kept(actual);
        let mut names: BTreeMap<&String, ()> = BTreeMap::new();
        names.extend(expected.keys().map(|k| (k, ())));
        names.extend(kept_actual.keys().map(|k| (k, ())));
        Some(
            names
                .into_keys()
                .filter(|name| expected.get(*name) != kept_actual.get(*name))
                .map(|name| PartDifference {
                    part: name.clone(),
                    expected: expected.get(name).cloned(),
                    actual: actual
                        .iter()
                        .find(|(k, _)| one_line(k) == **name)
                        .map(|(_, v)| v.clone()),
                })
                .collect(),
        )
    }

    /// The state at the end of tick `tick`, whole, as the list keeps it.
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
        let mut numbers: HashMap<&str, usize> = HashMap::new();
        for (n, tick) in self.ticks.iter().enumerate() {
            text.push_str(&format!(
                "tick {n} w {:016x} g {:016x}\n",
                tick.world, tick.game
            ));
            for (part, now) in &tick.changes {
                let line = match (numbers.get(part.as_str()), now) {
                    (Some(n), Some(value)) => format!("= {n}\t{value}\n"),
                    (Some(n), None) => format!("- {n}\n"),
                    (None, Some(value)) => {
                        let n = numbers.len();
                        numbers.insert(part, n);
                        format!("+ {n} {part}\t{value}\n")
                    }
                    // Gone before it ever came: nothing to say.
                    (None, None) => continue,
                };
                text.push_str(&line);
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
        let mut names: Vec<String> = Vec::new();
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
            let change = read_change(line, &mut names).ok_or_else(damaged)?;
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

/// A part's line (`+`, `=` or `-`), with `names` the parts numbered so
/// far (a new one joins them).
fn read_change(line: &str, names: &mut Vec<String>) -> Option<(String, Option<String>)> {
    let number = |word: &str| word.parse::<usize>().ok();
    if let Some(rest) = line.strip_prefix("+ ") {
        let (head, value) = rest.split_once('\t')?;
        let (n, part) = head.split_once(' ')?;
        if number(n)? != names.len() || part.is_empty() {
            return None;
        }
        names.push(part.to_string());
        Some((part.to_string(), Some(value.to_string())))
    } else if let Some(rest) = line.strip_prefix("= ") {
        let (n, value) = rest.split_once('\t')?;
        Some((names.get(number(n)?)?.clone(), Some(value.to_string())))
    } else {
        let n = line.strip_prefix("- ")?;
        Some((names.get(number(n)?)?.clone(), None))
    }
}

/// `state` as a list keeps it: each name and value on one line, and a
/// value longer than [`KEPT_LENGTH`] as its checksum.
pub fn kept(state: &StateParts) -> StateParts {
    state
        .iter()
        .map(|(part, now)| {
            let now = one_line(now);
            let now = if now.len() > KEPT_LENGTH {
                let mut sum = reflect::Checksum::new();
                sum.bytes(now.as_bytes());
                // Eight digits are plenty to tell one value from another
                // (the tick's two checksums hold the game to every bit).
                format!("#{:08x}", sum.finish() as u32)
            } else {
                now
            };
            (one_line(part), now)
        })
        .collect()
}

/// Text kept to one line with no tab, so it can't break the file's lines.
fn one_line(text: &str) -> String {
    text.replace(['\n', '\r', '\t'], " ")
}

#[cfg(test)]
#[path = "state_list_tests.rs"]
mod tests;
