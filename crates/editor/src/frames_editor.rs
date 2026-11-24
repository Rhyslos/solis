//! A sprite's clips file open for editing (Phase 9, D3): the Clips tab
//! beside Properties lists its clips (the sheet's Aseprite tags and clips
//! made here) with their speed, direction and repeats; the Frames tab at
//! the bottom shows the chosen clip's frames along a time ruler, each as
//! wide as it lasts, with their events in a lane below, and plays it.
//! Both tabs come when a `.clips.lst` file is opened and go when it's
//! closed (the project lead's design, from a mock-up).
//!
//! Every change is saved at once (the app writes the file) and shown at
//! once on every sprite playing the file; Ctrl+Z and Ctrl+Y over either
//! tab undo and redo the file's changes.

use crate::file_history::FileHistory;
use animation::clips_file::{self, Clip as OwnClip, ClipsFile, Facing, TagChange};
use animation::frames_file;
use animation::{Clip, ClipFrame, ClipSet, Direction};
use assets::sprite_sheet::SpriteSheet;
use reflect::AssetId;

/// The most changes kept to undo.
const UNDO_LIMIT: usize = 200;

/// A clips file open in the Clips and Frames tabs.
#[derive(Debug, Clone)]
pub struct FramesEditor {
    /// The file, by path inside the assets (`/` between folders).
    pub path: String,
    /// The file as it is now.
    pub file: ClipsFile,
    /// The main look's file name (`knight.png`), for the file's first line.
    pub picture: String,
    /// The clip chosen in the Clips tab, by name.
    pub clip: String,
    /// The chosen clip's frames picked in the Frames tab, by their place
    /// in the clip, in order.
    pub selected: Vec<usize>,
    /// Where a Shift+click picks from.
    pub anchor: Option<usize>,
    /// Whether the clip is playing in the Frames tab (and on sprites
    /// playing the file, if Preview animation is on).
    pub playing: bool,
    /// How far the clip has played in the Frames tab, in seconds.
    pub time: f64,
    /// Screen points a second of clip takes in the Frames tab.
    pub zoom: f32,
    /// A clip's name being typed (renaming), with the clip's old name.
    pub renaming: Option<(String, String)>,
    /// The Clips tab's search.
    pub search: String,
    /// Cut picture… was pressed: the workbench opens the pop-up.
    pub cut_asked: bool,
    /// The chosen clip's row the Frames tab picks frames in.
    pub facing: Facing,
    /// The cells picked on the Cells page, in order: a run to drop on a
    /// row.
    pub cells_picked: Vec<u32>,
    history: FileHistory<ClipsFile>,
}

/// The standard zoom: a 100 ms frame 100 points wide.
pub const STANDARD_ZOOM: f32 = 1000.0;
/// How far the Frames tab zooms out and in.
pub const ZOOM_RANGE: (f32, f32) = (250.0, 4000.0);

impl FramesEditor {
    /// `file`, at `path`, open with its first clip (or the one played
    /// first) chosen.
    pub fn new(path: String, file: ClipsFile, clips: &ClipSet) -> Self {
        let clip = clips
            .first
            .and_then(|i| clips.clips.get(i))
            .or_else(|| clips.clips.first())
            .map(|c| c.name.clone())
            .unwrap_or_default();
        // The picture's own name comes once the sheet is known (see
        // `Workbench::open_frames`); until then, the file's.
        let picture = path.rsplit('/').next().unwrap_or_default().to_owned();
        let mut editor = Self {
            path,
            file,
            picture,
            clip,
            selected: Vec::new(),
            anchor: None,
            playing: false,
            time: 0.0,
            zoom: STANDARD_ZOOM,
            renaming: None,
            search: String::new(),
            cut_asked: false,
            facing: Facing::Any,
            cells_picked: Vec::new(),
            history: FileHistory::new(UNDO_LIMIT),
        };
        editor.pick_row(None);
        editor
    }

    /// The clips of the file over `sheet`, as they are now.
    pub fn clips(&self, sheet: &SpriteSheet) -> ClipSet {
        ClipSet::of_sheet(sheet, Some(&self.file))
    }

    /// Changes the file with `change`, kept for undo. Returns whether it
    /// changed (and so must be saved).
    pub fn change(&mut self, change: impl FnOnce(&mut ClipsFile)) -> bool {
        self.history.change(&mut self.file, |f| {
            change(f);
            true
        })
    }

    /// Undoes the last change. Returns whether there was one.
    pub fn undo(&mut self) -> bool {
        let undone = self.history.undo(&mut self.file);
        if undone {
            self.selected.clear();
        }
        undone
    }

