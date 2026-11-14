//! A manual page's blocks as the guide's HTML: a chapter, its sections,
//! and the guide's own elements for what the manual writes:
//!
//! - a quote starting "Good to know" or "Note" is the blue box, one
//!   starting "Careful" or "Warning" the amber one, and the quote under a
//!   page's title is its opening sentence;
//! - a numbered list is numbered steps;
//! - keys are keys (see [`crate::inline`]);
//! - every heading gets a `#` link to itself;
//! - a picture alone on its line, with a caption line under it, is a
//!   figure that opens larger when clicked ([`crate::pictures`]);
//! - three fenced blocks one after another, `lust`, `rust` and `lua`, are
//!   one example with a switch between the languages
//!   ([`mod@crate::examples`]).

use crate::blocks::{Block, List, Table};
use crate::examples::{self, LANGUAGES};
use crate::inline::{self, Places, escape};

/// A heading's anchor, as GitHub makes it: lower case, letters, numbers,
/// spaces and hyphens kept, spaces made hyphens. Links between manual
/// pages use these, so the guide does too.
pub fn slug(words: &str) -> String {
    let plain = plain_words(words);
    plain
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-' || *c == '_')
        .map(|c| if c == ' ' { '-' } else { c })
        .collect()
}

/// A heading's words without Markdown's marks: what a reader sees.
pub fn plain_words(words: &str) -> String {
    let mut out = String::new();
    let mut chars = words.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '`' | '*' => {}
            '_' if out.ends_with(|c: char| !c.is_alphanumeric()) || out.is_empty() => {}
            '\\' => {
                if let Some(next) = chars.next() {
                    out.push(next);
                }
            }
            // A link's words stay; its address goes.
            ']' if chars.peek() == Some(&'(') => {
                for c in chars.by_ref() {
                    if c == ')' {
                        break;
                    }
                }
            }
            '[' => {}
            _ => out.push(c),
        }
    }
    out
}

/// Every anchor a page has: its headings' slugs (repeats numbered as
/// GitHub numbers them) and the ids it names itself.
pub fn anchors(blocks: &[Block]) -> Vec<String> {
    let mut headings = Vec::new();
    let mut named = Vec::new();
    collect_anchors(blocks, &mut headings, &mut named);
    headings.extend(named);
    headings
}

fn collect_anchors(blocks: &[Block], headings: &mut Vec<String>, named: &mut Vec<String>) {
    for block in blocks {
        match block {
            Block::Heading(_, words) => {
                let id = unique(&slug(words), headings);
                headings.push(id);
            }
            Block::Paragraph(words) => named_ids(words, named),
            Block::List(list) => list
                .items
                .iter()
                .for_each(|i| collect_anchors(i, headings, named)),
            Block::Quote(inner) => collect_anchors(inner, headings, named),
            Block::Table(table) => table
                .head
                .iter()
                .chain(table.rows.iter().flatten())
                .for_each(|cell| named_ids(cell, named)),
            Block::Code(..) | Block::Rule => {}
        }
    }
}

/// The ids named with `<a id="…">` in some words.
fn named_ids(words: &str, seen: &mut Vec<String>) {
    let mut rest = words;
    while let Some(at) = rest.find("<a id=\"") {
        rest = &rest[at + 7..];
        if let Some(end) = rest.find('"') {
            seen.push(rest[..end].to_string());
            rest = &rest[end..];
        }
    }
}

fn unique(slug: &str, seen: &[String]) -> String {
    if !seen.iter().any(|s| s == slug) {
        return slug.to_string();
    }
    (1..)
        .map(|n| format!("{slug}-{n}"))
        .find(|candidate| !seen.iter().any(|s| s == candidate))
        .unwrap_or_default()
}

/// What kind of box a quote is, from its first words.
fn box_kind(blocks: &[Block]) -> Option<(&'static str, usize)> {
    let Some(Block::Paragraph(words)) = blocks.first() else {
        return None;
    };
    let plain = plain_words(words);
    [
        ("Good to know", "note"),
        ("Note", "note"),
        ("Careful", "warn"),
        ("Warning", "warn"),
    ]
    .into_iter()
    .find(|(start, _)| {
        plain.starts_with(start) && plain[start.len()..].starts_with([':', '.', ' ', '\n'])
    })
    .map(|(start, kind)| (kind, start.len()))
}

