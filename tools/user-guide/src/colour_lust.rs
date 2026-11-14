//! Lust coloured as Ledger colours it: the guide colourer of Ledger's own
//! guides (`highlightLust`), with the editor's fuller rules where the
//! manual's examples need them: states, objects used directly, the
//! pieces of a view and their style words.

use super::{Tk, ahead, blank, capital, colour_with, line, number, quoted, through, word};

/// The words that give a program its shape.
const STRUCTURE: &[&str] = &[
    "class",
    "object",
    "node",
    "assembly",
    "component",
    "event",
    "schema",
    "route",
    "method",
    "hook",
    "has",
    "new",
    "create",
    "lock",
    "fetch",
    "get",
    "pass",
    "end",
    "save",
    "load",
    "saved",
    "lua",
    "view",
    "style",
    "native",
    "database",
];
/// Questions, loops, sending and the rest of the flow.
const FLOW: &[&str] = &[
    "is", "then", "or", "either", "check", "when", "loop", "stop", "skip", "return", "try",
    "error", "like", "keep", "send", "on", "tween", "to", "over", "not", "in", "enter",
];
/// Blocks written after a piece (`open { }`, a canvas's `press { }`):
/// flow only when a block or `(` follows, so fields with these names stay
/// plain (`float open = 0;`).
const ATTACHED: &[&str] = &[
    "open", "menu", "press", "drag", "release", "hover", "close", "click",
];
/// Lust's types.
const TYPES: &[&str] = &[
    "int", "int8", "int16", "int32", "uint8", "uint16", "uint32", "float", "string", "bool",
    "array", "cMap", "map", "vec2", "vec3", "rotation", "colour", "entity",
];
const VALUES: &[&str] = &["true", "false", "nothing", "this"];
/// Lust's own objects and built-in calls.
const OBJECTS: &[&str] = &[
    "Maths", "Files", "Time", "Json", "Web", "Random", "Colour", "Rotation", "Ease", "App",
    "print", "wait", "ask",
];
/// The pieces of a view: pieces only at the start of a line, as in Lust.
const PIECES: &[&str] = &[
    "column",
    "row",
    "stack",
    "space",
    "text",
    "button",
    "scroll",
    "image",
    "every",
    "input",
    "textarea",
    "checkbox",
    "switch",
    "radio",
    "number",
    "slider",
    "datePicker",
    "colourPicker",
    "icon",
    "grid",
    "split",
    "tabs",
    "tab",
    "branch",
    "progress",
    "canvas",
    "box",
    "circle",
    "line",
    "dropdown",
    "menu",
    "item",
    "divider",
    "dialog",
    "window",
    "table",
    "list",
    "key",
    "drop",
    "tray",
    "toast",
    "press",
    "drag",
    "release",
    "hover",
];

/// What the next name is, after the word before it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Naming {
    #[default]
    Nothing,
    /// After `class` or `schema`: a class.
    Class,
    /// After `object`, `node`, `assembly`, `database`: an object.
    Object,
    /// After `style`: the style's name.
    Style,
    /// After `state`: the state and its name.
    State,
    /// After `state is`: the state it is, plain like its options.
    StateIs,
}

/// The token before, as far as the next one cares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Last {
    #[default]
    Other,
    /// A name, and whether it starts with a capital.
    Name(bool),
    /// `)` or `]`.
    Closing,
}

/// Where the colouring is in a Lust file.
#[derive(Debug, Default)]
struct Reading {
    /// Nothing but spaces yet on this line.
    line_start: bool,
    last: Last,
    /// The token before was a `:` or `.` after a name or a bracket: the
    /// name now is a member (`velocity:length`), plain.
    member: bool,
    /// …and that was a `.` after a name with a capital: a name with a
    /// capital now is a state (`Action.Jump`).
    state_member: bool,
    naming: Naming,
    /// `{` and `[` open now.
    depth: usize,
    /// The depth of a state's `[ options ]` while inside them.
    options: Option<usize>,
    /// A `[ ]` coming up holds styles (after `view`, a piece, a style).
    styled: bool,
    /// The depths of the style lists open (`hover:[ … ]` nests one).
    styles: Vec<usize>,
    /// The next name in a style list is a style's word.
    style_word: bool,
}

/// Lust in Ledger's quiet colours.
pub fn lust(text: &str) -> String {
    let mut reading = Reading {
        line_start: true,
        ..Reading::default()
    };
    colour_with(text, |rest| reading.token(rest))
}

impl Reading {
    fn token(&mut self, rest: &str) -> (Option<Tk>, usize) {
        if rest.starts_with("--[[") {
            return (Some(Tk::C), through(rest, 4, "]]"));
        }
        if rest.starts_with("--") || rest.starts_with("//") {
            return (Some(Tk::C), line(rest));
        }
        if rest.starts_with('\n') {
            self.line_start = true;
            return (None, 1);
        }
        let spaces = blank(rest);
        if spaces > 0 {
            return (None, spaces);
        }
        let found = self.solid(rest);
        self.line_start = false;
        found
    }

    fn solid(&mut self, rest: &str) -> (Option<Tk>, usize) {
        let member = std::mem::take(&mut self.member);
        let state_member = std::mem::take(&mut self.state_member);
        if rest.starts_with('"') {
            self.last = Last::Other;
            return (Some(Tk::V), quoted(rest, true));
        }
        let n = number(rest);
        if n > 0 {
            self.last = Last::Other;
            return (Some(Tk::V), n);
        }
        let n = word(rest);
        if n > 0 {
            let w = &rest[..n];
            let tk = self.name(w, ahead(&rest[n..]), member, state_member);
            self.last = Last::Name(capital(w));
            return (tk, n);
        }
        self.punctuation(rest)
    }

