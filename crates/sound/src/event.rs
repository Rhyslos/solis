//! Sound events (Phase 7, S2; decision 0056, question 3): what a game
//! plays, such as "footstep". An event holds a few takes (sound files, by
//! asset ID) and how to vary them, so each play sounds a little
//! different: a random take, volume and pitch within a range.
//!
//! An event is a `.solsound` file in the project, one per event, in the
//! same Lust style as areas so Git merges it cleanly:
//!
//! ```text
//! -- A sound event. Safe to edit by hand: one line per setting.
//! sound version 1;
//!
//! takes = ["2EGJNFMTCSQ8G", "5A329QT768NJB"];
//! volume = -3.0;
//! volume_range = 2.0;
//! pitch = 0.0;
//! pitch_range = 1.0;
//! bus = "Effects";
//! placement = "In the world";
//! near = 2.0;
//! far = 30.0;
//! fade = "Realistic";
//! at_once = 8;
//! priority = "Normal";
//! loop = false;
//! heard_by = "Everyone";
//! heard_by_groups = [];
//! ```
//!
//! Read as a hostile file: within limits, a value out of range is brought
//! into range (with a note), a setting it doesn't know is skipped (with a
//! note), and anything malformed is refused with its line.
//!
//! **Sound never changes the game** (decision 0056): picking a take uses
//! a random stream of its own ([`pick`]), made from the game's seed, the
//! tick and the request's place, never the game's own random numbers.

use area::settings::{self, Field};
use reflect::{AssetId, Value};
use std::fmt;

/// The extension of sound event files.
pub const EXTENSION: &str = "solsound";
/// The layout version this build writes and reads.
pub const VERSION: u32 = 1;
/// The word the file starts with.
const KIND: &str = "sound";
/// This file's version and upgraders (Phase 8, C1): each step turns
/// fields read at one version into the next, before they're applied.
/// Every version ever released keeps an example file in `tests/fixtures/`.
pub static FORMAT: reflect::upgrade::Format<settings::Settings> = reflect::upgrade::Format {
    name: "sound event",
    current: VERSION,
    version_of: |text| reflect::upgrade::header_version(text, &[KIND]),
    steps: &[],
};
/// The biggest sound event file read (they're a few hundred bytes).
pub const MAX_BYTES: usize = 64 << 10;
/// The most takes in one event.
pub const MAX_TAKES: usize = 32;
/// The quietest and loudest an event may be set to, in decibels.
pub const VOLUME_RANGE_DB: (f32, f32) = (-60.0, 12.0);
/// The most a play's volume may vary either way, in decibels.
pub const MAX_VOLUME_VARIATION_DB: f32 = 24.0;
/// How far an event may be tuned down or up, in semitones (two octaves).
pub const PITCH_RANGE_ST: (f32, f32) = (-24.0, 24.0);
/// The most a play's pitch may vary either way, in semitones.
pub const MAX_PITCH_VARIATION_ST: f32 = 12.0;
/// The farthest a sound may carry, in metres.
pub const MAX_DISTANCE_M: f32 = 100_000.0;
/// The most plays of one event heard at once.
pub const MAX_AT_ONCE: u32 = 64;
/// The longest bus name kept.
const MAX_BUS_NAME: usize = 64;
/// The most groups an event is heard by.
pub const MAX_GROUPS: usize = 16;
/// The longest group name.
pub const MAX_GROUP_NAME: usize = 64;

/// Who hears an event (Phase 7, S8; decision 0058).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HeardBy {
    /// Every listener.
    #[default]
    Everyone,
    /// Only the listener on the entity that played it (or on what it's
    /// part of): the player's own footsteps, a click in their menu.
    Caller,
    /// Only listeners in one of the event's groups.
    Groups,
}

impl HeardBy {
    /// Every choice, in the order Properties lists them.
    pub const ALL: [Self; 3] = [Self::Everyone, Self::Caller, Self::Groups];

    /// As written in the file and shown in Properties.
    pub fn name(self) -> &'static str {
        match self {
            Self::Everyone => "Everyone",
            Self::Caller => "Only who caused it",
            Self::Groups => "Groups",
        }
    }
}

/// Whether an event is placed in the world.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Space {
    /// Heard the same everywhere: music, the interface.
    Flat,
    /// Placed in the world: quieter with distance, from its direction.
    #[default]
    Placed,
}

/// How a placed sound gets quieter between its near and far distance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Fade {
    /// As sound does in the open: quickly at first, then slowly.
    #[default]
    Realistic,
    /// Evenly, from full at the near distance to silent at the far.
    Even,
    /// Not at all, until past the far distance.
    None,
}

/// Which plays are kept when too many sounds play at once.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum Priority {
    /// Dropped first: debris, distant chatter.
    Low,
    /// Most sounds.
    #[default]
    Normal,
    /// Kept longest: the player's own gun, dialogue.
    High,
}

