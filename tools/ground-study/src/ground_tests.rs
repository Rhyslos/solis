use super::*;
use crate::brush::Action;
use crate::shapes::{ball, fill, hill_with_cave};
use crate::store::{Full, Octree, Sparse};
use glam::Vec3;

pub(crate) fn ball_ground<S: Store>() -> Ground<S> {
    let shape = ball(Vec3::splat(32.0), 20.0);
    Ground::build(UVec3::splat(2), |key, dense| fill(key, dense, &shape))
}

#[test]
fn gathering_matches_reading_voxel_by_voxel() {
    let ground: Ground<Octree> = ball_ground();
    let mut samples = Samples::default();
    let mut scratch = Dense::filled(Voxel::AIR);
    for key in ground.keys() {
        ground.gather(key, &mut scratch, &mut samples);
        for z in -APRON..SIZE_I + APRON {
            for y in [-APRON, 0, 17, SIZE_I + 1] {
                for x in -APRON..SIZE_I + APRON {
                    let p = IVec3::new(x, y, z);
                    assert_eq!(samples.get(p), ground.get(key * SIZE_I + p), "{key} {p}");
                }
            }
        }
    }
}

#[test]
fn past_an_edge_the_edge_voxel_goes_on() {
    let ground: Ground<Full> = ball_ground();
    let edge = ground.get(IVec3::new(0, 31, 32));
    assert_eq!(ground.get(IVec3::new(-5, 31, 32)), edge);
    assert_eq!(ground.get(IVec3::new(-500, 31, 32)), edge);
}

#[test]
fn chunks_away_from_the_surface_are_uniform() {
    let ground: Ground<Sparse> = Ground::build(UVec3::new(3, 3, 2), |key, dense| {
        fill(key, dense, hill_with_cave)
    });
    let mixed = ground.mixed_keys().len();
    assert!(mixed > 0 && mixed < 18, "{mixed} of 18 mixed");
}

fn dab(center: Vec3, action: Action) -> Brush {
    Brush {
        center,
        radius: 3.0,
        strength: 1.0,
        action,
    }
}

#[test]
fn a_dab_on_a_corner_remeshes_the_chunks_round_it() {
    let mut ground: Ground<Full> = ball_ground();
    let keys = ground.apply(&dab(Vec3::new(32.0, 32.0, 52.0), Action::Add(1)));
    assert_eq!(
        keys.len(),
        4,
        "the four chunks that meet under it: {keys:?}"
    );
    let keys = ground.apply(&dab(Vec3::new(16.0, 16.0, 52.0), Action::Add(1)));
    assert_eq!(keys, vec![IVec3::new(0, 0, 1)], "one chunk well inside");
}

#[test]
fn a_dab_in_empty_air_remeshes_nothing() {
    let mut ground: Ground<Octree> = ball_ground();
    assert!(
        ground
            .apply(&dab(Vec3::new(4.0, 4.0, 4.0), Action::Dig))
            .is_empty()
    );
}

#[test]
fn every_store_edits_to_the_same_voxels() {
    let mut full: Ground<Full> = ball_ground();
    let mut octree: Ground<Octree> = ball_ground();
    let mut sparse: Ground<Sparse> = ball_ground();
    let dabs = [
        dab(Vec3::new(32.0, 32.0, 52.0), Action::Add(2)),
        dab(Vec3::new(31.0, 33.0, 12.0), Action::Dig),
        dab(Vec3::new(14.0, 30.0, 33.0), Action::Smooth),
        dab(Vec3::new(50.0, 30.0, 33.0), Action::Paint(3)),
    ];
    for d in &dabs {
        let keys = full.apply(d);
        assert_eq!(octree.apply(d), keys);
        assert_eq!(sparse.apply(d), keys);
    }
    for z in 0..64 {
        for y in 0..64 {
            for x in 0..64 {
                let p = IVec3::new(x, y, z);
                let v = full.get(p);
                assert_eq!(octree.get(p), v, "{p}");
                // Digging shows the sparse store's one deep material.
                assert_eq!(sparse.get(p).distance, v.distance, "{p}");
            }
        }
    }
}

#[test]
fn bytes_count_only_mixed_chunks_in_full() {
    let ground: Ground<Full> = ball_ground();
    let mixed = ground.mixed_keys().len();
    assert!(ground.bytes() >= mixed * crate::voxel::VOLUME * 2);
    assert!(ground.bytes() < (mixed + 1) * crate::voxel::VOLUME * 2);
}
