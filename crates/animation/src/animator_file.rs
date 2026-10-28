//! An animator file (`.solanim`, Phase 9, D3b): a state machine that
//! chooses which clip a sprite plays. Its parameters (set by game code),
//! its states (each playing a clip, or a clip per facing direction), and
//! the transitions between them, each with conditions that must all hold.
//!
//! A settings file in the same Lust style as areas, with a block per
//! parameter, state and transition, each by a stable random ID, so two
//! people's changes merge in Git
//! ([decision 0066](../../../docs/decisions/0066-animator-state-machine.md)):
//!
//! ```text
//! -- An animator. Written by Solis; safe to edit by hand.
//! animator version 1;
//!
//! sheet = "1BWHNR3RV8PYY";
//! entry = "3KD8V0Q2MX7TA";
//!
//! parameter 5RT0M2C8K1V4D {
//!     name = "facing";
//!     kind = "direction";
//!     start = "down";
//! }
//!
//! state 3KD8V0Q2MX7TA {
//!     name = "Walk";
//!     place = [120.0, 80.0];
//!     by = "facing";
//!     clips = [["down", "walk_down"], ["up", "walk_up"]];
//!     speed = 1.0;
//!     speed_by = "speed";
//! }
//!
//! transition 9XM1R4T6HD2QE {
//!     from = "3KD8V0Q2MX7TA";
//!     to = "7PQ2B5N8WK1ZC";
//!     when = ["not moving"];
//! }
//! ```
//!
//! - `sheet`: the sprite sheet (or clips file) whose clips it names, for
//!   the editor's lists and preview; a sprite plays its own sheet's.
//! - `entry`: the state an animator starts in (the first, if none).
//! - A **parameter**'s `kind` is `"number"`, `"switch"`, `"trigger"` or
//!   `"direction"`, with a `start` (a number, true or false, or a
//!   direction's name; triggers have none).
//! - A **state** plays one `clip`, or with `by` (a direction parameter) a
//!   clip per direction in `clips`: the nearest to the parameter plays.
//!   `speed` multiplies its clips' own; `speed_by` names a number
//!   parameter that multiplies it too.
//! - A **transition** goes `from` a state's ID (or `"any"`, from every
//!   state but its own `to`) `to` another, once every condition in `when`
//!   holds: `"moving"` (a switch on, or a trigger fired), `"not moving"`,
//!   `"speed > 0.5"`, `"speed < 0.5"`, or `"finished"` (the clip played
//!   all its times).
//!
//! Read as a hostile file: within limits; a value out of range is brought
//! into range with a note; a block or condition it can't use is skipped
//! with a note; only a file that can't be read at all (or one from a newer
//! version) is refused, with its line.

use crate::frames_file::clean_name;
use area::settings::{self, BlockOut, Field};
use reflect::{AssetId, Id, Value};
use std::fmt;

/// The layout version this build writes and reads.
pub const VERSION: u32 = 1;
/// The word the file starts with.
const KIND: &str = "animator";
/// The file's extension.
pub const EXTENSION: &str = "solanim";
/// The biggest animator file read.
pub const MAX_BYTES: usize = 1 << 20;
/// The most states.
pub const MAX_STATES: usize = 256;
/// The most transitions.
pub const MAX_TRANSITIONS: usize = 1024;
/// The most parameters.
pub const MAX_PARAMETERS: usize = 64;
/// The most conditions on one transition.
pub const MAX_CONDITIONS: usize = 8;
/// The longest parameter name, in characters.
pub const MAX_PARAMETER_NAME: usize = 32;
/// The slowest and fastest a state plays.
pub const SPEED_RANGE: (f32, f32) = (0.0, 16.0);
/// How far from the middle a state may be placed in the graph.
pub const MAX_PLACE: f32 = 100_000.0;
/// The biggest number a parameter or condition holds.
pub const MAX_NUMBER: f32 = 1.0e9;
/// The condition that holds once a state's clip has played all its times.
pub const FINISHED: &str = "finished";
/// The `from` of a transition from any state.
pub const ANY: &str = "any";

/// This file's version and upgraders (Phase 8, C1): each step turns what
/// was read at one version into the next, before it's applied. Every
/// version ever released keeps an example file in `tests/fixtures/`.
pub static FORMAT: reflect::upgrade::Format<settings::BlockSettings> = reflect::upgrade::Format {
    name: "animator",
    current: VERSION,
    version_of: |text| reflect::upgrade::header_version(text, &[KIND]),
    steps: &[],
};

