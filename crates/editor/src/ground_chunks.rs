//! The level's ground in the editor (Tools 1b, G1): the ground loaded,
//! and its chunks meshed and made solid on workers, never on the frame
//! (rule 10).
//!
//! Each frame ([`GroundChunks::frame`]):
//!
//! 1. Finished chunks come back by message. A chunk carries the
//!    generation it was asked for with; one made again since (a newer
//!    generation) is thrown away.
//! 2. The chunks still to make are sorted nearest the camera first, so a
//!    big ground appears around the camera first.
//! 3. A few are handed out: the frame only **copies** each one's samples
//!    (its voxels and one more all round, `ground::Ground::gather`) and
//!    hands them to a job on the shared pool, which meshes them (the
//!    Smooth look) and builds their collider's shape
//!    (`world::prepare_ground_chunk`).
//!
//! The app takes the meshes that arrived ([`GroundChunks::take_fresh`])
//! and uploads them; the running game takes the colliders
//! ([`GroundChunks::take_for_play`]) and adds them on its own thread.

use glam::{IVec3, Vec3};
use ground::{Answer, ChunkMesh, Ground, Samples, Settings};
use reflect::AssetId;
use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};

/// How many chunks may be on the workers at once: enough to keep every
/// core busy, few enough that a frame copies little (each copy is about
/// 78 KB).
const IN_FLIGHT: usize = 24;

/// How a job is handed to the workers: the shared pool in the program, a
/// list in tests.
pub(crate) type Spawn = Arc<dyn Fn(Box<dyn FnOnce() + Send>) + Send + Sync>;

/// A ground the editor has: its settings file, read.
#[derive(Debug, Clone)]
pub struct LoadedGround {
    /// Its settings file's asset.
    pub asset: AssetId,
    /// Its settings file, inside the assets.
    pub path: String,
    /// Its settings.
    pub settings: Settings,
    /// Its voxels.
    pub ground: Arc<Ground>,
}

/// A chunk made on a worker.
struct Done {
    key: IVec3,
    generation: u64,
    mesh: Arc<ChunkMesh>,
    shape: Option<physics::PreparedShape>,
}

/// What the app draws of the ground: whose it is, and the chunks that
/// arrived since it last asked.
#[derive(Debug, Clone, Default)]
pub struct GroundMeshes {
    /// Goes up when the ground is another (or none): everything uploaded
    /// before is let go.
    pub revision: u64,
    /// The kinds' colours, red, green and blue.
    pub colours: Vec<[u8; 3]>,
    /// Chunks made since last asked (an empty mesh: a chunk with nothing
    /// to draw any more).
    pub fresh: Vec<Arc<ChunkMesh>>,
}

/// The ground loaded in the editor, and its chunks on their way.
pub(crate) struct GroundChunks {
    loaded: Option<LoadedGround>,
    /// The asset asked for and not answered yet.
    pub(crate) asked: Option<AssetId>,
    revision: u64,
    generations: BTreeMap<[i32; 3], u64>,
    next_generation: u64,
    waiting: Vec<IVec3>,
    out: HashSet<[i32; 3]>,
    needed: Vec<IVec3>,
    shapes: BTreeMap<[i32; 3], Option<physics::PreparedShape>>,
    fresh: Vec<Arc<ChunkMesh>>,
    for_play: Vec<(IVec3, Option<physics::PreparedShape>)>,
    sender: Sender<Done>,
    receiver: Receiver<Done>,
    spawn: Spawn,
    /// When the ground was set, and when its last chunk arrived.
    pub(crate) timing: Option<(std::time::Instant, Option<std::time::Duration>)>,
}

impl Default for GroundChunks {
    fn default() -> Self {
        Self::with_spawn(Arc::new(|job| rayon::spawn(job)))
    }
}

impl GroundChunks {
    /// No ground, handing jobs out with `spawn`.
    pub(crate) fn with_spawn(spawn: Spawn) -> Self {
        let (sender, receiver) = channel();
        Self {
            loaded: None,
            asked: None,
            revision: 0,
            generations: BTreeMap::new(),
            next_generation: 0,
            waiting: Vec::new(),
            out: HashSet::new(),
            needed: Vec::new(),
            shapes: BTreeMap::new(),
            fresh: Vec::new(),
            for_play: Vec::new(),
            sender,
            receiver,
            spawn,
            timing: None,
        }
    }

    /// The ground loaded, if any.
    pub(crate) fn loaded(&self) -> Option<&LoadedGround> {
        self.loaded.as_ref()
    }

    /// Makes `loaded` the ground (or none): every chunk that may have
    /// triangles is made again, nearest the camera first.
    pub(crate) fn set(&mut self, loaded: Option<LoadedGround>) {
        self.revision += 1;
        self.shapes.clear();
        self.fresh.clear();
        self.for_play.clear();
        self.waiting.clear();
        self.needed.clear();
        if let Some(l) = &loaded {
            let ground = &l.ground;
            self.needed = ground::store::keys(ground.extent())
                .filter(|k| ground.is_bare(*k) != Answer::Known(true))
                .collect();
            for key in &self.needed {
                self.next_generation += 1;
                self.generations
                    .insert(key.to_array(), self.next_generation);
                self.shapes.insert(key.to_array(), None);
            }
            self.waiting = self.needed.clone();
            self.timing = Some((std::time::Instant::now(), None));
        } else {
            self.timing = None;
        }
        self.loaded = loaded;
    }

