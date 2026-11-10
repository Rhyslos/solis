//! Checks a data file before Lust does. Lust (rev 930afae) has no budget:
//! each mistake it finds is compared with every one before it, and a name
//! it can't find with every name it knows, so a file with thousands of
//! mistakes takes it time that grows with their square (200 KB of `@`: 43
//! seconds; 25,000 values of something unknown: more than a minute). A
//! data file comes with someone else's project, and holds only what
//! [`crate::lust_data`] writes: a `fetch` of its classes, `lock` values
//! made of numbers, text, `true` and `false`, a state's options, lists and
//! `new` things, and `end`. So that is all that's let through. Every piece
//! of the file is checked here first against the file's own classes, in
//! one pass, and anything else is refused with its line.
//!
//! What reaches Lust is then a file it reads in time that grows only with
//! the file's length, and finds nothing wrong with, but for an `end` left
//! out or a block comment left open (a mistake or two, which it says in
//! its own words). The tests sweep thousands of muddled files through
//! both to check exactly that.
//!
//! Reading even a file with nothing wrong costs Lust memory and time for
//! each piece of it, far more than the file's own length, so a file with
//! more pieces than [`MAX_PIECES`] is refused here too.
//!
//! The file is split as Lust splits it ([`crate::lust_guard::Pieces`]), and
//! nothing is let through that Lust might read another way. What Lust
//! allows and Solis never writes is refused too: `new Item` without its
//! `{ }`, a `;` between a thing's fields, `nothing`.

use crate::lust_data::{CLASSES_FOLDER, DataError, ENGINE_FOLDER, error};
use crate::lust_guard::{Piece, Pieces};
use std::fmt::Write as _;

/// The most `fetch` lines and values one data file has: far more than any
/// kind of file Solis writes (a tile map has ten). Each value may be a
/// list as long as the file's size allows.
pub(crate) const MAX_VALUES: usize = 64;

/// How deep lists and things go inside each other: deeper than any class
/// asks for, and well inside what Lust reads.
pub(crate) const MAX_DEPTH: usize = 32;

/// The most pieces (numbers, texts, names and symbols) one data file has.
/// A file's length says little of what Lust needs to read it: Lust (rev
/// 930afae) keeps about 100 bytes for each piece while it reads, and takes
/// over half a second for a million of them, and a megabyte is 65,000
/// pieces of a tile map's rows of codes or a million of `[1,1,1],`
/// (measured: 12 MB of those took 1.3 GB and 7.5 seconds; 60 MB of rows,
/// 1 GB and 4 seconds). This many keeps the biggest file read (a tile
/// map's 64 MB), at its worst, under 2 GB and 10 seconds.
pub(crate) const MAX_PIECES: usize = 12_000_000;

/// The most fields a class has (each is filled in once, kept track of in
/// one number).
const MAX_FIELDS: usize = 64;

/// How many characters of a word or number a mistake about it shows.
const SHOWN: usize = 40;

/// How many characters of Lust's own words about a name are passed on.
const LONGEST: usize = 200;

/// The mistake for anything at a file's top level that isn't a `fetch`
/// line, a `lock` value or `end`.
const ONLY_VALUES: &str = "a data file holds only lock values (lock int version = 1;), after \
                           its fetch line and before end";

/// What one value is, before any `array`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind<'a> {
    Int,
    Float,
    Text,
    Bool,
    /// A state or a class, by its name.
    Named(&'a str),
}

/// A value's type as a class writes it: `int`, `string array`,
/// `int array array`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Type<'a> {
    kind: Kind<'a>,
    /// How many times `array` follows it.
    lists: usize,
}

/// What a kind of data file may hold, read from its classes (the Lust text
/// its format keeps and hands to Lust).
pub(crate) struct Classes<'a> {
    /// What a file fetches them from: `solis/data/tilemap`.
    path: String,
    /// Each state and its options.
    states: Vec<(&'a str, Vec<&'a str>)>,
    /// Each class and its fields.
    classes: Vec<(&'a str, Vec<(&'a str, Type<'a>)>)>,
    /// The names a file can fetch.
    passed: Vec<&'a str>,
}

/// A file's pieces, one at a time, with the line of the last one read.
struct Reader<'a> {
    pieces: Pieces<'a>,
    ahead: Option<(u32, Piece<'a>)>,
    line: u32,
    /// How many pieces have been read.
    taken: usize,
}

impl<'a> Reader<'a> {
    /// The pieces of `text`, from its first.
    fn new(text: &'a str) -> Self {
        let mut pieces = Pieces::new(text);
        let ahead = pieces.next();
        Self {
            pieces,
            ahead,
            line: 1,
            taken: 0,
        }
    }