/// A direction on the ground, as a facing parameter and a state's clips
/// by direction name them: up is away from the camera (+Y), right is +X.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Facing {
    /// Away from the camera.
    Up,
    /// Up and right.
    UpRight,
    /// Right.
    Right,
    /// Down and right.
    DownRight,
    /// Toward the camera.
    Down,
    /// Down and left.
    DownLeft,
    /// Left.
    Left,
    /// Up and left.
    UpLeft,
}

impl Facing {
    /// Every direction, the four straight ones first, then the diagonals
    /// (the order "+ Direction" offers them in).
    pub const ALL: [Self; 8] = [
        Self::Up,
        Self::Right,
        Self::Down,
        Self::Left,
        Self::UpRight,
        Self::DownRight,
        Self::DownLeft,
        Self::UpLeft,
    ];

    /// Its name, as the file writes it.
    pub fn word(self) -> &'static str {
        match self {
            Self::Up => "up",
            Self::UpRight => "up-right",
            Self::Right => "right",
            Self::DownRight => "down-right",
            Self::Down => "down",
            Self::DownLeft => "down-left",
            Self::Left => "left",
            Self::UpLeft => "up-left",
        }
    }

    /// The direction a name names.
    pub fn of_word(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|f| f.word() == word.trim())
    }

    /// Its arrow, for lists (↑ → ↓ ← and the diagonals).
    pub fn arrow(self) -> &'static str {
        match self {
            Self::Up => "↑",
            Self::UpRight => "↗",
            Self::Right => "→",
            Self::DownRight => "↘",
            Self::Down => "↓",
            Self::DownLeft => "↙",
            Self::Left => "←",
            Self::UpLeft => "↖",
        }
    }

    /// Its direction as a length-1 vector on the ground (x right, y up).
    pub fn vector(self) -> [f32; 2] {
        const D: f32 = std::f32::consts::FRAC_1_SQRT_2;
        match self {
            Self::Up => [0.0, 1.0],
            Self::UpRight => [D, D],
            Self::Right => [1.0, 0.0],
            Self::DownRight => [D, -D],
            Self::Down => [0.0, -1.0],
            Self::DownLeft => [-D, -D],
            Self::Left => [-1.0, 0.0],
            Self::UpLeft => [-D, D],
        }
    }

    /// The direction nearest a vector (down for none).
    pub fn nearest(v: [f32; 2]) -> Self {
        let length = v[0] * v[0] + v[1] * v[1];
        if length.is_nan() || length <= 0.0 {
            return Self::Down;
        }
        let mut best = (Self::Down, f32::NEG_INFINITY);
        for f in Self::ALL {
            let [x, y] = f.vector();
            let dot = x * v[0] + y * v[1];
            if dot > best.1 + 1e-6 {
                best = (f, dot);
            }
        }
        best.0
    }
}

/// What kind of value a parameter holds, and what it starts at.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParameterKind {
    /// A number (speed).
    Number(f32),
    /// On or off (moving).
    Switch(bool),
    /// Fired by game code for one tick (swing).
    Trigger,
    /// A direction on the ground (facing).
    Direction(Facing),
}

impl ParameterKind {
    /// Its kind's name, as the file writes it.
    pub fn word(self) -> &'static str {
        match self {
            Self::Number(_) => "number",
            Self::Switch(_) => "switch",
            Self::Trigger => "trigger",
            Self::Direction(_) => "direction",
        }
    }
}

/// A value game code sets, which transitions test.
#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    /// Its ID, which never changes.
    pub id: Id,
    /// Its name, as game code and conditions use it: letters, digits and
    /// `_`.
    pub name: String,
    /// What it holds.
    pub kind: ParameterKind,
}

/// What a state plays.
#[derive(Debug, Clone, PartialEq)]
pub enum Plays {
    /// One clip, by name ("" for none: the sprite is left as it is).
    Clip(String),
    /// A clip per direction: the nearest to the direction parameter `by`
    /// plays.
    ByDirection {
        /// The direction parameter's name.
        by: String,
        /// Each direction's clip.
        clips: Vec<(Facing, String)>,
    },
}

