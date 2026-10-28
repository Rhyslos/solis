//! An animator at work (Phase 9, D3b): an [`AnimatorFile`] made ready to
//! run ([`Animator`]), and one entity's place in it ([`Run`]): the state
//! it's in and its parameters' values.
//!
//! The running game steps every animator once a tick, after game code has
//! set its parameters and before sprites move on: the first transition
//! (from any state first, then from its own, each in the file's order)
//! whose conditions all hold is taken, at most one a tick, and triggers
//! are then put out. What plays is the state's clip, or its clip for the
//! direction nearest the facing parameter: turning changes the clip but
//! keeps the time, so a walk stays in step. All plain comparisons on the
//! values game code set, so every PC and every replay agree.

use crate::animator_file::{AnimatorFile, Condition, Facing, ParameterKind, Plays};

/// A parameter's value while running.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParamValue {
    /// A number.
    Number(f32),
    /// On or off.
    Switch(bool),
    /// Whether it was fired this tick.
    Trigger(bool),
    /// A direction on the ground, length 1 (x right, y up).
    Direction([f32; 2]),
}

impl ParamValue {
    /// The value a parameter starts with.
    pub fn start(kind: ParameterKind) -> Self {
        match kind {
            ParameterKind::Number(v) => Self::Number(v),
            ParameterKind::Switch(b) => Self::Switch(b),
            ParameterKind::Trigger => Self::Trigger(false),
            ParameterKind::Direction(f) => Self::Direction(f.vector()),
        }
    }

    /// Whether `self` and `other` are the same kind of value.
    pub fn same_kind(self, other: Self) -> bool {
        std::mem::discriminant(&self) == std::mem::discriminant(&other)
    }
}

/// One entity's place in its animator: the state it's in and its
/// parameters' values (in the animator's order). Game state: in the
/// checksum and in saves.
#[derive(Debug, Clone, PartialEq)]
pub struct Run {
    /// The state it's in (an index into the animator's states).
    pub state: usize,
    /// Each parameter's value.
    pub values: Vec<ParamValue>,
}

/// Why game code's setting wasn't taken.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetError {
    /// The animator has no parameter by that name.
    NoParameter,
    /// The parameter is another kind (its kind's name).
    WrongKind(&'static str),
    /// The value isn't a usable number or direction.
    NotUsable,
}

/// What a state plays, ready to run.
#[derive(Debug, Clone, PartialEq)]
enum Played {
    Clip(String),
    ByDirection {
        param: Option<usize>,
        clips: Vec<([f32; 2], String)>,
    },
}

#[derive(Debug, Clone, PartialEq)]
struct ReadyState {
    name: String,
    plays: Played,
    speed: f32,
    speed_by: Option<usize>,
}

/// A condition, ready to run.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Test {
    On(usize),
    Off(usize),
    Fired(usize),
    Above(usize, f32),
    Below(usize, f32),
    Finished,
    /// A condition on a parameter that isn't there, or of another kind:
    /// never holds.
    Never,
}

#[derive(Debug, Clone, PartialEq)]
struct Way {
    to: usize,
    when: Vec<Test>,
}

/// An animator file made ready to run, shared by every entity using it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Animator {
    parameters: Vec<(String, ParameterKind)>,
    states: Vec<ReadyState>,
    /// Transitions from any state.
    any: Vec<Way>,
    /// Transitions from each state.
    out: Vec<Vec<Way>>,
    entry: usize,
    /// What in the file can't work, in plain words.
    pub notes: Vec<String>,
}

