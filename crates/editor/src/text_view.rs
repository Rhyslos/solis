//! Text files in Properties: notes (`.md`, `.txt`) and data (`.json`,
//! `.csv`, `.toml`, `.ron`) picked in the Library, shown as they read,
//! read only, with a button that opens them in the computer's program for
//! them (Ledger for Markdown). Markdown's headings, lists, quotes, code
//! and emphasis are drawn as such; everything else as plain text.
//!
//! The app reads the file (`Workbench::text_file_wanted`,
//! `Workbench::set_text_file`) when it's picked and again when it changes
//! on disk, at most [`MAX_TEXT_BYTES`].

use crate::library::{LibraryKind, LibraryRequest};
use crate::theme;
use crate::workbench::Workbench;
use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontId, RichText, Ui};

/// The most of a text file Properties shows.
pub const MAX_TEXT_BYTES: u64 = 512 * 1024;
/// The most lines Properties draws, so a long file stays quick.
pub const MAX_LINES: usize = 3_000;

/// A text file as the app read it: its text, or why it couldn't be.
pub type TextRead = Result<String, String>;

/// A text file as Properties shows it, worked out once when it's read,
/// or why it couldn't be read.
pub type TextShown = Result<ShownText, String>;

/// A text file made ready to show, once, when the app reads it (the
/// review, BUG-27: a note was cut to its first lines, read into blocks
/// and its marks into runs again every frame it was shown).
#[derive(Debug, Clone, PartialEq)]
pub struct ShownText {
    /// How many lines it has.
    lines: usize,
    /// Whether it holds nothing but spaces.
    empty: bool,
    /// Its first [`MAX_LINES`] lines, as they're drawn.
    body: Body,
    /// Whether it has more lines than that.
    more: bool,
}

/// What of a text file is drawn.
#[derive(Debug, Clone, PartialEq)]
enum Body {
    /// Markdown's blocks, each with its words as runs (none for code and
    /// lines across).
    Markdown(Vec<(Block, Vec<(Style, String)>)>),
    /// Text as it is.
    Plain(String),
}

impl ShownText {
    /// `text`, of the file at `path`: Markdown when its name ends `.md`.
    pub fn of(path: &str, text: &str) -> Self {
        let (shown, more) = first_lines(text);
        let body = if path.to_lowercase().ends_with(".md") {
            Body::Markdown(
                markdown_blocks(&shown)
                    .into_iter()
                    .map(|block| {
                        let runs = match &block {
                            Block::Heading(_, words)
                            | Block::Paragraph(words)
                            | Block::Item(_, _, words)
                            | Block::Quote(words) => inline_runs(words),
                            Block::Code(_) | Block::Rule => Vec::new(),
                        };
                        (block, runs)
                    })
                    .collect(),
            )
        } else {
            Body::Plain(shown)
        };
        Self {
            lines: text.lines().count(),
            empty: text.trim().is_empty(),
            body,
            more,
        }
    }
}

impl Workbench {
    /// The text file Properties shows, by path inside the assets, for the
    /// app to read (again whenever it changes on disk).
    pub fn text_file_wanted(&self) -> Option<&str> {
        let (path, _) = self.open.sound_shown.as_ref()?;
        self.open
            .library
            .iter()
            .any(|i| &i.path == path && i.kind == LibraryKind::Text)
            .then_some(path.as_str())
    }

    /// The text of the file at `path` (inside the assets), as the app read
    /// it: made ready to show here, once.
    pub fn set_text_file(&mut self, path: String, read: TextRead) {
        let shown = read.map(|text| ShownText::of(&path, &text));
        self.open.text_file = Some((path, shown));
    }
}

/// One block of a Markdown file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    /// A heading, its level (1 to 6) and words.
    Heading(u8, String),
    /// A paragraph's lines, joined with spaces.
    Paragraph(String),
    /// A list item: how deep (0 at the edge), its marker ("•" or "3.")
    /// and words.
    Item(u8, String, String),
    /// A quote's words.
    Quote(String),
    /// Code, as written.
    Code(String),
    /// A line across.
    Rule,
}