/// A state: what plays while the animator is in it.
#[derive(Debug, Clone, PartialEq)]
pub struct State {
    /// Its ID, which never changes.
    pub id: Id,
    /// Its name, as game code and the editor show it.
    pub name: String,
    /// Where it sits in the graph.
    pub place: [f32; 2],
    /// What it plays.
    pub plays: Plays,
    /// How fast: 1 at its clips' own speed.
    pub speed: f32,
    /// A number parameter its speed is multiplied by ("" for none).
    pub speed_by: String,
}

/// A test on the parameters, or on the state's clip.
#[derive(Debug, Clone, PartialEq)]
pub enum Condition {
    /// A switch on, or a trigger fired.
    Is(String),
    /// A switch off.
    Not(String),
    /// A number above a value.
    Above(String, f32),
    /// A number below a value.
    Below(String, f32),
    /// The state's clip has played all its times.
    Finished,
}

impl Condition {
    /// The condition as written: `"moving"`, `"not moving"`,
    /// `"speed > 0.5"`, `"finished"`.
    pub fn text(&self) -> String {
        match self {
            Self::Is(n) => n.clone(),
            Self::Not(n) => format!("not {n}"),
            Self::Above(n, v) => format!("{n} > {}", number_text(*v)),
            Self::Below(n, v) => format!("{n} < {}", number_text(*v)),
            Self::Finished => FINISHED.to_owned(),
        }
    }

    /// Reads a condition as written.
    pub fn of_text(text: &str) -> Result<Self, String> {
        let t = text.trim();
        if t == FINISHED {
            return Ok(Self::Finished);
        }
        for (sign, make) in [
            ('>', Self::Above as fn(String, f32) -> Self),
            ('<', Self::Below as fn(String, f32) -> Self),
        ] {
            if let Some((name, value)) = t.split_once(sign) {
                let name = parameter_name(name)?;
                let value: f32 = value
                    .trim()
                    .parse()
                    .ok()
                    .filter(|v: &f32| v.is_finite())
                    .ok_or_else(|| format!("\"{}\" isn't a number", value.trim()))?;
                return Ok(make(name, value.clamp(-MAX_NUMBER, MAX_NUMBER)));
            }
        }
        if let Some(name) = t.strip_prefix("not ") {
            return Ok(Self::Not(parameter_name(name)?));
        }
        Ok(Self::Is(parameter_name(t)?))
    }

    /// The parameter it tests, if any.
    pub fn parameter(&self) -> Option<&str> {
        match self {
            Self::Is(n) | Self::Not(n) | Self::Above(n, _) | Self::Below(n, _) => Some(n),
            Self::Finished => None,
        }
    }
}

/// A number as short as it reads back exactly (`0.5`, `2`).
fn number_text(v: f32) -> String {
    format!("{v}")
}

/// A parameter's name, checked: letters, digits and `_`, at most
/// [`MAX_PARAMETER_NAME`], not `finished`.
pub fn parameter_name(text: &str) -> Result<String, String> {
    let t = text.trim();
    if t.is_empty() {
        return Err("a parameter needs a name".to_owned());
    }
    if t == FINISHED || t == "not" {
        return Err(format!("\"{t}\" can't be a parameter's name"));
    }
    if t.chars().count() > MAX_PARAMETER_NAME || !t.chars().all(|c| c.is_alphanumeric() || c == '_')
    {
        return Err(format!(
            "\"{t}\" isn't a parameter's name (letters, digits and _, at most \
             {MAX_PARAMETER_NAME})"
        ));
    }
    Ok(t.to_owned())
}

/// A way from one state to another.
#[derive(Debug, Clone, PartialEq)]
pub struct Transition {
    /// Its ID, which never changes.
    pub id: Id,
    /// The state it leaves, or `None` for any state.
    pub from: Option<Id>,
    /// The state it goes to.
    pub to: Id,
    /// What must all hold for it to be taken (none: at once).
    pub when: Vec<Condition>,
}

/// An animator file.
#[derive(Debug, Clone, PartialEq)]
pub struct AnimatorFile {
    /// The sheet or clips file whose clips it names (for the editor).
    pub sheet: Option<AssetId>,
    /// The state it starts in (`None`: the first).
    pub entry: Option<Id>,
    /// Where the editor shows "Any state" (transitions from every state
    /// leave it).
    pub any_place: [f32; 2],
    /// Its parameters, in order.
    pub parameters: Vec<Parameter>,
    /// Its states, in order.
    pub states: Vec<State>,
    /// Its transitions, in order: earlier ones are tried first.
    pub transitions: Vec<Transition>,
}

