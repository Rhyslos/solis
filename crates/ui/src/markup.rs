//! Text effects written inside a screen's text (stage 1, text effects):
//! a small, fixed set of words in square brackets, read into plain text
//! and what's done to its parts.
//!
//! - `[wave]…[/wave]` and `[shake]…[/shake]` move letters;
//! - `[colour=gold]…[/colour]` colours them (Lust's colour words, or
//!   `#hex`);
//! - `[b]…[/b]` and `[i]…[/i]`: bold and slanted;
//! - `[icon=Picture.Coin]`: a picture the height of a line, by the
//!   project's name for it (Atlas Crop's, decision 0089);
//! - `[pause 0.5]` and `[speed 2]` while a text types itself out.
//!
//! `[[` writes a `[`. A word that isn't one of these, or one never closed,
//! is a [`Mistake`] (Problems lists it when the screen is checked) and is
//! shown as written, never hidden. Markup is read after translation, so a
//! translated text carries its own.

use std::fmt::Write as _;

/// The character an icon stands in the plain text as: laid out as a box
/// the height of a line, drawn as its picture.
pub const ICON: char = '\u{FFFC}';
/// The most words read from one text; the rest are shown as written.
pub const MAX_WORDS: usize = 512;
/// The longest a word may be between its brackets, in characters.
const MAX_WORD: usize = 96;
/// The most effects open inside each other.
const MAX_OPEN: usize = 16;
/// The longest pause, in seconds, and the fastest speed.
pub const MAX_PAUSE: f32 = 60.0;
/// The most `[speed n]` multiplies the screen's typing speed by.
pub const MAX_SPEED: f32 = 100.0;

/// What's done to a part of the text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Effect {
    /// Letters rise and fall in a wave.
    Wave,
    /// Letters shake.
    Shake,
    /// Letters in this colour (0xRRGGBB).
    Colour(u32),
    /// Bold.
    Bold,
    /// Slanted.
    Italic,
}

impl Effect {
    /// The word that opens it, as written (`wave`, `colour=#ffcc00`).
    fn opening(self) -> String {
        match self {
            Self::Wave => "wave".into(),
            Self::Shake => "shake".into(),
            Self::Colour(c) => format!("colour=#{c:06x}"),
            Self::Bold => "b".into(),
            Self::Italic => "i".into(),
        }
    }

    /// The word that closes it.
    fn name(self) -> &'static str {
        match self {
            Self::Wave => "wave",
            Self::Shake => "shake",
            Self::Colour(_) => "colour",
            Self::Bold => "b",
            Self::Italic => "i",
        }
    }
}

/// An effect over part of the plain text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Run {
    /// Where it starts and ends in the plain text, in bytes.
    pub from: u32,
    /// Where it ends (one past its last byte).
    pub to: u32,
    /// What it does.
    pub effect: Effect,
}

/// A picture inside the text.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Icon {
    /// Where its [`ICON`] is in the plain text, in bytes.
    pub at: u32,
    /// The project's name for it (`Picture.Coin`).
    pub name: String,
}

/// A change to typing out, before a letter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Beat {
    /// Wait this many seconds.
    Pause(f32),
    /// From here on, type this many times the screen's speed (0 at once).
    Speed(f32),
}

/// A mistake in a text's markup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mistake {
    /// Where its `[` is in the text, in characters from 0.
    pub at: u32,
    /// What's wrong, and what to write instead.
    pub message: String,
}

/// A text read: its plain words and what's done to them.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Marked {
    /// The words shown, each icon as an [`ICON`].
    pub plain: String,
    /// The effects, outer ones first where they start together.
    pub runs: Vec<Run>,
    /// The icons, in order.
    pub icons: Vec<Icon>,
    /// The pauses and speed changes, before the character (counted from
    /// 0) they come before, in order.
    pub beats: Vec<(u32, Beat)>,
    /// What's wrong with it.
    pub mistakes: Vec<Mistake>,
}

impl Marked {
    /// Whether it does anything to its words.
    pub fn is_plain(&self) -> bool {
        self.runs.is_empty() && self.icons.is_empty() && self.beats.is_empty()
    }
}

