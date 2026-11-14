//! What is kept in memory, read on a worker as it's wanted.

use super::Source;
use super::region;
use crate::brush::{Brush, Region};
use crate::ground::Chunk;
use crate::lod::{ChunkId, Kept, Levels, average};
use crate::store::Full;
use crate::voxel::{Dense, SIZE_I, Voxel, split};
use glam::{IVec2, IVec3};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Instant;

/// How the streamer chooses what to read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Policy {
    /// Whole regions (a level at a time) round what's wanted: one file
    /// read each, and everything in them kept.
    Regions,
    /// Just the chunks wanted, nearest first.
    Priority,
}

/// A chunk the frame wants, and how much: lower first.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Wanted {
    /// The chunk.
    pub id: ChunkId,
    /// Its place in line: lower is read first and let go of last.
    pub priority: f32,
    /// Whether it's drawn now (missing, it is late), or only read round
    /// what is (its neighbours' samples, units' ground).
    pub drawn: bool,
}

/// What streaming has done so far.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Stats {
    /// Chunks read.
    pub reads: usize,
    /// Bytes read from disk.
    pub bytes_read: usize,
    /// Chunks let go of.
    pub evicted: usize,
    /// Worker time spent reading, ms.
    pub worker_ms: f64,
    /// Drawn chunks missing when a frame wanted them, frames summed.
    pub late: usize,
    /// The most memory the kept chunks took, bytes.
    pub peak_bytes: usize,
    /// Regions written back.
    pub saves: usize,
}

enum Done {
    Read {
        chunks: Vec<(ChunkId, Chunk<Full>)>,
        asked: Vec<ChunkId>,
        bytes: usize,
        ms: f64,
    },
    Saved {
        region: IVec2,
        versions: Vec<(ChunkId, u64)>,
        ok: bool,
    },
}

/// The chunks kept, and the reading and writing of them.
pub struct Streamer<S: Source> {
    source: Arc<S>,
    policy: Policy,
    /// The most bytes of chunks kept; past it the least wanted go.
    pub budget: usize,
    kept: Kept,
    asked: HashSet<ChunkId>,
    pool: rayon::ThreadPool,
    send: Sender<Done>,
    done: Receiver<Done>,
    dirty: HashMap<ChunkId, u64>,
    saving: HashSet<IVec2>,
    edits: u64,
    priorities: HashMap<ChunkId, f32>,
    /// The most reads asked for in one frame.
    pub per_frame: usize,
    /// What it has done.
    pub stats: Stats,
}

impl<S: Source> Streamer<S> {
    /// A streamer reading from `source` with `workers` workers, keeping
    /// at most `budget` bytes of chunks (past it, the least wanted go),
    /// for a world `sizes` samples big at each level.
    pub fn new(
        source: S,
        policy: Policy,
        budget: usize,
        workers: usize,
        sizes: Vec<IVec3>,
    ) -> Result<Self, String> {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(workers.max(1))
            .build()
            .map_err(|e| e.to_string())?;
        let (send, done) = channel();
        Ok(Self {
            source: Arc::new(source),
            policy,
            budget,
            kept: Kept::new(sizes),
            asked: HashSet::new(),
            pool,
            send,
            done,
            dirty: HashMap::new(),
            saving: HashSet::new(),
            edits: 0,
            priorities: HashMap::new(),
            per_frame: 64,
            stats: Stats::default(),
        })
    }

    /// The kept chunks, to mesh from.
    pub fn kept(&self) -> &Kept {
        &self.kept
    }

    /// The source.
    pub fn source(&self) -> &S {
        &self.source
    }

    /// Whether every read and write asked for is done.
    pub fn idle(&self) -> bool {
        self.asked.is_empty() && self.saving.is_empty()
    }

