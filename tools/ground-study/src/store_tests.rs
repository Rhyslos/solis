use super::*;
use crate::shapes::{busy, fill, hill_with_cave};
use glam::IVec3;

fn chunks() -> Vec<Dense> {
    let mut out = Vec::new();
    for (key, shape) in [
        (
            IVec3::new(1, 1, 0),
            hill_with_cave as fn(glam::Vec3) -> Voxel,
        ),
        (IVec3::new(1, 0, 0), hill_with_cave),
        (IVec3::ZERO, busy),
    ] {
        let mut dense = Dense::filled(Voxel::AIR);
        fill(key, &mut dense, shape);
        out.push(dense);
    }
    out.push(Dense::filled(Voxel::AIR));
    out.push(Dense::filled(Voxel::solid(3)));
    out
}

fn round_trip<S: Store>(dense: &Dense) -> (S, Dense) {
    let kept = S::encode(dense);
    let mut back = Dense::filled(Voxel::solid(9));
    kept.decode(&mut back);
    (kept, back)
}

fn reads_back_the_same<S: Store>(lossless: bool) {
    for dense in chunks() {
        let (kept, back) = round_trip::<S>(&dense);
        for p in places() {
            assert_eq!(kept.get(p), back.get(p), "{} at {p}", S::NAME);
            let (was, now) = (dense.get(p), back.get(p));
            if lossless || !was.is_far() || !was.is_solid() {
                assert_eq!(was, now, "{} at {p}", S::NAME);
            } else {
                assert!(now.is_solid() && now.is_far(), "{} at {p}", S::NAME);
            }
        }
    }
}

#[test]
fn full_chunks_read_back_the_same() {
    reads_back_the_same::<Full>(true);
}

#[test]
fn sparse_chunks_read_back_the_same_but_deep_materials() {
    reads_back_the_same::<Sparse>(false);
}

#[test]
fn octrees_read_back_the_same() {
    reads_back_the_same::<Octree>(true);
}

#[test]
fn sparse_and_octree_chunks_are_smaller_where_the_surface_is_simple() {
    let hill = &chunks()[0];
    let full = Full::encode(hill).bytes();
    assert!(Sparse::encode(hill).bytes() < full);
    assert!(Octree::encode(hill).bytes() < full);
}

#[test]
fn a_uniform_octree_is_one_leaf() {
    let kept = Octree::encode(&Dense::filled(Voxel::solid(2)));
    assert_eq!(kept.words(), 1);
}
