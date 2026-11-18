//! Which state each Lust component has of its own, and the event it
//! hears when that changes (stage 1, states; decision 0126).
//!
//! An event's types are one set for the whole program, so no single
//! engine event can carry every component's own state. The game declares
//! one for each of its states beside it, named after it: `MoodChanged`
//! for a `Mood`, carrying the option before and the option now
//! (`event MoodChanged(Mood from, Mood to);`). A component whose own
//! state is a `Mood` hears it when its state changes. Lust's compiled
//! code doesn't say which state a component's own is (only its options),
//! so the name is read from where the component sets its starting option
//! (`state = Mood.Calm;` in its `has`), comments and text skipped.

use crate::code::{Code, CodeProblem};
use crate::events::EventKinds;
use reflect::{ComponentSchema, FieldKind};
use std::collections::BTreeMap;

/// The ending of the event a state's change is heard as: `Mood` gives
/// `MoodChanged`.
pub const CHANGED: &str = "Changed";

/// The most components whose state is looked for: past it, the rest
/// hear no change (a guard against generated code).
const MAX_COMPONENTS: usize = 4_096;

/// Each Lust component with a state of its own, and that state's name:
/// `Hero` → `Mood` for `component Hero { has { state = Mood.Calm; } }`.
pub fn state_types(code: &Code) -> BTreeMap<String, String> {
    let mut found = BTreeMap::new();
    for (_, text) in &code.files {
        let words = crate::timer_names::words_of(text);
        let mut i = 0;
        while i < words.len() {
            let component = words[i] == "component"
                && words.get(i + 2).is_some_and(|w| w == "{")
                && words.get(i + 1).is_some_and(|w| is_name(w));
            if !component {
                i += 1;
                continue;
            }
            let end = closing(&words, i + 2);
            if let Some(state) = own_state(&words[i + 3..end]) {
                found.insert(words[i + 1].clone(), state);
                if found.len() >= MAX_COMPONENTS {
                    return found;
                }
            }
            i = end;
        }
    }
    found
}

/// The state named in `body`'s `has { … state = Name.Option … }`, at the
/// top level of the component.
fn own_state(body: &[String]) -> Option<String> {
    let mut depth = 0_usize;
    let mut in_has = None;
    for (i, word) in body.iter().enumerate() {
        match word.as_str() {
            "{" => {
                if depth == 0 && i > 0 && body[i - 1] == "has" {
                    in_has = Some(1);
                }
                depth += 1;
            }
            "}" => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    in_has = None;
                }
            }
            "state" if in_has == Some(depth) => {
                if let [eq, name, dot, ..] = &body[i + 1..]
                    && eq == "="
                    && dot == "."
                    && is_name(name)
                {
                    return Some(name.clone());
                }
            }
            _ => {}
        }
    }
    None
}

/// Where the `{` at `open` closes (or the end).
fn closing(words: &[String], open: usize) -> usize {
    let mut depth = 0_usize;
    for (i, word) in words.iter().enumerate().skip(open) {
        match word.as_str() {
            "{" => depth += 1,
            "}" => {
                depth -= 1;
                if depth == 0 {
                    return i;
                }
            }
            _ => {}
        }
    }
    words.len()
}

/// A name Lust could give a component or a state.
fn is_name(word: &str) -> bool {
    word.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
        && word.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// The event each component with a state of its own hears when it
/// changes, by component: its state's name and [`CHANGED`], when the game
/// declares it carrying two of that state. One declared otherwise is a
/// mistake, at the component, saying how to declare it.
pub(crate) fn state_events(
    types: &BTreeMap<String, String>,
    schemas: &[ComponentSchema],
    events: &EventKinds,
    places: &BTreeMap<String, (String, u32)>,
) -> Result<BTreeMap<String, String>, Vec<CodeProblem>> {
    let mut heard = BTreeMap::new();
    let mut problems = Vec::new();
    for schema in schemas {
        let Some(options) = schema.fields.iter().find_map(|f| match &f.kind {
            FieldKind::Choice(options) if f.name == "state" => Some(options),
            _ => None,
        }) else {
            continue;
        };
        let Some(state) = types.get(&schema.name) else {
            continue;
        };
        let event = format!("{state}{CHANGED}");
        let Some(kinds) = events.get(&event) else {
            continue;
        };
        let carries_two = matches!(
            kinds.as_slice(),
            [FieldKind::Choice(a), FieldKind::Choice(b)] if a == options && b == options
        );
        if carries_two {
            heard.insert(schema.name.clone(), event);
            continue;
        }
        let (file, line) = places.get(&schema.name).cloned().unwrap_or_default();
        problems.push(CodeProblem {
            file,
            line,
            column: 0,
            message: format!(
                "{event} is what {name} hears when its own state changes, so it carries the \
                 option before and the option now: declare it as event {event}({state} from, \
                 {state} to);",
                name = schema.name
            ),
        });
    }
    if problems.is_empty() {
        Ok(heard)
    } else {
        Err(problems)
    }
}
