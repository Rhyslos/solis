//! The project's own fonts (stage 1, text layout), and a font ready to
//! lay text out with: read with `skrifa` and shaped with `harfrust`.

use crate::font::Font;
use crate::shaping::{Shaped, Shaping};
use crate::text::{MAX_FONT_BYTES, MAX_PROJECT_FONTS};
use std::sync::Arc;

/// A project's own fonts (stage 1, text layout): TrueType and OpenType
/// files from its assets, each checked as it's added, by their place in
/// the assets (`fonts/Lora.ttf`). Screens carry them
/// ([`crate::Screen::fonts`]); every [`crate::Fonts`] laying a screen out takes
/// them from it.
#[derive(Debug, Clone)]
pub struct FontFiles {
    pub(crate) files: Vec<FontFile>,
    pub(crate) revision: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct FontFile {
    pub(crate) name: String,
    pub(crate) bytes: Arc<Vec<u8>>,
    /// The same for this file wherever it's used, so a letter made from
    /// it is never made again because another set holds it.
    pub(crate) uid: u64,
}

impl Default for FontFiles {
    fn default() -> Self {
        Self::new()
    }
}

impl FontFiles {
    /// No fonts yet.
    pub fn new() -> Self {
        Self {
            files: Vec::new(),
            revision: crate::next_revision(),
        }
    }

    /// Adds the font file `name` (its place in the assets): refused, with
    /// why in plain words, when it's too big, isn't a font Solis can read,
    /// or one too many.
    pub fn add(&mut self, name: &str, bytes: Vec<u8>) -> Result<(), String> {
        if self.files.len() >= MAX_PROJECT_FONTS {
            return Err(format!(
                "screens can use at most {MAX_PROJECT_FONTS} fonts of the project's own"
            ));
        }
        check_font(&bytes)?;
        self.files.retain(|f| f.name != name);
        self.files.push(FontFile {
            name: name.to_string(),
            bytes: Arc::new(bytes),
            uid: crate::next_revision(),
        });
        self.revision = crate::next_revision();
        Ok(())
    }

    /// The fonts' names, in the order they were added.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.files.iter().map(|f| f.name.as_str())
    }

    /// How many there are.
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}

impl PartialEq for FontFiles {
    /// The same fonts read the same time.
    fn eq(&self, other: &Self) -> bool {
        self.revision == other.revision
    }
}

/// Why `bytes` can't be a font, if it can't.
fn check_font(bytes: &[u8]) -> Result<(), String> {
    if bytes.len() > MAX_FONT_BYTES {
        return Err(format!(
            "it's bigger than {} MB, the most a font may be",
            MAX_FONT_BYTES >> 20
        ));
    }
    if Font::read(bytes.to_vec()).is_none() {
        return Err("it isn't a TrueType or OpenType font Solis can read".into());
    }
    Ok(())
}

/// One font ready to lay text out with: read, and shaping with.
pub(crate) struct Slot {
    /// Its place in the assets (empty for the game's own fonts).
    pub(crate) name: String,
    /// Its letters' key in the atlas.
    pub(crate) uid: u64,
    pub(crate) font: Font,
    shaping: Shaping,
}

impl Slot {
    pub(crate) fn new(name: &str, uid: u64, font: Font) -> Self {
        let shaping = Shaping::new(font.bytes());
        Self {
            name: name.to_string(),
            uid,
            font,
            shaping,
        }
    }

    /// `run` shaped in this font, or `None` when the font can't be shaped.
    pub(crate) fn shape(&mut self, run: &str) -> Option<Arc<[Shaped]>> {
        let bytes = Arc::clone(self.font.bytes());
        self.shaping.shape(&bytes, run)
    }
}

impl Clone for Slot {
    fn clone(&self) -> Self {
        Self::new(&self.name, self.uid, self.font.clone())
    }
}

impl std::fmt::Debug for Slot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Slot")
            .field("name", &self.name)
            .field("shaping", &self.shaping)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
impl Slot {
    /// How many runs it has shaped and remembers.
    pub(crate) fn shaped(&self) -> usize {
        self.shaping.remembered()
    }
}
