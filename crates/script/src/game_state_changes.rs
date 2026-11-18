//! A [`Game`]'s components hearing their own state change (stage 1,
//! states; decision 0126). Each change lands in the tick's order; as the
//! tick ends, each component's changes are added up (the first `from`,
//! the last `to`), and one that ended where it began says nothing. What's
//! heard waits for the next tick's first round, after the timers'
//! events, by entity and then component: game state, saved and in the
//! checksum, so a loaded game hears exactly what the running one would.
//!
//! A save keeps what's waiting as a line each:
//!
//! ```text
//! ENTITY COMPONENT EVENT FROM TO
//! ```

use super::Game;
use crate::events::Event;
use crate::states::StateChange;
use reflect::{EntityId, Value};
use std::sync::Arc;

/// The longest component, event or option name a save's line may hold.
const MAX_NAME: usize = 200;

impl Game {
    /// Keeps the state changes that landed, for the components that hear
    /// theirs: the others' cost nothing more.
    pub(super) fn hear(&mut self, changes: Vec<StateChange>) {
        if self.state_events.is_empty() {
            return;
        }
        let events = Arc::clone(&self.state_events);
        self.heard.extend(
            changes
                .into_iter()
                .filter(|c| events.contains_key(&*c.component)),
        );
    }

    /// As a tick ends: each component's changes, added up, become the
    /// event it hears next tick (none when it's back where it began).
    pub(super) fn notice_states(&mut self) {
        if self.heard.is_empty() {
            return;
        }
        let mut heard = std::mem::take(&mut self.heard);
        // Stable, so each component's changes stay in the order they
        // landed: its first from and its last to.
        heard.sort_by(|a, b| (a.entity, &a.component).cmp(&(b.entity, &b.component)));
        let mut at = 0;
        while at < heard.len() {
            let first = &heard[at];
            let mut last = at;
            while heard
                .get(last + 1)
                .is_some_and(|c| c.entity == first.entity && c.component == first.component)
            {
                last += 1;
            }
            let to = &heard[last].to;
            if first.from != *to
                && let Some(event) = self.state_events.get(&*first.component)
            {
                self.states_waiting.push(Event {
                    to: first.entity,
                    name: event.clone(),
                    values: vec![
                        Value::Text(first.from.to_string()),
                        Value::Text(to.to_string()),
                    ],
                    only: Some(Arc::clone(&first.component)),
                    during: None,
                });
            }
            at = last + 1;
        }
        self.heard = heard;
        self.heard.clear();
    }

    /// The state changes waiting to be heard next tick, as a save keeps
    /// them; empty while none waits.
    pub fn states_waiting(&self) -> String {
        let mut text = String::new();
        for event in &self.states_waiting {
            let (Some(component), [Value::Text(from), Value::Text(to)]) =
                (&event.only, event.values.as_slice())
            else {
                continue;
            };
            text.push_str(&format!(
                "{} {component} {} {from} {to}\n",
                event.to, event.name
            ));
        }
        text
    }

    /// Puts back the state changes a save kept waiting (decision 0038), or
    /// says why they can't be: a damaged section is refused whole.
    pub fn set_states_waiting(&mut self, text: &str) -> Result<(), String> {
        let waiting = read_waiting(text)?;
        self.states_waiting = waiting;
        self.heard.clear();
        Ok(())
    }

    /// A checksum of the state changes waiting to be heard, or none while
    /// none waits (so a game from before states keeps its checksums).
    pub(super) fn states_checksum(&self) -> Option<u64> {
        if self.states_waiting.is_empty() {
            return None;
        }
        let mut sum = reflect::Checksum::new();
        sum.bytes(b"states");
        for event in &self.states_waiting {
            sum.u64(event.to.0.raw());
            sum.bytes(event.only.as_deref().unwrap_or("").as_bytes());
            sum.bytes(event.name.as_bytes());
            for value in &event.values {
                if let Value::Text(option) = value {
                    sum.bytes(option.as_bytes());
                }
            }
        }
        Some(sum.finish())
    }
}

/// The waiting changes a save's text holds, each line checked.
fn read_waiting(text: &str) -> Result<Vec<Event>, String> {
    let damaged = || "its waiting state changes are damaged".to_string();
    let mut waiting = Vec::new();
    for line in text.lines() {
        if waiting.len() >= crate::events::MAX_WAITING {
            return Err(damaged());
        }
        let words: Vec<&str> = line.split(' ').collect();
        let [entity, component, event, from, to] = words.as_slice() else {
            return Err(damaged());
        };
        let entity = entity
            .parse::<reflect::Id>()
            .map(EntityId)
            .map_err(|_| damaged())?;
        let named = |word: &str, dots: bool| {
            !word.is_empty()
                && word.len() <= MAX_NAME
                && word.split('.').count() <= if dots { MAX_NAME } else { 1 }
                && word.split('.').all(|part| {
                    part.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
                        && part.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                })
        };
        let fits = named(component, false)
            && named(event, false)
            && event.ends_with(crate::state_names::CHANGED)
            && named(from, true)
            && named(to, true)
            && from != to;
        if !fits {
            return Err(damaged());
        }
        waiting.push(Event {
            to: entity,
            name: (*event).to_string(),
            values: vec![
                Value::Text((*from).to_string()),
                Value::Text((*to).to_string()),
            ],
            only: Some(Arc::from(*component)),
            during: None,
        });
    }
    Ok(waiting)
}