    /// One frame: takes what the workers finished, asks for what's
    /// wanted and missing (nearest first), and lets go of the least
    /// wanted while over budget.
    pub fn update(&mut self, wanted: &[Wanted]) {
        while let Ok(done) = self.done.try_recv() {
            self.finish(done);
        }
        let mut order: Vec<&Wanted> = wanted.iter().collect();
        order.sort_by(|a, b| a.priority.total_cmp(&b.priority));
        self.priorities.clear();
        for w in &order {
            self.priorities.entry(w.id).or_insert(w.priority);
            if w.drawn && !self.kept.contains(w.id) && self.in_world(w.id) {
                self.stats.late += 1;
            }
        }
        if self.policy == Policy::Regions {
            self.want_whole_regions();
        }
        let mut asked = 0;
        for w in order {
            if asked >= self.per_frame {
                break;
            }
            if self.kept.contains(w.id) || self.asked.contains(&w.id) || !self.in_world(w.id) {
                continue;
            }
            asked += self.ask(w.id);
        }
        self.evict();
        self.stats.peak_bytes = self.stats.peak_bytes.max(self.kept.bytes());
    }

    /// Whether `id` is a chunk of the world.
    fn in_world(&self, (level, key): ChunkId) -> bool {
        let size = self.kept.size(level);
        key.cmpge(IVec3::ZERO).all() && (key * SIZE_I).cmplt(size).all()
    }

    /// With whole regions, every chunk of a region's level counts as
    /// wanted when any of them is.
    fn want_whole_regions(&mut self) {
        let mut groups: HashMap<(IVec2, u32), f32> = HashMap::new();
        for (id, p) in &self.priorities {
            let g = groups.entry((region::region_of(*id), id.0)).or_insert(*p);
            *g = g.min(*p);
        }
        for ((r, level), p) in groups {
            for key in region::chunks(r, level) {
                self.priorities.entry((level, key)).or_insert(p);
            }
        }
    }

    /// Asks a worker for `id` (and, reading whole regions, the rest of
    /// its region's level); returns how many reads that was.
    fn ask(&mut self, id: ChunkId) -> usize {
        let source = Arc::clone(&self.source);
        let send = self.send.clone();
        let asked: Vec<ChunkId> = match self.policy {
            Policy::Priority => vec![id],
            Policy::Regions => region::chunks(region::region_of(id), id.0)
                .into_iter()
                .map(|k| (id.0, k))
                .filter(|c| !self.asked.contains(c))
                .collect(),
        };
        self.asked.extend(asked.iter().copied());
        let policy = self.policy;
        let n = asked.len();
        self.pool.spawn(move || {
            let clock = Instant::now();
            let (chunks, bytes) = match policy {
                Policy::Priority => source
                    .chunk(id)
                    .map(|(c, b)| (vec![(id, c)], b))
                    .unwrap_or_default(),
                Policy::Regions => source
                    .region(region::region_of(id), id.0)
                    .unwrap_or_default(),
            };
            let ms = clock.elapsed().as_secs_f64() * 1000.0;
            let _ = send.send(Done::Read {
                chunks,
                asked,
                bytes,
                ms,
            });
        });
        n
    }

    fn finish(&mut self, done: Done) {
        match done {
            Done::Read {
                chunks,
                asked,
                bytes,
                ms,
            } => {
                for id in asked {
                    self.asked.remove(&id);
                }
                self.stats.bytes_read += bytes;
                self.stats.worker_ms += ms;
                for (id, chunk) in chunks {
                    // A chunk sculpted while it was being read keeps the
                    // sculpting.
                    if !self.kept.contains(id) {
                        self.kept.insert(id, chunk);
                        self.stats.reads += 1;
                    }
                }
            }
            Done::Saved {
                region,
                versions,
                ok,
            } => {
                self.saving.remove(&region);
                if ok {
                    self.stats.saves += 1;
                    for (id, version) in versions {
                        if self.dirty.get(&id) == Some(&version) {
                            self.dirty.remove(&id);
                        }
                    }
                }
            }
        }
    }

