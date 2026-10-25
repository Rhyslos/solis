//! A play's input, tick by tick, with checksums: played back, the same
//! game comes out the same, and a checksum that differs says at which
//! tick it stopped doing so (Phase 6, determinism).
//!
//! The file is text, a line per tick:
//!
//! ```text
//! solis recording 1
//! seed 12345
//! start 0f3a…
//! 0 w 9c1d… g 77ab… Move=h,3f800000,0 Jump=hp
//! 1 w 1e2f…
//! ```
//!
//! Each tick: its number, the world's checksum (`w`), game code's when
//! it was taken (`g`), and each busy action: `h`, `p`, `r` for held,
//! pressed and released, and an axis's two numbers as their exact bits
//! (hex), so playing back is exact.

use crate::{ActionState, Frame};
use glam::Vec2;

/// The most ticks a recording holds: an hour at 60 a second.
pub(crate) const MAX_TICKS: usize = 60 * 60 * 60;
/// The biggest recording read.
pub(crate) const MAX_FILE_BYTES: usize = 64 << 20;

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
}

/// Why a recording couldn't be read.
#[derive(Debug, Clone, PartialEq)]
pub enum RecordingError {
    /// Bigger than [`MAX_FILE_BYTES`], or longer than [`MAX_TICKS`].
    TooBig,
    /// Not a recording, or one from a newer Solis.
    NotARecording,
    /// Damaged at this line (from 1).
    Damaged(usize),
}

impl std::fmt::Display for RecordingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooBig => write!(f, "it's longer than Solis plays back (an hour, or 64 MB)"),
            Self::NotARecording => write!(f, "it isn't a recording this Solis can read"),
            Self::Damaged(line) => write!(f, "it's damaged at line {line}"),
        }
    }
}

const HEADER: &str = "solis recording 1";

impl Recording {
    /// The file's text.
    pub fn write(&self) -> String {
        let mut text = format!(
            "{HEADER}\nseed {}\nstart {:016x}\n",
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
                    text.push_str(&format!(",{:x},{:x}", s.axis.x.to_bits(), s.axis.y.to_bits()));
                }
            }
            text.push('\n');
        }
        text
    }

    /// Reads a recording, within the limits; never panics on damage.
    pub fn read(text: &str) -> Result<Self, RecordingError> {
        if text.len() > MAX_FILE_BYTES {
            return Err(RecordingError::TooBig);
        }
        let mut lines = text.lines().enumerate();
        if lines.next().map(|(_, l)| l.trim()) != Some(HEADER) {
            return Err(RecordingError::NotARecording);
        }
        let mut field = |name: &str| -> Result<String, RecordingError> {
            let (n, line) = lines.next().ok_or(RecordingError::Damaged(2))?;
            line.strip_prefix(name)
                .and_then(|rest| rest.strip_prefix(' '))
                .map(str::to_string)
                .ok_or(RecordingError::Damaged(n + 1))
        };
        let seed = field("seed")?.trim().parse().map_err(|_| RecordingError::Damaged(2))?;
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
                if !crate::valid_action_name(name) || tick.frame.actions.len() >= crate::MAX_ACTIONS {
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
        Ok(Self { seed, start, ticks })
    }

    /// The first tick (from 0) where `other`'s checksums differ from
    /// these, among the ticks both have; `None` when they all agree.
    pub fn first_difference(&self, other: &[Tick]) -> Option<usize> {
        self.ticks.iter().zip(other).position(|(a, b)| {
            a.world != b.world || matches!((a.game, b.game), (Some(x), Some(y)) if x != y)
        })
    }
}