/// Markdown `text` as blocks: never fails, whatever it holds.
pub fn markdown_blocks(text: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut paragraph: Vec<&str> = Vec::new();
    let mut code: Option<Vec<&str>> = None;
    let flush = |paragraph: &mut Vec<&str>, blocks: &mut Vec<Block>| {
        if !paragraph.is_empty() {
            blocks.push(Block::Paragraph(paragraph.join(" ")));
            paragraph.clear();
        }
    };
    for line in text.lines().take(MAX_LINES) {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            match code.take() {
                Some(lines) => blocks.push(Block::Code(lines.join("\n"))),
                None => {
                    flush(&mut paragraph, &mut blocks);
                    code = Some(Vec::new());
                }
            }
            continue;
        }
        if let Some(lines) = code.as_mut() {
            lines.push(line);
            continue;
        }
        if trimmed.is_empty() {
            flush(&mut paragraph, &mut blocks);
            continue;
        }
        let hashes = trimmed.chars().take_while(|c| *c == '#').count();
        if (1..=6).contains(&hashes) && trimmed[hashes..].starts_with(' ') {
            flush(&mut paragraph, &mut blocks);
            let words = trimmed[hashes..].trim().trim_end_matches('#').trim();
            blocks.push(Block::Heading(hashes as u8, words.to_owned()));
            continue;
        }
        let compact: String = trimmed.chars().filter(|c| !c.is_whitespace()).collect();
        if compact.len() >= 3
            && (compact.chars().all(|c| c == '-')
                || compact.chars().all(|c| c == '*')
                || compact.chars().all(|c| c == '_'))
        {
            flush(&mut paragraph, &mut blocks);
            blocks.push(Block::Rule);
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix('>') {
            flush(&mut paragraph, &mut blocks);
            blocks.push(Block::Quote(rest.trim().to_owned()));
            continue;
        }
        let depth = ((line.len() - trimmed.len()) / 2).min(6) as u8;
        if let Some(rest) = ["- ", "* ", "+ "]
            .iter()
            .find_map(|m| trimmed.strip_prefix(m))
        {
            flush(&mut paragraph, &mut blocks);
            let rest = rest.trim();
            // A task list's boxes.
            let (marker, words) = if let Some(done) = rest.strip_prefix("[x] ") {
                ("☑", done)
            } else if let Some(open) = rest.strip_prefix("[ ] ") {
                ("☐", open)
            } else {
                ("•", rest)
            };
            blocks.push(Block::Item(depth, marker.to_owned(), words.to_owned()));
            continue;
        }
        let digits = trimmed.chars().take_while(char::is_ascii_digit).count();
        if (1..=9).contains(&digits) && trimmed[digits..].starts_with(". ") {
            flush(&mut paragraph, &mut blocks);
            blocks.push(Block::Item(
                depth,
                format!("{}.", &trimmed[..digits]),
                trimmed[digits + 2..].trim().to_owned(),
            ));
            continue;
        }
        paragraph.push(trimmed.trim_end());
    }
    flush(&mut paragraph, &mut blocks);
    if let Some(lines) = code {
        blocks.push(Block::Code(lines.join("\n")));
    }
    blocks
}

/// A run of words and how it's drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    /// As it is.
    Plain,
    /// `**strong**`.
    Strong,
    /// `*emphasis*`.
    Emphasis,
    /// `` `code` ``.
    Code,
    /// `[a link](address)`: its words.
    Link,
}

/// The inline marks: how each opens and closes, and what it makes.
const MARKS: [(&str, &str, Style); 6] = [
    ("`", "`", Style::Code),
    ("**", "**", Style::Strong),
    ("__", "__", Style::Strong),
    ("*", "*", Style::Emphasis),
    ("_", "_", Style::Emphasis),
    ("[", "](", Style::Link),
];

/// Where a pattern next occurs in a text, remembered: asked from places
/// that only move forward, it reads each part of the text once.
struct Ahead<'a> {
    text: &'a str,
    pattern: &'static str,
    /// Where it was last looked for from, and where it was found.
    known: Option<(usize, Option<usize>)>,
}

impl<'a> Ahead<'a> {
    fn new(text: &'a str, pattern: &'static str) -> Self {
        Self {
            text,
            pattern,
            known: None,
        }
    }

