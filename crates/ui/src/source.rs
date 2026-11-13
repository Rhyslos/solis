//! A screen's Lust file as the UI editor reads and changes it (decision
//! 0089, the canvas): each assembly's `view { }` as a tree of pieces
//! (`column [gap 8] { … }`, `text "Paused" [size 28];`, `button "Resume"
//! { … }`), each with where it is in the text, so the canvas changes the
//! fewest letters it can and keeps everything else exactly as written:
//! comments, spacing, and code it doesn't understand.
//!
//! A piece of the view the canvas can't edit (a loop, a question, a
//! declaration) is [`Piece::Code`]: kept as it is, and, when it can make
//! pieces of the screen, the canvas shows what it makes as code.
//!
//! The reader is Lust's own grammar for views, read loosely: it never
//! panics, and a file it can't follow is left alone ([`parse`] says why).

use std::ops::Range;

/// The pieces Lust knows in a view (its `parser.rs`, `ELEMENTS`).
pub const ELEMENTS: &[&str] = &[
    "column", "row", "stack", "scroll", "space", "text", "button", "input", "checkbox", "slider",
    "image", "every", "key", "textarea", "dropdown", "menu", "item", "divider", "dialog", "grid",
    "split", "tabs", "tab", "branch", "progress", "canvas", "box", "circle", "line",
];

/// Pieces that hold others in their `{ }`.
pub fn is_container(kind: &str) -> bool {
    matches!(
        kind,
        "column"
            | "row"
            | "stack"
            | "scroll"
            | "menu"
            | "dialog"
            | "grid"
            | "split"
            | "tabs"
            | "tab"
            | "branch"
            | "canvas"
    )
}

/// Pieces that take no value before their `[ ]`.
pub fn is_bare(kind: &str) -> bool {
    matches!(
        kind,
        "column"
            | "row"
            | "stack"
            | "scroll"
            | "space"
            | "divider"
            | "grid"
            | "split"
            | "canvas"
            | "box"
            | "circle"
            | "line"
    )
}

/// The biggest file read.
pub const MAX_BYTES: usize = 1 << 20;

/// How deep pieces may sit inside one another.
pub const MAX_DEPTH: usize = 64;

/// One word in a piece's `[ ]`: `center`, `gap 12`, `hover:[ … ]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyleSource {
    /// Its name.
    pub name: String,
    /// The whole item, name and value.
    pub span: Range<usize>,
    /// Its value's text, if it has one (`12` in `gap 12`; the `[ … ]` of
    /// `hover:[ … ]`).
    pub value: Option<Range<usize>>,
}

/// A piece's `[ ]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyleList {
    /// From `[` to `]`, both in.
    pub span: Range<usize>,
    /// Its items, in order.
    pub items: Vec<StyleSource>,
}

/// A piece's value before its `[ ]`: `"Resume"`, `hp.text()`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArgSource {
    /// Its text.
    pub span: Range<usize>,
    /// The words, when it's plain text in quotes (no `{value}` in it).
    pub words: Option<String>,
}

/// A `{ }` holding pieces: the view's, or a container's.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Block {
    /// Inside the braces.
    pub inner: Range<usize>,
    /// What's in it, in order.
    pub pieces: Vec<Piece>,
}

/// A piece of the screen in the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementSource {
    /// What it is: `column`, `text`, `button`…
    pub kind: String,
    /// The whole piece, from its kind to its `;` or `}`.
    pub span: Range<usize>,
    /// Its line, from 1.
    pub line: u32,
    /// Its value, if it has one.
    pub arg: Option<ArgSource>,
    /// Its `[ ]`, if it has one.
    pub styles: Option<StyleList>,
    /// What it holds, for a container.
    pub children: Option<Block>,
    /// What it does (a button's `{ }`), inside the braces.
    pub action: Option<Range<usize>>,
    /// Where its kind, value and a dropdown's choices end: a new `[ ]`
    /// goes here.
    pub head_end: usize,
}

