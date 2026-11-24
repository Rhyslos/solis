//! The clips a sprite can play: its sheet's Aseprite tags, with what its
//! clips file adds to them, then the clips made in Solis.

use crate::clip::{Clip, ClipFrame};
use crate::clips_file::{ClipsFile, Facing};
use assets::sprite_sheet::SpriteSheet;

/// The clips of one sheet (and its clips file, if any).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ClipSet {
    /// The clips: the sheet's tags in Aseprite's order, then those made in
    /// Solis.
    pub clips: Vec<Clip>,
    /// The clip played when a sprite is given no other, by its place in
    /// [`Self::clips`].
    pub first: Option<usize>,
    /// What couldn't be used, in plain words.
    pub notes: Vec<String>,
    /// The rows of each clip with directions, by its name: each facing
    /// with its frames, and whether they're drawn flipped (a mirrored west
    /// side). A clip with one direction has none here.
    pub ways: std::collections::HashMap<String, Vec<(Facing, Clip, bool)>>,
}

impl ClipSet {
    /// The clips of `sheet`, with `file`'s changes and clips if there's
    /// one.
    pub fn of_sheet(sheet: &SpriteSheet, file: Option<&ClipsFile>) -> Self {
        let mut set = Self::default();
        let count = sheet.frames.len() as u32;
        for tag in &sheet.tags {
            if set.get(&tag.name).is_some() {
                set.notes.push(format!(
                    "the sheet has two tags called \"{}\"; the second is left out",
                    tag.name
                ));
                continue;
            }
            let change = file.and_then(|f| f.tags.iter().find(|t| t.name == tag.name));
            let last = tag.to.min(count.saturating_sub(1));
            let frames: Vec<ClipFrame> = (tag.from..=last)
                .filter(|_| count > 0)
                .map(|i| {
                    let f = &sheet.frames[i as usize];
                    let mut events = f.events.clone();
                    // Solis's events after Aseprite's, each once.
                    if let Some(change) = change {
                        for (_, e) in change.events.iter().filter(|(at, _)| *at == i) {
                            if !events.contains(e) {
                                events.push(e.clone());
                            }
                        }
                    }
                    ClipFrame {
                        frame: i,
                        ms: f.duration_ms,
                        events,
                    }
                })
                .collect();
            set.clips.push(Clip::new(
                tag.name.clone(),
                frames,
                tag.direction,
                tag.repeats,
                change.map_or(1.0, |c| c.speed),
                true,
            ));
        }
        if let Some(file) = file {
            for change in &file.tags {
                if !sheet.tags.iter().any(|t| t.name == change.name) {
                    set.notes.push(format!(
                        "the sheet has no tag called \"{}\" any more; what was added to it is \
                         kept but not used",
                        change.name
                    ));
                }
            }
            for own in &file.clips {
                if set.get(&own.name).is_some() {
                    set.notes.push(format!(
                        "a clip made here is called \"{}\", as one of the sheet's tags is; \
                         the tag is used",
                        own.name
                    ));
                    continue;
                }
                let ms_of =
                    |cell: u32| sheet.frames.get(cell as usize).map_or(1, |f| f.duration_ms);
                let all = own.frames_with(ms_of);
                let before = all.len();
                let frames: Vec<ClipFrame> = all.into_iter().filter(|f| f.frame < count).collect();
                if frames.len() < before {
                    set.notes.push(format!(
                        "{} names frames the sheet doesn't have (it has {count}); they're \
                         left out",
                        own.name
                    ));
                }
                set.clips.push(Clip::new(
                    own.name.clone(),
                    frames,
                    own.playback,
                    own.plays,
                    own.speed,
                    false,
                ));
                if own.directions > 1 {
                    let rows = (own.facings().iter())
                        .map(|&facing| {
                            let (from, flipped) = own.drawn_from(facing);
                            let frames = (own.frames_of(from, ms_of).into_iter())
                                .filter(|f| f.frame < count)
                                .collect();
                            let clip = Clip::new(
                                own.name.clone(),
                                frames,
                                own.playback,
                                own.plays,
                                own.speed,
                                false,
                            );
                            (facing, clip, flipped)
                        })
                        .collect();
                    set.ways.insert(own.name.clone(), rows);
                }
            }
            if !file.first.is_empty() {
                set.first = set.index_of(&file.first);
                if set.first.is_none() {
                    set.notes.push(format!(
                        "the clip played first, \"{}\", isn't one of its clips",
                        file.first
                    ));
                }
            }
        }
        set
    }

    /// The clip called `name`.
    pub fn get(&self, name: &str) -> Option<&Clip> {
        self.clips.iter().find(|c| c.name == name)
    }

    /// The place of the clip called `name`.
    pub fn index_of(&self, name: &str) -> Option<usize> {
        self.clips.iter().position(|c| c.name == name)
    }

    /// The clip called `name` (with no name, the one played first) as a
    /// sprite facing `facing` plays it, and whether it's drawn flipped:
    /// the row nearest the facing (`Facing::nearest_in`, keeping
    /// `showing` where it can), or the clip itself when it has one
    /// direction.
    pub fn facing(
        &self,
        name: &str,
        facing: Facing,
        showing: Option<Facing>,
    ) -> Option<(&Clip, bool)> {
        let clip = self.chosen(name)?;
        let Some(rows) = self.ways.get(&clip.name) else {
            return Some((clip, false));
        };
        let facings: Vec<Facing> = rows.iter().map(|r| r.0).collect();
        let shown = facing.nearest_in(&facings, showing);
        let row = rows.iter().find(|r| r.0 == shown);
        Some(row.map_or((clip, false), |(_, row, flipped)| (row, *flipped)))
    }

    /// The clip a sprite plays: the one called `name`, or with no name,
    /// the one played first.
    pub fn chosen(&self, name: &str) -> Option<&Clip> {
        if name.is_empty() {
            self.first.and_then(|i| self.clips.get(i))
        } else {
            self.get(name)
        }
    }
}

#[cfg(test)]
#[path = "clip_set_tests.rs"]
mod tests;
