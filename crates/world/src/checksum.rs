//! The world's checksum: one number for everything the game has set, for
//! telling whether a replay matches the game it recorded.

use crate::{Global, Id, Name, Values, World};
use area::builtin::AudioSource;
use reflect::EntityId;

impl World {
    /// A checksum of the game's world (Phase 6, determinism): every
    /// entity in order with its name, place and game components' values,
    /// the projectiles, physics and the seed. Two runs of a game that
    /// agree on it at a tick are in the same state (game code's running
    /// values are checked apart, by `script::Game::checksum`).
    pub fn checksum(&self) -> u64 {
        let mut sum = reflect::Checksum::new();
        // Not the clock: during a frame of several ticks it has moved on
        // by all of them before the first runs, and the tick's number is
        // known wherever checksums are compared.
        sum.u64(self.seed);
        // Which entities, in what tree, with what names and component
        // values: these change only with the area (which starts a new
        // revision), so they're worked out once for it.
        sum.u64(*self.structure.get_or_init(|| {
            let mut structure = reflect::Checksum::new();
            structure.u64(self.order.len() as u64);
            for &(entity, parent) in &self.order {
                let Ok(e) = self.ecs.entity(entity) else {
                    continue;
                };
                if let Some(id) = e.get::<&Id>() {
                    structure.entity(id.0);
                }
                structure.u64(parent.map_or(u64::MAX, |p| p as u64));
                if let Some(name) = e.get::<&Name>() {
                    structure.bytes(name.0.as_bytes());
                }
                if let Some(values) = e.get::<&Values>() {
                    for (name, data) in &values.0 {
                        structure.bytes(name.as_bytes());
                        structure.data(data);
                    }
                }
            }
            structure.finish()
        }));
        for &(entity, _) in &self.order {
            let Ok(global) = self.ecs.get::<&Global>(entity) else {
                continue;
            };
            // Where it is, turned and scaled: the matrix's top three rows
            // (the bottom is always 0 0 0 1).
            let m = global.0;
            for row in [m.row(0), m.row(1), m.row(2)] {
                for v in row.to_array() {
                    sum.f32(v);
                }
            }
        }
        // The game's speed, when game code changed it (so older
        // recordings' sums stand).
        if self.speed != 1.0 {
            sum.u64(self.speed.to_bits());
        }
        // The day moved on by game code, when it was (so older
        // recordings' sums stand).
        if self.day_shift != 0.0 {
            sum.u64(self.day_shift.to_bits());
        }
        // Which sounds game code turned on and off (it's in a save).
        // Only the on/off flag counts, so the sources aren't copied.
        let mut sounds: Vec<(EntityId, bool)> = self
            .ecs
            .query::<(&Id, &AudioSource)>()
            .iter()
            .map(|(id, source)| (id.0, source.playing))
            .collect();
        sounds.sort_unstable_by_key(|(id, _)| *id);
        for (id, playing) in sounds {
            sum.entity(id);
            sum.u64(u64::from(playing));
        }
        // Which clip each sprite plays and how far in (Phase 9, D3).
        for (id, clip, playing, time) in self.sprite_states() {
            sum.entity(id);
            sum.bytes(clip.as_bytes());
            sum.u64(u64::from(playing));
            sum.u64(u64::from(time));
        }
        // Which state each animator is in, and its parameters (D3b).
        for (id, state, values) in self.animator_states() {
            sum.entity(id);
            sum.bytes(state.as_bytes());
            for v in values {
                sum.bytes(v.as_bytes());
            }
        }
        // Every tile map's cells and values (D4).
        for (id, map) in self.map_sums() {
            sum.entity(id);
            sum.u64(map);
        }
        sum.u64(self.physics.state().checksum());
        sum.u64(self.flat.state().checksum());
        let flying = self.projectiles.flying();
        sum.u64(flying.len() as u64);
        for p in flying {
            for v in p
                .position
                .to_array()
                .into_iter()
                .chain(p.velocity.to_array())
            {
                sum.f32(v);
            }
            sum.f32(p.left);
            for v in p.colour {
                sum.f32(v);
            }
        }
        sum.finish()
    }
}
