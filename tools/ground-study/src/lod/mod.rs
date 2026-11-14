//! Far ground drawn coarser: levels of detail for the open-worlds study.
//!
//! Level 0 is the ground as sculpted, a voxel a metre. Each level up
//! has voxels twice as wide, made by averaging the level below, so a
//! chunk of 32 × 32 × 32 voxels at level `L` covers `32 · 2^L` metres. A
//! level's sample `i` sits at `i · 2^L` metres, so every other sample of
//! a level is one of the next level's, and chunk edges of every level
//! line up.
//!
//! - [`Levels`]: what the meshers read, a level's voxels.
//! - [`Pyramid`]: every level kept in memory, made by averaging, and
//!   kept up to date when the ground is sculpted.
//! - [`Kept`]: just the chunks wanted, as a streamer holds them.
//! - [`layout`]: which size of chunk is drawn where.
//! - [`nets`] and [`blocky`]: the looks' meshers at any level, with
//!   their ways of closing seams; [`skirts`]; [`cracks`] finds them.

pub mod blocky;
pub mod cracks;
pub mod layout;
pub mod nets;
pub mod skirts;

use crate::brush::Brush;
use crate::ground::{Chunk, Ground, Samples};
use crate::store::Full;
use crate::voxel::{Dense, SIZE_I, Voxel, place, split};
use glam::{IVec3, UVec3};
use rayon::prelude::*;
use std::collections::HashMap;

/// The width of a voxel of `level`, in metres.
pub fn voxel(level: u32) -> f32 {
    (1u32 << level) as f32
}

/// The width of a chunk of `level`, in metres.
pub fn chunk_metres(level: u32) -> i32 {
    SIZE_I << level
}

/// Where a ground's voxels come from, level by level.
pub trait Levels: Sync {
    /// The voxel at sample `index` of `level`; past the world's edges,
    /// the edge's own.
    fn voxel(&self, level: u32, index: IVec3) -> Voxel;

    /// The world's size in samples of `level`.
    fn size(&self, level: u32) -> IVec3;

    /// The samples the chunk `key` of `level` is meshed from (its own
    /// and two more all round). `scratch` is a chunk to decode into.
    fn gather(&self, level: u32, key: IVec3, scratch: &mut Dense, out: &mut Samples) {
        let _ = scratch;
        out.origin = key * SIZE_I;
        let apron = crate::ground::APRON;
        for z in -apron..SIZE_I + apron {
            for y in -apron..SIZE_I + apron {
                for x in -apron..SIZE_I + apron {
                    let p = IVec3::new(x, y, z);
                    out.set(p, self.voxel(level, out.origin + p));
                }
            }
        }
    }
}

/// The tent-filtered average of `fine`'s 27 voxels round `2 · g`: the
/// coarse level's sample `g`. The middle counts half along each axis,
/// its neighbours a quarter. Its kind is that of the solid voxel nearest
/// the surface, so a painted surface keeps its colour far away.
pub fn average(fine: impl Fn(IVec3) -> Voxel, g: IVec3) -> Voxel {
    const W: [f32; 3] = [0.25, 0.5, 0.25];
    let middle = g * 2;
    let mut sum = 0.0;
    let mut surface: Option<Voxel> = None;
    for (dz, wz) in W.iter().enumerate() {
        let mut plane = 0.0;
        for (dy, wy) in W.iter().enumerate() {
            let mut row = 0.0;
            for (dx, wx) in W.iter().enumerate() {
                let v = fine(middle + IVec3::new(dx as i32 - 1, dy as i32 - 1, dz as i32 - 1));
                row += wx * v.distance();
                if v.is_solid() && surface.is_none_or(|s| v.distance > s.distance) {
                    surface = Some(v);
                }
            }
            plane += wy * row;
        }
        sum += wz * plane;
    }
    let material = surface.map_or(0, |v| v.material);
    if sum == Voxel::solid(0).distance() {
        Voxel::solid(material)
    } else if sum == Voxel::AIR.distance() {
        Voxel::AIR
    } else {
        Voxel::at(sum / 2.0, material)
    }
}

/// The level above `fine`: each voxel the average of the 27 round its
/// place (see [`average`]), made on every core.
pub fn coarser(fine: &Ground<Full>) -> Ground<Full> {
    let across = fine.chunks_across();
    let up = UVec3::new(
        across.x.div_ceil(2),
        across.y.div_ceil(2),
        across.z.div_ceil(2),
    );
    Ground::build(up, |key, dense| {
        if let Some(voxel) = uniform_below(fine, key) {
            dense.voxels_mut().fill(voxel);
            return;
        }
        // The 65 fine voxels each way the chunk's samples average.
        let start = key * SIZE_I * 2 - 1;
        const SPAN: i32 = 2 * SIZE_I + 1;
        let block: Vec<Voxel> = (0..SPAN * SPAN * SPAN)
            .into_par_iter()
            .with_min_len(4096)
            .map(|i| {
                let p = IVec3::new(i % SPAN, (i / SPAN) % SPAN, i / (SPAN * SPAN));
                fine.get(start + p)
            })
            .collect();
        let at = |p: IVec3| {
            let q = p - start;
            block[(q.x + SPAN * (q.y + SPAN * q.z)) as usize]
        };
        for (i, v) in dense.voxels_mut().iter_mut().enumerate() {
            *v = average(at, key * SIZE_I + place(i).as_ivec3());
        }
    })
}

/// The one voxel every fine chunk under the coarse chunk `key` is made
/// of, when they are all the same all through.
fn uniform_below(fine: &Ground<Full>, key: IVec3) -> Option<Voxel> {
    let last = fine.chunks_across().as_ivec3() - 1;
    let mut found: Option<Voxel> = None;
    for k in crate::ground::boxed(key * 2 - 1, key * 2 + 2) {
        match fine.chunk(k.clamp(IVec3::ZERO, last)) {
            Some(Chunk::Uniform(v)) if found.is_none_or(|f| f == *v) => found = Some(*v),
            _ => return None,
        }
    }
    found
}

