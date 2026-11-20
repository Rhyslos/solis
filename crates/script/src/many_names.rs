//! Programs with any number of top-level names (Fixes 1, B1). Lust's chunk
//! keeps every class, method, object and state of every file a program
//! fetches as a local of its main function (`local Knight, walk, …` near
//! its top), and Luau allows a function 200 locals and 255 registers: a
//! screen fetching a file of a few hundred methods failed with "Out of
//! local registers" as play started.
//!
//! [`fit`] keeps the first [`KEPT`] of them as locals, and puts the rest in
//! the program's own globals instead (each Lust program has a table of its
//! own for them, never shared with a Lua file or Solis). The code means the
//! same: a top-level local is seen by everything after it, and so is a
//! global; reading one is a little slower, so the names first written,
//! which the rest are built on, stay locals. A name Lust's runtime uses
//! too (`type`, `next`, `vector`) always stays a local, so a game's method
//! called `next` never becomes the runtime's. Programs with fewer names
//! are left exactly as they were, and no line moves, so a failure still
//! names its file and line.
//!
//! What else could overflow was checked with programs far bigger than games
//! write (the tests): a class's methods and fields are a table's fields,
//! states' options and events are tables, and no function Lust writes
//! holds more than its own locals, so only the main function's count grew
//! with the program.

use std::collections::BTreeSet;

/// The most top-level names kept as locals: under Luau's 200, with room for
/// those that must stay ([`fit`]) and the main function's own working
/// registers.
pub(crate) const KEPT: usize = 120;

/// Where Lust's chunk closes its runtime (`local _L = (function() …
/// end)()`): the program's own code comes after.
const RUNTIME_END: &str = "\nend)()\n";

/// `chunk` with its top-level names past [`KEPT`] as its own globals.
pub(crate) fn fit(mut chunk: lust::LuaChunk) -> lust::LuaChunk {
    let end = chunk
        .code
        .find(RUNTIME_END)
        .map_or(0, |at| at + RUNTIME_END.len());
    let (runtime, program) = chunk.code.split_at(end);
    let Some(program) = fitted(program, &words(runtime)) else {
        return chunk;
    };
    chunk.code = format!("{runtime}{program}");
    chunk
}

/// The program's code with its names past [`KEPT`] made globals, or `None`
/// when it has no more than that. `taken`: names that stay locals however
/// many there are.
fn fitted(program: &str, taken: &BTreeSet<&str>) -> Option<String> {
    let lines: Vec<&str> = program.split('\n').collect();
    let total: usize = lines.iter().map(|line| declared(line).len()).sum();
    if total <= KEPT {
        return None;
    }
    let mut kept = 0;
    let mut keep = |name: &str| {
        let stays = kept < KEPT || taken.contains(name) || !spillable(name);
        if stays {
            kept += 1;
        }
        stays
    };
    let mut out = Vec::with_capacity(lines.len());
    for line in lines {
        let names = declared(line);
        if names.is_empty() {
            out.push(line.to_string());
        } else if forward(line).is_some() {
            let staying: Vec<&str> = names.into_iter().filter(|n| keep(n)).collect();
            // An empty line where none stay, so no line moves.
            out.push(if staying.is_empty() {
                String::new()
            } else {
                format!("local {}", staying.join(", "))
            });
        } else if names.iter().all(|n| keep(n)) {
            out.push(line.to_string());
        } else {
            // `local Name = …` becomes `Name = …`.
            out.push(line["local ".len()..].to_string());
        }
    }
    Some(out.join("\n"))
}

/// The names a top-level line of the program declares: every name of a
/// `local A, B, C` with nothing given them (Lust's names written up front),
/// or the one name of `local Name = …`. Anything else declares none that
/// can move (a line inside a function is indented, and a `local a, b = …`
/// is kept as it is, counted by [`KEPT`]'s room).
fn declared(line: &str) -> Vec<&str> {
    if let Some(list) = forward(line) {
        return list.split(',').map(str::trim).collect();
    }
    let Some(rest) = line.strip_prefix("local ") else {
        return Vec::new();
    };
    let name_len = rest
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(rest.len());
    let (name, after) = rest.split_at(name_len);
    if is_name(name) && after.starts_with(" = ") {
        vec![name]
    } else {
        Vec::new()
    }
}

/// The names of a `local A, B, C` line with nothing given them.
fn forward(line: &str) -> Option<&str> {
    let list = line.strip_prefix("local ")?;
    list.split(',')
        .all(|name| is_name(name.trim()))
        .then_some(list)
}

/// Whether `word` is a Lua name (`function` and the other words Lua keeps
/// aren't: Lust adds `_` to those).
fn is_name(word: &str) -> bool {
    let mut chars = word.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !matches!(word, "function" | "local" | "end" | "nil" | "true" | "false")
}

/// Whether a name may become a global: not one of the names a host gives
/// every program (`_G`, `LUST_NATIVES` and Solis's own, all starting with
/// `_` or in capitals), which Lust's names never are.
fn spillable(name: &str) -> bool {
    !(name == "_G" || name == "_L" || name.starts_with("__") || name.starts_with("LUST"))
}

/// Every word in `text` that could be a name: what Lust's runtime reads,
/// and so what the program's names must never be as globals.
fn words(text: &str) -> BTreeSet<&str> {
    text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .filter(|w| !w.is_empty())
        .collect()
}

#[cfg(test)]
#[path = "many_names_tests.rs"]
mod tests;
