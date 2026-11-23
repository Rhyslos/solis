//! A character's clips file as Lust (Fixes 3, F3.1; [the plan](../../../docs/plans/fixes-3.md)):
//! how its picture is cut (a grid, and cells moved or drawn by hand), its
//! looks (other pictures cut the same way), and its clips, each with one,
//! four or eight directions. A Lust data file beside the picture
//! (decision 0081), `knight.clips.lst`, replacing `.solframes`:
//!
//! ```text
//! -- Clips of knight.png. Written by Solis; safe to edit by hand.
//! fetch "solis/data/clips":[Sheet, Cell, Look, Clip, Way, Tag, Facing, Playback];
//!
//! lock int version = 2;
//! lock Sheet sheet = new Sheet { cell = [32, 32], between = [2, 2], edge = [1, 1], first = "walk" };
//! lock Cell array cells = [
//!     new Cell { number = 23, box = [136, 69, 32, 32] },
//! ];
//! lock Look array looks = [
//!     new Look { name = "Normal", picture = "1BWHNR3RV8PYY" },
//! ];
//! lock Clip array clips = [
//!     new Clip { name = "walk", speed = 1.0, playback = Playback.Forward, plays = 0, directions = 8, sameFrames = true, mirror = true, times = [120, 120], events = ["footstep", ""] },
//! ];
//! lock Way array ways = [
//!     new Way { clip = "walk", facing = Facing.South, cells = [3, 4], times = [], events = [] },
//! ];
//! lock Tag array tags = [];
//! end
//! ```
//!
//! - **Cells** are numbered from 0, left to right, top to bottom, by the
//!   grid; a `Cell` moves one (or adds one past the grid's) by hand.
//! - **Looks:** the first is the main one; every look is cut the same way.
//! - **Ways:** a clip's row for one facing. With `sameFrames`, the times and
//!   events are the clip's; without, each way has its own. A clip with one
//!   direction has one way, facing `Any`. With `mirror`, the west side is the
//!   east side flipped, so it has no ways of its own.
//! - **Tags:** what Solis adds to an Aseprite sheet's own tags (speed,
//!   events), as `.solframes` kept it.
//!
//! Read as a hostile file: within limits; anything out of range is left out
//! or put right, with a note. Never panics.

use crate::clip::Direction;
use level::lust_data::{self as data, DataError, Fixed};
use reflect::AssetId;

/// The word before `.lst` in a clips file's name (`knight.clips.lst`).
pub const KIND: &str = "clips";

/// The version Solis writes: 2, after `.solframes`' 1.
pub const VERSION: u32 = 2;

/// The biggest clips file read.
pub const MAX_BYTES: usize = 4 << 20;
/// The most clips (with tags), together.
pub const MAX_CLIPS: usize = 256;
/// The most frames in one row.
pub const MAX_FRAMES: usize = 1024;
/// The most cells moved or drawn by hand.
pub const MAX_CELLS: usize = 65_536;
/// The highest cell number.
pub const MAX_CELL_NUMBER: u32 = 65_535;
/// The most looks.
pub const MAX_LOOKS: usize = 64;
/// The longest a frame shows, in milliseconds.
pub const MAX_MS: u32 = 60_000;
/// The fastest a clip plays.
pub const MAX_SPEED: f32 = 16.0;
/// The most passes a clip plays (0 is over and over).
pub const MAX_PLAYS: u32 = 10_000;
/// The longest name, of a clip, look or event.
pub const MAX_NAME: usize = 64;
/// The biggest picture, either way, in pixels.
pub const MAX_SIDE: u32 = 16_384;

/// Clips files' version and upgraders (decision 0053). Every version keeps
/// an example in `tests/fixtures/clips/`.
pub static FORMAT: reflect::upgrade::Format<ClipsFile> = reflect::upgrade::Format {
    name: "clips",
    current: VERSION,
    version_of: data::version_of,
    steps: &[],
};