/// What a word between brackets means.
enum Word {
    Open(Effect),
    Close(&'static str),
    Icon(String),
    Beat(Beat),
}

/// The words, for a mistake's message.
const WORDS: &str = "wave, shake, colour, b, i, icon, pause and speed";

/// One word between brackets (without them) read, or why not.
fn read_word(inside: &str) -> Result<Word, String> {
    let inside = inside.trim();
    if let Some(name) = inside.strip_prefix('/') {
        return match name.trim() {
            "wave" => Ok(Word::Close("wave")),
            "shake" => Ok(Word::Close("shake")),
            "colour" => Ok(Word::Close("colour")),
            "b" => Ok(Word::Close("b")),
            "i" => Ok(Word::Close("i")),
            "color" => Err("[/color] is written [/colour]".into()),
            other => Err(format!(
                "[/{other}] closes nothing: only wave, shake, colour, b and i are closed"
            )),
        };
    }
    let (name, value) = match inside.find(['=', ' ']) {
        Some(at) => (&inside[..at], Some(inside[at + 1..].trim())),
        None => (inside, None),
    };
    let needs_none = |word: Word| match value {
        None => Ok(word),
        Some(_) => Err(format!("[{name}] takes nothing after it: write [{name}]")),
    };
    match name {
        "wave" => needs_none(Word::Open(Effect::Wave)),
        "shake" => needs_none(Word::Open(Effect::Shake)),
        "b" => needs_none(Word::Open(Effect::Bold)),
        "i" => needs_none(Word::Open(Effect::Italic)),
        "colour" => {
            let value = value.unwrap_or("");
            crate::view::colour(value)
                .map(|c| Word::Open(Effect::Colour(c)))
                .ok_or_else(|| {
                    format!(
                        "\"{value}\" isn't a colour: write one of the screen's colour words \
                         (yellow5, red, teal7…) or #hex, like [colour=#ffcc00]"
                    )
                })
        }
        "icon" => {
            let value = value.unwrap_or("");
            if value.starts_with("Picture.") && value.len() > "Picture.".len() {
                Ok(Word::Icon(value.to_string()))
            } else {
                Err(format!(
                    "an icon names one of the project's pictures, like [icon=Picture.Coin], \
                     not \"{value}\""
                ))
            }
        }
        "pause" | "speed" => {
            let number = value
                .and_then(|v| v.parse::<f32>().ok())
                .filter(|n| n.is_finite());
            match (name, number) {
                ("pause", Some(s)) if (0.0..=MAX_PAUSE).contains(&s) => {
                    Ok(Word::Beat(Beat::Pause(s)))
                }
                ("speed", Some(s)) if (0.0..=MAX_SPEED).contains(&s) => {
                    Ok(Word::Beat(Beat::Speed(s)))
                }
                ("pause", _) => Err(format!(
                    "[pause] takes the seconds to wait, from 0 to {MAX_PAUSE}: [pause 0.5]"
                )),
                _ => Err(format!(
                    "[speed] takes how many times the screen's typing speed, from 0 (at once) \
                     to {MAX_SPEED}: [speed 2]"
                )),
            }
        }
        "button" => Err(
            "button pictures in text come in a later milestone (they need the pictures of each \
             controller's buttons): name the button in words for now"
                .into(),
        ),
        "color" => Err("[color] is written [colour=…]".into()),
        "bold" => Err("bold is written [b]…[/b]".into()),
        "italic" => Err("italic is written [i]…[/i]".into()),
        other => Err(format!(
            "[{other}] isn't a text effect: the words are {WORDS}; write [[ for a [ in text"
        )),
    }
}

/// What reading a text keeps as it goes.
#[derive(Default)]
struct Reading {
    out: Marked,
    /// Effects open: what, and where they started in the plain text.
    open: Vec<(Effect, u32)>,
    /// Characters written into the plain text so far.
    chars: u32,
    words: usize,
}

impl Reading {
    fn push(&mut self, c: char) {
        // A text's own stand-in character never passes for an icon.
        let c = if c == ICON { '\u{FFFD}' } else { c };
        self.out.plain.push(c);
        self.chars += 1;
    }

    fn push_str(&mut self, s: &str) {
        for c in s.chars() {
            self.push(c);
        }
    }

    fn here(&self) -> u32 {
        u32::try_from(self.out.plain.len()).unwrap_or(u32::MAX)
    }

    fn mistake(&mut self, at: usize, message: String) {
        self.out.mistakes.push(Mistake {
            at: u32::try_from(at).unwrap_or(u32::MAX),
            message,
        });
    }

