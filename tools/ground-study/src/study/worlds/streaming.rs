//! Region files and streaming: a 2 × 2 km file set written, chunks read
//! from it, the camera flying across it at several speeds with each way
//! of choosing what to read, and a far edit written back.

use super::super::stats::{Spread, Table, count, mb, ms, timed};
use glam::{IVec2, IVec3, Vec2};
use ground_study::brush::{Action, Brush};
use ground_study::far::top;
use ground_study::lod::Levels;
use ground_study::lod::layout::{Column, Layout};
use ground_study::stream::region::{self, Entry, Index};
use ground_study::stream::{
    Files, Made, Policy, Source, Streamer, Wanted, made_region, wanted_for,
};
use ground_study::voxel::SIZE_I;
use ground_study::world;
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Regions along each side of the streamed square (2 km).
const SIDE_REGIONS: i32 = 8;

/// Where the study writes its region files.
pub(crate) fn folder() -> PathBuf {
    std::env::temp_dir().join("ground-study-worlds")
}

/// The regions of the 2 km square from `corner`.
fn regions(corner: IVec2) -> Vec<IVec2> {
    let first = corner / region::SIDE;
    (0..SIDE_REGIONS)
        .flat_map(|y| (0..SIDE_REGIONS).map(move |x| first + IVec2::new(x, y)))
        .collect()
}

pub(crate) fn files_and_streaming(corner: IVec2) {
    let folder = folder();
    let _ = std::fs::remove_dir_all(&folder);
    if std::fs::create_dir_all(&folder).is_err() {
        println!("\nNo folder for the region files: streaming not measured.");
        return;
    }
    let files = Files::new(&folder);
    let all = regions(corner);
    let (write_ms, written) = timed(|| {
        all.par_iter()
            .map(|r| {
                let bytes = made_region(*r);
                std::fs::write(files.path(*r), &bytes).is_ok()
            })
            .filter(|ok| *ok)
            .count()
    });
    println!(
        "\nThe 2 × 2 km file set: {written} region files made from the formulas and written in {} ms on every core.",
        ms(write_ms)
    );
    sizes(&files, &all);
    reads(&files, &all);
    flying(&folder, corner);
    far_edit(&folder, corner);
    from_formulas();
    let _ = std::fs::remove_dir_all(&folder);
}

/// Each region file's size, by level.
fn sizes(files: &Files, all: &[IVec2]) {
    let mut by_level = [0usize; region::LEVELS as usize];
    let mut sizes = Vec::new();
    let mut uniform = 0;
    let mut stored = 0;
    for r in all {
        let Ok(bytes) = std::fs::read(files.path(*r)) else {
            continue;
        };
        sizes.push(bytes.len() as f64);
        let Ok(index) = Index::read(&bytes, bytes.len(), *r) else {
            continue;
        };
        for ((level, _), e) in region::all_chunks(*r).iter().zip(&index.entries) {
            match e {
                Entry::Uniform(_) => uniform += 1,
                Entry::At { length, .. } => {
                    stored += 1;
                    by_level[*level as usize] += length;
                }
            }
        }
    }
    let total: f64 = sizes.iter().sum();
    let spread = Spread::of(&sizes);
    let table = Table::start(
        "The region files of the 2 × 2 km square (levels 0 to 3 each, chunks deflated one by one)",
        &[
            "Regions",
            "All of them",
            "A km²",
            "A region, mean",
            "Largest",
            "Level 0's share",
            "Levels 1 to 3's share",
            "Chunks the same all through",
        ],
    );
    let fine = by_level[0] as f64;
    let coarse: f64 = by_level[1..].iter().sum::<usize>() as f64;
    table.row(&[
        sizes.len().to_string(),
        mb(total),
        mb(total / 4.194),
        format!("{:.0} KB", spread.mean / 1024.0),
        format!("{:.0} KB", spread.worst / 1024.0),
        format!("{:.0} %", 100.0 * fine / (fine + coarse).max(1.0)),
        format!("{:.0} %", 100.0 * coarse / (fine + coarse).max(1.0)),
        format!("{} of {}", count(uniform), count(uniform + stored)),
    ]);
}

