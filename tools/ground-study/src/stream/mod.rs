//! Streaming the ground: only the chunks wanted near the camera (and
//! near units) kept in memory, read on a worker, let go of when memory
//! runs short, and sculpted chunks written back before they're let go.
//!
//! - [`region`]: a region's file.
//! - [`Source`]: where chunks come from: region files on disk
//!   ([`Files`]), or made from the world's formulas ([`Made`]), keeping
//!   only what was sculpted.
//! - [`Streamer`]: what is kept, two ways of choosing what to read
//!   ([`Policy`]), the memory budget, and edits.

pub mod region;
mod streamer;

pub use streamer::{Policy, Stats, Streamer, Wanted, wanted_for};

use crate::ground::Chunk;
use crate::lod::ChunkId;
use crate::store::Full;
use crate::voxel::{Dense, Voxel};
use crate::world;
use glam::IVec2;
use region::{Index, Packed};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Chunks read together, each with its id.
pub type Chunks = Vec<(ChunkId, Chunk<Full>)>;

/// Where a ground's chunks come from and go back to. Called on workers.
pub trait Source: Send + Sync + 'static {
    /// The chunk `id`, and how many bytes were read for it.
    fn chunk(&self, id: ChunkId) -> Result<(Chunk<Full>, usize), String>;

    /// Every chunk of `level` in the region `region`, read at once, and
    /// how many bytes were read.
    fn region(&self, region: IVec2, level: u32) -> Result<(Chunks, usize), String>;

    /// Keeps `changed` (chunks of the region `region`) for good.
    fn save(&self, region: IVec2, changed: &[(ChunkId, Chunk<Full>)]) -> Result<(), String>;
}

/// Region files in a folder, one per region (`x_y.region`).
pub struct Files {
    folder: PathBuf,
    indexes: Mutex<HashMap<IVec2, Arc<Index>>>,
}

impl Files {
    /// The region files in `folder`.
    pub fn new(folder: &Path) -> Self {
        Self {
            folder: folder.to_path_buf(),
            indexes: Mutex::new(HashMap::new()),
        }
    }

    /// Where the region `region`'s file is.
    pub fn path(&self, region: IVec2) -> PathBuf {
        self.folder
            .join(format!("{}_{}.region", region.x, region.y))
    }

    fn index(
        &self,
        region: IVec2,
        file: &mut File,
        length: usize,
    ) -> Result<(Arc<Index>, usize), String> {
        if let Some(index) = self
            .indexes
            .lock()
            .map_err(|_| "a worker failed")?
            .get(&region)
        {
            return Ok((Arc::clone(index), 0));
        }
        let mut head = vec![0; Index::most_bytes().min(length)];
        file.read_exact(&mut head).map_err(|e| e.to_string())?;
        let index = Arc::new(Index::read(&head, length, region)?);
        self.indexes
            .lock()
            .map_err(|_| "a worker failed")?
            .insert(region, Arc::clone(&index));
        Ok((index, head.len()))
    }
}

impl Source for Files {
    fn chunk(&self, id: ChunkId) -> Result<(Chunk<Full>, usize), String> {
        let r = region::region_of(id);
        let entry = region::all_chunks(r)
            .iter()
            .position(|c| *c == id)
            .ok_or("no such chunk")?;
        let mut file = File::open(self.path(r)).map_err(|e| e.to_string())?;
        let length = file.metadata().map_err(|e| e.to_string())?.len() as usize;
        let (index, mut read) = self.index(r, &mut file, length)?;
        let packed = match index.entries.get(entry) {
            Some(region::Entry::Uniform(v)) => Packed::Uniform(*v),
            Some(region::Entry::At { start, length }) => {
                let mut bytes = vec![0; *length];
                file.seek(SeekFrom::Start(*start as u64))
                    .and_then(|_| file.read_exact(&mut bytes))
                    .map_err(|e| e.to_string())?;
                read += *length;
                Packed::Runs(bytes)
            }
            None => return Err("no such chunk".into()),
        };
        Ok((region::unpack(&packed)?, read))
    }