impl Animator {
    /// Makes `file` ready to run. Conditions on parameters that aren't
    /// there (or are another kind) never hold, with a note.
    pub fn new(file: &AnimatorFile) -> Self {
        let mut notes = Vec::new();
        let parameters: Vec<(String, ParameterKind)> = file
            .parameters
            .iter()
            .map(|p| (p.name.clone(), p.kind))
            .collect();
        let index = |name: &str| parameters.iter().position(|(n, _)| n == name);
        let kind = |i: usize| parameters[i].1;
        let states: Vec<ReadyState> = file
            .states
            .iter()
            .map(|s| ReadyState {
                name: s.name.clone(),
                plays: match &s.plays {
                    Plays::Clip(c) => Played::Clip(c.clone()),
                    Plays::ByDirection { by, clips } => Played::ByDirection {
                        param: index(by)
                            .filter(|&i| matches!(kind(i), ParameterKind::Direction(_))),
                        clips: clips.iter().map(|(f, c)| (f.vector(), c.clone())).collect(),
                    },
                },
                speed: s.speed,
                speed_by: index(&s.speed_by)
                    .filter(|&i| matches!(kind(i), ParameterKind::Number(_))),
            })
            .collect();
        let state_index = |id| file.states.iter().position(|s| s.id == id);
        let mut any = Vec::new();
        let mut out = vec![Vec::new(); states.len()];
        for t in &file.transitions {
            let Some(to) = state_index(t.to) else {
                continue;
            };
            let when = t
                .when
                .iter()
                .map(|c| {
                    let test = match c {
                        Condition::Finished => Test::Finished,
                        Condition::Is(n) => match index(n).map(|i| (i, kind(i))) {
                            Some((i, ParameterKind::Switch(_))) => Test::On(i),
                            Some((i, ParameterKind::Trigger)) => Test::Fired(i),
                            _ => Test::Never,
                        },
                        Condition::Not(n) => match index(n).map(|i| (i, kind(i))) {
                            Some((i, ParameterKind::Switch(_))) => Test::Off(i),
                            _ => Test::Never,
                        },
                        Condition::Above(n, v) | Condition::Below(n, v) => {
                            match index(n).map(|i| (i, kind(i))) {
                                Some((i, ParameterKind::Number(_))) => {
                                    if matches!(c, Condition::Above(..)) {
                                        Test::Above(i, *v)
                                    } else {
                                        Test::Below(i, *v)
                                    }
                                }
                                _ => Test::Never,
                            }
                        }
                    };
                    if test == Test::Never {
                        notes.push(format!(
                            "the condition \"{}\" names no parameter it can test, so it never \
                             holds",
                            c.text()
                        ));
                    }
                    test
                })
                .collect();
            let way = Way { to, when };
            match t.from.map(state_index) {
                None => any.push(way),
                Some(Some(from)) => out[from].push(way),
                Some(None) => {}
            }
        }
        let entry = file
            .entry
            .and_then(state_index)
            .unwrap_or(0)
            .min(states.len().saturating_sub(1));
        Self {
            parameters,
            states,
            any,
            out,
            entry,
            notes,
        }
    }

    /// Its parameters' names and kinds, in order.
    pub fn parameters(&self) -> &[(String, ParameterKind)] {
        &self.parameters
    }

    /// The parameter called `name`, by index.
    pub fn parameter(&self, name: &str) -> Option<usize> {
        self.parameters.iter().position(|(n, _)| n == name)
    }

    /// How many states it has (none: it does nothing).
    pub fn state_count(&self) -> usize {
        self.states.len()
    }

    /// A state's name ("" past the end).
    pub fn state_name(&self, state: usize) -> &str {
        self.states.get(state).map_or("", |s| s.name.as_str())
    }

    /// The state called `name`.
    pub fn state_named(&self, name: &str) -> Option<usize> {
        self.states.iter().position(|s| s.name == name)
    }

    /// The state it starts in.
    pub fn entry(&self) -> usize {
        self.entry
    }

    /// A run at the start: the entry state (or `state`, by name, when
    /// it has one by that name), every parameter at its start.
    pub fn start(&self, state: &str) -> Run {
        Run {
            state: self.state_named(state).unwrap_or(self.entry),
            values: self
                .parameters
                .iter()
                .map(|(_, k)| ParamValue::start(*k))
                .collect(),
        }
    }

    /// `run`, made for an earlier version of this animator (`before`), as
    /// a run of this one: the same state and values by name, the entry
    /// and starts for what's new or changed kind.
    pub fn carry(&self, before: &Self, run: &Run) -> Run {
        let mut fresh = self.start(before.state_name(run.state));
        for (i, (name, _)) in self.parameters.iter().enumerate() {
            if let Some(old) = before.parameter(name).and_then(|j| run.values.get(j))
                && old.same_kind(fresh.values[i])
            {
                fresh.values[i] = *old;
            }
        }
        fresh
    }

