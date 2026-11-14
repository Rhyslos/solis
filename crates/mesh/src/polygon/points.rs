//! Point positions by id, in chunks shared between copies of a mesh.

use super::VertexId;
use crate::Bounds;
use glam::Vec3;
use std::sync::{Arc, OnceLock};

/// Points per chunk of [`Points`].
pub(super) const CHUNK: usize = 4096;

/// One chunk of point positions, with the box around them worked out when
/// first asked for.
#[derive(Default)]
struct Chunk {
    items: Vec<Option<Vec3>>,
    bounds: OnceLock<Option<Bounds>>,
}

impl Clone for Chunk {
    fn clone(&self) -> Self {
        Self {
            items: self.items.clone(),
            bounds: self.bounds.clone(),
        }
    }
}

impl Chunk {
    fn bounds(&self) -> Option<Bounds> {
        *self
            .bounds
            .get_or_init(|| Bounds::around(self.items.iter().flatten().copied()))
    }
}

/// Point positions by id, in chunks shared between copies of a mesh until
/// one copy changes them: moving a few points copies only their chunks,
/// and the box around all points is worked out again only for those
///.
#[derive(Clone, Default)]
pub(super) struct Points {
    chunks: Vec<Arc<Chunk>>,
    len: usize,
}

impl Points {
    pub(super) fn new(list: Vec<Option<Vec3>>) -> Self {
        let len = list.len();
        let chunks = list
            .chunks(CHUNK)
            .map(|c| {
                Arc::new(Chunk {
                    items: c.to_vec(),
                    bounds: OnceLock::new(),
                })
            })
            .collect();
        Self { chunks, len }
    }

    pub(super) fn len(&self) -> usize {
        self.len
    }

    pub(super) fn get(&self, i: usize) -> Option<Vec3> {
        self.chunks
            .get(i / CHUNK)?
            .items
            .get(i % CHUNK)
            .copied()
            .flatten()
    }

    pub(super) fn push(&mut self, p: Option<Vec3>) {
        if self.len.is_multiple_of(CHUNK) {
            self.chunks.push(Arc::new(Chunk::default()));
        }
        if let Some(last) = self.chunks.last_mut() {
            let chunk = Arc::make_mut(last);
            chunk.items.push(p);
            chunk.bounds = OnceLock::new();
        }
        self.len += 1;
    }

    /// Sets slot `i` (which must exist).
    pub(super) fn set(&mut self, i: usize, p: Option<Vec3>) {
        if let Some(chunk) = self.chunks.get_mut(i / CHUNK) {
            let chunk = Arc::make_mut(chunk);
            if let Some(slot) = chunk.items.get_mut(i % CHUNK) {
                *slot = p;
                chunk.bounds = OnceLock::new();
            }
        }
    }

    pub(super) fn iter(&self) -> impl ExactSizeIterator<Item = Option<Vec3>> + '_ {
        Slots {
            chunks: &self.chunks,
            at: 0,
            len: self.len,
        }
    }

    pub(super) fn bounds(&self) -> Option<Bounds> {
        self.chunks
            .iter()
            .filter_map(|c| c.bounds())
            .reduce(|a, b| Bounds {
                min: a.min.min(b.min),
                max: a.max.max(b.max),
            })
    }

    /// The slots that differ from `other` (of the same length), comparing
    /// only chunks that aren't shared.
    pub(super) fn differing(&self, other: &Self) -> Vec<VertexId> {
        let mut out = Vec::new();
        for (c, (a, b)) in self.chunks.iter().zip(&other.chunks).enumerate() {
            if Arc::ptr_eq(a, b) {
                continue;
            }
            for (i, (x, y)) in a.items.iter().zip(&b.items).enumerate() {
                if x != y {
                    out.push(VertexId((c * CHUNK + i) as u32));
                }
            }
        }
        out
    }

    pub(super) fn memory_size(&self) -> usize {
        self.chunks
            .iter()
            .map(|c| c.items.capacity() * std::mem::size_of::<Option<Vec3>>())
            .sum::<usize>()
            + self.chunks.capacity() * std::mem::size_of::<Arc<Chunk>>()
    }
}

/// Every slot of [`Points`], in order.
struct Slots<'a> {
    chunks: &'a [Arc<Chunk>],
    at: usize,
    len: usize,
}

impl Iterator for Slots<'_> {
    type Item = Option<Vec3>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.at >= self.len {
            return None;
        }
        let item = self.chunks[self.at / CHUNK].items[self.at % CHUNK];
        self.at += 1;
        Some(item)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let left = self.len - self.at;
        (left, Some(left))
    }
}

impl ExactSizeIterator for Slots<'_> {}

impl PartialEq for Points {
    fn eq(&self, other: &Self) -> bool {
        self.len == other.len
            && self
                .chunks
                .iter()
                .zip(&other.chunks)
                .all(|(a, b)| Arc::ptr_eq(a, b) || a.items == b.items)
    }
}
