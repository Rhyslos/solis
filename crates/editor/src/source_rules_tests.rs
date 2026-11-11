//! Rules about the source text itself, which the compiler can't check.
//! Each test reads the files as `theme`'s `symbols_have_glyphs` does.
//!
//! - **No colour written as numbers outside the tokens** ([decision 0100](../../../docs/decisions/0100-one-theme-five-accents.md)).
//!   A colour of the design belongs in `tokens.rs`; `theme.rs` keeps the
//!   few Solis draws that the design doesn't have yet. The ones written
//!   elsewhere are counted in [`KEPT`], each with what it is: one more
//!   anywhere fails.
//! - **No gap in the middle of a sentence.** A message broken over two
//!   lines of code without its `\` keeps the next line's indentation, and
//!   shows it as a run of spaces.

use std::path::{Path, PathBuf};

/// Every `.rs` file under the `src` of each of `crates`, with its text.
fn sources_of(crates: &[&str]) -> Vec<(PathBuf, String)> {
    fn gather(folder: &Path, found: &mut Vec<(PathBuf, String)>) {
        let files = std::fs::read_dir(folder).unwrap_or_else(|e| panic!("{folder:?}: {e}"));
        for file in files.flatten() {
            let path = file.path();
            if path.is_dir() {
                gather(&path, found);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let text = std::fs::read_to_string(&path).unwrap();
                found.push((path, plain_lines(&text)));
            }
        }
    }
    let all = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut found = Vec::new();
    for name in crates {
        gather(&all.join(name).join("src"), &mut found);
    }
    found.sort();
    found
}

/// `text` with each line ended the one way the rules look for: a checkout
/// on Windows ends lines with a carriage return too, and a rule that
/// looks for a line break must find the same thing there.
fn plain_lines(text: &str) -> String {
    text.replace("\r\n", "\n")
}

/// A file's name, for a message.
fn name_of(path: &Path) -> String {
    path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}

/// `text` without its comments, and without the tests at its end.
fn code_of(text: &str) -> String {
    let code = text.split("#[cfg(test)]\nmod ").next().unwrap_or_default();
    code.lines()
        .map(|line| line.split("//").next().unwrap_or_default())
        .collect::<Vec<_>>()
        .join("\n")
}

/// The ways a colour is written as numbers.
const WRITTEN: [&str; 6] = [
    "Color32::from_rgb(",
    "Color32::from_gray(",
    "Color32::from_rgba_premultiplied(",
    "Color32::from_rgba_unmultiplied(",
    "Color32::from_black_alpha(",
    "Color32::from_white_alpha(",
];

/// How many colours `code` writes as numbers. One made from values worked
/// out elsewhere (a lamp's own colour, a picture's pixel) isn't counted.
fn colours_written(code: &str) -> usize {
    WRITTEN
        .iter()
        .flat_map(|way| code.split(way).skip(1))
        .filter(|after| after.trim_start().starts_with(|c: char| c.is_ascii_digit()))
        .count()
}

/// The colours written as numbers outside `tokens.rs` and `theme.rs`: the
/// file, how many, and what they are. Gone through when the design's
/// tokens arrived (decision 0100): a grey that is one of the design's now
/// reads its token. What is left means something on its own screen (a
/// meter's green, a build's tag), shades or lights what is under it, or
/// is a grey the design has no name for; each goes, or takes a token,
/// when its screen is rebuilt to the design.
const KEPT: [(&str, usize, &str); 14] = [
    (
        "atlas_view.rs",
        3,
        "under and over a picture: the grey behind it, its grid, a crop's yellow frame",
    ),
    (
        "canvas_view.rs",
        4,
        "over a game's screen: a missing picture's grey, the dashed frame, code's yellow mark",
    ),
    (
        "frames_view.rs",
        2,
        "the playhead's blue and an event's light blue",
    ),
    (
        "game_panel.rs",
        1,
        "the shade behind a note over the running game",
    ),
    (
        "graph_canvas.rs",
        4,
        "the live state's blue and a condition's amber chip; a node's name and border",
    ),
    (
        "launcher.rs",
        21,
        "a build's green, amber or red tag, each project's letter tile, the amber note",
    ),
    (
        "map_tab.rs",
        3,
        "over a terrain's own colour: the wall mark, a count; the base layer's amber",
    ),
    (
        "mixer_view.rs",
        5,
        "mute red, solo yellow, the meter's green, yellow and red",
    ),
    (
        "panels.rs",
        1,
        "the shade over the window while files are dragged in",
    ),
    (
        "play_changes.rs",
        1,
        "the blue that says the game is running",
    ),
    ("profiler.rs", 6, "a colour for each part of a frame's time"),
    (
        "tile_view.rs",
        2,
        "over a tile map: a switched-on cell's blue, the eraser's red",
    ),
    (
        "tileset_view.rs",
        5,
        "over a tileset's picture: its grid, a missing tile's pink and red, pictograms",
    ),
    (
        "viewport_tools.rs",
        1,
        "the gizmo handle under the mouse: yellow",
    ),
];