    /// Lets go of the least wanted chunks while over budget; a sculpted
    /// one is written back first and let go of later.
    fn evict(&mut self) {
        let mut bytes = self.kept.bytes();
        if bytes <= self.budget {
            return;
        }
        let mut order: Vec<(f32, ChunkId)> = self
            .kept
            .ids()
            .map(|id| (self.priorities.get(&id).copied().unwrap_or(f32::MAX), id))
            .collect();
        order.sort_by(|a, b| b.0.total_cmp(&a.0).then(b.1.0.cmp(&a.1.0)));
        for (_, id) in order {
            if bytes <= self.budget {
                break;
            }
            if self.dirty.contains_key(&id) {
                self.save_region(region::region_of(id));
                continue;
            }
            if let Some(chunk) = self.kept.remove(id) {
                bytes = bytes.saturating_sub(chunk_bytes(&chunk));
                self.stats.evicted += 1;
            }
        }
    }

    /// Writes the region `r`'s sculpted chunks back on a worker.
    fn save_region(&mut self, r: IVec2) {
        if self.saving.contains(&r) {
            return;
        }
        let mut changed = Vec::new();
        let mut versions = Vec::new();
        for (id, version) in &self.dirty {
            if region::region_of(*id) == r
                && let Some(chunk) = self.kept.get(*id)
            {
                changed.push((*id, chunk.clone()));
                versions.push((*id, *version));
            }
        }
        if changed.is_empty() {
            return;
        }
        self.saving.insert(r);
        let source = Arc::clone(&self.source);
        let send = self.send.clone();
        self.pool.spawn(move || {
            let ok = source.save(r, &changed).is_ok();
            let _ = send.send(Done::Saved {
                region: r,
                versions,
                ok,
            });
        });
    }

    /// Writes every sculpted chunk back and waits until it's done.
    pub fn flush(&mut self) {
        let regions: HashSet<IVec2> = self.dirty.keys().map(|id| region::region_of(*id)).collect();
        for r in regions {
            self.save_region(r);
        }
        self.wait();
    }

    /// Waits until every read and write asked for is done.
    pub fn wait(&mut self) {
        while !self.idle() {
            match self.done.recv() {
                Ok(done) => self.finish(done),
                Err(_) => break,
            }
        }
    }

    /// Whether a chunk has sculpting not yet written back.
    pub fn is_dirty(&self, id: ChunkId) -> bool {
        self.dirty.contains_key(&id)
    }

    /// Reads `id` now, on this thread, if it isn't kept.
    fn need(&mut self, id: ChunkId) -> Result<(), String> {
        if self.kept.contains(id) {
            return Ok(());
        }
        let (chunk, bytes) = self.source.chunk(id)?;
        self.stats.reads += 1;
        self.stats.bytes_read += bytes;
        self.kept.insert(id, chunk);
        Ok(())
    }

    /// Sculpts level 0 with `brush`, reading what it reaches first, and
    /// averages levels 1 to 3 again where it changed; returns the chunks
    /// changed, level by level. They stay kept until written back.
    pub fn edit(&mut self, brush: &Brush) -> Result<Vec<ChunkId>, String> {
        let size = self.kept.size(0);
        let (lo, hi) = brush.reach();
        let (lo, hi) = (lo.max(IVec3::ZERO), hi.min(size));
        if lo.cmpge(hi).any() {
            return Ok(Vec::new());
        }
        self.need_box(0, lo - 1, hi + 1)?;
        let mut box_ = Region::new(lo, (hi - lo).as_uvec3());
        box_.fill(|p| self.kept.voxel(0, p));
        let Some((mut lo, mut hi)) = brush.apply(&mut box_) else {
            return Ok(Vec::new());
        };
        let mut changed = Vec::new();
        for key in crate::ground::boxed(split(lo).0, split(hi - 1).0 + 1) {
            let mut dense = self.dense((0, key));
            if box_.write_into(key, &mut dense) {
                self.keep_edited((0, key), &dense);
                changed.push((0, key));
            }
        }
        for level in 1..region::LEVELS {
            lo = lo.div_euclid(IVec3::splat(2));
            hi = (hi.div_euclid(IVec3::splat(2)) + 1).min(self.kept.size(level));
            self.need_box(level - 1, lo * 2 - 1, hi * 2 + 1)?;
            self.need_box(level, lo, hi)?;
            let mut any = false;
            for key in crate::ground::boxed(split(lo).0, split(hi - 1).0 + 1) {
                let mut dense = self.dense((level, key));
                let mut differs = false;
                let from = (key * SIZE_I).max(lo);
                let to = ((key + 1) * SIZE_I).min(hi);
                for p in crate::ground::boxed(from, to) {
                    let v = average(|q| self.kept.voxel(level - 1, q), p);
                    let local = (p - key * SIZE_I).as_uvec3();
                    if dense.get(local) != v {
                        dense.set(local, v);
                        differs = true;
                    }
                }
                if differs {
                    self.keep_edited((level, key), &dense);
                    changed.push((level, key));
                    any = true;
                }
            }
            if !any {
                break;
            }
        }
        Ok(changed)
    }

