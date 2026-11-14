//! Far chunks: what remaking one costs at each level, and what happens
//! when the ground far from the camera is sculpted (by the editor, or by
//! game code: a crater kilometres away).

use super::super::stats::{Spread, Table, count, ms, timed};
use super::{Looks, mesh_one};
use glam::IVec2;
use ground_study::brush::{Action, Brush};
use ground_study::far::top;
use ground_study::lod::layout::Layout;
use ground_study::lod::nets::Seams;
use ground_study::lod::{Levels, Pyramid, voxel};
use ground_study::mesh::ChunkMesh;
use ground_study::shapes::DIRT;
use ground_study::voxel::SIZE_I;

/// Remaking a chunk at each level, each look; then sculpting far away.
pub(crate) fn far_chunks(mut p: Pyramid) {
    let table = Table::start(
        "Remaking a chunk at each level (the square kilometre, every chunk with ground, one core)",
        &[
            "Look",
            "Level",
            "Chunk's width",
            "Chunks",
            "Chunk, mean (ms)",
            "95th (ms)",
            "Triangles a chunk, mean",
        ],
    );
    for look in Looks::ALL {
        for level in 0..p.count() {
            let layout = Layout::uniform(IVec2::splat(32), level);
            let size = p.size(level);
            let across = size / SIZE_I;
            let mut times = Vec::new();
            let mut triangles = 0;
            for z in 0..across.z {
                for y in 0..across.y {
                    for x in 0..across.x {
                        let m = mesh_one(
                            &p,
                            &layout,
                            (level, glam::IVec3::new(x, y, z)),
                            look,
                            Seams::Skirts,
                        );
                        if m.mesh.triangles() > 0 {
                            times.push(m.ms);
                            triangles += m.mesh.triangles();
                        }
                    }
                }
            }
            let spread = Spread::of(&times);
            table.row(&[
                look.name().into(),
                level.to_string(),
                format!("{} m", SIZE_I << level),
                times.len().to_string(),
                ms(spread.mean),
                ms(spread.p95),
                count(triangles / times.len().max(1)),
            ]);
        }
    }
    sculpting_far(&mut p);
}

/// The farthest any point of `after` is from every point of `before`.
fn moved(before: &ChunkMesh, after: &ChunkMesh) -> f32 {
    after
        .positions
        .iter()
        .map(|a| {
            before
                .positions
                .iter()
                .map(|b| a.distance_squared(*b))
                .fold(f32::MAX, f32::min)
        })
        .fold(0.0, f32::max)
        .sqrt()
}

/// Three brushes far from the camera, the chunk drawn there at levels 2,
/// 3 and 4: sculpting level 0 and averaging every level above, then the
/// far chunk made again.
fn sculpting_far(p: &mut Pyramid) {
    let table = Table::start(
        "Sculpting far away: a brush where the ground is drawn at a coarse level (Smooth)",
        &[
            "Brush",
            "Drawn at level",
            "Sculpt and average every level (ms)",
            "Chunks changed, levels 0 to 4",
            "Far chunks made again (ms)",
            "The far surface moved (m)",
        ],
    );
    let brushes = [
        ("Add, 3 m", 1.5, Action::Add(DIRT)),
        ("Dig, 12 m", 6.0, Action::Dig),
        ("Dig, a 40 m crater", 20.0, Action::Dig),
    ];
    let mut place = 0;
    for (name, radius, action) in brushes {
        for level in [2, 3, 4] {
            place += 1;
            // A different place each time, on the surface.
            let xy = IVec2::new(150 + 80 * (place % 9), 120 + 70 * (place / 3));
            let (h, _) = top(p, 0, xy);
            let brush = Brush {
                center: xy.as_vec2().extend(h),
                radius,
                strength: 1.0,
                action,
            };
            let layout = Layout::uniform(IVec2::splat(32), level);
            let far_key = (xy / (SIZE_I << level)).extend((h as i32) / (SIZE_I << level));
            let before = mesh_one(p, &layout, (level, far_key), Looks::Smooth, Seams::Skirts);
            let (edit_ms, changed) = timed(|| p.apply(&brush));
            let (remake_ms, after) = timed(|| {
                changed[level as usize]
                    .iter()
                    .map(|k| mesh_one(p, &layout, (level, *k), Looks::Smooth, Seams::Skirts))
                    .collect::<Vec<_>>()
            });
            let moved_m = after
                .iter()
                .find(|m| m.id.1 == far_key)
                .map_or(0.0, |m| moved(&before.mesh, &m.mesh));
            table.row(&[
                name.into(),
                format!("{level} ({} m voxels)", voxel(level)),
                ms(edit_ms),
                changed
                    .iter()
                    .map(|c| c.len().to_string())
                    .collect::<Vec<_>>()
                    .join(", "),
                ms(remake_ms),
                format!("{moved_m:.2}"),
            ]);
        }
    }
}
