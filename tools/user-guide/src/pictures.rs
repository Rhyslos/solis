//! The manual's pictures (decision 0113): screenshots of the editor in
//! `docs/manual/pictures/`, taken by `tools/guide-pictures`, shown in a
//! page as `![What it shows](pictures/name.png)` with a caption line
//! under it, and written into the guide as data so it stays one file.
//!
//! They are kept accurate three ways: a picture a page shows must be
//! there, every picture there must be shown by a page, and the pictures
//! must be as new as the editor's interface. For the last, the picture
//! script writes a stamp ([`STAMP`]): the editor's approved picture-test
//! images ([`INTERFACE`]) with a hash of each. When those images change,
//! the interface changed, and the pictures need taking again. Since
//! decision 0115 they are taken again once a phase, so that check is a
//! note, failing only when [`STRICT`] is set ([`verdict`]).

use std::collections::BTreeMap;
use std::path::Path;

/// Where the pictures are, from the repository's top folder.
pub const FOLDER: &str = "docs/manual/pictures";

/// How a page names one: a path from the page, in this folder.
pub const PREFIX: &str = "pictures/";

/// The editor's approved picture-test images, from the repository's top
/// folder: when they change, the interface changed.
pub const INTERFACE: &str = "crates/editor/tests/pictures";

/// The stamp the picture script writes, from the repository's top folder.
pub const STAMP: &str = "tools/guide-pictures/interface.txt";

/// The command that takes the pictures again, for messages.
pub const RETAKE: &str = "tools/guide-pictures/take-pictures.sh";

/// Set to `1`, pictures older than the interface fail the check instead
/// of giving a note (decision 0115): the end-of-phase run.
pub const STRICT: &str = "GUIDE_PICTURES_STRICT";

/// The largest a picture may be: the repository and the guide both
/// carry it.
pub const LARGEST: usize = 200 * 1024;

/// A picture file bigger than this isn't read at all.
const LARGEST_READ: u64 = 16 << 20;

/// More pictures than this aren't read.
const MOST: usize = 500;

/// The eight bytes every PNG starts with.
const PNG_START: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];

/// Every `.png` in the pictures folder, by file name. A missing folder
/// is no pictures.
pub fn read(root: &Path) -> Result<BTreeMap<String, Vec<u8>>, String> {
    let folder = root.join(FOLDER);
    let mut pictures = BTreeMap::new();
    let Ok(listing) = std::fs::read_dir(&folder) else {
        return Ok(pictures);
    };
    for entry in listing {
        let entry = entry.map_err(|e| format!("Couldn't read {}: {e}", folder.display()))?;
        let path = entry.path();
        let Some(name) = path
            .file_name()
            .and_then(|n| n.to_str())
            .filter(|n| n.ends_with(".png"))
        else {
            continue;
        };
        if pictures.len() >= MOST {
            return Err(format!("{FOLDER} has more than {MOST} pictures"));
        }
        let size = std::fs::metadata(&path)
            .map_err(|e| format!("Couldn't read {}: {e}", path.display()))?
            .len();
        if size > LARGEST_READ {
            return Err(format!(
                "{} is {size} bytes: far more than a picture should be",
                path.display()
            ));
        }
        let data =
            std::fs::read(&path).map_err(|e| format!("Couldn't read {}: {e}", path.display()))?;
        pictures.insert(name.to_string(), data);
    }
    Ok(pictures)
}

/// A PNG's width and height, from its header, or `None` when the bytes
/// aren't a PNG.
pub fn size(data: &[u8]) -> Option<(u32, u32)> {
    if data.len() < 24 || data[..8] != PNG_START || &data[12..16] != b"IHDR" {
        return None;
    }
    let number =
        |at: usize| u32::from_be_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]]);
    Some((number(16), number(20))).filter(|(w, h)| *w > 0 && *h > 0)
}

