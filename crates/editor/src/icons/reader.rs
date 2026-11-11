//! The reader of an icon's path text: the small language the design's
//! drawings are written in (`M3 2.5h8l2.5 2.5v8.5h-10.5z`), turned into
//! runs of straight lines on the icons' 16 × 16 grid.
//!
//! It reads what the icon table uses and no more: move, line, across,
//! down, curve, smooth curve, arc and close, each with a capital letter
//! (a place on the grid) or a small one (a step from the last place).
//! Curves and arcs become short straight pieces, fine enough for an icon
//! drawn at up to 48 px.
//!
//! The table is Solis's own, so nothing a user owns is read here; all the
//! same, text that isn't a path is an error with its place, never a
//! crash, and a test feeds it damaged paths to keep it so.

use std::f32::consts::{PI, TAU};

/// One run of straight lines: a stroke, or the edge of a filled shape.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Outline {
    /// Its corners, on the 16 × 16 grid.
    pub(crate) points: Vec<[f32; 2]>,
    /// Whether the last corner joins the first.
    pub(crate) closed: bool,
}

/// Why a path couldn't be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PathError {
    /// How many bytes into the text the trouble is.
    pub(crate) at: usize,
    /// What was wrong.
    pub(crate) what: &'static str,
}

impl std::fmt::Display for PathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (at letter {})", self.what, self.at + 1)
    }
}

/// The most corners one path may have: a hundred times the busiest icon.
const MOST_POINTS: usize = 4096;
/// The straight pieces a curve becomes.
const CURVE_PIECES: usize = 12;
/// The widest turn one straight piece of an arc covers.
const ARC_STEP: f32 = PI / 12.0;

/// The text being read, and how far the reading has got.
struct Text<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Text<'_> {
    fn error(&self, what: &'static str) -> PathError {
        PathError { at: self.at, what }
    }

    /// Steps over spaces and commas.
    fn skip_gaps(&mut self) {
        while self
            .bytes
            .get(self.at)
            .is_some_and(|b| b.is_ascii_whitespace() || *b == b',')
        {
            self.at += 1;
        }
    }

    /// The next letter, if a letter is next.
    fn command(&mut self) -> Option<u8> {
        self.skip_gaps();
        let letter = *self.bytes.get(self.at)?;
        letter.is_ascii_alphabetic().then(|| {
            self.at += 1;
            letter
        })
    }

    /// Whether a number is next (a command's numbers may repeat).
    fn number_follows(&mut self) -> bool {
        self.skip_gaps();
        self.bytes
            .get(self.at)
            .is_some_and(|b| b.is_ascii_digit() || matches!(b, b'-' | b'+' | b'.'))
    }

    /// The next number: a sign, digits, and digits after a point. Numbers
    /// may touch (`1.5.5` is 1.5 and 0.5; `2-3` is 2 and -3).
    fn number(&mut self) -> Result<f32, PathError> {
        self.skip_gaps();
        let start = self.at;
        if matches!(self.bytes.get(self.at), Some(b'-' | b'+')) {
            self.at += 1;
        }
        let digits_from = self.at;
        while self.bytes.get(self.at).is_some_and(u8::is_ascii_digit) {
            self.at += 1;
        }
        if self.bytes.get(self.at) == Some(&b'.') {
            self.at += 1;
            while self.bytes.get(self.at).is_some_and(u8::is_ascii_digit) {
                self.at += 1;
            }
        }
        let written = &self.bytes[start..self.at];
        let has_digit = self.bytes[digits_from..self.at]
            .iter()
            .any(u8::is_ascii_digit);
        let value = std::str::from_utf8(written)
            .ok()
            .filter(|_| has_digit)
            .and_then(|text| text.parse::<f32>().ok())
            .filter(|value| value.is_finite());
        value.ok_or(PathError {
            at: start,
            what: "a number was expected",
        })
    }

    /// An arc's flag: a lone 0 or 1, which may touch what follows it.
    fn flag(&mut self) -> Result<bool, PathError> {
        self.skip_gaps();
        match self.bytes.get(self.at) {
            Some(b'0') => {
                self.at += 1;
                Ok(false)
            }
            Some(b'1') => {
                self.at += 1;
                Ok(true)
            }
            _ => Err(self.error("an arc's flag must be 0 or 1")),
        }
    }

    fn pair(&mut self) -> Result<[f32; 2], PathError> {
        Ok([self.number()?, self.number()?])
    }
}

