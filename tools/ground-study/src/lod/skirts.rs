//! Skirts: a strip hung from each open edge along a chunk's sides, down
//! into the ground, so that where a neighbour of another level leaves a
//! crack, the strip fills it from either side. A chunk's mesh then never
//! depends on its neighbours.

use crate::mesh::ChunkMesh;
use glam::{Vec2, Vec3};
use std::collections::HashMap;

/// The open edges of `mesh` (used by one triangle only), each as its
/// two points in the order a triangle goes along it.
pub fn open_edges(mesh: &ChunkMesh) -> Vec<(u32, u32)> {
    let mut ids: HashMap<[u32; 3], u32> = HashMap::new();
    let id: Vec<u32> = mesh
        .positions
        .iter()
        .enumerate()
        .map(|(i, p)| {
            *ids.entry(p.to_array().map(f32::to_bits))
                .or_insert(i as u32)
        })
        .collect();
    let mut along: HashMap<(u32, u32), (u32, (u32, u32))> = HashMap::new();
    for t in mesh.indices.chunks_exact(3) {
        for (p, q) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            let (a, b) = (id[p as usize], id[q as usize]);
            if a == b {
                continue;
            }
            let entry = along.entry((a.min(b), a.max(b))).or_insert((0, (p, q)));
            entry.0 += 1;
        }
    }
    let mut out: Vec<(u32, u32)> = along
        .into_values()
        .filter(|(n, _)| *n == 1)
        .map(|(_, edge)| edge)
        .collect();
    out.sort_unstable();
    out
}

/// Hangs a skirt `depth` metres deep (along each point's facing, into
/// the ground) from every open edge of `mesh` within a voxel of the
/// sides of its square `lo..hi` (metres).
pub fn hang(mesh: &mut ChunkMesh, lo: Vec2, hi: Vec2, depth: f32) {
    let near = 0.51 * depth;
    let on_side = |p: Vec3| {
        let q = p.truncate();
        (q.x - lo.x).abs() <= near
            || (q.x - hi.x).abs() <= near
            || (q.y - lo.y).abs() <= near
            || (q.y - hi.y).abs() <= near
    };
    for (a, b) in open_edges(mesh) {
        let (pa, pb) = (mesh.positions[a as usize], mesh.positions[b as usize]);
        if !on_side(pa) || !on_side(pb) {
            continue;
        }
        let (na, nb) = (mesh.normals[a as usize], mesh.normals[b as usize]);
        let (ka, kb) = (mesh.materials[a as usize], mesh.materials[b as usize]);
        let top_b = mesh.push_point(pb, nb, kb);
        let top_a = mesh.push_point(pa, na, ka);
        let low_a = mesh.push_point(pa - na * depth, na, ka);
        let low_b = mesh.push_point(pb - nb * depth, nb, kb);
        mesh.indices
            .extend_from_slice(&[top_b, top_a, low_a, top_b, low_a, low_b]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square() -> ChunkMesh {
        ChunkMesh {
            positions: vec![Vec3::ZERO, Vec3::X, Vec3::new(1.0, 1.0, 0.0), Vec3::Y],
            normals: vec![Vec3::Z; 4],
            materials: vec![0; 4],
            indices: vec![0, 1, 2, 0, 2, 3],
        }
    }

    #[test]
    fn a_lone_square_has_four_open_edges_and_hangs_four_strips() {
        let mut mesh = square();
        assert_eq!(open_edges(&mesh).len(), 4);
        hang(&mut mesh, Vec2::ZERO, Vec2::ONE, 2.0);
        assert_eq!(mesh.triangles(), 2 + 8);
        let lowest = mesh.positions.iter().map(|p| p.z).fold(0.0, f32::min);
        assert_eq!(lowest, -2.0);
        // Each strip faces out of the square, away from its middle.
        for t in mesh.indices.chunks_exact(3).skip(2) {
            let [a, b, c] = [t[0], t[1], t[2]].map(|i| mesh.positions[i as usize]);
            let facing = (b - a).cross(c - a);
            let middle = (a + b + c) / 3.0 - Vec3::new(0.5, 0.5, 0.0);
            assert!(facing.dot(middle.with_z(0.0)) > 0.0);
        }
    }

    #[test]
    fn edges_away_from_the_sides_get_no_skirt() {
        let mut mesh = square();
        hang(&mut mesh, Vec2::splat(-10.0), Vec2::splat(10.0), 2.0);
        assert_eq!(mesh.triangles(), 2);
    }
}