    /// The next piece, left where it is.
    fn peek(&self) -> Option<Piece<'a>> {
        self.ahead.map(|(_, piece)| piece)
    }

    /// The next piece, or none at the end of the file.
    fn take(&mut self) -> Option<Piece<'a>> {
        let taken = std::mem::replace(&mut self.ahead, self.pieces.next());
        self.line = taken.map_or(self.pieces.line(), |(line, _)| line);
        self.taken += usize::from(taken.is_some());
        taken.map(|(_, piece)| piece)
    }

    /// A mistake on the line of the piece just read.
    fn mistake(&self, message: impl Into<String>) -> DataError {
        error(self.line as usize, message)
    }

    /// The mistake of finding `found` where `wanted` goes.
    fn wrong(&self, wanted: &str, found: Option<Piece<'_>>) -> DataError {
        let found = match found {
            None => "the end of the file".to_string(),
            Some(Piece::Word(word)) => format!("the word {}", clip(word)),
            Some(Piece::Text { value: true, .. }) => "text with a value in { }".to_string(),
            Some(Piece::Text { closed: false, .. }) => "text that's never closed".to_string(),
            Some(Piece::Text { .. }) => "text".to_string(),
            Some(Piece::Number(number)) => clip(number),
            Some(Piece::Symbol(symbol)) => format!("`{symbol}`"),
            Some(Piece::Stray(stray)) => format!("`{stray}`"),
        };
        self.mistake(format!("{wanted} goes here, not {found}"))
    }

    /// Reads the symbol `symbol`.
    fn expect(&mut self, symbol: &str) -> Result<(), DataError> {
        match self.take() {
            Some(Piece::Symbol(found)) if found == symbol => Ok(()),
            found => Err(self.wrong(&format!("`{symbol}`"), found)),
        }
    }

    /// Reads a word, which is `wanted`.
    fn word(&mut self, wanted: &str) -> Result<&'a str, DataError> {
        match self.take() {
            Some(Piece::Word(word)) => Ok(word),
            found => Err(self.wrong(wanted, found)),
        }
    }

    /// Reads a type: `int`, `Layer array`.
    fn ty(&mut self) -> Result<Type<'a>, DataError> {
        let kind = match self.word("a type, like int or string")? {
            "int" => Kind::Int,
            "float" => Kind::Float,
            "string" => Kind::Text,
            "bool" => Kind::Bool,
            name => Kind::Named(name),
        };
        let mut lists = 0;
        while self.peek() == Some(Piece::Word("array")) {
            self.take();
            lists += 1;
        }
        Ok(Type { kind, lists })
    }

    /// Reads the items after a `[` or `{` up to its `close`: each read by
    /// `item`, with a `,` between them, and one after the last if it likes
    /// (as Lust reads a list, a thing's fields and a fetch's names). All but
    /// a few of a file's pieces are read here, so a file with more than
    /// [`MAX_PIECES`] is refused here, without reading the rest of it.
    fn items(
        &mut self,
        close: &str,
        mut item: impl FnMut(&mut Self) -> Result<(), DataError>,
    ) -> Result<(), DataError> {
        loop {
            if self.taken > MAX_PIECES {
                return Err(error(0, too_big()));
            }
            if self.peek() == Some(Piece::Symbol(close)) {
                self.take();
                return Ok(());
            }
            item(self)?;
            match self.take() {
                Some(Piece::Symbol(",")) => {}
                Some(Piece::Symbol(found)) if found == close => return Ok(()),
                found => return Err(self.wrong(&format!("`,` or `{close}`"), found)),
            }
        }
    }
}

/// The start of a word or number, for a mistake about it.
fn clip(text: &str) -> String {
    match text.char_indices().nth(SHOWN) {
        Some((end, _)) => format!("{}…", &text[..end]),
        None => text.to_string(),
    }
}

/// Checks that each `\` in a text is one of the six Lust knows, which are
/// the six [`crate::lust_data::text`] writes.
fn escapes(inside: &str) -> Result<(), String> {
    let mut rest = inside;
    while let Some(at) = rest.find('\\') {
        let mut after = rest[at + 1..].chars();
        match after.next() {
            Some('n' | 't' | '"' | '\\' | '{' | '}') => rest = after.as_str(),
            _ => {
                return Err(
                    "a \\ in text starts one of \\n, \\t, \\\", \\\\, \\{ and \\}: \
                            write \\\\ for a plain \\"
                        .to_string(),
                );
            }
        }
    }
    Ok(())
}