/// Reading one chunk, and a whole region's level 0.
fn reads(files: &Files, all: &[IVec2]) {
    let mut one = Vec::new();
    let mut one_bytes = 0;
    let mut whole = Vec::new();
    let mut whole_bytes = 0;
    for r in all.iter().step_by(3) {
        for (level, key) in region::chunks(*r, 0)
            .into_iter()
            .step_by(37)
            .map(|k| (0, k))
        {
            let (t, read) = timed(|| files.chunk((level, key)));
            if let Ok((_, bytes)) = read {
                one.push(t);
                one_bytes += bytes;
            }
        }
        let (t, read) = timed(|| files.region(*r, 0));
        if let Ok((_, bytes)) = read {
            whole.push(t);
            whole_bytes += bytes;
        }
    }
    let table = Table::start(
        "Reading from the region files (on one core; the files are in the system's cache, as just written)",
        &["Read", "Mean (ms)", "95th (ms)", "Bytes, mean"],
    );
    let a = Spread::of(&one);
    table.row(&[
        "One chunk (its region's table read once, then kept)".into(),
        ms(a.mean),
        ms(a.p95),
        count(one_bytes / one.len().max(1)),
    ]);
    let b = Spread::of(&whole);
    table.row(&[
        "A region's level 0 whole (256 chunks)".into(),
        ms(b.mean),
        ms(b.p95),
        count(whole_bytes / whole.len().max(1)),
    ]);
}

/// The columns drawn round `camera` inside the 2 km square from
/// `corner`: rings of 8, levels 0 to 3.
fn columns_in(corner: IVec2, camera: Vec2) -> Vec<Column> {
    let layout = Layout::rings(IVec2::splat(world::SIDE / SIZE_I), camera, 3, 8);
    let lo = corner;
    let hi = corner + SIDE_REGIONS * region::SIDE;
    layout
        .columns()
        .into_iter()
        .filter(|c| {
            let size = SIZE_I << c.level;
            let a = c.key * size;
            a.cmpge(lo).all() && (a + size).cmple(hi).all()
        })
        .collect()
}

/// What a frame wants inside the square.
fn wanted(corner: IVec2, camera: Vec2) -> Vec<Wanted> {
    let lo = corner;
    let hi = corner + SIDE_REGIONS * region::SIDE;
    wanted_for(&columns_in(corner, camera), camera)
        .into_iter()
        .filter(|w| {
            let size = SIZE_I << w.id.0;
            let a = w.id.1.truncate() * size;
            a.cmpge(lo).all() && (a + size).cmple(hi).all()
        })
        .collect()
}

fn sizes_20km() -> Vec<IVec3> {
    super::sizes(world::SIDE, region::LEVELS)
}

/// What a fly across the square found.
struct Flight {
    first_view_ms: f64,
    late_frames: usize,
    frames: usize,
    late_chunks: usize,
    peak: usize,
    bytes_per_km: f64,
    worker_share: f64,
    evicted: usize,
}

fn fly<S: Source>(
    source: S,
    policy: Policy,
    budget: usize,
    corner: IVec2,
    speed: f32,
    distance: f32,
) -> Option<Flight> {
    let mut s = Streamer::new(source, policy, budget, 1, sizes_20km()).ok()?;
    let y = corner.y as f32 + 1024.0;
    let start = corner.x as f32 + 256.0;
    let mut camera = Vec2::new(start, y);
    let mut want = wanted(corner, camera);
    let (first_view_ms, _) = timed(|| {
        s.per_frame = usize::MAX;
        s.update(&want);
        s.wait();
        s.update(&want);
        s.per_frame = 64;
    });
    let base = s.stats;
    let clock = Instant::now();
    let frame = Duration::from_secs_f64(1.0 / 60.0);
    let mut frames = 0;
    let mut late_frames = 0;
    let mut last_layout = camera;
    while camera.x < start + distance {
        let began = Instant::now();
        camera.x += speed / 60.0;
        if camera.distance(last_layout) >= 16.0 {
            want = wanted(corner, camera);
            last_layout = camera;
        }
        let late_before = s.stats.late;
        s.update(&want);
        if s.stats.late > late_before {
            late_frames += 1;
        }
        frames += 1;
        if let Some(rest) = frame.checked_sub(began.elapsed()) {
            std::thread::sleep(rest);
        }
    }
    let seconds = clock.elapsed().as_secs_f64();
    s.wait();
    Some(Flight {
        first_view_ms,
        late_frames,
        frames,
        late_chunks: s.stats.late - base.late,
        peak: s.stats.peak_bytes,
        bytes_per_km: (s.stats.bytes_read - base.bytes_read) as f64 / f64::from(distance / 1000.0),
        worker_share: (s.stats.worker_ms - base.worker_ms) / (seconds * 1000.0),
        evicted: s.stats.evicted,
    })
}