#[test]
fn no_colour_is_written_as_numbers_outside_the_palette() {
    let mut wrong = Vec::new();
    let mut seen = Vec::new();
    // The app draws the editor's window, so the rule is the same there.
    for (path, text) in sources_of(&["editor", "app"]) {
        let name = name_of(&path);
        if name.ends_with("tests.rs") || name == "tokens.rs" || name == "theme.rs" {
            continue;
        }
        let found = colours_written(&code_of(&text));
        let kept = KEPT.iter().find(|(file, ..)| *file == name);
        if found != kept.map_or(0, |(_, count, _)| *count) {
            wrong.push(format!(
                "{name} writes {found} colours as numbers; {} are listed",
                kept.map_or(0, |(_, count, _)| *count)
            ));
        }
        seen.extend(kept.map(|(file, ..)| *file));
    }
    for (file, ..) in KEPT {
        assert!(seen.contains(&file), "{file} is listed but wasn't found");
    }
    assert!(
        wrong.is_empty(),
        "A colour of the design goes in tokens.rs, and is read from there. If one is no token \
         of the design's, count it in KEPT and say what it is.\n{}",
        wrong.join("\n")
    );
}

/// A Windows checkout failed the colour rule: with a carriage return
/// before each line break, the tests at a file's end weren't found, and a
/// colour written in them was counted.
#[test]
fn the_rules_read_a_windows_checkout_like_any_other() {
    let windows = "let a = 1;\r\n#[cfg(test)]\r\nmod tests {\r\n    Color32::from_gray(40)\r\n}";
    assert_eq!(colours_written(&code_of(&plain_lines(windows))), 0);
    assert_eq!(plain_lines(windows), windows.replace('\r', ""));
}

#[test]
fn the_colour_rule_counts_numbers_and_nothing_else() {
    assert_eq!(colours_written("Color32::from_rgb(0x30, 0x30, 0x30)"), 1);
    assert_eq!(colours_written("Color32::from_gray(40)"), 1);
    assert_eq!(
        colours_written("(Color32::from_black_alpha(9), egui::Color32::from_rgb(\n    1, 2, 3))"),
        2
    );
    // A user's own colour, and a picture's pixels.
    assert_eq!(colours_written("Color32::from_rgb(r, g, b)"), 0);
    assert_eq!(
        colours_written("ColorImage::from_rgba_unmultiplied([4, 4], &pixels)"),
        0
    );
    assert_eq!(colours_written("theme::panel()"), 0);
    // Comments and the tests at a file's end aren't looked at.
    let file = "let a = 1; // Color32::from_gray(1)\n#[cfg(test)]\nmod tests {\n    \
                Color32::from_gray(2);\n}\n";
    assert_eq!(colours_written(&code_of(file)), 0);
}

/// Whether `line` holds a run of four or more spaces in the middle of a
/// sentence: after a small letter or a comma, colon or full stop, and
/// before a small letter. Code samples indent after `\n`, and tables put
/// their gaps before something else, so neither is caught.
fn gap_in_a_sentence(line: &str) -> bool {
    let code = line.split("//").next().unwrap_or_default();
    let mut rest = code;
    while let Some(start) = rest.find("    ") {
        let (before, gap) = rest.split_at(start);
        let after = gap.trim_start_matches(' ');
        let mid_sentence = before.ends_with(|c: char| c.is_ascii_lowercase() || ",.:;".contains(c))
            && !before.ends_with("\\n")
            && after.starts_with(|c: char| c.is_ascii_lowercase());
        if mid_sentence {
            return true;
        }
        rest = after;
    }
    false
}

#[test]
fn no_message_has_a_gap_in_the_middle_of_a_sentence() {
    let crates: Vec<String> = std::fs::read_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join(".."))
        .unwrap()
        .flatten()
        .filter(|entry| entry.path().join("src").is_dir())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    assert!(crates.len() > 10, "found too few crates: {crates:?}");
    let crates: Vec<&str> = crates.iter().map(String::as_str).collect();
    let mut wrong = Vec::new();
    for (path, text) in sources_of(&crates) {
        for (number, line) in text.lines().enumerate() {
            if gap_in_a_sentence(line) {
                wrong.push(format!(
                    "{}:{}: {}",
                    path.display(),
                    number + 1,
                    line.trim()
                ));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "A sentence broken over two lines of code needs a `\\` at the end of the first, or the \
         next line's indentation is shown as spaces:\n{}",
        wrong.join("\n")
    );
}

#[test]
fn the_gap_rule_catches_a_lost_line_break_and_nothing_else() {
    // Built here, so this file passes its own rule.
    let gap = " ".repeat(22);
    assert!(gap_in_a_sentence(&format!(
        "\"the game draws it{gap}on the nearest one\""
    )));
    assert!(gap_in_a_sentence(&format!("\"changes,{gap}them all\"")));
    assert!(gap_in_a_sentence(&format!(
        "\"changes{}them all\"",
        &gap[..4]
    )));
    assert!(!gap_in_a_sentence(&format!(
        "\"changes{}them all\"",
        &gap[..3]
    )));
    // Indentation, a code sample's lines, a table's columns and comments.
    for fine in [
        format!("{gap}\"on the nearest one, so it may seem to jump\""),
        format!("\"component Hive {{\\n{gap}has {{\\n{gap}int made = 0;\""),
        format!("\"  ok{gap}{{}}\""),
        format!("\"{{nl}}{{indent}}{gap}send {{event}}() to Ui:entity();\""),
        format!("let a = 1; // one{gap}two"),
    ] {
        assert!(!gap_in_a_sentence(&fine), "{fine}");
    }
}
