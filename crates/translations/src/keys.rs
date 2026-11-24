//! Text keys: the names lines of text are found by, made once and never
//! changed. A key is a group and a name, more parts if wanted, each a name
//! Lust can use: `Shop.Welcome`, `Menu.Options.Volume`,
//! `Items.IronSword.description`. Game code names it as an option of
//! `Text` (`Text.ShopWelcome`), so a misspelt key is a mistake before
//! the game runs.
//!
//! In a Fluent file, a key's dots are dashes: `Shop-Welcome = Welcome!`
//! (Fluent's names can't hold dots). A key's parts hold no dashes, so the
//! two always turn into each other.

/// The longest a key may be, in characters.
pub const MAX_KEY_CHARS: usize = 200;

/// The longest one part of a key may be, in characters.
pub const MAX_PART_CHARS: usize = 60;

/// Lust's own objects, which no part may be.
const LUST_OBJECTS: &[&str] = &["Maths", "Files", "Time", "Json", "Web", "Random"];

/// Why `key` can't be a text key, in plain words; `None` when it can.
pub fn key_problem(key: &str) -> Option<String> {
    let example = "A key is a group and a name joined by a dot, such as Shop.Welcome";
    if key.chars().count() > MAX_KEY_CHARS {
        return Some(format!("A key is at most {MAX_KEY_CHARS} characters."));
    }
    let parts: Vec<&str> = key.split('.').collect();
    if parts.len() < 2 {
        return Some(format!("{example}."));
    }
    for part in parts {
        if let Some(why) = part_problem(part) {
            return Some(format!("{why} {example}."));
        }
    }
    None
}

/// Why one part of a key can't be one.
fn part_problem(part: &str) -> Option<String> {
    let Some(first) = part.chars().next() else {
        return Some("A key has an empty part.".into());
    };
    if part.chars().count() > MAX_PART_CHARS {
        return Some(format!(
            "Each part of a key is at most {MAX_PART_CHARS} characters."
        ));
    }
    if !first.is_ascii_alphabetic() {
        return Some(format!("{part} doesn't start with a letter."));
    }
    if !part.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Some(format!(
            "{part} holds something other than letters, digits and _."
        ));
    }
    let letters: Vec<char> = part.chars().filter(char::is_ascii_alphabetic).collect();
    if letters.len() >= 2 && letters.iter().all(char::is_ascii_uppercase) {
        return Some(format!(
            "{part} is all capitals, which Lust keeps for its own."
        ));
    }
    if LUST_OBJECTS.iter().any(|o| o.eq_ignore_ascii_case(part)) {
        return Some(format!("{part} is one of Lust's own names."));
    }
    None
}

/// The name a key has in a Fluent file: `Shop-Welcome` for `Shop.Welcome`.
pub fn fluent_id(key: &str) -> String {
    key.replace('.', "-")
}

/// The key a Fluent message's name stands for (`Shop-Welcome` is
/// `Shop.Welcome`), or why it can't be one.
pub fn key_of(id: &str) -> Result<String, String> {
    let key = id.replace('-', ".");
    match key_problem(&key) {
        None => Ok(key),
        Some(why) => Err(why),
    }
}

/// The file a new key's lines go in, in each language's folder: its
/// group's name in small letters (`shop.ftl` for `Shop.Welcome`).
pub fn file_for(key: &str) -> String {
    let group = key.split('.').next().unwrap_or("text");
    format!("{}.ftl", group.to_ascii_lowercase())
}

/// The key `key` as a path of the `Text` state's options
/// (`Shop/Welcome`), the shape the project's names are given in.
pub fn as_path(key: &str) -> String {
    key.replace('.', "/")
}

/// The object game code asks for a group's lines with values:
/// `ShopText` for `Shop`.
pub fn group_object(group: &str) -> String {
    let mut chars = group.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_ascii_uppercase().to_string() + chars.as_str() + "Text"
    })
}

/// A line's method on its group's object: the key's parts after the
/// group, joined (`welcome` for `Shop.Welcome`, `optionsVolume` for
/// `Menu.Options.Volume`).
pub fn method_name(key: &str) -> String {
    let mut out = String::new();
    for (n, part) in key.split('.').skip(1).enumerate() {
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            if n == 0 {
                out.push(first.to_ascii_lowercase());
            } else {
                out.push(first.to_ascii_uppercase());
            }
            out.push_str(chars.as_str());
        }
    }
    out
}

/// A Fluent value's name as Lust names a method's value: `playerName` for
/// `player-name` or `player_name`. `None` when it can't be one.
pub fn value_name(name: &str) -> Option<String> {
    let mut out = String::new();
    let mut capital = false;
    for c in name.chars() {
        if c == '-' || c == '_' {
            capital = !out.is_empty();
            continue;
        }
        if !c.is_ascii_alphanumeric() {
            return None;
        }
        if out.is_empty() && !c.is_ascii_alphabetic() {
            return None;
        }
        if capital {
            out.push(c.to_ascii_uppercase());
            capital = false;
        } else if out.is_empty() {
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    (!out.is_empty()).then_some(out)
}

/// The key nearest `key` by spelling among `keys`, for a message.
pub fn nearest<'a>(key: &str, keys: impl Iterator<Item = &'a str>) -> Option<&'a str> {
    let lower = key.to_ascii_lowercase();
    keys.take(20_000)
        .map(|k| (distance(&lower, &k.to_ascii_lowercase()), k))
        .filter(|(d, _)| *d <= 3.max(key.len() / 3))
        .min_by_key(|(d, _)| *d)
        .map(|(_, k)| k)
}

/// How many letters must change to make `a` into `b`.
fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut last: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut now = vec![i + 1; b.len() + 1];
        for (j, cb) in b.iter().enumerate() {
            let swap = last[j] + usize::from(ca != *cb);
            now[j + 1] = swap.min(last[j + 1] + 1).min(now[j] + 1);
        }
        last = now;
    }
    last[b.len()]
}

#[cfg(test)]
#[path = "keys_tests.rs"]
mod tests;
