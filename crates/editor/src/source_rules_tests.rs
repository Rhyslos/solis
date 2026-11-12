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
//! - **Unfinished parts are drawn one way** ([decision 0101](../../../docs/decisions/0101-unfinished-parts-as-placeholders.md)):
//!   only `unbuilt.rs` and its folder may say that something isn't built
//!   or will come later; an unfinished part's id is named only where it
//!   is handed to an `unbuilt::` function or put in a list as
//!   `Unbuilt(..)`; and a control switched off for good must say why, so
//!   a dead button can't stand in for a placeholder.

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
const KEPT: [(&str, usize, &str); 12] = [
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
        "map_tab.rs",
        2,
        "over a terrain's own colour: the wall mark, a count",
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

/// Whether a file is the mechanism for unfinished parts itself, which
/// may say what no other file may.
fn is_the_mechanism(path: &Path) -> bool {
    path.components()
        .any(|part| part.as_os_str().to_string_lossy().starts_with("unbuilt"))
}

/// Whether a file holds tests (which name parts and quote messages).
fn is_tests(path: &Path) -> bool {
    let name = name_of(path);
    name.ends_with("tests.rs") || name == "test_support.rs" || name == "sheet.rs"
}

/// The ways code might tell the user that something isn't there yet. Any
/// of them in a string, outside the mechanism, is a placeholder drawn by
/// hand.
const SAYS_UNBUILT: [&str; 6] = [
    "not built",
    "n't built",
    "n\u{2019}t built",
    "arrives in",
    "coming in",
    "coming later",
];

/// The phrases of [`SAYS_UNBUILT`] that `code` (a file without its
/// comments) says: each found as words of its own, not as letters inside
/// a longer word ("becoming invisible" doesn't say "coming in").
fn says_unbuilt(code: &str) -> Vec<&'static str> {
    let lower = code.to_lowercase();
    SAYS_UNBUILT
        .into_iter()
        .filter(|phrase| {
            lower.match_indices(phrase).any(|(at, _)| {
                // "n't built" follows the letters of "is" or "are".
                let starts_a_word = !lower[..at].ends_with(|c: char| c.is_alphabetic());
                starts_a_word || phrase.starts_with("n'") || phrase.starts_with("n\u{2019}")
            })
        })
        .collect()
}

#[test]
fn nothing_outside_the_mechanism_says_that_something_is_not_built() {
    let mut wrong = Vec::new();
    for (path, text) in sources_of(&["editor", "app"]) {
        if is_the_mechanism(&path) || is_tests(&path) {
            continue;
        }
        for phrase in says_unbuilt(&code_of(&text)) {
            wrong.push(format!("{} says \"{phrase}\"", path.display()));
        }
    }
    assert!(
        wrong.is_empty(),
        "Something the editor can't do yet is drawn by crate::unbuilt (a line in \
         unbuilt/table.rs, then one of its functions), never written out by hand:\n{}",
        wrong.join("\n")
    );
}

#[test]
fn the_not_built_rule_reads_strings_and_leaves_comments_alone() {
    // The sentence the Layers panel used to show.
    assert_eq!(
        says_unbuilt("ui.label(\"Layers aren't built yet\")"),
        ["n't built"]
    );
    assert_eq!(says_unbuilt("ui.label(\"Not built yet\")"), ["not built"]);
    assert_eq!(says_unbuilt("\"Baking isn't built\""), ["n't built"]);
    assert_eq!(says_unbuilt("\"Coming later\""), ["coming later"]);
    assert_eq!(says_unbuilt("\"Arrives in Shared 7\""), ["arrives in"]);
    // Comments are not what the user reads.
    let file = "let a = 1; // not built yet\n/// Coming later, perhaps.\nlet b = 2;\n";
    assert!(says_unbuilt(&code_of(file)).is_empty());
    // Ordinary words that hold a phrase's letters apart aren't caught.
    assert!(says_unbuilt("\"Becoming invisible\"").is_empty());
}

/// The places in `code` where an unfinished part's id is named some
/// other way than being handed to an `unbuilt::` function or put in a
/// list as `Unbuilt(..)`: each as the text around it.
fn ids_named_loosely(code: &str) -> Vec<String> {
    let mut loose = Vec::new();
    let name = "UnbuiltId::";
    let mut from = 0;
    while let Some(found) = code[from..].find(name) {
        let at = from + found;
        from = at + name.len();
        let before = &code[..at];
        // The statement so far: from the last `;` (or the file's start).
        let statement = &before[before.rfind(';').map_or(0, |i| i + 1)..];
        let in_a_list = statement.trim_end().ends_with("Unbuilt(");
        // Inside the brackets of a call to one of the mechanism's own
        // functions: the bracket opened after its name isn't closed yet.
        let handed_over = statement.rfind("unbuilt::").is_some_and(|call| {
            let mut depth = 0_i32;
            let mut opened = false;
            for c in statement[call..].chars() {
                match c {
                    '(' => {
                        depth += 1;
                        opened = true;
                    }
                    ')' => depth -= 1,
                    _ => {}
                }
                if opened && depth == 0 {
                    return false;
                }
            }
            opened
        });
        if !in_a_list && !handed_over {
            let end = (at + 40).min(code.len());
            let start = at.saturating_sub(40);
            let around: String = code
                .char_indices()
                .filter(|(i, _)| (start..end).contains(i))
                .map(|(_, c)| if c == '\n' { ' ' } else { c })
                .collect();
            loose.push(around.trim().to_owned());
        }
    }
    loose
}