    /// Redoes the last change undone. Returns whether there was one.
    pub fn redo(&mut self) -> bool {
        let redone = self.history.redo(&mut self.file);
        if redone {
            self.selected.clear();
        }
        redone
    }

    /// Whether there's a change to undo, and one to redo.
    pub fn can_undo_redo(&self) -> (bool, bool) {
        (self.history.can_undo(), self.history.can_redo())
    }

    /// Chooses the clip called `name`: back to its start, nothing picked.
    pub fn choose(&mut self, name: &str) {
        if self.clip != name {
            self.clip = name.to_owned();
            self.time = 0.0;
            self.pick_row(None);
        }
    }

    /// The file as text, to save.
    pub fn text(&self) -> String {
        clips_file::write(&self.file, &self.picture)
    }

    /// Takes in the file as it is on disk, as one undo step: nothing
    /// stays picked, since its frames may not be the ones picked.
    pub(crate) fn taken_in(&mut self, file: ClipsFile) {
        if self.change(|f| *f = file) {
            self.selected.clear();
            self.anchor = None;
            self.renaming = None;
        }
    }

    /// A name for a new clip, not yet taken in `clips`: `base`, or `base 2`
    /// and so on.
    pub fn free_name(clips: &ClipSet, base: &str) -> String {
        if clips.get(base).is_none() {
            return base.to_owned();
        }
        (2..)
            .map(|n| format!("{base} {n}"))
            .find(|n| clips.get(n).is_none())
            .unwrap_or_else(|| base.to_owned())
    }

    /// Adds a clip made here, with `frames` (none for an empty one), and
    /// chooses it. Returns whether it was added.
    pub fn new_clip(&mut self, clips: &ClipSet, frames: Vec<ClipFrame>) -> bool {
        if clips.clips.len() >= clips_file::MAX_CLIPS {
            return false;
        }
        let name = Self::free_name(clips, "clip");
        let added = self.change(|f| {
            f.clips.push(OwnClip::one_way(
                name.clone(),
                1.0,
                Direction::Forward,
                0,
                &frames,
            ));
        });
        self.choose(&name);
        added
    }

    /// Adds a clip made here that copies the chosen one (an Aseprite tag's
    /// frames, times and events, to change here), and chooses it.
    pub fn copy_clip(&mut self, clips: &ClipSet) -> bool {
        let Some(clip) = clips.get(&self.clip) else {
            return false;
        };
        if clips.clips.len() >= clips_file::MAX_CLIPS {
            return false;
        }
        let name = Self::free_name(clips, &format!("{} copy", clip.name));
        let frames: Vec<ClipFrame> = clip
            .frames
            .iter()
            .take(clips_file::MAX_FRAMES)
            .map(|f| ClipFrame {
                frame: f.frame,
                ms: f.ms,
                // One event a frame in a clip made here.
                events: f.events.first().cloned().into_iter().collect(),
            })
            .collect();
        let own = OwnClip::one_way(
            name.clone(),
            clip.speed,
            clip.direction,
            clip.repeats,
            &frames,
        );
        let added = self.change(|f| f.clips.push(own));
        self.choose(&name);
        added
    }

    /// Takes the chosen clip away, if it was made here (a tag stays: it's
    /// Aseprite's); what Solis added to a tag is forgotten instead.
    pub fn delete_clip(&mut self) -> bool {
        let name = self.clip.clone();
        let changed = self.change(|f| {
            f.clips.retain(|c| c.name != name);
            f.tags.retain(|t| t.name != name);
            if f.first == name {
                f.first.clear();
            }
        });
        if changed {
            self.selected.clear();
        }
        changed
    }

    /// Renames the clip made here called `old` to `new` (trimmed), if it's
    /// free. Returns whether it changed.
    pub fn rename(&mut self, clips: &ClipSet, old: &str, new: &str) -> bool {
        let new = frames_file::clean_name(new);
        if new.is_empty() || new == old || clips.get(&new).is_some() {
            return false;
        }
        let changed = self.change(|f| {
            if let Some(c) = f.clips.iter_mut().find(|c| c.name == old) {
                c.name.clone_from(&new);
                if f.first == old {
                    f.first.clone_from(&new);
                }
            }
        });
        if changed && self.clip == old {
            self.clip = new;
        }
        changed
    }

    /// Makes the chosen clip the one played first (or none, if it was).
    pub fn toggle_first(&mut self) -> bool {
        let name = self.clip.clone();
        self.change(|f| {
            if f.first == name {
                f.first.clear();
            } else {
                f.first = name;
            }
        })
    }

