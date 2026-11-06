//! Tying a screen's values to the game (decision 0089, M5): beside a
//! value in Properties, a tie picks one of the screen's entity's
//! components and a field of the right kind, and the canvas writes the
//! Lust for it: the component read once at the top of the view
//! (`Health health = Ui:entity():health();`, fetched from its file), and
//! the value used (`width 360 * health.hp / health.maxHp` fills a bar).
//! Preview values try other numbers on the canvas without saving; and a
//! button's event is chosen from those the file fetches.

use crate::screen_doc::ScreenDoc;
use reflect::Value;
use ui::source::{self, Piece};

/// What kind of value a field holds, for offering it where it fits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    /// A number: sizes, opacity, a bar's value.
    Number,
    /// Words, or anything shown as words.
    Words,
}

/// A component the screen's entity can be read for.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Readable {
    /// The component (`Health`).
    pub(crate) component: String,
    /// The method that reads it on an entity (`health`).
    pub(crate) reader: String,
    /// The file it's fetched from (`main`, or `solis/screen` for Solis's
    /// own and other languages' components).
    pub(crate) module: String,
    /// Its fields: name, kind, and its starting number when it's one.
    pub(crate) fields: Vec<(String, Kind, f64)>,
}

/// What a screen can be tied to and send, worked out once per build of
/// the game's code.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct ScreenGame {
    /// The components the screen's entity can be read for.
    pub(crate) readable: Vec<Readable>,
    /// The game's events (name, file), for a button to send.
    pub(crate) events: Vec<(String, String)>,
}

/// What a screen can be tied to and send, for `program`.
pub(crate) fn screen_game(
    program: Option<&script::Compiled>,
    registry: &reflect::Registry,
) -> ScreenGame {
    ScreenGame {
        readable: readable(program, registry),
        events: program
            .and_then(|p| p.screens.as_ref())
            .map(|c| c.events.clone())
            .unwrap_or_default(),
    }
}

/// Every component screens can read, with its fields.
pub(crate) fn readable(
    program: Option<&script::Compiled>,
    registry: &reflect::Registry,
) -> Vec<Readable> {
    let Some(program) = program else {
        return Vec::new();
    };
    let Some(code) = &program.screens else {
        return Vec::new();
    };
    code.readers
        .iter()
        .filter_map(|(component, reader)| {
            let schema = program
                .schemas
                .iter()
                .find(|s| s.name == *component)
                .or_else(|| registry.get(component).map(|s| &**s))?;
            let module = program
                .places
                .get(component)
                .and_then(|(file, _)| file.strip_suffix(".lst"))
                .map(str::to_string)
                .unwrap_or_else(|| "solis/screen".into());
            let fields = schema
                .fields
                .iter()
                .filter_map(|f| {
                    let (kind, start) = match &f.default {
                        Value::Int(n) => (Kind::Number, *n as f64),
                        Value::Float(n) => (Kind::Number, *n),
                        Value::Text(_) | Value::Bool(_) => (Kind::Words, 0.0),
                        _ => return None,
                    };
                    Some((f.name.clone(), kind, start))
                })
                .collect();
            Some(Readable {
                component: component.clone(),
                reader: reader.clone(),
                module,
                fields,
            })
        })
        .collect()
}

/// The name a component is read into: `Health` as `health`.
pub(crate) fn var_of(component: &str) -> String {
    let mut c = component.chars();
    match c.next() {
        Some(f) => f.to_lowercase().chain(c).collect(),
        None => String::new(),
    }
}

/// Where a tie goes on the piece picked.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum TieTo {
    /// Its words (`.text()` of the field).
    Words,
    /// Its value (a progress bar's or a slider's).
    Value,
    /// A `[ ]` setting.
    Style(String),
    /// Its width (or height), filling a bar: `full × field ÷ max`.
    Bar {
        /// `width` or `height`.
        side: String,
        /// The size when the field is at its most.
        full: f32,
        /// The field holding the most, or a number.
        max: String,
    },
}

/// A `fetch "module":[A, B];` line read: its module, its names, and
/// where its statement is in the text (from `fetch` to its `;`).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FetchLine {
    pub(crate) module: String,
    pub(crate) names: Vec<String>,
    pub(crate) span: std::ops::Range<usize>,
}

