//! The ground in the running game (Tools 1b, G1): the running game's copy
//! of the editor's ground, and its chunks made solid as their colliders
//! arrive.
//!
//! - **The copy** is the editor's `Arc`: copy-on-write, as tile maps'
//!   layers are, so playing costs no copy and Stop leaves the editor's
//!   ground as it was. Game code's edits (G5) will change chunks of the
//!   copy only.
//! - **Colliders** are built on workers (`physics::prepare`) by whoever
//!   meshes the chunks, and only added here, on the game's thread
//!   ([`World::add_ground_shapes`]). Until a chunk's arrives, characters
//!   are held out of it (`physics::Physics::hold_out`), so none drops
//!   through ground that isn't solid yet.
//!
//! Each chunk's collider is a still body of its own in physics, under an
//! ID made from the ground entity's and the chunk's place.

use crate::World;
use glam::{IVec3, Vec3};
use reflect::EntityId;
use std::collections::BTreeMap;
use std::sync::Arc;

/// The ground as the game has it.
#[derive(Debug, Clone)]
pub(crate) struct GroundPlay {
    /// The game's copy of the ground.
    ground: Arc<ground::Ground>,
    /// The Ground entity.
    entity: EntityId,
    /// Whether it's solid.
    solid: bool,
    /// Each chunk that has triangles: its collider, once it has arrived.
    shapes: BTreeMap<[i32; 3], Option<physics::PreparedShape>>,
}

impl World {
    /// Gives the game its ground: `ground` (the editor's, shared), on the
    /// Ground entity `entity`, solid or not. `needed` are the chunks that
    /// may have triangles; `ready` the colliders made so far. Characters
    /// are held out of the needed chunks whose collider isn't here yet.
    pub fn set_ground(
        &mut self,
        ground: Arc<ground::Ground>,
        (entity, solid): (EntityId, bool),
        needed: &[IVec3],
        ready: Vec<(IVec3, Option<physics::PreparedShape>)>,
    ) {
        let shapes = needed.iter().map(|k| (k.to_array(), None)).collect();
        self.ground = Some(GroundPlay {
            ground,
            entity,
            solid,
            shapes,
        });
        self.add_ground_shapes(ready);
    }

    /// The game's copy of the ground, if the level has one.
    pub fn ground(&self) -> Option<&Arc<ground::Ground>> {
        self.ground.as_ref().map(|g| &g.ground)
    }

    /// Colliders made on a worker for chunks of the ground (`None` for a
    /// chunk that turned out to have no triangles), added to physics here.
    pub fn add_ground_shapes(&mut self, shapes: Vec<(IVec3, Option<physics::PreparedShape>)>) {
        let Some(play) = &mut self.ground else {
            return;
        };
        let mut joined = Vec::new();
        for (key, shape) in shapes {
            match shape {
                Some(shape) => {
                    play.shapes.insert(key.to_array(), Some(shape.clone()));
                    joined.push((key, shape));
                }
                None => {
                    play.shapes.remove(&key.to_array());
                }
            }
        }
        let (entity, solid) = (play.entity, play.solid);
        if solid {
            for (key, shape) in joined {
                self.join_ground_chunk(entity, key, shape);
            }
        }
        self.hold_out_of_unready_ground();
    }

    /// Whether every chunk of the ground that needs a collider has one.
    pub fn ground_is_solid(&self) -> bool {
        self.ground
            .as_ref()
            .is_none_or(|g| !g.solid || g.shapes.values().all(Option::is_some))
    }

    /// Adds every collider the ground has to physics again (after physics
    /// was emptied and built again).
    pub(crate) fn join_ground(&mut self) {
        let Some(play) = &self.ground else {
            return;
        };
        if play.solid {
            let entity = play.entity;
            let ready: Vec<(IVec3, physics::PreparedShape)> = play
                .shapes
                .iter()
                .filter_map(|(k, s)| Some((IVec3::from_array(*k), s.clone()?)))
                .collect();
            for (key, shape) in ready {
                self.join_ground_chunk(entity, key, shape);
            }
        }
        self.hold_out_of_unready_ground();
    }

    fn join_ground_chunk(&mut self, entity: EntityId, key: IVec3, shape: physics::PreparedShape) {
        let Some(id) = chunk_id(entity, key) else {
            return;
        };
        self.physics.add(
            id,
            (ground::store::origin(key).as_vec3(), glam::Quat::IDENTITY),
            Vec3::ONE,
            None,
            Some(physics::ColliderSpec {
                shape: physics::Shape::Prepared(shape),
                ..physics::ColliderSpec::default()
            }),
        );
    }

    /// Holds characters out of every chunk still waiting for its collider.
    fn hold_out_of_unready_ground(&mut self) {
        let boxes = match &self.ground {
            Some(play) if play.solid => play
                .shapes
                .iter()
                .filter(|(_, s)| s.is_none())
                .map(|(k, _)| {
                    let lo = ground::store::origin(IVec3::from_array(*k)).as_vec3();
                    (lo, lo + Vec3::splat(ground::voxel::SIZE as f32))
                })
                .collect(),
            _ => Vec::new(),
        };
        self.physics.hold_out(boxes);
    }
}

/// A chunk's collider, made from its mesh: points at the same place
/// joined, so the collider knows which triangles meet and smooths over
/// the seams between them, and made into physics' shape. The slow part of
/// a chunk's collider, so call it on a worker; `None` for a chunk with no
/// triangles.
pub fn prepare_ground_chunk(mesh: &ground::ChunkMesh) -> Option<physics::PreparedShape> {
    let mut at: std::collections::HashMap<[u32; 3], u32> =
        std::collections::HashMap::with_capacity(mesh.positions.len());
    let mut points = Vec::new();
    let ids: Vec<u32> = mesh
        .positions
        .iter()
        .map(|p| {
            *at.entry(p.map(f32::to_bits)).or_insert_with(|| {
                points.push(Vec3::from_array(*p));
                points.len() as u32 - 1
            })
        })
        .collect();
    let triangles: Vec<[u32; 3]> = mesh
        .indices
        .chunks_exact(3)
        .map(|t| [t[0], t[1], t[2]].map(|i| ids.get(i as usize).copied().unwrap_or(0)))
        .filter(|[a, b, c]| a != b && b != c && a != c)
        .collect();
    physics::prepare(&physics::SolidMesh { points, triangles })
}

/// The physics ID of the chunk at `key` of the ground on `entity`: made
/// from both, never zero.
fn chunk_id(entity: EntityId, key: IVec3) -> Option<EntityId> {
    let mut h = entity.0.raw() ^ 0x9e37_79b9_7f4a_7c15;
    for part in key.to_array() {
        h = (h ^ u64::from(part as u32)).wrapping_mul(0x0100_0000_01b3);
        h ^= h >> 29;
    }
    reflect::Id::from_raw(h.max(1)).map(EntityId)
}

#[cfg(test)]
#[path = "ground_play_tests.rs"]
mod tests;
