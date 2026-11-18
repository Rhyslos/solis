//! Where a ground keeps its chunks, behind one small interface
//! ([`Store`]), so sparse chunks can replace fixed ones when grounds grow
//! (decision 0116), and streaming can leave chunks out (decision 0117).
//!
//! A chunk the same all through (sky, deep rock) is kept as one voxel;
//! any other keeps its voxels in full behind an [`Arc`], so the editor's
//! ground, its undo history and the running game's copy share every chunk
//! none of them has changed.

use crate::voxel::{SIZE_I, VOLUME, Voxel, index};
use glam::{IVec3, UVec3};
use std::sync::Arc;

/// One chunk: 32 × 32 × 32 voxels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Chunk {
    /// The same voxel all through.
    Uniform(Voxel),
    /// The surface passes through it: every voxel, x fastest, shared.
    Mixed(Arc<[Voxel]>),
}

impl Chunk {
    /// Keeps `voxels` (exactly [`VOLUME`] of them; any other count is
    /// read as air): as one voxel if they're the same all through.
    pub fn keep(voxels: &[Voxel]) -> Self {
        let Some(first) = voxels.first().copied().filter(|_| voxels.len() == VOLUME) else {
            return Self::Uniform(Voxel::AIR);
        };
        if voxels.iter().all(|v| *v == first) {
            Self::Uniform(first)
        } else {
            Self::Mixed(Arc::from(voxels))
        }
    }

    /// The voxel at `p` (each part below 32).
    pub fn get(&self, p: UVec3) -> Voxel {
        match self {
            Self::Uniform(v) => *v,
            Self::Mixed(voxels) => voxels.get(index(p)).copied().unwrap_or(Voxel::AIR),
        }
    }

    /// Writes every voxel into `out` (which must hold [`VOLUME`]).
    pub fn decode(&self, out: &mut [Voxel]) {
        match self {
            Self::Uniform(v) => out.fill(*v),
            Self::Mixed(voxels) => {
                let n = out.len().min(voxels.len());
                out[..n].copy_from_slice(&voxels[..n]);
            }
        }
    }

    /// Whether it's kept as one voxel.
    pub fn is_uniform(&self) -> bool {
        matches!(self, Self::Uniform(_))
    }

    /// Bytes its voxels take (a uniform chunk's one voxel counts as
    /// nothing beyond its slot).
    pub fn bytes(&self) -> usize {
        match self {
            Self::Uniform(_) => 0,
            Self::Mixed(v) => v.len() * std::mem::size_of::<Voxel>(),
        }
    }
}

/// What a store has at a chunk's key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot<'a> {
    /// The chunk, loaded.
    Loaded(&'a Chunk),
    /// Inside the ground, but not loaded: nobody knows what's there yet.
    Absent,
    /// Past the ground's edge.
    Outside,
}

/// How a ground keeps its chunks. Keys are chunk places: x and y across
/// the ground, z up, each from 0.
pub trait Store: Clone + Send + Sync + 'static {
    /// The chunks it holds across, along and up (the ground's size).
    fn extent(&self) -> UVec3;
    /// What it has at `key`.
    fn slot(&self, key: IVec3) -> Slot<'_>;
    /// Keeps `chunk` at `key`; whether `key` is inside the ground.
    fn put(&mut self, key: IVec3, chunk: Chunk) -> bool;
    /// Forgets the chunk at `key`, so it reads as not loaded.
    fn unload(&mut self, key: IVec3);
}

/// Fixed chunks (decision 0116): a slot for every chunk of the ground, in
/// a list, x fastest, then y, then z.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fixed {
    extent: UVec3,
    chunks: Vec<Option<Chunk>>,
}

impl Fixed {
    /// A store `extent` chunks across, along and up, with nothing loaded.
    pub fn empty(extent: UVec3) -> Self {
        let count = extent.x as usize * extent.y as usize * extent.z as usize;
        Self {
            extent,
            chunks: vec![None; count],
        }
    }

    fn at(&self, key: IVec3) -> Option<usize> {
        let e = self.extent.as_ivec3();
        if key.cmplt(IVec3::ZERO).any() || key.cmpge(e).any() {
            return None;
        }
        Some((key.x + e.x * (key.y + e.y * key.z)) as usize)
    }
}

impl Store for Fixed {
    fn extent(&self) -> UVec3 {
        self.extent
    }

    fn slot(&self, key: IVec3) -> Slot<'_> {
        match self.at(key) {
            None => Slot::Outside,
            Some(i) => self.chunks[i].as_ref().map_or(Slot::Absent, Slot::Loaded),
        }
    }

    fn put(&mut self, key: IVec3, chunk: Chunk) -> bool {
        let Some(i) = self.at(key) else {
            return false;
        };
        self.chunks[i] = Some(chunk);
        true
    }

    fn unload(&mut self, key: IVec3) {
        if let Some(i) = self.at(key) {
            self.chunks[i] = None;
        }
    }
}

/// Every key of a box `extent` chunks big, x fastest, then y, then z.
pub fn keys(extent: UVec3) -> impl Iterator<Item = IVec3> {
    let e = extent.as_ivec3();
    (0..e.z)
        .flat_map(move |z| (0..e.y).flat_map(move |y| (0..e.x).map(move |x| IVec3::new(x, y, z))))
}

/// The first voxel of the chunk at `key`, in world voxels.
pub fn origin(key: IVec3) -> IVec3 {
    key * SIZE_I
}

#[cfg(test)]
#[path = "store_tests.rs"]
mod tests;