/// Code in a view that isn't a piece.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeSource {
    /// Its text.
    pub span: Range<usize>,
    /// Its line, from 1.
    pub line: u32,
    /// Whether it can make pieces of the screen (a loop or a question
    /// with pieces in it), so what follows it can't be matched by order.
    pub makes_pieces: bool,
}

/// What a view or a container holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Piece {
    /// A piece of the screen.
    Element(ElementSource),
    /// Code that isn't.
    Code(CodeSource),
}

/// An assembly with a view: a screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScreenSource {
    /// Its name (`PauseMenu`).
    pub name: String,
    /// Its line, from 1.
    pub line: u32,
    /// Its view's `[ ]` (`view [title "Menu"]`), if it has one.
    pub settings: Option<StyleList>,
    /// What its view holds.
    pub view: Block,
}

impl Block {
    /// The piece at `path` (indices into each block's pieces).
    pub fn at(&self, path: &[usize]) -> Option<&ElementSource> {
        let (first, rest) = path.split_first()?;
        let Piece::Element(e) = self.pieces.get(*first)? else {
            return None;
        };
        if rest.is_empty() {
            Some(e)
        } else {
            e.children.as_ref()?.at(rest)
        }
    }

    /// The block at `path`: this one for none, else that piece's.
    pub fn block_at(&self, path: &[usize]) -> Option<&Block> {
        if path.is_empty() {
            return Some(self);
        }
        self.at(path)?.children.as_ref()
    }
}

impl StyleList {
    /// The item called `name`, the last if it's there twice.
    pub fn get(&self, name: &str) -> Option<&StyleSource> {
        self.items.iter().rev().find(|s| s.name == name)
    }
}

impl ElementSource {
    /// Its `[ ]` item called `name`.
    pub fn style(&self, name: &str) -> Option<&StyleSource> {
        self.styles.as_ref()?.get(name)
    }

    /// The text of its `[ ]` item `name`'s value in `text` (`""` for a
    /// word on its own), if it has the item.
    pub fn style_text<'a>(&self, text: &'a str, name: &str) -> Option<&'a str> {
        let s = self.style(name)?;
        Some(s.value.clone().map_or("", |v| &text[v]))
    }

    /// Whether it has the word `name` in its `[ ]`, alone or with a value.
    pub fn has(&self, name: &str) -> bool {
        self.style(name).is_some()
    }
}

// ------------------------------------------------------------------ words

#[derive(Debug, Clone, PartialEq)]
enum Tk {
    Word(String),
    Number,
    /// Text in quotes, and its words if it's plain.
    Text(Option<String>),
    Sym(char),
}

#[derive(Debug, Clone)]
struct Token {
    kind: Tk,
    span: Range<usize>,
    line: u32,
}

