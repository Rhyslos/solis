//! A 20 km view: the camera in the middle of the 20 × 20 km world, the
//! ground drawn to its edges each way compared: what it costs to draw,
//! to make, whether seams show, and what moving the camera costs.

use super::super::stats::{Table, count, mb, ms, timed};
use super::{Looks, made_for, mesh_all};
use glam::{IVec2, Vec2, Vec3};
use ground_study::far::{self, top};
use ground_study::lod::cracks::{Scene, deep};
use ground_study::lod::layout::{Column, Layout, new_columns};
use ground_study::lod::nets::Seams;
use ground_study::lod::{Kept, voxel};
use ground_study::mesh::ChunkMesh;
use ground_study::voxel::SIZE_I;
use ground_study::world;
use rayon::prelude::*;

/// The world's side in level-0 columns, and its top level (a column of
/// 4 km).
pub(crate) const ACROSS: i32 = world::SIDE / SIZE_I;
pub(crate) const TOP: u32 = 7;

/// The middle of the world, 60 m over the ground.
pub(crate) fn camera() -> Vec3 {
    let flat = Vec2::splat(world::SIDE as f32 / 2.0) + Vec2::new(0.0, -1000.0);
    flat.extend(world::height(flat) + 60.0)
}

/// A way of drawing the 20 km.
#[derive(Clone, Copy)]
pub(crate) struct Way {
    pub(crate) name: &'static str,
    pub(crate) layout: Arrange,
    pub(crate) seams: Seams,
    /// From this level up, height-map tiles instead of voxels.
    pub(crate) far_from: Option<u32>,
}

/// How the columns are chosen.
#[derive(Clone, Copy)]
pub(crate) enum Arrange {
    Rings(i32),
    Octree(f32),
}

impl Way {
    pub(crate) fn layout(self, camera: Vec3) -> Layout {
        let across = IVec2::splat(ACROSS);
        match self.layout {
            Arrange::Rings(width) => Layout::rings(across, camera.truncate(), TOP, width),
            Arrange::Octree(split) => Layout::octree(across, camera, TOP, split).0,
        }
    }

    fn is_tile(self, c: &Column) -> bool {
        self.far_from.is_some_and(|f| c.level >= f)
    }
}

/// The ways compared.
pub(crate) const WAYS: [Way; 7] = [
    Way {
        name: "Rings of 8, coarser voxels to the edge, skirts",
        layout: Arrange::Rings(8),
        seams: Seams::Skirts,
        far_from: None,
    },
    Way {
        name: "Rings of 8, coarser voxels to the edge, transition band",
        layout: Arrange::Rings(8),
        seams: Seams::Transition,
        far_from: None,
    },
    Way {
        name: "Rings of 16, coarser voxels to the edge, transition band",
        layout: Arrange::Rings(16),
        seams: Seams::Transition,
        far_from: None,
    },
    Way {
        name: "Octree, split within 3 widths, transition band",
        layout: Arrange::Octree(3.0),
        seams: Seams::Transition,
        far_from: None,
    },
    Way {
        name: "Octree, split within 6 widths, transition band",
        layout: Arrange::Octree(6.0),
        seams: Seams::Transition,
        far_from: None,
    },
    Way {
        name: "Rings of 8, voxels to level 3 (2 km), far field beyond",
        layout: Arrange::Rings(8),
        seams: Seams::Transition,
        far_from: Some(4),
    },
    Way {
        name: "Rings of 8, voxels to level 2 (1 km), far field beyond",
        layout: Arrange::Rings(8),
        seams: Seams::Transition,
        far_from: Some(3),
    },
];

/// What a way drew.
pub(crate) struct Drawn {
    pub(crate) meshes: Vec<ChunkMesh>,
    pub(crate) one_core_ms: f64,
    pub(crate) wall_ms: f64,
    pub(crate) columns: usize,
    pub(crate) kept: Kept,
    pub(crate) layout: Layout,
}

