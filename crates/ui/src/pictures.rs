//! Pictures on screens ([decision 0089](../../../docs/decisions/0089-atlas-crop-and-the-ui-editor.md)):
//! Lust's `image "sprites/tools.png"` shows a picture from the project's
//! assets, and `image "sprites/tools.png#Axe"` one of its Atlas Crop crops.
//! Every picture a game's screens show is packed into one atlas, so the
//! renderer draws them all from a single texture in the same draw call as
//! the shapes and the letters.
//!
//! Each picture is kept with a pixel of its own edge copied round it, so
//! smoothing at its edges never takes in a neighbour, and its colours are
//! stored times their alpha (premultiplied), so a see-through pixel's colour
//! never shows as a dark fringe.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// The pictures' atlas is this many pixels each way.
pub const SIDE: u32 = 2048;

/// The most pictures in one atlas.
pub const MAX_PICTURES: usize = 1024;

/// The longest picture name kept (a path and a crop).
pub const MAX_NAME: usize = 512;

/// Every [`Pictures`] has its own revision, so the renderer can tell one
/// atlas from another.
static REVISIONS: AtomicU64 = AtomicU64::new(1);

/// The picture file and the crop a name gives: `sprites/tools.png#Axe` is
/// `("sprites/tools.png", Some("Axe"))`.
pub fn split(name: &str) -> (&str, Option<&str>) {
    match name.split_once('#') {
        Some((file, crop)) => (file, Some(crop)),
        None => (name, None),
    }
}

/// Where a picture went in the atlas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placed {
    /// Its size, in its own pixels.
    pub size: [u32; 2],
    /// Its top-left pixel in the atlas (inside its copied edge).
    pub at: [u32; 2],
}

impl Placed {
    /// Its box in the atlas, in atlas pixels: left, top, right, bottom.
    pub fn rect(&self) -> [f32; 4] {
        let [x, y] = self.at.map(|v| v as f32);
        let [w, h] = self.size.map(|v| v as f32);
        [x, y, x + w, y + h]
    }
}

/// The pictures a game's screens show, packed into one atlas.
#[derive(Clone)]
pub struct Pictures {
    /// The atlas, [`SIDE`] × [`SIDE`], RGBA with colours premultiplied;
    /// empty until a picture is added.
    pub pixels: Vec<u8>,
    /// Different for every atlas and after every change.
    pub revision: u64,
    placed: HashMap<String, Placed>,
    /// The shelf being filled: its left edge so far, its top, its height.
    shelf: [u32; 3],
}

impl Default for Pictures {
    fn default() -> Self {
        Self {
            pixels: Vec::new(),
            revision: REVISIONS.fetch_add(1, Ordering::Relaxed),
            placed: HashMap::new(),
            shelf: [0, 0, 0],
        }
    }
}

impl std::fmt::Debug for Pictures {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pictures")
            .field("pictures", &self.placed.len())
            .field("revision", &self.revision)
            .finish_non_exhaustive()
    }
}

impl PartialEq for Pictures {
    fn eq(&self, other: &Self) -> bool {
        self.revision == other.revision
    }
}

impl Pictures {
    /// No pictures yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Where the picture `name` is, if it was added.
    pub fn get(&self, name: &str) -> Option<Placed> {
        self.placed.get(name).copied()
    }

    /// How many pictures there are.
    pub fn len(&self) -> usize {
        self.placed.len()
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.placed.is_empty()
    }

