//! Shaping (stage 1, text layout): a run of text in one font turned into
//! that font's letters and where each goes, with `harfrust`, so pairs of
//! letters kern and letters join as the font says (`fi`, `ffl`, accents
//! placed over their letters).
//!
//! Each run shaped is remembered, by its words, in the font's own units:
//! text that changes every frame (a score, a list) is mostly words seen
//! before, so laying it out again costs a look-up, not a shaping (the
//! study, `docs/reports/text-layout-study.md`: 2,000 changing lines in
//! under a millisecond, against 21 shaped afresh).

use crate::font::GlyphId;
use harfrust::{Direction, FontRef, Script, ShapeOptions, ShapePlan, ShaperData, UnicodeBuffer};
use std::collections::HashMap;
use std::sync::Arc;

/// The most runs a font remembers; past it it starts again.
pub(crate) const MAX_RUNS: usize = 1 << 14;
/// The longest run remembered, in bytes: longer ones (a word with no
/// break in it, pasted) are shaped each time they're laid out.
const MAX_REMEMBERED_RUN: usize = 128;

/// One letter shaped, in the font's units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Shaped {
    /// The font's letter.
    pub(crate) id: GlyphId,
    /// How far the pen moves after it.
    pub(crate) advance: f32,
    /// Where it sits from the pen: right, and up.
    pub(crate) offset: [f32; 2],
    /// Where its characters start in the run, in bytes.
    pub(crate) cluster: u32,
}

/// What one font shapes with, and the runs it has shaped.
pub(crate) struct Shaping {
    /// The shaper's tables, read once; none when the file can't be
    /// shaped (each character then gets its own letter, unkerned).
    data: Option<ShaperData>,
    /// A plan per writing system and way, made once.
    plans: HashMap<(Script, Direction), ShapePlan>,
    /// Runs shaped, by their text.
    runs: HashMap<Box<str>, Arc<[Shaped]>>,
}

impl std::fmt::Debug for Shaping {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Shaping")
            .field("shapes", &self.data.is_some())
            .field("runs", &self.runs.len())
            .finish_non_exhaustive()
    }
}

impl Shaping {
    /// Ready to shape with the font in `bytes`.
    pub(crate) fn new(bytes: &[u8]) -> Self {
        let data = caught(|| {
            FontRef::new(bytes)
                .ok()
                .map(|font| ShaperData::new(&font))
        })
        .flatten();
        Self {
            data,
            plans: HashMap::new(),
            runs: HashMap::new(),
        }
    }

    /// How many runs it remembers.
    #[cfg(test)]
    pub(crate) fn remembered(&self) -> usize {
        self.runs.len()
    }

    /// `run` shaped in the font in `bytes` (the same bytes it was made
    /// with), or `None` when the font can't be shaped.
    pub(crate) fn shape(&mut self, bytes: &[u8], run: &str) -> Option<Arc<[Shaped]>> {
        if let Some(found) = self.runs.get(run) {
            return Some(found.clone());
        }
        let data = self.data.as_ref()?;
        let plans = &mut self.plans;
        let shaped: Option<Arc<[Shaped]>> = caught(|| {
            let font = FontRef::new(bytes).ok()?;
            let shaper = data.shaper(&font).build();
            let mut buffer = UnicodeBuffer::new();
            buffer.push_str(run);
            buffer.guess_segment_properties();
            let key = (buffer.script(), buffer.direction());
            let plan = plans
                .entry(key)
                .or_insert_with(|| ShapePlan::new(&shaper, key.1, Some(key.0), None, &[]));
            let out = shaper.shape(buffer, ShapeOptions::new().plan(Some(plan)));
            let letters: Vec<Shaped> = out
                .glyph_infos()
                .iter()
                .zip(out.glyph_positions())
                .map(|(info, at)| Shaped {
                    id: GlyphId::new(info.glyph_id),
                    advance: at.x_advance as f32,
                    offset: [at.x_offset as f32, at.y_offset as f32],
                    cluster: info.cluster,
                })
                .collect();
            Some(letters.into())
        })
        .flatten();
        let Some(shaped) = shaped else {
            // A font that broke the shaper is never shaped with again.
            self.data = None;
            return None;
        };
        if run.len() <= MAX_REMEMBERED_RUN {
            if self.runs.len() >= MAX_RUNS {
                self.runs.clear();
            }
            self.runs.insert(run.into(), shaped.clone());
        }
        Some(shaped)
    }
}

/// `work`'s answer, or `None` if it panicked: a font is a file from
/// anywhere (rule 9), and a damaged one must never stop the game.
fn caught<T>(work: impl FnOnce() -> T) -> Option<T> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(work)).ok()
}