/// What a clips file holds, in Lust: written into every project's
/// `solis/data/clips.lst`, and handed to Lust when one is read.
pub const CLASSES: &str = "-- What a clips file holds (Solis writes this; clips files fetch it).\n\
state Facing = [Any, North, NorthEast, East, SouthEast, South, SouthWest, West, NorthWest];\n\
state Playback = [Forward, Reverse, PingPong, PingPongReverse];\n\n\
class Sheet {\n    has {\n        int array cell = [];\n        int array between = [];\n        int array edge = [];\n        string first = \"\";\n    }\n}\n\n\
class Cell {\n    has {\n        int number = 0;\n        int array box = [];\n    }\n}\n\n\
class Look {\n    has {\n        string name = \"\";\n        string picture = \"\";\n    }\n}\n\n\
class Clip {\n    has {\n        string name = \"\";\n        float speed = 1.0;\n        Playback playback = Playback.Forward;\n        int plays = 0;\n        int directions = 1;\n        bool sameFrames = true;\n        bool mirror = true;\n        int array times = [];\n        string array events = [];\n    }\n}\n\n\
class Way {\n    has {\n        string clip = \"\";\n        Facing facing = Facing.Any;\n        int array cells = [];\n        int array times = [];\n        string array events = [];\n    }\n}\n\n\
class Tag {\n    has {\n        string name = \"\";\n        float speed = 1.0;\n        int array frames = [];\n        string array events = [];\n    }\n}\n\n\
pass Facing, Playback, Sheet, Cell, Look, Clip, Way, Tag;\nend\n";

/// Which way a clip's row faces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Facing {
    /// A clip with one direction: whatever the facing.
    Any,
    /// Up the screen.
    North,
    /// Up and right.
    NorthEast,
    /// Right.
    East,
    /// Down and right.
    SouthEast,
    /// Down the screen.
    South,
    /// Down and left.
    SouthWest,
    /// Left.
    West,
    /// Up and left.
    NorthWest,
}

impl Facing {
    /// Every facing, as the file's state lists them.
    pub const ALL: [Facing; 9] = [
        Facing::Any,
        Facing::North,
        Facing::NorthEast,
        Facing::East,
        Facing::SouthEast,
        Facing::South,
        Facing::SouthWest,
        Facing::West,
        Facing::NorthWest,
    ];

    /// Its option's name in the file (`NorthEast`).
    pub fn name(self) -> &'static str {
        [
            "Any",
            "North",
            "NorthEast",
            "East",
            "SouthEast",
            "South",
            "SouthWest",
            "West",
            "NorthWest",
        ][self as usize]
    }

    fn named(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|f| f.name() == name)
    }

    /// Whether it's on the west side, which a mirrored clip draws from
    /// the east's.
    pub fn is_west(self) -> bool {
        matches!(self, Facing::SouthWest | Facing::West | Facing::NorthWest)
    }
}

/// How a picture is cut into numbered cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Grid {
    /// A cell's width and height, in pixels (each at least 1).
    pub cell: [u32; 2],
    /// The gap between cells, across and down.
    pub between: [u32; 2],
    /// The margin round the picture's edge, across and down.
    pub edge: [u32; 2],
}

impl Default for Grid {
    fn default() -> Self {
        Self {
            cell: [16, 16],
            between: [0, 0],
            edge: [0, 0],
        }
    }
}

/// A cell moved or drawn by hand (Free crop): its number and box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    /// Its number: a grid cell's, or one past the grid's.
    pub number: u32,
    /// Its box in the picture: left, top, width, height.
    pub rect: [u32; 4],
}

/// One look: another picture, cut the same way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Look {
    /// Its name (`Mana`).
    pub name: String,
    /// Its picture.
    pub picture: AssetId,
}

/// One frame's time and event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Timing {
    /// How long it shows, in milliseconds.
    pub ms: u32,
    /// The event it sends (empty for none).
    pub event: String,
}

/// A clip's row for one facing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Way {
    /// Which way it faces.
    pub facing: Facing,
    /// Its cells, in order.
    pub cells: Vec<u32>,
    /// Its frames' times and events, when the clip's directions don't
    /// share them (otherwise empty).
    pub timings: Vec<Timing>,
}

