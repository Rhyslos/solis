//! A play's input, tick by tick, with checksums: played back, the same
//! game comes out the same, and a checksum that differs says at which
//! tick it stopped doing so (Phase 6, determinism).
//!
//! The file is text, a line per tick:
//!
//! ```text
//! solis recording 3
//! seed 12345
//! area areas/farm.solarea
//! start 0f3a…
//! 0 w 9c1d… g 77ab… Move=h,3f800000,0 Jump=hp
//! s 5A329QT768NJB Resumed
//! 1 w 1e2f…
//! ```
//!
//! After the seed, the area the play started in (version 2). Each tick:
//! its number, the world's checksum (`w`), game code's when it was taken
//! (`g`), and each busy action: `h`, `p`, `r` for held, pressed and
//! released, and an axis's two numbers as their exact bits (hex), so
//! playing back is exact. A line starting `s` after a tick is an event a
//! screen's button sent in it (version 3): the entity, the event and its
//! values.

use crate::{ActionState, Frame};
use glam::Vec2;

/// The most ticks a recording holds: an hour at 60 a second.
pub(crate) const MAX_TICKS: usize = 60 * 60 * 60;
/// The biggest recording read.
pub const MAX_FILE_BYTES: usize = 64 << 20;

/// One tick of a recording.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Tick {
    /// The input game code saw.
    pub frame: Frame,
    /// The world's checksum at the end of the tick.
    pub world: u64,
    /// Game code's checksum at the end of the tick, when it was taken
    /// (every second: asking every component costs).
    pub game: Option<u64>,
}

/// A play's input and checksums from Play onwards.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Recording {
    /// The seed the play started from.
    pub seed: u64,
    /// The world's checksum when Play was pressed, before any tick: a
    /// recording only plays back the same from the same start.
    pub start: u64,
    /// Each tick, from the first.
    pub ticks: Vec<Tick>,
    /// The area it was recorded in: its file inside the project's assets,
    /// with `/` between folders (format 2 on; `None` when not known), so
    /// a project's check can play it back on its own (Phase 8, C4).
    pub area: Option<String>,
}

/// The longest area path a recording keeps.
const MAX_AREA_PATH: usize = 1024;

/// Why a recording couldn't be read.
#[derive(Debug, Clone, PartialEq)]
pub enum RecordingError {
    /// Bigger than 64 MB, or longer than an hour of ticks.
    TooBig,
    /// Not a recording, or one from a newer Solis.
    NotARecording,
    /// Damaged at this line (from 1).
    Damaged(usize),
    /// Newer than this build reads, or an older one it couldn't upgrade.
    Upgrade(String),
}

impl std::fmt::Display for RecordingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooBig => write!(f, "it's longer than Solis plays back (an hour, or 64 MB)"),
            Self::NotARecording => write!(f, "it isn't a recording this Solis can read"),
            Self::Damaged(line) => write!(f, "it's damaged at line {line}"),
            Self::Upgrade(why) => f.write_str(why),
        }
    }
}

/// The format version this build writes.
pub(crate) const VERSION: u32 = 3;
/// The first line, before the version.
const HEADER: &str = "solis recording";

/// Recordings' version and upgraders (Phase 8, C1): each step turns a
/// recording read at one version into the next. Every version ever
/// released keeps an example file in `tests/fixtures/recording/`.
pub static FORMAT: reflect::upgrade::Format<Recording> = reflect::upgrade::Format {
    name: "recording",
    current: VERSION,
    version_of: |text| {
        let line = text.lines().next()?.trim();
        line.strip_prefix(HEADER)?.trim().parse().ok()
    },
    steps: &[the_area_isn_t_known, nothing_was_sent],
};

/// Version 2 to 3: screens' buttons sent nothing before version 3
/// (decision 0088), so there's nothing to add.
fn nothing_was_sent(_: &mut Recording) -> Result<(), String> {
    Ok(())
}

/// Version 1 to 2: version 1 didn't say which area it was recorded in.
fn the_area_isn_t_known(recording: &mut Recording) -> Result<(), String> {
    recording.area = None;
    Ok(())
}

