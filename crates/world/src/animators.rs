//! Animators in the running game (Phase 9, D3b): each tick, before
//! sprites move on, every entity's animator takes the transition its
//! parameters call for, and sets the clip and speed of the Sprite on the
//! same entity.
//!
//! An animator's state and parameters are game state: in the checksum and
//! in saves. Each entity's step uses only its own values, set by game code
//! in a fixed order, so every PC and every replay come out the same.

use crate::{Id, World};
use animation::{Animator as Machine, ParamValue, Run, SetError};
use area::builtin::{Animator, Sprite};
use reflect::{AssetId, EntityId};
use std::collections::HashMap;
use std::sync::Arc;

/// The animator files ready to run, by asset ID.
pub type Animators = HashMap<AssetId, Arc<Machine>>;

/// One entity's animator at work: the animator it runs (to notice it
/// changed) and its place in it. Kept beside the entity's components, and
/// over edits while playing.
#[derive(Debug, Clone)]
pub(crate) struct Running {
    of: Arc<Machine>,
    run: Run,
}

/// A parameter's value as saves and the checksum keep it: its kind and
/// its exact bits.
pub fn value_words(value: ParamValue) -> String {
    match value {
        ParamValue::Number(n) => format!("n{:x}", n.to_bits()),
        ParamValue::Switch(b) => format!("s{}", u8::from(b)),
        ParamValue::Trigger(b) => format!("t{}", u8::from(b)),
        ParamValue::Direction([x, y]) => format!("d{:x},{:x}", x.to_bits(), y.to_bits()),
    }
}

/// A value written by [`value_words`], if it's one.
pub fn value_of_words(word: &str) -> Option<ParamValue> {
    let bits = |w: &str| u32::from_str_radix(w, 16).ok().map(f32::from_bits);
    let flag = |w: &str| match w {
        "0" => Some(false),
        "1" => Some(true),
        _ => None,
    };
    let rest = word.get(1..)?;
    match word.get(..1)? {
        "n" => bits(rest).filter(|n| n.is_finite()).map(ParamValue::Number),
        "s" => flag(rest).map(ParamValue::Switch),
        "t" => flag(rest).map(ParamValue::Trigger),
        "d" => {
            let (x, y) = rest.split_once(',')?;
            let v = [bits(x)?, bits(y)?];
            v.iter()
                .all(|c| c.is_finite())
                .then_some(ParamValue::Direction(v))
        }
        _ => None,
    }
}

impl World {
    /// Gives the world the animators entities run (again whenever an
    /// animator file changes: each entity's run carries over by name).
    pub fn set_animators(&mut self, animators: Arc<Animators>) {
        if !Arc::ptr_eq(&self.animators, &animators) {
            self.animators = animators;
        }
    }

    /// The animators entities run.
    pub fn animators(&self) -> &Arc<Animators> {
        &self.animators
    }

    /// The run `entity` should start with (or carry over to a changed
    /// file), if its animator has none, or an old one.
    fn run_to_start(&self, entity: hecs::Entity) -> Option<Running> {
        let mut query = self
            .ecs
            .query_one::<(&Id, &Animator, Option<&Running>)>(entity);
        let (id, animator, running) = query.get().ok()?;
        let machine = animator
            .animator
            .and_then(|a| self.animators.get(&a.asset))?;
        if running.is_some_and(|r| Arc::ptr_eq(&r.of, machine)) {
            return None;
        }
        let mut run = match running {
            Some(r) => machine.carry(&r.of, &r.run),
            None => machine.start(&animator.state),
        };
        // Values a loaded save gives it.
        if let Some(values) = self.saved_values.get(&id.0) {
            for (name, value) in values {
                if let Some(i) = machine.parameter(name)
                    && run.values[i].same_kind(*value)
                {
                    run.values[i] = *value;
                }
            }
        }
        Some(Running {
            of: Arc::clone(machine),
            run,
        })
    }

