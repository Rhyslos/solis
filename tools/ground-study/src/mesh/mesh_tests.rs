use super::*;
use crate::ground::{Ground, Samples};
use crate::shapes::{ball, busy, fill, hill_with_cave};
use crate::store::{Full, Octree, Sparse, Store};
use crate::voxel::{Dense, SIZE_I, Voxel};
use glam::{IVec3, UVec3};
use std::collections::HashMap;

fn meshes<S: Store>(ground: &Ground<S>, method: Method) -> Vec<ChunkMesh> {
    let mut samples = Samples::default();
    let mut scratch = Dense::filled(Voxel::AIR);
    ground
        .keys()
        .map(|key| {
            ground.gather(key, &mut scratch, &mut samples);
            method.mesh(&samples)
        })
        .collect()
}

fn ball_ground<S: Store>() -> Ground<S> {
    let shape = ball(glam::Vec3::splat(32.0), 20.0);
    Ground::build(UVec3::splat(2), |key, dense| fill(key, dense, &shape))
}

/// Every chunk's triangles welded by place: each edge, by its two
/// points, with how many triangles go along it each way.
fn edges(meshes: &[ChunkMesh]) -> HashMap<(u32, u32), (u32, u32)> {
    let mut ids: HashMap<[u32; 3], u32> = HashMap::new();
    let mut edges: HashMap<(u32, u32), (u32, u32)> = HashMap::new();
    for mesh in meshes {
        let id: Vec<u32> = mesh
            .positions
            .iter()
            .map(|p| {
                let next = ids.len() as u32;
                *ids.entry(p.to_array().map(f32::to_bits)).or_insert(next)
            })
            .collect();
        for t in mesh.indices.chunks(3) {
            let [a, b, c] = [t[0], t[1], t[2]].map(|i| id[i as usize]);
            if a == b || b == c || a == c {
                continue; // no area: its edges cancel out
            }
            for (p, q) in [(a, b), (b, c), (c, a)] {
                let entry = edges.entry((p.min(q), p.max(q))).or_default();
                if p < q {
                    entry.0 += 1;
                } else {
                    entry.1 += 1;
                }
            }
        }
    }
    edges
}

#[test]
fn a_closed_shape_meshes_closed_across_chunks() {
    let ground: Ground<Full> = ball_ground();
    for method in Method::ALL {
        if method == Method::GreedyCubes {
            continue; // merged rectangles meet others mid-side; see below
        }
        let all = meshes(&ground, method);
        assert!(all.iter().filter(|m| m.triangles() > 0).count() == 8);
        for ((a, b), (forth, back)) in edges(&all) {
            assert_eq!(
                forth,
                back,
                "{}: edge {a}-{b} isn't shared both ways",
                method.name()
            );
            assert!(forth > 0);
        }
    }
}

#[test]
fn merged_cubes_cover_exactly_the_cube_faces() {
    let ground: Ground<Full> = ball_ground();
    let area = |method: Method| {
        let mut by_way: HashMap<([i32; 3], u8), f32> = HashMap::new();
        for mesh in meshes(&ground, method) {
            for t in mesh.indices.chunks(3) {
                let [a, b, c] = [t[0], t[1], t[2]].map(|i| mesh.positions[i as usize]);
                let n = mesh.normals[t[0] as usize].round().as_ivec3().to_array();
                *by_way
                    .entry((n, mesh.materials[t[0] as usize]))
                    .or_default() += (b - a).cross(c - a).length() * 0.5;
            }
        }
        let mut sorted: Vec<_> = by_way.into_iter().collect();
        sorted.sort_by_key(|(k, _)| *k);
        sorted
    };
    let (faces, merged) = (area(Method::CubeFaces), area(Method::GreedyCubes));
    assert_eq!(faces.len(), merged.len());
    for ((way, a), (way2, b)) in faces.iter().zip(&merged) {
        assert_eq!(way, way2);
        assert!((a - b).abs() < 0.01, "{way:?}: {a} against {b}");
    }
    let count = |m: Method| {
        meshes(&ground, m)
            .iter()
            .map(ChunkMesh::triangles)
            .sum::<usize>()
    };
    assert!(count(Method::GreedyCubes) < count(Method::CubeFaces));
}