impl Tick {
    /// At most how many bytes its line takes in a recording's file (a
    /// little over, never under), so a recording can stop before it's
    /// bigger than Solis reads back.
    pub fn written_len(&self) -> usize {
        // The tick's number and world sum, the game's sum, then each
        // action's name, flags and two numbers by their bits.
        28 + 19
            + self
                .frame
                .actions
                .keys()
                .map(|name| name.len() + 23)
                .sum::<usize>()
            + self
                .frame
                .sent
                .iter()
                .map(|s| 20 + s.name.len() + s.values.iter().map(value_len).sum::<usize>())
                .sum::<usize>()
    }
}

impl Recording {
    /// The file's text.
    pub fn write(&self) -> String {
        // An area path on one line, as written (a line break would
        // break the file's lines).
        let area = self
            .area
            .as_deref()
            .unwrap_or_default()
            .replace(char::is_control, " ");
        let mut text = format!(
            "{HEADER} {VERSION}\nseed {}\narea {area}\nstart {:016x}\n",
            self.seed, self.start
        );
        for (n, tick) in self.ticks.iter().enumerate() {
            text.push_str(&format!("{n} w {:016x}", tick.world));
            if let Some(game) = tick.game {
                text.push_str(&format!(" g {game:016x}"));
            }
            for (name, s) in &tick.frame.actions {
                text.push(' ');
                text.push_str(name);
                text.push('=');
                for (on, c) in [(s.held, 'h'), (s.pressed, 'p'), (s.released, 'r')] {
                    if on {
                        text.push(c);
                    }
                }
                if s.axis != Vec2::ZERO {
                    text.push_str(&format!(
                        ",{:x},{:x}",
                        s.axis.x.to_bits(),
                        s.axis.y.to_bits()
                    ));
                }
            }
            text.push('\n');
            // What screens sent (format 3): a line each, after its tick.
            for sent in &tick.frame.sent {
                text.push_str(&format!("s {} {}", sent.to, sent.name));
                for value in &sent.values {
                    text.push(' ');
                    write_value(&mut text, value);
                }
                text.push('\n');
            }
        }
        text
    }

