//! One region's file: the voxels of a 256 × 256 m square of the ground,
//! every level from 0 to 3, as the project lead chose ("one compressed
//! binary file per 256 × 256 m region").
//!
//! The file starts with a table of its chunks, so one chunk can be read
//! without reading the rest: a chunk the same all through is one voxel
//! in the table; any other is its runs of equal voxels, deflated on its
//! own, at a place the table gives.
//!
//! ```text
//! "GRGN"  version (u16)  region x, y (i32)  levels (u8)
//! per chunk, level by level, then z, y, x:
//!     0, the voxel (u16)          a chunk the same all through
//!     1, offset, length (u32)     where its deflated runs are, after the table
//! the deflated runs
//! ```
//!
//! The reader trusts nothing: every count, offset and length is checked
//! before anything is made from it, and a chunk inflates to at most the
//! size its runs can take.

use crate::ground::Chunk;
use crate::lod::{ChunkId, chunk_metres};
use crate::store::Full;
use crate::voxel::{Dense, VOLUME, Voxel};
use crate::world;
use glam::{IVec2, IVec3};
use std::io::{Read, Write};

/// A region's side in metres.
pub const SIDE: i32 = 256;
/// The levels a region's file holds: 0 to 3 (a level-3 chunk is a whole
/// region).
pub const LEVELS: u32 = 4;
const MAGIC: &[u8; 4] = b"GRGN";
const VERSION: u16 = 1;
const HEADER: usize = 4 + 2 + 4 + 4 + 1;
/// The most bytes one chunk's runs take: a run per voxel.
const MOST_RUNS_BYTES: usize = 4 + VOLUME * 4;

/// The chunks of `level` in the region `region`, in the file's order.
pub fn chunks(region: IVec2, level: u32) -> Vec<IVec3> {
    let across = SIDE / chunk_metres(level);
    let first = region * across;
    let mut out = Vec::new();
    for z in 0..world::chunks_high(level) as i32 {
        for y in 0..across {
            for x in 0..across {
                out.push((first + IVec2::new(x, y)).extend(z));
            }
        }
    }
    out
}

/// The region holding the chunk `id`.
pub fn region_of((level, key): ChunkId) -> IVec2 {
    let across = SIDE / chunk_metres(level);
    key.truncate().div_euclid(IVec2::splat(across))
}

/// Every chunk a region's file holds, level by level.
pub fn all_chunks(region: IVec2) -> Vec<ChunkId> {
    (0..LEVELS)
        .flat_map(|level| chunks(region, level).into_iter().map(move |k| (level, k)))
        .collect()
}

/// A chunk as the file keeps it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Packed {
    /// The same voxel all through.
    Uniform(Voxel),
    /// Its runs, deflated.
    Runs(Vec<u8>),
}

/// Packs a chunk for the file.
pub fn pack(chunk: &Chunk<Full>) -> Packed {
    let mut dense = Dense::filled(Voxel::AIR);
    chunk.decode(&mut dense);
    if let Some(v) = dense.uniform() {
        return Packed::Uniform(v);
    }
    let mut runs: Vec<(u16, Voxel)> = Vec::new();
    for v in dense.voxels() {
        match runs.last_mut() {
            Some((n, last)) if *last == *v && *n < u16::MAX => *n += 1,
            _ => runs.push((1, *v)),
        }
    }
    let mut raw = Vec::with_capacity(4 + runs.len() * 4);
    raw.extend_from_slice(&(runs.len() as u32).to_le_bytes());
    for (n, v) in runs {
        raw.extend_from_slice(&n.to_le_bytes());
        raw.extend_from_slice(&v.to_bits().to_le_bytes());
    }
    let mut encoder =
        flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
    if encoder.write_all(&raw).is_err() {
        return Packed::Runs(Vec::new());
    }
    Packed::Runs(encoder.finish().unwrap_or_default())
}

/// Unpacks a chunk read from a file.
pub fn unpack(packed: &Packed) -> Result<Chunk<Full>, String> {
    let bytes = match packed {
        Packed::Uniform(v) => return Ok(Chunk::Uniform(*v)),
        Packed::Runs(bytes) => bytes,
    };
    let mut raw = Vec::new();
    flate2::read::DeflateDecoder::new(bytes.as_slice())
        .take(MOST_RUNS_BYTES as u64 + 1)
        .read_to_end(&mut raw)
        .map_err(|e| format!("a chunk doesn't inflate: {e}"))?;
    if raw.len() > MOST_RUNS_BYTES || raw.len() < 4 {
        return Err("a chunk's runs are the wrong size".into());
    }
    let count = u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]) as usize;
    if count == 0 || count > VOLUME || raw.len() != 4 + count * 4 {
        return Err(format!("a chunk says it has {count} runs"));
    }
    let mut dense = Dense::filled(Voxel::AIR);
    let mut filled = 0;
    for run in raw[4..].chunks_exact(4) {
        let n = usize::from(u16::from_le_bytes([run[0], run[1]]));
        let v = Voxel::from_bits(u16::from_le_bytes([run[2], run[3]]));
        if n == 0 || filled + n > VOLUME {
            return Err("a chunk's runs don't add up to a chunk".into());
        }
        dense.voxels_mut()[filled..filled + n].fill(v);
        filled += n;
    }
    if filled != VOLUME {
        return Err("a chunk's runs don't add up to a chunk".into());
    }
    Ok(Chunk::keep(&dense))
}

