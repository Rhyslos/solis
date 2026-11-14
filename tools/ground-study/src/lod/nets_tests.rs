use super::*;
use crate::ground::Ground;
use crate::lod::cracks::{Scene, open_edges};
use crate::lod::layout::Layout;
use crate::lod::tests::ball_pyramid;
use crate::lod::{Pyramid, blocky};
use crate::mesh::Method;
use crate::store::Full;
use glam::IVec2;

/// A layout of the ball's ground with all three levels: level 0 in the
/// corner square of 64 m, level 1 round it to 128 m, level 2 beyond.
fn mixed() -> Layout {
    let (layout, _) = Layout::octree(IVec2::splat(8), Vec3::new(10.0, 10.0, 20.0), 2, 0.6);
    let levels: std::collections::HashSet<u32> = layout.columns().iter().map(|c| c.level).collect();
    assert_eq!(levels.len(), 3, "{:?}", layout.columns());
    layout
}

/// Every chunk of every column, meshed.
fn all(p: &Pyramid, layout: &Layout, look: Look, seams: Seams) -> Vec<ChunkMesh> {
    let mut out = Vec::new();
    for c in layout.columns() {
        let high = p.size(c.level).z / SIZE_I;
        for z in 0..high {
            let made = mesh(p, layout, c.level, c.key.extend(z), look, seams);
            out.push(made.mesh);
            out.push(made.border);
        }
    }
    out
}

fn blocky_all(p: &Pyramid, layout: &Layout, seams: Seams) -> Vec<ChunkMesh> {
    let mut out = Vec::new();
    for c in layout.columns() {
        let high = p.size(c.level).z / SIZE_I;
        for z in 0..high {
            out.push(blocky::mesh(p, layout, c.level, c.key.extend(z), seams));
        }
    }
    out
}

/// Places just inside the cracks between levels: beside each open edge
/// of the ground meshed with nothing done (but at the world's edges), a
/// third of a voxel out from its triangle.
fn crack_targets(p: &Pyramid, layout: &Layout) -> Vec<(Vec3, Vec3)> {
    let bare = all(p, layout, Look::Smooth, Seams::None);
    let mut out = Vec::new();
    for mesh in &bare {
        for (a, b) in crate::lod::skirts::open_edges(mesh) {
            let (pa, pb) = (mesh.positions[a as usize], mesh.positions[b as usize]);
            let middle = (pa + pb) / 2.0;
            if middle.x < 1.0 || middle.y < 1.0 || middle.x > 250.0 || middle.y > 250.0 {
                continue;
            }
            let along = (pb - pa).normalize_or_zero();
            let up = mesh.normals[a as usize];
            let out_of = along.cross(up).normalize_or_zero();
            out.push((middle + out_of * 0.3 * (pb - pa).length().max(0.5), up));
        }
    }
    out
}

/// Rays at each target from 8 m out along the ground's facing there and
/// 40 m to each of four sides, grazing the seam as a far one is seen.
fn rays_at(
    p: &Pyramid,
    layout: &Layout,
    meshes: &[ChunkMesh],
    targets: &[(Vec3, Vec3)],
) -> crate::lod::cracks::Seen {
    let refs: Vec<&ChunkMesh> = meshes.iter().collect();
    let scene = Scene::new(&refs, 4.0);
    let mut seen = crate::lod::cracks::Seen::default();
    for (t, up) in targets {
        let across = up.any_orthonormal_vector();
        let along = up.cross(across);
        for side in [across, -across, along, -along] {
            let eye = *t + *up * 8.0 + side * 40.0;
            // Clear of the ground at every level, so a ray never starts
            // behind a coarser level's bulge.
            let clear = (0..p.count()).all(|l| {
                p.voxel(l, (eye / crate::lod::voxel(l)).round().as_ivec3())
                    .distance()
                    >= 1.0
            });
            if !clear {
                continue;
            }
            let s = scene.look(
                eye,
                &[*t],
                |q| crate::lod::cracks::deep(p, layout, q),
                |_| 0.25,
            );
            seen.rays += s.rays;
            seen.through += s.through;
        }
    }
    seen
}

#[test]
fn at_one_level_it_is_the_ground_studys_surface_nets() {
    let p = ball_pyramid(1);
    let layout = Layout::uniform(IVec2::splat(8), 0);
    let mine: Vec<ChunkMesh> = all(&p, &layout, Look::Smooth, Seams::None);
    let Some(base) = p.level(0) else {
        panic!("no level 0")
    };
    let mut samples = crate::ground::Samples::default();
    let mut scratch = crate::voxel::Dense::filled(crate::voxel::Voxel::AIR);
    let theirs: Vec<ChunkMesh> = base
        .keys()
        .map(|key| {
            base.gather(key, &mut scratch, &mut samples);
            Method::SurfaceNets.mesh(&samples)
        })
        .collect();
    let count = |m: &[ChunkMesh]| m.iter().map(ChunkMesh::triangles).sum::<usize>();
    assert_eq!(count(&mine), count(&theirs));
    let points = |m: &[ChunkMesh]| {
        let mut all: Vec<[u32; 3]> = m
            .iter()
            .flat_map(|m| m.positions.iter().map(|p| p.to_array().map(f32::to_bits)))
            .collect();
        all.sort_unstable();
        all.dedup();
        all
    };
    assert_eq!(points(&mine), points(&theirs));
}

