//! Hard edges: each corner of a triangle faces the average of the
//! triangles round its point that face nearly its own way, so a gentle
//! slope stays smooth while a cliff's lip gets a crisp edge. A point
//! whose triangles fall into several such groups is split in as many.

use super::ChunkMesh;
use glam::Vec3;

/// `mesh` with its facings worked out again from its triangles, a hard
/// edge wherever two meet at more than `degrees`.
pub(crate) fn sharpen(mesh: &ChunkMesh, degrees: f32) -> ChunkMesh {
    let limit = degrees.to_radians().cos();
    let triangles = mesh.indices.len() / 3;
    let corners = |t: usize| {
        let i = &mesh.indices[3 * t..3 * t + 3];
        [i[0] as usize, i[1] as usize, i[2] as usize]
    };
    // Each triangle's facing, as long as twice its area.
    let facing: Vec<Vec3> = (0..triangles)
        .map(|t| {
            let [a, b, c] = corners(t).map(|i| mesh.positions[i]);
            (b - a).cross(c - a)
        })
        .collect();
    let unit: Vec<Vec3> = facing.iter().map(|f| f.normalize_or_zero()).collect();
    // The triangles round each point, in order.
    let points = mesh.positions.len();
    let mut start = vec![0u32; points + 1];
    for &i in &mesh.indices {
        start[i as usize + 1] += 1;
    }
    for p in 0..points {
        start[p + 1] += start[p];
    }
    let mut fill = start.clone();
    let mut round = vec![0u32; mesh.indices.len()];
    for (k, &i) in mesh.indices.iter().enumerate() {
        round[fill[i as usize] as usize] = (k / 3) as u32;
        fill[i as usize] += 1;
    }
    let mut out = ChunkMesh::default();
    let mut made: Vec<Vec<(Vec3, u32)>> = vec![Vec::new(); points];
    for t in 0..triangles {
        for p in corners(t) {
            let mut normal = Vec3::ZERO;
            for &u in &round[start[p] as usize..start[p + 1] as usize] {
                if unit[u as usize].dot(unit[t]) >= limit {
                    normal += facing[u as usize];
                }
            }
            let normal = normal.normalize_or(mesh.normals[p]);
            let index = match made[p].iter().find(|(n, _)| *n == normal) {
                Some((_, index)) => *index,
                None => {
                    let index = out.push_point(mesh.positions[p], normal, mesh.materials[p]);
                    made[p].push((normal, index));
                    index
                }
            };
            out.indices.push(index);
        }
    }
    out
}
