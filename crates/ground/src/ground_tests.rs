use super::*;
use crate::start::Start;

#[test]
fn past_the_edges_the_edge_voxels_carry_on() {
    let g = Start::Flat { height: 10.5 }.ground(UVec3::new(1, 1, 1));
    assert_eq!(g.voxel(IVec3::new(-5, 3, 9)), g.voxel(IVec3::new(0, 3, 9)));
    assert_eq!(g.voxel(IVec3::new(3, 40, 2)), g.voxel(IVec3::new(3, 31, 2)));
    assert_eq!(g.voxel(IVec3::new(3, 3, -9)), g.voxel(IVec3::new(3, 3, 0)));
}

#[test]
fn samples_hold_the_neighbours_border() {
    let g = Start::Gentle {
        height: 40.0,
        seed: 9,
    }
    .ground(UVec3::new(3, 3, 2));
    let key = IVec3::new(1, 1, 1);
    let mut s = Samples::default();
    assert_eq!(g.gather(key, &mut s), Answer::Known(()));
    let first = origin(key);
    for p in [
        IVec3::new(-1, -1, -1),
        IVec3::new(32, 5, 7),
        IVec3::new(0, 31, 32),
        IVec3::new(16, 16, 16),
    ] {
        assert_eq!(Answer::Known(s.get(p)), g.voxel(first + p), "{p}");
    }
}

#[test]
fn gathering_beside_an_absent_chunk_is_unknown() {
    let mut g = Start::Flat { height: 40.0 }.ground(UVec3::new(2, 1, 2));
    g.store_mut().unload(IVec3::new(1, 0, 1));
    let mut s = Samples::default();
    assert_eq!(g.gather(IVec3::new(0, 0, 1), &mut s), Answer::Unknown);
    assert_eq!(g.gather(IVec3::new(1, 0, 1), &mut s), Answer::Unknown);
    assert_eq!(g.is_bare(IVec3::new(0, 0, 0)), Answer::Unknown);
}

#[test]
fn chunks_far_from_the_surface_are_bare() {
    let g = Start::Flat { height: 40.0 }.ground(UVec3::new(2, 2, 4));
    assert_eq!(g.is_bare(IVec3::new(0, 0, 3)), Answer::Known(true));
    assert_eq!(g.is_bare(IVec3::new(0, 0, 2)), Answer::Known(false));
    assert_eq!(g.is_bare(IVec3::new(1, 1, 1)), Answer::Known(false));
}