/// Writes one page's blocks. `places` turns the manual's links and
/// anchors into the guide's; `ids` gives each heading its id.
pub struct Writer<'a> {
    places: &'a mut dyn Places,
    seen: Vec<String>,
    out: String,
}

impl<'a> Writer<'a> {
    /// A writer for one page.
    pub fn new(places: &'a mut dyn Places) -> Self {
        Self {
            places,
            seen: Vec::new(),
            out: String::new(),
        }
    }

    /// The HTML written.
    pub fn finish(self) -> String {
        self.out
    }

    /// The id the guide gives a heading with these words: its slug,
    /// through the page's anchors.
    pub fn heading_id(&mut self, words: &str) -> String {
        let id = unique(&slug(words), &self.seen);
        self.seen.push(id.clone());
        self.places.anchor(&id)
    }

    /// Words as HTML.
    pub fn words(&mut self, text: &str) -> String {
        inline::render(text, self.places)
    }

    /// Adds raw HTML.
    pub fn push(&mut self, html: &str) {
        self.out.push_str(html);
    }

    /// Writes blocks, with each heading one level deeper than the manual
    /// has it (a page's sections are a chapter's).
    pub fn blocks(&mut self, blocks: &[Block]) {
        let mut at = 0;
        while at < blocks.len() {
            if let Some(codes) = examples::three_at(blocks, at) {
                self.example(codes);
                at += LANGUAGES.len();
            } else {
                self.block(&blocks[at]);
                at += 1;
            }
        }
    }

    /// One example in the three languages: the switch, then each
    /// language's code. Without the guide's script all three show, each
    /// named; with it, the one chosen.
    fn example(&mut self, codes: [&str; 3]) {
        self.out.push_str(
            "<div class=\"example\">\n<div class=\"langs\" role=\"tablist\" aria-label=\"Language\">",
        );
        for (n, (word, name)) in LANGUAGES.iter().enumerate() {
            let chosen = if n == 0 { "true" } else { "false" };
            self.out.push_str(&format!(
                "<button type=\"button\" role=\"tab\" data-lang=\"{word}\" aria-selected=\"{chosen}\">{name}</button>"
            ));
        }
        self.out.push_str("</div>\n");
        for ((word, name), code) in LANGUAGES.iter().zip(codes) {
            self.out.push_str(&format!(
                "<pre data-lang=\"{word}\" data-name=\"{name}\"><code>{}</code></pre>\n",
                escape(code)
            ));
        }
        self.out.push_str("</div>\n");
    }

    /// A paragraph: a figure when it starts with one of the manual's
    /// pictures alone on its line (the lines under it are its caption).
    fn paragraph(&mut self, words: &str) {
        let (first, caption) = words.split_once('\n').unwrap_or((words, ""));
        let first = first.trim();
        let alone = first.starts_with("![")
            && first.ends_with(')')
            && first.contains(&format!("]({}", crate::pictures::PREFIX));
        if !alone {
            let html = self.words(words);
            self.out.push_str(&format!("<p>{html}</p>\n"));
            return;
        }
        let picture = self.words(first);
        self.out.push_str(&format!(
            "<figure class=\"shot\">\n<button type=\"button\" class=\"zoom\" aria-label=\"Show this picture larger\">{picture}</button>\n"
        ));
        if caption.trim().is_empty() {
            let why = format!("the picture {first} has no caption: write one on the line under it");
            self.places.problem(why);
        } else {
            let html = self.words(caption.trim());
            self.out
                .push_str(&format!("<figcaption>{html}</figcaption>\n"));
        }
        self.out.push_str("</figure>\n");
    }

    fn block(&mut self, block: &Block) {
        match block {
            Block::Heading(level, words) => {
                let id = self.heading_id(words);
                let level = (level + 1).min(6);
                let html = self.words(words);
                self.out.push_str(&format!(
                    "<h{level} id=\"{id}\">{html}{}</h{level}>\n",
                    anchor_link(&id)
                ));
            }
            Block::Paragraph(words) => self.paragraph(words),
            Block::Code(language, text) => {
                if examples::is_game_code(language) {
                    self.places.problem(format!(
                        "a game-code example in {language} isn't in all three languages: write \
                         it as three blocks one after another, ```lust, ```rust and ```lua \
                         (_template.md)"
                    ));
                } else if language.is_empty() && examples::looks_like_lust(text) {
                    self.places.problem(format!(
                        "a code block starting \"{}\" reads as Lust game code: fence it \
                         ```lust, with ```rust and ```lua after it (_template.md)",
                        text.lines().next().unwrap_or("").trim()
                    ));
                }
                let class = if language.is_empty() {
                    String::new()
                } else {
                    format!(" data-lang=\"{}\"", escape(language))
                };
                self.out.push_str(&format!(
                    "<pre{class}><code>{}</code></pre>\n",
                    escape(text)
                ));
            }
            Block::List(list) => self.list(list),
            Block::Table(table) => self.table(table),
            Block::Quote(inner) => self.quote(inner),
            Block::Rule => self.out.push_str("<hr>\n"),
        }
    }