/// Lust's words, with where each is; comments left out.
fn tokens(text: &str) -> Vec<Token> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    let mut line = 1u32;
    while i < b.len() {
        let c = b[i];
        if c == b'\n' {
            line += 1;
            i += 1;
            continue;
        }
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        // Comments: --[[ block ]], -- line, // line.
        if c == b'-' && b.get(i + 1) == Some(&b'-') {
            if b.get(i + 2) == Some(&b'[') && b.get(i + 3) == Some(&b'[') {
                i += 4;
                while i < b.len() && !(b[i] == b']' && b.get(i + 1) == Some(&b']')) {
                    if b[i] == b'\n' {
                        line += 1;
                    }
                    i += 1;
                }
                i = (i + 2).min(b.len());
            } else {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            continue;
        }
        if c == b'/' && b.get(i + 1) == Some(&b'/') {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        let start = i;
        if c == b'"' {
            i += 1;
            let mut words = String::new();
            let mut plain = true;
            while i < b.len() && b[i] != b'\n' {
                match b[i] {
                    b'"' => {
                        i += 1;
                        break;
                    }
                    b'\\' => {
                        match b.get(i + 1) {
                            Some(b'n') => words.push('\n'),
                            Some(b't') => words.push('\t'),
                            Some(&e) if matches!(e, b'"' | b'\\' | b'{' | b'}') => {
                                words.push(e as char)
                            }
                            _ => plain = false,
                        }
                        // Past the backslash and the whole letter after it.
                        i = (i + 2).min(b.len());
                        while !text.is_char_boundary(i) {
                            i += 1;
                        }
                    }
                    b'{' if b.get(i + 1).is_some_and(|n| {
                        n.is_ascii_alphabetic() || matches!(n, b'(' | b'#' | b'"')
                    }) =>
                    {
                        // A value inside the text: not plain.
                        plain = false;
                        let mut depth = 0;
                        while i < b.len() && b[i] != b'\n' {
                            match b[i] {
                                b'{' => depth += 1,
                                b'}' => {
                                    depth -= 1;
                                    if depth == 0 {
                                        i += 1;
                                        break;
                                    }
                                }
                                b'"' => {
                                    i += 1;
                                    while i < b.len() && b[i] != b'"' && b[i] != b'\n' {
                                        i += if b[i] == b'\\' { 2 } else { 1 };
                                    }
                                }
                                _ => {}
                            }
                            i += 1;
                        }
                    }
                    _ => {
                        // A whole letter, however many bytes.
                        let ch = text[i..].chars().next().unwrap_or(' ');
                        words.push(ch);
                        i += ch.len_utf8();
                    }
                }
            }
            let i2 = i.min(b.len());
            i = i2;
            out.push(Token {
                kind: Tk::Text(plain.then_some(words)),
                span: start..i,
                line,
            });
            continue;
        }
        if c.is_ascii_alphabetic() || c == b'_' || c >= 0x80 {
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_' || b[i] >= 0x80) {
                i += 1;
            }
            // Never inside a letter.
            while !text.is_char_boundary(i) {
                i += 1;
            }
            out.push(Token {
                kind: Tk::Word(text[start..i].to_string()),
                span: start..i,
                line,
            });
            continue;
        }
        if c.is_ascii_digit() {
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'.') {
                i += 1;
            }
            out.push(Token {
                kind: Tk::Number,
                span: start..i,
                line,
            });
            continue;
        }
        out.push(Token {
            kind: Tk::Sym(c as char),
            span: i..i + 1,
            line,
        });
        i += 1;
    }
    out
}

// ---------------------------------------------------------------- reading

struct Reader<'a> {
    t: &'a [Token],
    at: usize,
}

