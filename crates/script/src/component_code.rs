//! The game's components told to the rest of its code (Fixes 2a;
//! [the manual](../../../docs/manual/game-code.md#reading-other-entities)):
//! `solis/components.lst`, written each time the code is checked, from the
//! components in the game's Lust.
//!
//! - `state Component`, every component, for `World:the(Component.Quest)`.
//! - For each component, a class of its fields to read and never change
//!   (`KnightReading`, every field `lock`ed, so writing to one is a
//!   mistake before the game runs), and `method KnightReading
//!   entity:knight() native;`, a copy of another entity's as the last
//!   tick left it.
//! - For each component, a class to give one spawned its first values
//!   (`KnightValues`, like `SpawnValues`), with the component's fields:
//!   `new KnightValues { coins = 3 }` gives the fields written (Fixes 2b,
//!   `crate::readings::values_new`).
//!
//! A field's type is fetched from where the component's own file gets it:
//! a name the file fetches from the file it fetched it from, a name the
//! file declares from the file itself. A name declared there but kept to
//! itself is shared by adding `pass Name;` before the file's closing
//! `end`, on the same line, so no line of the game's moves. A field holding
//! a thing (an animator's class) isn't in the reading.

use std::collections::{BTreeMap, BTreeSet};

/// The file's name inside the engine's folder: `solis/components.lst`.
pub const FILE: &str = "components";

/// The class every component's values class is like, in this file:
/// `World:spawnWith` takes a list of them.
pub const VALUES_BASE: &str = "SpawnValues";

/// The state of the game's components.
pub const STATE: &str = "Component";

/// The most components, and fields of each, the file is written for.
const MOST: usize = 4096;

/// A field a component's reading and values hold: its Lust type, its name
/// and the starting value the reading class gives it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Field {
    kind: String,
    name: String,
    start: String,
    /// The component's own starting value, when it's a plain value that
    /// reads the same here (`3`, `true`, `"Ada"`, `Mood.Calm`, `[0, 1]`):
    /// a values object's field not written reads as it.
    own: Option<String>,
}

/// Whether `start`, written for a field of kind `kind`, is a plain value
/// that means the same in another file: a number, a bool, text holding no
/// value, an option of the field's state, or a list of numbers.
fn plain(kind: &str, start: &str) -> bool {
    let number = |t: &str| {
        let t = t.trim().strip_prefix('-').unwrap_or(t.trim());
        !t.is_empty() && t.chars().all(|c| c.is_ascii_digit() || c == '.')
    };
    match kind {
        "int" | "float" => number(start),
        "bool" => start == "true" || start == "false",
        "string" => {
            start.len() >= 2
                && start.starts_with('"')
                && start.ends_with('"')
                && !start[1..start.len() - 1].contains(['"', '{', '\\'])
        }
        "vec3" | "vec2" | "rotation" => start
            .strip_prefix('[')
            .and_then(|s| s.strip_suffix(']'))
            .is_some_and(|inside| inside.split(',').all(number)),
        _ => option_of(kind, start),
    }
}

/// What the game's code reads of other entities' components, for
/// [`crate::Compiled`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Reach {
    /// The kinds its code reads or asks `World:the` for: copied each tick.
    pub reads: Vec<String>,
    /// Each Lust component game code can read, with its reading method.
    pub readable: Vec<(String, String)>,
    /// What game code can't reach, in plain words (the editor shows them).
    pub notes: Vec<String>,
}

/// What [`write()`] made.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ComponentCode {
    /// The game's files as Lust is handed them: `solis/components.lst`
    /// written (in place of the one on disk), and `pass` added to a file
    /// whose names a reading needs.
    pub files: Vec<(String, String)>,
    /// Each component game code can read, with its reading method's name
    /// (`Knight`, `knight`).
    pub readable: Vec<(String, String)>,
    /// Each component with a values class, and that class's name.
    pub with_values: Vec<(String, String)>,
    /// What game code can't reach, in plain words.
    pub notes: Vec<String>,
}

/// The file before the code is first checked (and in the screens'
/// program, which reads components its own way): the state with no
/// components in it, so `solis/world.lst` always finds it.
pub fn placeholder(made_by: &str) -> String {
    format!(
        "-- Made by {made_by}: the game's components, for World:the, reading another entity's\n\
         -- and spawning with values. It's written again each time the game's code is checked,\n\
         -- so changes made here don't last.\n\n\
         -- The game's components.\nstate {STATE} = [{}];\n\n{}pass {STATE}, {VALUES_BASE};\nend\n",
        crate::project_names::NO_OPTION,
        values_base()
    )
}

/// The class every values class is like.
fn values_base() -> String {
    format!(
        "-- What every component's values class (KnightValues) is like: World:spawnWith takes \
         a list of them.\nclass {VALUES_BASE} {{\n    has {{\n    }}\n}}\n\n"
    )
}

