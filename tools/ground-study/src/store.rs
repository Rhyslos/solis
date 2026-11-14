//! The three ways of keeping a chunk's voxels that the study compares.
//!
//! Chunks that are the same all through (all air above the ground, all
//! rock deep below it) are kept as one voxel by the ground itself
//! (`ground::Chunk::Uniform`) whichever store is used, so these only ever
//! hold chunks the surface passes through.
//!
//! - [`Full`]: every voxel, two bytes each. Simple and quickest to read
//!   and change; the most memory.
//! - [`Sparse`]: only the voxels near the surface, found through a bit
//!   per voxel; the rest are just "air" or "solid".
//! - [`Octree`]: the chunk halved again and again, each part that is the
//!   same all through kept as one value.

use crate::voxel::{Dense, SIZE, VOLUME, Voxel, index, place};
use glam::UVec3;

/// One way of keeping a chunk.
pub trait Store: Sized + Send + Sync + 'static {
    /// The way's name in the report's tables.
    const NAME: &'static str;
    /// Keeps `dense`.
    fn encode(dense: &Dense) -> Self;
    /// Writes every voxel into `out`.
    fn decode(&self, out: &mut Dense);
    /// The voxel at `p`.
    fn get(&self, p: UVec3) -> Voxel;
    /// The memory it takes, in bytes, its own fields included.
    fn bytes(&self) -> usize;
}

/// Every voxel, as laid out.
pub struct Full(Dense);

impl Store for Full {
    const NAME: &'static str = "Fixed chunks";

    fn encode(dense: &Dense) -> Self {
        Self(dense.clone())
    }

    fn decode(&self, out: &mut Dense) {
        out.voxels_mut().copy_from_slice(self.0.voxels());
    }

    fn get(&self, p: UVec3) -> Voxel {
        self.0.get(p)
    }

    fn bytes(&self) -> usize {
        std::mem::size_of::<Self>() + VOLUME * std::mem::size_of::<Voxel>()
    }
}

const WORDS: usize = VOLUME / 64;

/// Only the voxels within two voxels of the surface, in order, with a bit
/// per voxel saying which are kept and another saying, for the rest,
/// whether they are inside. Voxels deep inside all take one material, the
/// chunk's most common deep one: digging into the ground shows that, not
/// what was there.
pub struct Sparse {
    near: Box<[u64]>,
    solid: Box<[u64]>,
    rank: Box<[u16]>,
    values: Vec<Voxel>,
    under: u8,
}

impl Store for Sparse {
    const NAME: &'static str = "Sparse chunks";

    fn encode(dense: &Dense) -> Self {
        let mut near = vec![0u64; WORDS];
        let mut solid = vec![0u64; WORDS];
        let mut values = Vec::new();
        let mut deep = [0u32; 256];
        for (i, v) in dense.voxels().iter().enumerate() {
            if !v.is_far() {
                near[i / 64] |= 1 << (i % 64);
                values.push(*v);
            } else if v.is_solid() {
                solid[i / 64] |= 1 << (i % 64);
                deep[usize::from(v.material)] += 1;
            }
        }
        let under = (0..=255u8)
            .max_by_key(|m| (deep[usize::from(*m)], std::cmp::Reverse(*m)))
            .unwrap_or(0);
        let mut rank = vec![0u16; WORDS];
        let mut count = 0u32;
        for (r, word) in rank.iter_mut().zip(&near) {
            *r = count as u16;
            count += word.count_ones();
        }
        values.shrink_to_fit();
        Self {
            near: near.into_boxed_slice(),
            solid: solid.into_boxed_slice(),
            rank: rank.into_boxed_slice(),
            values,
            under,
        }
    }

    fn decode(&self, out: &mut Dense) {
        let mut next = self.values.iter();
        for (i, v) in out.voxels_mut().iter_mut().enumerate() {
            let bit = 1u64 << (i % 64);
            *v = if self.near[i / 64] & bit != 0 {
                next.next().copied().unwrap_or(Voxel::AIR)
            } else if self.solid[i / 64] & bit != 0 {
                Voxel::solid(self.under)
            } else {
                Voxel::AIR
            };
        }
    }