impl Reader<'_> {
    fn peek(&self, n: usize) -> Option<&Token> {
        self.t.get(self.at + n)
    }

    fn is_sym(&self, n: usize, c: char) -> bool {
        matches!(self.peek(n), Some(Token { kind: Tk::Sym(s), .. }) if *s == c)
    }

    fn is_word(&self, n: usize, w: &str) -> bool {
        matches!(self.peek(n), Some(Token { kind: Tk::Word(s), .. }) if s == w)
    }

    fn end(&self) -> usize {
        self.t
            .get(self.at.saturating_sub(1))
            .map_or(0, |t| t.span.end)
    }

    /// Past a bracketed run starting at an opening `open`; gives the
    /// closing one's index, or why not.
    fn skip_balanced(&mut self, open: char, close: char) -> Result<(), String> {
        let mut depth = 0usize;
        while let Some(t) = self.peek(0) {
            match t.kind {
                Tk::Sym(c) if c == open => depth += 1,
                Tk::Sym(c) if c == close => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        self.at += 1;
                        return Ok(());
                    }
                }
                _ => {}
            }
            self.at += 1;
        }
        Err(format!("a {open} is never closed with {close}"))
    }

    /// A statement that isn't a piece, to its `;` or its last `}` at this
    /// depth; whether a piece's word was inside a `{ }` of it.
    fn code(&mut self) -> Result<(Range<usize>, u32, bool), String> {
        let first = self.peek(0).ok_or("the view ends early")?;
        let (start, line) = (first.span.start, first.line);
        let mut makes = false;
        let mut depth = 0usize;
        while let Some(t) = self.peek(0).cloned() {
            match &t.kind {
                Tk::Sym('{') | Tk::Sym('(') | Tk::Sym('[') => depth += 1,
                Tk::Sym('}') if depth == 0 => {
                    // The block holding it ends: it had no ;.
                    return Ok((start..self.end().max(start), line, makes));
                }
                Tk::Sym('}') | Tk::Sym(')') | Tk::Sym(']') => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 && matches!(t.kind, Tk::Sym('}')) {
                        self.at += 1;
                        // `is …? then { } or { }` and `loop … { }` go on
                        // past a } when `or` follows.
                        if self.is_word(0, "or") {
                            continue;
                        }
                        return Ok((start..t.span.end, line, makes));
                    }
                }
                Tk::Sym(';') if depth == 0 => {
                    self.at += 1;
                    return Ok((start..t.span.end, line, makes));
                }
                Tk::Word(w) if depth > 0 && ELEMENTS.contains(&w.as_str()) => makes = true,
                _ => {}
            }
            self.at += 1;
        }
        Err("a statement in the view never ends".into())
    }

    /// A `[ ]` list, the reader at its `[`.
    fn style_list(&mut self) -> Result<StyleList, String> {
        let open = self.peek(0).ok_or("the view ends early")?.span.start;
        self.at += 1;
        let mut items = Vec::new();
        loop {
            if self.is_sym(0, ']') {
                let end = self.peek(0).map_or(open, |t| t.span.end);
                self.at += 1;
                return Ok(StyleList {
                    span: open..end,
                    items,
                });
            }
            let Some(Token {
                kind: Tk::Word(name),
                span,
                ..
            }) = self.peek(0).cloned()
            else {
                return Err("a [ ] holds words like gap 8 or center".into());
            };
            self.at += 1;
            let item_start = span.start;
            let mut value = None;
            if self.is_sym(0, ':') && self.is_sym(1, '[') {
                self.at += 1;
                let v0 = self.peek(0).map_or(0, |t| t.span.start);
                self.skip_balanced('[', ']')?;
                value = Some(v0..self.end());
            } else if !self.is_sym(0, ',') && !self.is_sym(0, ']') {
                // A value to the next , or ] at this depth.
                let v0 = self.peek(0).map_or(0, |t| t.span.start);
                let mut depth = 0usize;
                while let Some(t) = self.peek(0) {
                    match t.kind {
                        Tk::Sym('(') | Tk::Sym('[') => depth += 1,
                        Tk::Sym(')') => depth = depth.saturating_sub(1),
                        Tk::Sym(']') if depth == 0 => break,
                        Tk::Sym(']') => depth -= 1,
                        Tk::Sym(',') if depth == 0 => break,
                        _ => {}
                    }
                    self.at += 1;
                }
                value = Some(v0..self.end());
            }
            items.push(StyleSource {
                name,
                span: item_start..self.end(),
                value,
            });
            if self.is_sym(0, ',') {
                self.at += 1;
            } else if !self.is_sym(0, ']') {
                return Err("the items in a [ ] are split by commas".into());
            }
        }
    }

    /// The pieces in a `{ }`, the reader just past its `{`; stops at its
    /// `}` (past it).
    fn block(&mut self, open: usize, depth: usize) -> Result<Block, String> {
        if depth > MAX_DEPTH {
            return Err(format!("pieces sit more than {MAX_DEPTH} deep"));
        }
        let mut pieces = Vec::new();
        loop {
            let Some(t) = self.peek(0).cloned() else {
                return Err("a { in the view is never closed".into());
            };
            if matches!(t.kind, Tk::Sym('}')) {
                self.at += 1;
                return Ok(Block {
                    inner: open..t.span.start,
                    pieces,
                });
            }
            let element = match &t.kind {
                Tk::Word(w) if ELEMENTS.contains(&w.as_str()) => {
                    // `text = 3;` is code, not a piece.
                    !(self.is_sym(1, '=') || self.is_sym(1, '.') || self.is_sym(1, '('))
                }
                // A game's own piece: `TodoRow [w 200] { label = x; }`.
                Tk::Word(w) if w.starts_with(|c: char| c.is_ascii_uppercase()) => {
                    self.is_sym(1, '[') || self.is_sym(1, '{') || self.is_sym(1, ';')
                }
                _ => false,
            };
            if element {
                pieces.push(Piece::Element(self.element(depth)?));
            } else {
                let (span, line, makes_pieces) = self.code()?;
                pieces.push(Piece::Code(CodeSource {
                    span,
                    line,
                    makes_pieces,
                }));
            }
        }
    }

    /// A piece, the reader at its kind.
    fn element(&mut self, depth: usize) -> Result<ElementSource, String> {
        let t = self.peek(0).cloned().ok_or("the view ends early")?;
        let Tk::Word(kind) = t.kind else {
            return Err("a piece starts with its kind".into());
        };
        self.at += 1;
        let start = t.span.start;
        let container = is_container(&kind);
        let mut arg = None;
        if !is_bare(&kind) && !self.is_sym(0, '[') && !self.is_sym(0, '{') && !self.is_sym(0, ';') {
            let a0 = self.peek(0).ok_or("the view ends early")?.span.start;
            let lone_text = match self.peek(0) {
                Some(Token {
                    kind: Tk::Text(words),
                    ..
                }) => words.clone(),
                _ => None,
            };
            let first = self.at;
            let mut depth = 0usize;
            while let Some(t) = self.peek(0) {
                match t.kind {
                    Tk::Sym('(') => depth += 1,
                    Tk::Sym(')') => depth = depth.saturating_sub(1),
                    Tk::Sym('[') | Tk::Sym('{') | Tk::Sym(';') if depth == 0 => break,
                    Tk::Word(ref w)
                        if depth == 0 && (w == "in" || w == "enter") && self.at > first =>
                    {
                        break;
                    }
                    _ => {}
                }
                self.at += 1;
            }
            let words = if self.at == first + 1 {
                lone_text
            } else {
                None
            };
            arg = Some(ArgSource {
                span: a0..self.end(),
                words,
            });
        }
        // dropdown size in ["S", "M"]: its choices, then perhaps its [ ].
        if self.is_word(0, "in") {
            self.at += 1;
            if self.is_sym(0, '[') {
                self.skip_balanced('[', ']')?;
            } else {
                while self.peek(0).is_some()
                    && !(self.is_sym(0, '[') || self.is_sym(0, ';') || self.is_sym(0, '{'))
                {
                    self.at += 1;
                }
            }
        }
        let head_end = self.end().max(start + kind.len());
        let styles = if self.is_sym(0, '[') {
            Some(self.style_list()?)
        } else {
            None
        };
        let mut children = None;
        let mut action = None;
        if self.is_sym(0, '{') {
            let open = self.peek(0).map_or(0, |t| t.span.end);
            if container {
                self.at += 1;
                children = Some(self.block(open, depth + 1)?);
            } else {
                self.skip_balanced('{', '}')?;
                let close = self.t.get(self.at - 1).map_or(open, |t| t.span.start);
                action = Some(open..close);
            }
        } else if self.is_sym(0, ';') {
            self.at += 1;
        } else if !(matches!(kind.as_str(), "input" | "textarea")
            && self.is_word(0, "enter")
            && self.is_sym(1, '{'))
        {
            return Err(format!("a {kind} ends with ; or {{ }}"));
        }
        if self.is_word(0, "enter") && self.is_sym(1, '{') {
            self.at += 1;
            self.skip_balanced('{', '}')?;
        }
        Ok(ElementSource {
            kind,
            span: start..self.end(),
            line: t.line,
            arg,
            styles,
            children,
            action,
            head_end,
        })
    }
}