impl<'a> Classes<'a> {
    /// Reads the classes of a `kind` of file from their Lust text: its
    /// states (`state Kind = [A, B];`), its classes (each a `has` block of
    /// fields with a type and a starting value) and what it passes. They're
    /// Solis's own, so a mistake here is one in Solis.
    pub(crate) fn read(kind: &str, text: &'a str) -> Result<Self, DataError> {
        let mut out = Self {
            path: format!("{ENGINE_FOLDER}/{CLASSES_FOLDER}/{kind}"),
            states: Vec::new(),
            classes: Vec::new(),
            passed: Vec::new(),
        };
        out.declarations(&mut Reader::new(text)).map_err(|why| {
            error(
                0,
                format!("its classes ({}.lst) can't be read: {why}", out.path),
            )
        })?;
        Ok(out)
    }

    /// Reads every state, class and `pass` line up to `end`.
    fn declarations(&mut self, r: &mut Reader<'a>) -> Result<(), DataError> {
        loop {
            match r.word("state, class, pass or end")? {
                "state" => {
                    let name = r.word("the state's name")?;
                    r.expect("=")?;
                    r.expect("[")?;
                    let mut options = Vec::new();
                    r.items("]", |r| {
                        options.push(r.word("an option")?);
                        Ok(())
                    })?;
                    r.expect(";")?;
                    self.states.push((name, options));
                }
                "class" => {
                    let name = r.word("the class's name")?;
                    r.expect("{")?;
                    if r.word("has")? != "has" {
                        return Err(r.mistake("a data file's class holds only a has block"));
                    }
                    r.expect("{")?;
                    let mut fields = Vec::new();
                    while r.peek() != Some(Piece::Symbol("}")) {
                        let ty = r.ty()?;
                        fields.push((r.word("the field's name")?, ty));
                        r.expect("=")?;
                        // Its starting value, which only Lust needs.
                        while r.peek().is_some_and(|piece| piece != Piece::Symbol(";")) {
                            r.take();
                        }
                        r.expect(";")?;
                    }
                    r.expect("}")?;
                    r.expect("}")?;
                    if fields.len() > MAX_FIELDS {
                        return Err(r.mistake(format!("{name} has more than {MAX_FIELDS} fields")));
                    }
                    self.classes.push((name, fields));
                }
                "pass" => {
                    self.passed.push(r.word("a name to pass")?);
                    while r.peek() == Some(Piece::Symbol(",")) {
                        r.take();
                        self.passed.push(r.word("a name to pass")?);
                    }
                    r.expect(";")?;
                }
                "end" => break,
                _ => return Err(r.mistake("a data file's classes are states and classes")),
            }
        }
        let fields = self.classes.iter().flat_map(|(_, fields)| fields);
        for (field, ty) in fields {
            if matches!(ty.kind, Kind::Named(name) if !self.declares(name)) {
                return Err(error(
                    0,
                    format!("{field}'s type isn't one of its states or classes"),
                ));
            }
        }
        Ok(())
    }

    /// The options of the state called `name`.
    fn state(&self, name: &str) -> Option<&[&'a str]> {
        let (_, options) = self.states.iter().find(|(state, _)| *state == name)?;
        Some(options)
    }

    /// The fields of the class called `name`.
    fn class(&self, name: &str) -> Option<&[(&'a str, Type<'a>)]> {
        let (_, fields) = self.classes.iter().find(|(class, _)| *class == name)?;
        Some(fields)
    }

    /// Whether `name` is one of these states and classes.
    fn declares(&self, name: &str) -> bool {
        self.state(name).is_some() || self.class(name).is_some()
    }

    /// Checks that `text` is a data file of these classes and nothing more
    /// (see the top of this file), or says what on which line isn't. One
    /// pass, in time that grows only with the text's length.
    pub(crate) fn check(&self, text: &str) -> Result<(), DataError> {
        let mut r = Reader::new(text);
        // The states and classes the file has fetched, each once.
        let mut fetched: Vec<&str> = Vec::new();
        let mut fetches = 0;
        // Each value's name, with its line.
        let mut names: Vec<(u32, &str)> = Vec::new();
        loop {
            match r.take() {
                // Without its `end`, Lust says so.
                None => break,
                Some(Piece::Word("end")) => {
                    if r.take().is_some() {
                        return Err(r.mistake("nothing can come after end"));
                    }
                    break;
                }
                Some(_) if fetches + names.len() == MAX_VALUES => {
                    return Err(r.mistake(format!(
                        "more than {MAX_VALUES} fetch lines and values, which no data file has"
                    )));
                }
                Some(Piece::Word("fetch")) if names.is_empty() => {
                    self.fetch(&mut r, &mut fetched)?;
                    fetches += 1;
                }
                Some(Piece::Word("lock")) => {
                    let name = self.lock(&mut r, &fetched)?;
                    if names.iter().any(|(_, earlier)| *earlier == name.1) {
                        let twice = format!("a second value called {}", clip(name.1));
                        return Err(error(name.0 as usize, twice));
                    }
                    names.push(name);
                }
                Some(_) => return Err(r.mistake(ONLY_VALUES)),
            }
        }
        names_fit(&names)
    }

