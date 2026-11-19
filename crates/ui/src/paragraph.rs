//! A text laid out as a paragraph (stage 1, text layout): letters shaped
//! in the first font that has them, broken onto lines by Unicode's
//! line-break rules inside a width, and cut with "…" when a box asks.
//!
//! Everything is in design pixels at the text's size, from the
//! paragraph's top left: the screen's scale only multiplies it, so one
//! layout serves every screen size, and [`crate::text::Fonts`] keeps it
//! while the text, its look and its width stay the same.

use crate::font::GlyphId;
use crate::text::Slot;
use unicode_linebreak::BreakOpportunity;

/// How a text looks: its size and fonts, and the room between its lines
/// and letters.
#[derive(Debug, Clone, PartialEq)]
pub struct TextStyle {
    /// How tall its letters are, in design pixels (from the top of the
    /// font's ascent to the bottom of its descent).
    pub size: f32,
    /// Which of the game's own fonts (0: the standard one, 1: its bold),
    /// used for any letter the named fonts don't have.
    pub font: usize,
    /// The project's fonts to draw it in, by their place in the assets
    /// (`fonts/Lora.ttf`), first choice first: a letter the first doesn't
    /// have comes from the next (fallback fonts), then from [`Self::font`].
    pub fonts: Vec<String>,
    /// The room from one line to the next, times the font's own (1).
    pub line_height: f32,
    /// Room added after each letter, in design pixels (0: the font's).
    pub letter_spacing: f32,
    /// What its markup changes in its layout: bold parts and icons'
    /// widths (stage 1, text effects).
    pub inline: Inline,
}

/// What a text's markup changes in how it's laid out (stage 1, text
/// effects); every other effect leaves the layout alone.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Inline {
    /// The parts in bold, as bytes of the text (from, to): letters from
    /// the game's own font come from its bold, which is wider.
    pub bold: Vec<[u32; 2]>,
    /// Each icon's width over its height, in order: each
    /// [`crate::markup::ICON`] in the text is a box as tall as the text's
    /// size and this many times as wide.
    pub icons: Vec<f32>,
}

impl Inline {
    /// Whether byte `byte` is in bold.
    fn bold_at(&self, byte: usize) -> bool {
        self.bold
            .iter()
            .any(|[from, to]| (*from as usize..*to as usize).contains(&byte))
    }
}

impl TextStyle {
    /// Text `size` design pixels tall in the game's font `font`, as the
    /// font spaces it.
    pub fn new(size: f32, font: usize) -> Self {
        Self {
            size,
            font,
            fonts: Vec::new(),
            line_height: 1.0,
            letter_spacing: 0.0,
            inline: Inline::default(),
        }
    }
}

/// The room a text has, and what happens when it doesn't fit.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Fit {
    /// How wide it may be, in design pixels (none: as wide as it is).
    pub width: Option<f32>,
    /// Whether words wrap onto the next line at [`Self::width`] (else a
    /// line is as long as its words, up to a `\n`).
    pub wrap: bool,
    /// The most lines it shows (none: every one).
    pub lines: Option<u32>,
    /// Whether text that doesn't fit is cut with "…": the lines past
    /// [`Self::lines`], and a line wider than [`Self::width`] that can't
    /// wrap.
    pub cut: bool,
}

/// The [`Glyph::slot`] of an icon: its [`Glyph::id`] is the icon's
/// number in its text.
pub(crate) const ICON_SLOT: u16 = u16::MAX;

/// One letter placed in a paragraph.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Glyph {
    /// The font it's from: its place among the fonts laid out with
    /// ([`ICON_SLOT`] for an icon).
    pub(crate) slot: u16,
    /// The font's letter (an icon's number).
    pub(crate) id: GlyphId,
    /// Where its pen is: from its line's left, and from its baseline (down).
    pub(crate) at: [f32; 2],
    /// Where its characters start in the text, in bytes.
    pub(crate) cluster: u32,
}

/// One line of a paragraph.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Line {
    /// How wide its letters reach, spaces at its end left out.
    pub width: f32,
    /// Its baseline, from the paragraph's top.
    pub baseline: f32,
    /// Its letters: where they start and end in the paragraph's.
    pub(crate) glyphs: [u32; 2],
}

