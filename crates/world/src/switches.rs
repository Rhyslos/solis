//! What game code switches off on an entity (Fixes 2c): `setSolid(false)`
//! takes its collider out of the way (in 2D it leaves 2D physics; in 3D
//! its colliders are turned off and its body keeps moving), and
//! `setVisible(false)` stops it and everything under it being drawn.
//!
//! They're the world's own lists, not fields the level holds, so a level
//! file never changes for them. They're game state: in the checksum and in
//! saves, and they outlive an edit while playing because the world keeps
//! them by entity ID. A despawned entity leaves both.

use crate::World;
use reflect::EntityId;
use std::collections::BTreeSet;

/// The entities game code switched off.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Switches {
    /// Hidden, with everything under them.
    pub hidden: BTreeSet<EntityId>,
    /// Their colliders out of the way.
    pub unsolid: BTreeSet<EntityId>,
}

/// The most entities either list holds: far more than any game hides.
const MOST: usize = 1 << 20;

impl World {
    /// Shows `id` and everything under it again (`true`), or hides them.
    /// Only drawing: they still collide, and their code still runs.
    pub fn set_visible(&mut self, id: EntityId, visible: bool) {
        if !self.by_id.contains_key(&id) {
            return;
        }
        let changed = if visible {
            self.switches.hidden.remove(&id)
        } else {
            self.switches.hidden.len() < MOST && self.switches.hidden.insert(id)
        };
        if changed {
            self.touched = true;
        }
    }

    /// Turns `id`'s collider back on (`true`), or off.
    pub fn set_solid(&mut self, id: EntityId, solid: bool) {
        if !self.by_id.contains_key(&id) {
            return;
        }
        let changed = if solid {
            self.switches.unsolid.remove(&id)
        } else {
            self.switches.unsolid.len() < MOST && self.switches.unsolid.insert(id)
        };
        if changed {
            self.switch_physics(id);
        }
    }

    /// Makes physics match `id`'s switch: 2D physics takes it out or in
    /// again; a 3D body keeps moving as it did, its colliders turned off
    /// or on.
    fn switch_physics(&mut self, id: EntityId) {
        self.flat_join(id);
        self.physics.set_solid(id, self.is_solid(id));
        self.touched = true;
    }

    /// Whether `id`'s collider is on (game code hasn't turned it off).
    pub fn is_solid(&self, id: EntityId) -> bool {
        !self.switches.unsolid.contains(&id)
    }

    /// Whether `id` is drawn: neither it nor anything it's under is
    /// hidden.
    pub fn is_shown(&self, id: EntityId) -> bool {
        if self.switches.hidden.is_empty() {
            return true;
        }
        let mut at = self.place.get(&id).copied();
        while let Some(here) = at {
            let Some(&(entity, parent)) = self.order.get(here) else {
                break;
            };
            if let Ok(found) = self.ecs.get::<&crate::Id>(entity)
                && self.switches.hidden.contains(&found.0)
            {
                return false;
            }
            at = parent;
        }
        true
    }

    /// What game code has switched off, for saves.
    pub fn switches(&self) -> &Switches {
        &self.switches
    }

    /// Puts back what a save switched off (entities not in the world are
    /// left out), and makes physics match.
    pub fn set_switches(&mut self, switches: Switches) {
        let known = |ids: BTreeSet<EntityId>| -> BTreeSet<EntityId> {
            ids.into_iter()
                .filter(|id| self.by_id.contains_key(id))
                .take(MOST)
                .collect()
        };
        let hidden = known(switches.hidden);
        let unsolid = known(switches.unsolid);
        let rejoin: Vec<EntityId> = self
            .switches
            .unsolid
            .symmetric_difference(&unsolid)
            .copied()
            .collect();
        self.switches = Switches { hidden, unsolid };
        for id in rejoin {
            self.switch_physics(id);
        }
        self.touched = true;
    }

    /// An entity that left the world leaves both lists.
    pub(crate) fn forget_switches(&mut self, id: EntityId) {
        self.switches.hidden.remove(&id);
        self.switches.unsolid.remove(&id);
    }

    /// Adds what's switched off to a checksum (nothing when nothing is,
    /// so older recordings' sums stand).
    pub(crate) fn sum_switches(&self, sum: &mut reflect::Checksum) {
        for (word, ids) in [(1, &self.switches.hidden), (2, &self.switches.unsolid)] {
            if ids.is_empty() {
                continue;
            }
            sum.u64(word);
            sum.u64(ids.len() as u64);
            for &id in ids {
                sum.entity(id);
            }
        }
    }
}

#[cfg(test)]
#[path = "switches_tests.rs"]
mod tests;
