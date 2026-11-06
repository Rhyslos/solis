//! Atlas crops (decision 0089; Godot's AtlasTexture): named boxes cut out
//! of a picture, a PNG, JPEG or Aseprite sheet, so one sprite or a few can
//! be used on their own (a screen's icon, an item). Made in the Atlas Crop
//! tab and kept in a Lust data file beside the picture (decision 0081),
//! never changing the picture:
//!
//! ```text
//! -- Pictures cut from tools.png (Atlas Crop). Written by Solis; safe to edit by hand.
//! fetch "solis/data/atlas":[Atlas, Crop];
//!
//! lock int version = 1;
//! lock Atlas atlas = new Atlas { picture = "tools.png", grid = [16, 16] };
//! lock Crop array crops = [
//!     new Crop { name = "Axe", box = [0, 0, 16, 16] },
//! ];
//! end
//! ```
//!
//! Game code names a crop with the picture's path and its name
//! (`Picture.Sprites.Tools.Axe` for `sprites/tools.png`'s Axe).

use area::lust_data::{self as data, DataError};

/// The word before `.lst` in an atlas file's name (`tools.atlas.lst`).
pub const KIND: &str = "atlas";

/// The version Solis writes.
pub const VERSION: u32 = 1;

/// Atlas files' version and upgraders (decision 0053): version 1 is the
/// first, so there's nothing to upgrade yet. Every version keeps an
/// example in `tests/fixtures/atlas/`.
pub static FORMAT: reflect::upgrade::Format<Atlas> = reflect::upgrade::Format {
    name: "atlas",
    current: VERSION,
    version_of: data::version_of,
    steps: &[],
};

/// The most crops one picture has.
pub const MAX_CROPS: usize = 4096;

/// The longest crop name.
pub const MAX_NAME: usize = 64;

/// The biggest picture a crop can be in, either way, in pixels.
pub const MAX_SIDE: u32 = 16_384;

/// The biggest atlas file read.
pub const MAX_BYTES: usize = 4 << 20;

/// What an atlas file holds, in Lust: written into every project's
/// `solis/data/atlas.lst`, and handed to Lust when one is read.
pub const CLASSES: &str = "-- What an atlas file holds (Solis writes this; atlas files fetch it).\n\
class Atlas {\n    has {\n        string picture = \"\";\n        int array grid = [];\n    }\n}\n\n\
class Crop {\n    has {\n        string name = \"\";\n        int array box = [];\n    }\n}\n\n\
pass Atlas, Crop;\nend\n";

/// One crop: a box of the picture, with its name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Crop {
    /// Its name (`Axe`): a letter, then letters, digits and `_`.
    pub name: String,
    /// Its box in the picture's pixels: left, top, width, height.
    pub rect: [u32; 4],
}

/// The crops of one picture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Atlas {
    /// The picture's file name, beside the atlas file (`tools.png`).
    pub picture: String,
    /// The grid crops snap to, in pixels (each side at least 1).
    pub grid: [u32; 2],
    /// The crops, in order.
    pub crops: Vec<Crop>,
}

impl Atlas {
    /// An atlas of `picture` with no crops yet, on a 16 × 16 grid.
    pub fn new(picture: &str) -> Self {
        Self {
            picture: picture.to_string(),
            grid: [16, 16],
            crops: Vec::new(),
        }
    }

    /// The crop called `name` (capitals don't matter).
    pub fn crop(&self, name: &str) -> Option<&Crop> {
        self.crops
            .iter()
            .find(|c| c.name.eq_ignore_ascii_case(name))
    }

    /// A name no crop has yet: `stem`, or `stem2`, `stem3`…
    pub fn free_name(&self, stem: &str) -> String {
        std::iter::once(stem.to_string())
            .chain((2..).map(|n| format!("{stem}{n}")))
            .find(|n| self.crop(n).is_none())
            .unwrap_or_default()
    }

    /// One crop for every grid cell of a picture `size` pixels big that
    /// has something in it (`filled` says, by the cell's box), named after
    /// its place (`Tile_0_0`, by column and row), leaving out cells a crop
    /// already covers exactly; at most [`MAX_CROPS`] in all. Gives how
    /// many were added.
    pub fn crop_every_tile(&mut self, size: [u32; 2], filled: &dyn Fn([u32; 4]) -> bool) -> usize {
        let [gw, gh] = self.grid.map(|g| g.max(1));
        let mut added = 0;
        for row in 0..size[1] / gh {
            for column in 0..size[0] / gw {
                if self.crops.len() >= MAX_CROPS {
                    return added;
                }
                let rect = [column * gw, row * gh, gw, gh];
                if self.crops.iter().any(|c| c.rect == rect) || !filled(rect) {
                    continue;
                }
                let name = self.free_name(&format!("Tile_{column}_{row}"));
                self.crops.push(Crop { name, rect });
                added += 1;
            }
        }
        added
    }
}

