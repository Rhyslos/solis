//! A region file: the voxels of a 256 × 256 m square of a ground, in
//! binary (decision 0116: voxels are kept like a texture, and each region
//! is a file Git stores and locks on its own).
//!
//! It is shaped for the Open worlds phase now (decision 0117), so that
//! phase needs no new version: a **table of chunks** at the start, so one
//! chunk can be read without the rest, with **room for coarser levels 1
//! to 3**, and the chunks' height written in it, never assumed.
//!
//! ```text
//! "GRGN"  version (u16)  region x, y (u32)  chunks high (u16)  levels (u8, 4)
//! the table: level 0 to 3, then z, y, x, a chunk each:
//!     0                           not kept here (levels 1 to 3 for now)
//!     1, the voxel (u16)          a chunk the same all through
//!     2, offset, length (u32)     its deflated runs, after the table
//! the deflated runs, in the table's order
//! ```
//!
//! A chunk's runs are its voxels in order (x fastest) as runs of equal
//! voxels: a count of runs (u32), then each run's length and voxel (u16
//! each). Everything is little-endian.
//!
//! The reader trusts nothing (rule 9): every count, offset and length is
//! checked before anything is made from it, and a chunk inflates to at
//! most the bytes its runs can take.

use crate::ground::Ground;
use crate::store::{Chunk, Slot, Store};
use crate::voxel::{SIZE_I, VOLUME, Voxel};
use glam::IVec3;
use std::io::{Read, Write};

/// A region's side in metres.
pub const SIDE: u32 = 256;
/// Chunks along a region's side, at level 0.
pub const CHUNKS_ACROSS: u32 = SIDE / SIZE_I as u32;
/// The levels a region file has room for: 0 (one-metre voxels) to 3.
pub const LEVELS: u8 = 4;
/// The version Solis writes.
pub const VERSION: u16 = 1;
/// The end of a region file's name (`0_1.region`).
pub const EXTENSION: &str = "region";
/// The most chunks high a region holds (512 m, as settings allow).
pub const MAX_CHUNKS_HIGH: u32 = crate::settings::MAX_HEIGHT / SIZE_I as u32;
/// The biggest region file read: every chunk's runs at their longest.
pub const MAX_BYTES: usize = 160 << 20;
const MAGIC: &[u8; 4] = b"GRGN";
const HEADER: usize = 4 + 2 + 4 + 4 + 2 + 1;
/// The most bytes one chunk's runs take: a run per voxel.
const MOST_RUNS_BYTES: usize = 4 + VOLUME * 4;

/// Why a region file couldn't be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegionError(pub String);

impl std::fmt::Display for RegionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for RegionError {}

fn fail<T>(why: impl Into<String>) -> Result<T, RegionError> {
    Err(RegionError(why.into()))
}

/// The file name of region `[x, y]` (`2_0.region`).
pub fn file_name(region: [u32; 2]) -> String {
    format!("{}_{}.{EXTENSION}", region[0], region[1])
}

/// The region a file name names, if it's one [`file_name`] makes.
pub fn region_of_name(name: &str) -> Option<[u32; 2]> {
    let stem = name.strip_suffix(&format!(".{EXTENSION}"))?;
    let (x, y) = stem.split_once('_')?;
    Some([x.parse().ok()?, y.parse().ok()?])
}

/// The level-0 chunks of `region` in the file's order (z, then y, then
/// x), for a ground `high` chunks high.
pub fn keys(region: [u32; 2], high: u32) -> Vec<IVec3> {
    let first = IVec3::new(
        (region[0] * CHUNKS_ACROSS) as i32,
        (region[1] * CHUNKS_ACROSS) as i32,
        0,
    );
    crate::store::keys(glam::UVec3::new(CHUNKS_ACROSS, CHUNKS_ACROSS, high))
        .map(|k| first + k)
        .collect()
}

