//! Disk and the editor: the 20 km file set's size from a sample of its
//! regions, opening a project without reading it all, making a starting
//! ground (from formulas, or a height map picture), and how far each
//! way goes.

use super::super::stats::{Spread, Table, count, mb, ms, timed};
use super::{Looks, made_for, mesh_all};
use glam::{IVec2, UVec3, Vec2};
use ground_study::far::{self, Heights};
use ground_study::ground::{Chunk, Ground};
use ground_study::lod::Pyramid;
use ground_study::lod::layout::{Column, Layout};
use ground_study::lod::nets::Seams;
use ground_study::mesh::ChunkMesh;
use ground_study::store::Full;
use ground_study::stream::region::{self, Packed};
use ground_study::stream::{Made, Source};
use ground_study::voxel::{SIZE_I, Voxel};
use ground_study::world;
use rayon::prelude::*;

/// Regions along each side of the 20 km world.
const REGIONS: i32 = world::SIDE / region::SIDE;

/// The file set, opening, and making a starting ground; returns a
/// region's mean size (bytes) and the time to make one (ms).
pub(crate) fn disk_and_generation() -> (f64, f64) {
    let (region_bytes, make_ms, pack_ms) = sample_regions();
    opening();
    starting_ground(make_ms + pack_ms);
    (region_bytes, make_ms + pack_ms)
}

/// Sixteen regions spread over the 20 km world, made from the formulas
/// and packed, one core each: their sizes and times.
fn sample_regions() -> (f64, f64, f64) {
    let picks: Vec<IVec2> = (0..16)
        .map(|i| IVec2::new((i * 37 + 5) % REGIONS, (i * 23 + 11) % REGIONS))
        .collect();
    let made = Made::default();
    let mut sizes = Vec::new();
    let mut makes = Vec::new();
    let mut packs = Vec::new();
    for r in &picks {
        let (make, chunks): (f64, Vec<Chunk<Full>>) = timed(|| {
            region::all_chunks(*r)
                .into_iter()
                .filter_map(|id| made.chunk(id).ok().map(|(c, _)| c))
                .collect()
        });
        let (pack, packed): (f64, Vec<Packed>) =
            timed(|| chunks.iter().map(region::pack).collect());
        let bytes = region::write(*r, &packed);
        sizes.push(bytes.len() as f64);
        makes.push(make);
        packs.push(pack);
    }
    let size = Spread::of(&sizes);
    let (make, pack) = (Spread::of(&makes).mean, Spread::of(&packs).mean);
    let regions = f64::from(REGIONS * REGIONS);
    let table = Table::start(
        "The 20 × 20 km file set, from 16 regions spread over it (one core each)",
        &[
            "Regions in the world",
            "A region, mean",
            "Smallest",
            "Largest",
            "The whole set",
            "Making a region from the formulas (ms)",
            "Packing it (ms)",
        ],
    );
    table.row(&[
        count(regions as usize),
        format!("{:.0} KB", size.mean / 1024.0),
        format!(
            "{:.0} KB",
            sizes.iter().copied().fold(f64::MAX, f64::min) / 1024.0
        ),
        format!("{:.0} KB", size.worst / 1024.0),
        format!("{:.2} GB", size.mean * regions / 1.0e9),
        ms(make),
        ms(pack),
    ]);
    (size.mean, make, pack)
}

/// Opening a 20 km project: listing its 6,400 region files (empty ones
/// here, as only their names are read), and reading the far field.
fn opening() {
    let folder = std::env::temp_dir().join("ground-study-names");
    let _ = std::fs::remove_dir_all(&folder);
    if std::fs::create_dir_all(&folder).is_err() {
        return;
    }
    for y in 0..REGIONS {
        for x in 0..REGIONS {
            let _ = std::fs::write(folder.join(format!("{x}_{y}.region")), []);
        }
    }
    let (list_ms, found) = timed(|| {
        std::fs::read_dir(&folder).map_or(0, |entries| {
            entries
                .filter_map(Result::ok)
                .filter(|e| {
                    let name = e.file_name();
                    let name = name.to_string_lossy();
                    name.strip_suffix(".region")
                        .and_then(|n| n.split_once('_'))
                        .is_some_and(|(x, y)| x.parse::<i32>().is_ok() && y.parse::<i32>().is_ok())
                })
                .count()
        })
    });
    let _ = std::fs::remove_dir_all(&folder);
    let (sample_ms, heights) =
        timed(|| Heights::sampled(world::SIDE as f32, 32.0, |p| (world::height(p), 0)));
    let raw: Vec<u8> = heights
        .heights
        .iter()
        .flat_map(|h| ((h * 64.0) as u16).to_le_bytes())
        .chain(heights.kinds.iter().copied())
        .collect();
    let path = std::env::temp_dir().join("ground-study-far-field");
    let _ = std::fs::write(&path, &raw);
    let (read_ms, back) = timed(|| std::fs::read(&path).map(|b| b.len()).unwrap_or(0));
    let _ = std::fs::remove_file(&path);
    let table = Table::start(
        "Opening a 20 km project: what is read before the first view",
        &["Step", "Time (ms)", "Size"],
    );
    table.row(&[
        format!("Listing the ground's folder ({found} region files, names only)"),
        ms(list_ms),
        "-".into(),
    ]);
    table.row(&[
        "Reading the far field: the whole world's heights and kinds every 32 m".into(),
        ms(read_ms),
        mb(back as f64),
    ]);
    table.row(&[
        "(Making that far field from the formulas, once, when the ground is saved)".into(),
        ms(sample_ms),
        "-".into(),
    ]);
}