/// Whether `name` can name a crop: a letter, then at most
/// [`MAX_NAME`] letters, digits and `_` in all.
pub fn good_name(name: &str) -> bool {
    let mut chars = name.chars();
    name.len() <= MAX_NAME
        && chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// The atlas file's path for a picture's (`sprites/tools.png` gives
/// `sprites/tools.atlas.lst`).
pub fn file_for(picture: &str) -> String {
    let stem = picture.rsplit_once('.').map_or(picture, |(s, _)| s);
    format!("{stem}.{KIND}.lst")
}

/// The picture's path (inside the assets) for an atlas file at `path`
/// holding `atlas`: its file name, in the same folder.
pub fn picture_path(path: &str, atlas: &Atlas) -> String {
    match path.rsplit_once('/') {
        Some((folder, _)) => format!("{folder}/{}", atlas.picture),
        None => atlas.picture.clone(),
    }
}

/// Writes `atlas` as its Lust data file.
pub fn write(atlas: &Atlas) -> String {
    let mut out = data::header(
        &format!(
            "Pictures cut from {} (Atlas Crop). Written by Solis; safe to edit by hand.",
            atlas.picture
        ),
        KIND,
        &["Atlas", "Crop"],
        VERSION,
    );
    out.push_str(&format!(
        "lock Atlas atlas = {};\n",
        data::thing(
            "Atlas",
            &[
                ("picture", data::text(&atlas.picture)),
                ("grid", data::ints(atlas.grid.iter().map(|&g| i64::from(g))),),
            ],
        )
    ));
    let crops: Vec<String> = atlas
        .crops
        .iter()
        .map(|c| {
            data::thing(
                "Crop",
                &[
                    ("name", data::text(&c.name)),
                    ("box", data::ints(c.rect.iter().map(|&v| i64::from(v)))),
                ],
            )
        })
        .collect();
    data::list(&mut out, "Crop", "crops", &crops);
    out.push_str("end\n");
    out
}

/// An atlas read back, and what was put right or left out.
#[derive(Debug, Clone, PartialEq)]
pub struct ReadAtlas {
    /// The atlas.
    pub atlas: Atlas,
    /// What was left out or put right, in plain words.
    pub notes: Vec<String>,
}

/// Reads an atlas file, as a hostile file: within [`MAX_BYTES`], a crop
/// with a bad name, a box past [`MAX_SIDE`] or empty, or a name used twice
/// is left out with a note. Never panics.
pub fn read(text: &str) -> Result<ReadAtlas, DataError> {
    let values = data::read(text, KIND, CLASSES, MAX_BYTES)?;
    if let Some(v) = data::version_of(text).filter(|&v| v > VERSION) {
        return Err(data::error(
            0,
            format!("it's version {v}, made by a newer Solis"),
        ));
    }
    let head = data::get(&values, "atlas")?;
    let picture = data::field_text(head, "picture").to_string();
    if picture.is_empty() || picture.contains(['/', '\\']) || picture.contains("..") {
        return Err(data::error(
            0,
            "its picture must be a file beside it (a name like tools.png)",
        ));
    }
    let mut notes = Vec::new();
    let grid = match data::field_ints(head, "grid").as_deref() {
        Some(&[w, h])
            if (1..=i64::from(MAX_SIDE)).contains(&w) && (1..=i64::from(MAX_SIDE)).contains(&h) =>
        {
            [w as u32, h as u32]
        }
        _ => {
            notes.push("Its grid wasn't two sizes from 1 to 16384, so it's 16 × 16.".to_string());
            [16, 16]
        }
    };
    let mut atlas = Atlas {
        picture,
        grid,
        crops: Vec::new(),
    };
    for crop in data::things(&values, "crops") {
        if atlas.crops.len() >= MAX_CROPS {
            notes.push(format!("Only the first {MAX_CROPS} crops are kept."));
            break;
        }
        let name = data::field_text(crop, "name");
        if !good_name(name) {
            notes.push(format!(
                "The crop \"{}\" was left out: a name starts with a letter and holds only \
                 letters, digits and _.",
                name.chars().take(MAX_NAME).collect::<String>()
            ));
            continue;
        }
        if atlas.crop(name).is_some() {
            notes.push(format!("A second crop called {name} was left out."));
            continue;
        }
        let rect = match data::field_ints(crop, "box").as_deref() {
            Some(&[x, y, w, h])
                if [x, y].iter().all(|v| (0..i64::from(MAX_SIDE)).contains(v))
                    && [w, h].iter().all(|v| (1..=i64::from(MAX_SIDE)).contains(v)) =>
            {
                [x as u32, y as u32, w as u32, h as u32]
            }
            _ => {
                notes.push(format!(
                    "The crop {name} was left out: its box isn't left, top, width and height \
                     inside a picture of at most 16384 pixels."
                ));
                continue;
            }
        };
        atlas.crops.push(Crop {
            name: name.to_string(),
            rect,
        });
    }
    Ok(ReadAtlas { atlas, notes })
}

#[cfg(test)]
#[path = "atlas_tests.rs"]
mod tests;