    /// Adds the picture `name`: `rgba` (straight RGBA, rows from the top)
    /// `size` pixels big. Gives why not if it's empty, the wrong length,
    /// too big for the atlas or there's no room left; one already added
    /// stays as it was.
    pub fn add(&mut self, name: &str, size: [u32; 2], rgba: &[u8]) -> Result<(), String> {
        if self.placed.contains_key(name) {
            return Ok(());
        }
        if name.len() > MAX_NAME {
            return Err(format!("its name is longer than {MAX_NAME} letters"));
        }
        if self.placed.len() >= MAX_PICTURES {
            return Err(format!(
                "the game's screens show more than {MAX_PICTURES} pictures"
            ));
        }
        let [w, h] = size;
        if w == 0 || h == 0 {
            return Err("it has no pixels".into());
        }
        if rgba.len() as u64 != u64::from(w) * u64::from(h) * 4 {
            return Err("its pixels don't match its size".into());
        }
        // Its edge copied round it: a pixel each side.
        let (pw, ph) = (w + 2, h + 2);
        if pw > SIDE || ph > SIDE {
            return Err(format!(
                "it's {w} × {h} pixels, and pictures on screens are at most {} × {}",
                SIDE - 2,
                SIDE - 2
            ));
        }
        let [mut x, mut y, mut tall] = self.shelf;
        if x + pw > SIDE {
            // A new shelf under this one.
            y += tall;
            x = 0;
            tall = 0;
        }
        if y + ph > SIDE {
            return Err(format!(
                "there's no room left for it among the screens' pictures ({SIDE} × {SIDE} \
                 pixels in all): use smaller pictures, or crops of a sheet"
            ));
        }
        if self.pixels.is_empty() {
            self.pixels = vec![0; (SIDE * SIDE * 4) as usize];
        }
        for row in 0..ph {
            let from_y = row.saturating_sub(1).min(h - 1) as usize;
            for column in 0..pw {
                let from_x = column.saturating_sub(1).min(w - 1) as usize;
                let from = (from_y * w as usize + from_x) * 4;
                let px = &rgba[from..from + 4];
                let a = u16::from(px[3]);
                let times = |c: u8| ((u16::from(c) * a + 127) / 255) as u8;
                let to = (((y + row) * SIDE + x + column) * 4) as usize;
                self.pixels[to..to + 4].copy_from_slice(&[
                    times(px[0]),
                    times(px[1]),
                    times(px[2]),
                    px[3],
                ]);
            }
        }
        self.placed.insert(
            name.to_string(),
            Placed {
                size,
                at: [x + 1, y + 1],
            },
        );
        self.shelf = [x + pw, y, tall.max(ph)];
        self.revision = REVISIONS.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}

/// The part `rect` (left, top, width, height) of a picture `size` pixels
/// big, cut to the picture; `None` if nothing of it is inside.
pub fn cut(rgba: &[u8], size: [u32; 2], rect: [u32; 4]) -> Option<(Vec<u8>, [u32; 2])> {
    let [x, y, w, h] = rect;
    let right = x.saturating_add(w).min(size[0]);
    let bottom = y.saturating_add(h).min(size[1]);
    if x >= right || y >= bottom || rgba.len() as u64 != u64::from(size[0]) * u64::from(size[1]) * 4
    {
        return None;
    }
    let (cw, ch) = (right - x, bottom - y);
    let mut out = Vec::with_capacity((cw * ch * 4) as usize);
    for row in y..bottom {
        let start = ((row * size[0] + x) * 4) as usize;
        out.extend_from_slice(&rgba[start..start + (cw * 4) as usize]);
    }
    Some((out, [cw, ch]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn picture(w: u32, h: u32, px: [u8; 4]) -> Vec<u8> {
        px.repeat((w * h) as usize)
    }

    #[test]
    fn a_name_is_a_file_and_perhaps_a_crop() {
        assert_eq!(
            split("sprites/tools.png#Axe"),
            ("sprites/tools.png", Some("Axe"))
        );
        assert_eq!(split("cat.png"), ("cat.png", None));
        assert_eq!(split("a#"), ("a", Some("")));
    }

    #[test]
    fn pictures_are_packed_with_their_edge_copied_round_them_and_premultiplied() {
        let mut p = Pictures::new();
        let before = p.revision;
        assert!(p.pixels.is_empty(), "nothing until a picture is added");
        p.add("a", [2, 1], &[255, 0, 0, 255, 0, 0, 255, 128])
            .unwrap();
        assert_ne!(p.revision, before);
        let a = p.get("a").unwrap();
        assert_eq!(
            a,
            Placed {
                size: [2, 1],
                at: [1, 1]
            }
        );
        let px = |x: u32, y: u32| {
            let i = ((y * SIDE + x) * 4) as usize;
            p.pixels[i..i + 4].to_vec()
        };
        assert_eq!(px(1, 1), [255, 0, 0, 255]);
        assert_eq!(px(2, 1), [0, 0, 128, 128], "times its alpha");
        assert_eq!(px(0, 0), [255, 0, 0, 255], "the corner copied out");
        assert_eq!(px(3, 2), [0, 0, 128, 128]);
        // The next goes beside it on the same shelf.
        p.add("b", [3, 3], &picture(3, 3, [9; 4])).unwrap();
        assert_eq!(p.get("b").unwrap().at, [5, 1]);
        let r = p.get("b").unwrap().rect();
        assert_eq!(r, [5.0, 1.0, 8.0, 4.0]);
        let rev = p.revision;
        p.add("b", [1, 1], &[0; 4]).unwrap();
        assert_eq!(p.revision, rev, "added already: kept as it was");
        assert_eq!(p.len(), 2);
    }

    #[test]
    fn a_full_shelf_starts_another_and_a_full_atlas_says_so() {
        let mut p = Pictures::new();
        let big = picture(1000, 1000, [1; 4]);
        p.add("1", [1000, 1000], &big).unwrap();
        p.add("2", [1000, 1000], &big).unwrap();
        p.add("3", [1000, 1000], &big).unwrap();
        assert_eq!(p.get("3").unwrap().at, [1, 1003], "under the first shelf");
        p.add("4", [1000, 1000], &big).unwrap();
        let full = p.add("5", [1000, 1000], &big).unwrap_err();
        assert!(full.contains("no room"), "{full}");
        assert!(p.get("5").is_none());
    }

    #[test]
    fn hostile_pictures_are_refused_with_why() {
        let mut p = Pictures::new();
        assert!(p.add("e", [0, 4], &[]).is_err());
        assert!(p.add("short", [2, 2], &[0; 3]).is_err());
        assert!(
            p.add("huge", [SIDE, 1], &picture(SIDE, 1, [0; 4]))
                .unwrap_err()
                .contains("at most")
        );
        assert!(p.add(&"x".repeat(MAX_NAME + 1), [1, 1], &[0; 4]).is_err());
        assert!(p.is_empty());
    }

    #[test]
    fn a_crop_is_cut_from_its_picture_and_kept_inside_it() {
        // 3 × 2, each pixel its own number.
        let rgba: Vec<u8> = (0..6u8).flat_map(|i| [i; 4]).collect();
        let (part, size) = cut(&rgba, [3, 2], [1, 0, 2, 2]).unwrap();
        assert_eq!(size, [2, 2]);
        assert_eq!(
            part.chunks(4).map(|c| c[0]).collect::<Vec<_>>(),
            [1, 2, 4, 5]
        );
        let (part, size) = cut(&rgba, [3, 2], [2, 1, 9, 9]).unwrap();
        assert_eq!((size, part[0]), ([1, 1], 5), "cut to the picture");
        assert_eq!(cut(&rgba, [3, 2], [3, 0, 1, 1]), None, "outside");
        assert_eq!(cut(&rgba, [3, 2], [0, 0, 0, 1]), None, "empty");
        assert_eq!(cut(&rgba[..4], [3, 2], [0, 0, 1, 1]), None, "wrong length");
    }
}