impl Space {
    /// As written in the file and shown in Properties.
    pub fn name(self) -> &'static str {
        match self {
            Self::Flat => "Everywhere",
            Self::Placed => "In the world",
        }
    }
}

impl Fade {
    /// As written in the file and shown in Properties.
    pub fn name(self) -> &'static str {
        match self {
            Self::Realistic => "Realistic",
            Self::Even => "Even",
            Self::None => "None",
        }
    }
}

impl Priority {
    /// As written in the file and shown in Properties.
    pub fn name(self) -> &'static str {
        match self {
            Self::Low => "Low",
            Self::Normal => "Normal",
            Self::High => "High",
        }
    }
}

/// A sound event's settings.
#[derive(Debug, Clone, PartialEq)]
pub struct SoundEvent {
    /// The sound files it picks from, by asset ID.
    pub takes: Vec<AssetId>,
    /// How loud, in decibels: 0 as recorded, -6 about half as loud.
    pub volume_db: f32,
    /// How much each play's volume may differ, either way, in decibels.
    pub volume_range_db: f32,
    /// How it's tuned, in semitones: 12 is an octave up.
    pub pitch_st: f32,
    /// How much each play's pitch may differ, either way, in semitones.
    pub pitch_range_st: f32,
    /// The bus it plays through (the Mixer's, from S4), by name.
    pub bus: String,
    /// Whether it's placed in the world.
    pub space: Space,
    /// Placed sounds are full volume up to here, in metres.
    pub near_m: f32,
    /// ...and silent past here, in metres.
    pub far_m: f32,
    /// How it gets quieter between the two.
    pub fade: Fade,
    /// How many plays of it may be heard at once.
    pub at_once: u32,
    /// Which plays are kept when too many sounds play.
    pub priority: Priority,
    /// Whether it plays again from the start when it ends.
    pub looping: bool,
    /// Who hears it.
    pub heard_by: HeardBy,
    /// The groups that hear it, when it's heard by Groups.
    pub groups: Vec<String>,
}

impl Default for SoundEvent {
    fn default() -> Self {
        Self {
            takes: Vec::new(),
            volume_db: 0.0,
            volume_range_db: 0.0,
            pitch_st: 0.0,
            pitch_range_st: 0.0,
            bus: "Effects".to_owned(),
            space: Space::Placed,
            near_m: 2.0,
            far_m: 30.0,
            fade: Fade::Realistic,
            at_once: 8,
            priority: Priority::Normal,
            looping: false,
            heard_by: HeardBy::Everyone,
            groups: Vec::new(),
        }
    }
}

/// Why a sound event file couldn't be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventError {
    /// The line it went wrong on (from 1), or 0 for the whole file.
    pub line: usize,
    /// What went wrong, in plain words.
    pub message: String,
}

impl fmt::Display for EventError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line == 0 {
            f.write_str(&self.message)
        } else {
            write!(f, "line {}: {}", self.line, self.message)
        }
    }
}

impl std::error::Error for EventError {}

/// A sound event as read, with notes about what was put right.
#[derive(Debug, Clone, PartialEq)]
pub struct ReadEvent {
    /// The event.
    pub event: SoundEvent,
    /// What was skipped or brought into range, in plain words with lines.
    pub notes: Vec<String>,
}

/// Reads a `.solsound` file.
pub fn read(text: &str) -> Result<ReadEvent, EventError> {
    if text.len() > MAX_BYTES {
        return Err(EventError {
            line: 0,
            message: format!(
                "the file is {} KB; sound events up to {} KB are read",
                text.len() >> 10,
                MAX_BYTES >> 10
            ),
        });
    }
    let mut file = settings::read_settings(text, KIND, VERSION).map_err(|e| EventError {
        line: e.line,
        message: e.message,
    })?;
    let version = file.version;
    FORMAT.upgrade(version, &mut file).map_err(|e| EventError {
        line: 1,
        message: e.to_string(),
    })?;
    let mut event = SoundEvent::default();
    let mut notes = Vec::new();
    for field in &file.fields {
        apply(&mut event, field, &mut notes)?;
    }
    if event.near_m > event.far_m {
        notes.push(format!(
            "near ({} m) was past far ({} m), so it's set to far",
            event.near_m, event.far_m
        ));
        event.near_m = event.far_m;
    }
    Ok(ReadEvent { event, notes })
}

