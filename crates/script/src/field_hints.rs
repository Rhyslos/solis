//! Field hints (stage 1; decision 0127): how a component's own settings
//! show in Properties. A slider between limits, a step, a unit after the
//! number, a sentence under the field, a heading, text over several
//! lines, or not shown at all.
//!
//! Lust has no attributes on fields yet, so a Lust field's hints are the
//! comment straight after it, in the `[ ]` words Lust's screens use
//! (`float speed = 4.0; -- [range 0 10, step 0.5, unit "m/s"]`, read by
//! [`crate::hint_comments`]); a Lua field's are a table after its options
//! (`{ range = { 0, 10 }, step = 0.5 }`); a Rust one's are the schema's
//! own `reflect::Hint`. Both come here as [`Word`]s and are put on the field
//! the same way, so every language means the same by each.

use reflect::{FieldInfo, FieldKind, Value};
use std::borrow::Cow;

/// The longest text a hint holds (a tip is a sentence, not a page).
pub(crate) const MAX_HINT_TEXT: usize = 300;

/// The words a hint may use, for "isn't a hint Solis knows".
pub(crate) const WORDS: &str = "range, step, unit, tip, group, multiline or hidden";

/// One hint, as written.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Word {
    /// `range 0 10`: where dragging and typing stop, and a slider.
    Range(f64, f64),
    /// `step 0.5`.
    Step(f64),
    /// `unit "m/s"`.
    Unit(String),
    /// `tip "Walking speed"`.
    Tip(String),
    /// `group "Movement"`.
    Group(String),
    /// `multiline`.
    Multiline,
    /// `hidden`.
    Hidden,
}

impl Word {
    /// How it's written, for mistakes.
    fn name(&self) -> &'static str {
        match self {
            Self::Range(..) => "range",
            Self::Step(_) => "step",
            Self::Unit(_) => "unit",
            Self::Tip(_) => "tip",
            Self::Group(_) => "group",
            Self::Multiline => "multiline",
            Self::Hidden => "hidden",
        }
    }
}

/// A piece of a hint's text.
#[derive(Debug, Clone, PartialEq)]
enum Token {
    Word(String),
    Number(f64),
    Text(String),
    Comma,
}

/// The hints between `[` and `]` (`range 0 10, unit "m"`), and a
/// sentence for each mistake. A mistake leaves out that hint only.
pub(crate) fn read(inside: &str) -> (Vec<Word>, Vec<String>) {
    let mut mistakes = Vec::new();
    let tokens = match tokens(inside) {
        Ok(tokens) => tokens,
        Err(mistake) => return (Vec::new(), vec![mistake]),
    };
    let mut words = Vec::new();
    for item in tokens.split(|t| *t == Token::Comma) {
        match item {
            [] => {}
            [Token::Word(word), rest @ ..] => match word_of(word, rest) {
                Ok(word) if words.iter().any(|w: &Word| w.name() == word.name()) => {
                    mistakes.push(format!("{} is given twice", word.name()));
                }
                Ok(word) => words.push(word),
                Err(mistake) => mistakes.push(mistake),
            },
            [first, ..] => mistakes.push(format!(
                "each hint starts with its word ({WORDS}), and this one starts with {}",
                shown(first)
            )),
        }
    }
    (words, mistakes)
}

/// One hint from its word and what follows it.
fn word_of(word: &str, rest: &[Token]) -> Result<Word, String> {
    let text = |what: &str| match rest {
        [Token::Text(t)] if t.chars().count() <= MAX_HINT_TEXT => Ok(t.clone()),
        [Token::Text(_)] => Err(format!(
            "{word}'s text is longer than {MAX_HINT_TEXT} characters"
        )),
        _ => Err(format!("{word} takes {what} in quotes: {word} \"…\"")),
    };
    match word {
        "range" => match rest {
            [Token::Number(low), Token::Number(high)] if low < high => Ok(Word::Range(*low, *high)),
            [Token::Number(low), Token::Number(high)] => Err(format!(
                "range's first number is where it starts, so it must be smaller than the \
                 second: range {high} {low}, not range {low} {high}"
            )),
            _ => Err("range takes two numbers, where it starts and ends: range 0 10".into()),
        },
        "step" => match rest {
            [Token::Number(step)] if *step > 0.0 => Ok(Word::Step(*step)),
            [Token::Number(_)] => Err("step must be more than 0: step 0.5".into()),
            _ => Err("step takes one number: step 0.5".into()),
        },
        "unit" => text("a short unit").map(Word::Unit),
        "tip" => text("a sentence").map(Word::Tip),
        "group" => text("a heading").map(Word::Group),
        "multiline" | "hidden" if !rest.is_empty() => Err(format!("{word} takes nothing after it")),
        "multiline" => Ok(Word::Multiline),
        "hidden" => Ok(Word::Hidden),
        other => Err(format!("{other} isn't a hint Solis knows: {WORDS}")),
    }
}