/// Draws the 20 km `way`'s way in `look`.
pub(crate) fn draw(way: Way, look: Looks, camera: Vec3) -> Drawn {
    let layout = way.layout(camera);
    let columns = layout.columns();
    let kept = made_for(&columns, world::SIDE, TOP + 1);
    let (voxel_columns, tile_columns): (Vec<Column>, Vec<Column>) =
        columns.iter().partition(|c| !way.is_tile(c));
    let (wall_ms, (meshed, tiles)) = timed(|| {
        // Voxel chunks next to the far field mesh up to it as if it were
        // their own level; the far field's skirts close the seam.
        let voxel_layout = way
            .far_from
            .map_or_else(|| layout.clone(), |f| layout.capped(f - 1));
        let meshed = mesh_all(&kept, &voxel_layout, &voxel_columns, look, way.seams);
        let tiles: Vec<(ChunkMesh, f64)> = tile_columns
            .par_iter()
            .map(|c| {
                let (t, mesh) = timed(|| far::tile(&kept, c.level, c.key, true));
                (mesh, t)
            })
            .collect();
        (meshed, tiles)
    });
    let one_core_ms =
        meshed.iter().map(|m| m.ms).sum::<f64>() + tiles.iter().map(|(_, t)| *t).sum::<f64>();
    let mut meshes = Vec::new();
    for m in meshed {
        meshes.push(m.mesh);
        if m.border.triangles() > 0 {
            meshes.push(m.border);
        }
    }
    meshes.extend(tiles.into_iter().map(|(m, _)| m));
    Drawn {
        meshes,
        one_core_ms,
        wall_ms,
        columns: columns.len(),
        kept,
        layout,
    }
}

/// Triangles whose middles are within 45° of looking north (+y) from
/// `camera`: a 90° wide view.
fn in_view(meshes: &[ChunkMesh], camera: Vec3) -> usize {
    meshes
        .iter()
        .map(|m| {
            m.indices
                .chunks_exact(3)
                .filter(|t| {
                    let middle = (m.positions[t[0] as usize]
                        + m.positions[t[1] as usize]
                        + m.positions[t[2] as usize])
                        / 3.0;
                    let d = (middle - camera).truncate();
                    d.y > 0.0 && d.x.abs() <= d.y
                })
                .count()
        })
        .sum()
}

/// Places on the borders between columns of different levels within
/// `reach` metres of the camera, on the finer side's ground.
fn seam_targets(layout: &Layout, kept: &Kept, camera: Vec3, reach: f32) -> Vec<Vec3> {
    let mut out = Vec::new();
    for y in 0..ACROSS {
        for x in 0..ACROSS {
            let here = IVec2::new(x, y);
            let Some(a) = layout.level_of(here) else {
                continue;
            };
            for d in [IVec2::X, IVec2::Y] {
                let Some(b) = layout.level_of(here + d) else {
                    continue;
                };
                if a == b {
                    continue;
                }
                let finer = a.min(b);
                for along in [0.25, 0.75] {
                    let edge = (here + d).as_vec2() * SIZE_I as f32;
                    let flat = if d == IVec2::X {
                        Vec2::new(edge.x, (y as f32 + along) * SIZE_I as f32)
                    } else {
                        Vec2::new((x as f32 + along) * SIZE_I as f32, edge.y)
                    };
                    if flat.distance(camera.truncate()) > reach {
                        continue;
                    }
                    let g = (flat / voxel(finer)).round().as_ivec2();
                    let (h, _) = top(kept, finer, g);
                    out.push(flat.extend(h));
                }
            }
        }
    }
    out
}

/// The 20 km view drawn each way (Smooth), then each look for two ways.
pub(crate) fn views() {
    let camera = camera();
    println!(
        "\nThe 20 km view: the camera at {:.0} m, {:.0} m, {:.0} m up (60 m over the ground), the world 20,480 m square.",
        camera.x, camera.y, camera.z
    );
    let table = Table::start(
        "The 20 km view, Smooth: each way of drawing far ground coarser",
        &[
            "Way",
            "Columns",
            "Triangles, all round",
            "In a 90° view",
            "Graphics memory",
            "Made on one core (ms)",
            "On both cores (ms)",
            "Voxels kept",
            "Seam rays seen through (within 3 km)",
        ],
    );
    let l0 = draw(WAYS[0], Looks::Smooth, camera);
    let level_0: Vec<&ChunkMesh> = l0
        .meshes
        .iter()
        .filter(|m| {
            m.positions
                .first()
                .is_some_and(|p| l0.layout.level_at(p.truncate()) == 0)
        })
        .collect();
    let l0_triangles: usize = level_0.iter().map(|m| m.triangles()).sum();
    let l0_bytes: usize = level_0.iter().map(|m| m.upload_bytes()).sum();
    let l0_area = l0.layout.columns().iter().filter(|c| c.level == 0).count() as f64
        * f64::from(SIZE_I * SIZE_I)
        / 1.0e6;
    let every = l0_triangles as f64 / l0_area.max(1e-9) * 400.0;
    let every_bytes = l0_bytes as f64 / l0_area.max(1e-9) * 400.0;
    table.row(&[
        "Every chunk at level 0 (from level 0's triangles round the camera)".into(),
        "409,600".into(),
        format!("about {}", count(every as usize)),
        format!("about {}", count((every / 4.0) as usize)),
        format!("about {:.0} GB", every_bytes / 1.0e9),
        "-".into(),
        "-".into(),
        "about 47 GB".into(),
        "-".into(),
    ]);
    drop(l0);
    for way in WAYS {
        let drawn = draw(way, Looks::Smooth, camera);
        let triangles: usize = drawn.meshes.iter().map(ChunkMesh::triangles).sum();
        let bytes: usize = drawn.meshes.iter().map(ChunkMesh::upload_bytes).sum();
        let refs: Vec<&ChunkMesh> = drawn.meshes.iter().collect();
        let scene = Scene::new(&refs, 16.0);
        let targets = seam_targets(&drawn.layout, &drawn.kept, camera, 3000.0);
        let seen = scene.look(
            camera,
            &targets,
            |p| deep(&drawn.kept, &drawn.layout, p),
            |p| 0.25 * voxel(drawn.layout.level_at(p.truncate())),
        );
        table.row(&[
            way.name.into(),
            count(drawn.columns),
            count(triangles),
            count(in_view(&drawn.meshes, camera)),
            mb(bytes as f64),
            ms(drawn.one_core_ms),
            ms(drawn.wall_ms),
            mb(drawn.kept.bytes() as f64),
            format!("{} of {}", seen.through, seen.rays),
        ]);
    }
    looks(camera);
}