/// Sets one field, or says why it can't be.
fn apply(event: &mut SoundEvent, field: &Field, notes: &mut Vec<String>) -> Result<(), EventError> {
    let line = field.line;
    let wrong = |what: &str| EventError {
        line,
        message: format!("{} should be {what}", field.key),
    };
    let number = |lo: f32, hi: f32, notes: &mut Vec<String>| -> Result<f32, EventError> {
        let v = field.value.as_f32().ok_or_else(|| wrong("a number"))?;
        let fitted = v.clamp(lo, hi);
        if fitted != v {
            notes.push(format!(
                "line {line}: {} = {v} is outside {lo} to {hi}, so it's {fitted}",
                field.key
            ));
        }
        Ok(fitted)
    };
    let word = || match &field.value {
        Value::Text(t) => Ok(t.as_str()),
        _ => Err(wrong("text in quotes")),
    };
    match field.key.as_str() {
        "takes" => {
            let Value::List(items) = &field.value else {
                return Err(wrong(
                    "a list of sound files' IDs, such as [\"2EGJNFMTCSQ8G\"]",
                ));
            };
            if items.len() > MAX_TAKES {
                return Err(EventError {
                    line,
                    message: format!("{} takes; an event holds up to {MAX_TAKES}", items.len()),
                });
            }
            event.takes.clear();
            for item in items {
                let id = match item {
                    Value::Text(t) => t.parse::<AssetId>().map_err(|e| EventError {
                        line,
                        message: format!("a take's ID: {e}"),
                    })?,
                    _ => return Err(wrong("a list of IDs in quotes")),
                };
                if event.takes.contains(&id) {
                    notes.push(format!(
                        "line {line}: the take {id} is listed twice; kept once"
                    ));
                } else {
                    event.takes.push(id);
                }
            }
        }
        "volume" => event.volume_db = number(VOLUME_RANGE_DB.0, VOLUME_RANGE_DB.1, notes)?,
        "volume_range" => {
            event.volume_range_db = number(0.0, MAX_VOLUME_VARIATION_DB, notes)?;
        }
        "pitch" => event.pitch_st = number(PITCH_RANGE_ST.0, PITCH_RANGE_ST.1, notes)?,
        "pitch_range" => event.pitch_range_st = number(0.0, MAX_PITCH_VARIATION_ST, notes)?,
        "bus" => {
            let name = word()?.trim();
            if name.is_empty()
                || name.chars().count() > MAX_BUS_NAME
                || name.contains(char::is_control)
            {
                return Err(wrong(&format!(
                    "a bus's name, up to {MAX_BUS_NAME} letters"
                )));
            }
            event.bus = name.to_owned();
        }
        "placement" => {
            event.space = match word()? {
                "Everywhere" => Space::Flat,
                "In the world" => Space::Placed,
                _ => return Err(wrong("\"In the world\" or \"Everywhere\"")),
            };
        }
        "near" => event.near_m = number(0.0, MAX_DISTANCE_M, notes)?,
        "far" => event.far_m = number(0.01, MAX_DISTANCE_M, notes)?,
        "fade" => {
            event.fade = match word()? {
                "Realistic" => Fade::Realistic,
                "Even" => Fade::Even,
                "None" => Fade::None,
                _ => return Err(wrong("\"Realistic\", \"Even\" or \"None\"")),
            };
        }
        "at_once" => {
            let n = match field.value {
                Value::Int(n) => n,
                _ => return Err(wrong("a whole number")),
            };
            let fitted = n.clamp(1, i64::from(MAX_AT_ONCE));
            if fitted != n {
                notes.push(format!(
                    "line {line}: at_once = {n} is outside 1 to {MAX_AT_ONCE}, so it's {fitted}"
                ));
            }
            event.at_once = u32::try_from(fitted).unwrap_or(1);
        }
        "priority" => {
            event.priority = match word()? {
                "Low" => Priority::Low,
                "Normal" => Priority::Normal,
                "High" => Priority::High,
                _ => return Err(wrong("\"Low\", \"Normal\" or \"High\"")),
            };
        }
        "loop" => {
            event.looping = match field.value {
                Value::Bool(b) => b,
                _ => return Err(wrong("true or false")),
            };
        }
        "heard_by" => {
            let name = word()?;
            event.heard_by = HeardBy::ALL
                .into_iter()
                .find(|h| h.name() == name)
                .ok_or_else(|| wrong("\"Everyone\", \"Only who caused it\" or \"Groups\""))?;
        }
        "heard_by_groups" => {
            let Value::List(items) = &field.value else {
                return Err(wrong(
                    "a list of group names in quotes, such as [\"Team Red\"]",
                ));
            };
            if items.len() > MAX_GROUPS {
                return Err(EventError {
                    line,
                    message: format!(
                        "{} groups; an event is heard by up to {MAX_GROUPS}",
                        items.len()
                    ),
                });
            }
            event.groups.clear();
            for item in items {
                let Value::Text(name) = item else {
                    return Err(wrong("a list of group names in quotes"));
                };
                let name = name.trim();
                if name.is_empty()
                    || name.chars().count() > MAX_GROUP_NAME
                    || name.contains(char::is_control)
                    || name.contains(',')
                {
                    return Err(wrong(&format!(
                        "a list of group names, each up to {MAX_GROUP_NAME} letters, without commas"
                    )));
                }
                if !event.groups.iter().any(|g| g == name) {
                    event.groups.push(name.to_owned());
                }
            }
        }
        other => notes.push(format!(
            "line {line}: \"{other}\" isn't a sound event setting, so it's skipped"
        )),
    }
    Ok(())
}