#[test]
fn nothing_done_leaves_cracks_between_levels() {
    let p = ball_pyramid(3);
    let layout = mixed();
    let meshes = all(&p, &layout, Look::Smooth, Seams::None);
    let refs: Vec<&ChunkMesh> = meshes.iter().collect();
    assert!(open_edges(&refs, |_| false) > 0);
}

#[test]
fn transition_bands_and_stitches_close_every_seam_in_both_looks() {
    let p = ball_pyramid(3);
    let layout = mixed();
    for look in [Look::Smooth, Look::Sharp] {
        let mut triangles = Vec::new();
        for seams in [Seams::Transition, Seams::Stitched] {
            let meshes = all(&p, &layout, look, seams);
            let refs: Vec<&ChunkMesh> = meshes.iter().collect();
            assert_eq!(open_edges(&refs, |_| false), 0, "{look:?} {seams:?}");
            let seen = rays_at(&p, &layout, &meshes, &crack_targets(&p, &layout));
            assert!(seen.rays > 200, "{seen:?}");
            assert_eq!(seen.through, 0, "{look:?} {seams:?}: {seen:?}");
            triangles.push(meshes.iter().map(ChunkMesh::triangles).sum::<usize>());
        }
        assert_eq!(triangles[0], triangles[1], "the same joins, kept apart");
    }
}

#[test]
fn skirts_hide_what_nothing_shows() {
    let p = ball_pyramid(3);
    let layout = mixed();
    let targets = crack_targets(&p, &layout);
    let bare = rays_at(
        &p,
        &layout,
        &all(&p, &layout, Look::Smooth, Seams::None),
        &targets,
    );
    let skirted = rays_at(
        &p,
        &layout,
        &all(&p, &layout, Look::Smooth, Seams::Skirts),
        &targets,
    );
    assert!(bare.through > 0, "{bare:?}");
    assert!(
        skirted.through * 4 < bare.through,
        "{skirted:?} against {bare:?}"
    );
}

#[test]
fn blocky_walls_between_sizes_leave_nothing_to_see_through() {
    let p = ball_pyramid(3);
    let layout = mixed();
    let targets = crack_targets(&p, &layout);
    let bare = rays_at(&p, &layout, &blocky_all(&p, &layout, Seams::None), &targets);
    let walls = rays_at(
        &p,
        &layout,
        &blocky_all(&p, &layout, Seams::Transition),
        &targets,
    );
    assert!(bare.through > 0, "{bare:?}");
    assert_eq!(walls.through, 0, "{walls:?}");
}

#[test]
fn the_same_voxels_give_the_same_mesh() {
    let p = ball_pyramid(3);
    let layout = mixed();
    for seams in Seams::ALL {
        for c in layout.columns() {
            let a = mesh(&p, &layout, c.level, c.key.extend(0), Look::Sharp, seams);
            let b = mesh(&p, &layout, c.level, c.key.extend(0), Look::Sharp, seams);
            assert_eq!(a.mesh, b.mesh);
            assert_eq!(a.border, b.border);
        }
    }
}

#[test]
fn a_stitched_chunks_own_mesh_ignores_its_neighbours_levels() {
    let p = ball_pyramid(3);
    let layout = mixed();
    let flat = Layout::uniform(IVec2::splat(8), 0);
    for c in layout.columns().iter().filter(|c| c.level == 0) {
        for z in 0..4 {
            let key = c.key.extend(z);
            let here = mesh(&p, &layout, 0, key, Look::Smooth, Seams::Stitched);
            let alone = mesh(&p, &flat, 0, key, Look::Smooth, Seams::Stitched);
            assert_eq!(here.mesh, alone.mesh, "{key}");
        }
    }
}

#[test]
fn the_world_edge_stays_open_and_nothing_else_does() {
    // A slab filling the box from side to side: its surface meets the
    // world's edges, where the ground just stops.
    let base: Ground<Full> = Ground::build(glam::UVec3::new(8, 8, 4), |key, dense| {
        crate::shapes::fill(key, dense, |q| {
            crate::voxel::Voxel::at(q.z - 40.0 - 6.0 * (q.x / 17.0).sin(), 0)
        });
    });
    let p = Pyramid::build(base, 3);
    let layout = mixed();
    let meshes = all(&p, &layout, Look::Smooth, Seams::Transition);
    let refs: Vec<&ChunkMesh> = meshes.iter().collect();
    let edge = |q: Vec3| q.x <= 0.5 || q.y <= 0.5 || q.x >= 253.0 || q.y >= 253.0;
    assert_eq!(open_edges(&refs, edge), 0);
}

#[test]
fn a_border_alone_is_the_stitched_chunks_border() {
    let p = ball_pyramid(3);
    let layout = mixed();
    for c in layout.columns().iter().take(6) {
        let key = c.key.extend(1);
        let whole = mesh(&p, &layout, c.level, key, Look::Sharp, Seams::Stitched);
        assert_eq!(
            border(&p, &layout, c.level, key, Look::Sharp),
            whole.border,
            "{key}"
        );
    }
}
