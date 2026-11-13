//! Guards for text on its way to the Lust compiler. Lust (2.0.0-alpha.67,
//! rev ee12407, as at rev 930afae) never finishes checking a file that
//! declares two things with one name, and a long run of `!` takes it
//! time that grows with the square of the run's length. So game code (`script::code`) and data files
//! ([`crate::lust_data`]) are looked over here first, and what would set
//! either off is refused as an ordinary mistake. A data file is then
//! checked piece by piece against what one may hold
//! (`crate::lust_check`, with the same `Pieces`).
//!
//! The text is split as Lust's own reader splits it: comments, text,
//! numbers, `lua { }` blocks and the characters Lust skips each end where
//! Lust ends them, so nothing Lust would read can be hidden from a guard.
//! The one exception is text holding a value (`"hp: {hp}"`): Lust finds
//! its end by reading the value, which costs it the square of the line's
//! length, so here it ends at its next quote and is reported
//! ([`Found::Value`]). A data file never holds one, and is refused; for
//! game code, which may, `script::code::COMPILE_LIMIT` stands behind the
//! guards.

/// The longest run of `!` (as in `!!!x`) handed to Lust: longer than any
/// real code has.
pub const MAX_PREFIX_RUN: usize = 64;

/// The words that declare something Lust orders by name before checking
/// it: two of one name are what it never finishes.
const DECLARING: [&str; 6] = ["class", "object", "component", "node", "schema", "assembly"];

/// Something a guard looks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Found<'a> {
    /// A declaring word straight before a name (`component Health`): the
    /// word, and the name.
    Declared(&'static str, &'a str),
    /// A run of `!` that has just passed [`MAX_PREFIX_RUN`].
    LongRun,
    /// Text holding a value in `{ }`.
    Value,
}

/// Everything the guards look for in `text`, in order, each with its line
/// (from 1). One pass, in time that grows only with the text's length.
pub fn scan(text: &str) -> impl Iterator<Item = (u32, Found<'_>)> {
    let mut pieces = Pieces::new(text);
    let mut declaring = None;
    let mut run = 0usize;
    std::iter::from_fn(move || {
        loop {
            let (line, piece) = pieces.next()?;
            run = if piece == Piece::Stray('!') {
                run + 1
            } else {
                0
            };
            match piece {
                Piece::Word(word) => {
                    // A name that is itself a declaring word is read both
                    // ways, as Lust may once it has given up on a line.
                    let word_is = DECLARING.iter().copied().find(|d| *d == word);
                    if let Some(kind) = std::mem::replace(&mut declaring, word_is) {
                        return Some((line, Found::Declared(kind, word)));
                    }
                }
                Piece::Stray('!') if run == MAX_PREFIX_RUN + 1 => {
                    return Some((line, Found::LongRun));
                }
                // Lust skips these, so a word and its name still meet.
                Piece::Stray(_) => {}
                Piece::Text { value, .. } => {
                    declaring = None;
                    if value {
                        return Some((line, Found::Value));
                    }
                }
                Piece::Number(_) | Piece::Symbol(_) => declaring = None,
            }
        }
    })
}

