//! A ground: a box of chunks, each kept by one of the stores, and the
//! samples a chunk's mesh is made from.
//!
//! The ground's edges continue its outermost voxels outwards, so the
//! surface simply stops at the edge (no walls are made there), and
//! nothing below the ground's floor is ever drawn.

use crate::brush::{Brush, Region};
use crate::store::Store;
use crate::voxel::{Dense, SIZE, SIZE_I, Voxel, split};
use glam::{IVec3, UVec3};
use rayon::prelude::*;
use std::collections::HashMap;

/// One chunk of a ground.
#[derive(Clone)]
pub enum Chunk<S> {
    /// The same voxel all through (all air, or deep in the ground).
    Uniform(Voxel),
    /// The surface passes through it, so it's kept by the store.
    Mixed(S),
}

impl<S: Store> Chunk<S> {
    /// Keeps `dense`: as one voxel if it's the same all through.
    pub fn keep(dense: &Dense) -> Self {
        match dense.uniform() {
            Some(voxel) => Self::Uniform(voxel),
            None => Self::Mixed(S::encode(dense)),
        }
    }

    fn get(&self, p: UVec3) -> Voxel {
        match self {
            Self::Uniform(v) => *v,
            Self::Mixed(s) => s.get(p),
        }
    }

    /// Writes every voxel into `out`.
    pub fn decode(&self, out: &mut Dense) {
        match self {
            Self::Uniform(v) => out.voxels_mut().fill(*v),
            Self::Mixed(s) => s.decode(out),
        }
    }
}

/// A ground of `S`-kept chunks.
pub struct Ground<S> {
    chunks_across: UVec3,
    chunks: Vec<Chunk<S>>,
}

impl<S: Store> Ground<S> {
    /// A ground `chunks_across` chunks in each direction, each chunk's
    /// voxels written by `fill` (given its key), made on every core.
    pub fn build(chunks_across: UVec3, fill: impl Fn(IVec3, &mut Dense) + Sync) -> Self {
        let count = (chunks_across.x * chunks_across.y * chunks_across.z) as usize;
        let chunks = (0..count)
            .into_par_iter()
            .map_init(
                || Dense::filled(Voxel::AIR),
                |dense, i| {
                    fill(key_at(chunks_across, i), dense);
                    Chunk::keep(dense)
                },
            )
            .collect();
        Self {
            chunks_across,
            chunks,
        }
    }

    /// A ground from its chunks, in [`Self::keys`]' order (as a file
    /// reader makes them); `chunks` must hold one for every key.
    pub fn assemble(chunks_across: UVec3, chunks: Vec<Chunk<S>>) -> Self {
        Self {
            chunks_across,
            chunks,
        }
    }

    /// How many chunks it is across, in each direction.
    pub fn chunks_across(&self) -> UVec3 {
        self.chunks_across
    }

    /// Its size in voxels.
    pub fn size(&self) -> IVec3 {
        self.chunks_across.as_ivec3() * SIZE_I
    }