/// Reads every screen (an assembly with a view) in a Lust file. Gives why
/// not when the file can't be followed: it's then left alone.
pub fn parse(text: &str) -> Result<Vec<ScreenSource>, String> {
    if text.len() > MAX_BYTES {
        return Err(format!("it's bigger than {} KB", MAX_BYTES / 1024));
    }
    let tokens = tokens(text);
    let mut r = Reader { t: &tokens, at: 0 };
    let mut screens = Vec::new();
    while let Some(t) = r.peek(0).cloned() {
        if !matches!(&t.kind, Tk::Word(w) if w == "assembly") {
            r.at += 1;
            continue;
        }
        let Some(Token {
            kind: Tk::Word(name),
            ..
        }) = r.peek(1).cloned()
        else {
            r.at += 1;
            continue;
        };
        r.at += 2;
        if !r.is_sym(0, '{') {
            continue;
        }
        r.at += 1;
        // The assembly's body: has { }, methods, and its view.
        let mut depth = 1usize;
        let mut view = None;
        while let Some(t) = r.peek(0).cloned() {
            match &t.kind {
                Tk::Word(w) if w == "view" && depth == 1 => {
                    r.at += 1;
                    let settings = if r.is_sym(0, '[') {
                        Some(r.style_list()?)
                    } else {
                        None
                    };
                    if !r.is_sym(0, '{') {
                        return Err(format!("{name}'s view has no {{ }}"));
                    }
                    let open = r.peek(0).map_or(0, |t| t.span.end);
                    r.at += 1;
                    let block = r.block(open, 0)?;
                    view = Some((settings, block));
                    continue;
                }
                Tk::Sym('{') => depth += 1,
                Tk::Sym('}') => {
                    depth -= 1;
                    if depth == 0 {
                        r.at += 1;
                        break;
                    }
                }
                _ => {}
            }
            r.at += 1;
        }
        if let Some((settings, view)) = view {
            screens.push(ScreenSource {
                name,
                line: t.line,
                settings,
                view,
            });
        }
    }
    Ok(screens)
}

