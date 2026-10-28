//! A sprite's clips file (`.solframes`, Phase 9, D3): which sheet (an
//! Aseprite file) it plays, what Solis adds to the sheet's tags, and
//! clips made in Solis from any of the sheet's frames. A settings file in
//! the same Lust style as areas, one line per clip, so Git merges it:
//!
//! ```text
//! -- A sprite's clips. Safe to edit by hand.
//! frames version 1;
//!
//! sheet = "1BWHNR3RV8PYY";
//! first = "walk";
//! tag1 = ["walk", 1.0, [[3, "footstep"]]];
//! clip1 = ["hoe", 1.0, "forward", 1, [[5, 100, ""], [6, 120, "swing"]]];
//! ```
//!
//! - `first`: the clip a sprite plays when it's given no other.
//! - `tagN`: one of the sheet's Aseprite tags, by name: its speed, and
//!   events Solis adds, each `[frame, "name"]` (frames counted from 1, as
//!   Aseprite shows them). Its frames, times, direction and repeats stay
//!   Aseprite's.
//! - `clipN`: a clip made here: name, speed, direction (`"forward"`,
//!   `"reverse"`, `"ping-pong"` or `"ping-pong reversed"`), repeats (0 for
//!   over and over), and its frames, each `[frame, milliseconds, "event"]`
//!   (`""` for none).
//!
//! Read as a hostile file: within limits; a value out of range is brought
//! into range with a note; a line it can't use is skipped with a note;
//! only a file that can't be read at all (or one from a newer version) is
//! refused, with its line.

use crate::clip::{ClipFrame, Direction};
use area::settings;
use reflect::{AssetId, Value};
use std::fmt;

/// The layout version this build writes and reads.
pub const VERSION: u32 = 1;
/// The word the file starts with.
const KIND: &str = "frames";
/// The file's extension.
pub const EXTENSION: &str = "solframes";
/// The biggest clips file read.
pub const MAX_BYTES: usize = 512 << 10;
/// The most tag lines and clips, together.
pub const MAX_CLIPS: usize = 256;
/// The most frames in a clip made here.
pub const MAX_FRAMES: usize = 1024;
/// The most events Solis adds to one tag.
pub const MAX_TAG_EVENTS: usize = 256;
/// The longest a frame shows, in milliseconds.
pub const MAX_MS: u32 = 60_000;
/// The slowest and fastest a clip plays.
pub const SPEED_RANGE: (f32, f32) = (0.0, 16.0);
/// The most passes a clip made here plays (0 is over and over).
pub const MAX_REPEATS: u32 = 10_000;
/// The longest clip or event name, in characters.
pub const MAX_NAME: usize = 64;
/// The highest frame number (counting from 1).
pub const MAX_FRAME_NUMBER: u32 = 65_536;

/// This file's version and upgraders (Phase 8, C1): each step turns
/// fields read at one version into the next, before they're applied.
/// Every version ever released keeps an example file in `tests/fixtures/`.
pub static FORMAT: reflect::upgrade::Format<settings::Settings> = reflect::upgrade::Format {
    name: "frames",
    current: VERSION,
    version_of: |text| reflect::upgrade::header_version(text, &[KIND]),
    steps: &[],
};

/// What Solis adds to one of the sheet's Aseprite tags.
#[derive(Debug, Clone, PartialEq)]
pub struct TagChange {
    /// The tag's name, as in Aseprite.
    pub name: String,
    /// How fast it plays: 1 as drawn.
    pub speed: f32,
    /// Events added, each on a sheet frame (from 0).
    pub events: Vec<(u32, String)>,
}

/// A clip made in Solis.
#[derive(Debug, Clone, PartialEq)]
pub struct OwnClip {
    /// Its name.
    pub name: String,
    /// How fast it plays: 1 as set.
    pub speed: f32,
    /// Which way it plays.
    pub direction: Direction,
    /// How many passes: 0 for over and over.
    pub repeats: u32,
    /// Its frames (sheet frames from 0), times and events.
    pub frames: Vec<ClipFrame>,
}

