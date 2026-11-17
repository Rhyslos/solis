//! Levels were called areas until decision 0118. What a user reads says
//! "level" for the thing that's built, saved and played: every message,
//! label and hint in the program's own code, the user manual (which is
//! the guide, built) and the guide's picture captions. "Area" stays only
//! where it means something else, or names what older files say, and
//! each such place is listed in [`ALLOWED`] with why.

use std::path::{Path, PathBuf};

/// Where "area" is still right: a file (or `*` for any), a phrase that
/// holds the word there (the whole text of a string in code when it
/// starts with `=`; the whole line or string that holds it when it starts
/// with `~`), and why.
const ALLOWED: &[(&str, &str, &str)] = &[
    (
        "*",
        "safe area",
        "the guide on the game-screen canvas: a region of the screen",
    ),
    (
        "text.rs",
        "=area",
        "the word older levels' headers start with (`area version 3;`), still read",
    ),
    (
        "recording.rs",
        "=area",
        "the word of a recording's level line before version 4, still read",
    ),
    (
        "export_dialog.rs",
        "=area",
        "the word of a playtest file's level line in version 1, still read",
    ),
    (
        "save_reading.rs",
        "=area",
        "the name of a save's level section before version 9, still read",
    ),
    (
        "save_reading.rs",
        "its area section",
        "why a version 8 save without that section can't be upgraded",
    ),
    (
        "panel.rs",
        "=area",
        "what a layout saved before decision 0118 calls the Level panel",
    ),
    (
        "layouts.rs",
        "area left assets",
        "a panel layout's key as saved before decision 0118, still read",
    ),
    (
        "layouts.rs",
        "area and assets left",
        "a panel layout's key as saved before decision 0118, still read",
    ),
    (
        "window.rs",
        "=areas",
        "the folder older projects kept their levels in, where Open still starts",
    ),
    (
        "older_files.rs",
        "area{} made by an older solis",
        "the Console's note that a project's areas were renamed levels",
    ),
    (
        "older_files.rs",
        "areas are called levels now, so an area's",
        "the same note, saying what changed",
    ),
    (
        "older_files.rs",
        "read as an area for now",
        "the note for an older file that couldn't be renamed",
    ),
    (
        "older_files.rs",
        "-- an area. safe to edit by hand",
        "the comment an area's first line had, which the rename replaces",
    ),
    (
        "older_files.rs",
        "it isn't an area",
        "why a file with the old ending couldn't be renamed",
    ),
    (
        "concepts.md",
        "~**area:** the old name for a level",
        "the pointer for anyone who knew the old name",
    ),
    (
        "backups-and-upgrades.md",
        "~**areas renamed levels:**",
        "explains the rename a project from before it goes through",
    ),
];

/// The crates whose code holds what the program says.
const CRATES: &[&str] = &[
    "app",
    "editor",
    "script",
    "level",
    "input",
    "asset_db",
    "world",
    "tiles",
    "sound",
    "animation",
    "ui",
];