    fn get(&self, p: UVec3) -> Voxel {
        let i = index(p);
        let (word, bit) = (i / 64, i % 64);
        let near = self.near[word];
        if near & (1 << bit) != 0 {
            let before = (near & ((1u64 << bit) - 1)).count_ones() as usize;
            let at = usize::from(self.rank[word]) + before;
            return self.values.get(at).copied().unwrap_or(Voxel::AIR);
        }
        if self.solid[word] & (1 << bit) != 0 {
            Voxel::solid(self.under)
        } else {
            Voxel::AIR
        }
    }

    fn bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + WORDS * (8 + 8 + 2)
            + self.values.capacity() * std::mem::size_of::<Voxel>()
    }
}

/// A word in [`Octree`]: a leaf holding a voxel when its top bit is set,
/// else where its eight children start.
const LEAF: u32 = 1 << 31;

/// The chunk as an octree: a part the same all through is one leaf;
/// otherwise its eight halves are kept, each the same way, down to single
/// voxels. Eight children always sit together, so a node is one word.
pub struct Octree {
    root: u32,
    nodes: Vec<u32>,
}

impl Octree {
    fn build(dense: &Dense, origin: UVec3, size: u32, nodes: &mut Vec<u32>) -> u32 {
        if size == 1 {
            return LEAF | u32::from(dense.get(origin).to_bits());
        }
        let half = size / 2;
        let mut children = [0u32; 8];
        for (octant, child) in children.iter_mut().enumerate() {
            *child = Self::build(dense, origin + offset(octant) * half, half, nodes);
        }
        if children[0] & LEAF != 0 && children.iter().all(|c| *c == children[0]) {
            return children[0];
        }
        let first = nodes.len() as u32;
        nodes.extend_from_slice(&children);
        first
    }

    fn fill(&self, word: u32, origin: UVec3, size: u32, out: &mut Dense) {
        if word & LEAF != 0 {
            let voxel = Voxel::from_bits(word as u16);
            for z in 0..size {
                for y in 0..size {
                    let row = index(origin + UVec3::new(0, y, z));
                    out.voxels_mut()[row..row + size as usize].fill(voxel);
                }
            }
            return;
        }
        let half = size / 2;
        for octant in 0..8 {
            let child = self.nodes[word as usize + octant];
            self.fill(child, origin + offset(octant) * half, half, out);
        }
    }

    /// How many words it holds (for the binary file's size).
    pub fn words(&self) -> usize {
        self.nodes.len() + 1
    }
}

fn offset(octant: usize) -> UVec3 {
    UVec3::new(
        (octant & 1) as u32,
        ((octant >> 1) & 1) as u32,
        ((octant >> 2) & 1) as u32,
    )
}

impl Store for Octree {
    const NAME: &'static str = "Octree";

    fn encode(dense: &Dense) -> Self {
        let mut nodes = Vec::new();
        let root = Self::build(dense, UVec3::ZERO, SIZE as u32, &mut nodes);
        nodes.shrink_to_fit();
        Self { root, nodes }
    }

    fn decode(&self, out: &mut Dense) {
        self.fill(self.root, UVec3::ZERO, SIZE as u32, out);
    }

    fn get(&self, p: UVec3) -> Voxel {
        let mut word = self.root;
        let mut half = SIZE as u32 / 2;
        while word & LEAF == 0 {
            let octant = usize::from(p.x & half != 0)
                | usize::from(p.y & half != 0) << 1
                | usize::from(p.z & half != 0) << 2;
            word = self.nodes[word as usize + octant];
            half /= 2;
        }
        Voxel::from_bits(word as u16)
    }

    fn bytes(&self) -> usize {
        std::mem::size_of::<Self>() + self.nodes.capacity() * 4
    }
}

/// Every place in a chunk, for tests and the file writer.
pub fn places() -> impl Iterator<Item = UVec3> {
    (0..VOLUME).map(place)
}

#[cfg(test)]
#[path = "store_tests.rs"]
mod tests;