    /// The rest of a `fetch` line: these classes' path, and names they pass.
    fn fetch<'b>(&self, r: &mut Reader<'b>, fetched: &mut Vec<&'b str>) -> Result<(), DataError> {
        let path = &self.path;
        match r.take() {
            Some(Piece::Text {
                inside,
                value: false,
                closed: true,
            }) if inside == path => {}
            _ => {
                return Err(r.mistake(format!(
                    "a data file fetches only its own classes: fetch \"{path}\":[…];"
                )));
            }
        }
        r.expect(":")?;
        let mut name = |r: &mut Reader<'b>| {
            let name = r.word("a name to fetch")?;
            if !self.passed.contains(&name) || !self.declares(name) {
                return Err(r.mistake(format!("{path} has no {}", clip(name))));
            }
            if !fetched.contains(&name) {
                fetched.push(name);
            }
            Ok(())
        };
        if r.peek() == Some(Piece::Symbol("[")) {
            r.take();
            r.items("]", name)?;
        } else {
            name(r)?;
        }
        r.expect(";")
    }

    /// The rest of a `lock` value: its type, its name, and a value of that
    /// type. Gives the name, with its line.
    fn lock<'b>(&self, r: &mut Reader<'b>, fetched: &[&str]) -> Result<(u32, &'b str), DataError> {
        let ty = r.ty()?;
        if ty.lists > MAX_DEPTH {
            return Err(r.mistake(too_deep()));
        }
        if let Kind::Named(name) = ty.kind {
            self.fetched(r, name, fetched)?;
        }
        let name = r.word("the value's name")?;
        let name = (r.line, name);
        // Lust would take the name for the value from here on, and find a
        // mistake in every use of the state.
        if self.declares(name.1) {
            return Err(r.mistake(format!(
                "{} is one of this file's states and classes, so a value can't be called it",
                name.1
            )));
        }
        r.expect("=")?;
        self.value(r, ty, fetched, 0)?;
        r.expect(";")?;
        Ok(name)
    }

    /// Checks that the file has fetched the state or class `name`.
    fn fetched(&self, r: &Reader<'_>, name: &str, fetched: &[&str]) -> Result<(), DataError> {
        if fetched.contains(&name) {
            return Ok(());
        }
        Err(r.mistake(format!(
            "{} isn't a state or class this file fetches from {}",
            clip(name),
            self.path
        )))
    }

    /// A value of type `ty`, inside `depth` lists and things.
    fn value(
        &self,
        r: &mut Reader<'_>,
        ty: Type<'_>,
        fetched: &[&str],
        depth: usize,
    ) -> Result<(), DataError> {
        if ty.lists > 0 {
            let item = Type {
                lists: ty.lists - 1,
                ..ty
            };
            return match r.take() {
                Some(Piece::Symbol("[")) if depth == MAX_DEPTH => Err(r.mistake(too_deep())),
                Some(Piece::Symbol("[")) => {
                    r.items("]", |r| self.value(r, item, fetched, depth + 1))
                }
                found => Err(r.wrong("a list in [ ]", found)),
            };
        }
        match ty.kind {
            Kind::Int => number(r, true),
            Kind::Float => number(r, false),
            Kind::Text => match r.take() {
                Some(Piece::Text {
                    inside,
                    value: false,
                    closed: true,
                }) => escapes(inside).map_err(|why| r.mistake(why)),
                found => Err(r.wrong("text in quotes", found)),
            },
            Kind::Bool => match r.take() {
                Some(Piece::Word("true" | "false")) => Ok(()),
                found => Err(r.wrong("true or false", found)),
            },
            Kind::Named(name) => {
                self.fetched(r, name, fetched)?;
                match self.state(name) {
                    Some(options) => option(r, name, options),
                    None => self.thing(r, name, fetched, depth),
                }
            }
        }
    }

    /// A thing of the class `name`: `new Layer { id = "…", over = false }`,
    /// each field one of the class's, filled in once with a value of its
    /// type.
    fn thing(
        &self,
        r: &mut Reader<'_>,
        name: &str,
        fetched: &[&str],
        depth: usize,
    ) -> Result<(), DataError> {
        let fields = self.class(name).unwrap_or_default();
        for word in ["new", name] {
            match r.take() {
                Some(Piece::Word(found)) if found == word => {}
                found => return Err(r.wrong(&format!("new {name} {{ … }}"), found)),
            }
        }
        r.expect("{")?;
        if depth == MAX_DEPTH {
            return Err(r.mistake(too_deep()));
        }
        let mut filled = 0u64;
        r.items("}", |r| {
            let field = r.word("a field's name")?;
            let Some(at) = fields.iter().position(|(name, _)| *name == field) else {
                let all: Vec<&str> = fields.iter().map(|(name, _)| *name).collect();
                return Err(r.mistake(format!(
                    "{name} has no field called {}: its fields are {}",
                    clip(field),
                    all.join(", ")
                )));
            };
            if filled & (1 << at) != 0 {
                return Err(r.mistake(format!("{field} is already filled in here")));
            }
            filled |= 1 << at;
            r.expect("=")?;
            self.value(r, fields[at].1, fetched, depth + 1)
        })
    }
}

