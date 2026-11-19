use super::*;
use proptest::prelude::*;

const SHOP: &str = "# The shop's words.\n\
Shop-Welcome = Welcome, { $name }!\n\
Shop-Apples =\n    { $count ->\n        [one] One apple\n       *[other] { $count } apples\n    }\n\
\n\
Shop-Price = { NUMBER($gold) } gold\n    .title = Price\n\
-brand = Solis\n";

#[test]
fn lines_are_read_with_their_keys_places_and_values() {
    let read = read("locale/en/shop.ftl", SHOP);
    assert!(read.problems.is_empty(), "{:?}", read.problems);
    let keys: Vec<&str> = read.lines.iter().map(|l| l.key.as_str()).collect();
    assert_eq!(keys, ["Shop.Welcome", "Shop.Apples", "Shop.Price"]);
    let welcome = &read.lines[0];
    assert_eq!(
        (welcome.line, welcome.text.as_str()),
        (2, "Welcome, { $name }!")
    );
    assert_eq!(welcome.values, [("name".to_owned(), ValueKind::Text)]);
    let apples = &read.lines[1];
    assert_eq!(apples.line, 3);
    assert!(
        apples.text.starts_with("{ $count ->\n    [one] One apple"),
        "{}",
        apples.text
    );
    assert_eq!(apples.values, [("count".to_owned(), ValueKind::Number)]);
    let price = &read.lines[2];
    assert_eq!(
        price.text, "{ NUMBER($gold) } gold",
        "the attribute isn't its text"
    );
    assert_eq!(price.values, [("gold".to_owned(), ValueKind::Number)]);
}

#[test]
fn mistakes_are_said_at_their_lines() {
    let text = "Shop-Ok = Fine\nShop-Broken = { $name\nshop_lower = fine too\nBad = one part\nShop-Ok = again\n";
    let read = read("locale/en/shop.ftl", text);
    let keys: Vec<&str> = read.lines.iter().map(|l| l.key.as_str()).collect();
    assert_eq!(keys, ["Shop.Ok"], "{:?}", read.problems);
    assert!(
        read.problems
            .iter()
            .any(|p| (2..=3).contains(&p.line) && p.message.contains("isn't Fluent")),
        "the broken placeable: {:?}",
        read.problems
    );
    assert!(
        read.problems
            .iter()
            .any(|p| p.message.contains("can't be a key"))
    );
    assert!(
        read.problems
            .iter()
            .any(|p| p.line == 5 && p.message.contains("twice"))
    );
}

#[test]
fn a_file_past_the_limit_is_left_out() {
    let big = "Shop-A = x\n".repeat(MAX_FILE_BYTES / 10);
    let read = read("locale/en/big.ftl", &big);
    assert!(read.lines.is_empty());
    assert!(read.problems[0].message.contains("KB"));
}

#[test]
fn blocks_span_their_attributes_but_not_the_empty_lines_after() {
    let found = blocks(SHOP);
    assert_eq!(found.len(), 3);
    assert_eq!(found[2].id, "Shop-Price");
    assert_eq!(found[2].lines, 8..10);
    assert_eq!(found[1].lines, 2..7);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(3000))]

    /// Damaged files never crash the reader (rule 9), and every line read
    /// has a key that can be one.
    #[test]
    fn damaged_files_never_crash(cut in 0usize..SHOP.len(), noise in "[ -~\\n{}$\\[\\]*.=-]{0,80}") {
        let at = (0..=cut).rev().find(|c| SHOP.is_char_boundary(*c)).unwrap_or(0);
        let text = format!("{}{noise}{}", &SHOP[..at], &SHOP[at..]);
        let read = read("locale/en/x.ftl", &text);
        for line in read.lines {
            prop_assert!(crate::keys::key_problem(&line.key).is_none());
        }
    }

    #[test]
    fn any_text_never_crashes(text in "\\PC{0,300}") {
        let _ = read("locale/en/x.ftl", &text);
        let _ = blocks(&text);
    }
}