impl Default for AnimatorFile {
    fn default() -> Self {
        Self {
            sheet: None,
            entry: None,
            any_place: [0.0, 240.0],
            parameters: Vec::new(),
            states: Vec::new(),
            transitions: Vec::new(),
        }
    }
}

impl AnimatorFile {
    /// The state with this ID.
    pub fn state(&self, id: Id) -> Option<&State> {
        self.states.iter().find(|s| s.id == id)
    }

    /// The state it starts in: the entry, or the first.
    pub fn entry_state(&self) -> Option<&State> {
        self.entry
            .and_then(|id| self.state(id))
            .or(self.states.first())
    }

    /// The parameter with this name.
    pub fn parameter(&self, name: &str) -> Option<&Parameter> {
        self.parameters.iter().find(|p| p.name == name)
    }
}

/// Why an animator file couldn't be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimatorError {
    /// The line it went wrong on (from 1), or 0 for the whole file.
    pub line: usize,
    /// What went wrong, in plain words.
    pub message: String,
}

impl fmt::Display for AnimatorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line == 0 {
            f.write_str(&self.message)
        } else {
            write!(f, "line {}: {}", self.line, self.message)
        }
    }
}

impl std::error::Error for AnimatorError {}

/// An animator file as read, with notes about what was put right.
#[derive(Debug, Clone, PartialEq)]
pub struct ReadAnimator {
    /// The file.
    pub file: AnimatorFile,
    /// What was skipped or put right, in plain words with lines.
    pub notes: Vec<String>,
}

/// Whether `path` names an animator file (by its extension).
pub fn is_animator_file(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case(EXTENSION))
}

/// Reads an animator file.
pub fn read(text: &str) -> Result<ReadAnimator, AnimatorError> {
    if text.len() > MAX_BYTES {
        return Err(AnimatorError {
            line: 0,
            message: format!(
                "the file is {} KB; animator files up to {} KB are read",
                text.len() >> 10,
                MAX_BYTES >> 10
            ),
        });
    }
    let mut read =
        settings::read_block_settings(text, KIND, VERSION).map_err(|e| AnimatorError {
            line: e.line,
            message: e.message,
        })?;
    let version = read.version;
    FORMAT
        .upgrade(version, &mut read)
        .map_err(|e| AnimatorError {
            line: 1,
            message: e.to_string(),
        })?;
    let mut notes = Vec::new();
    let mut file = AnimatorFile::default();
    let mut entry: Option<(usize, String)> = None;
    for field in &read.fields {
        let line = field.line;
        match (field.key.as_str(), &field.value) {
            ("sheet", Value::Text(t)) if t.trim().is_empty() => {}
            ("sheet", Value::Text(t)) => match t.trim().parse::<AssetId>() {
                Ok(id) => file.sheet = Some(id),
                Err(_) => notes.push(format!(
                    "line {line}: the sheet isn't an asset's ID, so it's skipped"
                )),
            },
            ("entry", Value::Text(t)) => entry = Some((line, t.trim().to_owned())),
            ("any_place", v) => match place_of(v) {
                Some(p) => file.any_place = p,
                None => notes.push(format!(
                    "line {line}: any_place should be two numbers, so it's skipped"
                )),
            },
            (key, _) => notes.push(format!(
                "line {line}: \"{key}\" isn't an animator setting (or isn't text in quotes), \
                 so it's skipped"
            )),
        }
    }
    // Parameters first, then states, then transitions (which name both),
    // whatever order the blocks are in.
    for word in ["parameter", "state", "transition"] {
        for block in read.blocks.iter().filter(|b| b.word == word) {
            let line = block.line;
            let result = block
                .name
                .parse::<Id>()
                .map_err(|_| format!("\"{}\" isn't an ID", block.name))
                .and_then(|id| match word {
                    "parameter" => read_parameter(&mut file, id, &block.fields, &mut notes),
                    "state" => read_state(&mut file, id, &block.fields, &mut notes),
                    _ => read_transition(&mut file, id, &block.fields, &mut notes),
                });
            if let Err(why) = result {
                notes.push(format!("line {line}: {why}, so this {word} is skipped"));
            }
        }
    }
    for block in &read.blocks {
        if !matches!(block.word.as_str(), "parameter" | "state" | "transition") {
            notes.push(format!(
                "line {}: \"{}\" isn't a part of an animator, so it's skipped",
                block.line, block.word
            ));
        }
    }
    if let Some((line, e)) = entry
        && !e.is_empty()
    {
        match e.parse::<Id>().ok().filter(|id| file.state(*id).is_some()) {
            Some(id) => file.entry = Some(id),
            None => notes.push(format!(
                "line {line}: the entry isn't one of its states, so it starts in the first"
            )),
        }
    }
    check_states_played(&mut file, &mut notes);
    Ok(ReadAnimator { file, notes })
}

