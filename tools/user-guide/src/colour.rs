//! Game code coloured the way Ledger, the project lead's own code editor
//! for Lust, colours it: its "quiet colours". The guide's examples are
//! coloured here, as the guide is built, so the page stays one file and
//! needs no script to colour them.
//!
//! Each piece of code becomes `<span class="tk-…">` around its tokens,
//! with the classes Ledger's guides use:
//!
//! | Class | What it colours | Colour |
//! | --- | --- | --- |
//! | `tk-k` | structure: the words that give code its shape (`component`, `has`, `fn`, `local`) | the logo's red, a touch heavier |
//! | `tk-f` | flow: questions, loops, sending (`is then or`, `if match`, `return`) | a lighter red |
//! | `tk-s` | states (`state`, `Action.Jump`) and Rust's constants | marine green |
//! | `tk-n` | classes: names that start with a capital, and a view's pieces | soft blue |
//! | `tk-o` | objects used directly (`World:say`, `Clock:isNight`) | orange |
//! | `tk-t` | types (`float`, `f64`) and a view's style words | muted |
//! | `tk-v` | text, numbers, `true`, `nothing` | sand |
//! | `tk-c` | comments | green, in italics |
//! | `tk-p` | punctuation | grey |
//!
//! Lust is coloured by [`lust`] (Ledger's own rules); Rust and Lua by
//! [`rust`] and [`lua`], the way Ledger's Settings › Code colours puts
//! other languages in the same colours. Any other code stays plain.
//! Colouring never changes the text: take the spans away and the code is
//! as the manual wrote it ([`text_of`]).

use crate::inline::escape;

#[path = "colour_lust.rs"]
mod lust_rules;

pub use lust_rules::lust;

/// A kind of token, and so its colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tk {
    /// Structure.
    K,
    /// Flow.
    F,
    /// A type.
    T,
    /// A value: text, a number, `true`.
    V,
    /// A comment.
    C,
    /// A class.
    N,
    /// An object.
    O,
    /// Punctuation.
    P,
    /// A state.
    S,
}

impl Tk {
    /// Its class in the guide's style.
    fn class(self) -> &'static str {
        match self {
            Tk::K => "tk-k",
            Tk::F => "tk-f",
            Tk::T => "tk-t",
            Tk::V => "tk-v",
            Tk::C => "tk-c",
            Tk::N => "tk-n",
            Tk::O => "tk-o",
            Tk::P => "tk-p",
            Tk::S => "tk-s",
        }
    }
}

/// Code in the guide: coloured when it's Lust, Rust or Lua, otherwise
/// only made safe.
pub fn code(language: &str, text: &str) -> String {
    match language {
        "lust" => lust(text),
        "rust" => rust(text),
        "lua" => lua(text),
        _ => escape(text),
    }
}

/// The code a coloured block shows: its spans taken away and its text
/// unescaped. For the tests, and for anyone checking the guide.
pub fn text_of(html: &str) -> String {
    let mut plain = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(at) = rest.find('<') {
        plain.push_str(&rest[..at]);
        let end = rest[at..].find('>').map_or(rest.len(), |e| at + e + 1);
        rest = &rest[end..];
    }
    plain.push_str(rest);
    plain
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&amp;", "&")
}

/// Writes coloured code: each token in a span of its class. A token next
/// to one of the same colour, with only spaces between, joins its span,
/// which keeps the page small (`();` is one span, not three).
#[derive(Default)]
struct Painter {
    out: String,
    open: Option<Tk>,
    gap: String,
}

impl Painter {
    fn paint(&mut self, tk: Option<Tk>, text: &str) {
        match tk {
            None if self.open.is_some() && text.bytes().all(|b| b == b' ' || b == b'\t') => {
                self.gap.push_str(text);
            }
            None => {
                self.close();
                self.out.push_str(&escape(text));
            }
            Some(tk) if self.open == Some(tk) => {
                self.out.push_str(&self.gap);
                self.gap.clear();
                self.out.push_str(&escape(text));
            }
            Some(tk) => {
                self.close();
                self.out.push_str("<span class=\"");
                self.out.push_str(tk.class());
                self.out.push_str("\">");
                self.out.push_str(&escape(text));
                self.open = Some(tk);
            }
        }
    }

    fn close(&mut self) {
        if self.open.take().is_some() {
            self.out.push_str("</span>");
        }
        self.out.push_str(&self.gap);
        self.gap.clear();
    }

    fn finish(mut self) -> String {
        self.close();
        self.out
    }
}

/// Colours `text` token by token: `next` says how long the token at the
/// front of what's left is, and its colour. A length of 0 takes one
/// character, so the colouring always moves on.
fn colour_with(text: &str, mut next: impl FnMut(&str) -> (Option<Tk>, usize)) -> String {
    let mut painter = Painter::default();
    let mut rest = text;
    while !rest.is_empty() {
        let (tk, len) = next(rest);
        let len = if len == 0 || !rest.is_char_boundary(len) {
            one_char(rest)
        } else {
            len.min(rest.len())
        };
        painter.paint(tk, &rest[..len]);
        rest = &rest[len..];
    }
    painter.finish()
}