    /// Sets the parameter `name` (game code). A trigger is fired by any
    /// trigger value; a direction of no length is ignored (it keeps
    /// facing the way it was).
    pub fn set(&self, run: &mut Run, name: &str, value: ParamValue) -> Result<(), SetError> {
        let i = self.parameter(name).ok_or(SetError::NoParameter)?;
        let slot = run.values.get_mut(i).ok_or(SetError::NoParameter)?;
        if !slot.same_kind(value) {
            return Err(SetError::WrongKind(self.parameters[i].1.word()));
        }
        *slot = match value {
            ParamValue::Number(v) if v.is_finite() => ParamValue::Number(v.clamp(
                -crate::animator_file::MAX_NUMBER,
                crate::animator_file::MAX_NUMBER,
            )),
            ParamValue::Number(_) => return Err(SetError::NotUsable),
            ParamValue::Direction([x, y]) => {
                let length = (x * x + y * y).sqrt();
                if !(length.is_finite() && length > 1e-6) {
                    return Ok(());
                }
                ParamValue::Direction([x / length, y / length])
            }
            ParamValue::Trigger(_) => ParamValue::Trigger(true),
            other => other,
        };
        Ok(())
    }

    /// Moves `run` on one tick: takes the first transition whose
    /// conditions all hold (`finished`: the state's clip has played all
    /// its times), then puts triggers out. Returns whether the state
    /// changed. A run that doesn't fit (another animator's) starts again.
    pub fn step(&self, run: &mut Run, finished: bool) -> bool {
        if self.states.is_empty() {
            return false;
        }
        if run.state >= self.states.len() || run.values.len() != self.parameters.len() {
            *run = self.start("");
        }
        let holds = |test: &Test| match *test {
            Test::On(i) => run.values[i] == ParamValue::Switch(true),
            Test::Off(i) => run.values[i] == ParamValue::Switch(false),
            Test::Fired(i) => run.values[i] == ParamValue::Trigger(true),
            Test::Above(i, v) => matches!(run.values[i], ParamValue::Number(n) if n > v),
            Test::Below(i, v) => matches!(run.values[i], ParamValue::Number(n) if n < v),
            Test::Finished => finished,
            Test::Never => false,
        };
        let taken = self
            .any
            .iter()
            .filter(|w| w.to != run.state)
            .chain(&self.out[run.state])
            .find(|w| w.when.iter().all(holds))
            .map(|w| w.to);
        for v in &mut run.values {
            if let ParamValue::Trigger(fired) = v {
                *fired = false;
            }
        }
        match taken {
            Some(to) if to != run.state => {
                run.state = to;
                true
            }
            _ => false,
        }
    }

    /// The clip `run`'s state plays: its clip, or its clip for the
    /// direction nearest the facing parameter. `playing` is the clip
    /// playing now: when two are as near (facing between them), it stays,
    /// so a sprite doesn't flicker between them.
    pub fn clip<'a>(&'a self, run: &Run, playing: &str) -> &'a str {
        let Some(state) = self.states.get(run.state) else {
            return "";
        };
        match &state.plays {
            Played::Clip(c) => c,
            Played::ByDirection { param, clips } => {
                let facing = match param.and_then(|i| run.values.get(i)) {
                    Some(ParamValue::Direction(v)) => *v,
                    _ => Facing::Down.vector(),
                };
                let dot = |v: [f32; 2]| v[0] * facing[0] + v[1] * facing[1];
                let mut best: Option<(f32, &str)> = None;
                for (v, c) in clips {
                    let d = dot(*v);
                    if best.is_none_or(|(b, _)| d > b + 1e-4) {
                        best = Some((d, c));
                    }
                }
                let Some((b, chosen)) = best else {
                    return "";
                };
                clips
                    .iter()
                    .find(|(v, c)| c == playing && dot(*v) >= b - 1e-4)
                    .map_or(chosen, |(_, c)| c)
            }
        }
    }

    /// How fast `run`'s state plays its clips: its speed, times its speed
    /// parameter's value (0 for a negative value), within 0 to 16.
    pub fn speed(&self, run: &Run) -> f32 {
        let Some(state) = self.states.get(run.state) else {
            return 1.0;
        };
        let by = match state.speed_by.and_then(|i| run.values.get(i)) {
            Some(ParamValue::Number(n)) => n.max(0.0),
            _ => 1.0,
        };
        (state.speed * by).clamp(0.0, crate::animator_file::SPEED_RANGE.1)
    }
}

#[cfg(test)]
#[path = "animator_tests.rs"]
mod tests;