/// One clip made in Solis.
#[derive(Debug, Clone, PartialEq)]
pub struct Clip {
    /// Its name (`walk`).
    pub name: String,
    /// How fast it plays: 1 as drawn.
    pub speed: f32,
    /// Forward, reverse, or there and back.
    pub playback: Direction,
    /// How many passes (0 is over and over).
    pub plays: u32,
    /// One, four or eight.
    pub directions: u8,
    /// Whether every direction shares the frames' times and events.
    pub same_frames: bool,
    /// Whether the west side is the east side flipped.
    pub mirror: bool,
    /// The frames' times and events, when shared.
    pub timings: Vec<Timing>,
    /// Its rows, one a facing.
    pub ways: Vec<Way>,
}

/// What Solis adds to one of an Aseprite sheet's tags.
#[derive(Debug, Clone, PartialEq)]
pub struct TagChange {
    /// The tag's name, as in Aseprite.
    pub name: String,
    /// How fast it plays: 1 as drawn.
    pub speed: f32,
    /// Events added, each on a sheet frame (from 0).
    pub events: Vec<(u32, String)>,
}

/// A character's clips.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ClipsFile {
    /// How the pictures are cut.
    pub grid: Grid,
    /// The clip a sprite plays when given no other.
    pub first: String,
    /// Cells moved or drawn by hand.
    pub cells: Vec<Cell>,
    /// The looks; the first is the main one.
    pub looks: Vec<Look>,
    /// The clips made here.
    pub clips: Vec<Clip>,
    /// What's added to an Aseprite sheet's tags.
    pub tags: Vec<TagChange>,
}

/// The clips file's path for a picture's (`sprites/knight.png` gives
/// `sprites/knight.clips.lst`).
pub fn file_for(picture: &str) -> String {
    let stem = picture.rsplit_once('.').map_or(picture, |(s, _)| s);
    format!("{stem}.{KIND}.lst")
}

/// Whether `name` can name a clip, look or event: a letter, then at most
/// [`MAX_NAME`] letters, digits, `_` and `-` in all.
pub fn good_name(name: &str) -> bool {
    let mut chars = name.chars();
    name.len() <= MAX_NAME
        && chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn playback_name(d: Direction) -> &'static str {
    match d {
        Direction::Forward => "Forward",
        Direction::Reverse => "Reverse",
        Direction::PingPong => "PingPong",
        Direction::PingPongReverse => "PingPongReverse",
    }
}

fn playback_named(name: &str) -> Option<Direction> {
    Some(match name {
        "Forward" => Direction::Forward,
        "Reverse" => Direction::Reverse,
        "PingPong" => Direction::PingPong,
        "PingPongReverse" => Direction::PingPongReverse,
        _ => return None,
    })
}

fn pair(v: [u32; 2]) -> String {
    data::ints(v.iter().map(|&n| i64::from(n)))
}