    /// Every chunk's key, in order.
    pub fn keys(&self) -> impl Iterator<Item = IVec3> + '_ {
        (0..self.chunks.len()).map(|i| key_at(self.chunks_across, i))
    }

    /// The keys of the chunks the surface passes through.
    pub fn mixed_keys(&self) -> Vec<IVec3> {
        self.keys()
            .filter(|k| matches!(self.chunk(*k), Some(Chunk::Mixed(_))))
            .collect()
    }

    /// The chunk at `key`, if the ground has one there.
    pub fn chunk(&self, key: IVec3) -> Option<&Chunk<S>> {
        self.slot(key).map(|i| &self.chunks[i])
    }

    fn slot(&self, key: IVec3) -> Option<usize> {
        let across = self.chunks_across.as_ivec3();
        if key.cmplt(IVec3::ZERO).any() || key.cmpge(across).any() {
            return None;
        }
        Some((key.x + across.x * (key.y + across.y * key.z)) as usize)
    }

    /// The voxel at world place `p`; past an edge, the edge's own.
    pub fn get(&self, p: IVec3) -> Voxel {
        let (key, local) = split(p.clamp(IVec3::ZERO, self.size() - 1));
        self.chunk(key).map_or(Voxel::AIR, |c| c.get(local))
    }

    /// The memory the chunks take, in bytes.
    pub fn bytes(&self) -> usize {
        self.chunks
            .iter()
            .map(|c| match c {
                Chunk::Uniform(_) => std::mem::size_of::<Chunk<S>>(),
                Chunk::Mixed(s) => std::mem::size_of::<Chunk<S>>() + s.bytes(),
            })
            .sum()
    }

    /// The samples the chunk at `key` is meshed from: its own voxels and
    /// two more all round. `scratch` is a chunk to decode into.
    pub fn gather(&self, key: IVec3, scratch: &mut Dense, out: &mut Samples) {
        let origin = key * SIZE_I;
        out.origin = origin;
        match self.chunk(key) {
            Some(chunk) => chunk.decode(scratch),
            None => scratch.voxels_mut().fill(Voxel::AIR),
        }
        let inner = 0..SIZE_I;
        for z in -APRON..SIZE_I + APRON {
            for y in -APRON..SIZE_I + APRON {
                let row = Samples::index(IVec3::new(-APRON, y, z));
                let inside_row = inner.contains(&y) && inner.contains(&z);
                for x in -APRON..SIZE_I + APRON {
                    let p = IVec3::new(x, y, z);
                    out.voxels[row + (x + APRON) as usize] = if inside_row && inner.contains(&x) {
                        scratch.get(p.as_uvec3())
                    } else {
                        self.get(origin + p)
                    };
                }
            }
        }
    }

    /// Applies `brush`, chunk by chunk, and says which chunks' meshes it
    /// changed (the chunks whose samples include a changed voxel).
    pub fn apply(&mut self, brush: &Brush) -> Vec<IVec3> {
        match self.apply_changed(brush) {
            Some((lo, hi)) => self.meshes_reaching(lo, hi),
            None => Vec::new(),
        }
    }

    /// Applies `brush` and says which voxels it changed: the box from
    /// the first up to (not including) the second, if any.
    pub fn apply_changed(&mut self, brush: &Brush) -> Option<(IVec3, IVec3)> {
        let (lo, hi) = brush.reach();
        let lo = lo.max(IVec3::ZERO);
        let hi = hi.min(self.size());
        if lo.cmpge(hi).any() {
            return None;
        }
        let (first, _) = split(lo);
        let (last, _) = split(hi - 1);
        let mut decoded: HashMap<IVec3, Dense> = HashMap::new();
        for key in boxed(first, last + 1) {
            let mut dense = Dense::filled(Voxel::AIR);
            if let Some(chunk) = self.chunk(key) {
                chunk.decode(&mut dense);
            }
            decoded.insert(key, dense);
        }
        let mut region = Region::new(lo, (hi - lo).as_uvec3());
        region.fill(|p| {
            let (key, local) = split(p);
            decoded.get(&key).map_or(Voxel::AIR, |d| d.get(local))
        });
        let changed = brush.apply(&mut region)?;
        let mut touched = Vec::new();
        for (key, dense) in &mut decoded {
            if region.write_into(*key, dense) {
                touched.push(*key);
            }
        }
        for key in touched {
            if let (Some(slot), Some(dense)) = (self.slot(key), decoded.get(&key)) {
                self.chunks[slot] = Chunk::keep(dense);
            }
        }
        Some(changed)
    }

    /// Rewrites every voxel from `lo` up to (not including) `hi` with
    /// `voxel` (given its world place), chunk by chunk; returns the keys
    /// of the chunks that changed.
    pub fn rewrite(&mut self, lo: IVec3, hi: IVec3, voxel: impl Fn(IVec3) -> Voxel) -> Vec<IVec3> {
        let lo = lo.max(IVec3::ZERO);
        let hi = hi.min(self.size());
        if lo.cmpge(hi).any() {
            return Vec::new();
        }
        let (first, _) = split(lo);
        let (last, _) = split(hi - 1);
        let mut changed = Vec::new();
        let mut dense = Dense::filled(Voxel::AIR);
        for key in boxed(first, last + 1) {
            let Some(slot) = self.slot(key) else {
                continue;
            };
            self.chunks[slot].decode(&mut dense);
            let from = (key * SIZE_I).max(lo);
            let to = ((key + 1) * SIZE_I).min(hi);
            let mut any = false;
            for p in boxed(from, to) {
                let local = (p - key * SIZE_I).as_uvec3();
                let v = voxel(p);
                if dense.get(local) != v {
                    dense.set(local, v);
                    any = true;
                }
            }
            if any {
                self.chunks[slot] = Chunk::keep(&dense);
                changed.push(key);
            }
        }
        changed
    }

    /// The chunks whose samples include any voxel in `lo..hi`.
    pub fn meshes_reaching(&self, lo: IVec3, hi: IVec3) -> Vec<IVec3> {
        let (first, _) = split(lo - APRON);
        let (last, _) = split(hi - 1 + APRON);
        let mut keys: Vec<IVec3> = boxed(first, last + 1)
            .filter(|k| self.slot(*k).is_some())
            .collect();
        keys.sort_by_key(|k| (k.z, k.y, k.x));
        keys
    }
}