    /// Where the pattern first occurs at or after byte `from`.
    fn from(&mut self, from: usize) -> Option<usize> {
        if let Some((looked, found)) = self.known
            && from >= looked
            && found.is_none_or(|at| at >= from)
        {
            return found;
        }
        let found = self
            .text
            .get(from..)
            .and_then(|rest| rest.find(self.pattern))
            .map(|at| from + at);
        self.known = Some((from, found));
        found
    }
}

/// `text`'s inline Markdown (`**strong**`, `*emphasis*`, `` `code` ``,
/// `[link](address)`) as runs, the marks taken out. Marks that aren't
/// closed are kept as written.
///
/// One pass: where each mark next opens and closes is remembered, not
/// looked for again from each run (the review, BUG-27: a mark the text
/// never uses was looked for to the end after every run, so a long
/// paragraph took seconds).
pub fn inline_runs(text: &str) -> Vec<(Style, String)> {
    let mut runs: Vec<(Style, String)> = Vec::new();
    let mut push = |style: Style, words: &str| {
        if words.is_empty() {
            return;
        }
        match runs.last_mut() {
            Some((last, so_far)) if *last == style => so_far.push_str(words),
            _ => runs.push((style, words.to_owned())),
        }
    };
    let mut opens = MARKS.map(|(open, ..)| Ahead::new(text, open));
    let mut closes = MARKS.map(|(_, close, _)| Ahead::new(text, close));
    let mut addresses = Ahead::new(text, ")");
    let mut at = 0;
    while at < text.len() {
        // The mark opening first (the longer of two at one place) that is
        // closed with something inside.
        let mut found: Option<(usize, usize, usize)> = None;
        for (m, (open, ..)) in MARKS.iter().enumerate() {
            let Some(start) = opens[m].from(at) else {
                continue;
            };
            let inside = start + open.len();
            let Some(end) = closes[m].from(inside) else {
                continue;
            };
            let first = found.is_none_or(|(f, other, _)| {
                (start, std::cmp::Reverse(open.len()))
                    < (f, std::cmp::Reverse(MARKS[other].0.len()))
            });
            if end > inside && first {
                found = Some((start, m, end));
            }
        }
        let Some((start, m, end)) = found else {
            push(Style::Plain, &text[at..]);
            break;
        };
        let (open, close, style) = MARKS[m];
        push(Style::Plain, &text[at..start]);
        push(style, &text[start + open.len()..end]);
        at = end + close.len();
        if style == Style::Link {
            // The address, up to its ")", isn't shown.
            if let Some(paren) = addresses.from(at) {
                at = paren + 1;
            }
        }
    }
    runs
}

/// Draws words as their inline Markdown's `runs`, wrapped, at `size`.
fn inline(ui: &mut Ui, runs: &[(Style, String)], size: f32, colour: Color32) {
    let mut job = LayoutJob::default();
    for (style, text) in runs {
        let format = match style {
            Style::Plain => TextFormat::simple(FontId::proportional(size), colour),
            Style::Strong => TextFormat::simple(FontId::proportional(size), theme::text_strong()),
            Style::Emphasis => TextFormat {
                italics: true,
                ..TextFormat::simple(FontId::proportional(size), colour)
            },
            Style::Code => TextFormat {
                background: theme::field(),
                ..TextFormat::simple(FontId::monospace(size - 1.0), colour)
            },
            Style::Link => TextFormat {
                underline: egui::Stroke::new(1.0, crate::tokens::LINK),
                ..TextFormat::simple(FontId::proportional(size), crate::tokens::LINK)
            },
        };
        job.append(text, 0.0, format);
    }
    job.wrap.max_width = ui.available_width();
    ui.label(job);
}