/// The repository's top folder.
fn top() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every file under `folder` ending in `.ext`, with its text (lines
/// ended the one way).
fn files_in(folder: &Path, ext: &str) -> Vec<(PathBuf, String)> {
    let mut found = Vec::new();
    let mut folders = vec![folder.to_path_buf()];
    while let Some(f) = folders.pop() {
        for entry in std::fs::read_dir(&f).unwrap_or_else(|e| panic!("{f:?}: {e}")) {
            let path = entry.unwrap().path();
            if path.is_dir() {
                folders.push(path);
            } else if path.extension().is_some_and(|e| e == ext) {
                let text = std::fs::read_to_string(&path)
                    .unwrap()
                    .replace("\r\n", "\n");
                found.push((path, text));
            }
        }
    }
    found.sort();
    found
}

/// The texts of the string literals in `code` (Rust), leaving out
/// comments and the tests at a file's end: what the program can show.
fn strings_of(code: &str) -> Vec<String> {
    let code = code.split("#[cfg(test)]\nmod ").next().unwrap_or_default();
    let mut found = Vec::new();
    let chars: Vec<char> = code.chars().collect();
    let mut at = 0;
    while at < chars.len() {
        match chars[at] {
            '/' if chars.get(at + 1) == Some(&'/') => {
                while at < chars.len() && chars[at] != '\n' {
                    at += 1;
                }
            }
            '\'' if chars.get(at + 2) == Some(&'\'') => at += 3,
            '\'' if chars.get(at + 1) == Some(&'\\') && chars.get(at + 3) == Some(&'\'') => {
                at += 4;
            }
            '"' => {
                let mut text = String::new();
                at += 1;
                while at < chars.len() && chars[at] != '"' {
                    if chars[at] == '\\' {
                        at += 1;
                        // A line broken with `\` goes on with no gap.
                        if chars.get(at) == Some(&'\n') {
                            at += 1;
                            while chars.get(at).is_some_and(|c| c.is_whitespace()) {
                                at += 1;
                            }
                            continue;
                        }
                    }
                    if let Some(c) = chars.get(at) {
                        text.push(*c);
                    }
                    at += 1;
                }
                found.push(text);
                at += 1;
            }
            _ => at += 1,
        }
    }
    found
}

/// Where in `text` (already in lower case) the word "area" or "areas"
/// stands alone.
fn words_at(text: &str) -> Vec<usize> {
    let letter = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_');
    let mut found = Vec::new();
    for (at, _) in text.match_indices("area") {
        let before = text[..at].chars().next_back();
        let rest = &text[at + 4..];
        let after = rest.strip_prefix('s').unwrap_or(rest).chars().next();
        if !letter(before) && !letter(after) {
            found.push(at);
        }
    }
    found
}

/// The uses of "area" in `text` (from the file named `file`) that no
/// allowed phrase covers, each with the text around it. `whole` is the
/// whole text of a string in code, for the `=` phrases.
fn unexplained(file: &str, text: &str, whole: bool) -> Vec<String> {
    let lower = text.to_lowercase();
    let mut covered: Vec<(usize, usize)> = Vec::new();
    for (in_file, phrase, _) in ALLOWED {
        if *in_file != "*" && *in_file != file {
            continue;
        }
        if let Some(exact) = phrase.strip_prefix('=') {
            if whole && lower == exact {
                covered.push((0, lower.len()));
            }
            continue;
        }
        if let Some(marker) = phrase.strip_prefix('~') {
            if lower.contains(marker) {
                covered.push((0, lower.len()));
            }
            continue;
        }
        for (at, _) in lower.match_indices(phrase) {
            covered.push((at, at + phrase.len()));
        }
    }
    words_at(&lower)
        .into_iter()
        .filter(|at| !covered.iter().any(|(a, b)| a <= at && *at < *b))
        .map(|at| {
            let from = lower[..at]
                .char_indices()
                .rev()
                .nth(30)
                .map_or(0, |(i, _)| i);
            let to = lower[at..]
                .char_indices()
                .nth(34)
                .map_or(lower.len(), |(i, _)| at + i);
            format!("{file}: …{}…", &text[from..to])
        })
        .collect()
}

#[test]
fn the_program_says_level_for_what_was_an_area() {
    let mut said = Vec::new();
    for name in CRATES {
        for (path, text) in files_in(&top().join("crates").join(name).join("src"), "rs") {
            let file = path.file_name().unwrap().to_string_lossy().into_owned();
            if file.ends_with("_tests.rs") || file == "tests.rs" {
                continue;
            }
            for string in strings_of(&text) {
                said.extend(unexplained(&file, &string, true));
            }
        }
    }
    assert!(said.is_empty(), "say \"level\":\n{}", said.join("\n"));
}

#[test]
fn the_manual_and_the_guide_s_captions_say_level() {
    let mut said = Vec::new();
    let mut pages = files_in(&top().join("docs/manual"), "md");
    let captions = top().join("tools/guide-pictures/pictures.list");
    pages.push((
        captions.clone(),
        std::fs::read_to_string(&captions).unwrap(),
    ));
    for (path, text) in pages {
        let file = path.file_name().unwrap().to_string_lossy().into_owned();
        for line in text.lines() {
            said.extend(unexplained(&file, line, false));
        }
    }
    assert!(said.is_empty(), "say \"level\":\n{}", said.join("\n"));
}

#[test]
fn the_check_finds_what_it_looks_for() {
    // The checker itself: a word alone is found, in any case and plural;
    // inside another word, an allowed phrase or an exact string, it isn't.
    assert_eq!(words_at("the area, two areas"), [4, 14]);
    assert!(words_at("solarea textarea areal").is_empty());
    assert_eq!(unexplained("x.rs", "Save Area", false).len(), 1);
    assert!(unexplained("x.rs", "Show the safe area", false).is_empty());
    assert!(unexplained("text.rs", "area", true).is_empty());
    assert_eq!(unexplained("text.rs", "an area", true).len(), 1);
    assert_eq!(unexplained("other.rs", "area", true).len(), 1);
    assert_eq!(
        strings_of("let a = \"one\"; // \"not this\"\nlet b = 'x'; let c = \"two \\\n    lines\";"),
        ["one", "two lines"]
    );
}