/// What is wrong with a picture, if anything: it isn't a PNG, or it is
/// too large.
pub fn problem(name: &str, data: &[u8]) -> Option<String> {
    if size(data).is_none() {
        return Some(format!(
            "pictures/{name} isn't a PNG picture: take it again with {RETAKE}"
        ));
    }
    (data.len() > LARGEST).then(|| {
        format!(
            "pictures/{name} is {} KB, more than the {} KB a picture may be: crop it to what it \
             shows, or make it smaller",
            data.len() / 1024,
            LARGEST / 1024
        )
    })
}

/// FNV-1a, 64 bits: enough to tell whether a file changed, and the same
/// on every computer.
pub fn hash(data: &[u8]) -> u64 {
    data.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

/// The stamp for the interface as it is now: each of the editor's
/// approved picture-test images, by name, with its hash.
pub fn stamp(root: &Path) -> Result<String, String> {
    let folder = root.join(INTERFACE);
    let listing = std::fs::read_dir(&folder)
        .map_err(|e| format!("Couldn't read {}: {e}", folder.display()))?;
    let mut lines = BTreeMap::new();
    for entry in listing {
        let entry = entry.map_err(|e| format!("Couldn't read {}: {e}", folder.display()))?;
        let path = entry.path();
        let Some(name) = path
            .file_name()
            .and_then(|n| n.to_str())
            .filter(|n| n.ends_with(".png"))
        else {
            continue;
        };
        if lines.len() >= MOST {
            return Err(format!("{INTERFACE} has more than {MOST} pictures"));
        }
        let data =
            std::fs::read(&path).map_err(|e| format!("Couldn't read {}: {e}", path.display()))?;
        lines.insert(name.to_string(), hash(&data));
    }
    let mut out = format!(
        "# The editor's approved picture-test images ({INTERFACE}) when the\n\
         # guide's pictures were last taken, with a hash of each (FNV-1a, 64 bits).\n\
         # Written by {RETAKE}; a test fails when they change.\n"
    );
    for (name, hash) in lines {
        out.push_str(&format!("{name} {hash:016x}\n"));
    }
    Ok(out)
}

/// Whether the pictures are older than the interface: the stamp kept in
/// the repository isn't the stamp for the interface as it is now. A
/// missing stamp counts as older.
pub fn older_than_interface(root: &Path) -> Result<bool, String> {
    let now = stamp(root)?;
    let kept = std::fs::read_to_string(root.join(STAMP)).unwrap_or_default();
    let lines = |text: &str| -> Vec<String> {
        text.lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(str::to_string)
            .collect()
    };
    Ok(lines(&kept) != lines(&now))
}

/// What the check makes of pictures older than the interface (decision
/// 0115): the pictures are taken again once a phase, so between phases
/// it is a note (`Ok(Some(..))`), and only at the end of a phase, run
/// with [`STRICT`] set to `1`, a failure (`Err`). As new as the
/// interface, it says nothing.
pub fn verdict(older: bool, strict: bool) -> Result<Option<String>, String> {
    if !older {
        return Ok(None);
    }
    let words = format!(
        "The guide's pictures are older than the interface: the editor's approved picture-test \
         images ({INTERFACE}) changed since they were taken. They are taken again at the end of \
         each phase, which also builds the guide; commit both:\n  {RETAKE}"
    );
    if strict {
        Err(words)
    } else {
        Ok(Some(format!(
            "Note: {words}\n(It fails only with {STRICT}=1, at the end of a phase.)"
        )))
    }
}

/// Whether [`STRICT`] is set to `1` in this run.
pub fn strict_from_env() -> bool {
    std::env::var(STRICT).is_ok_and(|v| v.trim() == "1")
}

/// A picture as an address the page can show without the network.
pub fn data_address(data: &[u8]) -> String {
    format!("data:image/png;base64,{}", crate::base64(data))
}

#[cfg(test)]
#[path = "pictures_tests.rs"]
mod tests;