    /// Takes in a word read at character `at` of the text: false when it
    /// can't be (it's then written as it is).
    fn take(&mut self, word: Word, at: usize) -> bool {
        match word {
            Word::Open(effect) => {
                if self.open.len() >= MAX_OPEN {
                    self.mistake(at, format!("more than {MAX_OPEN} effects are open at once"));
                    return false;
                }
                let here = self.here();
                self.open.push((effect, here));
            }
            Word::Close(name) => {
                let Some(i) = self.open.iter().rposition(|(e, _)| e.name() == name) else {
                    self.mistake(
                        at,
                        format!("[/{name}] closes nothing: no [{name}] is open before it"),
                    );
                    return false;
                };
                let (effect, from) = self.open.remove(i);
                let to = self.here();
                self.out.runs.push(Run { from, to, effect });
            }
            Word::Icon(name) => {
                let here = self.here();
                self.out.icons.push(Icon { at: here, name });
                self.out.plain.push(ICON);
                self.chars += 1;
            }
            Word::Beat(beat) => self.out.beats.push((self.chars, beat)),
        }
        true
    }
}

/// `text` read: its plain words, effects, icons, pauses and speeds, and
/// what's wrong with its markup.
pub fn parse(text: &str) -> Marked {
    if !text.contains('[') && !text.contains(ICON) {
        return Marked {
            plain: text.to_string(),
            ..Marked::default()
        };
    }
    let chars: Vec<char> = text.chars().collect();
    let mut r = Reading::default();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c != '[' {
            r.push(c);
            i += 1;
            continue;
        }
        if chars.get(i + 1) == Some(&'[') {
            r.push('[');
            i += 2;
            continue;
        }
        let end = chars[i + 1..]
            .iter()
            .take(MAX_WORD + 1)
            .position(|&c| c == ']' || c == '[' || c == '\n');
        let Some(len) = end.filter(|&n| chars[i + 1 + n] == ']') else {
            r.mistake(i, "a [ with no ] after it: write [[ for a [ in text".into());
            r.push('[');
            i += 1;
            continue;
        };
        let inside: String = chars[i + 1..i + 1 + len].iter().collect();
        let whole = i + len + 2;
        r.words += 1;
        if r.words > MAX_WORDS {
            if r.words == MAX_WORDS + 1 {
                r.mistake(i, format!("more than {MAX_WORDS} effects in one text"));
            }
            r.push_str(&format!("[{inside}]"));
        } else {
            match read_word(&inside) {
                Ok(word) => {
                    if !r.take(word, i) {
                        r.push_str(&format!("[{inside}]"));
                    }
                }
                Err(message) => {
                    r.mistake(i, message);
                    r.push_str(&format!("[{inside}]"));
                }
            }
        }
        i = whole;
    }
    let end = r.here();
    for (effect, from) in std::mem::take(&mut r.open) {
        let name = effect.name();
        // Said at the end of the text, where the closing word is missing.
        r.mistake(
            chars.len(),
            format!("[{name}] is never closed: end it with [/{name}]"),
        );
        r.out.runs.push(Run {
            from,
            to: end,
            effect,
        });
    }
    let mut out = r.out;
    out.runs.retain(|run| run.to > run.from);
    out.runs
        .sort_by_key(|run| (run.from, std::cmp::Reverse(run.to), run.effect));
    out
}

/// `marked` written back as markup: [`parse`] of it gives it again
/// (its mistakes aside).
pub fn write(marked: &Marked) -> String {
    let plain = &marked.plain;
    let mut out = String::with_capacity(plain.len() + 16 * marked.runs.len());
    let mut open: Vec<Run> = Vec::new();
    let mut runs = marked.runs.iter().peekable();
    let mut icons = marked.icons.iter();
    let mut beats = marked.beats.iter().peekable();
    let ends = plain
        .char_indices()
        .map(|(at, c)| (at, Some(c)))
        .chain(std::iter::once((plain.len(), None)));
    for (index, (at, c)) in ends.enumerate() {
        let index = u32::try_from(index).unwrap_or(u32::MAX);
        let at = u32::try_from(at).unwrap_or(u32::MAX);
        // Close what ends here, innermost first.
        let mut i = open.len();
        while i > 0 {
            i -= 1;
            if open[i].to <= at {
                let _ = write!(out, "[/{}]", open[i].effect.name());
                open.remove(i);
            }
        }
        while let Some((_, beat)) = beats.next_if(|(i, _)| *i == index) {
            let _ = match beat {
                Beat::Pause(s) => write!(out, "[pause {s}]"),
                Beat::Speed(s) => write!(out, "[speed {s}]"),
            };
        }
        while let Some(run) = runs.next_if(|run| run.from == at) {
            let _ = write!(out, "[{}]", run.effect.opening());
            open.push(*run);
        }
        match c {
            Some(ICON) => {
                if let Some(icon) = icons.next() {
                    let _ = write!(out, "[icon={}]", icon.name);
                }
            }
            Some('[') => out.push_str("[["),
            Some(c) => out.push(c),
            None => {}
        }
    }
    out
}

#[cfg(test)]
#[path = "markup_tests.rs"]
mod tests;