    /// Starts `entity`'s run, if it needs one.
    fn start_run(&mut self, entity: hecs::Entity) {
        if let Some(running) = self.run_to_start(entity) {
            if let Ok(id) = self.ecs.get::<&Id>(entity).map(|id| id.0) {
                self.saved_values.remove(&id);
            }
            let _ = self.ecs.insert_one(entity, running);
        }
    }

    /// Starts (or carries over to a changed file) the run of every entity
    /// whose animator has none, or an old one.
    fn start_runs(&mut self) {
        let entities: Vec<hecs::Entity> = self
            .ecs
            .query::<(hecs::Entity, &Animator, Option<&Running>)>()
            .iter()
            .filter(|(_, a, running)| {
                let machine = a.animator.and_then(|a| self.animators.get(&a.asset));
                match (machine, running) {
                    (None, _) => false,
                    (Some(m), Some(r)) => !Arc::ptr_eq(&r.of, m),
                    (Some(_), None) => true,
                }
            })
            .map(|(e, ..)| e)
            .collect();
        for entity in entities {
            self.start_run(entity);
        }
    }

    /// Moves every animator on one tick (before [`Self::step_sprites`]):
    /// each takes the transition its parameters call for, and sets its
    /// Sprite's clip (restarted when the state changed; changed in place,
    /// keeping its time, when only the facing did) and speed.
    pub fn step_animators(&mut self) {
        self.start_runs();
        let sets = Arc::clone(&self.clip_sets);
        let mut any = false;
        for (animator, running, sprite) in
            self.ecs
                .query_mut::<(&mut Animator, &mut Running, Option<&mut Sprite>)>()
        {
            let machine = Arc::clone(&running.of);
            let finished = sprite.as_deref().is_some_and(|s| {
                !s.playing
                    && s.sheet
                        .and_then(|sheet| sets.get(&sheet.asset))
                        .and_then(|set| set.chosen(&s.clip))
                        .and_then(animation::Clip::length_seconds)
                        .is_some_and(|length| f64::from(s.time) >= length)
            });
            let changed = machine.step(&mut running.run, finished);
            any |= changed;
            let name = machine.state_name(running.run.state);
            if animator.state != name {
                animator.state = name.to_owned();
                any = true;
            }
            let Some(sprite) = sprite else {
                continue;
            };
            let clip = machine.clip(&running.run, &sprite.clip);
            if !clip.is_empty() {
                if changed {
                    sprite.clip = clip.to_owned();
                    sprite.time = 0.0;
                    sprite.playing = true;
                    any = true;
                } else if sprite.clip != clip {
                    // Turned: the same step in the new direction.
                    sprite.clip = clip.to_owned();
                    any = true;
                }
            }
            let speed =
                (machine.speed(&running.run) * animator.speed).clamp(0.0, Sprite::MAX_SPEED);
            if sprite.speed != speed {
                sprite.speed = speed;
                any = true;
            }
        }
        if any {
            self.touched = true;
        }
    }

    /// Sets a parameter of `id`'s animator (game code). Taken at the next
    /// tick's step; a trigger stays fired until then.
    pub fn set_animator_value(
        &mut self,
        id: EntityId,
        name: &str,
        value: ParamValue,
    ) -> Result<(), SetError> {
        let entity = *self.by_id.get(&id).ok_or(SetError::NoParameter)?;
        self.start_run(entity);
        let mut running = self
            .ecs
            .get::<&mut Running>(entity)
            .map_err(|_| SetError::NoParameter)?;
        let machine = Arc::clone(&running.of);
        let before = running.run.clone();
        machine.set(&mut running.run, name, value)?;
        if running.run != before {
            self.touched = true;
        }
        Ok(())
    }

    /// Puts `id`'s animator straight into the state called `state` (game
    /// code), its clip started at once. Whether it has that state.
    pub fn jump_to_animator_state(&mut self, id: EntityId, state: &str) -> bool {
        let Some(&entity) = self.by_id.get(&id) else {
            return false;
        };
        self.start_run(entity);
        let Ok((animator, running, sprite)) =
            self.ecs
                .query_one_mut::<(&mut Animator, &mut Running, Option<&mut Sprite>)>(entity)
        else {
            return false;
        };
        let Some(to) = running.of.state_named(state) else {
            return false;
        };
        running.run.state = to;
        animator.state = state.to_owned();
        if let Some(sprite) = sprite {
            let clip = running.of.clip(&running.run, &sprite.clip);
            if !clip.is_empty() {
                sprite.clip = clip.to_owned();
                sprite.time = 0.0;
                sprite.playing = true;
            }
        }
        self.touched = true;
        true
    }

