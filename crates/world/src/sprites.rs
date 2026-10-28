//! Sprites playing clips in the running game (Phase 9, D3): each tick,
//! every playing Sprite's clip moves on by the tick times its speeds, and
//! the events it passes and the clips that finish are handed back to be
//! played as sounds and sent to game code.
//!
//! A Sprite's clip, whether it's playing and how far into the clip it is
//! are game state: in the checksum and in saves. They're plain additions
//! on each Sprite's own time, in a fixed order (by entity ID), so every PC
//! and every replay comes out the same.

use crate::{Id, World};
use animation::ClipSet;
use area::builtin::Sprite;
use reflect::{AssetId, EntityId};
use std::collections::HashMap;
use std::sync::Arc;

/// The clips each sheet (an Aseprite file) or clips file (`.solframes`)
/// offers, by its asset ID.
pub type ClipSets = HashMap<AssetId, Arc<ClipSet>>;

/// What a Sprite's clip did in a tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpriteCue {
    /// It reached a frame with this event ("footstep").
    Event(EntityId, String),
    /// It played all its times: this clip.
    Finished(EntityId, String),
}

/// A Sprite's clip, whether it's playing, and how far in: what the game
/// changes of a Sprite.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Playback {
    clip: String,
    playing: bool,
    time: f32,
}

impl Playback {
    pub(crate) fn of(sprite: &Sprite) -> Self {
        Self {
            clip: sprite.clip.clone(),
            playing: sprite.playing,
            time: sprite.time,
        }
    }

    fn put(&self, sprite: &mut Sprite) {
        sprite.clip.clone_from(&self.clip);
        sprite.playing = self.playing;
        sprite.time = self.time;
    }
}

/// A Sprite's playback as the area gave it when the world took it in: the
/// game's own playback is kept over an edit unless the edit changed the
/// area's.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SpriteStart(pub(crate) Playback);

impl World {
    /// Gives the world the clips sprites play (again whenever a sheet or a
    /// clips file changes).
    pub fn set_clip_sets(&mut self, sets: Arc<ClipSets>) {
        if !Arc::ptr_eq(&self.clip_sets, &sets) {
            self.clip_sets = sets;
        }
    }

    /// The clips sprites play.
    pub fn clip_sets(&self) -> &Arc<ClipSets> {
        &self.clip_sets
    }

    /// Plays `clip` on `id`'s Sprite (game code's `playClip`): already
    /// playing it, it carries on; stopped part way, it carries on from
    /// there; another clip, or one that has finished, starts from its
    /// beginning. Nothing happens for an entity without a Sprite.
    pub fn play_clip(&mut self, id: EntityId, clip: &str) {
        let Some(&entity) = self.by_id.get(&id) else {
            return;
        };
        let Ok(mut sprite) = self.ecs.get::<&mut Sprite>(entity) else {
            return;
        };
        let same = sprite.clip == clip;
        let finished = same
            && sprite
                .sheet
                .and_then(|s| self.clip_sets.get(&s.asset))
                .and_then(|set| set.chosen(clip))
                .and_then(animation::Clip::length_seconds)
                .is_some_and(|length| f64::from(sprite.time) >= length);
        if same && sprite.playing && !finished {
            return;
        }
        if !same || finished {
            sprite.clip = clip.to_owned();
            sprite.time = 0.0;
        }
        sprite.playing = true;
        self.touched = true;
    }

    /// Stops `id`'s Sprite where it is in its clip (game code's
    /// `stopClip`).
    pub fn stop_clip(&mut self, id: EntityId) {
        if let Some(&entity) = self.by_id.get(&id)
            && let Ok(mut sprite) = self.ecs.get::<&mut Sprite>(entity)
            && sprite.playing
        {
            sprite.playing = false;
            self.touched = true;
        }
    }

