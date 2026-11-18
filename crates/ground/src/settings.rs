//! A ground's settings file (`farm.ground.lst`, decision 0116): a Lust
//! data file (decision 0081) a person can read, holding its look, its
//! size and height, its palette of ground kinds, and the recipe slot the
//! Open worlds phase fills (decision 0117; empty for now). Its voxels are
//! binary region files in the folder beside it ([`crate::region`]).
//!
//! ```text
//! -- A ground (Ground Kind). Written by Solis; safe to edit by hand.
//! fetch "solis/data/ground":[GroundLook, Ground, Kind];
//!
//! lock int version = 1;
//! lock Ground ground = new Ground { look = GroundLook.Smooth, size = [1024, 1024], height = 128, recipe = "" };
//! lock Kind array kinds = [
//!     new Kind { name = "Grass", colour = [92, 138, 64] },
//! ];
//! end
//! ```

use crate::region::SIDE;
use crate::voxel::SIZE_I;
use glam::UVec3;
use level::lust_data::{self as data, DataError};

/// The word before `.lst` in a ground's settings file name.
pub const KIND: &str = "ground";
/// The end of a ground's settings file name.
pub const ENDING: &str = ".ground.lst";
/// The end of the folder of region files beside it (`farm.ground/`).
pub const FOLDER_ENDING: &str = ".ground";
/// The version Solis writes.
pub const VERSION: u32 = 1;
/// The biggest settings file read.
pub const MAX_BYTES: usize = 1 << 20;
/// The widest ground, either way, in metres (2 × 2 km, decision 0116;
/// the Open worlds phase raises it).
pub const MAX_SIDE: u32 = 2048;
/// The tallest ground, in metres (decision 0117 plans 512 m worlds; a
/// new ground is 128 m).
pub const MAX_HEIGHT: u32 = 512;
/// A new ground's size either way: 1 × 1 km (four regions).
pub const STANDARD_SIDE: u32 = 1024;
/// A new ground's height.
pub const STANDARD_HEIGHT: u32 = 128;
/// The most ground kinds a palette holds (one byte a voxel).
pub const MAX_KINDS: usize = 256;
/// The longest kind name, in characters.
pub const MAX_NAME: usize = 32;
/// The longest recipe text.
pub const MAX_RECIPE: usize = 4096;

/// Settings files' version and upgraders (decision 0053): version 1 is
/// the first. Every version keeps an example in `tests/fixtures/ground/`.
pub static FORMAT: reflect::upgrade::Format<Settings> = reflect::upgrade::Format {
    name: "ground",
    current: VERSION,
    version_of: data::version_of,
    steps: &[],
};

/// What a ground's settings file holds, in Lust: written into every
/// project's `solis/data/ground.lst`, and handed to Lust when one is read.
pub const CLASSES: &str = "-- What a ground's settings file holds (Solis writes this; grounds fetch it).\n\
state GroundLook = [Smooth];\n\n\
class Ground {\n    has {\n        GroundLook look = GroundLook.Smooth;\n        int array size = [];\n        int height = 128;\n        string recipe = \"\";\n    }\n}\n\n\
class Kind {\n    has {\n        string name = \"\";\n        int array colour = [];\n    }\n}\n\n\
pass GroundLook, Ground, Kind;\nend\n";

/// How a ground is drawn (decision 0111). Blocky and Sharp come in G3.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Look {
    /// Rounded hills (surface nets).
    #[default]
    Smooth,
}

impl Look {
    /// Its name in the file and on screen.
    pub fn name(self) -> &'static str {
        match self {
            Self::Smooth => "Smooth",
        }
    }
}

/// One ground kind: what a voxel can be made of (painting comes in G4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroundKind {
    /// Its name (`Grass`).
    pub name: String,
    /// Its colour, red, green and blue from 0 to 255.
    pub colour: [u8; 3],
}

/// A ground's settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// How it's drawn.
    pub look: Look,
    /// Its size across and along, in metres: each a whole number of
    /// regions (256 m), up to [`MAX_SIDE`].
    pub size: [u32; 2],
    /// Its height in metres: a whole number of chunks (32 m), up to
    /// [`MAX_HEIGHT`].
    pub height: u32,
    /// Its kinds, at least one, at most [`MAX_KINDS`].
    pub kinds: Vec<GroundKind>,
    /// What makes the ground where nothing's been sculpted (decision
    /// 0117). Empty: the voxels are all in the region files.
    pub recipe: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            look: Look::Smooth,
            size: [STANDARD_SIDE; 2],
            height: STANDARD_HEIGHT,
            kinds: standard_kinds(),
            recipe: String::new(),
        }
    }
}

/// A new ground's palette.
pub fn standard_kinds() -> Vec<GroundKind> {
    [
        ("Grass", [92, 138, 64]),
        ("Rock", [128, 124, 118]),
        ("Dirt", [120, 92, 64]),
        ("Sand", [206, 188, 140]),
    ]
    .into_iter()
    .map(|(name, colour)| GroundKind {
        name: name.into(),
        colour,
    })
    .collect()
}

impl Settings {
    /// How many chunks the ground is across, along and up.
    pub fn extent(&self) -> UVec3 {
        UVec3::new(self.size[0], self.size[1], self.height) / SIZE_I as u32
    }

    /// How many regions it is across and along.
    pub fn regions(&self) -> [u32; 2] {
        self.size.map(|s| s / SIDE)
    }