/// Takes `key` from a block's fields.
fn get<'a>(fields: &'a [Field], key: &str) -> Option<&'a Value> {
    fields.iter().find(|f| f.key == key).map(|f| &f.value)
}

/// Notes fields a block has that its kind doesn't use.
fn note_unknown(fields: &[Field], known: &[&str], what: &str, notes: &mut Vec<String>) {
    for f in fields {
        if !known.contains(&f.key.as_str()) {
            notes.push(format!(
                "line {}: \"{}\" isn't a {what} setting, so it's skipped",
                f.line, f.key
            ));
        }
    }
}

fn text_of(value: Option<&Value>, what: &str) -> Result<String, String> {
    match value {
        Some(Value::Text(t)) => Ok(t.clone()),
        None => Ok(String::new()),
        Some(_) => Err(format!("its {what} should be text in quotes")),
    }
}

fn read_parameter(
    file: &mut AnimatorFile,
    id: Id,
    fields: &[Field],
    notes: &mut Vec<String>,
) -> Result<(), String> {
    note_unknown(fields, &["name", "kind", "start"], "parameter", notes);
    if file.parameters.len() == MAX_PARAMETERS {
        return Err(format!("more than {MAX_PARAMETERS} parameters"));
    }
    if file.parameters.iter().any(|p| p.id == id) {
        return Err("a second parameter with this ID".to_owned());
    }
    let name = parameter_name(&text_of(get(fields, "name"), "name")?)?;
    if file.parameter(&name).is_some() {
        return Err(format!("a second parameter called \"{name}\""));
    }
    let start = get(fields, "start");
    let kind = match text_of(get(fields, "kind"), "kind")?.trim() {
        "number" => ParameterKind::Number(match start {
            None => 0.0,
            Some(v) => v
                .as_f32()
                .filter(|v| v.is_finite())
                .ok_or("a number's start should be a number")?
                .clamp(-MAX_NUMBER, MAX_NUMBER),
        }),
        "switch" => ParameterKind::Switch(match start {
            None => false,
            Some(Value::Bool(b)) => *b,
            Some(_) => return Err("a switch's start should be true or false".to_owned()),
        }),
        "trigger" => ParameterKind::Trigger,
        "direction" => ParameterKind::Direction(match start {
            None => Facing::Down,
            Some(Value::Text(t)) => {
                Facing::of_word(t).ok_or_else(|| format!("\"{t}\" isn't a direction"))?
            }
            Some(_) => return Err("a direction's start should be a direction's name".to_owned()),
        }),
        other => {
            return Err(format!(
                "\"{other}\" isn't a kind of parameter (number, switch, trigger or direction)"
            ));
        }
    };
    file.parameters.push(Parameter { id, name, kind });
    Ok(())
}