/// A piece of Lust text, as Lust's reader splits it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Piece<'a> {
    /// A name or one of Lust's words. A `lua { }` block after the word
    /// `lua` is stepped over with it.
    Word(&'a str),
    /// Text in quotes: what's between them as written, whether a `{` in it
    /// starts a value, and whether its closing quote is there.
    Text {
        inside: &'a str,
        value: bool,
        closed: bool,
    },
    /// A number, as written.
    Number(&'a str),
    /// A symbol Lust reads (`;`, `=`, `!=`). Lust joins a few more pairs
    /// into one (`==`, `-=`), which come here as two: no guard needs them
    /// joined.
    Symbol(&'a str),
    /// A character Lust has no use for, which it reports and skips.
    Stray(char),
}

/// Lust text, a piece at a time, each with the line it starts on.
pub(crate) struct Pieces<'a> {
    rest: &'a str,
    line: u32,
}

impl<'a> Pieces<'a> {
    /// The pieces of `text`, from its first line.
    pub(crate) fn new(text: &'a str) -> Self {
        Self {
            rest: text,
            line: 1,
        }
    }

    /// The line the next piece is looked for from (the text's last, once
    /// there are no more).
    pub(crate) fn line(&self) -> u32 {
        self.line
    }

    /// The next `bytes` of the text, stepped over with their line breaks
    /// counted.
    fn take(&mut self, bytes: usize) -> &'a str {
        let (taken, rest) = self.rest.split_at(bytes);
        for byte in taken.bytes() {
            if byte == b'\n' {
                self.line += 1;
            }
        }
        self.rest = rest;
        taken
    }

    /// The next piece and the line it starts on, past blank space and
    /// comments.
    pub(crate) fn next(&mut self) -> Option<(u32, Piece<'a>)> {
        loop {
            let first = self.rest.chars().next()?;
            let line = self.line;
            if first.is_whitespace() {
                self.take(first.len_utf8());
            } else if let Some(inside) = self.rest.strip_prefix("--[[") {
                // A block comment, to its `]]` or the end of the text.
                let end = inside.find("]]").map_or(self.rest.len(), |at| at + 6);
                self.take(end);
            } else if self.rest.starts_with("--") || self.rest.starts_with("//") {
                self.take(self.rest.find('\n').unwrap_or(self.rest.len()));
            } else if self.rest.starts_with("/*") {
                self.take(2);
                return Some((line, Piece::Stray('/')));
            } else if first.is_alphabetic() || first == '_' {
                let end = self
                    .rest
                    .find(|c: char| !c.is_alphanumeric() && c != '_')
                    .unwrap_or(self.rest.len());
                let word = self.take(end);
                if word == "lua" {
                    self.lua_block();
                }
                return Some((line, Piece::Word(word)));
            } else if first.is_ascii_digit() {
                let number = self.take(number_len(self.rest));
                return Some((line, Piece::Number(number)));
            } else if first == '"' {
                return Some((line, self.text()));
            } else {
                let symbol = match first {
                    ';' | ',' | '.' | ':' | '(' | ')' | '{' | '}' | '[' | ']' | '?' | '#' | '+'
                    | '-' | '*' | '/' | '%' | '<' | '>' | '=' => 1,
                    '!' if self.rest.starts_with("!=") => 2,
                    '&' if self.rest.starts_with("&&") => 2,
                    '|' if self.rest.starts_with("||") => 2,
                    _ => {
                        self.take(first.len_utf8());
                        return Some((line, Piece::Stray(first)));
                    }
                };
                let symbol = self.take(symbol);
                return Some((line, Piece::Symbol(symbol)));
            }
        }
    }

    /// Steps over `{ … }` after the word `lua`: everything to the brace
    /// that closes it is Lua, which Lust keeps as it is. Left open, it
    /// takes the rest of the text, and Lust reads nothing after it.
    fn lua_block(&mut self) {
        let blank = self
            .rest
            .find(|c| !matches!(c, ' ' | '\t' | '\r' | '\n'))
            .unwrap_or(self.rest.len());
        if !self.rest[blank..].starts_with('{') {
            return;
        }
        let mut depth = 0usize;
        let close = self.rest[blank..].bytes().position(|byte| {
            match byte {
                b'{' => depth += 1,
                b'}' => depth -= 1,
                _ => {}
            }
            depth == 0
        });
        self.take(close.map_or(self.rest.len(), |at| blank + at + 1));
    }

    /// Steps over text from its opening quote to its closing one or the
    /// end of its line, and gives it as a piece. A backslash takes the next
    /// character with it, a line break too.
    fn text(&mut self) -> Piece<'a> {
        self.take(1);
        let start = self.rest;
        let mut value = false;
        loop {
            let plain = self.rest.find(['"', '\\', '{', '\n']);
            self.take(plain.unwrap_or(self.rest.len()));
            let inside = &start[..start.len() - self.rest.len()];
            let mut chars = self.rest.chars();
            let closed = match chars.next() {
                Some('"') => {
                    self.take(1);
                    true
                }
                // Lust reads a file with its `\r\n` line breaks made `\n`.
                Some('\\') if self.rest[1..].starts_with("\r\n") => {
                    self.take(3);
                    continue;
                }
                Some('\\') => {
                    self.take(1 + chars.next().map_or(0, char::len_utf8));
                    continue;
                }
                Some('{') => {
                    value |= chars
                        .next()
                        .is_some_and(|c| c.is_alphabetic() || matches!(c, '(' | '#' | '"'));
                    self.take(1);
                    continue;
                }
                _ => false,
            };
            return Piece::Text {
                inside,
                value,
                closed,
            };
        }
    }
}