/// Making a starting ground: from formulas (per region, so the whole 20
/// km is an estimate), and from a height map picture (a square
/// kilometre, measured).
fn starting_ground(region_ms: f64) {
    let regions = f64::from(REGIONS * REGIONS);
    let table = Table::start(
        "Making a starting ground",
        &[
            "How",
            "Measured",
            "The whole 20 × 20 km, one core",
            "On both cores here",
            "On 8 cores (an estimate)",
        ],
    );
    let minutes = |ms: f64| format!("{:.0} min", ms / 60_000.0);
    table.row(&[
        "From formulas (noise): every level 0 to 3 of a region, packed".into(),
        format!("{} ms a region", ms(region_ms)),
        minutes(region_ms * regions),
        minutes(region_ms * regions / 2.0),
        minutes(region_ms * regions / 7.0),
    ]);
    // A height map picture of a square kilometre, a metre a pixel,
    // 16 bits a height: here made from the formulas' heights.
    let side = 1024usize;
    let heights: Vec<u16> = (0..side * side)
        .map(|i| {
            let p = Vec2::new((i % side) as f32, (i / side) as f32);
            (world::height(p) * 256.0) as u16
        })
        .collect();
    let at = |x: i32, y: i32| {
        let (x, y) = (
            x.clamp(0, side as i32 - 1) as usize,
            y.clamp(0, side as i32 - 1) as usize,
        );
        f32::from(heights[x + side * y]) / 256.0
    };
    let (import_ms, pyramid) = timed(|| {
        let base: Ground<Full> = Ground::build(UVec3::new(32, 32, 4), |key, dense| {
            let origin = key * SIZE_I;
            for (i, v) in dense.voxels_mut().iter_mut().enumerate() {
                let p = origin + ground_study::voxel::place(i).as_ivec3();
                let h = at(p.x, p.y);
                let slope = 0.5
                    * Vec2::new(
                        at(p.x + 1, p.y) - at(p.x - 1, p.y),
                        at(p.x, p.y + 1) - at(p.x, p.y - 1),
                    )
                    .length();
                *v = Voxel::at(
                    (p.z as f32 - h) / (1.0 + slope * slope).sqrt(),
                    u8::from(slope > 0.8),
                );
            }
        });
        Pyramid::build(base, 4)
    });
    let km2 = 1.024 * 1.024;
    table.row(&[
        "From a height map picture (16 bits a metre): voxels and levels 1 to 3".into(),
        format!("{} ms a km² on both cores", ms(import_ms / km2)),
        minutes(import_ms / km2 * 2.0 * 400.0),
        minutes(import_ms / km2 * 400.0),
        minutes(import_ms / km2 * 2.0 * 400.0 / 7.0),
    ]);
    drop(pyramid);
}

/// The view drawn round the middle of a world `side` metres square (top
/// level `top`): rings of 8, voxels to level 3, the far field beyond.
fn far_field_view(side: i32, top: u32) -> (usize, usize, usize) {
    let across = IVec2::splat(side / SIZE_I);
    let camera = Vec2::splat(side as f32 / 2.0);
    let layout = Layout::rings(across, camera, top, 8);
    let columns = layout.columns();
    let kept = made_for(&columns, side, top + 1);
    let (voxels, tiles): (Vec<Column>, Vec<Column>) = columns.iter().partition(|c| c.level < 4);
    let meshed = mesh_all(
        &kept,
        &layout.capped(3),
        &voxels,
        Looks::Smooth,
        Seams::Transition,
    );
    let tiles: Vec<ChunkMesh> = tiles
        .par_iter()
        .map(|c| far::tile(&kept, c.level, c.key, true))
        .collect();
    let triangles = meshed
        .iter()
        .map(|m| m.mesh.triangles() + m.border.triangles())
        .sum::<usize>()
        + tiles.iter().map(ChunkMesh::triangles).sum::<usize>();
    let bytes = meshed
        .iter()
        .map(|m| m.mesh.upload_bytes() + m.border.upload_bytes())
        .sum::<usize>()
        + tiles.iter().map(ChunkMesh::upload_bytes).sum::<usize>();
    (columns.len(), triangles, bytes)
}

/// How far the recommended way goes: worlds 20, 41, 82 and 164 km wide.
pub(crate) fn how_far((region_bytes, region_ms): (f64, f64)) {
    let table = Table::start(
        "How far it goes: rings of 8, voxels to 2 km, the far field beyond (the camera in the middle)",
        &[
            "World",
            "Regions",
            "Region files, all made",
            "Far field every 32 m",
            "Columns drawn",
            "Triangles drawn",
            "Graphics memory",
            "Place precision at the far edge",
            "Making it all from formulas, one core",
        ],
    );
    for (side, top) in [(20_480, 7), (40_960, 8), (81_920, 9), (163_840, 10)] {
        let regions = f64::from(side / region::SIDE).powi(2);
        let (columns, triangles, bytes) = far_field_view(side, top);
        let far_field = (f64::from(side) / 32.0 + 1.0).powi(2) * 3.0;
        let edge = side as f32;
        let precision = f32::from_bits(edge.to_bits() + 1) - edge;
        table.row(&[
            format!(
                "{:.0} × {:.0} km",
                f64::from(side) / 1000.0,
                f64::from(side) / 1000.0
            ),
            count(regions as usize),
            format!("{:.1} GB", regions * region_bytes / 1.0e9),
            mb(far_field),
            count(columns),
            count(triangles),
            mb(bytes as f64),
            format!("{:.1} mm", precision * 1000.0),
            format!("{:.1} h", regions * region_ms / 3.6e6),
        ]);
    }
}