fn read_state(
    file: &mut AnimatorFile,
    id: Id,
    fields: &[Field],
    notes: &mut Vec<String>,
) -> Result<(), String> {
    note_unknown(
        fields,
        &["name", "place", "clip", "by", "clips", "speed", "speed_by"],
        "state",
        notes,
    );
    if file.states.len() == MAX_STATES {
        return Err(format!("more than {MAX_STATES} states"));
    }
    if file.states.iter().any(|s| s.id == id) {
        return Err("a second state with this ID".to_owned());
    }
    let mut name = clean_name(&text_of(get(fields, "name"), "name")?);
    if name.is_empty() {
        name = "State".to_owned();
    }
    let name = free_state_name(file, &name);
    let place = match get(fields, "place") {
        None => [0.0, 0.0],
        Some(v) => place_of(v).ok_or("its place should be two numbers")?,
    };
    let by = text_of(get(fields, "by"), "by")?;
    let plays = if by.trim().is_empty() {
        Plays::Clip(clean_name(&text_of(get(fields, "clip"), "clip")?))
    } else {
        let by = parameter_name(&by)?;
        let mut clips: Vec<(Facing, String)> = Vec::new();
        let shape = "its clips should be [[\"direction\", \"clip\"], …]";
        match get(fields, "clips") {
            None => {}
            Some(Value::List(items)) => {
                for item in items {
                    let Value::List(pair) = item else {
                        return Err(shape.to_owned());
                    };
                    let [Value::Text(d), Value::Text(c)] = pair.as_slice() else {
                        return Err(shape.to_owned());
                    };
                    let facing =
                        Facing::of_word(d).ok_or_else(|| format!("\"{d}\" isn't a direction"))?;
                    if clips.iter().any(|(f, _)| *f == facing) {
                        return Err(format!("{d} is given two clips"));
                    }
                    clips.push((facing, clean_name(c)));
                }
            }
            Some(_) => return Err(shape.to_owned()),
        }
        Plays::ByDirection { by, clips }
    };
    let speed = match get(fields, "speed") {
        None => 1.0,
        Some(v) => {
            let s = v
                .as_f32()
                .filter(|s| !s.is_nan())
                .ok_or("its speed should be a number")?;
            let (lo, hi) = SPEED_RANGE;
            let fitted = s.clamp(lo, hi);
            if fitted != s {
                notes.push(format!(
                    "{name}: speed {s} is outside {lo} to {hi}, so it's {fitted}"
                ));
            }
            fitted
        }
    };
    let speed_by = match text_of(get(fields, "speed_by"), "speed_by")?.trim() {
        "" => String::new(),
        n => parameter_name(n)?,
    };
    file.states.push(State {
        id,
        name,
        place,
        plays,
        speed,
        speed_by,
    });
    Ok(())
}

/// A place in the graph: two numbers, brought within [`MAX_PLACE`].
fn place_of(v: &Value) -> Option<[f32; 2]> {
    let [x, y] = v
        .as_floats::<2>()
        .filter(|p| p.iter().all(|c| c.is_finite()))?;
    Some([
        x.clamp(-MAX_PLACE, MAX_PLACE),
        y.clamp(-MAX_PLACE, MAX_PLACE),
    ])
}

/// `name`, or with a number after it if a state has it already.
pub fn free_state_name(file: &AnimatorFile, name: &str) -> String {
    if !file.states.iter().any(|s| s.name == name) {
        return name.to_owned();
    }
    (2..)
        .map(|n| {
            let tail = format!(" {n}");
            let head: String = name
                .chars()
                .take(crate::frames_file::MAX_NAME - tail.len())
                .collect();
            format!("{}{tail}", head.trim_end())
        })
        .find(|n| !file.states.iter().any(|s| s.name == *n))
        .unwrap_or_default()
}

fn read_transition(
    file: &mut AnimatorFile,
    id: Id,
    fields: &[Field],
    notes: &mut Vec<String>,
) -> Result<(), String> {
    note_unknown(fields, &["from", "to", "when"], "transition", notes);
    if file.transitions.len() == MAX_TRANSITIONS {
        return Err(format!("more than {MAX_TRANSITIONS} transitions"));
    }
    if file.transitions.iter().any(|t| t.id == id) {
        return Err("a second transition with this ID".to_owned());
    }
    let state = |text: String, what: &str| -> Result<Id, String> {
        text.trim()
            .parse::<Id>()
            .ok()
            .filter(|id| file.state(*id).is_some())
            .ok_or_else(|| format!("its {what} isn't one of the states"))
    };
    let from = text_of(get(fields, "from"), "from")?;
    let from = if from.trim() == ANY {
        None
    } else {
        Some(state(from, "from")?)
    };
    let to = state(text_of(get(fields, "to"), "to")?, "to")?;
    let mut when = Vec::new();
    match get(fields, "when") {
        None => {}
        Some(Value::List(items)) => {
            for item in items {
                let Value::Text(t) = item else {
                    return Err("its conditions should be text in quotes".to_owned());
                };
                if when.len() == MAX_CONDITIONS {
                    notes.push(format!(
                        "a transition has more than {MAX_CONDITIONS} conditions; the rest are \
                         skipped"
                    ));
                    break;
                }
                match Condition::of_text(t) {
                    Ok(c) => when.push(c),
                    Err(why) => notes.push(format!("{why}, so that condition is skipped")),
                }
            }
        }
        Some(_) => return Err("its conditions should be a list".to_owned()),
    }
    file.transitions.push(Transition { id, from, to, when });
    Ok(())
}

