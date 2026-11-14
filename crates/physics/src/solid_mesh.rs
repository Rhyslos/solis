//! A collider made of triangles (a brush's, Tools 1a): a still surface
//! the game's characters stand on and bodies bump into, however its faces
//! are shaped.

use glam::Vec3;
use rapier3d::prelude as rp;

/// The most triangles one solid mesh has: what the biggest brush can be
/// cut into. More are refused (no collider), never cut short.
pub const MAX_TRIANGLES: usize = 1 << 18;

/// Triangles over shared points, in the entity's own space before its
/// scale, counter-clockwise seen from outside.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SolidMesh {
    /// The points.
    pub points: Vec<Vec3>,
    /// Three of the points per triangle, by their place in `points`.
    pub triangles: Vec<[u32; 3]>,
}

/// The collider for `mesh` scaled by `scale` (a mirroring scale turns
/// every triangle round, so it still faces out), or `None` when there's
/// nothing usable: no triangles, too many, a point that isn't a number or
/// a triangle naming a point that isn't there. Places are held within
/// `limit` of the entity.
pub(crate) fn collider(mesh: &SolidMesh, scale: Vec3, limit: f32) -> Option<rp::ColliderBuilder> {
    if mesh.triangles.is_empty() || mesh.triangles.len() > MAX_TRIANGLES || !scale.is_finite() {
        return None;
    }
    let count = mesh.points.len();
    if mesh
        .triangles
        .iter()
        .flatten()
        .any(|i| *i as usize >= count)
    {
        return None;
    }
    let points: Vec<Vec3> = mesh
        .points
        .iter()
        .map(|p| (*p * scale).clamp(Vec3::splat(-limit), Vec3::splat(limit)))
        .collect();
    if points.iter().any(|p| !p.is_finite()) {
        return None;
    }
    let mirrored = scale.x * scale.y * scale.z < 0.0;
    let triangles: Vec<[u32; 3]> = mesh
        .triangles
        .iter()
        .map(|&[a, b, c]| if mirrored { [a, c, b] } else { [a, b, c] })
        .collect();
    // Edges inside the surface are smoothed over, so a character walking
    // across two faces of a flat floor never catches on the seam; that
    // needs a well-made surface, so anything else is taken as it is.
    let flags = rp::TriMeshFlags::FIX_INTERNAL_EDGES;
    rp::ColliderBuilder::trimesh_with_flags(points.clone(), triangles.clone(), flags)
        .or_else(|_| rp::ColliderBuilder::trimesh(points, triangles))
        .ok()
}

#[cfg(test)]
#[path = "solid_mesh_tests.rs"]
mod tests;