/// The outlines being built, and the pen.
#[derive(Default)]
struct Pen {
    done: Vec<Outline>,
    points: Vec<[f32; 2]>,
    at: [f32; 2],
    /// Where the outline being drawn began.
    start: [f32; 2],
    /// The last curve's second handle, for a smooth curve to mirror.
    handle: Option<[f32; 2]>,
    count: usize,
}

impl Pen {
    /// Ends the outline being drawn, if it has a line in it.
    fn finish(&mut self, closed: bool) {
        let points = std::mem::take(&mut self.points);
        if points.len() > 1 {
            self.done.push(Outline { points, closed });
        }
    }

    fn move_to(&mut self, to: [f32; 2]) {
        self.finish(false);
        self.at = to;
        self.start = to;
    }

    fn line_to(&mut self, to: [f32; 2]) -> Result<(), &'static str> {
        if self.points.is_empty() {
            self.points.push(self.at);
            self.count += 1;
        }
        self.points.push(to);
        self.count += 1;
        self.at = to;
        if self.count > MOST_POINTS {
            return Err("the path has too many corners");
        }
        Ok(())
    }

    fn close(&mut self) {
        self.finish(true);
        self.at = self.start;
    }

    /// A curve from the pen through two handles to `to`.
    fn curve_to(&mut self, a: [f32; 2], b: [f32; 2], to: [f32; 2]) -> Result<(), &'static str> {
        let from = self.at;
        for piece in 1..=CURVE_PIECES {
            let t = piece as f32 / CURVE_PIECES as f32;
            let u = 1.0 - t;
            let along = |i: usize| {
                u * u * u * from[i]
                    + 3.0 * u * u * t * a[i]
                    + 3.0 * u * t * t * b[i]
                    + t * t * t * to[i]
            };
            self.line_to([along(0), along(1)])?;
        }
        Ok(())
    }

    /// Part of a circle or an ellipse from the pen to `to`, as the path
    /// language gives it: the two radii, and which of the four possible
    /// arcs is meant (the long way round or the short; turning clockwise
    /// or not). Turned ellipses aren't read: no icon has one.
    fn arc_to(
        &mut self,
        radii: [f32; 2],
        long: bool,
        clockwise: bool,
        to: [f32; 2],
    ) -> Result<(), &'static str> {
        let from = self.at;
        let (mut rx, mut ry) = (radii[0].abs(), radii[1].abs());
        if from == to {
            return Ok(());
        }
        if rx < 1e-6 || ry < 1e-6 {
            return self.line_to(to);
        }
        // Half the way between the ends, measured in radii.
        let half = [(from[0] - to[0]) / 2.0, (from[1] - to[1]) / 2.0];
        let reach = (half[0] / rx).powi(2) + (half[1] / ry).powi(2);
        if reach > 1.0 {
            // Too small to span the ends: grown until it just does.
            let grow = reach.sqrt();
            rx *= grow;
            ry *= grow;
        }
        let above = (rx * ry).powi(2) - (rx * half[1]).powi(2) - (ry * half[0]).powi(2);
        let below = (rx * half[1]).powi(2) + (ry * half[0]).powi(2);
        let side = if long == clockwise { -1.0 } else { 1.0 };
        let k = side * (above.max(0.0) / below).sqrt();
        let offset = [k * rx * half[1] / ry, -k * ry * half[0] / rx];
        let centre = [
            offset[0] + (from[0] + to[0]) / 2.0,
            offset[1] + (from[1] + to[1]) / 2.0,
        ];
        let angle = |p: [f32; 2]| ((p[1] - centre[1]) / ry).atan2((p[0] - centre[0]) / rx);
        let first = angle(from);
        let mut sweep = angle(to) - first;
        if clockwise && sweep < 0.0 {
            sweep += TAU;
        } else if !clockwise && sweep > 0.0 {
            sweep -= TAU;
        }
        if !sweep.is_finite() || !centre[0].is_finite() || !centre[1].is_finite() {
            return self.line_to(to);
        }
        let pieces = ((sweep.abs() / ARC_STEP).ceil() as usize).clamp(1, 48);
        for piece in 1..pieces {
            let turn = first + sweep * piece as f32 / pieces as f32;
            self.line_to([centre[0] + rx * turn.cos(), centre[1] + ry * turn.sin()])?;
        }
        self.line_to(to)
    }
}