    /// Sets the chosen clip's speed.
    pub fn set_speed(&mut self, speed: f32) -> bool {
        let (lo, hi) = frames_file::SPEED_RANGE;
        let speed = if speed.is_finite() {
            speed.clamp(lo, hi)
        } else {
            1.0
        };
        let name = self.clip.clone();
        self.change(|f| {
            if let Some(c) = f.clips.iter_mut().find(|c| c.name == name) {
                c.speed = speed;
            } else {
                tag_change(f, &name).speed = speed;
                tidy_tags(f);
            }
        })
    }

    /// Sets the chosen clip's direction (clips made here).
    pub fn set_direction(&mut self, direction: Direction) -> bool {
        let name = self.clip.clone();
        self.change(|f| {
            if let Some(c) = f.clips.iter_mut().find(|c| c.name == name) {
                c.playback = direction;
            }
        })
    }

    /// Gives the chosen clip (made here) one, four or eight directions,
    /// and picks the row it shows first.
    pub fn set_directions(&mut self, directions: u8) -> bool {
        let name = self.clip.clone();
        let changed = self.change(|f| {
            if let Some(c) = f.clip_mut(&name) {
                c.set_directions(directions);
            }
        });
        if changed {
            self.pick_row(None);
        }
        changed
    }

    /// Fills the chosen clip's (made here) row for `facing` with the cells
    /// `cells`, in order, and picks the row. A frame keeps the time and
    /// event its place had; a new place takes the cell's time from
    /// `sheet`.
    pub fn fill_row(&mut self, facing: Facing, cells: &[u32], sheet: &SpriteSheet) -> bool {
        let name = self.clip.clone();
        let cells: Vec<u32> = (cells.iter().copied())
            .filter(|&n| (n as usize) < sheet.frames.len())
            .take(clips_file::MAX_FRAMES)
            .collect();
        if cells.is_empty() {
            return false;
        }
        let changed = self.change(|f| {
            if let Some(c) = f.clip_mut(&name) {
                c.edit_frames_of(facing, |frames| {
                    let before = std::mem::take(frames);
                    *frames = (cells.iter().enumerate())
                        .map(|(i, &cell)| match before.get(i) {
                            Some(was) => ClipFrame {
                                frame: cell,
                                ..was.clone()
                            },
                            None => ClipFrame {
                                frame: cell,
                                ms: sheet.frames[cell as usize]
                                    .duration_ms
                                    .clamp(1, clips_file::MAX_MS),
                                events: Vec::new(),
                            },
                        })
                        .collect();
                });
            }
        });
        if changed {
            self.pick_row(Some(facing));
        }
        changed
    }

    /// Shows `cell` in the picked frames of the chosen clip's (made here)
    /// picked row, keeping their times and events.
    pub fn swap_cell(&mut self, cell: u32) -> bool {
        let name = self.clip.clone();
        let facing = self.row_facing();
        let picked = self.selected.clone();
        if picked.is_empty() {
            return false;
        }
        self.change(|f| {
            if let Some(c) = f.clip_mut(&name) {
                c.edit_frames_of(facing, |frames| {
                    for &i in &picked {
                        if let Some(frame) = frames.get_mut(i) {
                            frame.frame = cell;
                        }
                    }
                });
            }
        })
    }

    /// The row the picked facing's frames are in: a mirrored clip's west
    /// side is its east side's row.
    pub fn row_facing(&self) -> Facing {
        (self.file.clips.iter().find(|c| c.name == self.clip))
            .map_or(self.facing, |c| c.drawn_from(self.facing).0)
    }

    /// Turns the chosen clip's (made here) same frames in every direction
    /// on or off.
    pub fn set_same_frames(&mut self, same: bool) -> bool {
        let name = self.clip.clone();
        self.change(|f| {
            if let Some(c) = f.clip_mut(&name) {
                c.set_same_frames(same);
            }
        })
    }

    /// Turns the chosen clip's (made here) mirrored west side on or off.
    pub fn set_mirror(&mut self, mirror: bool) -> bool {
        let name = self.clip.clone();
        self.change(|f| {
            if let Some(c) = f.clip_mut(&name) {
                c.set_mirror(mirror);
            }
        })
    }

    /// Picks the chosen clip's row for `facing` (its first, for none),
    /// with nothing picked in it.
    pub fn pick_row(&mut self, facing: Option<Facing>) {
        let clip = self.file.clips.iter().find(|c| c.name == self.clip);
        self.facing = facing
            .or_else(|| clip.map(|c| c.facings()[0]))
            .unwrap_or(Facing::Any);
        self.selected.clear();
        self.anchor = None;
    }