/// The most entries a Lua field's hints table is read for.
const MOST_LUA_HINTS: usize = 32;

/// A Lua field's hints table (`{ range = { 0, 10 }, step = 0.5, unit =
/// "m/s", tip = "…", group = "…", multiline = true, hidden = true }`),
/// read raw so game code's metatables never run, and a sentence for each
/// mistake.
pub(crate) fn from_lua(hints: &mlua::Table) -> (Vec<Word>, Vec<String>) {
    use mlua::Value as Lua;
    let mut words = Vec::new();
    let mut mistakes = Vec::new();
    let text = |value: &Lua| match value {
        Lua::String(s) => s.to_str().ok().map(|s| s.to_string()),
        _ => None,
    };
    for pair in hints.pairs::<Lua, Lua>().take(MOST_LUA_HINTS) {
        let Ok((key, value)) = pair else {
            mistakes.push("its hints table couldn't be read".into());
            continue;
        };
        let Some(key) = text(&key) else {
            mistakes.push(format!(
                "each hint is named, as in {{ tip = \"…\" }}: {WORDS}"
            ));
            continue;
        };
        let word = match (key.as_str(), &value) {
            ("range", Lua::Table(t)) => {
                let low = t.raw_get::<f64>(1).ok();
                let high = t.raw_get::<f64>(2).ok();
                match (low, high) {
                    (Some(low), Some(high)) => {
                        word_of("range", &[Token::Number(low), Token::Number(high)])
                    }
                    _ => Err(
                        "range takes two numbers, where it starts and ends: range = { 0, 10 }"
                            .into(),
                    ),
                }
            }
            ("step", Lua::Number(n)) => word_of("step", &[Token::Number(*n)]),
            ("step", Lua::Integer(n)) => word_of("step", &[Token::Number(*n as f64)]),
            ("unit" | "tip" | "group", value) => match text(value) {
                Some(t) => word_of(&key, &[Token::Text(t)]),
                None => Err(format!("{key} takes text: {key} = \"…\"")),
            },
            ("multiline" | "hidden", Lua::Boolean(true)) => word_of(&key, &[]),
            ("multiline" | "hidden", Lua::Boolean(false)) => continue,
            ("multiline" | "hidden", _) => Err(format!("{key} is true or false: {key} = true")),
            ("range" | "step", _) => Err(format!(
                "{key} takes numbers: range = {{ 0, 10 }}, step = 0.5"
            )),
            (other, _) => Err(format!("{other} isn't a hint Solis knows: {WORDS}")),
        };
        match word {
            Ok(word) => words.push(word),
            Err(mistake) => mistakes.push(mistake),
        }
    }
    // A table's pairs come in no fixed order: said the same way each time.
    mistakes.sort();
    (words, mistakes)
}

/// A token as written, for mistakes.
fn shown(token: &Token) -> String {
    match token {
        Token::Word(w) => w.clone(),
        Token::Number(n) => n.to_string(),
        Token::Text(t) => format!("\"{t}\""),
        Token::Comma => ",".into(),
    }
}