/// How many bytes of `text`, which starts with a digit, are one number to
/// Lust: digits, a `.` with digits after it, and a power of ten (`1e30`,
/// `1.5e-7`). A letter straight after starts a word of its own.
fn number_len(text: &str) -> usize {
    let bytes = text.as_bytes();
    let digit = |at: usize| bytes.get(at).is_some_and(u8::is_ascii_digit);
    let digits = |mut at: usize| {
        while digit(at) {
            at += 1;
        }
        at
    };
    let mut end = digits(0);
    if bytes.get(end) == Some(&b'.') && digit(end + 1) {
        end = digits(end + 1);
    }
    if matches!(bytes.get(end), Some(b'e' | b'E')) {
        let signed = matches!(bytes.get(end + 1), Some(b'+' | b'-'));
        if digit(end + 1 + usize::from(signed)) {
            end = digits(end + 1 + usize::from(signed));
        }
    }
    end
}

#[cfg(test)]
mod tests {
    use super::*;

    fn declared(text: &str) -> Vec<(u32, &'static str, &str)> {
        scan(text)
            .filter_map(|(line, found)| match found {
                Found::Declared(word, name) => Some((line, word, name)),
                _ => None,
            })
            .collect()
    }

    fn long_runs(text: &str) -> Vec<u32> {
        scan(text)
            .filter(|(_, found)| *found == Found::LongRun)
            .map(|(line, _)| line)
            .collect()
    }

    fn values(text: &str) -> usize {
        scan(text).filter(|(_, f)| *f == Found::Value).count()
    }

    #[test]
    fn declarations_are_found_with_their_word_name_and_line() {
        // After a string with an escaped quote; across a line break between
        // the word and its name; not after a comment.
        assert_eq!(
            declared(
                "s = \"\\\"component Fake\";\ncomponent\n  Real {}\n-- component Gone\nclass Kept {}\n"
            ),
            [(3, "component", "Real"), (5, "class", "Kept")]
        );
        // A word that only starts like one isn't a declaration.
        assert!(declared("components X {}\nmy_class Y {}\nclassy Z {}\n").is_empty());
        for word in DECLARING {
            assert_eq!(declared(&format!("{word} A {{}}")), [(1, word, "A")]);
        }
        // Words that declare other things aren't ordered by name.
        assert!(declared("state A = [X];\nmethod a() {}\nstyle a = [];\n").is_empty());
        // A name that's a declaring word is tried as one too.
        assert_eq!(
            declared("x = class\nclass A {}"),
            [(2, "class", "class"), (2, "class", "A")]
        );
        assert_eq!(declared("class"), []);
    }