/// A text laid out: its lines and letters, in design pixels.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Paragraph {
    /// How wide (its widest line) and tall (all its lines) it is.
    pub size: [f32; 2],
    /// Its lines, top to bottom.
    pub lines: Vec<Line>,
    /// The size its letters are, in design pixels.
    pub text_size: f32,
    /// Whether it was cut with "…".
    pub cut: bool,
    pub(crate) glyphs: Vec<Glyph>,
    /// Each icon's width, in design pixels, by its number.
    pub(crate) icons: Vec<f32>,
}

impl Paragraph {
    /// The letters of line `line` (its place among [`Self::lines`]).
    pub(crate) fn glyphs_of(&self, line: &Line) -> &[Glyph] {
        let [from, to] = line.glyphs.map(|i| i as usize);
        self.glyphs.get(from..to).unwrap_or(&[])
    }

    /// How many letters it holds.
    pub fn letters(&self) -> usize {
        self.glyphs.len()
    }
}

/// A little room, so a line exactly as wide as its box (worked out at
/// another scale) still fits.
const EASE: f32 = 0.5;

/// A letter shaped and sized, before it has a line.
#[derive(Debug, Clone, Copy)]
struct Piece {
    slot: u16,
    id: GlyphId,
    advance: f32,
    offset: [f32; 2],
    cluster: u32,
    space: bool,
}

/// A line being filled.
#[derive(Debug, Default)]
struct Filling {
    glyphs: Vec<Glyph>,
    /// Each letter's advance, to take letters back off for "…".
    advances: Vec<f32>,
    /// Whether each is a space.
    spaces: Vec<bool>,
    pen: f32,
    /// How far its letters reach, spaces at its end left out.
    ink: f32,
}

impl Filling {
    fn push(&mut self, p: &Piece) {
        self.glyphs.push(Glyph {
            slot: p.slot,
            id: p.id,
            at: [self.pen + p.offset[0], -p.offset[1]],
            cluster: p.cluster,
        });
        self.advances.push(p.advance);
        self.spaces.push(p.space);
        self.pen += p.advance;
        if !p.space {
            self.ink = self.pen;
        }
    }

    /// Takes the last letter, and any others of its cluster, back off.
    fn pop_cluster(&mut self) {
        let Some(last) = self.glyphs.last().map(|g| g.cluster) else {
            return;
        };
        while self.glyphs.last().is_some_and(|g| g.cluster == last) {
            self.glyphs.pop();
            self.spaces.pop();
            self.pen -= self.advances.pop().unwrap_or(0.0);
        }
        let ink_end = self.spaces.iter().rposition(|s| !s);
        self.ink = ink_end.map_or(0.0, |i| self.advances[..=i].iter().sum::<f32>());
    }

    fn is_empty(&self) -> bool {
        self.glyphs.is_empty()
    }
}

/// Lays paragraphs out with a set of fonts.
pub(crate) struct Builder<'a> {
    pub(crate) slots: &'a mut [Slot],
    /// The fonts to try, in order (places in `slots`); never empty.
    pub(crate) stack: &'a [usize],
    pub(crate) style: &'a TextStyle,
    pub(crate) fit: Fit,
    pub(crate) max_letters: usize,
    /// The game's own font and its bold (places in `slots`), for bold
    /// parts of the text.
    pub(crate) bold: Option<(usize, usize)>,
    /// Icons met so far.
    pub(crate) icons: usize,
}

/// Characters that belong with the letter before them, never choosing a
/// font of their own: joiners, variation selectors and combining marks.
fn follows(c: char) -> bool {
    matches!(c as u32,
        0x0300..=0x036F | 0x1AB0..=0x1AFF | 0x1DC0..=0x1DFF | 0x20D0..=0x20FF
        | 0xFE00..=0xFE0F | 0xFE20..=0xFE2F | 0x200C | 0x200D | 0xE0100..=0xE01EF)
}

/// The characters a line may end with that start a new one.
const LINE_ENDS: &[char] = &[
    '\n', '\r', '\u{0B}', '\u{0C}', '\u{85}', '\u{2028}', '\u{2029}',
];

/// The line being filled: the last one.
fn filling(lines: &mut Vec<Filling>) -> &mut Filling {
    if lines.is_empty() {
        lines.push(Filling::default());
    }
    let last = lines.len() - 1;
    &mut lines[last]
}

/// An icon's width over its height, kept sensible.
fn icon_width(ratio: f32) -> f32 {
    if ratio.is_finite() {
        ratio.clamp(0.1, 10.0)
    } else {
        1.0
    }
}

