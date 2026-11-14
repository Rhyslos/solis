//! The ground's file, two ways, to measure what each costs on disk.
//!
//! Both write a chunk the same all through as one voxel and any other
//! as runs of equal voxels in order (x fastest), which is where the
//! savings are: above and below the surface, long runs of air or rock.
//!
//! - **Text**: one chunk a block, a run as `count distance material`,
//!   readable and diffable in Git.
//! - **Binary**: the same runs as bytes; and that again squeezed with
//!   the deflate compressor, as a zip file would.
//!
//! The readers trust nothing: every count is checked before anything is
//! made from it, and a damaged file is an error, never a crash.

use crate::ground::{Chunk, Ground};
use crate::store::Store;
use crate::voxel::{Dense, VOLUME, Voxel};
use glam::UVec3;
use std::io::Write;

/// The most chunks a ground's file may hold (a 4 by 4 km ground, 256
/// voxels high).
pub const MAX_CHUNKS: u32 = 128 * 128 * 8;
const MAGIC: &[u8; 4] = b"GRND";
const VERSION: u16 = 1;

/// Runs of equal voxels in `dense`, in order.
fn runs(dense: &Dense) -> Vec<(u16, Voxel)> {
    let mut out: Vec<(u16, Voxel)> = Vec::new();
    for v in dense.voxels() {
        match out.last_mut() {
            Some((count, last)) if *last == *v => *count += 1,
            _ => out.push((1, *v)),
        }
    }
    out
}

/// The ground as text.
pub fn text<S: Store>(ground: &Ground<S>) -> String {
    let mut out = String::new();
    let across = ground.chunks_across();
    out.push_str(&format!(
        "ground {VERSION}\nchunks {} {} {}\n",
        across.x, across.y, across.z
    ));
    let mut dense = Dense::filled(Voxel::AIR);
    for key in ground.keys() {
        let Some(chunk) = ground.chunk(key) else {
            continue;
        };
        chunk.decode(&mut dense);
        let found = runs(&dense);
        out.push_str(&format!(
            "chunk {} {} {} {}",
            key.x,
            key.y,
            key.z,
            found.len()
        ));
        for (k, (count, v)) in found.iter().enumerate() {
            out.push(if k % 12 == 0 { '\n' } else { ' ' });
            out.push_str(&format!("{count} {} {}", v.distance, v.material));
        }
        out.push('\n');
    }
    out
}

/// The ground as bytes.
pub fn binary<S: Store>(ground: &Ground<S>) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&VERSION.to_le_bytes());
    for n in ground.chunks_across().to_array() {
        out.extend_from_slice(&n.to_le_bytes());
    }
    let mut dense = Dense::filled(Voxel::AIR);
    for key in ground.keys() {
        let Some(chunk) = ground.chunk(key) else {
            continue;
        };
        chunk.decode(&mut dense);
        let found = runs(&dense);
        out.extend_from_slice(&(found.len() as u32).to_le_bytes());
        for (count, v) in found {
            out.extend_from_slice(&count.to_le_bytes());
            out.extend_from_slice(&v.to_bits().to_le_bytes());
        }
    }
    out
}

/// `bytes` squeezed by deflate, as a zip file would.
pub fn squeezed(bytes: &[u8]) -> Vec<u8> {
    let mut encoder =
        flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
    if encoder.write_all(bytes).is_err() {
        return Vec::new();
    }
    encoder.finish().unwrap_or_default()
}

/// Reads [`binary`]'s bytes back into a ground.
pub fn read_binary<S: Store>(bytes: &[u8]) -> Result<Ground<S>, String> {
    let mut at = Reader { bytes, at: 0 };
    if at.take(4)? != MAGIC {
        return Err("not a ground file".into());
    }
    let version = u16::from_le_bytes(at.array()?);
    if version != VERSION {
        return Err(format!("version {version} isn't one this reads"));
    }
    let across = UVec3::new(at.u32()?, at.u32()?, at.u32()?);
    let count = chunk_count(across)?;
    let mut chunks = Vec::with_capacity(count);
    let mut dense = Dense::filled(Voxel::AIR);
    for _ in 0..count {
        let runs = at.u32()? as usize;
        if runs == 0 || runs > VOLUME || runs * 4 > at.left() {
            return Err(format!("a chunk says it has {runs} runs"));
        }
        let mut filled = 0usize;
        for _ in 0..runs {
            let length = usize::from(u16::from_le_bytes(at.array()?));
            let voxel = Voxel::from_bits(u16::from_le_bytes(at.array()?));
            if length == 0 || filled + length > VOLUME {
                return Err("a chunk's runs don't add up to a chunk".into());
            }
            dense.voxels_mut()[filled..filled + length].fill(voxel);
            filled += length;
        }
        if filled != VOLUME {
            return Err("a chunk's runs don't add up to a chunk".into());
        }
        chunks.push(Chunk::keep(&dense));
    }
    if at.left() != 0 {
        return Err("bytes left over after the last chunk".into());
    }
    Ok(Ground::assemble(across, chunks))
}