/// Every level of a ground in memory, each made from the one below.
pub struct Pyramid {
    levels: Vec<Ground<Full>>,
}

impl Pyramid {
    /// `base` and `count - 1` coarser levels above it.
    pub fn build(base: Ground<Full>, count: u32) -> Self {
        let mut levels = vec![base];
        for _ in 1..count {
            let next = levels.last().map(coarser);
            levels.extend(next);
        }
        Self { levels }
    }

    /// How many levels it has.
    pub fn count(&self) -> u32 {
        self.levels.len() as u32
    }

    /// One level.
    pub fn level(&self, level: u32) -> Option<&Ground<Full>> {
        self.levels.get(level as usize)
    }

    /// Sculpts level 0 with `brush` and averages every level above again
    /// where it changed; returns, level by level, the chunks whose meshes
    /// changed.
    pub fn apply(&mut self, brush: &Brush) -> Vec<Vec<IVec3>> {
        let mut out = vec![Vec::new(); self.levels.len()];
        let Some((mut lo, mut hi)) = self.levels[0].apply_changed(brush) else {
            return out;
        };
        out[0] = self.levels[0].meshes_reaching(lo, hi);
        for level in 1..self.levels.len() {
            let (below, above) = self.levels.split_at_mut(level);
            let fine = &below[level - 1];
            let coarse = &mut above[0];
            // A coarse sample averages fine ones within one of twice it.
            lo = lo.div_euclid(IVec3::splat(2));
            hi = hi.div_euclid(IVec3::splat(2)) + 1;
            let changed = coarse.rewrite(lo, hi, |g| average(|p| fine.get(p), g));
            if changed.is_empty() {
                break;
            }
            out[level] = coarse.meshes_reaching(lo, hi);
        }
        out
    }

    /// The memory its voxels take, in bytes, level by level.
    pub fn bytes(&self) -> Vec<usize> {
        self.levels.iter().map(Ground::bytes).collect()
    }
}

impl Levels for Pyramid {
    fn voxel(&self, level: u32, index: IVec3) -> Voxel {
        self.levels
            .get(level as usize)
            .map_or(Voxel::AIR, |g| g.get(index))
    }

    fn size(&self, level: u32) -> IVec3 {
        self.levels
            .get(level as usize)
            .map_or(IVec3::ZERO, Ground::size)
    }

    fn gather(&self, level: u32, key: IVec3, scratch: &mut Dense, out: &mut Samples) {
        if let Some(g) = self.levels.get(level as usize) {
            g.gather(key, scratch, out);
        }
    }
}

/// A chunk of some level: the level, and its key in that level's chunks.
pub type ChunkId = (u32, IVec3);

/// Just some chunks of each level, as a streamer holds them: a chunk
/// not kept reads as air. Sizes are the whole world's.
pub struct Kept {
    chunks: HashMap<ChunkId, Chunk<Full>>,
    sizes: Vec<IVec3>,
}

impl Kept {
    /// Nothing kept yet, of a world `sizes` samples big at each level.
    pub fn new(sizes: Vec<IVec3>) -> Self {
        Self {
            chunks: HashMap::new(),
            sizes,
        }
    }

    /// Keeps `chunk` as `id`.
    pub fn insert(&mut self, id: ChunkId, chunk: Chunk<Full>) {
        self.chunks.insert(id, chunk);
    }

    /// Lets go of `id`.
    pub fn remove(&mut self, id: ChunkId) -> Option<Chunk<Full>> {
        self.chunks.remove(&id)
    }

    /// The chunk `id`, if kept.
    pub fn get(&self, id: ChunkId) -> Option<&Chunk<Full>> {
        self.chunks.get(&id)
    }

    /// Whether `id` is kept.
    pub fn contains(&self, id: ChunkId) -> bool {
        self.chunks.contains_key(&id)
    }

    /// Every kept chunk's id.
    pub fn ids(&self) -> impl Iterator<Item = ChunkId> + '_ {
        self.chunks.keys().copied()
    }

    /// How many chunks are kept.
    pub fn len(&self) -> usize {
        self.chunks.len()
    }

    /// Whether nothing is kept.
    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    /// The memory kept chunks take, in bytes.
    pub fn bytes(&self) -> usize {
        use crate::store::Store;
        self.chunks
            .values()
            .map(|c| match c {
                Chunk::Uniform(_) => std::mem::size_of::<Chunk<Full>>(),
                Chunk::Mixed(s) => std::mem::size_of::<Chunk<Full>>() + s.bytes(),
            })
            .sum::<usize>()
            + self.chunks.capacity() * std::mem::size_of::<(ChunkId, Chunk<Full>)>()
    }
}

impl Levels for Kept {
    fn voxel(&self, level: u32, index: IVec3) -> Voxel {
        let size = self.size(level);
        if size.cmple(IVec3::ZERO).any() {
            return Voxel::AIR;
        }
        let (key, local) = split(index.clamp(IVec3::ZERO, size - 1));
        match self.chunks.get(&(level, key)) {
            Some(Chunk::Uniform(v)) => *v,
            Some(Chunk::Mixed(s)) => crate::store::Store::get(s, local),
            None => Voxel::AIR,
        }
    }

    fn size(&self, level: u32) -> IVec3 {
        self.sizes
            .get(level as usize)
            .copied()
            .unwrap_or(IVec3::ZERO)
    }
}

#[cfg(test)]
#[path = "lod_tests.rs"]
mod tests;