/// The Lust type and starting value of a field of a built-in type.
fn built_in(kind: &str) -> Option<&'static str> {
    Some(match kind {
        "int" => "0",
        "float" => "0.0",
        "string" => "\"\"",
        "bool" => "false",
        "vec3" | "rotation" => "[0, 0, 0]",
        "vec2" => "[0, 0]",
        "colour" => "[1, 1, 1]",
        "entity" => "nothing",
        _ => return None,
    })
}

/// Whether `start` is an option of the state `kind` written out:
/// `Mode.Calm`, `Clip.KnightWalk`.
fn option_of(kind: &str, start: &str) -> bool {
    start.strip_prefix(kind).is_some_and(|rest| {
        rest.starts_with('.')
            && rest[1..].split('.').all(|part| {
                part.starts_with(|c: char| c.is_ascii_alphabetic())
                    && part.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            })
    })
}

/// The module a file is fetched as: its path without `.lst`.
fn module(path: &str) -> &str {
    path.strip_suffix(".lst").unwrap_or(path)
}

/// The game's files reached from `entry` by their fetches, outside the
/// engine's folder: only their components are part of the game.
fn reached(files: &[(String, String)], entry: &str) -> BTreeSet<usize> {
    let place: BTreeMap<String, usize> = files
        .iter()
        .enumerate()
        .map(|(at, (path, _))| (path.clone(), at))
        .collect();
    let mut seen = BTreeSet::new();
    let mut todo: Vec<usize> = place.get(entry).copied().into_iter().collect();
    while let Some(at) = todo.pop() {
        if !seen.insert(at) {
            continue;
        }
        for path in level::lust_guard::fetched(&files[at].1) {
            if let Some(&next) = place.get(&crate::reach::file_of(path))
                && !crate::describe::in_engine_folder(&files[next].0)
            {
                todo.push(next);
            }
        }
    }
    seen
}

