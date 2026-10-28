//! The clips a sprite can play: its sheet's Aseprite tags, with what its
//! clips file adds to them, then the clips made in Solis.

use crate::clip::{Clip, ClipFrame};
use crate::frames_file::FramesFile;
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
}

impl ClipSet {
    /// The clips of `sheet`, with `file`'s changes and clips if there's
    /// one.
    pub fn of_sheet(sheet: &SpriteSheet, file: Option<&FramesFile>) -> Self {
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
                    if let Some(change) = change {
                        events.extend(
                            change
                                .events
                                .iter()
                                .filter(|(at, _)| *at == i)
                                .map(|(_, e)| e.clone()),
                        );
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
                let before = own.frames.len();
                let frames: Vec<ClipFrame> = own
                    .frames
                    .iter()
                    .filter(|f| f.frame < count)
                    .cloned()
                    .collect();
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
                    own.direction,
                    own.repeats,
                    own.speed,
                    false,
                ));
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