    fn region(&self, r: IVec2, level: u32) -> Result<(Chunks, usize), String> {
        let bytes = std::fs::read(self.path(r)).map_err(|e| e.to_string())?;
        let index = Index::read(&bytes, bytes.len(), r)?;
        let mut out = Vec::new();
        for (i, id) in region::all_chunks(r).into_iter().enumerate() {
            if id.0 == level {
                out.push((id, region::unpack(&index.packed(&bytes, i)?)?));
            }
        }
        Ok((out, bytes.len()))
    }

    fn save(&self, r: IVec2, changed: &[(ChunkId, Chunk<Full>)]) -> Result<(), String> {
        let path = self.path(r);
        let old = std::fs::read(&path).ok();
        let index = old
            .as_ref()
            .map(|b| Index::read(b, b.len(), r))
            .transpose()?;
        let mut packed = Vec::new();
        for (i, id) in region::all_chunks(r).into_iter().enumerate() {
            let p = match changed.iter().find(|(c, _)| *c == id) {
                Some((_, chunk)) => region::pack(chunk),
                None => match (&index, &old) {
                    (Some(index), Some(bytes)) => index.packed(bytes, i)?,
                    _ => return Err("a new region must be saved whole".into()),
                },
            };
            packed.push(p);
        }
        // Written beside it, then swapped in: a crash never leaves half
        // a file (rule 11).
        let temporary = path.with_extension("region.new");
        std::fs::write(&temporary, region::write(r, &packed)).map_err(|e| e.to_string())?;
        std::fs::rename(&temporary, &path).map_err(|e| e.to_string())?;
        if let Ok(mut indexes) = self.indexes.lock() {
            indexes.remove(&r);
        }
        Ok(())
    }
}

/// The world made from its formulas as it's asked for; only chunks that
/// were sculpted are kept (in memory here; on disk in a real project).
#[derive(Default)]
pub struct Made {
    saved: Mutex<HashMap<ChunkId, Chunk<Full>>>,
}

impl Made {
    /// How many sculpted chunks it keeps.
    pub fn saved(&self) -> usize {
        self.saved.lock().map_or(0, |s| s.len())
    }
}

impl Source for Made {
    fn chunk(&self, id: ChunkId) -> Result<(Chunk<Full>, usize), String> {
        if let Some(chunk) = self.saved.lock().map_err(|_| "a worker failed")?.get(&id) {
            return Ok((chunk.clone(), 0));
        }
        let mut dense = Dense::filled(Voxel::AIR);
        world::fill(id.0, id.1, &mut dense);
        Ok((Chunk::keep(&dense), 0))
    }

    fn region(&self, r: IVec2, level: u32) -> Result<(Chunks, usize), String> {
        region::chunks(r, level)
            .into_iter()
            .map(|key| Ok(((level, key), self.chunk((level, key))?.0)))
            .collect::<Result<Vec<_>, String>>()
            .map(|chunks| (chunks, 0))
    }

    fn save(&self, _: IVec2, changed: &[(ChunkId, Chunk<Full>)]) -> Result<(), String> {
        let mut saved = self.saved.lock().map_err(|_| "a worker failed")?;
        for (id, chunk) in changed {
            saved.insert(*id, chunk.clone());
        }
        Ok(())
    }
}

/// Writes the region `r` of the world's formulas (levels 0 to 3) as a
/// file's bytes: each level's chunks made, packed and laid out.
pub fn made_region(r: IVec2) -> Vec<u8> {
    let made = Made::default();
    let packed: Vec<Packed> = region::all_chunks(r)
        .into_iter()
        .map(|id| {
            made.chunk(id)
                .map(|(c, _)| region::pack(&c))
                .unwrap_or(Packed::Uniform(Voxel::AIR))
        })
        .collect();
    region::write(r, &packed)
}

#[cfg(test)]
#[path = "stream_tests.rs"]
mod tests;
