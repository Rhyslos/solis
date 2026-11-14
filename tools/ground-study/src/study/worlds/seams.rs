//! On a square kilometre with every level made by averaging: how far
//! each level's surface strays from the one below (popping), and each
//! way of closing the seams between levels, in each look.

use super::super::stats::{Spread, Table, count, ms, timed};
use super::{Looks, Meshed, mesh_all, mesh_one};
use glam::{IVec2, Vec3};
use ground_study::far::top;
use ground_study::ground::Chunk;
use ground_study::lod::cracks::{Scene, Seen, deep, open_edges};
use ground_study::lod::layout::{Layout, new_columns};
use ground_study::lod::nets::{self, Look, Seams};
use ground_study::lod::skirts;
use ground_study::lod::{Kept, Levels, Pyramid, voxel};
use ground_study::mesh::ChunkMesh;
use ground_study::voxel::SIZE_I;
use std::collections::HashSet;

/// Pixels a metre covers a metre away, on a 1080-line screen with the
/// editor's 50° tall view.
pub(crate) const PIXELS_A_RADIAN: f32 = 1158.0;

/// The rings' width the study recommends, and the distance at which a
/// level's square ends (where it switches to the next): about three and
/// a half of its columns from the camera.
pub(crate) fn switch_distance(level: u32) -> f32 {
    3.5 * (SIZE_I << level) as f32
}

/// How far each level's top surface strays from the level below's.
pub(crate) fn popping(p: &Pyramid) {
    let table = Table::start(
        "Popping: each level's surface against the level below's (a square kilometre, every sample)",
        &[
            "Levels",
            "Voxel",
            "Height apart, mean (m)",
            "95th (m)",
            "Worst (m)",
            "Switch distance, rings of 8 (m)",
            "On screen, mean (px)",
            "95th (px)",
        ],
    );
    for level in 0..p.count() - 1 {
        let size = p.size(level + 1);
        let mut apart = Vec::new();
        for y in 2..size.y - 2 {
            for x in 2..size.x - 2 {
                let g = IVec2::new(x, y);
                let (fine, _) = top(p, level, g * 2);
                let (coarse, _) = top(p, level + 1, g);
                apart.push(f64::from((fine - coarse).abs()));
            }
        }
        let spread = Spread::of(&apart);
        let d = switch_distance(level);
        let px = |m: f64| format!("{:.2}", m as f32 / d * PIXELS_A_RADIAN);
        table.row(&[
            format!("{level} → {}", level + 1),
            format!("{} m", voxel(level + 1)),
            format!("{:.2}", spread.mean),
            format!("{:.2}", spread.p95),
            format!("{:.2}", spread.worst),
            format!("{d:.0}"),
            px(spread.mean),
            px(spread.p95),
        ]);
    }
}

/// Every chunk of a pyramid as kept chunks.
pub(crate) fn kept(p: &Pyramid) -> Kept {
    let mut kept = Kept::new((0..p.count()).map(|l| p.size(l)).collect());
    for level in 0..p.count() {
        if let Some(g) = p.level(level) {
            for key in g.keys() {
                if let Some(c) = g.chunk(key) {
                    kept.insert((level, key), c.clone());
                }
            }
        }
    }
    kept
}

/// Places just past the open edges of the Smooth ground meshed with
/// nothing done (the cracks), away from the world's edges, with the
/// ground's facing there; at most `most`, spread evenly.
fn crack_targets(meshed: &[Meshed], most: usize) -> Vec<(Vec3, Vec3)> {
    let mut out = Vec::new();
    for m in meshed {
        for (a, b) in skirts::open_edges(&m.mesh) {
            let (pa, pb) = (m.mesh.positions[a as usize], m.mesh.positions[b as usize]);
            let middle = (pa + pb) / 2.0;
            if middle.x < 8.0 || middle.y < 8.0 || middle.x > 1016.0 || middle.y > 1016.0 {
                continue;
            }
            let up = m.mesh.normals[a as usize];
            if up.z < 0.3 {
                continue; // under overhangs and in caves: seen from no camera here
            }
            let out_of = (pb - pa).normalize_or_zero().cross(up).normalize_or_zero();
            out.push((middle + out_of * 0.3 * (pb - pa).length().max(0.5), up));
        }
    }
    let step = (out.len() / most).max(1);
    out.into_iter().step_by(step).take(most).collect()
}

/// Rays at each target from a camera 8 m out along the ground's facing
/// and 60 m to four sides: low, grazing looks at the seams, as far seams
/// are seen.
fn rays(kept: &Kept, layout: &Layout, meshes: &[&ChunkMesh], targets: &[(Vec3, Vec3)]) -> Seen {
    let scene = Scene::new(meshes, 8.0);
    let mut seen = Seen::default();
    for (t, up) in targets {
        let across = up.any_orthonormal_vector();
        let along = up.cross(across);
        for side in [across, -across, along, -along] {
            let eye = *t + *up * 8.0 + side * 60.0;
            let clear = (0..kept.levels()).all(|l| {
                kept.voxel(l, (eye / voxel(l)).round().as_ivec3())
                    .distance()
                    >= 1.0
            });
            if !clear {
                continue;
            }
            let s = scene.look(eye, &[*t], |q| deep(kept, layout, q), |_| 0.25);
            seen.rays += s.rays;
            seen.through += s.through;
        }
    }
    seen
}