    fn list(&mut self, list: &List) {
        let open = match list.start {
            Some(1) => "<ol class=\"steps\">".to_string(),
            Some(n) => format!(
                "<ol class=\"steps\" start=\"{n}\" style=\"counter-reset: step {}\">",
                n.saturating_sub(1)
            ),
            None => "<ul>".to_string(),
        };
        self.out.push_str(&open);
        self.out.push('\n');
        for item in &list.items {
            self.out.push_str("<li>");
            match item.as_slice() {
                [Block::Paragraph(words), rest @ ..] if !list.loose => {
                    let html = self.words(words);
                    self.out.push_str(&html);
                    if !rest.is_empty() {
                        self.out.push('\n');
                        self.blocks(rest);
                    }
                }
                blocks => self.blocks(blocks),
            }
            self.out.push_str("</li>\n");
        }
        self.out.push_str(if list.start.is_some() {
            "</ol>\n"
        } else {
            "</ul>\n"
        });
    }

    fn table(&mut self, table: &Table) {
        // A table whose first column says what to do and whose second
        // gives keys is a table of keys, drawn a little differently.
        let keys = table.head.get(1).is_some_and(|h| {
            ["Shortcut", "Key", "Mouse or key", "Keyboard and mouse"].contains(&h.as_str())
        });
        let class = if keys { " class=\"keys\"" } else { "" };
        self.out.push_str(&format!(
            "<div class=\"table-wrap\"><table{class}>\n<thead><tr>"
        ));
        for cell in &table.head {
            let html = self.words(cell);
            self.out.push_str(&format!("<th>{html}</th>"));
        }
        self.out.push_str("</tr></thead>\n<tbody>\n");
        for row in &table.rows {
            self.out.push_str("<tr>");
            for cell in row {
                let html = self.words(cell);
                self.out.push_str(&format!("<td>{html}</td>"));
            }
            self.out.push_str("</tr>\n");
        }
        self.out.push_str("</tbody>\n</table></div>\n");
    }

    fn quote(&mut self, inner: &[Block]) {
        match box_kind(inner) {
            Some((kind, start)) => {
                self.out.push_str(&format!("<div class=\"{kind}\">\n"));
                // The box says "Good to know" itself: the words saying so
                // are left out.
                if let Some((Block::Paragraph(words), rest)) = inner.split_first() {
                    let words = strip_start(words, start);
                    if !words.trim().is_empty() {
                        let html = self.words(&words);
                        self.out.push_str(&format!("<p>{html}</p>\n"));
                    }
                    self.blocks(rest);
                }
                self.out.push_str("</div>\n");
            }
            None => {
                self.out.push_str("<blockquote>\n");
                self.blocks(inner);
                self.out.push_str("</blockquote>\n");
            }
        }
    }
}

/// A box's first words without the `length` letters that named it, and
/// the marks round them (`**Good to know:**`).
fn strip_start(words: &str, length: usize) -> String {
    let mut seen = 0;
    let mut cut = 0;
    for (at, c) in words.char_indices() {
        if seen == length {
            cut = at;
            break;
        }
        if c != '*' && c != '_' {
            seen += 1;
        }
        cut = at + c.len_utf8();
    }
    let rest = words[cut..]
        .trim_start_matches(['*', '_', ':', '.'])
        .trim_start_matches(['*', '_'])
        .trim_start();
    // The box's own label ends a sentence: what follows starts one.
    let mut chars = rest.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// The `#` after a heading that links to it.
pub fn anchor_link(id: &str) -> String {
    format!("<a class=\"anchor\" href=\"#{id}\" aria-label=\"Link to this section\">#</a>")
}

#[cfg(test)]
#[path = "html_tests.rs"]
mod tests;
