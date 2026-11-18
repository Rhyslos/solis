use super::*;
use crate::ground::{Answer, Ground};
use crate::start::Start;
use crate::store::{Chunk, keys};
use crate::voxel::{STEPS, VOLUME, place};
use glam::UVec3;
use std::collections::HashMap;

/// A ground of `extent` chunks holding a ball of rock of `radius` metres
/// at `centre`, so its surface is closed and crosses chunk seams.
fn ball(extent: UVec3, centre: Vec3, radius: f32) -> Ground {
    Ground::make(extent, |key| {
        let first = origin(key);
        let voxels: Vec<Voxel> = (0..VOLUME)
            .map(|i| {
                let p = (first + place(i).as_ivec3()).as_vec3();
                let d = p.distance(centre) - radius;
                Voxel::at_steps((d * STEPS as f32).round() as i32, 1)
            })
            .collect();
        Chunk::keep(&voxels)
    })
}

fn meshes(ground: &Ground) -> Vec<ChunkMesh> {
    let mut samples = Samples::default();
    let mut mesher = Mesher::default();
    keys(ground.extent())
        .map(|key| {
            assert_eq!(ground.gather(key, &mut samples), Answer::Known(()));
            mesher.mesh(&samples)
        })
        .collect()
}

/// Edges used by only one triangle once the chunks' points are joined
/// by their places in the world: a crack, or the surface's open end.
fn open_edges(meshes: &[ChunkMesh]) -> usize {
    let mut ids: HashMap<[u32; 3], u32> = HashMap::new();
    let mut edges: HashMap<(u32, u32), i32> = HashMap::new();
    for m in meshes {
        let id: Vec<u32> = (0..m.positions.len() as u32)
            .map(|i| {
                let w = m.world(i).to_array().map(f32::to_bits);
                let next = ids.len() as u32;
                *ids.entry(w).or_insert(next)
            })
            .collect();
        for t in m.indices.chunks_exact(3) {
            for k in 0..3 {
                let (a, b) = (id[t[k] as usize], id[t[(k + 1) % 3] as usize]);
                *edges.entry((a.min(b), a.max(b))).or_default() += if a < b { 1 } else { -1 };
            }
        }
    }
    edges.values().filter(|n| **n != 0).count()
}

#[test]
fn a_chunk_meshed_twice_gives_identical_bytes() {
    let g = Start::Gentle {
        height: 40.0,
        seed: 4,
    }
    .ground(UVec3::new(2, 2, 2));
    let mut s = Samples::default();
    assert_eq!(g.gather(IVec3::new(1, 0, 1), &mut s), Answer::Known(()));
    let once = mesh(&s);
    assert!(!once.is_empty());
    let mut reused = Mesher::default();
    let _ = reused.mesh(&Samples::default());
    assert_eq!(once.to_bytes(), reused.mesh(&s).to_bytes());
    assert_eq!(once.to_bytes(), mesh(&s.clone()).to_bytes());
}

#[test]
fn a_ball_across_eight_chunks_has_no_cracks() {
    let g = ball(UVec3::new(2, 2, 2), Vec3::splat(32.3), 20.0);
    let all = meshes(&g);
    assert!(all.iter().filter(|m| !m.is_empty()).count() == 8);
    assert_eq!(open_edges(&all), 0);
}

#[test]
fn neighbours_share_their_seam_points_exactly_with_the_same_facing() {
    let g = Start::Gentle {
        height: 40.0,
        seed: 11,
    }
    .ground(UVec3::new(2, 1, 2));
    let all = meshes(&g);
    let seam = |m: &ChunkMesh| -> HashMap<[u32; 3], [u32; 3]> {
        (0..m.positions.len() as u32)
            .filter(|i| (m.world(*i).x - 32.0).abs() < 1.0)
            .map(|i| {
                (
                    m.world(i).to_array().map(f32::to_bits),
                    m.normals[i as usize].map(f32::to_bits),
                )
            })
            .collect()
    };
    let (left, right) = (&all[2], &all[3]);
    assert_eq!(left.key, IVec3::new(0, 0, 1));
    let (l, r) = (seam(left), seam(right));
    let shared: Vec<_> = l.keys().filter(|k| r.contains_key(*k)).collect();
    assert!(shared.len() > 20, "{} shared", shared.len());
    for k in shared {
        assert_eq!(l[k], r[k], "the same facing either side");
    }
}

#[test]
fn every_triangle_faces_out_of_the_ball() {
    let centre = Vec3::splat(32.3);
    let g = ball(UVec3::new(2, 2, 2), centre, 20.0);
    for m in meshes(&g) {
        for t in m.indices.chunks_exact(3) {
            let [a, b, c] = [t[0], t[1], t[2]].map(|i| m.world(i));
            let facing = (b - a).cross(c - a);
            assert!(facing.dot((a + b + c) / 3.0 - centre) > 0.0);
        }
        for n in &m.normals {
            assert!((Vec3::from_array(*n).length() - 1.0).abs() < 1e-4);
        }
    }
}

#[test]
fn a_bare_chunk_makes_nothing() {
    let g = Start::Flat { height: 40.0 }.ground(UVec3::new(1, 1, 4));
    let mut s = Samples::default();
    assert_eq!(g.gather(IVec3::new(0, 0, 3), &mut s), Answer::Known(()));
    assert!(mesh(&s).is_empty());
    assert_eq!(g.gather(IVec3::new(0, 0, 1), &mut s), Answer::Known(()));
    let m = mesh(&s);
    assert_eq!(m.triangles(), 2 * 32 * 32);
    assert!(m.positions.iter().all(|p| (p[2] - 8.0).abs() < 1e-3));
    assert!(m.kinds.iter().all(|k| *k == 0));
}
