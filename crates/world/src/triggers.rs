//! Trigger zones in the running game (stage 1, triggers): Solis's own
//! overlap pass. Once a tick, on where physics' last step left everything,
//! each zone asks its world's physics which characters and bodies overlap
//! it (3D's for a box or a sphere, 2D's for every shape); the answers,
//! sorted by zone and then by entity, are compared with last tick's, and
//! each difference is a [`Crossing`] for game code to hear. What is inside
//! is game state: kept in saves and in the checksum, so loading a save
//! never says "entered" twice.
//!
//! Only where things are at each tick counts, so something that goes in
//! and out within one tick (a teleport straight through) crosses nothing.
//! Something taken out of the game while inside (despawned, or its body
//! gone) leaves: the zone hears `Exited`. A zone taken out says nothing.
//! A zone that fires once tells of the first tick anything enters it, and
//! never again (also kept in saves); it still knows who is inside.

use crate::{Global, Id, World};
use glam::{EulerRot, Vec2, Vec3};
use level::builtin::{Fires, TriggerZone, ZoneForm};
use physics::flat::{FlatZone, FlatZoneQuery};
use physics::{ZoneQuery, ZoneShape};
use reflect::EntityId;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// Something walking into or out of a zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Crossing {
    /// The zone.
    pub zone: EntityId,
    /// What walked in or out.
    pub who: EntityId,
    /// In (true) or out.
    pub entered: bool,
}

/// Who is inside each zone, by zone (each list sorted).
pub type InsideZones = BTreeMap<EntityId, Vec<EntityId>>;

/// The zones' game state, and what's made ready to ask physics with.
#[derive(Debug, Clone, Default)]
pub(crate) struct Triggers {
    /// Who is inside each zone, as (zone, who).
    inside: BTreeSet<(EntityId, EntityId)>,
    /// Zones that fire once and have.
    spent: BTreeSet<EntityId>,
    /// `inside` by zone, shared with game code each tick.
    by_zone: Arc<InsideZones>,
    /// Each zone's 2D shape, made when it or its scale changes.
    prepared: BTreeMap<EntityId, Prepared>,
}

/// A zone's 2D shape, and what it was made from.
#[derive(Debug, Clone)]
struct Prepared {
    zone: TriggerZone,
    scale: Vec2,
    flat: FlatZone,
}

/// One zone as the pass sees it this tick.
struct Placed {
    id: EntityId,
    zone: TriggerZone,
    place: Vec3,
    rotation: glam::Quat,
    scale: Vec3,
}

/// A zone's 2D shape, grown with its entity's scale: the outline's convex
/// pieces are worked out once, here, never each tick.
fn flat_shape(zone: &TriggerZone, scale: Vec2) -> FlatZone {
    let scale = scale.abs();
    match zone.form {
        ZoneForm::Box => FlatZone::rectangle(zone.size.truncate() * scale),
        ZoneForm::Round => FlatZone::circle(zone.radius * scale.max_element()),
        ZoneForm::Freeform => {
            let pieces = mesh::outline::convex_pieces(&zone.points);
            let grown: Vec<Vec<Vec2>> = pieces
                .into_iter()
                .map(|piece| piece.into_iter().map(|p| p * scale).collect())
                .collect();
            FlatZone::pieces(&grown)
        }
    }
}

/// A zone's 3D shape, grown with its entity's scale; none for a freeform
/// one, which is 2D only.
fn solid_shape(zone: &TriggerZone, scale: Vec3) -> Option<ZoneShape> {
    let scale = scale.abs();
    match zone.form {
        ZoneForm::Box => Some(ZoneShape::Box(zone.size * scale)),
        ZoneForm::Round => Some(ZoneShape::Ball(zone.radius * scale.max_element())),
        ZoneForm::Freeform => None,
    }
}

impl World {
    /// The overlap pass: who is inside each zone now, compared with the
    /// last pass, as crossings sorted by zone and then by entity. Called
    /// once a tick by the editor, as the tick begins, before game code.
    pub fn overlap_zones(&mut self) -> Vec<Crossing> {
        let zones = self.placed_zones();
        let now = self.who_is_inside(&zones);
        let crossings = self.triggers.cross(&zones, &now);
        self.triggers.inside = now;
        self.triggers.by_zone = Arc::new(by_zone(&self.triggers.inside));
        crossings
    }

    /// Every zone in the game, in entity order, with where it is.
    fn placed_zones(&self) -> Vec<Placed> {
        let mut zones: Vec<Placed> = self
            .ecs
            .query::<(&Id, &TriggerZone, &Global)>()
            .iter()
            .filter_map(|(id, zone, global)| {
                let (scale, rotation, place) = global.0.to_scale_rotation_translation();
                (place.is_finite() && rotation.is_finite() && scale.is_finite()).then(|| Placed {
                    id: id.0,
                    zone: zone.clone(),
                    place,
                    rotation,
                    scale,
                })
            })
            .collect();
        zones.sort_unstable_by_key(|z| z.id);
        zones
    }