    /// Reads a recording, within the limits; never panics on damage.
    pub fn read(text: &str) -> Result<Self, RecordingError> {
        if text.len() > MAX_FILE_BYTES {
            return Err(RecordingError::TooBig);
        }
        let version = (FORMAT.version_of)(text).ok_or(RecordingError::NotARecording)?;
        // A newer one is refused before reading on: its layout may not
        // parse. (Upgrading a blank recording only checks the version.)
        if version > VERSION
            && let Err(e) = FORMAT.upgrade(version, &mut Self::default())
        {
            return Err(RecordingError::Upgrade(e.to_string()));
        }
        let mut lines = text.lines().enumerate();
        lines.next();
        let mut field = |name: &str| -> Result<String, RecordingError> {
            let (n, line) = lines.next().ok_or(RecordingError::Damaged(2))?;
            line.strip_prefix(name)
                .and_then(|rest| rest.strip_prefix(' '))
                .map(str::to_string)
                .ok_or(RecordingError::Damaged(n + 1))
        };
        let seed = field("seed")?
            .trim()
            .parse()
            .map_err(|_| RecordingError::Damaged(2))?;
        // Format 2 on: the area it was recorded in (its line may be just
        // `area` when it isn't known).
        let area = if version >= 2 {
            let (n, line) = lines.next().ok_or(RecordingError::Damaged(3))?;
            let path = match line.strip_prefix("area") {
                Some("") => "",
                Some(rest) => rest
                    .strip_prefix(' ')
                    .ok_or(RecordingError::Damaged(n + 1))?,
                None => return Err(RecordingError::Damaged(n + 1)),
            };
            if path.len() > MAX_AREA_PATH
                || (!path.trim().is_empty() && !Self::is_plain_area_path(path.trim()))
            {
                return Err(RecordingError::Damaged(n + 1));
            }
            (!path.trim().is_empty()).then(|| path.trim().to_string())
        } else {
            None
        };
        let mut field = |name: &str| -> Result<String, RecordingError> {
            let (n, line) = lines.next().ok_or(RecordingError::Damaged(4))?;
            line.strip_prefix(name)
                .and_then(|rest| rest.strip_prefix(' '))
                .map(str::to_string)
                .ok_or(RecordingError::Damaged(n + 1))
        };
        let start = u64::from_str_radix(field("start")?.trim(), 16)
            .map_err(|_| RecordingError::Damaged(3))?;
        let mut ticks = Vec::new();
        for (n, line) in lines {
            if line.trim().is_empty() {
                continue;
            }
            if ticks.len() >= MAX_TICKS {
                return Err(RecordingError::TooBig);
            }
            let damaged = RecordingError::Damaged(n + 1);
            // What a screen sent in the tick before (format 3).
            if let Some(rest) = line.strip_prefix("s ").filter(|_| version >= 3) {
                let tick: &mut Tick = ticks.last_mut().ok_or(damaged.clone())?;
                if tick.frame.sent.len() >= crate::MAX_SENT {
                    return Err(damaged);
                }
                tick.frame
                    .sent
                    .push(read_sent(rest).ok_or(damaged.clone())?);
                continue;
            }
            let mut words = line.split_ascii_whitespace();
            let number: usize = words
                .next()
                .and_then(|w| w.parse().ok())
                .ok_or(damaged.clone())?;
            if number != ticks.len() {
                return Err(damaged);
            }
            if words.next() != Some("w") {
                return Err(damaged);
            }
            let hex = |w: Option<&str>| w.and_then(|w| u64::from_str_radix(w, 16).ok());
            let world = hex(words.next()).ok_or(damaged.clone())?;
            let mut tick = Tick {
                world,
                ..Tick::default()
            };
            let mut rest: Vec<&str> = words.collect();
            if rest.first() == Some(&"g") {
                tick.game = Some(hex(rest.get(1).copied()).ok_or(damaged.clone())?);
                rest.drain(..2);
            }
            for word in rest {
                let (name, state) = word.split_once('=').ok_or(damaged.clone())?;
                if !crate::valid_action_name(name) || tick.frame.actions.len() >= crate::MAX_ACTIONS
                {
                    return Err(damaged);
                }
                let mut parts = state.split(',');
                let flags = parts.next().unwrap_or_default();
                if flags.len() > 3 || !flags.chars().all(|c| "hpr".contains(c)) {
                    return Err(damaged);
                }
                let bits = |p: Option<&str>| {
                    p.and_then(|p| u32::from_str_radix(p, 16).ok())
                        .map(f32::from_bits)
                        .filter(|v| v.is_finite())
                };
                let axis = match (parts.next(), parts.next(), parts.next()) {
                    (None, None, None) => Vec2::ZERO,
                    (x @ Some(_), y @ Some(_), None) => Vec2::new(
                        bits(x).ok_or(damaged.clone())?,
                        bits(y).ok_or(damaged.clone())?,
                    ),
                    _ => return Err(damaged),
                };
                tick.frame.actions.insert(
                    name.to_string(),
                    ActionState {
                        held: flags.contains('h'),
                        pressed: flags.contains('p'),
                        released: flags.contains('r'),
                        axis,
                    },
                );
            }
            ticks.push(tick);
        }
        let mut recording = Self {
            seed,
            start,
            ticks,
            area,
        };
        FORMAT
            .upgrade(version, &mut recording)
            .map_err(|e| RecordingError::Upgrade(e.to_string()))?;
        Ok(recording)
    }

    /// Whether `path` is a plain path inside a project's assets: names
    /// joined by `/`, none of them empty, `.` or `..`, and no backslash,
    /// colon or control character (so it can't name anything outside the
    /// assets on any system).
    pub fn is_plain_area_path(path: &str) -> bool {
        !path.is_empty()
            && !path.contains(['\\', ':'])
            && !path.contains(char::is_control)
            && path
                .split('/')
                .all(|p| !p.is_empty() && p != "." && p != "..")
    }