/// Writes a sound event as a `.solsound` file, every setting on its own
/// line (so a change is one line in Git).
pub fn write(event: &SoundEvent) -> String {
    let takes = Value::List(
        event
            .takes
            .iter()
            .map(|id| Value::Text(id.to_string()))
            .collect(),
    );
    let f = |v: f32| Value::Float(f64::from(v));
    settings::write_settings(
        KIND,
        VERSION,
        "A sound event. Safe to edit by hand: one line per setting.",
        &[
            ("takes", takes),
            ("volume", f(event.volume_db)),
            ("volume_range", f(event.volume_range_db)),
            ("pitch", f(event.pitch_st)),
            ("pitch_range", f(event.pitch_range_st)),
            ("bus", Value::Text(event.bus.clone())),
            ("placement", Value::Text(event.space.name().to_owned())),
            ("near", f(event.near_m)),
            ("far", f(event.far_m)),
            ("fade", Value::Text(event.fade.name().to_owned())),
            ("at_once", Value::Int(i64::from(event.at_once))),
            ("priority", Value::Text(event.priority.name().to_owned())),
            ("loop", Value::Bool(event.looping)),
            ("heard_by", Value::Text(event.heard_by.name().to_owned())),
            (
                "heard_by_groups",
                Value::List(
                    event
                        .groups
                        .iter()
                        .map(|g| Value::Text(g.clone()))
                        .collect(),
                ),
            ),
        ],
    )
}

/// One play of an event: which take, and how it's varied.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pick {
    /// The take to play.
    pub take: AssetId,
    /// Its volume, in decibels (the event's, varied).
    pub volume_db: f32,
    /// Its pitch, in semitones (the event's, varied).
    pub pitch_st: f32,
}

impl Pick {
    /// The volume as a factor (1 as recorded).
    pub fn amplitude(&self) -> f64 {
        10_f64.powf(f64::from(self.volume_db) / 20.0)
    }

    /// The pitch as a playback speed (1 as recorded, 2 an octave up).
    pub fn speed(&self) -> f64 {
        2_f64.powf(f64::from(self.pitch_st) / 12.0)
    }
}

/// Picks a play of `event`: a take (never `last` again when there's
/// another), and its volume and pitch within the event's ranges. The
/// randomness is sound's own (decision 0056): it comes only from `seed`,
/// `tick` and `order` (the request's place in the tick), so the same
/// game sounds the same every time and sound never touches the game's
/// random numbers. `None` when the event has no takes.
pub fn pick(
    event: &SoundEvent,
    last: Option<AssetId>,
    seed: u64,
    tick: u64,
    order: u64,
) -> Option<Pick> {
    if event.takes.is_empty() {
        return None;
    }
    let mut random = Random::new(seed, tick, order);
    let choices: Vec<AssetId> = if event.takes.len() > 1 {
        event
            .takes
            .iter()
            .copied()
            .filter(|t| Some(*t) != last)
            .collect()
    } else {
        event.takes.clone()
    };
    let at = (random.unit() * choices.len() as f64) as usize;
    let take = choices[at.min(choices.len() - 1)];
    let vary = |random: &mut Random, middle: f32, range: f32| {
        middle + range * (random.unit() as f32 * 2.0 - 1.0)
    };
    let volume_db = vary(&mut random, event.volume_db, event.volume_range_db).clamp(
        VOLUME_RANGE_DB.0 - MAX_VOLUME_VARIATION_DB,
        VOLUME_RANGE_DB.1,
    );
    let pitch_st = vary(&mut random, event.pitch_st, event.pitch_range_st).clamp(
        PITCH_RANGE_ST.0 - MAX_PITCH_VARIATION_ST,
        PITCH_RANGE_ST.1 + MAX_PITCH_VARIATION_ST,
    );
    Some(Pick {
        take,
        volume_db,
        pitch_st,
    })
}

/// Sound's own random numbers (SplitMix64), from a seed, a tick and an
/// order, so they never draw from the game's.
struct Random(u64);

impl Random {
    fn new(seed: u64, tick: u64, order: u64) -> Self {
        let mut r = Self(seed ^ 0x5D58_5D58_5D58_5D58);
        let a = r.next();
        let mut r = Self(a ^ tick);
        let b = r.next();
        Self(b ^ order.wrapping_mul(0x9E37_79B9_7F4A_7C15))
    }

    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number from 0 up to (not including) 1.
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1_u64 << 53) as f64
    }
}