/// A sprite's clips file.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FramesFile {
    /// The sheet (an Aseprite file) whose frames it plays.
    pub sheet: Option<AssetId>,
    /// The clip played when a sprite is given no other ("" for none).
    pub first: String,
    /// What's added to the sheet's tags, by name.
    pub tags: Vec<TagChange>,
    /// Clips made here.
    pub clips: Vec<OwnClip>,
}

/// Why a clips file couldn't be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FramesError {
    /// The line it went wrong on (from 1), or 0 for the whole file.
    pub line: usize,
    /// What went wrong, in plain words.
    pub message: String,
}

impl fmt::Display for FramesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line == 0 {
            f.write_str(&self.message)
        } else {
            write!(f, "line {}: {}", self.line, self.message)
        }
    }
}

impl std::error::Error for FramesError {}

/// A clips file as read, with notes about what was put right.
#[derive(Debug, Clone, PartialEq)]
pub struct ReadFrames {
    /// The file.
    pub file: FramesFile,
    /// What was skipped or put right, in plain words with lines.
    pub notes: Vec<String>,
}

/// Whether `path` names a clips file (by its extension).
pub fn is_frames_file(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case(EXTENSION))
}

/// The name of a direction as the file writes it.
pub fn direction_word(direction: Direction) -> &'static str {
    match direction {
        Direction::Forward => "forward",
        Direction::Reverse => "reverse",
        Direction::PingPong => "ping-pong",
        Direction::PingPongReverse => "ping-pong reversed",
    }
}

fn direction_of(word: &str) -> Option<Direction> {
    [
        Direction::Forward,
        Direction::Reverse,
        Direction::PingPong,
        Direction::PingPongReverse,
    ]
    .into_iter()
    .find(|d| direction_word(*d) == word.trim())
}

/// Reads a clips file.
pub fn read(text: &str) -> Result<ReadFrames, FramesError> {
    if text.len() > MAX_BYTES {
        return Err(FramesError {
            line: 0,
            message: format!(
                "the file is {} KB; clips files up to {} KB are read",
                text.len() >> 10,
                MAX_BYTES >> 10
            ),
        });
    }
    let mut settings = settings::read_settings(text, KIND, VERSION).map_err(|e| FramesError {
        line: e.line,
        message: e.message,
    })?;
    let version = settings.version;
    FORMAT
        .upgrade(version, &mut settings)
        .map_err(|e| FramesError {
            line: 1,
            message: e.to_string(),
        })?;
    let mut notes = Vec::new();
    let mut file = FramesFile::default();
    for field in &settings.fields {
        let line = field.line;
        let key = field.key.as_str();
        let skip = |notes: &mut Vec<String>, why: &str| {
            notes.push(format!("line {line}: {why}, so it's skipped"));
        };
        match key {
            "sheet" => match &field.value {
                Value::Text(t) if t.trim().is_empty() => {}
                Value::Text(t) => match t.trim().parse::<AssetId>() {
                    Ok(id) => file.sheet = Some(id),
                    Err(_) => skip(&mut notes, "the sheet isn't an asset's ID"),
                },
                _ => skip(&mut notes, "the sheet should be an asset's ID in quotes"),
            },
            "first" => match &field.value {
                Value::Text(t) => file.first = name(t),
                _ => skip(&mut notes, "first should be a clip's name in quotes"),
            },
            _ if key.starts_with("tag") || key.starts_with("clip") => {
                if file.tags.len() + file.clips.len() == MAX_CLIPS {
                    notes.push(format!(
                        "line {line}: more than {MAX_CLIPS} clips; the rest are skipped"
                    ));
                    break;
                }
                let result = if key.starts_with("tag") {
                    read_tag(&field.value, line, &mut notes).map(|t| {
                        if file.tags.iter().any(|o| o.name == t.name) {
                            Err(format!("a second line for the tag \"{}\"", t.name))
                        } else {
                            file.tags.push(t);
                            Ok(())
                        }
                    })
                } else {
                    read_clip(&field.value, line, &mut notes).map(|c| {
                        if file.clips.iter().any(|o| o.name == c.name) {
                            Err(format!("a second clip called \"{}\"", c.name))
                        } else {
                            file.clips.push(c);
                            Ok(())
                        }
                    })
                };
                match result {
                    Ok(Ok(())) => {}
                    Ok(Err(why)) | Err(why) => skip(&mut notes, &why),
                }
            }
            _ => skip(&mut notes, &format!("\"{key}\" isn't a clips setting")),
        }
    }
    Ok(ReadFrames { file, notes })
}