    /// Moves every playing Sprite's clip on by `dt` seconds times its own
    /// speed and its clip's, and gives back what happened: the events
    /// passed and the clips finished, by entity ID, each entity's in the
    /// order they happened. A clip that plays over and over keeps its
    /// time within one pass, so it never loses precision.
    pub fn step_sprites(&mut self, dt: f64) -> Vec<SpriteCue> {
        let sets = Arc::clone(&self.clip_sets);
        let mut cues: Vec<(EntityId, SpriteCue)> = Vec::new();
        let mut names = Vec::new();
        let mut any = false;
        for (id, sprite) in self.ecs.query_mut::<(&Id, &mut Sprite)>() {
            if !sprite.playing || sprite.speed <= 0.0 {
                continue;
            }
            let Some(clip) = sprite
                .sheet
                .and_then(|s| sets.get(&s.asset))
                .and_then(|set| set.chosen(&sprite.clip))
            else {
                continue;
            };
            let step = dt * f64::from(sprite.speed) * f64::from(clip.speed);
            if !(step.is_finite() && step > 0.0) {
                continue;
            }
            any = true;
            let from = f64::from(sprite.time);
            // The time as it will be kept, so the next tick starts exactly
            // where this one stopped and no event is passed twice.
            let to = f64::from((from + step) as f32);
            names.clear();
            clip.events_between(from, to, &mut names);
            cues.extend(
                names
                    .drain(..)
                    .map(|name| (id.0, SpriteCue::Event(id.0, name))),
            );
            match clip.length_seconds() {
                Some(length) if to >= length => {
                    sprite.time = length as f32;
                    sprite.playing = false;
                    cues.push((id.0, SpriteCue::Finished(id.0, clip.name.clone())));
                }
                Some(_) => sprite.time = to as f32,
                None => {
                    let pass = clip.pass_seconds();
                    sprite.time = if pass > 0.0 && to >= pass {
                        (to - (to / pass).floor() * pass) as f32
                    } else {
                        to as f32
                    };
                }
            }
        }
        if any {
            self.touched = true;
        }
        // By entity, keeping each one's own order.
        cues.sort_by_key(|(id, _)| *id);
        cues.into_iter().map(|(_, cue)| cue).collect()
    }

    /// Whether `id` has components of game code (or ones no one knows):
    /// only those can take the events sprites send.
    pub fn has_game_components(&self, id: EntityId) -> bool {
        self.by_id
            .get(&id)
            .is_some_and(|&e| self.ecs.satisfies::<&crate::Values>(e))
    }

    /// Every Sprite's playback, by entity ID: what the checksum covers of
    /// sprites.
    pub(crate) fn sprite_states(&self) -> Vec<(EntityId, String, bool, u32)> {
        let mut states: Vec<(EntityId, String, bool, u32)> = self
            .ecs
            .query::<(&Id, &Sprite)>()
            .iter()
            .map(|(id, s)| (id.0, s.clip.clone(), s.playing, s.time.to_bits()))
            .collect();
        states.sort_unstable_by_key(|(id, ..)| *id);
        states
    }

    /// Each Sprite's playback now with what the area gave it, by ID: kept
    /// over an edit that leaves the area's playback as it was.
    pub(crate) fn playbacks(&self) -> Vec<(EntityId, Playback, Playback)> {
        self.ecs
            .query::<(&Id, &Sprite, &SpriteStart)>()
            .iter()
            .filter(|(_, s, start)| Playback::of(s) != start.0)
            .map(|(id, s, start)| (id.0, start.0.clone(), Playback::of(s)))
            .collect()
    }

    /// Puts back the game's playback of each sprite whose area playback
    /// is still `start` (after a rebuild or an update).
    pub(crate) fn keep_playbacks(&mut self, kept: Vec<(EntityId, Playback, Playback)>) {
        for (id, start, now) in kept {
            if let Some(&entity) = self.by_id.get(&id)
                && let Ok((sprite, area_start)) = self
                    .ecs
                    .query_one_mut::<(&mut Sprite, &SpriteStart)>(entity)
                && area_start.0 == start
            {
                now.put(sprite);
            }
        }
    }
}

#[cfg(test)]
#[path = "sprites_tests.rs"]
mod tests;