    fn need_box(&mut self, level: u32, lo: IVec3, hi: IVec3) -> Result<(), String> {
        let size = self.kept.size(level);
        let (lo, hi) = (lo.max(IVec3::ZERO), hi.min(size));
        if lo.cmpge(hi).any() {
            return Ok(());
        }
        for key in crate::ground::boxed(split(lo).0, split(hi - 1).0 + 1) {
            self.need((level, key))?;
        }
        Ok(())
    }

    fn dense(&self, id: ChunkId) -> Dense {
        let mut dense = Dense::filled(Voxel::AIR);
        if let Some(chunk) = self.kept.get(id) {
            chunk.decode(&mut dense);
        }
        dense
    }

    fn keep_edited(&mut self, id: ChunkId, dense: &Dense) {
        self.edits += 1;
        self.kept.insert(id, Chunk::keep(dense));
        self.dirty.insert(id, self.edits);
    }

    /// How many chunks are kept.
    pub fn kept_count(&self) -> usize {
        self.kept.len()
    }
}

fn chunk_bytes(chunk: &Chunk<Full>) -> usize {
    use crate::store::Store;
    std::mem::size_of::<Chunk<Full>>()
        + match chunk {
            Chunk::Uniform(_) => 0,
            Chunk::Mixed(s) => s.bytes(),
        }
}

/// The chunks a frame wants round `place` (metres) for the layout's
/// `columns`: each column's chunks, drawn, and two columns of each level
/// round them: one for their samples, and one read ahead, since rings
/// move two columns at a time. Nearer is wanted first.
pub fn wanted_for(columns: &[crate::lod::layout::Column], place: glam::Vec2) -> Vec<Wanted> {
    let mut out: HashMap<ChunkId, Wanted> = HashMap::new();
    for c in columns {
        let size = crate::lod::chunk_metres(c.level) as f32;
        let middle = (c.key.as_vec2() + 0.5) * size;
        let priority = middle.distance(place) / size;
        for dy in -2..=2 {
            for dx in -2..=2 {
                let key = c.key + IVec2::new(dx, dy);
                if key.cmplt(IVec2::ZERO).any() {
                    continue;
                }
                let drawn = dx == 0 && dy == 0;
                for z in 0..crate::world::chunks_high(c.level) as i32 {
                    let id = (c.level, key.extend(z));
                    let ring = dx.abs().max(dy.abs()) as f32;
                    let w = out.entry(id).or_insert(Wanted {
                        id,
                        priority: priority + ring,
                        drawn,
                    });
                    w.priority = w.priority.min(priority + ring);
                    if drawn {
                        w.drawn = true;
                        w.priority = w.priority.min(priority);
                    }
                }
            }
        }
    }
    let mut list: Vec<Wanted> = out.into_values().collect();
    list.sort_by(|a, b| a.priority.total_cmp(&b.priority).then(a.id.0.cmp(&b.id.0)));
    list
}
