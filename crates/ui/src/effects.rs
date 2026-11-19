//! What a text's markup does to its letters once they're laid out (stage
//! 1, text effects): each letter is its own shape, so a wave, a shake, a
//! colour, a slant or typing out changes only that letter's shape, never
//! the layout, and costs nothing when the text has none.

use crate::markup::{Effect, Icon, Marked, Run};
use crate::typing::TypeOut;

/// How high a wave lifts a letter, times the text's size.
pub const WAVE_HEIGHT: f32 = 0.12;
/// How many times a second a wave goes round.
const WAVE_SPEED: f64 = 1.2;
/// How far along a wave each letter is from the one before, in turns.
const WAVE_STEP: f64 = 0.09;
/// How far a shake moves a letter, times the text's size.
pub const SHAKE_REACH: f32 = 0.05;
/// How many times a second a shaking letter jumps.
const SHAKE_RATE: f64 = 24.0;
/// How far a slanted letter's top leans right, for each pixel it's tall.
pub const SLANT: f32 = 0.2;
/// How much bolder `[b]` makes a letter its font has no bold of, times
/// the text's size (its edge moved out).
pub const FAUX_BOLD: f32 = 0.035;

/// A text's effects, as its element carries them.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TextEffects {
    /// The effects over its parts.
    pub runs: Vec<Run>,
    /// Its pictures, in order.
    pub icons: Vec<Icon>,
    /// When its letters show as it types out, if it does.
    pub typing: Option<TypeOut>,
    /// Its place among its view's texts, by which the screen keeps when
    /// it began typing out.
    pub number: u32,
}

impl TextEffects {
    /// What `marked` does, for text number `number`, typing out at `speed`
    /// letters a second (0: at once). None when it does nothing.
    pub fn of(marked: &Marked, number: u32, speed: Option<f32>) -> Option<Self> {
        let typing = speed
            .map(|s| TypeOut::of(marked, s))
            .filter(TypeOut::waits);
        if marked.runs.is_empty() && marked.icons.is_empty() && typing.is_none() {
            return None;
        }
        Some(Self {
            runs: marked.runs.clone(),
            icons: marked.icons.clone(),
            typing,
            number,
        })
    }

    /// The effects over byte `byte` of the plain text, outer ones first.
    pub fn at(&self, byte: u32) -> impl Iterator<Item = Effect> + '_ {
        self.runs
            .iter()
            .filter(move |r| r.from <= byte && byte < r.to)
            .map(|r| r.effect)
    }
}

/// What the effects make of one letter.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct LetterLook {
    /// Moved by this much, in design pixels at the text's size (times its
    /// size, so a bigger text waves higher).
    pub(crate) moved: [f32; 2],
    /// Its colour, if an effect gives it one (0xRRGGBB).
    pub(crate) colour: Option<u32>,
    /// Bold.
    pub(crate) bold: bool,
    /// Slanted.
    pub(crate) slant: bool,
}

/// `effects` on the letter numbered `index` (its place in its text) at
/// byte `byte`, at UI time `now` seconds.
pub(crate) fn letter_look(effects: &TextEffects, byte: u32, index: usize, now: f64) -> LetterLook {
    let mut look = LetterLook::default();
    for effect in effects.at(byte) {
        match effect {
            Effect::Wave => {
                let turn = now * WAVE_SPEED - index as f64 * WAVE_STEP;
                let lift = (turn.fract() * std::f64::consts::TAU).sin() as f32;
                look.moved[1] -= lift * WAVE_HEIGHT;
            }
            Effect::Shake => {
                let jump = (now * SHAKE_RATE).floor() as i64;
                look.moved[0] += noise(index, jump, 1) * SHAKE_REACH;
                look.moved[1] += noise(index, jump, 2) * SHAKE_REACH;
            }
            // The innermost colour wins.
            Effect::Colour(c) => look.colour = Some(c),
            Effect::Bold => look.bold = true,
            Effect::Italic => look.slant = true,
        }
    }
    look
}

/// A number from -1 to 1 for letter `index` at jump `jump`, the same each
/// time it's asked (a hash, never the game's random numbers).
fn noise(index: usize, jump: i64, salt: u64) -> f32 {
    let mut x = (index as u64)
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add((jump as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9))
        .wrapping_add(salt.wrapping_mul(0x94D0_49BB_1331_11EB));
    x ^= x >> 31;
    x = x.wrapping_mul(0xD6E8_FEB8_6659_FD93);
    x ^= x >> 32;
    (x >> 40) as f32 / (1u64 << 23) as f32 - 1.0
}

#[cfg(test)]
#[path = "effects_tests.rs"]
mod tests;