/// The region `region`'s file, its chunks given by `packed` in
/// [`all_chunks`]' order.
pub fn write(region: IVec2, packed: &[Packed]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.extend_from_slice(&region.x.to_le_bytes());
    out.extend_from_slice(&region.y.to_le_bytes());
    out.push(LEVELS as u8);
    let mut offset = 0u32;
    for p in packed {
        match p {
            Packed::Uniform(v) => {
                out.push(0);
                out.extend_from_slice(&v.to_bits().to_le_bytes());
            }
            Packed::Runs(bytes) => {
                out.push(1);
                out.extend_from_slice(&offset.to_le_bytes());
                out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                offset += bytes.len() as u32;
            }
        }
    }
    for p in packed {
        if let Packed::Runs(bytes) = p {
            out.extend_from_slice(bytes);
        }
    }
    out
}

/// Where a chunk is in a region's file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Entry {
    /// The same voxel all through.
    Uniform(Voxel),
    /// Its deflated runs.
    At {
        /// Where they start, from the file's first byte.
        start: usize,
        /// How many bytes they take.
        length: usize,
    },
}

/// A region file's table: every chunk's entry, in [`all_chunks`] order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Index {
    /// Which region it is.
    pub region: IVec2,
    /// Every chunk's entry.
    pub entries: Vec<Entry>,
}

impl Index {
    /// How many bytes the table takes at most, read before the rest.
    pub fn most_bytes() -> usize {
        HEADER + all_chunks(IVec2::ZERO).len() * 9
    }

    /// Reads the table from the start of a file (`bytes` may be just its
    /// start); `length` is the whole file's.
    pub fn read(bytes: &[u8], length: usize, region: IVec2) -> Result<Self, String> {
        if bytes.len() < HEADER || &bytes[..4] != MAGIC {
            return Err("not a region file".into());
        }
        let version = u16::from_le_bytes([bytes[4], bytes[5]]);
        if version != VERSION {
            return Err(format!("version {version} isn't one this reads"));
        }
        let x = i32::from_le_bytes([bytes[6], bytes[7], bytes[8], bytes[9]]);
        let y = i32::from_le_bytes([bytes[10], bytes[11], bytes[12], bytes[13]]);
        if IVec2::new(x, y) != region || u32::from(bytes[14]) != LEVELS {
            return Err("the file is another region's".into());
        }
        let count = all_chunks(region).len();
        let mut at = HEADER;
        let mut entries = Vec::with_capacity(count);
        let mut lengths = Vec::with_capacity(count);
        for _ in 0..count {
            let tag = *bytes.get(at).ok_or("the table ends early")?;
            at += 1;
            match tag {
                0 => {
                    let b = bytes.get(at..at + 2).ok_or("the table ends early")?;
                    entries.push(Entry::Uniform(Voxel::from_bits(u16::from_le_bytes([
                        b[0], b[1],
                    ]))));
                    at += 2;
                }
                1 => {
                    let b = bytes.get(at..at + 8).ok_or("the table ends early")?;
                    let offset = u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize;
                    let length = u32::from_le_bytes([b[4], b[5], b[6], b[7]]) as usize;
                    entries.push(Entry::At {
                        start: offset,
                        length,
                    });
                    lengths.push(length);
                    at += 8;
                }
                _ => return Err(format!("a chunk's entry has kind {tag}")),
            }
        }
        let data = length.checked_sub(at).ok_or("the file ends in its table")?;
        for e in &mut entries {
            if let Entry::At { start, length } = e {
                if *length == 0 || start.checked_add(*length).is_none_or(|end| end > data) {
                    return Err("a chunk's place is past the file's end".into());
                }
                *start += at;
            }
        }
        if lengths.iter().sum::<usize>() != data {
            return Err("the chunks don't fill the file".into());
        }
        Ok(Self { region, entries })
    }

    /// The chunk at `entry` from the whole file `bytes`.
    pub fn packed(&self, bytes: &[u8], entry: usize) -> Result<Packed, String> {
        match self.entries.get(entry).ok_or("no such chunk")? {
            Entry::Uniform(v) => Ok(Packed::Uniform(*v)),
            Entry::At { start, length } => bytes
                .get(*start..*start + *length)
                .map(|b| Packed::Runs(b.to_vec()))
                .ok_or_else(|| "the file ends early".to_string()),
        }
    }
}

/// Reads a whole region's file into its chunks.
pub fn read(bytes: &[u8], region: IVec2) -> Result<Vec<(ChunkId, Chunk<Full>)>, String> {
    let index = Index::read(bytes, bytes.len(), region)?;
    all_chunks(region)
        .into_iter()
        .enumerate()
        .map(|(i, id)| Ok((id, unpack(&index.packed(bytes, i)?)?)))
        .collect()
}

#[cfg(test)]
#[path = "region_tests.rs"]
mod tests;
