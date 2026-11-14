//! The open-worlds study's measurements (`docs/reports/open-worlds-study.md`):
//! how a sculpted ground reaches 20 × 20 km and beyond. Run with
//! `cargo run --release -p ground-study -- worlds`.

mod disk;
mod far_edits;
mod seams;
mod streaming;
mod units;
mod view;

#[cfg(feature = "gpu")]
pub(crate) mod pictures;

use super::stats::timed;
use glam::{IVec2, IVec3, Vec2};
use ground_study::ground::{Chunk, Ground};
use ground_study::lod::layout::{Column, Layout};
use ground_study::lod::nets::{self, Look, Seams};
use ground_study::lod::{ChunkId, Kept, Levels, Pyramid, blocky};
use ground_study::mesh::ChunkMesh;
use ground_study::store::Full;
use ground_study::stream::wanted_for;
use ground_study::voxel::{Dense, SIZE_I, Voxel};
use ground_study::world;
use rayon::prelude::*;

/// The three looks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Looks {
    Smooth,
    Sharp,
    Blocky,
}

impl Looks {
    pub(crate) const ALL: [Self; 3] = [Self::Smooth, Self::Sharp, Self::Blocky];

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Smooth => "Smooth",
            Self::Sharp => "Sharp",
            Self::Blocky => "Blocky",
        }
    }
}

/// Every measurement, in the report's order, or one part of them
/// (`levels`, `view`, `streaming`, `units` or `disk`).
pub(crate) fn run(part: Option<&str>) {
    println!(
        "Open-worlds study: release build, {} cores here, {} threads in the job pool.",
        std::thread::available_parallelism().map_or(1, |n| n.get()),
        rayon::current_num_threads()
    );
    let corner = hilly_corner();
    println!(
        "\nThe measured square kilometre and the streamed 2 km start at {} m, {} m in the 20 km world.",
        corner.x, corner.y
    );
    let wants = |name: &str| part.is_none_or(|p| p == name);
    if wants("levels") {
        let pyramid = square_km(corner);
        seams::popping(&pyramid);
        seams::seams(&pyramid);
        far_edits::far_chunks(pyramid);
    }
    if wants("view") {
        view::views();
        view::moving();
    }
    if wants("streaming") {
        streaming::files_and_streaming(corner);
    }
    if wants("units") {
        units::colliders_and_units(corner);
    }
    if wants("disk") {
        let region = disk::disk_and_generation();
        disk::how_far(region);
    }
}

/// The world's size in samples of each level, for a world `side` metres
/// square.
pub(crate) fn sizes(side: i32, levels: u32) -> Vec<IVec3> {
    (0..levels)
        .map(|l| IVec3::new(side >> l, side >> l, SIZE_I * world::chunks_high(l) as i32))
        .collect()
}

/// The south-west corner (metres, a multiple of 2 km) of the 2 × 2 km
/// square of the world with the most between its lowest and highest
/// ground: mountains and valleys both.
pub(crate) fn hilly_corner() -> IVec2 {
    let mut best = (0.0, IVec2::ZERO);
    for y in 0..10 {
        for x in 0..10 {
            let corner = IVec2::new(x, y) * 2048;
            let (mut lo, mut hi) = (f32::MAX, f32::MIN);
            for j in 0..16 {
                for i in 0..16 {
                    let h = world::height((corner + IVec2::new(i, j) * 128).as_vec2());
                    lo = lo.min(h);
                    hi = hi.max(h);
                }
            }
            if hi - lo > best.0 {
                best = (hi - lo, corner);
            }
        }
    }
    best.1
}

