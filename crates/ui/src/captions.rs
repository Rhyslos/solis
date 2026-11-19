//! The ready-made caption screen (stage 1, translations): words shown low
//! in the middle of the game, each line on a box as dark as the player
//! chose, as big as they chose. Sounds heard show their caption while the
//! player has captions on; a voice line shows its subtitle, in step with
//! the voice, while they have subtitles on. Solis draws the screen itself,
//! over every game screen.
//!
//! A line is shown for a given time, or until told to stop ([`Captions::stop`]),
//! so a dialogue's line can be kept up exactly as long as its voice plays.
//! Captions are what the player sees, never the game's state: they go by
//! the UI's clock.

use crate::screen::{ScaleRule, Screen};
use crate::view::{Node, Style};

/// The most lines shown at once: the newest, oldest at the top.
pub const MAX_SHOWN: usize = 3;
/// The most lines kept waiting (shown or about to be): past it the oldest
/// go.
pub const MAX_KEPT: usize = 32;
/// The longest a line is kept, in characters: the rest is cut.
pub const MAX_CHARS: usize = 500;
/// The text's size on a 1080-pixel-tall screen, at caption size 1.
pub const TEXT_SIZE: f32 = 34.0;

/// What a line is: a sound's caption, or what a character says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A sound heard (a door creaking): shown while captions are on.
    Sound,
    /// A voice: shown while subtitles are on.
    Subtitle,
}

/// Which line: what [`Captions::show`] gives back, for [`Captions::stop`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LineId(u64);

/// One line shown.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    /// Which line.
    pub id: LineId,
    /// Who says it (empty: nobody).
    pub speaker: String,
    /// The words.
    pub text: String,
    /// Sound or voice.
    pub kind: Kind,
    /// Until when, on the UI's clock, in seconds (none: until stopped).
    pub until: Option<f64>,
}

/// How the player wants captions: which kinds, how big, how dark a box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Look {
    /// Whether voices' subtitles show.
    pub subtitles: bool,
    /// Whether sounds' captions show.
    pub captions: bool,
    /// Their size, times as designed (0.5 to 2).
    pub size: f32,
    /// The box's darkness: 0 none, 1 solid.
    pub background: f32,
}

impl Default for Look {
    fn default() -> Self {
        Self {
            subtitles: true,
            captions: false,
            size: 1.0,
            background: 0.6,
        }
    }
}

impl Look {
    /// Whether lines of `kind` are shown.
    pub fn shows(&self, kind: Kind) -> bool {
        match kind {
            Kind::Sound => self.captions,
            Kind::Subtitle => self.subtitles,
        }
    }
}

/// The lines to show.
#[derive(Debug, Clone, Default)]
pub struct Captions {
    lines: Vec<Line>,
    next: u64,
}

impl Captions {
    /// Shows `text` (said by `speaker`, empty for nobody) from `now` for
    /// `seconds`, or until [`Captions::stop`] when `None`. The same words
    /// already showing are kept up for longer instead of shown twice.
    pub fn show(
        &mut self,
        kind: Kind,
        speaker: &str,
        text: &str,
        now: f64,
        seconds: Option<f64>,
    ) -> LineId {
        let text: String = text.trim().chars().take(MAX_CHARS).collect();
        let speaker: String = speaker.trim().chars().take(100).collect();
        let until = seconds.map(|s| now + s.max(0.0));
        if let Some(line) = self
            .lines
            .iter_mut()
            .find(|l| l.kind == kind && l.speaker == speaker && l.text == text)
        {
            line.until = match (line.until, until) {
                (Some(a), Some(b)) => Some(a.max(b)),
                _ => None,
            };
            return line.id;
        }
        let id = LineId(self.next);
        self.next += 1;
        self.lines.push(Line {
            id,
            speaker,
            text,
            kind,
            until,
        });
        if self.lines.len() > MAX_KEPT {
            self.lines.remove(0);
        }
        id
    }