    /// Whether a size and height are ones a ground can have, and if not,
    /// why not.
    pub fn check_size(size: [u32; 2], height: u32) -> Result<(), String> {
        if size
            .iter()
            .any(|s| *s == 0 || *s > MAX_SIDE || !s.is_multiple_of(SIDE))
        {
            return Err(format!(
                "a ground's size is a whole number of {SIDE} m regions, up to {MAX_SIDE} m either way"
            ));
        }
        if height == 0 || height > MAX_HEIGHT || !height.is_multiple_of(SIZE_I as u32) {
            return Err(format!(
                "a ground's height is a whole number of {SIZE_I} m chunks, up to {MAX_HEIGHT} m"
            ));
        }
        Ok(())
    }
}

/// The folder of region files for the settings file at `path`
/// (`maps/farm.ground.lst` gives `maps/farm.ground`).
pub fn folder_for(path: &str) -> String {
    let stem = path.strip_suffix(ENDING).unwrap_or(path);
    format!("{stem}{FOLDER_ENDING}")
}

/// Whether `path` is a ground's settings file (capitals don't matter).
pub fn is_settings_file(path: &str) -> bool {
    path.to_ascii_lowercase().ends_with(ENDING)
}

/// Writes `settings` as its Lust data file.
pub fn write(settings: &Settings) -> String {
    let mut out = data::header(
        "A ground (Ground Kind). Written by Solis; safe to edit by hand.\n\
         Its voxels are in the folder beside it, a binary file per 256 m region.",
        KIND,
        &["GroundLook", "Ground", "Kind"],
        VERSION,
    );
    out.push_str(&format!(
        "lock Ground ground = {};\n",
        data::thing(
            "Ground",
            &[
                ("look", format!("GroundLook.{}", settings.look.name())),
                ("size", data::ints(settings.size.map(i64::from))),
                ("height", settings.height.to_string()),
                ("recipe", data::text(&settings.recipe)),
            ],
        )
    ));
    let kinds: Vec<String> = settings
        .kinds
        .iter()
        .map(|k| {
            data::thing(
                "Kind",
                &[
                    ("name", data::text(&k.name)),
                    ("colour", data::ints(k.colour.map(i64::from))),
                ],
            )
        })
        .collect();
    data::list(&mut out, "Kind", "kinds", &kinds);
    out.push_str("end\n");
    out
}

/// Settings read back, and what was put right on the way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadSettings {
    /// The settings.
    pub settings: Settings,
    /// What was left out or put right, in plain words.
    pub notes: Vec<String>,
}

/// Reads a settings file, as a hostile file: within [`MAX_BYTES`]; a size
/// or height a ground can't have is a mistake (the region files depend on
/// it); a kind with a bad name or colour, or past [`MAX_KINDS`], is left
/// out with a note. Never panics.
pub fn read(text: &str) -> Result<ReadSettings, DataError> {
    let values = data::read(text, KIND, CLASSES, MAX_BYTES)?;
    if let Some(v) = data::version_of(text).filter(|&v| v > VERSION) {
        return Err(data::error(
            0,
            format!("it's version {v}, made by a newer version of the program"),
        ));
    }
    let head = data::get(&values, "ground")?;
    let look = match data::field_option(head, "look") {
        Some("Smooth") | None => Look::Smooth,
        Some(other) => return Err(data::error(0, format!("it has a look called {other}"))),
    };
    let size = match data::field_ints(head, "size").as_deref() {
        Some(&[x, y]) => [x, y].map(|n| u32::try_from(n).unwrap_or(0)),
        _ => [0, 0],
    };
    let height = data::field_int(head, "height")
        .and_then(|h| u32::try_from(h).ok())
        .unwrap_or(0);
    Settings::check_size(size, height).map_err(|why| data::error(0, why))?;
    let recipe = data::field_text(head, "recipe");
    if recipe.len() > MAX_RECIPE {
        return Err(data::error(0, "its recipe is longer than 4096 characters"));
    }
    let (kinds, mut notes) = read_kinds(&values);
    let mut settings = Settings {
        look,
        size,
        height,
        kinds,
        recipe: recipe.to_string(),
    };
    if settings.kinds.is_empty() {
        notes.push("It had no ground kinds, so it has the standard ones.".into());
        settings.kinds = standard_kinds();
    }
    Ok(ReadSettings { settings, notes })
}

fn read_kinds(values: &data::Values) -> (Vec<GroundKind>, Vec<String>) {
    let mut kinds = Vec::new();
    let mut notes = Vec::new();
    for kind in data::things(values, "kinds") {
        if kinds.len() >= MAX_KINDS {
            notes.push(format!("Only the first {MAX_KINDS} ground kinds are kept."));
            break;
        }
        let name = data::field_text(kind, "name");
        if name.trim().is_empty() || name.chars().count() > MAX_NAME {
            notes.push(format!(
                "A ground kind was left out: a name is 1 to {MAX_NAME} characters."
            ));
            continue;
        }
        let colour = match data::field_ints(kind, "colour").as_deref() {
            Some(&[r, g, b]) if [r, g, b].iter().all(|c| (0..=255).contains(c)) => {
                [r as u8, g as u8, b as u8]
            }
            _ => {
                notes.push(format!(
                    "{name}'s colour wasn't three numbers from 0 to 255, so it's grey."
                ));
                [128, 128, 128]
            }
        };
        kinds.push(GroundKind {
            name: name.to_string(),
            colour,
        });
    }
    (kinds, notes)
}

#[cfg(test)]
#[path = "settings_tests.rs"]
mod tests;
