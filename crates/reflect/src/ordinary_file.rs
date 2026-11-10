//! Opening and reading the files a project ships, safely: only ordinary
//! files, and never more of one than was asked for.
//!
//! A project's folder can hold a link to a device (which never ends) or a
//! pipe (which waits for ever for someone to write to it), named like any
//! picture, sound or model. Every reader opens its files here, so the
//! check is made in one place, the same way for all of them.

use std::fs::File;
use std::io::Read;
use std::path::Path;

/// Why a file wasn't opened or read.
#[derive(Debug)]
pub enum Refused {
    /// It isn't an ordinary file: a folder, a device or a pipe, or a link
    /// to one.
    NotAFile,
    /// It holds more than the most that's read.
    TooBig {
        /// How many bytes it holds (or how many were read before it was
        /// found to have grown).
        bytes: u64,
        /// The most that's read.
        limit: u64,
    },
    /// The system couldn't open or read it (it isn't there, say).
    Io(std::io::Error),
}

impl std::fmt::Display for Refused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotAFile => f.write_str("it isn't an ordinary file"),
            Self::TooBig { bytes, limit } => {
                write!(f, "it's {bytes} bytes; the most read is {limit}")
            }
            Self::Io(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for Refused {}

impl From<std::io::Error> for Refused {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// For callers that speak `std::io::Error`: not an ordinary file is
/// `InvalidData`, too big is `FileTooLarge`, with the same words.
impl From<Refused> for std::io::Error {
    fn from(refused: Refused) -> Self {
        let kind = match refused {
            Refused::Io(e) => return e,
            Refused::NotAFile => std::io::ErrorKind::InvalidData,
            Refused::TooBig { .. } => std::io::ErrorKind::FileTooLarge,
        };
        Self::new(kind, refused.to_string())
    }
}

/// Opens the ordinary file at `path` for reading. A folder, a device or a
/// pipe, or a link to one, is refused at once.
pub fn open(path: &Path) -> Result<File, Refused> {
    // Checked before opening: opening a pipe waits for a writer.
    if !std::fs::metadata(path)?.is_file() {
        return Err(Refused::NotAFile);
    }
    let file = File::open(path)?;
    // And once it's open: the name may have been pointed somewhere else
    // in between.
    if !file.metadata()?.is_file() {
        return Err(Refused::NotAFile);
    }
    Ok(file)
}

/// Reads the whole ordinary file at `path`, if it holds at most `limit`
/// bytes: a bigger one is refused before any of it is read, and never
/// more than `limit` bytes are read even if it grows meanwhile.
pub fn read_limited(path: &Path, limit: u64) -> Result<Vec<u8>, Refused> {
    let file = open(path)?;
    let size = file.metadata()?.len();
    if size > limit {
        return Err(Refused::TooBig { bytes: size, limit });
    }
    let mut bytes = Vec::with_capacity(size as usize);
    file.take(limit.saturating_add(1)).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(Refused::TooBig {
            bytes: bytes.len() as u64,
            limit,
        });
    }
    Ok(bytes)
}

/// Reads at most `most` bytes from the start of the ordinary file at
/// `path`, however big it is.
pub fn read_start(path: &Path, most: u64) -> Result<Vec<u8>, Refused> {
    let file = open(path)?;
    // Room for exactly what will be read: a list left to grow by itself
    // doubles, and could take twice the bytes it was counted as.
    let size = file.metadata()?.len();
    let mut bytes = Vec::with_capacity(most.min(size) as usize);
    file.take(most).read_to_end(&mut bytes)?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::ErrorKind;
    use std::path::PathBuf;

    fn folder(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("solis-ordinary-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn an_ordinary_file_is_opened_and_read() {
        let dir = folder("read");
        let path = dir.join("a.txt");
        std::fs::write(&path, b"twelve bytes").unwrap();
        let mut text = String::new();
        open(&path).unwrap().read_to_string(&mut text).unwrap();
        assert_eq!(text, "twelve bytes");
        assert_eq!(read_limited(&path, 12).unwrap(), b"twelve bytes");
        assert_eq!(read_limited(&path, u64::MAX).unwrap(), b"twelve bytes");
        assert_eq!(read_start(&path, 6).unwrap(), b"twelve");
        assert_eq!(read_start(&path, 0).unwrap(), b"");
        assert_eq!(read_start(&path, u64::MAX).unwrap(), b"twelve bytes");
        std::fs::write(&path, b"").unwrap();
        assert_eq!(read_limited(&path, 0).unwrap(), b"");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_file_past_the_limit_is_refused_saying_its_size() {
        let dir = folder("big");
        let path = dir.join("a.txt");
        std::fs::write(&path, b"twelve bytes").unwrap();
        for limit in [0, 1, 11] {
            match read_limited(&path, limit) {
                Err(Refused::TooBig {
                    bytes: 12,
                    limit: l,
                }) => assert_eq!(l, limit),
                other => panic!("{other:?}"),
            }
        }
        let as_io = std::io::Error::from(read_limited(&path, 11).unwrap_err());
        assert_eq!(as_io.kind(), ErrorKind::FileTooLarge);
        assert_eq!(as_io.to_string(), "it's 12 bytes; the most read is 11");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_folder_or_a_missing_file_is_refused() {
        let dir = folder("folder");
        for refused in [
            open(&dir).map(drop),
            read_limited(&dir, 1 << 20).map(drop),
            read_start(&dir, 64).map(drop),
        ] {
            assert!(matches!(refused, Err(Refused::NotAFile)), "{refused:?}");
        }
        let as_io = std::io::Error::from(open(&dir).unwrap_err());
        assert_eq!(as_io.kind(), ErrorKind::InvalidData);
        assert_eq!(as_io.to_string(), "it isn't an ordinary file");
        match open(&dir.join("none")) {
            Err(Refused::Io(e)) => {
                assert_eq!(e.kind(), ErrorKind::NotFound);
                assert_eq!(
                    std::io::Error::from(Refused::Io(e)).kind(),
                    ErrorKind::NotFound
                );
            }
            other => panic!("{other:?}"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A device never ends and a pipe nobody writes to never opens: both
    /// are refused at once, under any name, directly or through a link.
    /// (If this test hangs, the check before opening has gone.) Windows
    /// has neither in a project's folder, so this part is for Unix.
    #[cfg(unix)]
    #[test]
    fn a_device_or_a_pipe_is_refused_at_once() {
        let dir = folder("special");
        let pipe = dir.join("pipe.wav");
        let made = std::process::Command::new("mkfifo").arg(&pipe).status();
        assert!(made.unwrap().success(), "mkfifo");
        let device = dir.join("zero.png");
        std::os::unix::fs::symlink("/dev/zero", &device).unwrap();
        let link_to_pipe = dir.join("linked.ogg");
        std::os::unix::fs::symlink(&pipe, &link_to_pipe).unwrap();
        for path in [&pipe, &device, &link_to_pipe, Path::new("/dev/zero")] {
            for refused in [
                open(path).map(drop),
                read_limited(path, 1 << 20).map(drop),
                read_start(path, 64).map(drop),
            ] {
                assert!(
                    matches!(refused, Err(Refused::NotAFile)),
                    "{}: {refused:?}",
                    path.display()
                );
            }
        }
        // A link to an ordinary file is still read.
        let real = dir.join("real.txt");
        std::fs::write(&real, b"here").unwrap();
        let link = dir.join("link.txt");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        assert_eq!(read_limited(&link, 16).unwrap(), b"here");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Files of every length around the limits asked for: what's read is
    /// the file's start, never more than asked, and a file past the limit
    /// is always refused.
    #[test]
    fn every_length_and_limit_reads_no_more_than_asked() {
        let dir = folder("sweep");
        let path = dir.join("a.bin");
        let mut seed: u64 = 15;
        let mut next = |n: u64| {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (seed >> 33) % n.max(1)
        };
        for _ in 0..300 {
            let length = next(5_000) as usize;
            let bytes: Vec<u8> = (0..length).map(|i| (i * 31) as u8).collect();
            std::fs::write(&path, &bytes).unwrap();
            let limit = match next(4) {
                0 => length as u64,
                1 => (length as u64).saturating_sub(1),
                2 => length as u64 + 1,
                _ => next(6_000),
            };
            match read_limited(&path, limit) {
                Ok(read) => assert!(length as u64 <= limit && read == bytes),
                Err(Refused::TooBig { bytes: b, limit: l }) => {
                    assert!(length as u64 > limit && (b, l) == (length as u64, limit));
                }
                Err(other) => panic!("{other:?}"),
            }
            let start = read_start(&path, limit).unwrap();
            assert_eq!(start, bytes[..length.min(limit as usize)]);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