    #[test]
    fn nothing_lust_reads_is_hidden_from_the_guards() {
        // Each of these left the older scanners blind to what follows while
        // Lust went on to read it.
        for hidden in [
            "--[[ a block comment ]] class A {}",
            "--[[\nover\nlines ]] class A {}",
            "--[[]]class A {}",
            "// \"\\\nclass A {}",
            "lua { \" } class A {}",
            "lua\n\t{ { } \" } class A {}",
            "' class A {}",
            "/* class A {}",
            "s = \"a\\\r\nb\"; class A {}",
        ] {
            let found = declared(hidden);
            assert_eq!(found.len(), 1, "{hidden:?}: {found:?}");
            assert_eq!((found[0].1, found[0].2), ("class", "A"), "{hidden:?}");
        }
        // Characters Lust skips don't come between a word and its name;
        // comments and blank space don't either.
        for between in [
            "@",
            "'",
            "!",
            "$ ~ ^ `",
            "&",
            "|",
            "/*",
            "\u{a0}",
            "-- c\n",
            "--[[ c ]]",
        ] {
            let text = format!("class {between} A {{}}");
            assert_eq!(declared(&text).len(), 1, "{text:?}");
        }
        // Anything Lust reads as a piece of its own does.
        for between in [
            "=", "\"x\"", "5", "!=", "&&", "||", ".", "-", "/", "lua {}", "x",
        ] {
            let text = format!("class {between} A {{}}");
            assert!(
                declared(&text).iter().all(|(_, _, name)| *name != "A"),
                "{text:?}"
            );
        }
        // A number ends where Lust ends it, and a word starts straight after.
        for number in ["1", "12.5", "1e5", "1E+5", "1.5e-7", "3."] {
            let text = format!("{number}class A {{}}");
            assert_eq!(declared(&text), [(1, "class", "A")], "{text:?}");
        }
        // No digit after the e: it's a letter, and starts the word.
        for number in ["1e", "2.5E"] {
            let text = format!("{number}class A {{}}");
            assert!(declared(&text).is_empty(), "{text:?}");
        }
        assert_eq!(declared("1.5e+class A {}"), [(1, "class", "A")]);
    }

    #[test]
    fn comments_text_and_lua_blocks_hold_no_declarations() {
        for text in [
            "-- class A {}",
            "// class A {}",
            "--[[ class A {}\nclass A {} ]]",
            "--[[ never closed\nclass A {}",
            "s = \"class A {}\";",
            "s = \"an escaped \\\" quote, class A {}\";",
            "s = \"an escaped line break \\\nclass A {}\";",
            "lua { class A {} }",
            "lua { { never closed }\nclass A {}",
        ] {
            assert!(declared(text).is_empty(), "{text:?}");
        }
        // Text left open ends with its line; lines are counted through all
        // of it.
        assert_eq!(
            declared("s = \"open\nclass A {}\n--[[\n\n]] lua {\n}\n\"\\\n\" class B {}"),
            [(2, "class", "A"), (8, "class", "B")]
        );
    }

    #[test]
    fn a_long_run_is_found_once_just_past_the_limit() {
        let run = |n: usize| "!".repeat(n);
        assert!(long_runs(&run(MAX_PREFIX_RUN)).is_empty());
        assert_eq!(long_runs(&run(MAX_PREFIX_RUN + 1)), [1]);
        assert_eq!(long_runs(&format!("\n\n{}", run(1000))), [3]);
        // Blank space and comments don't break a run; anything else does.
        let spaced = "! ".repeat(MAX_PREFIX_RUN) + "-- more\n!";
        assert_eq!(long_runs(&spaced), [2]);
        for breaking in ["x", "=", "1", "\"\"", "@", "'"] {
            let text = format!("{}{breaking}{}", run(MAX_PREFIX_RUN), run(MAX_PREFIX_RUN));
            assert!(long_runs(&text).is_empty(), "{text:?}");
        }
        // In text and comments they're only letters.
        assert!(long_runs(&format!("s = \"{0}\"; -- {0}\n--[[ {0} ]]", run(200))).is_empty());
        // Two runs are two.
        assert_eq!(
            long_runs(&format!("{0}\nx\n{0}", run(MAX_PREFIX_RUN + 5))),
            [1, 3]
        );
    }

    #[test]
    fn text_holding_a_value_is_found() {
        for holding in [
            "\"hp: {hp}\"",
            "\"{(a)}\"",
            "\"{#list}\"",
            "\"{\"x\"}\"",
            "\"{é}\"",
            "\"never closed {a",
            "\"{\"",
        ] {
            assert_eq!(values(holding), 1, "{holding:?}");
        }
        for plain in [
            "\"\\{hp\\}\"",
            "\"{ hp }\"",
            "\"[0-9]{4}\"",
            "\"{}\"",
            "\"{_a}\"",
            "\"{",
            "{hp}",
            "-- \"{hp}\"",
        ] {
            assert_eq!(values(plain), 0, "{plain:?}");
        }
        assert_eq!(
            scan("\n\"{a} {b}\" \"{c}\"")
                .map(|(l, _)| l)
                .collect::<Vec<_>>(),
            [2, 2]
        );
    }
}