/// A name from the file: trimmed, at most [`MAX_NAME`] characters.
fn name(text: &str) -> String {
    text.trim().chars().take(MAX_NAME).collect()
}

/// A speed, brought into range.
fn speed(value: &Value, line: usize, notes: &mut Vec<String>) -> Result<f32, String> {
    let v = value
        .as_f32()
        .ok_or("its speed should be a number".to_owned())?;
    let (lo, hi) = SPEED_RANGE;
    let fitted = v.clamp(lo, hi);
    if fitted != v {
        notes.push(format!(
            "line {line}: speed {v} is outside {lo} to {hi}, so it's {fitted}"
        ));
    }
    Ok(fitted)
}

/// A frame number counting from 1, as a sheet frame from 0.
fn frame_number(value: &Value) -> Result<u32, String> {
    match value {
        Value::Int(n) if (1..=i64::from(MAX_FRAME_NUMBER)).contains(n) => Ok(*n as u32 - 1),
        _ => Err(format!(
            "a frame should be a whole number from 1 to {MAX_FRAME_NUMBER}"
        )),
    }
}

/// An event's name ("" for none).
fn event(value: &Value) -> Result<String, String> {
    match value {
        Value::Text(t) => Ok(name(t)),
        _ => Err("an event should be a name in quotes".to_owned()),
    }
}

/// A `tagN` line: [name, speed, events].
fn read_tag(value: &Value, line: usize, notes: &mut Vec<String>) -> Result<TagChange, String> {
    let shape = "a tag line should be [name, speed, [[frame, \"event\"], …]]";
    let Value::List(items) = value else {
        return Err(shape.to_owned());
    };
    let [Value::Text(n), s, Value::List(evs)] = items.as_slice() else {
        return Err(shape.to_owned());
    };
    let n = name(n);
    if n.is_empty() {
        return Err("a tag line needs the tag's name".to_owned());
    }
    let speed = speed(s, line, notes)?;
    let mut events = Vec::new();
    for e in evs {
        if events.len() == MAX_TAG_EVENTS {
            notes.push(format!(
                "line {line}: more than {MAX_TAG_EVENTS} events on {n}; the rest are skipped"
            ));
            break;
        }
        let Value::List(pair) = e else {
            return Err(shape.to_owned());
        };
        let [f, ev] = pair.as_slice() else {
            return Err(shape.to_owned());
        };
        let ev = event(ev)?;
        if !ev.is_empty() {
            events.push((frame_number(f)?, ev));
        }
    }
    Ok(TagChange {
        name: n,
        speed,
        events,
    })
}