/// Writes `file` as its Lust data file, `picture` (the main look's file
/// name) in its first comment.
pub fn write(file: &ClipsFile, picture: &str) -> String {
    let mut out = data::header(
        &format!("Clips of {picture}. Written by Solis; safe to edit by hand."),
        KIND,
        &[
            "Sheet", "Cell", "Look", "Clip", "Way", "Tag", "Facing", "Playback",
        ],
        VERSION,
    );
    out.push_str(&format!(
        "lock Sheet sheet = {};\n",
        data::thing(
            "Sheet",
            &[
                ("cell", pair(file.grid.cell)),
                ("between", pair(file.grid.between)),
                ("edge", pair(file.grid.edge)),
                ("first", data::text(&file.first)),
            ],
        )
    ));
    let cells: Vec<String> = file
        .cells
        .iter()
        .map(|c| {
            data::thing(
                "Cell",
                &[
                    ("number", c.number.to_string()),
                    ("box", data::ints(c.rect.iter().map(|&v| i64::from(v)))),
                ],
            )
        })
        .collect();
    data::list(&mut out, "Cell", "cells", &cells);
    let looks: Vec<String> = file
        .looks
        .iter()
        .map(|l| {
            data::thing(
                "Look",
                &[
                    ("name", data::text(&l.name)),
                    ("picture", data::text(&l.picture.to_string())),
                ],
            )
        })
        .collect();
    data::list(&mut out, "Look", "looks", &looks);
    let times = |t: &[Timing]| data::ints(t.iter().map(|t| i64::from(t.ms)));
    let events = |t: &[Timing]| data::texts(t.iter().map(|t| t.event.as_str()));
    let clips: Vec<String> = file
        .clips
        .iter()
        .map(|c| {
            data::thing(
                "Clip",
                &[
                    ("name", data::text(&c.name)),
                    ("speed", data::single(c.speed)),
                    (
                        "playback",
                        format!("Playback.{}", playback_name(c.playback)),
                    ),
                    ("plays", c.plays.to_string()),
                    ("directions", c.directions.to_string()),
                    ("sameFrames", c.same_frames.to_string()),
                    ("mirror", c.mirror.to_string()),
                    ("times", times(&c.timings)),
                    ("events", events(&c.timings)),
                ],
            )
        })
        .collect();
    data::list(&mut out, "Clip", "clips", &clips);
    let ways: Vec<String> = file
        .clips
        .iter()
        .flat_map(|c| {
            c.ways.iter().map(move |w| {
                data::thing(
                    "Way",
                    &[
                        ("clip", data::text(&c.name)),
                        ("facing", format!("Facing.{}", w.facing.name())),
                        ("cells", data::ints(w.cells.iter().map(|&n| i64::from(n)))),
                        ("times", times(&w.timings)),
                        ("events", events(&w.timings)),
                    ],
                )
            })
        })
        .collect();
    data::list(&mut out, "Way", "ways", &ways);
    let tags: Vec<String> = file
        .tags
        .iter()
        .map(|t| {
            data::thing(
                "Tag",
                &[
                    ("name", data::text(&t.name)),
                    ("speed", data::single(t.speed)),
                    (
                        "frames",
                        data::ints(t.events.iter().map(|(f, _)| i64::from(*f))),
                    ),
                    (
                        "events",
                        data::texts(t.events.iter().map(|(_, e)| e.as_str())),
                    ),
                ],
            )
        })
        .collect();
    data::list(&mut out, "Tag", "tags", &tags);
    out.push_str("end\n");
    out
}

/// A clips file read back, and what was put right or left out.
#[derive(Debug, Clone, PartialEq)]
pub struct ReadClips {
    /// The clips.
    pub file: ClipsFile,
    /// What was left out or put right, in plain words.
    pub notes: Vec<String>,
}

/// A speed from 0 to [`MAX_SPEED`], else 1 with a note.
fn speed_of(thing: &Fixed, what: &str, notes: &mut Vec<String>) -> f32 {
    match data::field_float(thing, "speed") {
        Some(s) if s.is_finite() && (0.0..=f64::from(MAX_SPEED)).contains(&s) => s as f32,
        _ => {
            notes.push(format!("{what}'s speed wasn't from 0 to 16, so it's 1."));
            1.0
        }
    }
}

/// A row's times and events, paired; `None` (with a note) if they don't
/// pair up or a time is out of range.
fn timings_of(thing: &Fixed, what: &str, notes: &mut Vec<String>) -> Option<Vec<Timing>> {
    let times = data::field_ints(thing, "times").unwrap_or_default();
    let events = data::field_texts(thing, "events");
    let fine = times.len() <= MAX_FRAMES
        && (events.is_empty() || events.len() == times.len())
        && times.iter().all(|t| (1..=i64::from(MAX_MS)).contains(t))
        && events.iter().all(|e| e.is_empty() || good_name(e));
    if !fine {
        notes.push(format!(
            "{what} was left out: its times must be from 1 to 60000 ms, with an event name (or \
             \"\") for each."
        ));
        return None;
    }
    Some(
        times
            .iter()
            .enumerate()
            .map(|(i, &ms)| Timing {
                ms: ms as u32,
                event: events.get(i).copied().unwrap_or("").to_string(),
            })
            .collect(),
    )
}