/// The ways of closing seams, each look, on the square kilometre seen
/// from its middle (rings 6 columns wide, levels 0 to 4).
pub(crate) fn seams(p: &Pyramid) {
    let kept = kept(p);
    let across = IVec2::splat(32);
    let camera = glam::Vec2::new(520.0, 500.0);
    let layout = Layout::rings(across, camera, 4, 6);
    let columns = layout.columns();
    let levels: HashSet<u32> = columns.iter().map(|c| c.level).collect();
    println!(
        "\nSeams on the square kilometre: {} columns at levels {:?}.",
        columns.len(),
        {
            let mut l: Vec<u32> = levels.into_iter().collect();
            l.sort_unstable();
            l
        }
    );
    let bare = mesh_all(&kept, &layout, &columns, Looks::Smooth, Seams::None);
    let targets = crack_targets(&bare, 3000);
    let table = Table::start(
        "Closing the seams between levels: the square kilometre, each look",
        &[
            "Look",
            "Way",
            "Triangles",
            "More than nothing",
            "Chunk, mean (ms)",
            "Worst (ms)",
            "Open edges",
            "Rays at the seams",
            "Seen through",
        ],
    );
    for look in Looks::ALL {
        let mut none_triangles = 0;
        for seams in Seams::ALL {
            if look == Looks::Blocky && seams == Seams::Stitched {
                continue; // for cubes the same walls as the transition
            }
            let meshed = mesh_all(&kept, &layout, &columns, look, seams);
            let mut meshes: Vec<&ChunkMesh> = meshed.iter().map(|m| &m.mesh).collect();
            meshes.extend(meshed.iter().map(|m| &m.border));
            let triangles: usize = meshes.iter().map(|m| m.triangles()).sum();
            if seams == Seams::None {
                none_triangles = triangles;
            }
            let times: Vec<f64> = meshed.iter().map(|m| m.ms).collect();
            let spread = Spread::of(&times);
            let edges = if look == Looks::Blocky {
                "(rectangles meet mid-side)".to_string()
            } else {
                let world_edge = |q: Vec3| q.x < 1.0 || q.y < 1.0 || q.x > 1015.0 || q.y > 1015.0;
                count(open_edges(&meshes, world_edge))
            };
            let seen = rays(&kept, &layout, &meshes, &targets);
            let way = if look == Looks::Blocky && seams == Seams::Transition {
                "Walls between sizes"
            } else {
                seams.name()
            };
            table.row(&[
                look.name().into(),
                way.into(),
                count(triangles),
                format!(
                    "{:+.1} %",
                    100.0 * (triangles as f64 / none_triangles.max(1) as f64 - 1.0)
                ),
                ms(spread.mean),
                ms(spread.worst),
                edges,
                count(seen.rays),
                format!(
                    "{} ({:.2} %)",
                    count(seen.through),
                    100.0 * seen.through as f64 / seen.rays.max(1) as f64
                ),
            ]);
        }
    }
    rebuilds(&kept, camera);
}

/// What's made again when the camera moves 150 m and levels change round
/// it, each way (one core, Smooth).
fn rebuilds(kept: &Kept, camera: glam::Vec2) {
    let across = IVec2::splat(32);
    let before = Layout::rings(across, camera, 4, 6);
    let after = Layout::rings(across, camera + glam::Vec2::new(130.0, 70.0), 4, 6);
    let new: Vec<_> = new_columns(&before, &after);
    // Columns kept whose neighbours' levels changed.
    let changed_cells: HashSet<IVec2> = (0..32)
        .flat_map(|y| (0..32).map(move |x| IVec2::new(x, y)))
        .filter(|c| before.level_of(*c) != after.level_of(*c))
        .collect();
    let touched: Vec<_> = after
        .columns()
        .into_iter()
        .filter(|c| !new.contains(c))
        .filter(|c| {
            let lo = c.key << c.level;
            let hi = lo + (1 << c.level);
            (lo.y - 1..=hi.y)
                .any(|y| (lo.x - 1..=hi.x).any(|x| changed_cells.contains(&IVec2::new(x, y))))
        })
        .collect();
    let table = Table::start(
        "When the camera moves 150 m: what is made again (Smooth, one core)",
        &[
            "Way",
            "Columns new",
            "Neighbours made again",
            "Chunks",
            "Time (ms)",
        ],
    );
    let chunks = |cols: &[ground_study::lod::layout::Column]| super::chunks_of(kept, cols);
    let new_chunks = chunks(&new);
    let near_chunks = chunks(&touched);
    let full = |ids: &[ground_study::lod::ChunkId], seams: Seams| -> f64 {
        ids.iter()
            .map(|id| mesh_one(kept, &after, *id, Looks::Smooth, seams).ms)
            .sum()
    };
    for seams in Seams::ALL {
        let (near, time) = match seams {
            Seams::None | Seams::Skirts => (0, full(&new_chunks, seams)),
            Seams::Transition => (
                near_chunks.len(),
                full(&new_chunks, seams) + full(&near_chunks, seams),
            ),
            Seams::Stitched => {
                let (border, _) = timed(|| {
                    for id in &near_chunks {
                        std::hint::black_box(nets::border(kept, &after, id.0, id.1, Look::Smooth));
                    }
                });
                (near_chunks.len(), full(&new_chunks, seams) + border)
            }
        };
        table.row(&[
            seams.name().into(),
            new.len().to_string(),
            if near == 0 {
                "none".into()
            } else {
                format!("{} columns' {} chunks", touched.len(), near)
            },
            (new_chunks.len() + near).to_string(),
            ms(time),
        ]);
    }
    let uniform = kept
        .ids()
        .filter(|id| matches!(kept.get(*id), Some(Chunk::Uniform(_))))
        .count();
    println!(
        "\n({} of {} chunks kept are the same all through.)",
        count(uniform),
        count(kept.len())
    );
}
