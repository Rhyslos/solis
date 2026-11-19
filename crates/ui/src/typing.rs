//! A text typing itself out (stage 1, text effects): when each of its
//! letters appears, worked out once from the screen's typing speed and
//! the text's `[pause]` and `[speed]` words.
//!
//! Typing out is UI, never the game's state (decision 0087): what shows is
//! a function of how long the text has been typing, counted on the game's
//! ticks plus how far into the next one, so the same letters show at the
//! same moments at 30 or 144 frames a second.

use crate::markup::{Beat, ICON, Marked};

/// When one character of the plain text shows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Moment {
    /// Where it is in the plain text, in bytes.
    pub byte: u32,
    /// When it shows, in seconds from the start of typing.
    pub at: f64,
    /// Whether it's a letter (not a space), and whether it starts a word:
    /// what a sound per letter or per word plays on.
    pub letter: bool,
    /// It starts a word.
    pub word: bool,
}

/// When each character of a text shows as it types out.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TypeOut {
    /// Each character, in order.
    pub moments: Vec<Moment>,
    /// When it has finished: its last letter shown and any pause after it.
    pub done: f64,
}

/// Whether a text sounds on each letter or each word.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SoundEvery {
    /// Every letter shown (not spaces).
    #[default]
    Letter,
    /// The first letter of each word.
    Word,
}

impl TypeOut {
    /// `marked` typed out at `speed` letters a second (0 or less: at once),
    /// with its pauses and speed changes.
    pub fn of(marked: &Marked, speed: f32) -> Self {
        let speed = if speed.is_finite() {
            f64::from(speed.max(0.0))
        } else {
            0.0
        };
        let mut beats = marked.beats.iter().peekable();
        let mut times = 1.0_f64;
        let mut t = 0.0_f64;
        let mut moments = Vec::with_capacity(marked.plain.len());
        let mut after_space = true;
        for (index, (byte, c)) in marked.plain.char_indices().enumerate() {
            let index = u32::try_from(index).unwrap_or(u32::MAX);
            while let Some((_, beat)) = beats.next_if(|(i, _)| *i <= index) {
                apply(*beat, &mut t, &mut times);
            }
            let space = c.is_whitespace() && c != ICON;
            moments.push(Moment {
                byte: u32::try_from(byte).unwrap_or(u32::MAX),
                at: t,
                letter: !space,
                word: !space && after_space,
            });
            after_space = space;
            let rate = speed * times;
            if rate > 0.0 {
                t += 1.0 / rate;
            }
        }
        // The last letter shows at its moment; a pause after it still
        // counts before typing is finished.
        let mut done = moments.last().map_or(0.0, |m| m.at);
        let mut end = t;
        for (_, beat) in beats {
            if let Beat::Pause(s) = beat {
                end += f64::from(*s);
                done = end;
            }
        }
        Self { moments, done }
    }

    /// Whether anything waits: some letter shows later than the start.
    pub fn waits(&self) -> bool {
        self.done > 0.0
    }

    /// How many characters show `elapsed` seconds after typing began.
    pub fn shown(&self, elapsed: f64) -> usize {
        if elapsed.is_nan() {
            return self.moments.len();
        }
        self.moments.partition_point(|m| m.at <= elapsed)
    }

    /// Whether the character at byte `byte` shows `elapsed` seconds after
    /// typing began (a byte past the end shows: "…" and the like).
    pub fn shows(&self, byte: u32, elapsed: f64) -> bool {
        let i = self.moments.partition_point(|m| m.byte < byte);
        self.moments
            .get(i)
            .filter(|m| m.byte == byte)
            .is_none_or(|m| elapsed.is_nan() || m.at <= elapsed)
    }

    /// Whether typing is finished `elapsed` seconds after it began.
    pub fn finished(&self, elapsed: f64) -> bool {
        elapsed.is_nan() || elapsed >= self.done
    }

    /// Whether a letter (or a word, by `every`) shows after `from` seconds
    /// and by `to`: what a typing sound plays on, once however many did.
    pub fn sounds_between(&self, from: f64, to: f64, every: SoundEvery) -> bool {
        let first = self.moments.partition_point(|m| m.at <= from);
        self.moments[first..]
            .iter()
            .take_while(|m| m.at <= to)
            .any(|m| match every {
                SoundEvery::Letter => m.letter,
                SoundEvery::Word => m.word,
            })
    }
}

fn apply(beat: Beat, t: &mut f64, times: &mut f64) {
    match beat {
        Beat::Pause(s) => *t += f64::from(s),
        Beat::Speed(s) => *times = f64::from(s),
    }
}

/// When each text of a screen began typing out, by its number among the
/// view's texts (see [`crate::view::texts_of`]), in UI seconds: what the
/// editor keeps as the game runs. A text with no start, or one of
/// [`f64::NEG_INFINITY`] (finished at once: skipped), shows whole.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Started {
    /// Each text's start, by its number.
    pub at: Vec<f64>,
}

impl Started {
    /// How long text `number` has been typing at UI time `now`: NaN for
    /// one that shows whole.
    pub fn elapsed(&self, number: u32, now: f64) -> f64 {
        match self.at.get(number as usize) {
            Some(start) if start.is_finite() && now.is_finite() => (now - start).max(0.0),
            _ => f64::NAN,
        }
    }
}

#[cfg(test)]
#[path = "typing_tests.rs"]
mod tests;