    /// The first tick (from 0) where `other`'s checksums differ from
    /// these, among the ticks both have; `None` when they all agree.
    pub fn first_difference(&self, other: &[Tick]) -> Option<usize> {
        self.ticks.iter().zip(other).position(|(a, b)| {
            a.world != b.world || matches!((a.game, b.game), (Some(x), Some(y)) if x != y)
        })
    }
}

/// At most how many bytes a value takes in a recording.
fn value_len(value: &reflect::Value) -> usize {
    match value {
        reflect::Value::Text(t) => 2 + t.len() * 2,
        reflect::Value::List(items) => 2 + items.iter().map(|v| value_len(v) + 1).sum::<usize>(),
        _ => 40,
    }
}

/// A value as one word: `b1`, `i-3`, `f<bits>`, `t<hex>`, `e<id>` or `e-`,
/// `l(...)` for a list (its values with `,` between). Assets aren't sent.
fn write_value(text: &mut String, value: &reflect::Value) {
    use reflect::Value;
    match value {
        Value::Bool(b) => text.push_str(if *b { "b1" } else { "b0" }),
        Value::Int(n) => text.push_str(&format!("i{n}")),
        Value::Float(v) => text.push_str(&format!("f{:x}", v.to_bits())),
        Value::Text(t) => {
            text.push('t');
            for byte in t.bytes() {
                text.push_str(&format!("{byte:02x}"));
            }
        }
        Value::Entity(Some(id)) => text.push_str(&format!("e{id}")),
        Value::Entity(None) | Value::Asset(_) => text.push_str("e-"),
        Value::List(items) => {
            text.push_str("l(");
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    text.push(',');
                }
                write_value(text, item);
            }
            text.push(')');
        }
    }
}

/// One `s` line's rest: the entity, the event's name and its values.
fn read_sent(rest: &str) -> Option<crate::Sent> {
    let mut words = rest.split(' ');
    let to = words.next()?.parse().ok()?;
    let name = words.next()?;
    if name.is_empty()
        || name.len() > crate::MAX_SENT_NAME
        || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return None;
    }
    let mut values = Vec::new();
    for word in words {
        if values.len() >= crate::MAX_SENT_VALUES {
            return None;
        }
        values.push(read_value(word, 0)?);
    }
    Some(crate::Sent {
        to,
        name: name.to_string(),
        values,
    })
}

/// A value written by [`write_value`]; lists go at most 4 deep.
fn read_value(word: &str, depth: usize) -> Option<reflect::Value> {
    use reflect::Value;
    let (kind, rest) = word.split_at_checked(1)?;
    Some(match kind {
        "b" => match rest {
            "1" => Value::Bool(true),
            "0" => Value::Bool(false),
            _ => return None,
        },
        "i" => Value::Int(rest.parse().ok()?),
        "f" => {
            let v = f64::from_bits(u64::from_str_radix(rest, 16).ok()?);
            if !v.is_finite() {
                return None;
            }
            Value::Float(v)
        }
        "t" => {
            if rest.len() % 2 != 0 || rest.len() > 2 * crate::MAX_SENT_TEXT {
                return None;
            }
            let bytes: Option<Vec<u8>> = (0..rest.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(rest.get(i..i + 2)?, 16).ok())
                .collect();
            Value::Text(String::from_utf8(bytes?).ok()?)
        }
        "e" if rest == "-" => Value::Entity(None),
        "e" => Value::Entity(Some(rest.parse().ok()?)),
        "l" if depth < 4 => {
            let inner = rest.strip_prefix('(')?.strip_suffix(')')?;
            let mut items = Vec::new();
            // Split at the commas of this list only, not of lists inside.
            let (mut level, mut start) = (0usize, 0usize);
            for (i, c) in inner.char_indices() {
                match c {
                    '(' => level += 1,
                    ')' => level = level.checked_sub(1)?,
                    ',' if level == 0 => {
                        items.push(read_value(&inner[start..i], depth + 1)?);
                        start = i + 1;
                    }
                    _ => {}
                }
                if items.len() > crate::MAX_SENT_VALUES {
                    return None;
                }
            }
            if !inner.is_empty() {
                items.push(read_value(&inner[start..], depth + 1)?);
            }
            Value::List(items)
        }
        _ => return None,
    })
}
