//! A fast 64-bit hash of a file's bytes, to recognise the same content
//! under another name and to key the import cache. Not for security: a
//! hostile file could be made to match, which at worst pairs a sidecar
//! wrongly or reuses a cache entry for content that imports the same way.

use std::io::Read;
use std::path::Path;

const K: u64 = 0x9E37_79B9_7F4A_7C15;

/// Hashes bytes as they come, 8 at a time.
#[derive(Debug, Clone)]
pub struct ContentHash {
    state: u64,
    length: u64,
    pending: [u8; 8],
    pending_len: usize,
}

impl Default for ContentHash {
    fn default() -> Self {
        Self {
            state: 0x5EED_5011_5A55_E7DB,
            length: 0,
            pending: [0; 8],
            pending_len: 0,
        }
    }
}

impl ContentHash {
    fn mix(&mut self, word: u64) {
        self.state = (self.state ^ word).wrapping_mul(K).rotate_left(29);
    }

    /// Adds `bytes`.
    pub fn update(&mut self, mut bytes: &[u8]) {
        self.length = self.length.wrapping_add(bytes.len() as u64);
        if self.pending_len > 0 {
            let take = (8 - self.pending_len).min(bytes.len());
            self.pending[self.pending_len..self.pending_len + take].copy_from_slice(&bytes[..take]);
            self.pending_len += take;
            bytes = &bytes[take..];
            if self.pending_len < 8 {
                return;
            }
            self.mix(u64::from_le_bytes(self.pending));
            self.pending_len = 0;
        }
        let mut chunks = bytes.chunks_exact(8);
        for chunk in &mut chunks {
            let mut word = [0u8; 8];
            word.copy_from_slice(chunk);
            self.mix(u64::from_le_bytes(word));
        }
        let rest = chunks.remainder();
        self.pending[..rest.len()].copy_from_slice(rest);
        self.pending_len = rest.len();
    }

    /// The hash of everything added.
    pub fn finish(&self) -> u64 {
        let mut copy = self.clone();
        if copy.pending_len > 0 {
            let mut last = [0u8; 8];
            last[..copy.pending_len].copy_from_slice(&copy.pending[..copy.pending_len]);
            copy.mix(u64::from_le_bytes(last));
        }
        copy.mix(copy.length);
        // A last avalanche, so nearby inputs spread over all bits.
        let mut z = copy.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Adds the bytes of the file at `path`, read in pieces. Only an
    /// ordinary file of at most `MAX_HASHED_BYTES` (4 GB) is read: a link to a
    /// device or a pipe would never end.
    pub fn update_file(&mut self, path: &Path) -> std::io::Result<()> {
        let not_a_file = || std::io::Error::other("it isn't an ordinary file");
        // Checked before opening too: opening a pipe waits for a writer.
        if !std::fs::metadata(path)?.is_file() {
            return Err(not_a_file());
        }
        let file = std::fs::File::open(path)?;
        if !file.metadata()?.is_file() {
            return Err(not_a_file());
        }
        let mut file = file.take(MAX_HASHED_BYTES + 1);
        let mut buffer = vec![0u8; 1 << 16];
        let mut read = 0u64;
        loop {
            let n = file.read(&mut buffer)?;
            if n == 0 {
                return Ok(());
            }
            read += n as u64;
            if read > MAX_HASHED_BYTES {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::FileTooLarge,
                    "it's bigger than any asset Solis reads",
                ));
            }
            self.update(&buffer[..n]);
        }
    }
}

/// The biggest file hashed: bigger than any asset Solis imports.
pub(crate) const MAX_HASHED_BYTES: u64 = 4 << 30;

/// The hash of one file's bytes.
pub fn hash_file(path: &Path) -> std::io::Result<u64> {
    let mut hash = ContentHash::default();
    hash.update_file(path)?;
    Ok(hash.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn of(bytes: &[u8]) -> u64 {
        let mut h = ContentHash::default();
        h.update(bytes);
        h.finish()
    }

    #[test]
    fn pieces_hash_the_same_as_the_whole() {
        let data: Vec<u8> = (0..1000u32).map(|i| (i * 7 % 251) as u8).collect();
        let mut pieces = ContentHash::default();
        for chunk in data.chunks(13) {
            pieces.update(chunk);
        }
        assert_eq!(pieces.finish(), of(&data));
    }

    #[test]
    fn different_content_hashes_differently() {
        assert_ne!(of(b"crate"), of(b"crate "));
        assert_ne!(of(b""), of(&[0]));
        assert_ne!(of(&[0; 8]), of(&[0; 16]));
        let mut one = vec![0u8; 4096];
        let before = of(&one);
        one[4000] = 1;
        assert_ne!(of(&one), before);
    }
}