/// A `clipN` line: [name, speed, direction, repeats, frames].
fn read_clip(value: &Value, line: usize, notes: &mut Vec<String>) -> Result<OwnClip, String> {
    let shape = "a clip line should be [name, speed, direction, repeats, [[frame, ms, \"event\"], …]]";
    let Value::List(items) = value else {
        return Err(shape.to_owned());
    };
    let [Value::Text(n), s, Value::Text(d), r, Value::List(frames)] = items.as_slice() else {
        return Err(shape.to_owned());
    };
    let n = name(n);
    if n.is_empty() {
        return Err("a clip needs a name".to_owned());
    }
    let speed = speed(s, line, notes)?;
    let direction = direction_of(d).ok_or_else(|| {
        format!(
            "\"{d}\" isn't a direction (forward, reverse, ping-pong or ping-pong reversed)"
        )
    })?;
    let Value::Int(r) = r else {
        return Err("repeats should be a whole number (0 for over and over)".to_owned());
    };
    let repeats = (*r).clamp(0, i64::from(MAX_REPEATS)) as u32;
    if i64::from(repeats) != *r {
        notes.push(format!(
            "line {line}: repeats {r} is outside 0 to {MAX_REPEATS}, so it's {repeats}"
        ));
    }
    let mut out = Vec::new();
    for f in frames {
        if out.len() == MAX_FRAMES {
            notes.push(format!(
                "line {line}: {n} has more than {MAX_FRAMES} frames; the rest are skipped"
            ));
            break;
        }
        let Value::List(parts) = f else {
            return Err(shape.to_owned());
        };
        let [number, ms, ev] = parts.as_slice() else {
            return Err(shape.to_owned());
        };
        let Value::Int(ms) = ms else {
            return Err("a frame's time should be whole milliseconds".to_owned());
        };
        let fitted = (*ms).clamp(1, i64::from(MAX_MS)) as u32;
        if i64::from(fitted) != *ms {
            notes.push(format!(
                "line {line}: a frame of {ms} ms is outside 1 to {MAX_MS}, so it's {fitted} ms"
            ));
        }
        let ev = event(ev)?;
        out.push(ClipFrame {
            frame: frame_number(number)?,
            ms: fitted,
            events: if ev.is_empty() { Vec::new() } else { vec![ev] },
        });
    }
    Ok(OwnClip {
        name: n,
        speed,
        direction,
        repeats,
        frames: out,
    })
}

/// Writes a clips file that [`read`] reads back. A clip made here keeps
/// one event a frame (the editor gives each frame one).
pub fn write(file: &FramesFile) -> String {
    let mut fields: Vec<(String, Value)> = Vec::new();
    fields.push((
        "sheet".into(),
        Value::Text(file.sheet.map_or_else(String::new, |s| s.to_string())),
    ));
    fields.push(("first".into(), Value::Text(file.first.clone())));
    for (i, t) in file.tags.iter().enumerate() {
        let events = t
            .events
            .iter()
            .map(|(f, e)| Value::List(vec![Value::Int(i64::from(*f) + 1), Value::Text(e.clone())]))
            .collect();
        fields.push((
            format!("tag{}", i + 1),
            Value::List(vec![
                Value::Text(t.name.clone()),
                Value::Float(f64::from(t.speed)),
                Value::List(events),
            ]),
        ));
    }
    for (i, c) in file.clips.iter().enumerate() {
        let frames = c
            .frames
            .iter()
            .map(|f| {
                Value::List(vec![
                    Value::Int(i64::from(f.frame) + 1),
                    Value::Int(i64::from(f.ms)),
                    Value::Text(f.events.first().cloned().unwrap_or_default()),
                ])
            })
            .collect();
        fields.push((
            format!("clip{}", i + 1),
            Value::List(vec![
                Value::Text(c.name.clone()),
                Value::Float(f64::from(c.speed)),
                Value::Text(direction_word(c.direction).to_owned()),
                Value::Int(i64::from(c.repeats)),
                Value::List(frames),
            ]),
        ));
    }
    let borrowed: Vec<(&str, Value)> = fields.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
    settings::write_settings(
        KIND,
        VERSION,
        "A sprite's clips. Safe to edit by hand.\n\
         tagN: one of the sheet's Aseprite tags: [name, speed, [[frame, \"event\"], ...]].\n\
         clipN: a clip made here: [name, speed, direction, repeats, [[frame, ms, \"event\"], ...]].\n\
         Frames count from 1, as Aseprite shows them.",
        &borrowed,
    )
}

#[cfg(test)]
#[path = "frames_file_tests.rs"]
mod tests;