/// Reads [`text`] back into a ground.
pub fn read_text<S: Store>(text: &str) -> Result<Ground<S>, String> {
    let mut words = Words(text.split_ascii_whitespace());
    words.expect("ground")?;
    if words.int("the version", 0, i64::from(u16::MAX))? != i64::from(VERSION) {
        return Err("a version this doesn't read".into());
    }
    words.expect("chunks")?;
    let mut side = || {
        words
            .int("a size", 1, i64::from(MAX_CHUNKS))
            .map(|n| n as u32)
    };
    let across = UVec3::new(side()?, side()?, side()?);
    let count = chunk_count(across)?;
    let mut chunks = Vec::with_capacity(count);
    let mut dense = Dense::filled(Voxel::AIR);
    for _ in 0..count {
        words.expect("chunk")?;
        for _ in 0..3 {
            words.int("a chunk's place", 0, i64::from(MAX_CHUNKS))?;
        }
        let runs = words.int("a run count", 1, VOLUME as i64)?;
        let mut filled = 0usize;
        for _ in 0..runs {
            let length = words.int("a run's length", 1, VOLUME as i64)? as usize;
            let distance = words.int("a distance", i64::from(i8::MIN), i64::from(i8::MAX))?;
            let material = words.int("a material", 0, i64::from(u8::MAX))?;
            if filled + length > VOLUME {
                return Err("a chunk's runs don't add up to a chunk".into());
            }
            let voxel = Voxel {
                distance: distance as i8,
                material: material as u8,
            };
            dense.voxels_mut()[filled..filled + length].fill(voxel);
            filled += length;
        }
        if filled != VOLUME {
            return Err("a chunk's runs don't add up to a chunk".into());
        }
        chunks.push(Chunk::keep(&dense));
    }
    if words.0.next().is_some() {
        return Err("words left over after the last chunk".into());
    }
    Ok(Ground::assemble(across, chunks))
}

struct Words<'a>(std::str::SplitAsciiWhitespace<'a>);

impl Words<'_> {
    fn word(&mut self) -> Result<&str, String> {
        self.0
            .next()
            .ok_or_else(|| "the file ends early".to_string())
    }

    fn expect(&mut self, wanted: &str) -> Result<(), String> {
        let w = self.word()?;
        if w == wanted {
            Ok(())
        } else {
            Err(format!("expected {wanted}, found {w:.20}"))
        }
    }

    fn int(&mut self, what: &str, lo: i64, hi: i64) -> Result<i64, String> {
        let w = self.word()?;
        match w.parse::<i64>() {
            Ok(n) if (lo..=hi).contains(&n) => Ok(n),
            _ => Err(format!("{what} isn't a number from {lo} to {hi}: {w:.20}")),
        }
    }
}

fn chunk_count(across: UVec3) -> Result<usize, String> {
    let count = u64::from(across.x) * u64::from(across.y) * u64::from(across.z);
    if count == 0 || count > u64::from(MAX_CHUNKS) {
        return Err(format!("{count} chunks is more than a ground may have"));
    }
    Ok(count as usize)
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        let end = self.at.checked_add(n).filter(|e| *e <= self.bytes.len());
        let end = end.ok_or_else(|| "the file ends early".to_string())?;
        let out = &self.bytes[self.at..end];
        self.at = end;
        Ok(out)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], String> {
        let mut out = [0u8; N];
        out.copy_from_slice(self.take(N)?);
        Ok(out)
    }

    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.array()?))
    }

    fn left(&self) -> usize {
        self.bytes.len() - self.at
    }
}

#[cfg(test)]
#[path = "file_tests.rs"]
mod tests;