/// `inside` as words, numbers, text in quotes and commas.
fn tokens(inside: &str) -> Result<Vec<Token>, String> {
    let mut out = Vec::new();
    let mut chars = inside.char_indices().peekable();
    while let Some((at, c)) = chars.next() {
        match c {
            c if c.is_whitespace() => {}
            ',' => out.push(Token::Comma),
            '"' => {
                let mut text = String::new();
                let mut closed = false;
                while let Some((_, c)) = chars.next() {
                    match c {
                        '"' => {
                            closed = true;
                            break;
                        }
                        '\\' => text.extend(chars.next().map(|(_, c)| c)),
                        c => text.push(c),
                    }
                }
                if !closed {
                    return Err("a hint's text has no closing \"".into());
                }
                out.push(Token::Text(text));
            }
            c if c.is_ascii_digit() || matches!(c, '-' | '+' | '.') => {
                let mut end = at + c.len_utf8();
                while let Some(&(next, c)) = chars.peek() {
                    if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+') {
                        end = next + c.len_utf8();
                        chars.next();
                    } else {
                        break;
                    }
                }
                let written = &inside[at..end];
                match written.parse::<f64>() {
                    Ok(n) if n.is_finite() => out.push(Token::Number(n)),
                    _ => return Err(format!("{written} isn't a number")),
                }
            }
            c if c.is_alphabetic() || c == '_' => {
                let mut word = String::from(c);
                while let Some(&(_, c)) = chars.peek() {
                    if c.is_alphanumeric() || c == '_' {
                        word.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                out.push(Token::Word(word));
            }
            other => return Err(format!("{other} has no place in a hint")),
        }
    }
    Ok(out)
}

/// Whether `kind` holds numbers a range, a step and a unit fit.
fn numbers(kind: &FieldKind) -> bool {
    matches!(
        kind,
        FieldKind::Int | FieldKind::Float | FieldKind::Vec2 | FieldKind::Vec3
    )
}

/// Puts `words` on `field`'s hint, and gives a sentence for each that
/// doesn't fit its kind (`kind` is how its type was written, for them).
pub(crate) fn apply(field: &mut FieldInfo, words: Vec<Word>, kind: &str) -> Vec<String> {
    let mut mistakes = Vec::new();
    let name = field.name.clone();
    for word in words {
        let fits = match word {
            Word::Range(..) | Word::Step(_) | Word::Unit(_) => numbers(&field.kind),
            Word::Multiline => field.kind == FieldKind::Text,
            Word::Tip(_) | Word::Group(_) | Word::Hidden => true,
        };
        if !fits {
            let wants = if word == Word::Multiline {
                "text (a string)"
            } else {
                "numbers (int, float, vec2 and vec3)"
            };
            mistakes.push(format!(
                "{} is for {wants}, and {name} is a{} {kind}",
                word.name(),
                if kind.starts_with(['a', 'e', 'i', 'o', 'u']) {
                    "n"
                } else {
                    ""
                }
            ));
            continue;
        }
        let hint = &mut field.hint;
        match word {
            Word::Range(low, high) => {
                hint.range = Some((low, high));
                hint.slider = matches!(field.kind, FieldKind::Int | FieldKind::Float);
            }
            Word::Step(step) if field.kind == FieldKind::Int && step.fract() != 0.0 => {
                mistakes.push(format!(
                    "{name} is a whole number, so its step must be one too: step 1"
                ));
            }
            Word::Step(step) => hint.step = Some(step),
            Word::Unit(unit) => hint.unit = Cow::Owned(format!(" {unit}")),
            Word::Tip(tip) => {
                hint.help = Cow::Owned(tip.clone());
                hint.tip = Cow::Owned(tip);
            }
            Word::Group(group) => hint.group = Cow::Owned(group),
            Word::Multiline => hint.multiline = true,
            Word::Hidden => hint.hidden = true,
        }
    }
    mistakes
}

/// A sentence when `field` starts outside its range: Properties keeps
/// what's dragged or typed within it, but a starting value is the code's.
pub(crate) fn outside_range(field: &FieldInfo) -> Option<String> {
    let (low, high) = field.hint.range?;
    let out = |n: f32| f64::from(n) < low || f64::from(n) > high;
    let outside = match &field.default {
        Value::Int(_) | Value::Float(_) => field.default.as_f32().is_some_and(out),
        list @ Value::List(_) => match field.kind {
            FieldKind::Vec2 => list
                .as_floats::<2>()
                .is_some_and(|v| v.into_iter().any(out)),
            FieldKind::Vec3 => list
                .as_floats::<3>()
                .is_some_and(|v| v.into_iter().any(out)),
            _ => false,
        },
        _ => false,
    };
    outside.then(|| {
        format!(
            "{} starts outside its range ({low} to {high}): Properties keeps what's dragged or \
             typed within it, so this starting value can't be dragged back to",
            field.name
        )
    })
}

#[cfg(test)]
#[path = "field_hints_tests.rs"]
mod tests;
