use super::*;
use glam::UVec3;
use std::sync::Mutex;

type Jobs = Arc<Mutex<Vec<Box<dyn FnOnce() + Send>>>>;

/// Chunks whose jobs are kept in a list, to run when the test says.
fn held() -> (GroundChunks, Jobs) {
    let jobs: Jobs = Arc::default();
    let list = Arc::clone(&jobs);
    let chunks = GroundChunks::with_spawn(Arc::new(move |job| list.lock().unwrap().push(job)));
    (chunks, jobs)
}

fn run(jobs: &Jobs) -> usize {
    let taken: Vec<_> = std::mem::take(&mut *jobs.lock().unwrap());
    let n = taken.len();
    for job in taken {
        job();
    }
    n
}

fn flat(extent: UVec3) -> LoadedGround {
    LoadedGround {
        asset: AssetId(reflect::Id::from_raw(5).unwrap()),
        path: "hills.ground.lst".into(),
        settings: Settings::default(),
        ground: Arc::new(ground::Start::Flat { height: 40.0 }.ground(extent)),
    }
}

/// Meshing never runs on the frame: the frame only copies samples and
/// hands them out; nothing is made until a worker runs the jobs.
#[test]
fn the_frame_only_copies_and_hands_out() {
    let (mut chunks, jobs) = held();
    chunks.set(Some(flat(UVec3::new(2, 2, 4))));
    assert_eq!(chunks.left(), 12, "the surface's layer and one either side");
    assert!(chunks.frame(Vec3::ZERO));
    assert_eq!(jobs.lock().unwrap().len(), 12);
    assert!(chunks.take_fresh().fresh.is_empty(), "nothing made on the frame");
    assert_eq!(run(&jobs), 12);
    assert!(chunks.frame(Vec3::ZERO) || chunks.left() == 0);
    let meshes = chunks.take_fresh();
    assert_eq!(meshes.fresh.len(), 12);
    assert_eq!(meshes.fresh.iter().filter(|m| !m.is_empty()).count(), 4);
    assert_eq!(chunks.left(), 0);
    assert!(chunks.timing.unwrap().1.is_some());
}

#[test]
fn the_nearest_chunks_go_first() {
    let (mut chunks, jobs) = held();
    chunks.set(Some(flat(UVec3::new(8, 8, 2))));
    let camera = Vec3::new(250.0, 250.0, 40.0);
    chunks.frame(camera);
    run(&jobs);
    chunks.frame(camera);
    let first: Vec<IVec3> = chunks.take_fresh().fresh.iter().map(|m| m.key).collect();
    assert_eq!(first.len(), IN_FLIGHT);
    for key in &first {
        assert!(key.x >= 4 && key.y >= 4, "{key} is far from the camera");
    }
}

#[test]
fn a_mesh_made_from_older_voxels_is_thrown_away() {
    let (mut chunks, jobs) = held();
    chunks.set(Some(flat(UVec3::new(1, 1, 2))));
    chunks.frame(Vec3::ZERO);
    chunks.remake(IVec3::new(0, 0, 1));
    run(&jobs);
    chunks.frame(Vec3::ZERO);
    let first = chunks.take_fresh().fresh;
    assert_eq!(first.len(), 1, "only the chunk not made again");
    assert_eq!(first[0].key, IVec3::new(0, 0, 0));
    run(&jobs);
    chunks.frame(Vec3::ZERO);
    let again = chunks.take_fresh().fresh;
    assert_eq!(again.len(), 1);
    assert_eq!(again[0].key, IVec3::new(0, 0, 1));
}

#[test]
fn another_ground_starts_afresh_and_old_meshes_are_thrown_away() {
    let (mut chunks, jobs) = held();
    chunks.set(Some(flat(UVec3::new(1, 1, 2))));
    chunks.frame(Vec3::ZERO);
    let first = chunks.take_fresh().revision;
    chunks.set(None);
    run(&jobs);
    chunks.frame(Vec3::ZERO);
    let after = chunks.take_fresh();
    assert!(after.fresh.is_empty());
    assert!(after.revision > first);
}

#[test]
fn colliders_arrive_for_play() {
    let (mut chunks, jobs) = held();
    chunks.set(Some(flat(UVec3::new(1, 1, 2))));
    let (needed, ready) = chunks.for_new_play();
    assert_eq!(needed.len(), 2);
    assert!(ready.is_empty());
    chunks.frame(Vec3::ZERO);
    run(&jobs);
    chunks.frame(Vec3::ZERO);
    let arrived = chunks.take_for_play();
    assert_eq!(arrived.len(), 2);
    assert_eq!(arrived.iter().filter(|(_, s)| s.is_some()).count(), 1);
    assert!(chunks.take_for_play().is_empty());
}