    /// Sets how many times the chosen clip plays (clips made here; 0 over
    /// and over).
    pub fn set_repeats(&mut self, repeats: u32) -> bool {
        let name = self.clip.clone();
        let repeats = repeats.min(clips_file::MAX_PLAYS);
        self.change(|f| {
            if let Some(c) = f.clips.iter_mut().find(|c| c.name == name) {
                c.plays = repeats;
            }
        })
    }

    /// Sets how long the picked frames of the chosen clip show (clips made
    /// here).
    pub fn set_ms(&mut self, ms: u32) -> bool {
        let name = self.clip.clone();
        let facing = self.row_facing();
        let picked = self.selected.clone();
        let ms = ms.clamp(1, clips_file::MAX_MS);
        self.change(|f| {
            if let Some(c) = f.clip_mut(&name) {
                c.edit_frames_of(facing, |frames| {
                    for &i in &picked {
                        if let Some(frame) = frames.get_mut(i) {
                            frame.ms = ms;
                        }
                    }
                });
            }
        })
    }

    /// Sets the event the picked frames of the chosen clip start (empty
    /// for none). In a clip made here it's the frame's event; on an
    /// Aseprite tag it's an event Solis adds to that frame (Aseprite's own
    /// stay).
    pub fn set_event(&mut self, clips: &ClipSet, event: &str) -> bool {
        let event = frames_file::clean_name(event);
        let Some(clip) = clips.get(&self.clip) else {
            return false;
        };
        let name = self.clip.clone();
        let facing = self.row_facing();
        let picked: Vec<(usize, u32)> = self
            .selected
            .iter()
            .filter_map(|&i| Some((i, clip.frames.get(i)?.frame)))
            .collect();
        let own = !clip.from_aseprite;
        self.change(|f| {
            if own {
                if let Some(c) = f.clip_mut(&name) {
                    c.edit_frames_of(facing, |frames| {
                        for &(i, _) in &picked {
                            if let Some(frame) = frames.get_mut(i) {
                                frame.events = if event.is_empty() {
                                    Vec::new()
                                } else {
                                    vec![event.clone()]
                                };
                            }
                        }
                    });
                }
            } else {
                let change = tag_change(f, &name);
                for &(_, sheet_frame) in &picked {
                    change.events.retain(|(at, _)| *at != sheet_frame);
                    if !event.is_empty() && change.events.len() < frames_file::MAX_TAG_EVENTS {
                        change.events.push((sheet_frame, event.clone()));
                    }
                }
                change.events.sort_by_key(|(at, _)| *at);
                tidy_tags(f);
            }
        })
    }

    /// Adds the sheet's frames `frames` (from 0) to the chosen clip (made
    /// here), after the last picked frame or at the end, each for the
    /// time Aseprite gives it, and picks them.
    pub fn add_frames(&mut self, sheet: &SpriteSheet, frames: &[u32]) -> bool {
        let name = self.clip.clone();
        let facing = self.row_facing();
        let Some(len) = self
            .file
            .clips
            .iter()
            .find(|c| c.name == name)
            .map(|c| c.frames_of(self.row_facing(), |_| 1).len())
        else {
            return false;
        };
        let at = self.selected.last().map_or(len, |&i| (i + 1).min(len));
        let room = clips_file::MAX_FRAMES.saturating_sub(len);
        let adding: Vec<ClipFrame> = frames
            .iter()
            .filter_map(|&n| {
                let f = sheet.frames.get(n as usize)?;
                Some(ClipFrame {
                    frame: n,
                    ms: f.duration_ms.clamp(1, clips_file::MAX_MS),
                    events: Vec::new(),
                })
            })
            .take(room)
            .collect();
        let count = adding.len();
        let changed = self.change(|f| {
            if let Some(c) = f.clip_mut(&name) {
                c.edit_frames_of(facing, |frames| {
                    frames.splice(at..at, adding);
                });
            }
        });
        if changed {
            self.selected = (at..at + count).collect();
            self.anchor = Some(at);
        }
        changed
    }

    /// Takes the picked frames out of the chosen clip (made here).
    pub fn delete_frames(&mut self) -> bool {
        let name = self.clip.clone();
        let facing = self.row_facing();
        let picked = self.selected.clone();
        let changed = self.change(|f| {
            if let Some(c) = f.clip_mut(&name) {
                c.edit_frames_of(facing, |frames| {
                    let mut i = 0;
                    frames.retain(|_| {
                        let keep = !picked.contains(&i);
                        i += 1;
                        keep
                    });
                });
            }
        });
        if changed {
            self.selected.clear();
            self.anchor = None;
        }
        changed
    }