/// The keys from `lo` up to (not including) `hi`.
pub fn boxed(lo: IVec3, hi: IVec3) -> impl Iterator<Item = IVec3> {
    (lo.z..hi.z).flat_map(move |z| {
        (lo.y..hi.y).flat_map(move |y| (lo.x..hi.x).map(move |x| IVec3::new(x, y, z)))
    })
}

fn key_at(across: UVec3, i: usize) -> IVec3 {
    let (x, y) = (across.x as usize, across.y as usize);
    IVec3::new((i % x) as i32, ((i / x) % y) as i32, (i / (x * y)) as i32)
}

/// Samples kept round a chunk's own voxels.
pub const APRON: i32 = 2;
/// Samples along each side of [`Samples`].
pub const SPAN: usize = SIZE + 2 * APRON as usize;

/// A chunk's voxels and [`APRON`] more all round, which is everything
/// each look needs to mesh it the same way its neighbours do.
pub struct Samples {
    /// Where the chunk's first voxel is in the world.
    pub origin: IVec3,
    voxels: Vec<Voxel>,
}

impl Default for Samples {
    fn default() -> Self {
        Self {
            origin: IVec3::ZERO,
            voxels: vec![Voxel::AIR; SPAN * SPAN * SPAN],
        }
    }
}

impl Samples {
    fn index(p: IVec3) -> usize {
        let q = (p + APRON).as_uvec3();
        q.x as usize + SPAN * (q.y as usize + SPAN * q.z as usize)
    }

    /// The voxel at `p`, from the chunk's first voxel (each part from
    /// `-APRON` to `SIZE + APRON - 1`).
    pub fn get(&self, p: IVec3) -> Voxel {
        self.voxels[Self::index(p)]
    }

    /// Its distance at `p`, in voxels.
    pub fn distance(&self, p: IVec3) -> f32 {
        self.get(p).distance()
    }

    /// The way the distance grows at `p` (towards the air), by the
    /// difference of its neighbours; needs `p` one inside the samples.
    pub fn gradient(&self, p: IVec3) -> glam::Vec3 {
        let d = |q: IVec3| self.distance(q);
        glam::Vec3::new(
            d(p + IVec3::X) - d(p - IVec3::X),
            d(p + IVec3::Y) - d(p - IVec3::Y),
            d(p + IVec3::Z) - d(p - IVec3::Z),
        ) * 0.5
    }

    /// Sets the voxel at `p` (as [`Self::get`] counts), so a mesher can
    /// be handed samples changed at the chunk's edges.
    pub fn set(&mut self, p: IVec3, voxel: Voxel) {
        let i = Self::index(p);
        self.voxels[i] = voxel;
    }

    /// Bytes they take (copied to a worker).
    pub fn bytes(&self) -> usize {
        self.voxels.len() * std::mem::size_of::<Voxel>()
    }
}

#[cfg(test)]
#[path = "ground_tests.rs"]
mod tests;