fn finite_or(v: f32, or: f32) -> f32 {
    if v.is_finite() { v } else { or }
}

impl Builder<'_> {
    /// `text` laid out.
    pub(crate) fn build(mut self, text: &str) -> Paragraph {
        let size = finite_or(self.style.size, 0.0).clamp(0.0, 4096.0);
        let primary = self.stack[0];
        let sized = self.slots[primary].font.at(size);
        let natural = sized.line();
        let height = finite_or(self.style.line_height, 1.0);
        let height = if height > 0.0 {
            height.clamp(0.25, 8.0)
        } else {
            1.0
        };
        let advance = natural * height;
        let first = sized.ascent() + (advance - natural) * 0.5;
        let spacing = finite_or(self.style.letter_spacing, 0.0).clamp(-size, size * 4.0);
        let width = self.fit.width.filter(|w| w.is_finite()).map(|w| w.max(0.0));
        let wrap = self.fit.wrap && width.is_some();
        let limit = width.map_or(f32::INFINITY, |w| w + EASE);

        let mut lines = vec![Filling::default()];
        let mut pieces = Vec::new();
        let mut letters = 0;
        let mut start = 0;
        for (end, chance) in unicode_linebreak::linebreaks(text) {
            if letters >= self.max_letters {
                break;
            }
            let segment = &text[start..end];
            let body = segment.trim_end_matches(LINE_ENDS);
            pieces.clear();
            self.shape(body, start, size, spacing, &mut pieces);
            start = end;
            pieces.truncate(self.max_letters - letters);
            letters += pieces.len();
            let ink = pieces
                .iter()
                .rposition(|p| !p.space)
                .map_or(0.0, |i| pieces[..=i].iter().map(|p| p.advance).sum());
            if wrap && !filling(&mut lines).is_empty() && filling(&mut lines).pen + ink > limit {
                lines.push(Filling::default());
            }
            if wrap && ink > limit {
                // A word too long for a line on its own breaks between
                // its letters.
                let mut last_cluster = None;
                for p in &pieces {
                    let line = filling(&mut lines);
                    let new_cluster = last_cluster != Some(p.cluster);
                    if !p.space && new_cluster && !line.is_empty() && line.pen + p.advance > limit {
                        lines.push(Filling::default());
                    }
                    filling(&mut lines).push(p);
                    last_cluster = Some(p.cluster);
                }
            } else {
                let line = filling(&mut lines);
                for p in &pieces {
                    line.push(p);
                }
            }
            let hard = chance == BreakOpportunity::Mandatory && body.len() < segment.len();
            if hard {
                lines.push(Filling::default());
            }
        }
        let mut cut = false;
        if self.fit.cut {
            if let Some(most) = self.fit.lines.map(|n| n.max(1) as usize)
                && lines.len() > most
            {
                lines.truncate(most);
                if let Some(last) = lines.last_mut() {
                    self.ellipsis(last, limit, size, spacing);
                }
                cut = true;
            }
            if !wrap && width.is_some() {
                for line in &mut lines {
                    if line.ink > limit {
                        self.ellipsis(line, limit, size, spacing);
                        cut = true;
                    }
                }
            }
        }
        let mut paragraph = Paragraph {
            text_size: size,
            cut,
            icons: self
                .style
                .inline
                .icons
                .iter()
                .map(|&w| icon_width(w) * size)
                .collect(),
            ..Paragraph::default()
        };
        let mut widest = 0.0_f32;
        for (i, line) in lines.into_iter().enumerate() {
            let from = paragraph.glyphs.len() as u32;
            paragraph.glyphs.extend(line.glyphs);
            widest = widest.max(line.ink);
            paragraph.lines.push(Line {
                width: line.ink,
                baseline: first + advance * i as f32,
                glyphs: [from, paragraph.glyphs.len() as u32],
            });
        }
        paragraph.size = [widest, advance * paragraph.lines.len() as f32];
        paragraph
    }

    /// The font for `c`: the first in the stack that has it.
    fn font_for(&mut self, c: char) -> usize {
        let stack = self.stack;
        stack
            .iter()
            .copied()
            .find(|&s| self.slots[s].font.has(c))
            .unwrap_or(stack[0])
    }

    /// The font for `c` at byte `byte`: the first that has it, from the
    /// game's bold where the game's font is chosen in a bold part.
    fn font_at(&mut self, c: char, byte: usize) -> usize {
        let font = self.font_for(c);
        match self.bold {
            Some((regular, bold))
                if font == regular
                    && self.style.inline.bold_at(byte)
                    && self.slots[bold].font.has(c) =>
            {
                bold
            }
            _ => font,
        }
    }

    /// An icon's box at the pen: as tall as the text, its top at the
    /// font's ascent.
    fn icon(&mut self, byte: usize, size: f32, spacing: f32, out: &mut Vec<Piece>) {
        let number = self.icons;
        self.icons += 1;
        let ratio = self.style.inline.icons.get(number).copied().unwrap_or(1.0);
        let ascent = self.slots[self.stack[0]].font.at(size).ascent();
        out.push(Piece {
            slot: ICON_SLOT,
            id: GlyphId::new(u32::try_from(number).unwrap_or(u32::MAX)),
            advance: icon_width(ratio) * size + spacing,
            offset: [0.0, ascent],
            cluster: u32::try_from(byte).unwrap_or(u32::MAX),
            space: false,
        });
    }

    /// `body` (from byte `base` of the text) as letters: in runs of one
    /// font each, shaped, at `size` with `spacing` after each letter.
    fn shape(&mut self, body: &str, base: usize, size: f32, spacing: f32, out: &mut Vec<Piece>) {
        let mut run: Option<(usize, usize)> = None;
        for (i, c) in body.char_indices() {
            if c == crate::markup::ICON {
                if let Some((from, font)) = run.take() {
                    self.shape_run(&body[from..i], base + from, font, size, spacing, out);
                }
                self.icon(base + i, size, spacing, out);
                continue;
            }
            let keeps = c.is_whitespace() || c.is_control() || follows(c);
            let font = match run {
                Some((_, font)) if keeps => font,
                _ => self.font_at(c, base + i),
            };
            match run {
                Some((from, was)) if was != font => {
                    self.shape_run(&body[from..i], base + from, was, size, spacing, out);
                    run = Some((i, font));
                }
                None => run = Some((i, font)),
                Some(_) => {}
            }
        }
        if let Some((from, font)) = run {
            self.shape_run(&body[from..], base + from, font, size, spacing, out);
        }
    }

    fn shape_run(
        &mut self,
        run: &str,
        base: usize,
        slot: usize,
        size: f32,
        spacing: f32,
        out: &mut Vec<Piece>,
    ) {
        let s = &mut self.slots[slot];
        let k = size / s.font.height();
        let space_at = |cluster: usize| {
            run.get(cluster..)
                .and_then(|rest| rest.chars().next())
                .is_some_and(char::is_whitespace)
        };
        let slot16 = u16::try_from(slot).unwrap_or(u16::MAX);
        if let Some(shaped) = s.shape(run) {
            out.extend(shaped.iter().map(|g| Piece {
                slot: slot16,
                id: g.id,
                advance: g.advance * k + spacing,
                offset: [g.offset[0] * k, g.offset[1] * k],
                cluster: (base + g.cluster as usize) as u32,
                space: space_at(g.cluster as usize),
            }));
            return;
        }
        // A font that can't be shaped: a letter for each character.
        for (i, c) in run.char_indices() {
            let (id, units) = s.font.letter(c);
            out.push(Piece {
                slot: slot16,
                id,
                advance: units * k + spacing,
                offset: [0.0, 0.0],
                cluster: (base + i) as u32,
                space: c.is_whitespace(),
            });
        }
    }

    /// `line` cut short enough for "…" to follow within `limit`, and the
    /// "…" put after it ("..." in a font without it).
    fn ellipsis(&mut self, line: &mut Filling, limit: f32, size: f32, spacing: f32) {
        let cluster = line.glyphs.last().map_or(0, |g| g.cluster);
        let mut dots = Vec::new();
        let ellipsis = '\u{2026}';
        let font = self.font_for(ellipsis);
        let words = if self.slots[font].font.has(ellipsis) {
            "\u{2026}"
        } else {
            "..."
        };
        self.shape_run(words, 0, font, size, spacing, &mut dots);
        let wide: f32 = dots.iter().map(|p| p.advance).sum();
        while !line.is_empty() && (line.spaces.last() == Some(&true) || line.ink + wide > limit) {
            line.pop_cluster();
        }
        line.pen = line.ink;
        for mut dot in dots {
            dot.cluster = cluster;
            line.push(&dot);
        }
    }
}

#[cfg(test)]
#[path = "paragraph_tests.rs"]
mod tests;
