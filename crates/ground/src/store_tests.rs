use super::*;

#[test]
fn a_chunk_the_same_all_through_is_kept_as_one_voxel() {
    let rock = vec![Voxel::solid(2); VOLUME];
    assert_eq!(Chunk::keep(&rock), Chunk::Uniform(Voxel::solid(2)));
    assert_eq!(Chunk::keep(&rock).bytes(), 0);
    let mut mixed = rock;
    mixed[5] = Voxel::AIR;
    let kept = Chunk::keep(&mixed);
    assert!(!kept.is_uniform());
    assert_eq!(kept.get(crate::voxel::place(5)), Voxel::AIR);
    assert_eq!(kept.get(crate::voxel::place(6)), Voxel::solid(2));
}

#[test]
fn the_wrong_number_of_voxels_reads_as_air_never_a_crash() {
    assert_eq!(Chunk::keep(&[]), Chunk::Uniform(Voxel::AIR));
    assert_eq!(
        Chunk::keep(&[Voxel::solid(1); 10]),
        Chunk::Uniform(Voxel::AIR)
    );
}

#[test]
fn copies_share_their_voxels() {
    let mut voxels = vec![Voxel::AIR; VOLUME];
    voxels[0] = Voxel::solid(0);
    let a = Chunk::keep(&voxels);
    let b = a.clone();
    match (&a, &b) {
        (Chunk::Mixed(x), Chunk::Mixed(y)) => assert!(Arc::ptr_eq(x, y)),
        _ => panic!("a mixed chunk"),
    }
}

#[test]
fn a_store_tells_absent_from_outside() {
    let mut store = Fixed::empty(UVec3::new(2, 2, 1));
    assert_eq!(store.slot(IVec3::ZERO), Slot::Absent);
    assert_eq!(store.slot(IVec3::new(2, 0, 0)), Slot::Outside);
    assert_eq!(store.slot(IVec3::new(0, 0, -1)), Slot::Outside);
    assert!(store.put(IVec3::new(1, 1, 0), Chunk::Uniform(Voxel::AIR)));
    assert!(!store.put(IVec3::new(5, 1, 0), Chunk::Uniform(Voxel::AIR)));
    assert_eq!(
        store.slot(IVec3::new(1, 1, 0)),
        Slot::Loaded(&Chunk::Uniform(Voxel::AIR))
    );
    store.unload(IVec3::new(1, 1, 0));
    assert_eq!(store.slot(IVec3::new(1, 1, 0)), Slot::Absent);
}

#[test]
fn keys_cover_the_box_in_order() {
    let all: Vec<IVec3> = keys(UVec3::new(2, 1, 2)).collect();
    assert_eq!(
        all,
        [
            IVec3::new(0, 0, 0),
            IVec3::new(1, 0, 0),
            IVec3::new(0, 0, 1),
            IVec3::new(1, 0, 1)
        ]
    );
}