    /// The state `id`'s animator is in, by name, and its parameters'
    /// values by name (for game code, the editor and saves).
    pub fn animator_of(&self, id: EntityId) -> Option<(String, Vec<(String, ParamValue)>)> {
        let entity = *self.by_id.get(&id)?;
        let running = self.ecs.get::<&Running>(entity).ok()?;
        let names = running.of.parameters().iter().map(|(n, _)| n.clone());
        Some((
            running.of.state_name(running.run.state).to_owned(),
            names.zip(running.run.values.iter().copied()).collect(),
        ))
    }

    /// Every running animator's parameters, by entity ID (for saves).
    pub fn animator_values(&self) -> Vec<(EntityId, Vec<(String, ParamValue)>)> {
        let mut all: Vec<(EntityId, Vec<(String, ParamValue)>)> = self
            .ecs
            .query::<(&Id, &Running)>()
            .iter()
            .map(|(id, r)| {
                let names = r.of.parameters().iter().map(|(n, _)| n.clone());
                (id.0, names.zip(r.run.values.iter().copied()).collect())
            })
            .collect();
        all.extend(
            self.saved_values
                .iter()
                .filter(|(id, _)| {
                    self.by_id
                        .get(id)
                        .is_some_and(|&e| !self.ecs.satisfies::<&Running>(e))
                })
                .map(|(id, v)| (*id, v.clone())),
        );
        all.sort_unstable_by_key(|(id, _)| *id);
        all
    }

    /// Gives entities the parameter values a save had (a load): taken
    /// when each animator starts running, by name.
    pub fn restore_animator_values(&mut self, values: Vec<(EntityId, Vec<(String, ParamValue)>)>) {
        self.saved_values.extend(values);
        self.touched = true;
    }

    /// Every animator's state (by name) and parameter values, by entity
    /// ID: what the checksum covers of animators. One whose run hasn't
    /// started yet (a save just loaded) counts by what it will start
    /// from, so a loaded game sums as the saved one did.
    pub(crate) fn animator_states(&self) -> Vec<(EntityId, String, Vec<String>)> {
        let mut states: Vec<(EntityId, String, Vec<String>)> = self
            .ecs
            .query::<(&Id, &Animator, Option<&Running>)>()
            .iter()
            .filter_map(|(id, a, running)| match running {
                Some(r) => Some((
                    id.0,
                    r.of.state_name(r.run.state).to_owned(),
                    r.run.values.iter().map(|v| value_words(*v)).collect(),
                )),
                None => {
                    let saved = self.saved_values.get(&id.0)?;
                    Some((
                        id.0,
                        a.state.clone(),
                        saved.iter().map(|(_, v)| value_words(*v)).collect(),
                    ))
                }
            })
            .collect();
        states.sort_unstable_by_key(|(id, ..)| *id);
        states
    }

    /// Each entity's run, by ID, to keep over a rebuild.
    pub(crate) fn runs(&self) -> Vec<(EntityId, Running)> {
        self.ecs
            .query::<(&Id, &Running)>()
            .iter()
            .map(|(id, r)| (id.0, r.clone()))
            .collect()
    }

    /// Puts back runs kept over a rebuild or an update, on entities that
    /// still have an Animator.
    pub(crate) fn keep_runs(&mut self, runs: Vec<(EntityId, Running)>) {
        for (id, running) in runs {
            if let Some(&entity) = self.by_id.get(&id)
                && self.ecs.satisfies::<&Animator>(entity)
            {
                let _ = self.ecs.insert_one(entity, running);
            }
        }
    }
}

#[cfg(test)]
#[path = "animators_tests.rs"]
mod tests;
