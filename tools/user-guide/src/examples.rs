//! Game-code examples in three languages (decision 0113). In the manual,
//! an example is three fenced blocks one after another, `lust`, `rust`
//! and `lua`; the guide shows them as one example with a switch above it.
//! Code that isn't game code (commands, files, settings) is fenced with
//! no language or another one, and stays as it is.
//!
//! Here too are the checks that need no engine: each example has all
//! three, its Rust's brackets balance, and its Lua and Rust call only
//! what game code can call (`docs/game-code-interface.txt`). Its Lust,
//! and its Lua's syntax, are checked against the engine itself by the
//! script crate's tests (`crates/script/src/manual_examples_tests.rs`).

use crate::blocks::Block;

/// The three languages, in their order: the fence's word and the name
/// the switch shows.
pub const LANGUAGES: [(&str, &str); 3] = [("lust", "Lust"), ("rust", "Rust"), ("lua", "Lua")];

/// What game code can call, from the repository's top folder.
pub const INTERFACE: &str = "docs/game-code-interface.txt";

/// Whether a fence's word names one of game code's languages.
pub fn is_game_code(language: &str) -> bool {
    LANGUAGES.iter().any(|(word, _)| *word == language)
}

/// The three blocks' code, when `blocks` holds an example starting at
/// `at`: Lust, Rust and Lua, one after another.
pub fn three_at(blocks: &[Block], at: usize) -> Option<[&str; 3]> {
    let mut codes = [""; 3];
    for (n, (word, _)) in LANGUAGES.iter().enumerate() {
        match blocks.get(at + n) {
            Some(Block::Code(language, code)) if language == word => codes[n] = code,
            _ => return None,
        }
    }
    Some(codes)
}

/// Whether code fenced without a language reads as Lust game code, which
/// must be shown in all three.
pub fn looks_like_lust(code: &str) -> bool {
    code.lines().any(|line| {
        let line = line.trim_start();
        ["component ", "fetch \"", "pass ", "object ", "assembly "]
            .iter()
            .any(|start| line.starts_with(start))
    })
}

/// One example, as the manual writes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Example {
    /// The page it's on, without `.md`.
    pub page: String,
    /// Its Lust, Rust and Lua.
    pub code: [String; 3],
}

/// Every example on a page, in order, inside lists and boxes too.
pub fn on_page(page: &str, blocks: &[Block]) -> Vec<Example> {
    let mut found = Vec::new();
    gather(page, blocks, &mut found);
    found
}

fn gather(page: &str, blocks: &[Block], found: &mut Vec<Example>) {
    let mut at = 0;
    while at < blocks.len() {
        if let Some(codes) = three_at(blocks, at) {
            found.push(Example {
                page: page.to_string(),
                code: codes.map(str::to_string),
            });
            at += 3;
            continue;
        }
        match &blocks[at] {
            Block::List(list) => list.items.iter().for_each(|i| gather(page, i, found)),
            Block::Quote(inner) => gather(page, inner, found),
            _ => {}
        }
        at += 1;
    }
}

/// Whether Rust's brackets balance, leaving out strings, characters and
/// comments; if not, what is wrong, with its line.
pub fn brackets_balance(rust: &str) -> Result<(), String> {
    let chars: Vec<char> = rust.chars().collect();
    let mut open: Vec<(char, usize)> = Vec::new();
    let mut line = 1;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        match c {
            '\n' => line += 1,
            '/' if next == Some('/') => {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
                continue;
            }
            '/' if next == Some('*') => {
                i += 2;
                while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                    if chars[i] == '\n' {
                        line += 1;
                    }
                    i += 1;
                }
                i += 2;
                continue;
            }
            '"' => {
                i += 1;
                while i < chars.len() && chars[i] != '"' {
                    if chars[i] == '\\' {
                        i += 1;
                    } else if chars[i] == '\n' {
                        line += 1;
                    }
                    i += 1;
                }
            }
            // A character ('x', '\n'); otherwise a lifetime ('_, 'a).
            '\'' => {
                if next == Some('\\') {
                    i += 3;
                    while i < chars.len() && chars[i] != '\'' {
                        i += 1;
                    }
                } else if chars.get(i + 2) == Some(&'\'') {
                    i += 2;
                }
            }
            '(' | '[' | '{' => open.push((c, line)),
            ')' | ']' | '}' => {
                let wanted = match c {
                    ')' => '(',
                    ']' => '[',
                    _ => '{',
                };
                match open.pop() {
                    Some((opened, _)) if opened == wanted => {}
                    Some((opened, at)) => {
                        return Err(format!(
                            "line {line}: `{c}` closes the `{opened}` opened on line {at}"
                        ));
                    }
                    None => return Err(format!("line {line}: `{c}` closes nothing")),
                }
            }
            _ => {}
        }
        i += 1;
    }
    match open.last() {
        Some((opened, at)) => Err(format!("the `{opened}` opened on line {at} isn't closed")),
        None => Ok(()),
    }
}

/// The calls an example's Lua or Rust makes that game code can't, given
/// the interface's text (`docs/game-code-interface.txt`): Lua's
/// `solis.…` names, and Rust's `host().…` and `cx.…` methods.
pub fn unknown_calls(example: &Example, interface: &str) -> Vec<String> {
    let [_, rust, lua] = &example.code;
    let mut unknown = Vec::new();
    for name in lua_calls(lua) {
        let listed = interface.lines().any(|l| l.trim() == name)
            || interface.contains(&format!("function {name}("));
        if !listed {
            unknown.push(format!("Lua's {name}"));
        }
    }
    for (owner, method) in rust_calls(rust) {
        let wanted = format!("{owner}: ");
        let listed = interface.lines().any(|l| {
            l.strip_prefix(&wanted).is_some_and(|rest| {
                rest.contains(&format!("fn {method}(")) || rest.contains(&format!("fn {method}<"))
            })
        });
        if !listed {
            unknown.push(format!("Rust's {owner}::{method}"));
        }
    }
    unknown
}

/// Every `solis.…` name Lua code uses: `solis.World:say`,
/// `solis.entity.position`, `solis.send`.
fn lua_calls(lua: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut rest = lua;
    while let Some(at) = rest.find("solis.") {
        let before = rest[..at].chars().next_back();
        let tail = &rest[at..];
        let name: String = tail
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | ':' | '_'))
            .collect();
        let name = name.trim_end_matches(['.', ':']).to_string();
        // `solis.World` alone (kept to call through) names an object.
        let is_call = name.matches(['.', ':']).count() >= 2
            || ["solis.send", "solis.facingAlong"].contains(&name.as_str());
        if !before.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_') && is_call {
            names.push(name);
        }
        rest = &rest[at + 6..];
    }
    names.sort();
    names.dedup();
    names
}

/// Every engine method Rust code calls: `host().position(` is the
/// `Host`'s, `cx.send(` and `cx.me(` the `Context`'s.
fn rust_calls(rust: &str) -> Vec<(&'static str, String)> {
    let mut calls = Vec::new();
    for (marker, owner) in [("host().", "Host"), ("cx.", "Context")] {
        let mut rest = rust;
        while let Some(at) = rest.find(marker) {
            let tail = &rest[at + marker.len()..];
            let method: String = tail
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            let called = tail[method.len()..].starts_with('(');
            if called && !(owner == "Context" && method == "host") {
                calls.push((owner, method));
            }
            rest = &rest[at + marker.len()..];
        }
    }
    calls.sort();
    calls.dedup();
    calls
}

#[cfg(test)]
#[path = "examples_tests.rs"]
mod tests;