// ---------- the pieces every language is read with ----------

/// The first character's length in bytes.
fn one_char(rest: &str) -> usize {
    rest.chars().next().map_or(0, char::len_utf8)
}

/// Spaces and tabs at the front.
fn blank(rest: &str) -> usize {
    rest.len() - rest.trim_start_matches([' ', '\t']).len()
}

/// A name: a letter or `_`, then letters, digits and `_`.
fn word(rest: &str) -> usize {
    match rest.bytes().next() {
        Some(b) if b.is_ascii_alphabetic() || b == b'_' => rest
            .bytes()
            .position(|b| !(b.is_ascii_alphanumeric() || b == b'_'))
            .unwrap_or(rest.len()),
        _ => 0,
    }
}

/// A number: `42`, `2.5`, `1e-3`, `0x1F`, `2.0f32`, `1_000`.
fn number(rest: &str) -> usize {
    let b = rest.as_bytes();
    if !b.first().is_some_and(u8::is_ascii_digit) {
        return 0;
    }
    let digits = |from: usize| {
        from + b[from..]
            .iter()
            .position(|c| !(c.is_ascii_alphanumeric() || *c == b'_'))
            .unwrap_or(b.len() - from)
    };
    let mut at = digits(0);
    if b.get(at) == Some(&b'.') && b.get(at + 1).is_some_and(u8::is_ascii_digit) {
        at = digits(at + 1);
    }
    let signed = matches!(b.get(at), Some(b'+' | b'-'))
        && matches!(b[at - 1], b'e' | b'E')
        && b.get(at + 1).is_some_and(u8::is_ascii_digit);
    if signed {
        at = digits(at + 1);
    }
    at
}

/// Text in quotes, from the quote at the front to the one that closes it
/// (a `\` keeps the next character), or to the line's end when `one_line`
/// and it isn't closed, or to the end.
fn quoted(rest: &str, one_line: bool) -> usize {
    let mut chars = rest.char_indices();
    let Some((_, quote)) = chars.next() else {
        return 0;
    };
    while let Some((at, c)) = chars.next() {
        match c {
            '\\' => {
                if chars.next().is_some_and(|(_, c)| c == '\n' && one_line) {
                    return at + 1;
                }
            }
            '\n' if one_line => return at,
            c if c == quote => return at + c.len_utf8(),
            _ => {}
        }
    }
    rest.len()
}

/// To the line's end.
fn line(rest: &str) -> usize {
    rest.find('\n').unwrap_or(rest.len())
}

/// From an opening of `skip` bytes through `close`, or to the end.
fn through(rest: &str, skip: usize, close: &str) -> usize {
    rest.get(skip..)
        .and_then(|after| after.find(close))
        .map_or(rest.len(), |at| skip + at + close.len())
}

/// The first character after any spaces.
fn ahead(rest: &str) -> Option<char> {
    rest.trim_start_matches([' ', '\t']).chars().next()
}

/// Whether a name starts with a capital.
fn capital(word: &str) -> bool {
    word.starts_with(|c: char| c.is_ascii_uppercase())
}

/// Plain punctuation: grey.
const PUNCTUATION: &[u8] = b";,{}[]():.";

// ---------- Rust ----------

/// Rust's words that give code its shape.
const RUST_STRUCTURE: &[&str] = &[
    "as", "async", "const", "crate", "dyn", "enum", "extern", "fn", "impl", "let", "mod", "move",
    "mut", "pub", "ref", "static", "struct", "super", "trait", "type", "unsafe", "use", "where",
];
/// Rust's flow, and `self` (Ledger gives it the flow colour).
const RUST_FLOW: &[&str] = &[
    "await", "break", "continue", "else", "for", "if", "in", "loop", "match", "return", "while",
    "self", "Self",
];
/// Rust's own types.
const RUST_TYPES: &[&str] = &[
    "bool", "char", "str", "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64",
    "i128", "isize", "f32", "f64",
];

/// Rust in the quiet colours: keywords as structure or flow, macros
/// (`vec!`) as flow, types, constants (`MAX_SPEED`) as states, names with
/// a capital as classes, attributes (`#[derive]`) muted like types,
/// lifetimes (`'_`) like values.
pub fn rust(text: &str) -> String {
    colour_with(text, rust_token)
}