#[test]
fn an_unfinished_part_is_named_only_where_it_is_drawn_or_listed() {
    let mut wrong = Vec::new();
    for (path, text) in sources_of(&["editor", "app"]) {
        if is_the_mechanism(&path) || is_tests(&path) {
            continue;
        }
        for around in ids_named_loosely(&code_of(&text)) {
            wrong.push(format!("{}: ...{around}...", path.display()));
        }
    }
    assert!(
        wrong.is_empty(),
        "An unfinished part's id goes to an unbuilt:: function, or into a list as \
         Entry::Unbuilt(..) that unbuilt::filter goes over; nothing else may hold one, so \
         deleting its line from the table finds every place that drew it:\n{}",
        wrong.join("\n")
    );
}

#[test]
fn the_id_rule_allows_drawing_and_listing_and_nothing_else() {
    for fine in [
        "unbuilt::ribbon_big(ui, UnbuiltId::HomeLightingBake, Icon::Sun);",
        "let r = crate::unbuilt::setting_row(\n    ui,\n    UnbuiltId::PrefsCodeSplit,\n);",
        "vec![Built(a), Unbuilt(UnbuiltId::ShellTabVfx), Unbuilt(\n  UnbuiltId::ShellTabShading)]",
        "let rows = [Entry::Unbuilt(UnbuiltId::PrefsCodeSplit)];",
        "if unbuilt::tab(ui, pick(UnbuiltId::ShellTabVfx)).clicked() {}",
    ] {
        assert!(ids_named_loosely(fine).is_empty(), "{fine}");
    }
    for loose in [
        "let id = UnbuiltId::HomeLightingBake;",
        "if id == UnbuiltId::ShellTabVfx { ui.label(\"soon\"); }",
        "draw_grey_button(ui, UnbuiltId::HomeLightingBake);",
        "unbuilt::tab(ui, a); other(UnbuiltId::ShellTabVfx);",
        "unbuilt::tab(ui, a)\n    .on_hover_text(name(UnbuiltId::ShellTabVfx));",
    ] {
        assert_eq!(ids_named_loosely(loose).len(), 1, "{loose}");
    }
}

/// The controls in `code` that are switched off for good with nothing
/// said about why: `add_enabled(false, ..)` whose statement gives no
/// reason for hover.
fn dead_controls(code: &str) -> usize {
    ["add_enabled(false", "add_enabled_ui(false"]
        .iter()
        .flat_map(|dead| code.split(dead).skip(1))
        .filter(|after| {
            let statement = after.split(';').next().unwrap_or_default();
            !statement.contains("on_disabled_hover_text(")
        })
        .count()
}

#[test]
fn a_control_switched_off_for_good_says_why() {
    let mut wrong = Vec::new();
    for (path, text) in sources_of(&["editor", "app"]) {
        if is_tests(&path) {
            continue;
        }
        let dead = dead_controls(&code_of(&text));
        if dead > 0 {
            wrong.push(format!("{}: {dead}", path.display()));
        }
    }
    assert!(
        wrong.is_empty(),
        "A control that can never be used is either an unfinished part (crate::unbuilt) or \
         gives its reason with on_disabled_hover_text in the same statement:\n{}",
        wrong.join("\n")
    );
}

#[test]
fn the_dead_control_rule_asks_for_a_reason_in_the_same_statement() {
    assert_eq!(
        dead_controls("ui.add_enabled(false, Button::new(\"Bake\"));"),
        1
    );
    assert_eq!(
        dead_controls(
            "ui.add_enabled(false, Button::new(\"Save\"))\n    .on_disabled_hover_text(why);"
        ),
        0
    );
    // A reason given to some later control doesn't count.
    assert_eq!(
        dead_controls(
            "ui.add_enabled(false, a);\nui.add_enabled(can, b).on_disabled_hover_text(why);"
        ),
        1
    );
    assert_eq!(dead_controls("ui.add_enabled_ui(false, |ui| rows(ui));"), 1);
    // Switched off by a condition is another matter: it can be used at
    // other times.
    assert_eq!(
        dead_controls("ui.add_enabled(can_save, Button::new(\"Save\"));"),
        0
    );
}