/// Each look, drawn two ways.
fn looks(camera: Vec3) {
    let table = Table::start(
        "The 20 km view in each look",
        &[
            "Look",
            "Way",
            "Triangles, all round",
            "Graphics memory",
            "Made on one core (ms)",
        ],
    );
    for look in Looks::ALL {
        for way in [WAYS[1], WAYS[5]] {
            let drawn = draw(way, look, camera);
            table.row(&[
                look.name().into(),
                way.name.into(),
                count(drawn.meshes.iter().map(ChunkMesh::triangles).sum()),
                mb(drawn
                    .meshes
                    .iter()
                    .map(ChunkMesh::upload_bytes)
                    .sum::<usize>() as f64),
                ms(drawn.one_core_ms),
            ]);
        }
    }
}

/// The camera flying 2 km east in 32 m steps: what must be made each
/// kilometre, each way (Smooth, skirts so nothing else is made again).
pub(crate) fn moving() {
    let table = Table::start(
        "Flying 2 km east: what is made each kilometre (Smooth)",
        &[
            "Way",
            "Columns new a km",
            "Chunks and tiles made a km",
            "One core's time a km (ms)",
            "One worker keeps up to (m/s)",
        ],
    );
    let ways = [
        (WAYS[0], "Rings of 8, coarser voxels to the edge"),
        (
            Way {
                layout: Arrange::Rings(16),
                seams: Seams::Skirts,
                ..WAYS[2]
            },
            "Rings of 16, coarser voxels to the edge",
        ),
        (
            Way {
                seams: Seams::Skirts,
                ..WAYS[3]
            },
            "Octree, split within 3 widths",
        ),
        (
            Way {
                seams: Seams::Skirts,
                ..WAYS[5]
            },
            "Rings of 8, voxels to 2 km, far field beyond",
        ),
    ];
    for (way, name) in ways {
        let mut position = camera();
        let mut before = way.layout(position);
        let mut columns_new = 0;
        let mut made = 0;
        let mut time = 0.0;
        let steps = 2000 / 32;
        for _ in 0..steps {
            position.x += 32.0;
            let after = way.layout(position);
            let new = new_columns(&before, &after);
            columns_new += new.len();
            let kept = made_for(&new, world::SIDE, TOP + 1);
            let (voxels, tiles): (Vec<Column>, Vec<Column>) =
                new.iter().partition(|c| !way.is_tile(c));
            for m in mesh_all(&kept, &after, &voxels, Looks::Smooth, way.seams) {
                made += 1;
                time += m.ms;
            }
            for c in tiles {
                let (t, _) = timed(|| far::tile(&kept, c.level, c.key, true));
                made += 1;
                time += t;
            }
            before = after;
        }
        let km = steps as f64 * 32.0 / 1000.0;
        let per_km = time / km;
        table.row(&[
            name.into(),
            format!("{:.0}", columns_new as f64 / km),
            format!("{:.0}", made as f64 / km),
            ms(per_km),
            format!("{:.0}", 1000.0 / (per_km / 1000.0)),
        ]);
    }
}