/// Notes what a state uses that isn't there, or isn't the right kind;
/// it's kept (the editor shows it), and the running game passes over it.
fn check_states_played(file: &mut AnimatorFile, notes: &mut Vec<String>) {
    for state in &file.states {
        if let Plays::ByDirection { by, .. } = &state.plays
            && !matches!(
                file.parameter(by).map(|p| p.kind),
                Some(ParameterKind::Direction(_))
            )
        {
            notes.push(format!(
                "{}: \"{by}\" isn't a direction parameter, so it faces down",
                state.name
            ));
        }
        if !state.speed_by.is_empty()
            && !matches!(
                file.parameter(&state.speed_by).map(|p| p.kind),
                Some(ParameterKind::Number(_))
            )
        {
            notes.push(format!(
                "{}: \"{}\" isn't a number parameter, so its speed isn't changed by it",
                state.name, state.speed_by
            ));
        }
    }
}

/// Writes an animator file that [`read`] reads back.
pub fn write(file: &AnimatorFile) -> String {
    let fields = [
        (
            "sheet",
            Value::Text(file.sheet.map_or_else(String::new, |s| s.to_string())),
        ),
        (
            "entry",
            Value::Text(file.entry.map_or_else(String::new, |s| s.to_string())),
        ),
        (
            "any_place",
            Value::List(vec![
                Value::Float(f64::from(file.any_place[0])),
                Value::Float(f64::from(file.any_place[1])),
            ]),
        ),
    ];
    let mut blocks: Vec<BlockOut<'_>> = Vec::new();
    for p in &file.parameters {
        let mut f = vec![
            ("name", Value::Text(p.name.clone())),
            ("kind", Value::Text(p.kind.word().to_owned())),
        ];
        match p.kind {
            ParameterKind::Number(v) => f.push(("start", Value::Float(f64::from(v)))),
            ParameterKind::Switch(b) => f.push(("start", Value::Bool(b))),
            ParameterKind::Trigger => {}
            ParameterKind::Direction(d) => f.push(("start", Value::Text(d.word().to_owned()))),
        }
        blocks.push(("parameter", p.id.to_string(), f));
    }
    for s in &file.states {
        let mut f = vec![
            ("name", Value::Text(s.name.clone())),
            (
                "place",
                Value::List(vec![
                    Value::Float(f64::from(s.place[0])),
                    Value::Float(f64::from(s.place[1])),
                ]),
            ),
        ];
        match &s.plays {
            Plays::Clip(c) => f.push(("clip", Value::Text(c.clone()))),
            Plays::ByDirection { by, clips } => {
                f.push(("by", Value::Text(by.clone())));
                f.push((
                    "clips",
                    Value::List(
                        clips
                            .iter()
                            .map(|(d, c)| {
                                Value::List(vec![
                                    Value::Text(d.word().to_owned()),
                                    Value::Text(c.clone()),
                                ])
                            })
                            .collect(),
                    ),
                ));
            }
        }
        f.push(("speed", Value::Float(f64::from(s.speed))));
        if !s.speed_by.is_empty() {
            f.push(("speed_by", Value::Text(s.speed_by.clone())));
        }
        blocks.push(("state", s.id.to_string(), f));
    }
    for t in &file.transitions {
        let f = vec![
            (
                "from",
                Value::Text(t.from.map_or_else(|| ANY.to_owned(), |id| id.to_string())),
            ),
            ("to", Value::Text(t.to.to_string())),
            (
                "when",
                Value::List(t.when.iter().map(|c| Value::Text(c.text())).collect()),
            ),
        ];
        blocks.push(("transition", t.id.to_string(), f));
    }
    settings::write_block_settings(
        KIND,
        VERSION,
        "An animator. Written by Solis; safe to edit by hand.\n\
         Parameters are set by game code; each state plays a clip (or a clip per\n\
         direction, the nearest to its direction parameter); a transition is taken\n\
         once all its conditions hold, earlier ones first.",
        &fields,
        &blocks,
    )
}

#[cfg(test)]
#[path = "animator_file_tests.rs"]
pub(crate) mod tests;