/// Draws a Markdown file's blocks, each with its words' runs.
fn markdown(ui: &mut Ui, blocks: &[(Block, Vec<(Style, String)>)]) {
    for (block, words) in blocks {
        match block {
            Block::Heading(level, _) => {
                let size = match level {
                    1 => 20.0,
                    2 => 17.0,
                    3 => 15.0,
                    _ => 13.5,
                };
                ui.add_space(if *level <= 2 { 6.0 } else { 3.0 });
                inline(ui, words, size, theme::text_strong());
                ui.add_space(2.0);
            }
            Block::Paragraph(_) => {
                inline(ui, words, 13.0, theme::text());
                ui.add_space(6.0);
            }
            Block::Item(depth, marker, _) => {
                ui.horizontal_top(|ui| {
                    ui.add_space(4.0 + f32::from(*depth) * 14.0);
                    ui.label(RichText::new(marker).color(theme::text_weak()));
                    inline(ui, words, 13.0, theme::text());
                });
            }
            Block::Quote(_) => {
                let bar = egui::Frame::new()
                    .inner_margin(egui::Margin {
                        left: 10,
                        right: 0,
                        top: 2,
                        bottom: 2,
                    })
                    .show(ui, |ui| inline(ui, words, 13.0, theme::text_weak()))
                    .response
                    .rect;
                // A quote's bar down its left.
                ui.painter().vline(
                    bar.left() + 2.0,
                    bar.y_range(),
                    egui::Stroke::new(2.0, theme::text_weak().gamma_multiply(0.5)),
                );
            }
            Block::Code(code) => {
                plain_frame(ui, code);
                ui.add_space(6.0);
            }
            Block::Rule => {
                ui.separator();
            }
        }
    }
}

/// Text as it is, in a fixed-width face on a darker field.
fn plain_frame(ui: &mut Ui, text: &str) {
    egui::Frame::new()
        .fill(theme::field())
        .corner_radius(4.0)
        .inner_margin(8.0)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(
                RichText::new(text)
                    .font(FontId::monospace(12.0))
                    .color(theme::text()),
            );
        });
}

/// The first [`MAX_LINES`] lines of `text`, and whether there were more.
fn first_lines(text: &str) -> (String, bool) {
    let mut lines = text.lines();
    let shown: Vec<&str> = lines.by_ref().take(MAX_LINES).collect();
    (shown.join("\n"), lines.next().is_some())
}

/// The file at `path` in Properties: its name, kind and lines, a button
/// to open it in its program, and its text (`read`, while it's being
/// read `None`). Asks for what's pressed.
pub(crate) fn text_properties(
    ui: &mut Ui,
    path: &str,
    read: Option<&TextShown>,
) -> Vec<LibraryRequest> {
    let mut asked = Vec::new();
    let name = path.rsplit_once('/').map_or(path, |(_, n)| n);
    egui::Frame::new()
        .inner_margin(theme::panel_margin())
        .show(ui, |ui| {
            ui.label(RichText::new(name).strong().color(theme::text_strong()));
            let about = match read {
                Some(Ok(text)) => {
                    let lines = text.lines;
                    format!("Text · {lines} line{}", if lines == 1 { "" } else { "s" })
                }
                Some(Err(_)) => "Text".to_owned(),
                None => "Text · reading…".to_owned(),
            };
            ui.label(RichText::new(about).color(theme::text_weak()));
            ui.add_space(4.0);
            if ui
                .button("Open in its program ↗")
                .on_hover_text(
                    "Opens it in the program your computer uses for this kind of file \
                     (Ledger for Markdown), to edit it there",
                )
                .clicked()
            {
                asked.push(LibraryRequest::Open(LibraryKind::Text, path.to_owned()));
            }
            ui.separator();
        });
    egui::ScrollArea::vertical()
        .id_salt(("text file", path))
        .auto_shrink(false)
        .show(ui, |ui| {
            egui::Frame::new()
                .inner_margin(theme::panel_margin())
                .show(ui, |ui| match read {
                    None => {
                        ui.label(RichText::new("Reading…").color(theme::text_weak()));
                    }
                    Some(Err(why)) => {
                        ui.colored_label(theme::WARNING, format!("Can't show it: {why}."));
                    }
                    Some(Ok(text)) if text.empty => {
                        ui.label(RichText::new("It's empty.").color(theme::text_weak()));
                    }
                    Some(Ok(text)) => {
                        match &text.body {
                            Body::Markdown(blocks) => markdown(ui, blocks),
                            Body::Plain(shown) => plain_frame(ui, shown),
                        }
                        if text.more {
                            ui.label(
                                RichText::new(format!(
                                    "Only the first {MAX_LINES} lines are shown: open it in its \
                                     program for the rest."
                                ))
                                .color(theme::text_weak()),
                            );
                        }
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new(
                                "Read only: edit it in its program, and Solis shows the change \
                                 when you save.",
                            )
                            .color(theme::text_weak()),
                        );
                    }
                });
        });
    asked
}

#[cfg(test)]
#[path = "text_view_tests.rs"]
mod tests;