/// Reads one path into its outlines. An outline of a single point (a
/// move with no line after it) draws nothing and is left out.
pub(crate) fn read(path: &str) -> Result<Vec<Outline>, PathError> {
    let mut text = Text {
        bytes: path.as_bytes(),
        at: 0,
    };
    let mut pen = Pen::default();
    let mut started = false;
    loop {
        text.skip_gaps();
        if text.at >= text.bytes.len() {
            break;
        }
        let at = text.at;
        let Some(letter) = text.command() else {
            return Err(text.error("a command's letter was expected"));
        };
        if !started && !matches!(letter, b'M' | b'm') {
            return Err(PathError {
                at,
                what: "a path starts with a move",
            });
        }
        started = true;
        one_command(&mut text, &mut pen, letter).map_err(|error| match error {
            Trouble::Text(error) => error,
            Trouble::Pen(what) => PathError { at, what },
        })?;
    }
    pen.finish(false);
    Ok(pen.done)
}

/// What can go wrong inside one command: its numbers, or what it draws.
enum Trouble {
    Text(PathError),
    Pen(&'static str),
}

impl From<PathError> for Trouble {
    fn from(error: PathError) -> Self {
        Self::Text(error)
    }
}

impl From<&'static str> for Trouble {
    fn from(what: &'static str) -> Self {
        Self::Pen(what)
    }
}

/// Carries out the command `letter`, as many times as it has numbers.
fn one_command(text: &mut Text<'_>, pen: &mut Pen, letter: u8) -> Result<(), Trouble> {
    let step = letter.is_ascii_lowercase();
    // A place as written: from the pen for a small letter.
    let placed = |pen: &Pen, p: [f32; 2]| {
        if step {
            [pen.at[0] + p[0], pen.at[1] + p[1]]
        } else {
            p
        }
    };
    let mut first = true;
    loop {
        let mut handle = None;
        match letter.to_ascii_uppercase() {
            b'M' => {
                let to = placed(pen, text.pair()?);
                // Pairs after the first are lines.
                if first {
                    pen.move_to(to);
                } else {
                    pen.line_to(to)?;
                }
            }
            b'L' => {
                let to = placed(pen, text.pair()?);
                pen.line_to(to)?;
            }
            b'H' => {
                let x = text.number()?;
                let x = if step { pen.at[0] + x } else { x };
                pen.line_to([x, pen.at[1]])?;
            }
            b'V' => {
                let y = text.number()?;
                let y = if step { pen.at[1] + y } else { y };
                pen.line_to([pen.at[0], y])?;
            }
            b'C' => {
                let (a, b, to) = (text.pair()?, text.pair()?, text.pair()?);
                let (a, b, to) = (placed(pen, a), placed(pen, b), placed(pen, to));
                pen.curve_to(a, b, to)?;
                handle = Some(b);
            }
            b'S' => {
                let (b, to) = (text.pair()?, text.pair()?);
                let (b, to) = (placed(pen, b), placed(pen, to));
                // The first handle mirrors the last curve's second.
                let a = pen
                    .handle
                    .map_or(pen.at, |h| [2.0 * pen.at[0] - h[0], 2.0 * pen.at[1] - h[1]]);
                pen.curve_to(a, b, to)?;
                handle = Some(b);
            }
            b'A' => {
                let radii = text.pair()?;
                let turned = text.number()?;
                let (long, clockwise) = (text.flag()?, text.flag()?);
                let to = placed(pen, text.pair()?);
                if turned != 0.0 {
                    return Err(Trouble::Pen("a turned ellipse isn't read"));
                }
                pen.arc_to(radii, long, clockwise, to)?;
            }
            b'Z' => {
                pen.close();
                pen.handle = None;
                return Ok(());
            }
            _ => return Err(Trouble::Pen("a command the reader doesn't know")),
        }
        pen.handle = handle;
        first = false;
        if !text.number_follows() {
            return Ok(());
        }
    }
}

#[cfg(test)]
#[path = "reader_tests.rs"]
mod tests;