/// The camera flying 1.5 km east across the square, each way, at three
/// speeds, one worker.
fn flying(folder: &Path, corner: IVec2) {
    let table = Table::start(
        "Flying 1.5 km across the square with one worker reading (60 frames a second)",
        &[
            "Reading",
            "Budget",
            "Speed (m/s)",
            "First view ready (ms)",
            "Frames with a drawn chunk missing",
            "Drawn chunks missing, frames summed",
            "Most memory",
            "Read a km",
            "Worker busy",
            "Let go of",
        ],
    );
    let runs = [
        (Policy::Regions, usize::MAX, 50.0),
        (Policy::Regions, usize::MAX, 200.0),
        (Policy::Regions, usize::MAX, 800.0),
        (Policy::Regions, 96 << 20, 200.0),
        (Policy::Priority, usize::MAX, 50.0),
        (Policy::Priority, usize::MAX, 200.0),
        (Policy::Priority, usize::MAX, 800.0),
        (Policy::Priority, 48 << 20, 200.0),
        (Policy::Priority, 24 << 20, 200.0),
    ];
    for (policy, budget, speed) in runs {
        let Some(f) = fly(Files::new(folder), policy, budget, corner, speed, 1500.0) else {
            continue;
        };
        table.row(&[
            match policy {
                Policy::Regions => "Whole regions round the camera".into(),
                Policy::Priority => "Chunks by priority".to_string(),
            },
            if budget == usize::MAX {
                "none".into()
            } else {
                mb(budget as f64)
            },
            format!("{speed:.0}"),
            ms(f.first_view_ms),
            format!("{} of {}", f.late_frames, f.frames),
            count(f.late_chunks),
            mb(f.peak as f64),
            mb(f.bytes_per_km),
            format!("{:.0} %", 100.0 * f.worker_share),
            count(f.evicted),
        ]);
    }
}

/// A far edit: sculpting 1.2 km from the camera, where only level 3 is
/// drawn, then writing it back and reading it again.
fn far_edit(folder: &Path, corner: IVec2) {
    let Ok(mut s) = Streamer::new(
        Files::new(folder),
        Policy::Priority,
        usize::MAX,
        1,
        sizes_20km(),
    ) else {
        return;
    };
    let camera = corner.as_vec2() + Vec2::new(300.0, 300.0);
    let want = wanted(corner, camera);
    s.update(&want);
    s.wait();
    let place = corner + IVec2::new(1500, 1500);
    let (h, _) = top(s.kept(), 3, place / 8);
    let brush = Brush {
        center: place.as_vec2().extend(h),
        radius: 6.0,
        strength: 1.0,
        action: Action::Dig,
    };
    let reads_before = s.stats.reads;
    let (edit_ms, changed) = timed(|| s.edit(&brush));
    let reads = s.stats.reads - reads_before;
    let changed = changed.unwrap_or_default();
    let (save_ms, _) = timed(|| s.flush());
    // Every changed chunk read back from its file, fresh, and compared
    // voxel by voxel with what was sculpted.
    let files = Files::new(folder);
    let mut read_ms = 0.0;
    let mut same = true;
    for id in &changed {
        let (t, back) = timed(|| files.chunk(*id));
        read_ms += t;
        let mut fresh = ground_study::lod::Kept::new(sizes_20km());
        match back {
            Ok((chunk, _)) => fresh.insert(*id, chunk),
            Err(_) => same = false,
        }
        let origin = id.1 * SIZE_I;
        same &= ground_study::ground::boxed(origin, origin + SIZE_I)
            .all(|p| fresh.voxel(id.0, p) == s.kept().voxel(id.0, p));
    }
    let kept_same = same && !changed.is_empty();
    let changed = changed.len();
    let table = Table::start(
        "Sculpting 1.7 km from the camera (a 12 m dig), written back and read again",
        &[
            "Chunks read first",
            "Sculpt, read and average levels 0 to 3 (ms)",
            "Chunks changed",
            "Written back (ms)",
            "Read back, all of them (ms)",
            "Every voxel the same when read back",
        ],
    );
    table.row(&[
        reads.to_string(),
        ms(edit_ms),
        changed.to_string(),
        ms(save_ms),
        ms(read_ms),
        if kept_same {
            "yes".into()
        } else {
            "no".to_string()
        },
    ]);
}

/// Streaming the world made from its formulas, nothing on disk: how
/// fast chunks are made against read.
fn from_formulas() {
    let corner = IVec2::new(8192, 8192);
    let table = Table::start(
        "Streaming the world made from its formulas (nothing on disk), one worker",
        &[
            "Speed (m/s)",
            "First view ready (ms)",
            "Frames with a drawn chunk missing",
            "Worker busy",
        ],
    );
    for speed in [50.0, 200.0] {
        let Some(f) = fly(
            Made::default(),
            Policy::Priority,
            usize::MAX,
            corner,
            speed,
            1000.0,
        ) else {
            continue;
        };
        table.row(&[
            format!("{speed:.0}"),
            ms(f.first_view_ms),
            format!("{} of {}", f.late_frames, f.frames),
            format!("{:.0} %", 100.0 * f.worker_share),
        ]);
    }
}