/// `solis/components.lst` for the game's Lust `files` (starting at
/// `entry`), with the engine's folder `folder`; see the module's notes.
pub fn write(folder: &str, files: &[(String, String)], entry: &str) -> ComponentCode {
    let reached = reached(files, entry);
    let taken: BTreeSet<&str> = crate::describe::NATIVES
        .iter()
        .filter(|n| n.owner == "entity")
        .map(|n| n.name)
        // Lust's own: an entity's name as text.
        .chain(["text"])
        .collect();
    // Everything the game declares that Lust orders by name: a class of
    // Solis's by one of those names would be two things with one name.
    let mut declared: BTreeSet<&str> = BTreeSet::new();
    // Each component, the file it's in, and its fields as written.
    let mut components: BTreeMap<&str, usize> = BTreeMap::new();
    for &at in &reached {
        for (_, found) in level::lust_guard::scan(&files[at].1) {
            if let level::lust_guard::Found::Declared(word, name) = found {
                declared.insert(name);
                if word == "component" && components.len() < MOST {
                    components.entry(name).or_insert(at);
                }
            }
        }
    }
    let mut code = ComponentCode::default();
    // Each name fetched into the file, and the module it comes from.
    let mut fetched: BTreeMap<String, String> = BTreeMap::new();
    let ours: BTreeSet<String> = [STATE.to_string(), VALUES_BASE.to_string()]
        .into_iter()
        .collect();
    // The names each file must share that it keeps to itself, by file.
    let mut to_pass: BTreeMap<usize, BTreeSet<String>> = BTreeMap::new();
    let mut classes = String::new();
    let mut readers = String::new();
    let mut passed: Vec<String> = vec![STATE.to_string(), VALUES_BASE.to_string()];
    let mut names_made: BTreeSet<String> = BTreeSet::new();
    for (&component, &at) in &components {
        let (path, text) = &files[at];
        let shared = level::lust_guard::shared(text, MOST);
        let written: Vec<level::lust_fields::WrittenField<'_>> =
            level::lust_fields::fields(text, MOST)
                .into_iter()
                .filter(|f| f.component == component)
                .collect();
        let mut fields = Vec::new();
        for field in written {
            let start = match built_in(field.kind) {
                Some(start) => start.to_string(),
                None if option_of(field.kind, field.start) => field.start.to_string(),
                // A thing (an animator's class, filled in by the engine) or
                // a value that isn't written out: not read.
                None => continue,
            };
            if built_in(field.kind).is_none() {
                let from = shared
                    .fetched
                    .iter()
                    .find(|(_, name)| *name == field.kind)
                    .map(|(from, _)| from.to_string());
                let source = from.clone().unwrap_or_else(|| module(path).to_string());
                let clash = ours.contains(field.kind)
                    || names_made.contains(field.kind)
                    || fetched.get(field.kind).is_some_and(|s| *s != source);
                if clash || (from.is_none() && shared.end.is_none()) {
                    code.notes.push(format!(
                        "{component}'s {} can't be read from other entities or given to a spawned \
                         one: another file has a {} too. Give one of them another name.",
                        field.name, field.kind
                    ));
                    continue;
                }
                if from.is_none() && !shared.passed.contains(&field.kind) {
                    to_pass
                        .entry(at)
                        .or_default()
                        .insert(field.kind.to_string());
                }
                fetched.insert(field.kind.to_string(), source);
            }
            fields.push(Field {
                kind: field.kind.to_string(),
                name: field.name.to_string(),
                own: plain(field.kind, field.start).then(|| field.start.to_string()),
                start,
            });
        }
        let reading = format!("{component}Reading");
        let values = format!("{component}Values");
        let reader = crate::screens::reader_name(component);
        let free = |name: &str| !declared.contains(name) && !ours.contains(name);
        if !free(&reading) {
            code.notes.push(format!(
                "Other entities' {component} can't be read: the game has a {reading} of its own. \
                 Give it another name to read {component} from game code."
            ));
        } else if taken.contains(reader.as_str())
            || !crate::screens::lust_name(&reader)
            || !crate::screens::lust_name(component)
        {
            code.notes.push(format!(
                "Other entities' {component} can't be read: entities already have a {reader}(). \
                 Give the component another name to read it from game code."
            ));
        } else {
            classes.push_str(&format!(
                "-- {component}'s fields as the last tick left them: read, never changed.\n\
                 class {reading} {{\n    has {{\n"
            ));
            for f in &fields {
                classes.push_str(&format!(
                    "        lock {} {} = {};\n",
                    f.kind, f.name, f.start
                ));
            }
            classes.push_str("    }\n}\n\n");
            readers.push_str(&format!(
                "-- A copy of the entity's {component} as the last tick left it (every core sees \
                 the same), or nothing when it has none.\n\
                 method {reading} entity:{reader}() native;\n\n"
            ));
            code.readable.push((component.to_string(), reader));
            passed.push(reading.clone());
            names_made.insert(reading);
        }
        if free(&values) {
            // Its fields are the component's: only those the game writes
            // (`new PickupValues { item = Item.Red }`) are given
            // (`crate::readings::values_new`).
            classes.push_str(&format!(
                "-- First values for a {component} spawned with World:spawnWith, in place before \
                 its first update: only the fields written are given.\nclass {values} like \
                 {VALUES_BASE} {{\n    has {{\n"
            ));
            for f in &fields {
                classes.push_str(&format!(
                    "        {} {} = {};\n",
                    f.kind,
                    f.name,
                    f.own.as_deref().unwrap_or(&f.start)
                ));
            }
            classes.push_str("    }\n}\n\n");
            code.with_values
                .push((component.to_string(), values.clone()));
            passed.push(values.clone());
            names_made.insert(values);
        } else {
            code.notes.push(format!(
                "A spawned {component} can't be given values: the game has a {values} of its own."
            ));
        }
    }
    let mut text = String::from(
        "-- Made from the game's components: what game code can read of another entity's, and\n\
         -- give one it spawns. It's written again each time the game's code is checked, so\n\
         -- changes made here don't last.\n\n",
    );
    let mut by_module: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (name, from) in &fetched {
        by_module.entry(from).or_default().push(name);
    }
    for (from, names) in &by_module {
        text.push_str(&format!("fetch \"{from}\":[{}];\n", names.join(", ")));
    }
    if !by_module.is_empty() {
        text.push('\n');
    }
    let options: Vec<&str> = components.keys().copied().collect();
    text.push_str(&format!(
        "-- The game's components.\nstate {STATE} = [{}];\n\n",
        if options.is_empty() {
            crate::project_names::NO_OPTION.to_string()
        } else {
            options.join(", ")
        }
    ));
    text.push_str(&values_base());
    text.push_str(&classes);
    text.push_str(&readers);
    text.push_str(&format!("pass {};\nend\n", passed.join(", ")));
    let ours_path = format!("{folder}/{FILE}.lst");
    code.files = files
        .iter()
        .enumerate()
        .filter(|(_, (path, _))| *path != ours_path)
        .map(|(at, (path, original))| {
            let mut text = original.clone();
            if let (Some(names), Some(end)) = (
                to_pass.get(&at),
                level::lust_guard::shared(original, MOST).end,
            ) {
                let names: Vec<&str> = names.iter().map(String::as_str).collect();
                text.insert_str(end, &format!("pass {}; ", names.join(", ")));
            }
            (path.clone(), text)
        })
        .chain(std::iter::once((ours_path.clone(), text)))
        .collect();
    code
}

#[cfg(test)]
#[path = "component_code_tests.rs"]
mod tests;