    /// Moves the picked frames of the chosen clip (made here) to before
    /// the frame now at `to` (the clip's length for the end), keeping
    /// their order, and keeps them picked.
    pub fn move_frames(&mut self, to: usize) -> bool {
        let name = self.clip.clone();
        let facing = self.row_facing();
        let picked = self.selected.clone();
        let mut placed = Vec::new();
        let changed = self.change(|f| {
            let Some(c) = f.clip_mut(&name) else {
                return;
            };
            let mut frames = c.frames_of(facing, |_| clips_file::DEFAULT_MS);
            let to = to.min(frames.len());
            let mut moving = Vec::new();
            let mut rest = Vec::new();
            for (i, frame) in frames.drain(..).enumerate() {
                if picked.contains(&i) {
                    moving.push(frame);
                } else {
                    rest.push(frame);
                }
            }
            // Where `to` lands once the moving frames are out.
            let at = to - picked.iter().filter(|&&i| i < to).count();
            let n = moving.len();
            rest.splice(at..at, moving);
            c.set_frames_of(facing, &rest);
            placed = (at..at + n).collect();
        });
        if changed {
            self.selected = placed;
            self.anchor = self.selected.first().copied();
        }
        changed
    }

    /// Picks frame `i` of the chosen clip: alone, added or taken away
    /// (`toggle`, Ctrl+click), or every frame from the last picked
    /// (`range`, Shift+click).
    pub fn pick(&mut self, i: usize, toggle: bool, range: bool) {
        if range && let Some(from) = self.anchor {
            let (a, b) = if from <= i { (from, i) } else { (i, from) };
            self.selected = (a..=b).collect();
        } else if toggle {
            if let Some(at) = self.selected.iter().position(|&s| s == i) {
                self.selected.remove(at);
            } else {
                self.selected.push(i);
                self.selected.sort_unstable();
            }
            self.anchor = Some(i);
        } else {
            self.selected = vec![i];
            self.anchor = Some(i);
        }
    }

    /// Moves the Frames tab's playing on by `seconds` of real time at the
    /// clip's speed; a clip played a number of times stops at its end.
    pub fn play_on(&mut self, clip: &Clip, seconds: f64) {
        if !self.playing {
            return;
        }
        self.time += seconds.max(0.0) * f64::from(clip.speed);
        match clip.length_seconds() {
            Some(length) if self.time >= length => {
                self.time = length;
                self.playing = false;
            }
            Some(_) => {}
            None => {
                let pass = clip.pass_seconds();
                if pass > 0.0 {
                    self.time %= pass;
                }
            }
        }
    }
}

/// The tag change for the tag `name` in `file`, made if missing.
fn tag_change<'a>(file: &'a mut ClipsFile, name: &str) -> &'a mut TagChange {
    if let Some(i) = file.tags.iter().position(|t| t.name == name) {
        return &mut file.tags[i];
    }
    file.tags.push(TagChange {
        name: name.to_owned(),
        speed: 1.0,
        events: Vec::new(),
    });
    let last = file.tags.len() - 1;
    &mut file.tags[last]
}

/// Drops tag changes that change nothing, so the file stays short.
fn tidy_tags(file: &mut ClipsFile) {
    file.tags
        .retain(|t| (t.speed - 1.0).abs() > f32::EPSILON || !t.events.is_empty());
}

/// The asset ID of the file at `path`, if the project knows it yet (a new
/// file gets one a moment after it's made).
pub fn id_of(sheets: &crate::sprite_sheets::SpriteSheets, path: &str) -> Option<AssetId> {
    sheets
        .paths
        .iter()
        .find(|(_, p)| p.as_str() == path)
        .map(|(id, _)| *id)
}

impl crate::own_files::OpenFile for FramesEditor {
    type File = ClipsFile;

    fn path(&self) -> &str {
        &self.path
    }

    fn file(&self) -> &ClipsFile {
        &self.file
    }

    fn take_in(&mut self, file: ClipsFile) {
        self.taken_in(file);
    }

    fn of(held: &crate::own_files::OwnFile) -> Option<&std::sync::Arc<ClipsFile>> {
        match held {
            crate::own_files::OwnFile::Frames(file) => Some(file),
            _ => None,
        }
    }
}

#[cfg(test)]
#[path = "frames_editor_tests.rs"]
mod tests;
