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
    /// it.
    pub fn set_text_file(&mut self, path: String, read: TextRead) {
        self.open.text_file = Some((path, read));
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

/// `text`'s inline Markdown (`**strong**`, `*emphasis*`, `` `code` ``,
/// `[link](address)`) as runs, the marks taken out. Marks that aren't
/// closed are kept as written.
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
    let mut rest = text;
    while !rest.is_empty() {
        let found = [
            ("`", "`", Style::Code),
            ("**", "**", Style::Strong),
            ("__", "__", Style::Strong),
            ("*", "*", Style::Emphasis),
            ("_", "_", Style::Emphasis),
            ("[", "](", Style::Link),
        ]
        .iter()
        .filter_map(|(open, close, style)| {
            let at = rest.find(open)?;
            let inside = &rest[at + open.len()..];
            let end = inside.find(close)?;
            (end > 0).then_some((at, *open, *close, *style, end))
        })
        .min_by_key(|(at, open, ..)| (*at, std::cmp::Reverse(open.len())));
        let Some((at, open, close, style, end)) = found else {
            push(Style::Plain, rest);
            break;
        };
        push(Style::Plain, &rest[..at]);
        let inside = &rest[at + open.len()..at + open.len() + end];
        let mut after = &rest[at + open.len() + end + close.len()..];
        if style == Style::Link {
            // The address, up to its ")", isn't shown.
            after = after.find(')').map_or(after, |i| &after[i + 1..]);
        }
        push(style, inside);
        rest = after;
    }
    runs
}

/// Draws `words` with their inline Markdown, wrapped, at `size`.
fn inline(ui: &mut Ui, words: &str, size: f32, colour: Color32) {
    let mut job = LayoutJob::default();
    for (style, text) in inline_runs(words) {
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
                underline: egui::Stroke::new(1.0, theme::kind::NOTES),
                ..TextFormat::simple(FontId::proportional(size), theme::kind::NOTES)
            },
        };
        job.append(&text, 0.0, format);
    }
    job.wrap.max_width = ui.available_width();
    ui.label(job);
}

/// Draws a Markdown file's blocks.
fn markdown(ui: &mut Ui, text: &str) {
    for block in markdown_blocks(text) {
        match block {
            Block::Heading(level, words) => {
                let size = match level {
                    1 => 20.0,
                    2 => 17.0,
                    3 => 15.0,
                    _ => 13.5,
                };
                ui.add_space(if level <= 2 { 6.0 } else { 3.0 });
                inline(ui, &words, size, theme::text_strong());
                ui.add_space(2.0);
            }
            Block::Paragraph(words) => {
                inline(ui, &words, 13.0, theme::text());
                ui.add_space(6.0);
            }
            Block::Item(depth, marker, words) => {
                ui.horizontal_top(|ui| {
                    ui.add_space(4.0 + f32::from(depth) * 14.0);
                    ui.label(RichText::new(marker).color(theme::text_weak()));
                    inline(ui, &words, 13.0, theme::text());
                });
            }
            Block::Quote(words) => {
                let bar = egui::Frame::new()
                    .inner_margin(egui::Margin {
                        left: 10,
                        right: 0,
                        top: 2,
                        bottom: 2,
                    })
                    .show(ui, |ui| inline(ui, &words, 13.0, theme::text_weak()))
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
                plain_frame(ui, &code);
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
    read: Option<&TextRead>,
) -> Vec<LibraryRequest> {
    let mut asked = Vec::new();
    let name = path.rsplit_once('/').map_or(path, |(_, n)| n);
    let markdown_file = name.to_lowercase().ends_with(".md");
    egui::Frame::new()
        .inner_margin(theme::panel_margin())
        .show(ui, |ui| {
            ui.label(RichText::new(name).strong().color(theme::text_strong()));
            let about = match read {
                Some(Ok(text)) => {
                    let lines = text.lines().count();
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
                    Some(Ok(text)) if text.trim().is_empty() => {
                        ui.label(RichText::new("It's empty.").color(theme::text_weak()));
                    }
                    Some(Ok(text)) => {
                        let (shown, more) = first_lines(text);
                        if markdown_file {
                            markdown(ui, &shown);
                        } else {
                            plain_frame(ui, &shown);
                        }
                        if more {
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