    /// Who overlaps each zone now, from 3D and 2D physics together.
    fn who_is_inside(&mut self, zones: &[Placed]) -> BTreeSet<(EntityId, EntityId)> {
        let mut inside = BTreeSet::new();
        if zones.is_empty() {
            self.triggers.prepared.clear();
            return inside;
        }
        let solid: Vec<ZoneQuery> = zones
            .iter()
            .filter_map(|z| {
                Some(ZoneQuery {
                    zone: z.id,
                    place: z.place,
                    rotation: z.rotation,
                    shape: solid_shape(&z.zone, z.scale)?,
                })
            })
            .collect();
        for (query, found) in solid.iter().zip(self.physics.state().overlapping(&solid)) {
            inside.extend(found.into_iter().map(|who| (query.zone, who)));
        }
        self.prepare(zones);
        let flat: Vec<FlatZoneQuery<'_>> = zones
            .iter()
            .filter_map(|z| {
                Some(FlatZoneQuery {
                    zone: z.id,
                    place: z.place.truncate(),
                    angle: z.rotation.to_euler(EulerRot::ZYX).0,
                    shape: &self.triggers.prepared.get(&z.id)?.flat,
                })
            })
            .collect();
        for (query, found) in flat.iter().zip(self.flat.overlapping(&flat)) {
            inside.extend(found.into_iter().map(|who| (query.zone, who)));
        }
        inside
    }

    /// Makes each zone's 2D shape again where the zone or its scale
    /// changed, and forgets those of zones gone.
    fn prepare(&mut self, zones: &[Placed]) {
        let prepared = &mut self.triggers.prepared;
        prepared.retain(|id, _| zones.binary_search_by_key(id, |z| z.id).is_ok());
        for z in zones {
            let scale = z.scale.truncate();
            let fresh = prepared
                .get(&z.id)
                .is_some_and(|p| p.zone == z.zone && p.scale == scale);
            if !fresh {
                prepared.insert(
                    z.id,
                    Prepared {
                        zone: z.zone.clone(),
                        scale,
                        flat: flat_shape(&z.zone, scale),
                    },
                );
            }
        }
    }

    /// Who is inside `zone` now (as the last pass found), in entity order.
    pub fn inside(&self, zone: EntityId) -> &[EntityId] {
        self.triggers.by_zone.get(&zone).map_or(&[], Vec::as_slice)
    }

    /// Who is inside every zone now, by zone, shared: what game code's
    /// `inside()` reads for the tick.
    pub fn inside_zones(&self) -> Arc<InsideZones> {
        Arc::clone(&self.triggers.by_zone)
    }

    /// The zones' game state, as a save keeps it: who is inside each
    /// (zone, who), and the zones that fired once and have.
    pub fn zone_states(&self) -> (Vec<(EntityId, EntityId)>, Vec<EntityId>) {
        (
            self.triggers.inside.iter().copied().collect(),
            self.triggers.spent.iter().copied().collect(),
        )
    }

    /// Gives the zones a save's game state (see [`Self::zone_states`]).
    pub fn set_zone_states(&mut self, inside: &[(EntityId, EntityId)], spent: &[EntityId]) {
        self.triggers.inside = inside.iter().copied().collect();
        self.triggers.spent = spent.iter().copied().collect();
        self.triggers.by_zone = Arc::new(by_zone(&self.triggers.inside));
        self.touched = true;
    }

    /// Adds the zones' game state to a checksum: nothing while nothing is
    /// inside a zone and none has fired once, so older recordings' sums
    /// stand.
    pub(crate) fn sum_zones(&self, sum: &mut reflect::Checksum) {
        let (inside, spent) = (&self.triggers.inside, &self.triggers.spent);
        if inside.is_empty() && spent.is_empty() {
            return;
        }
        sum.u64(inside.len() as u64);
        for (zone, who) in inside {
            sum.entity(*zone);
            sum.entity(*who);
        }
        sum.u64(spent.len() as u64);
        for zone in spent {
            sum.entity(*zone);
        }
    }
}

impl Triggers {
    /// The crossings from what was inside to `now`, sorted, with zones
    /// that fire once marked as having fired. Pairs of zones that are gone
    /// are dropped silently.
    fn cross(&mut self, zones: &[Placed], now: &BTreeSet<(EntityId, EntityId)>) -> Vec<Crossing> {
        let fires = |id: EntityId| {
            zones
                .binary_search_by_key(&id, |z| z.id)
                .ok()
                .map(|at| zones[at].zone.fires)
        };
        self.spent.retain(|id| fires(*id).is_some());
        let mut crossings: Vec<Crossing> = self
            .inside
            .difference(now)
            .filter(|(zone, _)| fires(*zone).is_some())
            .map(|&(zone, who)| Crossing {
                zone,
                who,
                entered: false,
            })
            .chain(now.difference(&self.inside).map(|&(zone, who)| Crossing {
                zone,
                who,
                entered: true,
            }))
            .collect();
        crossings.sort_unstable();
        // A zone that fires once tells of its first entering, then stops.
        let mut first: BTreeSet<EntityId> = BTreeSet::new();
        crossings.retain(|c| match fires(c.zone) {
            Some(Fires::Once) => {
                if self.spent.contains(&c.zone) {
                    return false;
                }
                if c.entered {
                    first.insert(c.zone);
                }
                c.entered
            }
            _ => true,
        });
        self.spent.extend(first);
        crossings
    }
}

/// `inside` as lists by zone.
fn by_zone(inside: &BTreeSet<(EntityId, EntityId)>) -> InsideZones {
    let mut zones = InsideZones::new();
    for (zone, who) in inside {
        zones.entry(*zone).or_default().push(*who);
    }
    zones
}

#[cfg(test)]
#[path = "triggers_tests.rs"]
mod tests;
