//! A [`Game`]'s timers (stage 1, timers; decision 0125): kept by the game
//! itself, on the caller's thread, beside its shards. As each tick begins
//! game time moves on and what's due becomes `TimerFinished` events for
//! the first round, each for the one component that started its timer.
//! Starts and stops land at the end of the tick, after every phase, so
//! every phase and core asks the timers as the tick started.

use super::{Change, Game, GameWorld, WorldView};
use crate::events::Event;
use crate::states::StateChange;
use crate::timers::{Fired, TimerOp, Timers};
use reflect::{EntityId, Value};
use std::collections::BTreeSet;
use std::sync::Arc;

/// The event a finished timer sends.
pub(crate) const FINISHED: &str = "TimerFinished";

impl Game {
    /// The timers waiting, as the last tick left them: what a save keeps.
    pub fn timers(&self) -> &Timers {
        &self.timers
    }

    /// Puts back the timers a save kept (decision 0038): they fire on the
    /// tick they would have.
    pub fn set_timers(&mut self, timers: Timers) {
        self.timers = Arc::new(timers);
        self.timer_changes.clear();
    }

    /// The view of `world`, with the timers as the tick started.
    pub(super) fn view_of(&self, world: &mut dyn GameWorld) -> WorldView {
        let mut view = world.view();
        view.timers = Arc::clone(&self.timers);
        view.readings = Arc::clone(&self.readings);
        view
    }

    /// As a tick begins: game time moves on at `view`'s speed, and each
    /// timer due becomes an event for the component that started it, in
    /// order (due, entity, component, start).
    pub(super) fn timers_due(&mut self, view: &WorldView) -> Vec<Event> {
        let fired = Arc::make_mut(&mut self.timers).advance(view.game_speed, view.time_of_day);
        fired.into_iter().map(finished).collect()
    }

    /// Keeps what a phase landed for the timers, for the tick's end, and
    /// the state changes the components that changed hear (stage 1,
    /// states).
    pub(super) fn keep_landed(&mut self, (ops, states): Landed) {
        self.timer_changes.extend(ops);
        self.hear(states);
    }

    /// Makes the timer changes kept so far, in the order they landed;
    /// starts past the most timers are left out, said once until the code
    /// changes.
    pub(super) fn make_timer_changes(&mut self) -> Option<String> {
        if self.timer_changes.is_empty() {
            return None;
        }
        // A state left or kept changes nothing while no timer waits.
        let ops = std::mem::take(&mut self.timer_changes);
        if self.timers.is_empty() && !ops.iter().any(|op| matches!(op, TimerOp::Change(_))) {
            return None;
        }
        let timers = Arc::make_mut(&mut self.timers);
        let mut left_out = 0_usize;
        for op in ops {
            match op {
                TimerOp::Change(change) => {
                    if !timers.apply(&change) {
                        left_out += 1;
                    }
                }
                TimerOp::Left(change) => {
                    timers.state_left(change.entity, &change.component, &change.to);
                }
                TimerOp::Keep(entity, component) => timers.keep(entity, &component),
            }
        }
        (left_out > 0 && !std::mem::replace(&mut self.told_timers, true)).then(|| {
            format!(
                "{left_out} timers weren't started: a game keeps at most {} waiting at once.",
                crate::timers::MAX_TIMERS
            )
        })
    }

    /// Ends the timers of components no longer running: `keep` says which
    /// (entity, component) still run. Silent: a despawned entity's timers
    /// simply never finish.
    pub(super) fn end_timers_unless(&mut self, keep: impl FnMut(EntityId, &str) -> bool) {
        if !self.timers.is_empty() {
            Arc::make_mut(&mut self.timers).retain(keep);
        }
    }

    /// After new code: a timer whose component is gone from the code, or
    /// whose Lust component no longer starts a timer of its name, ends; a
    /// note says which, once.
    pub(super) fn end_timers_gone_from(&mut self, code: &crate::code::Compiled) -> Option<String> {
        if self.timers.is_empty() {
            return None;
        }
        let names: BTreeSet<&str> = code.timers.iter().map(String::as_str).collect();
        let rust = &self.rust;
        let lust = |component: &str| {
            code.places
                .get(component)
                .is_some_and(|(file, _)| !file.ends_with(".lua"))
        };
        let present = |component: &str| {
            rust.contains(component) || code.schemas.iter().any(|s| s.name == component)
        };
        let ended = Arc::make_mut(&mut self.timers).end_where(|_, component, name| {
            !present(component) || (lust(component) && !names.contains(name))
        });
        if ended.is_empty() {
            return None;
        }
        // Each component's timer once, with how many entities had it.
        let mut counted: Vec<(String, usize)> = Vec::new();
        for (_, component, name) in &ended {
            let shown = format!("{component}'s {name}");
            match counted.iter_mut().find(|(s, _)| *s == shown) {
                Some((_, n)) => *n += 1,
                None => counted.push((shown, 1)),
            }
        }
        counted.sort();
        let listed: Vec<String> = counted
            .iter()
            .take(8)
            .map(|(shown, n)| match n {
                1 => shown.clone(),
                n => format!("{shown} ({n})"),
            })
            .collect();
        let more = if counted.len() > listed.len() {
            " and more"
        } else {
            ""
        };
        Some(format!(
            "The new code no longer has {} timer{}, so {} stopped: {}{more}.",
            ended.len(),
            if ended.len() == 1 { "" } else { "s" },
            if ended.len() == 1 { "it" } else { "they" },
            listed.join(", "),
        ))
    }
}

/// A finished timer as the event its component hears.
fn finished(fired: Fired) -> Event {
    Event {
        to: fired.entity,
        name: FINISHED.to_string(),
        values: vec![Value::Text(fired.name.to_string())],
        only: Some(fired.component),
        during: fired.during,
    }
}

/// What a phase landed for the timers, in order, and the states that
/// changed.
pub(super) type Landed = (Vec<TimerOp>, Vec<StateChange>);

/// What `changes` do to the timers, in order: timers started and stopped
/// (taken out: the game keeps its timers, and the world never sees them),
/// states left and what's kept past them (left in, for the world's
/// sounds); and the states that changed.
pub(super) fn timer_ops(changes: &mut Vec<Change>) -> Landed {
    let mut ops = Vec::new();
    let mut states = Vec::new();
    changes.retain_mut(|change| match change {
        Change::Timer(timer) => {
            ops.push(TimerOp::Change(timer.clone()));
            false
        }
        Change::State(state) => {
            ops.push(TimerOp::Left(state.clone()));
            states.push(state.clone());
            true
        }
        Change::KeepStarted(entity, component) => {
            ops.push(TimerOp::Keep(*entity, Arc::clone(component)));
            true
        }
        _ => true,
    });
    (ops, states)
}