    /// Makes the chunk at `key` again (a newer generation): a mesh of it
    /// still on its way is thrown away when it arrives.
    pub(crate) fn remake(&mut self, key: IVec3) {
        self.next_generation += 1;
        self.generations
            .insert(key.to_array(), self.next_generation);
        if !self.waiting.contains(&key) {
            self.waiting.push(key);
        }
    }

    /// One frame's part: takes in what the workers finished, then hands
    /// out the chunks nearest `camera`, copying their samples. Returns
    /// whether anything is still on its way.
    pub(crate) fn frame(&mut self, camera: Vec3) -> bool {
        self.take_in();
        let Some(loaded) = &self.loaded else {
            return false;
        };
        if self.waiting.is_empty() {
            return !self.out.is_empty();
        }
        let free = IN_FLIGHT.saturating_sub(self.out.len());
        if free > 0 {
            // Farthest first, so the nearest are popped from the end.
            let middle = |k: &IVec3| {
                (ground::store::origin(*k).as_vec3() + Vec3::splat(16.0)).distance_squared(camera)
            };
            self.waiting.sort_by(|a, b| {
                middle(b)
                    .total_cmp(&middle(a))
                    .then(b.to_array().cmp(&a.to_array()))
            });
            let ground = Arc::clone(&loaded.ground);
            for _ in 0..free {
                let Some(key) = self.waiting.pop() else {
                    break;
                };
                if self.out.contains(&key.to_array()) {
                    // Still being made from older voxels: asked again
                    // once that one's back.
                    self.waiting.insert(0, key);
                    continue;
                }
                self.hand_out(&ground, key);
            }
        }
        true
    }

    /// Copies the chunk at `key`'s samples and hands them to a worker.
    fn hand_out(&mut self, ground: &Ground, key: IVec3) {
        let mut samples = Samples::default();
        if ground.gather(key, &mut samples) != Answer::Known(()) {
            return;
        }
        let generation = self
            .generations
            .get(&key.to_array())
            .copied()
            .unwrap_or_default();
        let sender = self.sender.clone();
        self.out.insert(key.to_array());
        (self.spawn)(Box::new(move || {
            let mesh = MESHER.with(|m| m.borrow_mut().mesh(&samples));
            let shape = world::prepare_ground_chunk(&mesh);
            let _ = sender.send(Done {
                key,
                generation,
                mesh: Arc::new(mesh),
                shape,
            });
        }));
    }

    /// Takes in the chunks the workers finished, throwing away those made
    /// again since they were asked for.
    fn take_in(&mut self) {
        while let Ok(done) = self.receiver.try_recv() {
            let key = done.key.to_array();
            self.out.remove(&key);
            if self.generations.get(&key) != Some(&done.generation) {
                continue;
            }
            self.shapes.insert(key, done.shape.clone());
            self.for_play.push((done.key, done.shape));
            self.fresh.push(done.mesh);
        }
        if let Some((started, finished @ None)) = &mut self.timing
            && self.waiting.is_empty()
            && self.out.is_empty()
        {
            *finished = Some(started.elapsed());
        }
    }

    /// What the app draws: the chunks that arrived since it last asked.
    pub(crate) fn take_fresh(&mut self) -> GroundMeshes {
        GroundMeshes {
            revision: self.revision,
            colours: self
                .loaded
                .as_ref()
                .map(|l| l.settings.kinds.iter().map(|k| k.colour).collect())
                .unwrap_or_default(),
            fresh: std::mem::take(&mut self.fresh),
        }
    }

    /// The colliders that arrived since the running game last took them.
    pub(crate) fn take_for_play(&mut self) -> Vec<(IVec3, Option<physics::PreparedShape>)> {
        std::mem::take(&mut self.for_play)
    }

    /// For a new play: the chunks that need a collider, and those made so
    /// far.
    pub(crate) fn for_new_play(
        &mut self,
    ) -> (Vec<IVec3>, Vec<(IVec3, Option<physics::PreparedShape>)>) {
        self.for_play.clear();
        let ready = self
            .shapes
            .iter()
            .filter(|(_, s)| s.is_some())
            .map(|(k, s)| (IVec3::from_array(*k), s.clone()))
            .collect();
        (self.needed.clone(), ready)
    }

    /// How many chunks are still to make or on their way.
    pub(crate) fn left(&self) -> usize {
        self.waiting.len() + self.out.len()
    }
}

thread_local! {
    /// Each worker's mesher, kept between chunks so its memory is reused.
    static MESHER: std::cell::RefCell<ground::Mesher> = std::cell::RefCell::default();
}

#[cfg(test)]
#[path = "ground_chunks_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "ground_scale_tests.rs"]
mod scale_tests;