/// A ground exactly one region big and `high` chunks high, with nothing
/// loaded: what a region file is read into on its own (the fuzz target).
pub fn lone(high: u32) -> Ground<crate::store::Fixed> {
    let extent = glam::UVec3::new(CHUNKS_ACROSS, CHUNKS_ACROSS, high.min(MAX_CHUNKS_HIGH));
    Ground::new(crate::store::Fixed::empty(extent))
}

/// How many table entries a level has, for a ground `high` chunks high.
fn entries_in(level: u8, high: u32) -> usize {
    let across = (CHUNKS_ACROSS >> level).max(1) as usize;
    across * across * high.div_ceil(1 << level) as usize
}

/// A chunk's runs, deflated.
fn pack(voxels: &[Voxel]) -> Vec<u8> {
    let mut runs: Vec<(u16, Voxel)> = Vec::new();
    for v in voxels {
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
    // Writing into memory can't fail.
    let _ = encoder.write_all(&raw);
    encoder.finish().unwrap_or_default()
}

/// A chunk's voxels from its deflated runs.
fn unpack(bytes: &[u8]) -> Result<Chunk, RegionError> {
    let mut raw = Vec::new();
    if flate2::read::DeflateDecoder::new(bytes)
        .take(MOST_RUNS_BYTES as u64 + 1)
        .read_to_end(&mut raw)
        .is_err()
    {
        return fail("a chunk's voxels are damaged (they don't unpack)");
    }
    if raw.len() > MOST_RUNS_BYTES || raw.len() < 4 {
        return fail("a chunk's voxels are the wrong size");
    }
    let count = u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]) as usize;
    if count == 0 || count > VOLUME || raw.len() != 4 + count * 4 {
        return fail(format!("a chunk says it has {count} runs of voxels"));
    }
    let mut voxels = vec![Voxel::AIR; VOLUME];
    let mut filled = 0;
    for run in raw[4..].chunks_exact(4) {
        let n = usize::from(u16::from_le_bytes([run[0], run[1]]));
        let v = Voxel::from_bits(u16::from_le_bytes([run[2], run[3]]));
        if n == 0 || filled + n > VOLUME {
            return fail("a chunk's runs don't add up to a chunk");
        }
        voxels[filled..filled + n].fill(v);
        filled += n;
    }
    if filled != VOLUME {
        return fail("a chunk's runs don't add up to a chunk");
    }
    Ok(Chunk::keep(&voxels))
}