#[test]
fn triangles_face_out_of_the_ground() {
    let ground: Ground<Full> = ball_ground();
    for method in Method::ALL {
        let (mut out, mut all) = (0, 0);
        for mesh in meshes(&ground, method) {
            for t in mesh.indices.chunks(3) {
                let [a, b, c] = [t[0], t[1], t[2]].map(|i| mesh.positions[i as usize]);
                let middle = (a + b + c) / 3.0 - glam::Vec3::splat(32.0);
                let facing = (b - a).cross(c - a);
                all += 1;
                if facing.dot(middle) > 0.0 {
                    out += 1;
                }
                let given: glam::Vec3 = t.iter().map(|i| mesh.normals[*i as usize]).sum();
                assert!(
                    facing.dot(given) >= 0.0,
                    "{} against its facings",
                    method.name()
                );
                let n = mesh.normals[t[0] as usize];
                assert!(
                    n.is_finite() && (n.length() - 1.0).abs() < 1.0e-3,
                    "{}: facing {n}",
                    method.name()
                );
            }
        }
        // Merged rectangles are too big to judge by their middles.
        if method != Method::GreedyCubes {
            assert!(
                out * 100 >= all * 97,
                "{}: {out} of {all} face out",
                method.name()
            );
        }
    }
}

#[test]
fn an_open_ground_is_open_only_at_its_edges() {
    let ground: Ground<Sparse> = Ground::build(UVec3::new(3, 3, 2), |key, dense| {
        fill(key, dense, hill_with_cave)
    });
    let size = ground.size().as_vec3();
    for method in [
        Method::MarchingCubes,
        Method::SurfaceNets,
        Method::DualContouring,
    ] {
        let all = meshes(&ground, method);
        // Each place by its number in `edges`' welding: first seen first.
        let mut place: Vec<glam::Vec3> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for p in all.iter().flat_map(|m| &m.positions) {
            if seen.insert(p.to_array().map(f32::to_bits)) {
                place.push(*p);
            }
        }
        for ((a, b), (forth, back)) in edges(&all) {
            if forth != back {
                let near_edge = |p: glam::Vec3| {
                    p.x < 1.0 || p.y < 1.0 || p.x > size.x - 2.0 || p.y > size.y - 2.0
                };
                assert!(
                    near_edge(place[a as usize]) && near_edge(place[b as usize]),
                    "{}: a crack inside the ground at {}",
                    method.name(),
                    place[a as usize]
                );
            }
        }
    }
}

#[test]
fn the_same_voxels_give_the_same_mesh_in_every_store() {
    let full: Ground<Full> = ball_ground();
    let octree: Ground<Octree> = ball_ground();
    for method in Method::ALL {
        let a = meshes(&full, method);
        assert_eq!(a, meshes(&full, method), "{} again", method.name());
        assert_eq!(
            a,
            meshes(&octree, method),
            "{} from the octree",
            method.name()
        );
    }
}

#[test]
fn a_busy_chunk_meshes_within_its_cubes() {
    let ground: Ground<Full> = Ground::build(UVec3::ONE, |key, dense| fill(key, dense, busy));
    for method in Method::ALL {
        let mesh = &meshes(&ground, method)[0];
        assert!(mesh.triangles() > 1000, "{}", method.name());
        for p in &mesh.positions {
            assert!(p.is_finite());
            assert!(p.cmpge(glam::Vec3::splat(-1.5)).all());
            assert!(p.cmple(glam::Vec3::splat(SIZE_I as f32 + 0.5)).all());
        }
    }
}

#[test]
fn sharp_looks_keep_a_cut_edge_sharper() {
    // A box of ground with a flat top and straight sides: along its top
    // edge, the sharp looks put their points nearer the edge than plain
    // surface nets, which wear it round. (With facings worked out from
    // the voxels, not even dual contouring lands on it exactly: the
    // voxels beside an edge face half one way and half the other.)
    let block = |p: glam::Vec3| {
        let q = (p - glam::Vec3::new(16.3, 16.3, 10.3)).abs() - glam::Vec3::new(6.0, 6.0, 6.0);
        Voxel::at(
            q.max_element().min(0.0) + q.max(glam::Vec3::ZERO).length(),
            1,
        )
    };
    let ground: Ground<Full> = Ground::build(UVec3::ONE, |key, dense| fill(key, dense, block));
    let nearest = |method: Method| {
        meshes(&ground, method)[0]
            .positions
            .iter()
            .filter(|p| (p.y - 16.3).abs() < 3.0)
            .map(|p| glam::Vec2::new(p.x - 22.3, p.z - 16.3).length())
            .fold(f32::MAX, f32::min)
    };
    let (dc, nets, sharp) = (
        nearest(Method::DualContouring),
        nearest(Method::SurfaceNets),
        nearest(Method::SharpNets),
    );
    assert!(dc < 0.2 && sharp < 0.2, "{dc} {sharp}");
    assert!(dc < nets && sharp < nets, "{dc} {sharp} {nets}");
}

#[test]
fn the_ground_is_as_wide_as_its_chunks() {
    let ground: Ground<Full> = ball_ground();
    assert_eq!(ground.size(), IVec3::splat(64));
}
