use super::*;

fn para(words: &str) -> Block {
    Block::Paragraph(words.to_string())
}

#[test]
fn headings_paragraphs_and_rules() {
    let blocks = parse("# Title\n\nOne line\nand the next.\n\n## Part ##\n\n---\n");
    assert_eq!(
        blocks,
        [
            Block::Heading(1, "Title".into()),
            para("One line\nand the next."),
            Block::Heading(2, "Part".into()),
            Block::Rule,
        ]
    );
}

#[test]
fn a_hash_without_a_space_is_words() {
    assert_eq!(parse("#hashtag"), [para("#hashtag")]);
}

#[test]
fn fenced_code_keeps_its_lines_and_language() {
    let blocks = parse("```lust\nlet a = 1;\n\n  indented\n```\nafter");
    assert_eq!(
        blocks,
        [
            Block::Code("lust".into(), "let a = 1;\n\n  indented".into()),
            para("after"),
        ]
    );
}

#[test]
fn an_unclosed_fence_runs_to_the_end() {
    assert_eq!(
        parse("~~~\nno end"),
        [Block::Code(String::new(), "no end".into())]
    );
}

#[test]
fn a_tight_list_and_a_numbered_one() {
    let blocks = parse("- one\n- two\n\n1. first\n2. second\n");
    assert_eq!(
        blocks,
        [
            Block::List(List {
                start: None,
                items: vec![vec![para("one")], vec![para("two")]],
                loose: false,
            }),
            Block::List(List {
                start: Some(1),
                items: vec![vec![para("first")], vec![para("second")]],
                loose: false,
            }),
        ]
    );
}

#[test]
fn a_list_item_holds_code_and_a_list_of_its_own() {
    let text = "1. Make it:\n   ```\n   code\n   ```\n2. Then:\n   - inner\n   - more\n";
    let blocks = parse(text);
    let [Block::List(list)] = blocks.as_slice() else {
        panic!("one list");
    };
    assert_eq!(list.items.len(), 2);
    assert_eq!(
        list.items[0],
        [para("Make it:"), Block::Code(String::new(), "code".into())]
    );
    assert!(matches!(&list.items[1][1], Block::List(inner) if inner.items.len() == 2));
}

#[test]
fn blank_lines_between_items_make_a_loose_list() {
    let blocks = parse("- one\n\n- two");
    let [Block::List(list)] = blocks.as_slice() else {
        panic!("one list");
    };
    assert!(list.loose);
}

#[test]
fn a_lazy_line_carries_the_item_on() {
    let blocks = parse("- one\nstill one\n- two");
    let [Block::List(list)] = blocks.as_slice() else {
        panic!("one list");
    };
    assert_eq!(list.items[0], [para("one\nstill one")]);
}

#[test]
fn tables_with_escaped_bars_and_short_rows() {
    let blocks = parse("| A | B |\n|---|:-:|\n| `a|b` | x \\| y |\n| only |\n");
    assert_eq!(
        blocks,
        [Block::Table(Table {
            head: vec!["A".into(), "B".into()],
            rows: vec![
                vec!["`a|b`".into(), "x | y".into()],
                vec!["only".into(), String::new()],
            ],
        })]
    );
}

#[test]
fn quotes_hold_blocks() {
    let blocks = parse("> **Good to know:** this\n> and that\n>\n> - a list\n");
    let [Block::Quote(inner)] = blocks.as_slice() else {
        panic!("one quote");
    };
    assert_eq!(inner[0], para("**Good to know:** this\nand that"));
    assert!(matches!(inner[1], Block::List(_)));
}

#[test]
fn windows_line_ends_read_the_same() {
    assert_eq!(parse("# A\r\n\r\nb\r\n"), parse("# A\n\nb\n"));
}

#[test]
fn deep_nesting_stops_without_running_out_of_stack() {
    let deep = ">".repeat(10_000) + " words";
    let blocks = parse(&deep);
    assert_eq!(blocks.len(), 1);
    let listed = "- ".repeat(5_000) + "x";
    assert_eq!(parse(&listed).len(), 1);
}