    fn in_styles(&self) -> bool {
        self.styles.last() == Some(&self.depth)
    }

    fn name(
        &mut self,
        w: &str,
        ahead: Option<char>,
        member: bool,
        state_member: bool,
    ) -> Option<Tk> {
        let is = |list: &[&str]| list.contains(&w);
        // [width fill, hover:[background red7]]: each style's first word.
        if self.in_styles() {
            if std::mem::take(&mut self.style_word) {
                return Some(Tk::T);
            }
            return if is(VALUES) {
                Some(Tk::V)
            } else if is(FLOW) {
                Some(Tk::F)
            } else {
                None
            };
        }
        if self.naming == Naming::Style {
            self.naming = Naming::Nothing;
            self.styled = true;
            return None;
        }
        if !member {
            if w == "style" {
                self.naming = Naming::Style;
                return Some(Tk::K);
            }
            if w == "view" {
                self.styled = true;
                return Some(Tk::K);
            }
            // A piece starts its line (column [gap 8] {, text "Hi";), and
            // so does your own (TodoRow [width 300] { … }).
            let piece = is(PIECES) && !matches!(ahead, Some('=' | '(' | '.' | ':' | ','));
            if self.line_start && (piece || (capital(w) && ahead == Some('['))) {
                self.styled = true;
                return Some(Tk::N);
            }
        }
        if w == "state" {
            self.naming = Naming::State;
            return Some(Tk::S);
        }
        if self.options.is_some_and(|depth| self.depth >= depth) {
            return None;
        }
        match self.naming {
            Naming::State if w == "is" => {
                self.naming = Naming::StateIs;
                return Some(Tk::F);
            }
            Naming::State if is(FLOW) => self.naming = Naming::Nothing,
            Naming::State => return Some(Tk::S),
            Naming::StateIs => return None,
            _ => {}
        }
        if member {
            return (state_member && capital(w)).then_some(Tk::S);
        }
        let block_follows = matches!(ahead, Some('{' | '('));
        if is(VALUES) {
            Some(Tk::V)
        } else if is(FLOW) || (is(ATTACHED) && block_follows) {
            Some(Tk::F)
        } else if ["post", "put", "delete"].contains(&w) && ahead == Some('"') {
            Some(Tk::K)
        } else if is(STRUCTURE) {
            self.naming = match w {
                "class" | "schema" => Naming::Class,
                "object" | "node" | "assembly" | "database" => Naming::Object,
                _ => Naming::Nothing,
            };
            Some(Tk::K)
        } else if is(TYPES) {
            Some(Tk::T)
        } else if is(OBJECTS) {
            Some(Tk::O)
        } else if let Some(tk) = self.declared() {
            Some(tk)
        } else if capital(w) {
            // Used directly (World:say, Farm.gold) it's an object; as a
            // type (RayHit below) a class.
            Some(if matches!(ahead, Some(':' | '.')) {
                Tk::O
            } else {
                Tk::N
            })
        } else {
            None
        }
    }

    /// The name being declared: `class Player`, `object Clock`.
    fn declared(&mut self) -> Option<Tk> {
        let tk = match self.naming {
            Naming::Class => Tk::N,
            Naming::Object => Tk::O,
            _ => return None,
        };
        self.naming = Naming::Nothing;
        Some(tk)
    }

    fn punctuation(&mut self, rest: &str) -> (Option<Tk>, usize) {
        let in_styles = self.in_styles();
        let last = std::mem::take(&mut self.last);
        let Some(c) = rest.chars().next() else {
            return (None, 0);
        };
        let tk = match c {
            '{' | '[' => {
                self.depth += 1;
                if c == '[' && self.naming == Naming::State && self.options.is_none() {
                    self.options = Some(self.depth);
                }
                if c == '[' && (self.styled || in_styles) {
                    self.styles.push(self.depth);
                    self.style_word = true;
                }
                if c == '{' {
                    self.styled = false;
                }
                self.naming = Naming::Nothing;
                Some(Tk::P)
            }
            '}' | ']' => {
                if in_styles {
                    self.styles.pop();
                }
                self.depth = self.depth.saturating_sub(1);
                if self.options.is_some_and(|depth| self.depth < depth) {
                    self.options = None;
                }
                if c == ']' && self.styles.is_empty() {
                    self.styled = false;
                }
                self.last = Last::Closing;
                Some(Tk::P)
            }
            ',' if in_styles => {
                self.style_word = true;
                Some(Tk::P)
            }
            '(' | ')' => {
                self.naming = Naming::Nothing;
                if c == ')' {
                    self.last = Last::Closing;
                }
                Some(Tk::P)
            }
            '?' => Some(Tk::F),
            // After "file": (fetch "solis/input":Input) the name is
            // coloured as itself, not as a member.
            ':' | '.' => {
                self.member = matches!(last, Last::Name(_) | Last::Closing);
                self.state_member = c == '.' && last == Last::Name(true);
                Some(Tk::P)
            }
            ';' | ',' => {
                self.naming = Naming::Nothing;
                if c == ';' {
                    self.styled = false;
                }
                Some(Tk::P)
            }
            '=' if rest.starts_with("=>") => return (Some(Tk::P), 2),
            _ => None,
        };
        (tk, c.len_utf8())
    }
}
