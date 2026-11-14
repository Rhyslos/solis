//! A chunk made solid: its triangles as the physics crate's still
//! triangle collider (the one brushes use), and a character walking over
//! the ground and into its hills.

use crate::mesh::ChunkMesh;
use glam::{Quat, Vec3};
use physics::{CharacterSpec, ColliderSpec, GRAVITY, Physics, Shape, SolidMesh};
use reflect::{EntityId, Id};
use std::collections::HashMap;
use std::sync::Arc;

/// `mesh`'s triangles over shared points: points at the same place (a
/// hard edge's two facings, a cube face's corners) become one, so the
/// collider knows which triangles meet and smooths over the seams.
pub fn solid(mesh: &ChunkMesh) -> SolidMesh {
    let mut at: HashMap<[u32; 3], u32> = HashMap::with_capacity(mesh.positions.len());
    let mut points = Vec::new();
    let ids: Vec<u32> = mesh
        .positions
        .iter()
        .map(|p| {
            *at.entry(p.to_array().map(f32::to_bits)).or_insert_with(|| {
                points.push(*p);
                points.len() as u32 - 1
            })
        })
        .collect();
    let triangles = mesh
        .indices
        .chunks_exact(3)
        .map(|t| [ids[t[0] as usize], ids[t[1] as usize], ids[t[2] as usize]])
        .filter(|[a, b, c]| a != b && b != c && a != c)
        .collect();
    SolidMesh { points, triangles }
}

/// An id for physics; the study's own numbers, never zero.
pub fn entity(n: u64) -> Option<EntityId> {
    Id::from_raw(n.max(1)).map(EntityId)
}

/// Adds `mesh` to `physics` as a still collider; returns whether it was
/// made.
pub fn add_chunk(physics: &mut Physics, n: u64, mesh: SolidMesh) -> bool {
    let Some(id) = entity(n) else {
        return false;
    };
    physics.add(
        id,
        (Vec3::ZERO, Quat::IDENTITY),
        Vec3::ONE,
        None,
        Some(ColliderSpec {
            shape: Shape::Mesh(Arc::new(mesh)),
            ..ColliderSpec::default()
        }),
    )
}

/// What a walk found.
#[derive(Debug, Clone, Copy, Default)]
pub struct Walk {
    /// Ticks walked.
    pub ticks: usize,
    /// Ticks standing on the ground.
    pub grounded: usize,
    /// How far it got along the way it walked, metres.
    pub distance: f32,
    /// The largest drop in one tick while it had been standing: a sign of
    /// falling through, or of catching on a seam and dropping off it.
    pub worst_drop: f32,
    /// Its height at the end.
    pub height: f32,
    /// Seconds spent moving it and stepping physics, all ticks.
    pub seconds: f64,
}

/// The character's number in a walk.
const WALKER: u64 = 1_000_000;

/// Drops a character at `start` and walks it along `way` (metres a
/// second) for `ticks` ticks at 60 a second.
pub fn walk(physics: &mut Physics, start: Vec3, way: Vec3, ticks: usize) -> Walk {
    let mut found = Walk::default();
    let Some(id) = entity(WALKER) else {
        return found;
    };
    if !physics.add_character(id, (start, Quat::IDENTITY), CharacterSpec::default()) {
        return found;
    }
    let dt = 1.0 / 60.0;
    let mut fall = 0.0f32;
    let mut last = start;
    let mut was_grounded = false;
    let clock = std::time::Instant::now();
    for _ in 0..ticks {
        let grounded = physics.state().character(id).is_some_and(|c| c.grounded);
        fall = if grounded { 0.0 } else { fall + GRAVITY * dt };
        physics.move_character(id, (way + Vec3::Z * fall) * dt, dt);
        physics.step(dt);
        let _ = physics.moved();
        let feet = physics.state().pose(id).map_or(last, |(p, _)| p);
        if was_grounded {
            found.worst_drop = found.worst_drop.max(last.z - feet.z);
        }
        was_grounded = grounded;
        found.grounded += usize::from(grounded);
        found.ticks += 1;
        last = feet;
    }
    found.seconds = clock.elapsed().as_secs_f64();
    found.distance = (last - start)
        .truncate()
        .dot(way.truncate().normalize_or_zero());
    found.height = last.z;
    physics.remove(id);
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ground::{Ground, Samples};
    use crate::mesh::Method;
    use crate::shapes::{fill, hill_with_cave};
    use crate::store::Full;
    use crate::voxel::{Dense, Voxel};
    use glam::UVec3;

    fn solid_hill(method: Method) -> Physics {
        let ground: Ground<Full> = Ground::build(UVec3::new(3, 3, 2), |key, dense| {
            fill(key, dense, hill_with_cave)
        });
        let mut physics = Physics::new();
        let (mut samples, mut scratch) = (Samples::default(), Dense::filled(Voxel::AIR));
        for (n, key) in ground.mixed_keys().into_iter().enumerate() {
            ground.gather(key, &mut scratch, &mut samples);
            assert!(add_chunk(
                &mut physics,
                n as u64 + 1,
                solid(&method.mesh(&samples))
            ));
        }
        physics
    }

    #[test]
    fn a_character_walks_across_chunk_seams_without_catching() {
        for method in [Method::SurfaceNets, Method::GreedyCubes] {
            let mut physics = solid_hill(method);
            // Dropped from 4 m up, then along the meadow's south edge,
            // over the seam at x = 32.
            let walked = walk(
                &mut physics,
                Vec3::new(14.0, 12.0, 16.0),
                Vec3::X * 4.0,
                300,
            );
            assert!(walked.grounded > 230, "{}: {walked:?}", method.name());
            assert!(walked.distance > 15.0, "{}: {walked:?}", method.name());
            assert!(
                walked.height > 8.0,
                "{}: didn't fall through: {walked:?}",
                method.name()
            );
        }
    }

    #[test]
    fn a_character_is_stopped_by_the_hill() {
        let mut physics = solid_hill(Method::SurfaceNets);
        // West to east into the hill's steep flank at mid height.
        let walked = walk(
            &mut physics,
            Vec3::new(20.0, 54.0, 16.0),
            Vec3::X * 4.0,
            400,
        );
        assert!(walked.distance < 26.0, "{walked:?}");
    }

    #[test]
    fn welding_joins_points_at_one_place() {
        let mesh = ChunkMesh {
            positions: vec![Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::X, Vec3::ONE],
            normals: vec![Vec3::Z; 5],
            materials: vec![0; 5],
            indices: vec![0, 1, 2, 2, 3, 4, 0, 0, 1],
        };
        let solid = solid(&mesh);
        assert_eq!(solid.points.len(), 4);
        assert_eq!(solid.triangles, vec![[0, 1, 2], [2, 1, 3]]);
    }
}