/// A square kilometre of the world from `corner`, every level from 0 to
/// 4 made by averaging (level 0 from the formulas).
pub(crate) fn square_km(corner: IVec2) -> Pyramid {
    let offset = (corner / SIZE_I).extend(0);
    let (time, pyramid) = timed(|| {
        let base: Ground<Full> = Ground::build(glam::UVec3::new(32, 32, 4), |key, dense| {
            world::fill(0, key + offset, dense);
        });
        Pyramid::build(base, 5)
    });
    println!(
        "\nThe square kilometre: levels 0 to 4 made in {} ms on every core (level 0 from the formulas, the rest by averaging).",
        super::stats::ms(time)
    );
    pyramid
}

/// The chunks wanted for `columns` (and a column round each), made from
/// the world's formulas on every core.
pub(crate) fn made_for(columns: &[Column], side: i32, levels: u32) -> Kept {
    let wanted = wanted_for(columns, Vec2::ZERO);
    let sizes = sizes(side, levels);
    let made: Vec<(ChunkId, Chunk<Full>)> = wanted
        .par_iter()
        .filter(|w| {
            let size = sizes.get(w.id.0 as usize).copied().unwrap_or(IVec3::ZERO);
            (w.id.1 * SIZE_I).cmplt(size).all()
        })
        .map_init(
            || Dense::filled(Voxel::AIR),
            |dense, w| {
                world::fill(w.id.0, w.id.1, dense);
                (w.id, Chunk::keep(dense))
            },
        )
        .collect();
    let mut kept = Kept::new(sizes);
    for (id, chunk) in made {
        kept.insert(id, chunk);
    }
    kept
}

/// One chunk's mesh, made one way, and the time it took (ms).
pub(crate) struct Meshed {
    pub(crate) id: ChunkId,
    pub(crate) mesh: ChunkMesh,
    pub(crate) border: ChunkMesh,
    pub(crate) ms: f64,
}

/// The chunk `id` meshed in `look`, meeting its neighbours by `seams`.
pub(crate) fn mesh_one(
    levels: &impl Levels,
    layout: &Layout,
    id: ChunkId,
    look: Looks,
    seams: Seams,
) -> Meshed {
    let (ms, (mesh, border)) = timed(|| match look {
        Looks::Blocky => (
            blocky::mesh(levels, layout, id.0, id.1, seams),
            ChunkMesh::default(),
        ),
        Looks::Smooth | Looks::Sharp => {
            let shape = if look == Looks::Smooth {
                Look::Smooth
            } else {
                Look::Sharp
            };
            let made = nets::mesh(levels, layout, id.0, id.1, shape, seams);
            (made.mesh, made.border)
        }
    });
    Meshed {
        id,
        mesh,
        border,
        ms,
    }
}

/// The chunks of `columns` that might hold ground (not wholly air or
/// rock, by what `levels` keeps), in a fixed order.
pub(crate) fn chunks_of(levels: &Kept, columns: &[Column]) -> Vec<ChunkId> {
    let mut out = Vec::new();
    for c in columns {
        let high = levels.size(c.level).z / SIZE_I;
        for z in 0..high {
            let id = (c.level, c.key.extend(z));
            if !matches!(levels.get(id), Some(Chunk::Uniform(v)) if far_from_neighbours(levels, id, *v))
            {
                out.push(id);
            }
        }
    }
    out
}

/// Whether the chunk `id`, the same all through (`v`), has neighbours of
/// the same: nothing to mesh there.
fn far_from_neighbours(levels: &Kept, (level, key): ChunkId, v: Voxel) -> bool {
    ground_study::ground::boxed(key - 1, key + 2).all(|k| {
        let id = (level, k);
        match levels.get(id) {
            Some(Chunk::Uniform(u)) => u.is_solid() == v.is_solid(),
            Some(Chunk::Mixed(_)) => false,
            None => true,
        }
    })
}

/// The chunks of `columns` meshed on every core.
pub(crate) fn mesh_all(
    levels: &Kept,
    layout: &Layout,
    columns: &[Column],
    look: Looks,
    seams: Seams,
) -> Vec<Meshed> {
    chunks_of(levels, columns)
        .par_iter()
        .map(|id| mesh_one(levels, layout, *id, look, seams))
        .collect()
}