/// Reads the line starting at byte `at` of `text` as a fetch, if it is
/// one written plainly (spaces anywhere between its parts; anything after
/// its `;`, such as a comment, kept). Anything unusual: `None`.
fn fetch_line(at: usize, line: &str) -> Option<FetchLine> {
    let lead = line.len() - line.trim_start().len();
    let rest = line[lead..].strip_prefix("fetch")?;
    let mut i = lead + 5;
    let skip = |s: &str, i: &mut usize| {
        while s[*i..].starts_with([' ', '\t']) {
            *i += 1;
        }
    };
    if !rest.starts_with([' ', '\t', '"']) {
        return None;
    }
    skip(line, &mut i);
    let after_quote = line[i..].strip_prefix('"')?;
    let close = after_quote.find('"')?;
    let module = after_quote[..close].to_string();
    if module.contains('\\') {
        return None;
    }
    i += 1 + close + 1;
    skip(line, &mut i);
    i += line[i..].strip_prefix(':').map(|_| 1)?;
    skip(line, &mut i);
    let semi = i + line[i..].find(';')?;
    let list = line[i..semi].trim();
    let inner = match list.strip_prefix('[') {
        Some(l) => l.strip_suffix(']')?,
        None => list,
    };
    let names: Vec<String> = inner
        .split(',')
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map(str::to_string)
        .collect();
    if names
        .iter()
        .any(|n| !n.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
    {
        return None;
    }
    Some(FetchLine {
        module,
        names,
        span: at + lead..at + semi + 1,
    })
}

/// Every plainly written fetch line in `text`.
pub(crate) fn fetch_lines(text: &str) -> Vec<FetchLine> {
    let mut out = Vec::new();
    let mut at = 0;
    for line in text.split_inclusive('\n') {
        let body = line.trim_end_matches(['\n', '\r']);
        if let Some(f) = fetch_line(at, body) {
            out.push(f);
        }
        at += line.len();
    }
    out
}

/// `text` with `name` fetched from `module`: added to its fetch line, or
/// a new fetch line after the others (at the top when there are none).
pub(crate) fn fetched(text: &str, module: &str, name: &str) -> String {
    let lines = fetch_lines(text);
    if lines
        .iter()
        .any(|f| f.module == module && f.names.iter().any(|n| n == name))
    {
        return text.to_string();
    }
    if let Some(f) = lines.iter().find(|f| f.module == module) {
        let mut names = f.names.clone();
        names.push(name.to_string());
        let list = if names.len() == 1 {
            names[0].clone()
        } else {
            format!("[{}]", names.join(", "))
        };
        return source::replaced(text, f.span.clone(), &format!("fetch \"{module}\":{list};"));
    }
    // A new line, after the last fetch, or at the top.
    let after = lines.last().map_or(0, |f| {
        text[f.span.end..]
            .find('\n')
            .map_or(text.len(), |i| f.span.end + i + 1)
    });
    let nl = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let lead = if after == text.len() && !text.is_empty() && !text.ends_with('\n') {
        nl
    } else {
        ""
    };
    source::replaced(
        text,
        after..after,
        &format!("{lead}fetch \"{module}\":{name};{nl}"),
    )
}

/// The module a file is fetched as: `ui/hud.lst` as `ui/hud`.
fn module_of(path: &str) -> &str {
    path.strip_suffix(".lst").unwrap_or(path)
}

impl ScreenDoc {
    /// The components the view reads (`Health health = Ui:entity():
    /// health();`), as (component, name it's read into).
    pub(crate) fn reads(&self) -> Vec<(String, String)> {
        let Some(block) = self.block() else {
            return Vec::new();
        };
        block
            .pieces
            .iter()
            .filter_map(|p| match p {
                Piece::Code(c) => {
                    let t = self.text[c.span.clone()].trim();
                    let (decl, rest) = t.split_once('=')?;
                    rest.trim().strip_prefix("Ui:entity():")?;
                    let mut words = decl.split_whitespace();
                    let (ty, var) = (words.next()?, words.next()?);
                    Some((ty.to_string(), var.to_string()))
                }
                Piece::Element(_) => None,
            })
            .collect()
    }

    /// Ties the piece picked's `to` to `field` of `r`, as one undo step:
    /// the component read at the top of the view (and fetched) if it
    /// isn't yet.
    pub(crate) fn tie(&mut self, r: &Readable, field: &str, to: &TieTo) -> bool {
        let Some(path) = self.picked.first().cloned() else {
            return false;
        };
        let var = self
            .reads()
            .into_iter()
            .find(|(ty, _)| *ty == r.component)
            .map(|(_, v)| v);
        let reads_it = var.is_some();
        let var = var.unwrap_or_else(|| var_of(&r.component));
        let value = format!("{var}.{field}");
        let Some(piece) = self.piece(&path).cloned() else {
            return false;
        };
        // The value used.
        let mut text = match to {
            TieTo::Words => match source::set_arg(&self.text, &piece, &format!("{value}.text()")) {
                Some(t) => t,
                None => return false,
            },
            TieTo::Value => match source::set_arg(&self.text, &piece, &value) {
                Some(t) => t,
                None => return false,
            },
            TieTo::Style(name) => source::set_style(&self.text, &piece, name, Some(&value)),
            TieTo::Bar { side, full, max } => source::set_style(
                &self.text,
                &piece,
                side,
                Some(&format!(
                    "{} * {value} / {}",
                    crate::canvas_tools::num(*full),
                    if max.parse::<f64>().is_ok() {
                        max.clone()
                    } else {
                        format!("{var}.{max}")
                    }
                )),
            ),
        };
        // Read at the top of the view, and fetched.
        let mut moved = false;
        if !reads_it {
            let Ok(screens) = source::parse(&text) else {
                return false;
            };
            let Some(view) = screens
                .into_iter()
                .find(|s| s.name == self.screen)
                .map(|s| s.view)
            else {
                return false;
            };
            let decl = format!("{} {var} = Ui:entity():{}();", r.component, r.reader);
            text = source::insert_piece(&text, &view, 0, &decl);
            moved = true;
        }
        if r.module != module_of(&self.path) {
            text = fetched(&text, &r.module, &r.component);
        }
        text = fetched(&text, "solis/screen", "Ui");
        let (hidden, locked) = (self.hidden.clone(), self.locked.clone());
        let changed = self.change(text);
        if changed {
            self.behind = true;
            if moved {
                self.hidden = hidden;
                self.locked = locked;
                // Everything in the view is one further down now.
                let down = |p: &Vec<usize>| {
                    let mut p = p.clone();
                    p[0] += 1;
                    p
                };
                self.picked = vec![down(&path)];
                self.hidden = self.hidden.iter().map(down).collect();
                self.locked = self.locked.iter().map(down).collect();
            }
        }
        changed
    }

    /// The event the piece picked sends, if what it does is just that:
    /// `send Name() to Ui:entity();` (or nothing yet, `Some(None)`).
    pub(crate) fn sends(&self) -> Option<Option<String>> {
        let piece = self.piece(self.picked.first()?)?;
        let body = self.text[piece.action.clone()?].trim();
        if body.is_empty() {
            return Some(None);
        }
        let rest = body.strip_prefix("send ")?.strip_suffix(';')?;
        let (call, to) = rest.split_once(" to ")?;
        if to.trim() != "Ui:entity()" {
            return None;
        }
        let name = call.trim().strip_suffix("()")?;
        name.chars()
            .all(|c| c.is_alphanumeric() || c == '_')
            .then(|| Some(name.to_string()))
    }

    /// Makes the piece picked send `event` (written in `file`) to the
    /// screen's entity when clicked, fetching it if it isn't yet.
    pub(crate) fn set_sends(&mut self, event: &str, file: &str) -> bool {
        let Some(piece) = self.picked.first().and_then(|p| self.piece(p)).cloned() else {
            return false;
        };
        let Some(action) = piece.action.clone() else {
            return false;
        };
        let line_start = self.text[..piece.span.start]
            .rfind('\n')
            .map_or(0, |i| i + 1);
        let indent: String = self.text[line_start..piece.span.start]
            .chars()
            .take_while(|c| c.is_whitespace())
            .collect();
        let nl = if self.text.contains("\r\n") {
            "\r\n"
        } else {
            "\n"
        };
        let body = format!("{nl}{indent}    send {event}() to Ui:entity();{nl}{indent}");
        let mut text = source::replaced(&self.text, action, &body);
        if module_of(file) != module_of(&self.path) {
            text = fetched(&text, module_of(file), event);
        }
        text = fetched(&text, "solis/screen", "Ui");
        let changed = self.change(text);
        self.behind |= changed;
        changed
    }
}

#[cfg(test)]
#[path = "screen_ties_tests.rs"]
mod tests;