/// Reads a clips file, as a hostile file: within [`MAX_BYTES`] and the
/// other limits, anything out of range left out or put right with a note.
/// Never panics.
pub fn read(text: &str) -> Result<ReadClips, DataError> {
    let values = data::read(text, KIND, CLASSES, MAX_BYTES)?;
    if let Some(v) = data::version_of(text).filter(|&v| v > VERSION) {
        return Err(data::error(
            0,
            format!("it's version {v}, made by a newer Solis"),
        ));
    }
    let mut notes = Vec::new();
    let mut file = ClipsFile::default();
    let sheet = data::get(&values, "sheet")?;
    let side = |v: i64, low: i64| (low..=i64::from(MAX_SIDE)).contains(&v);
    let two = |name: &str, low: i64| match data::field_ints(sheet, name).as_deref() {
        Some(&[a, b]) if side(a, low) && side(b, low) => Some([a as u32, b as u32]),
        _ => None,
    };
    file.grid.cell = two("cell", 1).unwrap_or_else(|| {
        notes.push("Its cell size wasn't two sizes from 1 to 16384, so it's 16 × 16.".into());
        [16, 16]
    });
    file.grid.between = two("between", 0).unwrap_or([0, 0]);
    file.grid.edge = two("edge", 0).unwrap_or([0, 0]);
    file.first = data::field_text(sheet, "first").to_string();

    for cell in data::things(&values, "cells").iter().take(MAX_CELLS) {
        let number = data::field_int(cell, "number")
            .filter(|n| (0..=i64::from(MAX_CELL_NUMBER)).contains(n));
        let rect = match data::field_ints(cell, "box").as_deref() {
            Some(&[x, y, w, h]) if side(x, 0) && side(y, 0) && side(w, 1) && side(h, 1) => {
                Some([x as u32, y as u32, w as u32, h as u32])
            }
            _ => None,
        };
        match (number, rect) {
            (Some(n), Some(rect)) if !file.cells.iter().any(|c| c.number as i64 == n) => {
                file.cells.push(Cell {
                    number: n as u32,
                    rect,
                });
            }
            (Some(n), Some(_)) => notes.push(format!("A second cell {n} was left out.")),
            _ => notes.push(
                "A cell was left out: it needs a number from 0 to 65535 and a box inside a \
                 picture of at most 16384 pixels."
                    .into(),
            ),
        }
    }

    for look in data::things(&values, "looks") {
        if file.looks.len() >= MAX_LOOKS {
            notes.push(format!("Only the first {MAX_LOOKS} looks are kept."));
            break;
        }
        let name = data::field_text(look, "name");
        let picture = data::field_text(look, "picture")
            .parse::<reflect::Id>()
            .ok();
        match picture {
            Some(id) if good_name(name) && !file.looks.iter().any(|l| l.name == name) => {
                file.looks.push(Look {
                    name: name.to_string(),
                    picture: AssetId(id),
                });
            }
            _ => notes.push(format!(
                "The look \"{}\" was left out: it needs a name of its own and a picture's ID.",
                name.chars().take(MAX_NAME).collect::<String>()
            )),
        }
    }

    for clip in data::things(&values, "clips") {
        if file.clips.len() >= MAX_CLIPS {
            notes.push(format!("Only the first {MAX_CLIPS} clips are kept."));
            break;
        }
        let name = data::field_text(clip, "name");
        if !good_name(name) || file.clips.iter().any(|c| c.name == name) {
            notes.push(format!(
                "The clip \"{}\" was left out: a clip needs a name of its own, starting with a \
                 letter.",
                name.chars().take(MAX_NAME).collect::<String>()
            ));
            continue;
        }
        let what = format!("The clip {name}");
        let directions = match data::field_int(clip, "directions") {
            Some(d @ (1 | 4 | 8)) => d as u8,
            _ => {
                notes.push(format!(
                    "{what}'s directions weren't 1, 4 or 8, so it has one."
                ));
                1
            }
        };
        let Some(timings) = timings_of(clip, &what, &mut notes) else {
            continue;
        };
        file.clips.push(Clip {
            name: name.to_string(),
            speed: speed_of(clip, &what, &mut notes),
            playback: data::field_option(clip, "playback")
                .and_then(playback_named)
                .unwrap_or(Direction::Forward),
            plays: data::field_int(clip, "plays")
                .filter(|p| (0..=i64::from(MAX_PLAYS)).contains(p))
                .unwrap_or(0) as u32,
            directions,
            same_frames: data::field_bool(clip, "sameFrames"),
            mirror: data::field_bool(clip, "mirror"),
            timings,
            ways: Vec::new(),
        });
    }

    for way in data::things(&values, "ways") {
        let clip_name = data::field_text(way, "clip");
        let Some(clip) = file.clips.iter_mut().find(|c| c.name == clip_name) else {
            notes.push(format!(
                "A row for \"{}\" was left out: there's no such clip.",
                clip_name.chars().take(MAX_NAME).collect::<String>()
            ));
            continue;
        };
        let what = format!("The clip {}'s row", clip.name);
        let Some(facing) = data::field_option(way, "facing").and_then(Facing::named) else {
            notes.push(format!("{what} was left out: it faces no known way."));
            continue;
        };
        let fits = match clip.directions {
            1 => facing == Facing::Any,
            4 => matches!(
                facing,
                Facing::North | Facing::East | Facing::South | Facing::West
            ),
            _ => facing != Facing::Any,
        };
        if !fits
            || (clip.mirror && facing.is_west())
            || clip.ways.iter().any(|w| w.facing == facing)
        {
            notes.push(format!(
                "{what} facing {} was left out: the clip has no such direction, has it twice, or \
                 mirrors its west side.",
                facing.name()
            ));
            continue;
        }
        let cells = match data::field_ints(way, "cells") {
            Some(cells)
                if cells.len() <= MAX_FRAMES
                    && cells
                        .iter()
                        .all(|c| (0..=i64::from(MAX_CELL_NUMBER)).contains(c)) =>
            {
                cells.into_iter().map(|c| c as u32).collect::<Vec<_>>()
            }
            _ => {
                notes.push(format!(
                    "{what} facing {} was left out: its cells aren't cell numbers.",
                    facing.name()
                ));
                continue;
            }
        };
        let timings = if clip.same_frames {
            Vec::new()
        } else {
            match timings_of(way, &what, &mut notes) {
                Some(t) if t.len() == cells.len() => t,
                Some(_) => {
                    notes.push(format!(
                        "{what} facing {} was left out: a time is needed for each cell.",
                        facing.name()
                    ));
                    continue;
                }
                None => continue,
            }
        };
        clip.ways.push(Way {
            facing,
            cells,
            timings,
        });
    }

    for tag in data::things(&values, "tags") {
        if file.clips.len() + file.tags.len() >= MAX_CLIPS {
            notes.push(format!("Only {MAX_CLIPS} clips and tags in all are kept."));
            break;
        }
        let name = data::field_text(tag, "name");
        if name.is_empty() || name.len() > MAX_NAME || file.tags.iter().any(|t| t.name == name) {
            notes.push("A tag's changes were left out: it needs the tag's name, once.".into());
            continue;
        }
        let frames = data::field_ints(tag, "frames").unwrap_or_default();
        let events = data::field_texts(tag, "events");
        let what = format!("The tag {name}");
        let speed = speed_of(tag, &what, &mut notes);
        if frames.len() != events.len()
            || frames.len() > MAX_FRAMES
            || frames
                .iter()
                .any(|f| !(0..=i64::from(MAX_CELL_NUMBER)).contains(f))
            || events.iter().any(|e| !good_name(e))
        {
            notes.push(format!(
                "{what}'s events were left out: each needs a frame and a name."
            ));
        }
        let events = if frames.len() == events.len() && events.iter().all(|e| good_name(e)) {
            frames
                .iter()
                .zip(events)
                .filter(|(f, _)| (0..=i64::from(MAX_CELL_NUMBER)).contains(*f))
                .take(MAX_FRAMES)
                .map(|(&f, e)| (f as u32, e.to_string()))
                .collect()
        } else {
            Vec::new()
        };
        file.tags.push(TagChange {
            name: name.to_string(),
            speed,
            events,
        });
    }
    Ok(ReadClips { file, notes })
}

#[cfg(test)]
#[path = "clips_file_tests.rs"]
mod tests;