fn rust_token(rest: &str) -> (Option<Tk>, usize) {
    let b = rest.as_bytes();
    if rest.starts_with("//") {
        return (Some(Tk::C), line(rest));
    }
    if rest.starts_with("/*") {
        return (Some(Tk::C), through(rest, 2, "*/"));
    }
    if rest.starts_with("::") || rest.starts_with("->") || rest.starts_with("=>") {
        return (Some(Tk::P), 2);
    }
    if rest.starts_with("#[") || rest.starts_with("#![") {
        return (Some(Tk::T), attribute(rest));
    }
    match b[0] {
        b'"' => return (Some(Tk::V), quoted(rest, false)),
        b'\'' => return character_or_lifetime(rest),
        b'?' => return (Some(Tk::F), 1),
        c if PUNCTUATION.contains(&c) => return (Some(Tk::P), 1),
        _ => {}
    }
    let n = number(rest);
    if n > 0 {
        return (Some(Tk::V), n);
    }
    let n = word(rest);
    if n == 0 {
        return (None, blank(rest));
    }
    let w = &rest[..n];
    let after = &rest[n..];
    if after.starts_with('!') && !after.starts_with("!=") {
        return (Some(Tk::F), n + 1);
    }
    let tk = if RUST_STRUCTURE.contains(&w) {
        Some(Tk::K)
    } else if RUST_FLOW.contains(&w) {
        Some(Tk::F)
    } else if RUST_TYPES.contains(&w) {
        Some(Tk::T)
    } else if w == "true" || w == "false" {
        Some(Tk::V)
    } else if n > 1
        && w.bytes()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == b'_')
        && capital(w)
    {
        Some(Tk::S)
    } else if capital(w) {
        Some(Tk::N)
    } else {
        None
    };
    (tk, n)
}

/// `#[derive(Debug)]`: to its closing `]`, on its line.
fn attribute(rest: &str) -> usize {
    let mut depth = 0;
    for (at, c) in rest.char_indices() {
        match c {
            '[' => depth += 1,
            ']' if depth <= 1 => return at + 1,
            ']' => depth -= 1,
            '\n' => return at,
            _ => {}
        }
    }
    rest.len()
}

/// `'x'` and `'\n'` are characters; `'a` and `'_` are lifetimes.
fn character_or_lifetime(rest: &str) -> (Option<Tk>, usize) {
    let mut chars = rest[1..].chars();
    match chars.next() {
        Some('\\') => {
            let skip = 2 + chars.next().map_or(0, char::len_utf8);
            let close = rest
                .get(skip..line(rest))
                .and_then(|after| after.find('\''))
                .map_or(2, |at| skip + at + 1);
            (Some(Tk::V), close)
        }
        Some(c) if chars.next() == Some('\'') => (Some(Tk::V), 1 + c.len_utf8() + 1),
        _ => match word(&rest[1..]) {
            0 => (None, 1),
            n => (Some(Tk::V), 1 + n),
        },
    }
}

// ---------- Lua ----------

/// Lua's words that give code its shape.
const LUA_STRUCTURE: &[&str] = &["local", "function", "end"];
/// Lua's flow, and `self`.
const LUA_FLOW: &[&str] = &[
    "and", "break", "do", "else", "elseif", "for", "goto", "if", "in", "not", "or", "repeat",
    "return", "then", "until", "while", "self",
];

/// Lua in the quiet colours: keywords as structure or flow, text (`"…"`,
/// `'…'`, `[[…]]`) and numbers as values, `--` and `--[[ ]]` comments,
/// names with a capital as classes.
pub fn lua(text: &str) -> String {
    colour_with(text, lua_token)
}

fn lua_token(rest: &str) -> (Option<Tk>, usize) {
    if rest.starts_with("--[[") {
        return (Some(Tk::C), through(rest, 4, "]]"));
    }
    if rest.starts_with("--") {
        return (Some(Tk::C), line(rest));
    }
    if rest.starts_with("[[") {
        return (Some(Tk::V), through(rest, 2, "]]"));
    }
    // `..` joins text and `...` is the rest of a function's values:
    // operators, not punctuation.
    if rest.starts_with("..") {
        return (None, if rest.starts_with("...") { 3 } else { 2 });
    }
    match rest.as_bytes()[0] {
        b'"' | b'\'' => return (Some(Tk::V), quoted(rest, true)),
        c if PUNCTUATION.contains(&c) => return (Some(Tk::P), 1),
        _ => {}
    }
    let n = number(rest);
    if n > 0 {
        return (Some(Tk::V), n);
    }
    let n = word(rest);
    if n == 0 {
        return (None, blank(rest));
    }
    let w = &rest[..n];
    let tk = if LUA_STRUCTURE.contains(&w) {
        Some(Tk::K)
    } else if LUA_FLOW.contains(&w) {
        Some(Tk::F)
    } else if ["true", "false", "nil"].contains(&w) {
        Some(Tk::V)
    } else if capital(w) {
        Some(Tk::N)
    } else {
        None
    };
    (tk, n)
}

#[cfg(test)]
#[path = "colour_tests.rs"]
mod tests;
