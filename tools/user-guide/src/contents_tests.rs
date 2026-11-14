use super::*;

const SUMMARY: &str = "# Manual\n\nWelcome words.\n\n## Start\n\n- [Getting started](getting-started.md)\n- [Concepts: ideas](concepts.md)\n\n## Reference\n\n- [Keys](keyboard-shortcuts.md)\n";

#[test]
fn groups_and_pages_in_order() {
    let contents = read(SUMMARY).unwrap();
    assert_eq!(
        contents.welcome,
        [Block::Paragraph("Welcome words.".into())]
    );
    assert_eq!(contents.groups.len(), 2);
    assert_eq!(contents.groups[1].name, "Reference");
    assert_eq!(
        contents.pages().collect::<Vec<_>>(),
        ["getting-started", "concepts", "keyboard-shortcuts"]
    );
}

#[test]
fn a_page_listed_twice_is_a_problem() {
    let twice = format!("{SUMMARY}- [Again](concepts.md)\n");
    let problems = read(&twice).unwrap_err();
    assert!(problems[0].contains("concepts.md twice"), "{problems:?}");
}

#[test]
fn an_item_without_a_page_is_a_problem() {
    let problems = read("## G\n\n- just words\n- [Out](../ROADMAP.md)\n").unwrap_err();
    assert_eq!(problems.len(), 3, "{problems:?}");
}

#[test]
fn no_pages_is_a_problem() {
    assert!(read("# Manual\n\nNothing.").is_err());
    assert!(read("").is_err());
}