/// The region file of `region` of `ground`. A chunk that isn't loaded is
/// written as not kept. Levels 1 to 3 are left empty for now.
pub fn write<S: Store>(ground: &Ground<S>, region: [u32; 2]) -> Vec<u8> {
    let high = ground.extent().z;
    let mut table = Vec::new();
    let mut data = Vec::new();
    for key in keys(region, high) {
        match ground.store().slot(key) {
            Slot::Loaded(Chunk::Uniform(v)) => {
                table.push(1);
                table.extend_from_slice(&v.to_bits().to_le_bytes());
            }
            Slot::Loaded(Chunk::Mixed(voxels)) => {
                let packed = pack(voxels);
                table.push(2);
                table.extend_from_slice(&(data.len() as u32).to_le_bytes());
                table.extend_from_slice(&(packed.len() as u32).to_le_bytes());
                data.extend_from_slice(&packed);
            }
            Slot::Absent | Slot::Outside => table.push(0),
        }
    }
    for level in 1..LEVELS {
        table.extend(std::iter::repeat_n(0u8, entries_in(level, high)));
    }
    let mut out = Vec::with_capacity(HEADER + table.len() + data.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.extend_from_slice(&region[0].to_le_bytes());
    out.extend_from_slice(&region[1].to_le_bytes());
    out.extend_from_slice(&(high as u16).to_le_bytes());
    out.push(LEVELS);
    out.extend_from_slice(&table);
    out.extend_from_slice(&data);
    out
}

/// The level-0 chunks of a region file, in [`keys`]' order (`None` for a
/// chunk the file doesn't keep), checked to be `region`'s, for a ground
/// `high` chunks high.
pub fn read(bytes: &[u8], region: [u32; 2], high: u32) -> Result<Vec<Option<Chunk>>, RegionError> {
    if bytes.len() > MAX_BYTES {
        return fail("it's bigger than any region file can be");
    }
    if bytes.len() < HEADER || &bytes[..4] != MAGIC {
        return fail("it isn't a ground's region file");
    }
    let version = u16::from_le_bytes([bytes[4], bytes[5]]);
    if version > VERSION || version == 0 {
        return fail(format!(
            "it's version {version}, which this version of the program doesn't read"
        ));
    }
    let x = u32::from_le_bytes([bytes[6], bytes[7], bytes[8], bytes[9]]);
    let y = u32::from_le_bytes([bytes[10], bytes[11], bytes[12], bytes[13]]);
    if [x, y] != region {
        return fail(format!(
            "it holds region {x}_{y}, not {}_{}",
            region[0], region[1]
        ));
    }
    let said_high = u32::from(u16::from_le_bytes([bytes[14], bytes[15]]));
    if said_high != high || high > MAX_CHUNKS_HIGH {
        return fail(format!(
            "it's {} m high, and the ground is {} m",
            said_high * SIZE_I as u32,
            high * SIZE_I as u32
        ));
    }
    if bytes[16] != LEVELS {
        return fail(format!("it says it has {} levels", bytes[16]));
    }
    let mut at = HEADER;
    let mut entries = Vec::new();
    let mut next = 0usize;
    for level in 0..LEVELS {
        for _ in 0..entries_in(level, high) {
            let entry = read_entry(bytes, &mut at, &mut next)?;
            if level == 0 {
                entries.push(entry);
            }
        }
    }
    let data = &bytes[at..];
    if next != data.len() {
        return fail("its chunks don't fill the file");
    }
    entries
        .into_iter()
        .map(|e| match e {
            Entry::None => Ok(None),
            Entry::Uniform(v) => Ok(Some(Chunk::Uniform(v))),
            Entry::Runs(start, len) => unpack(&data[start..start + len]).map(Some),
        })
        .collect()
}

enum Entry {
    None,
    Uniform(Voxel),
    Runs(usize, usize),
}

/// One table entry; `next` is where the next chunk's runs must start.
fn read_entry(bytes: &[u8], at: &mut usize, next: &mut usize) -> Result<Entry, RegionError> {
    let take = |at: &mut usize, n: usize| -> Result<&[u8], RegionError> {
        let end = at.checked_add(n).filter(|e| *e <= bytes.len());
        let Some(end) = end else {
            return fail("the file ends in its table of chunks");
        };
        let out = &bytes[*at..end];
        *at = end;
        Ok(out)
    };
    let tag = take(at, 1)?[0];
    match tag {
        0 => Ok(Entry::None),
        1 => {
            let b = take(at, 2)?;
            Ok(Entry::Uniform(Voxel::from_bits(u16::from_le_bytes([
                b[0], b[1],
            ]))))
        }
        2 => {
            let b = take(at, 8)?;
            let start = u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize;
            let length = u32::from_le_bytes([b[4], b[5], b[6], b[7]]) as usize;
            if start != *next || length == 0 || length > MOST_RUNS_BYTES * 2 {
                return fail("a chunk's place in the file is wrong");
            }
            // The table must still fit before the chunk's runs can.
            if bytes.len().saturating_sub(*at) < start + length {
                return fail("a chunk's place is past the file's end");
            }
            *next = start + length;
            Ok(Entry::Runs(start, length))
        }
        _ => fail(format!("a chunk's entry has kind {tag}")),
    }
}

/// Puts the chunks of `region` read from `bytes` into `ground`; chunks
/// the file doesn't keep are left as they were.
pub fn load_into<S: Store>(
    ground: &mut Ground<S>,
    region: [u32; 2],
    bytes: &[u8],
) -> Result<(), RegionError> {
    let high = ground.extent().z;
    let chunks = read(bytes, region, high)?;
    for (key, chunk) in keys(region, high).into_iter().zip(chunks) {
        if let Some(chunk) = chunk {
            ground.store_mut().put(key, chunk);
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "region_tests.rs"]
mod tests;
