//! The keyboard shortcuts page of the manual (and so of the user guide,
//! decision 0112) and the editor's one table of shortcuts
//! (`ShellCommand::keys`, `shell/command_words.rs`) agree: every key the
//! table binds is on the page, and every key the page gives for editing
//! and for the tools is one the editor binds. Keys on the page are
//! written in backticks (`` `Ctrl + S` ``, the manual's convention).

use crate::shell::ShellCommand;

/// Keys the editor binds outside the table of commands
/// (`workbench_keys.rs` and the search), which the page names too.
const ALSO_BOUND: [&str; 3] = [
    // Points the camera at the selection.
    "F",
    // Opens the search.
    "Space",
    // Redo's second key.
    "Ctrl + Shift + Z",
];

fn page() -> String {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/manual/keyboard-shortcuts.md"
    );
    std::fs::read_to_string(path).unwrap().replace("\r\n", "\n")
}

/// The table's way of writing keys (`Ctrl+Shift+S`) as the page writes
/// them (`Ctrl + Shift + S`).
fn spaced(keys: &str) -> String {
    keys.split('+')
        .map(str::trim)
        .collect::<Vec<_>>()
        .join(" + ")
}

/// Everything in backticks in `text`.
fn in_backticks(text: &str) -> Vec<String> {
    text.split('`')
        .skip(1)
        .step_by(2)
        .map(|k| k.trim().to_string())
        .collect()
}

/// The part of the page under the heading starting with `heading`.
fn section<'a>(page: &'a str, heading: &str) -> &'a str {
    let start = page
        .find(&format!("\n## {heading}"))
        .unwrap_or_else(|| panic!("the page has no section {heading:?}"));
    let rest = &page[start + 1..];
    let end = rest[3..].find("\n## ").map_or(rest.len(), |e| e + 3);
    &rest[..end]
}

/// The keys in the shortcut column of a section's table.
fn table_keys(section: &str) -> Vec<String> {
    section
        .lines()
        .filter(|l| l.starts_with('|') && !l.starts_with("|---"))
        .filter_map(|l| l.split('|').nth(2))
        .flat_map(in_backticks)
        .collect()
}

/// Every key the editor's table binds, as the page writes it.
fn bound() -> Vec<String> {
    ShellCommand::ALL
        .iter()
        .map(|c| c.keys())
        .filter(|k| !k.is_empty())
        .map(spaced)
        .collect()
}

#[test]
fn every_key_the_editor_binds_is_on_the_shortcuts_page() {
    let on_the_page = in_backticks(&page());
    let missing: Vec<String> = bound()
        .into_iter()
        .filter(|k| !on_the_page.contains(k))
        .collect();
    assert!(
        missing.is_empty(),
        "docs/manual/keyboard-shortcuts.md doesn't name these keys of the editor's table \
         (ShellCommand::keys), written in backticks: {missing:?}"
    );
}

#[test]
fn every_key_the_page_gives_for_editing_and_the_tools_is_bound() {
    let page = page();
    let bound = bound();
    for heading in ["Editing", "Tools"] {
        let keys = table_keys(section(&page, heading));
        assert!(keys.len() >= 5, "{heading}: only {keys:?}");
        for key in keys {
            assert!(
                bound.contains(&key) || ALSO_BOUND.contains(&key.as_str()),
                "the shortcuts page gives `{key}` under {heading}, but the editor binds no such \
                 key: change the page, or the editor's table (shell/command_words.rs)"
            );
        }
    }
}

#[test]
fn a_key_the_editor_doesnt_bind_would_be_found() {
    let made_up = "## Editing\n\n| Action | Shortcut |\n|---|---|\n| Undo | `Ctrl + Q` |\n";
    let keys = table_keys(section(&format!("\n{made_up}"), "Editing"));
    assert_eq!(keys, ["Ctrl + Q"]);
    assert!(!bound().contains(&keys[0]));
    assert_eq!(spaced("Ctrl+Shift+S"), "Ctrl + Shift + S");
}