/// The mistake of a file with more pieces than Lust is handed.
fn too_big() -> String {
    format!(
        "it's too big to read: it holds more than {} million numbers, texts, names and symbols",
        MAX_PIECES / 1_000_000
    )
}

/// The mistake of lists and things too deep inside each other.
fn too_deep() -> String {
    format!("lists and things more than {MAX_DEPTH} deep, which no data file has")
}

/// One of the state `name`'s `options`: `ValueKind.Choice`.
fn option(r: &mut Reader<'_>, name: &str, options: &[&str]) -> Result<(), DataError> {
    let wanted = || {
        let all: Vec<String> = options.iter().map(|o| format!("{name}.{o}")).collect();
        format!("one of {name}'s options ({})", all.join(", "))
    };
    match r.take() {
        Some(Piece::Word(state)) if state == name => {}
        found => return Err(r.wrong(&wanted(), found)),
    }
    r.expect(".")?;
    match r.take() {
        Some(Piece::Word(option)) if options.contains(&option) => Ok(()),
        found => Err(r.wrong(&wanted(), found)),
    }
}

/// A number, with a `-` before it or not: a whole one if `whole`. Each is
/// read as Lust reads it, so one too big for Lust is a mistake here.
fn number(r: &mut Reader<'_>, whole: bool) -> Result<(), DataError> {
    let wanted = if whole { "a whole number" } else { "a number" };
    let mut found = r.take();
    if found == Some(Piece::Symbol("-")) {
        found = r.take();
    }
    let Some(Piece::Number(digits)) = found else {
        return Err(r.wrong(wanted, found));
    };
    if !digits.contains(['.', 'e', 'E']) {
        return match digits.parse::<i64>() {
            Ok(_) => Ok(()),
            Err(_) => Err(r.mistake(format!("{} is too big for a whole number", clip(digits)))),
        };
    }
    if whole {
        return Err(r.wrong(wanted, found));
    }
    match digits.parse::<f64>() {
        Ok(number) if number.is_finite() => Ok(()),
        _ => Err(r.mistake(format!("{} is too big for a number", clip(digits)))),
    }
}

/// Asks Lust whether each of `names` (with its line) can name a value, by
/// handing it a file of those names alone. Lust stops reading a value at a
/// name that's one of its own words (`lock Layer array end = […];`), picks
/// up again inside the value, and finds a mistake in every thing there: so
/// such a name never reaches it with its value.
fn names_fit(names: &[(u32, &str)]) -> Result<(), DataError> {
    const FILE: &str = "names.lst";
    let mut text = String::new();
    for (_, name) in names {
        let _ = writeln!(text, "lock int {name} = 0;");
    }
    text.push_str("end\n");
    let Err(problems) = lust::check(FILE, &[(FILE.to_string(), text)]) else {
        return Ok(());
    };
    let first = problems.first();
    let line = first
        .and_then(|problem| names.get((problem.line as usize).checked_sub(1)?))
        .map_or(0, |(line, _)| *line as usize);
    // Lust's words for it, which may hold the name, however long that is.
    let why: String = first
        .map_or("", |problem| &problem.message)
        .chars()
        .take(LONGEST)
        .collect();
    Err(error(line, format!("a value can't have this name: {why}")))
}

#[cfg(test)]
#[path = "lust_check_tests.rs"]
mod tests;