    /// Stops showing the line `id` (a voice that ended).
    pub fn stop(&mut self, id: LineId) {
        self.lines.retain(|l| l.id != id);
    }

    /// Stops showing every line (play stopped, a game loaded).
    pub fn clear(&mut self) {
        self.lines.clear();
    }

    /// Lets go of the lines whose time is up at `now`.
    pub fn tick(&mut self, now: f64) {
        self.lines
            .retain(|l| l.until.is_none_or(|until| until > now));
    }

    /// The lines kept, oldest first.
    pub fn lines(&self) -> &[Line] {
        &self.lines
    }

    /// The lines `look` shows, the newest [`MAX_SHOWN`], oldest first.
    pub fn shown(&self, look: &Look) -> Vec<&Line> {
        let shown: Vec<&Line> = self.lines.iter().filter(|l| look.shows(l.kind)).collect();
        let skip = shown.len().saturating_sub(MAX_SHOWN);
        shown.into_iter().skip(skip).collect()
    }

    /// The caption screen for `look`, or none when nothing shows: a
    /// 1920 × 1080 design that grows with the screen, its lines stacked
    /// low in the middle.
    pub fn screen(&self, look: &Look) -> Option<Screen> {
        let shown = self.shown(look);
        if shown.is_empty() {
            return None;
        }
        let size = f64::from(TEXT_SIZE * look.size.clamp(0.5, 2.0));
        let lines = shown
            .iter()
            .map(|l| line_node(l, size, look.background))
            .collect();
        let mut screen = Screen::new("Captions", [1920.0, 1080.0], ScaleRule::Grow);
        // The view's top is the screen itself; the lines' column fills it.
        let column = node(
            "column",
            "",
            vec![
                ("width", Style::Text("fill".into())),
                ("height", Style::Text("fill".into())),
                ("bottom", Style::Flag(true)),
                ("center", Style::Flag(true)),
                ("padBottom", Style::Number(72.0)),
                ("gap", Style::Number(10.0)),
            ],
            lines,
        );
        screen.view = Some(node("view", "", Vec::new(), vec![column]));
        Some(screen)
    }
}

/// A piece of a view.
fn node(kind: &str, text: &str, styles: Vec<(&str, Style)>, children: Vec<Node>) -> Node {
    Node {
        kind: kind.into(),
        text: text.into(),
        value: None,
        styles: styles.into_iter().map(|(k, v)| (k.to_owned(), v)).collect(),
        children,
        action: None,
    }
}

/// One line: its box behind (as dark as asked), the speaker's name in
/// gold, then the words in white.
fn line_node(line: &Line, size: f64, background: f32) -> Node {
    let mut words = Vec::new();
    if !line.speaker.is_empty() {
        words.push(node(
            "text",
            &format!("{}:", line.speaker),
            vec![
                ("size", Style::Number(size)),
                ("bold", Style::Flag(true)),
                ("color", Style::Text("#ffd43b".into())),
                ("nowrap", Style::Flag(true)),
            ],
            Vec::new(),
        ));
    }
    words.push(node(
        "text",
        &line.text,
        vec![
            ("size", Style::Number(size)),
            ("color", Style::Text("white".into())),
            ("maxWidth", Style::Number(1300.0)),
        ],
        Vec::new(),
    ));
    let row = node(
        "row",
        "",
        vec![
            ("gap", Style::Number(size * 0.3)),
            ("padX", Style::Number(size * 0.6)),
            ("padY", Style::Number(size * 0.25)),
        ],
        words,
    );
    let shade = node(
        "column",
        "",
        vec![
            ("width", Style::Text("fill".into())),
            ("height", Style::Text("fill".into())),
            ("background", Style::Text("black".into())),
            (
                "opacity",
                Style::Number(f64::from(background.clamp(0.0, 1.0))),
            ),
            ("round", Style::Number(size * 0.2)),
        ],
        Vec::new(),
    );
    node("stack", "", Vec::new(), vec![shade, row])
}

#[cfg(test)]
#[path = "captions_tests.rs"]
mod tests;