// ---------------------------------------------------------------- changing

/// The line end `text` uses: Windows' `\r\n` if it has one, else `\n`.
fn eol(text: &str) -> &'static str {
    if text.contains("\r\n") { "\r\n" } else { "\n" }
}

/// The spaces at the start of the line `at` is on.
fn indent_at(text: &str, at: usize) -> &str {
    let line_start = text[..at.min(text.len())].rfind('\n').map_or(0, |i| i + 1);
    let rest = &text[line_start..];
    let n = rest.len() - rest.trim_start_matches([' ', '\t']).len();
    &rest[..n]
}

/// `text` with `range` replaced by `with`.
pub fn replaced(text: &str, range: Range<usize>, with: &str) -> String {
    let mut out = String::with_capacity(text.len() + with.len());
    out.push_str(&text[..range.start]);
    out.push_str(with);
    out.push_str(&text[range.end..]);
    out
}

/// Text in quotes for Lust: `Say "hi"` as `"Say \"hi\""`, with `{` kept
/// plain.
pub fn quoted(words: &str) -> String {
    let mut out = String::from("\"");
    for c in words.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '{' => out.push_str("\\{"),
            '}' => out.push_str("\\}"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Sets `piece`'s `[ ]` item `name` to `value` (`None`: the word alone,
/// as `center`), changing only that item, or adding it at the end of its
/// `[ ]` (making one after its value if it has none).
pub fn set_style(text: &str, piece: &ElementSource, name: &str, value: Option<&str>) -> String {
    let item = match value {
        Some(v) if !v.is_empty() => {
            if v.starts_with('[') {
                format!("{name}:{v}")
            } else {
                format!("{name} {v}")
            }
        }
        _ => name.to_string(),
    };
    match &piece.styles {
        Some(list) => match list.get(name) {
            Some(s) => replaced(text, s.span.clone(), &item),
            None => {
                let close = list.span.end - 1;
                let sep = if list.items.is_empty() { "" } else { ", " };
                replaced(text, close..close, &format!("{sep}{item}"))
            }
        },
        None => {
            // After the value (and a dropdown's choices), or the kind.
            let after = piece.head_end;
            replaced(text, after..after, &format!(" [{item}]"))
        }
    }
}

/// Takes `piece`'s `[ ]` items called any of `names` away, and its `[ ]`
/// with them if nothing is left in it.
pub fn remove_styles(text: &str, piece: &ElementSource, names: &[&str]) -> String {
    let Some(list) = &piece.styles else {
        return text.to_string();
    };
    let kept: Vec<&StyleSource> = list
        .items
        .iter()
        .filter(|s| !names.contains(&s.name.as_str()))
        .collect();
    if kept.len() == list.items.len() {
        return text.to_string();
    }
    if kept.is_empty() {
        // The [ ] and the space before it.
        let mut start = list.span.start;
        while start > 0 && text.as_bytes()[start - 1] == b' ' {
            start -= 1;
        }
        return replaced(text, start..list.span.end, "");
    }
    // The kept items as written, joined as they were.
    let inner: Vec<&str> = kept.iter().map(|s| &text[s.span.clone()]).collect();
    replaced(
        text,
        list.span.start + 1..list.span.end - 1,
        &inner.join(", "),
    )
}

/// Sets `piece`'s words (its value in quotes), or gives `None` when its
/// value isn't plain words (code makes it).
pub fn set_words(text: &str, piece: &ElementSource, words: &str) -> Option<String> {
    match &piece.arg {
        Some(a) if a.words.is_some() => Some(replaced(text, a.span.clone(), &quoted(words))),
        Some(_) => None,
        None if is_bare(&piece.kind) => None,
        None => {
            let after = piece.span.start + piece.kind.len();
            Some(replaced(text, after..after, &format!(" {}", quoted(words))))
        }
    }
}

/// Sets `piece`'s value to the code `expr` (`health.hp`, or
/// `health.hp.text()` for words), or gives `None` for a piece that takes
/// no value.
pub fn set_arg(text: &str, piece: &ElementSource, expr: &str) -> Option<String> {
    match &piece.arg {
        Some(a) => Some(replaced(text, a.span.clone(), expr)),
        None if is_bare(&piece.kind) => None,
        None => {
            let after = piece.span.start + piece.kind.len();
            Some(replaced(text, after..after, &format!(" {expr}")))
        }
    }
}

/// The range a piece takes up as whole lines (with its indent and line
/// end), when nothing else is on its lines; else just its text.
fn whole_lines(text: &str, span: &Range<usize>) -> Range<usize> {
    let line_start = text[..span.start].rfind('\n').map_or(0, |i| i + 1);
    let before = &text[line_start..span.start];
    let line_end = text[span.end..]
        .find('\n')
        .map_or(text.len(), |i| span.end + i + 1);
    let after = &text[span.end..line_end];
    if before.trim().is_empty() && after.trim().is_empty() {
        line_start..line_end
    } else {
        span.clone()
    }
}

/// Takes `piece` out of the file.
pub fn remove_piece(text: &str, piece: &ElementSource) -> String {
    replaced(text, whole_lines(text, &piece.span), "")
}

/// `piece`'s text, its lines indented as `indent` instead of as they were.
fn reindented(text: &str, piece: &ElementSource, indent: &str) -> String {
    let own = indent_at(text, piece.span.start);
    text[piece.span.clone()]
        .lines()
        .enumerate()
        .map(|(i, l)| {
            if i == 0 {
                l.to_string()
            } else {
                format!("{indent}{}", l.strip_prefix(own).unwrap_or(l.trim_start()))
            }
        })
        .collect::<Vec<_>>()
        .join(eol(text))
}

/// Where a new piece goes in `block` at `index` among its pieces: the
/// byte, and the indent for it.
fn place_in(text: &str, block: &Block, index: usize) -> (usize, String) {
    match block.pieces.get(index) {
        Some(p) => {
            let span = match p {
                Piece::Element(e) => &e.span,
                Piece::Code(c) => &c.span,
            };
            let indent = indent_at(text, span.start).to_string();
            let line_start = text[..span.start].rfind('\n').map_or(0, |i| i + 1);
            if text[line_start..span.start].trim().is_empty() {
                (line_start, indent)
            } else {
                (span.start, indent)
            }
        }
        None => {
            // At its end: after its last piece, else inside the braces.
            let indent = match block.pieces.last() {
                Some(Piece::Element(e)) => indent_at(text, e.span.start).to_string(),
                Some(Piece::Code(c)) => indent_at(text, c.span.start).to_string(),
                None => format!(
                    "{}    ",
                    indent_at(text, block.inner.start.saturating_sub(1))
                ),
            };
            let close = block.inner.end;
            let line_start = text[..close].rfind('\n').map_or(0, |i| i + 1);
            if text[line_start..close].trim().is_empty() && line_start > block.inner.start {
                (line_start, indent)
            } else {
                (close, indent)
            }
        }
    }
}

/// Puts `piece_text` (one piece, written for no indent) into `block` at
/// `index`, on lines of its own.
pub fn insert_piece(text: &str, block: &Block, index: usize, piece_text: &str) -> String {
    let (at, indent) = place_in(text, block, index);
    let body: Vec<String> = piece_text.lines().map(|l| format!("{indent}{l}")).collect();
    let line_start = text[..at].rfind('\n').map_or(0, |i| i + 1);
    let on_own_line = at == line_start;
    let nl = eol(text);
    let joined = body.join(nl);
    if on_own_line {
        replaced(text, at..at, &format!("{joined}{nl}"))
    } else {
        // A one-line block (`row { text "a"; }`): on its own line inside.
        let outer = indent_at(text, at);
        replaced(text, at..at, &format!("{nl}{joined}{nl}{outer}"))
    }
}

/// Moves `piece` into `block` (found again in the changed text by
/// `block_path` under the screen called `screen`) at `index`. Gives `None`
/// when the place is inside the piece itself or can't be found.
pub fn move_piece(
    text: &str,
    screen: &str,
    piece_path: &[usize],
    block_path: &[usize],
    index: usize,
) -> Option<String> {
    if block_path.starts_with(piece_path) {
        return None;
    }
    let screens = parse(text).ok()?;
    let view = &screens.iter().find(|s| s.name == screen)?.view;
    let piece = view.at(piece_path)?.clone();
    // Only a block that's there takes it. Its indent is the one it has
    // after the piece is taken out, so the piece goes in unindented here
    // and `insert_piece` indents it there.
    view.block_at(block_path)?;
    let moved = reindented(text, &piece, "");
    // In the same block, taking it out first moves those after it up one.
    let (parent, own) = piece_path.split_at(piece_path.len() - 1);
    let mut index = index;
    let mut block_path = block_path.to_vec();
    if parent == block_path.as_slice() && own[0] < index {
        index -= 1;
    }
    // A later block in the parent moves up one too.
    if block_path.len() > parent.len()
        && block_path.starts_with(parent)
        && block_path[parent.len()] > own[0]
    {
        block_path[parent.len()] -= 1;
    }
    let without = remove_piece(text, &piece);
    let screens = parse(&without).ok()?;
    let view = &screens.iter().find(|s| s.name == screen)?.view;
    let target = view.block_at(&block_path)?;
    let index = index.min(target.pieces.len());
    Some(insert_piece(&without, target, index, &moved))
}

#[cfg(test)]
#[path = "source_tests.rs"]
mod tests;
