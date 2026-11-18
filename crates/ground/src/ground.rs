//! A ground: its chunks in a [`Store`], a voxel at any place, and the
//! samples a chunk's mesh is made from.
//!
//! The ground's edges continue its outermost voxels outwards, so the
//! surface simply stops at the edge (decision 0116: no walls are made
//! there), and nothing below its floor is ever drawn.

use crate::store::{Chunk, Fixed, Slot, Store, keys, origin};
use crate::voxel::{SIZE, SIZE_I, Voxel, split};
use glam::{IVec3, UVec3};

/// An answer about the ground that may not be known yet: a chunk it
/// needs isn't loaded (decision 0117). Never read as air.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Answer<T> {
    /// The answer.
    Known(T),
    /// A chunk the answer needs isn't loaded.
    Unknown,
}

impl<T> Answer<T> {
    /// The answer, if known.
    pub fn known(self) -> Option<T> {
        match self {
            Self::Known(t) => Some(t),
            Self::Unknown => None,
        }
    }
}

/// A ground of chunks kept by `S`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ground<S: Store = Fixed> {
    store: S,
}

impl Ground<Fixed> {
    /// A ground `extent` chunks across, along and up, every chunk made by
    /// `chunk` (given its key), in key order.
    pub fn make(extent: UVec3, mut chunk: impl FnMut(IVec3) -> Chunk) -> Self {
        let mut store = Fixed::empty(extent);
        for key in keys(extent) {
            store.put(key, chunk(key));
        }
        Self { store }
    }
}

impl<S: Store> Ground<S> {
    /// A ground kept by `store`.
    pub fn new(store: S) -> Self {
        Self { store }
    }

    /// Its store.
    pub fn store(&self) -> &S {
        &self.store
    }

    /// Its store, to change.
    pub fn store_mut(&mut self) -> &mut S {
        &mut self.store
    }

    /// How many chunks it is across, along and up.
    pub fn extent(&self) -> UVec3 {
        self.store.extent()
    }

    /// Its size in voxels (metres).
    pub fn size(&self) -> IVec3 {
        self.extent().as_ivec3() * SIZE_I
    }

    /// The chunk at `key`, if it's loaded.
    pub fn chunk(&self, key: IVec3) -> Option<&Chunk> {
        match self.store.slot(key) {
            Slot::Loaded(c) => Some(c),
            Slot::Absent | Slot::Outside => None,
        }
    }

    /// The voxel at world place `p`; past an edge, the edge's own.
    pub fn voxel(&self, p: IVec3) -> Answer<Voxel> {
        let (key, local) = split(p.clamp(IVec3::ZERO, self.size() - 1));
        match self.store.slot(key) {
            Slot::Loaded(c) => Answer::Known(c.get(local)),
            Slot::Absent => Answer::Unknown,
            Slot::Outside => Answer::Known(Voxel::AIR),
        }
    }

    /// Whether the chunk at `key` makes no triangles whatever is around
    /// it: it and its 26 neighbours (as the edges continue them) are
    /// loaded, uniform and all solid or all air. Unknown if any isn't
    /// loaded.
    pub fn is_bare(&self, key: IVec3) -> Answer<bool> {
        let last = self.extent().as_ivec3() - 1;
        let mut solid = None;
        let mut bare = true;
        for dz in -1..=1 {
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let k = (key + IVec3::new(dx, dy, dz)).clamp(IVec3::ZERO, last);
                    match self.store.slot(k) {
                        Slot::Loaded(Chunk::Uniform(v)) => {
                            bare &= *solid.get_or_insert(v.is_solid()) == v.is_solid();
                        }
                        Slot::Loaded(Chunk::Mixed(_)) => bare = false,
                        Slot::Absent => return Answer::Unknown,
                        Slot::Outside => {}
                    }
                }
            }
        }
        Answer::Known(bare)
    }

    /// The samples the chunk at `key` is meshed from: its own voxels and
    /// one more all round from its neighbours, copied. Unknown if a
    /// chunk they come from isn't loaded.
    pub fn gather(&self, key: IVec3, out: &mut Samples) -> Answer<()> {
        let Slot::Loaded(own) = self.store.slot(key) else {
            return Answer::Unknown;
        };
        out.key = key;
        let first = origin(key);
        for z in -BORDER..SIZE_I + BORDER {
            for y in -BORDER..SIZE_I + BORDER {
                let inner_row = (0..SIZE_I).contains(&y) && (0..SIZE_I).contains(&z);
                let row = Samples::index(IVec3::new(-BORDER, y, z));
                for x in -BORDER..SIZE_I + BORDER {
                    let p = IVec3::new(x, y, z);
                    let at = row + (x + BORDER) as usize;
                    out.voxels[at] = if inner_row && (0..SIZE_I).contains(&x) {
                        own.get(p.as_uvec3())
                    } else {
                        match self.voxel(first + p) {
                            Answer::Known(v) => v,
                            Answer::Unknown => return Answer::Unknown,
                        }
                    };
                }
            }
        }
        Answer::Known(())
    }
}

/// Samples kept round a chunk's own voxels: one (the brief and the
/// mesher agree: each cube's facing comes from its own eight corners).
pub const BORDER: i32 = 1;
/// Samples along each side of [`Samples`].
pub const SPAN: usize = SIZE + 2 * BORDER as usize;

/// A chunk's voxels and [`BORDER`] more all round, copied out of the
/// ground: everything the mesher needs to mesh it the same way its
/// neighbours do, so it can be handed to a worker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Samples {
    /// The chunk's key.
    pub key: IVec3,
    voxels: Vec<Voxel>,
}

impl Default for Samples {
    fn default() -> Self {
        Self {
            key: IVec3::ZERO,
            voxels: vec![Voxel::AIR; SPAN * SPAN * SPAN],
        }
    }
}

impl Samples {
    fn index(p: IVec3) -> usize {
        let q = (p + BORDER).as_uvec3();
        q.x as usize + SPAN * (q.y as usize + SPAN * q.z as usize)
    }

    /// The voxel at `p`, counted from the chunk's first voxel (each part
    /// from `-1` to `32`).
    pub fn get(&self, p: IVec3) -> Voxel {
        self.voxels
            .get(Self::index(p))
            .copied()
            .unwrap_or(Voxel::AIR)
    }

    /// Sets the voxel at `p` (as [`Self::get`] counts).
    pub fn set(&mut self, p: IVec3, voxel: Voxel) {
        let i = Self::index(p);
        if let Some(v) = self.voxels.get_mut(i) {
            *v = voxel;
        }
    }

    /// Whether every sample is on one side of the surface, so the chunk
    /// makes no triangles.
    pub fn is_bare(&self) -> bool {
        let first = self.voxels[0].is_solid();
        self.voxels.iter().all(|v| v.is_solid() == first)
    }

    /// Bytes they take when copied to a worker.
    pub fn bytes(&self) -> usize {
        self.voxels.len() * std::mem::size_of::<Voxel>()
    }
}

#[cfg(test)]
#[path = "ground_tests.rs"]
mod tests;
