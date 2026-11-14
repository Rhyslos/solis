use super::*;
use crate::brush::{Action, Brush};
use crate::lod::{Levels, average, voxel};
use crate::voxel::SIZE_I;
use glam::{IVec3, Vec3};
use std::sync::atomic::{AtomicU32, Ordering};

/// A wavy slab, the same at every level (each sample the formula at its
/// place).
fn slab(level: u32, key: IVec3) -> Chunk<Full> {
    let mut dense = Dense::filled(Voxel::AIR);
    let w = voxel(level);
    for (i, v) in dense.voxels_mut().iter_mut().enumerate() {
        let g = key * SIZE_I + crate::voxel::place(i).as_ivec3();
        let m = g.as_vec3() * w;
        *v = Voxel::at(
            (m.z - 40.0 - 5.0 * (m.x / 20.0).sin()) / w,
            1 + (g.y % 2) as u8,
        );
    }
    Chunk::keep(&dense)
}

/// Two regions of the slab (512 × 256 m) written into a new folder.
fn folder() -> PathBuf {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let folder = std::env::temp_dir().join(format!(
        "ground-study-stream-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::create_dir_all(&folder);
    let files = Files::new(&folder);
    for r in [IVec2::ZERO, IVec2::X] {
        let packed: Vec<Packed> = region::all_chunks(r)
            .into_iter()
            .map(|(level, key)| region::pack(&slab(level, key)))
            .collect();
        let _ = std::fs::write(files.path(r), region::write(r, &packed));
    }
    folder
}

fn sizes() -> Vec<IVec3> {
    (0..region::LEVELS)
        .map(|l| IVec3::new(512 >> l, 256 >> l, SIZE_I * world::chunks_high(l) as i32))
        .collect()
}

fn streamer(folder: &Path, policy: Policy, budget: usize) -> Streamer<Files> {
    Streamer::new(Files::new(folder), policy, budget, 1, sizes()).unwrap_or_else(|e| panic!("{e}"))
}

fn want(ids: &[ChunkId]) -> Vec<Wanted> {
    ids.iter()
        .enumerate()
        .map(|(i, id)| Wanted {
            id: *id,
            priority: i as f32,
            drawn: true,
        })
        .collect()
}

#[test]
fn wanted_chunks_are_read_on_a_worker_as_they_were_written() {
    let folder = folder();
    let mut s = streamer(&folder, Policy::Priority, usize::MAX);
    let ids = [
        (0, IVec3::new(9, 2, 1)),
        (1, IVec3::new(5, 1, 0)),
        (3, IVec3::new(1, 0, 0)),
    ];
    s.update(&want(&ids));
    s.wait();
    s.update(&want(&ids));
    assert_eq!(s.kept_count(), 3);
    for (level, key) in ids {
        let g = key * SIZE_I + IVec3::new(3, 17, 9);
        let Chunk::Mixed(expected) = slab(level, key) else {
            continue;
        };
        assert_eq!(
            s.kept().voxel(level, g),
            crate::store::Store::get(&expected, glam::UVec3::new(3, 17, 9))
        );
    }
    assert!(s.stats.bytes_read > 0);
    let _ = std::fs::remove_dir_all(folder);
}

#[test]
fn whole_regions_read_every_chunk_of_the_level_at_once() {
    let folder = folder();
    let mut s = streamer(&folder, Policy::Regions, usize::MAX);
    s.update(&want(&[(1, IVec3::new(5, 1, 0))]));
    s.wait();
    assert_eq!(s.kept_count(), 4 * 4 * 2);
    let _ = std::fs::remove_dir_all(folder);
}

#[test]
fn over_budget_the_least_wanted_go_first() {
    let folder = folder();
    let mut s = streamer(&folder, Policy::Priority, usize::MAX);
    let ids: Vec<ChunkId> = (0..8).map(|x| (0, IVec3::new(x, 0, 1))).collect();
    s.update(&want(&ids));
    s.wait();
    s.update(&want(&ids));
    let all = s.kept().bytes();
    s.budget = all * 5 / 8;
    s.update(&want(&ids));
    assert!(s.kept().bytes() <= s.budget);
    assert!(s.kept().contains(ids[0]) && !s.kept().contains(ids[7]));
    assert!(s.stats.evicted > 0);
    let _ = std::fs::remove_dir_all(folder);
}

#[test]
fn streaming_never_loses_an_edit() {
    let folder = folder();
    let mut s = streamer(&folder, Policy::Priority, usize::MAX);
    let brush = Brush {
        center: Vec3::new(300.0, 100.0, 42.0),
        radius: 5.0,
        strength: 1.0,
        action: Action::Add(3),
    };
    let changed = s.edit(&brush).unwrap_or_else(|e| panic!("{e}"));
    assert!(changed.iter().any(|(l, _)| *l == 0) && changed.iter().any(|(l, _)| *l == 3));
    let top = IVec3::new(300, 100, 45);
    let edited = s.kept().voxel(0, top);
    assert!(edited.is_solid() && edited.material == 3);
    // Nothing wanted and no memory: the sculpted chunks are written back
    // before they go.
    s.budget = 0;
    s.update(&[]);
    assert!(s.kept().voxel(0, top) == edited, "kept until written");
    s.wait();
    s.update(&[]);
    assert!(s.kept_count() == 0 && s.stats.saves > 0);
    // Wanted again: read back from the file, sculpted.
    s.budget = usize::MAX;
    let id = (0, crate::voxel::split(top).0);
    s.update(&want(&[id]));
    s.wait();
    assert_eq!(s.kept().voxel(0, top), edited);
    // And a fresh start reads it too.
    let mut again = streamer(&folder, Policy::Priority, usize::MAX);
    again.update(&want(&[id]));
    again.wait();
    assert_eq!(again.kept().voxel(0, top), edited);
    let _ = std::fs::remove_dir_all(folder);
}

#[test]
fn an_edit_averages_the_coarser_levels_again() {
    let folder = folder();
    let mut s = streamer(&folder, Policy::Priority, usize::MAX);
    let brush = Brush {
        center: Vec3::new(130.0, 130.0, 40.0),
        radius: 9.0,
        strength: 1.0,
        action: Action::Dig,
    };
    let changed = s.edit(&brush).unwrap_or_else(|e| panic!("{e}"));
    for level in 1..region::LEVELS {
        let g = IVec3::new(130, 130, 40) >> level;
        assert_eq!(
            s.kept().voxel(level, g),
            average(|p| s.kept().voxel(level - 1, p), g)
        );
        assert!(changed.iter().any(|(l, _)| *l == level), "level {level}");
    }
    assert!(!s.kept().voxel(0, IVec3::new(130, 130, 38)).is_solid());
    s.flush();
    assert!(changed.iter().all(|id| !s.is_dirty(*id)));
    let _ = std::fs::remove_dir_all(folder);
}

#[test]
fn the_world_made_from_formulas_keeps_only_what_was_sculpted() {
    let made = Made::default();
    let id = (3, IVec3::new(4, 4, 0));
    let (first, read) = made.chunk(id).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(read, 0);
    assert!(
        made.save(IVec2::new(4, 4), &[(id, Chunk::Uniform(Voxel::AIR))])
            .is_ok()
    );
    assert_eq!(made.saved(), 1);
    assert!(matches!(made.chunk(id), Ok((Chunk::Uniform(v), _)) if v == Voxel::AIR));
    assert!(matches!(first, Chunk::Mixed(_)));
}
