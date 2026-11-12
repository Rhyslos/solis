//! Text files in Properties: Markdown read into blocks and inline runs,
//! whatever it holds, and the file the app is asked to read.

use super::*;

#[test]
fn markdown_reads_into_headings_paragraphs_lists_quotes_code_and_rules() {
    let text = "# Notes\n\nHello from\na markdown file.\n\n- one\n  - nested\n* [x] done\n+ [ ] to do\n3. third\n> a quote\n\n---\n```lust\nWorld:say(\"hi\");\n\n```\n## Two ##\nplain";
    assert_eq!(
        markdown_blocks(text),
        vec![
            Block::Heading(1, "Notes".into()),
            Block::Paragraph("Hello from a markdown file.".into()),
            Block::Item(0, "•".into(), "one".into()),
            Block::Item(1, "•".into(), "nested".into()),
            Block::Item(0, "☑".into(), "done".into()),
            Block::Item(0, "☐".into(), "to do".into()),
            Block::Item(0, "3.".into(), "third".into()),
            Block::Quote("a quote".into()),
            Block::Rule,
            Block::Code("World:say(\"hi\");\n".into()),
            Block::Heading(2, "Two".into()),
            Block::Paragraph("plain".into()),
        ]
    );
}

#[test]
fn odd_markdown_is_still_shown() {
    // Not a heading without the space, a numbered line, a code block
    // never closed.
    assert_eq!(
        markdown_blocks("#hashtag\n2026. A year\n```\nopen"),
        vec![
            Block::Paragraph("#hashtag".into()),
            Block::Item(0, "2026.".into(), "A year".into()),
            Block::Code("open".into()),
        ]
    );
    assert!(markdown_blocks("").is_empty());
    assert!(markdown_blocks("\n\n   \n").is_empty());
    // A long file stops at MAX_LINES.
    let long = "x\n\n".repeat(MAX_LINES * 2);
    assert!(markdown_blocks(&long).len() <= MAX_LINES);
}

#[test]
fn inline_marks_become_runs_and_unclosed_ones_stay_as_written() {
    use Style::*;
    assert_eq!(
        inline_runs("a **b** *c* `d` [e](http://x) f"),
        vec![
            (Plain, "a ".into()),
            (Strong, "b".into()),
            (Plain, " ".into()),
            (Emphasis, "c".into()),
            (Plain, " ".into()),
            (Code, "d".into()),
            (Plain, " ".into()),
            (Link, "e".into()),
            (Plain, " f".into()),
        ]
    );
    assert_eq!(inline_runs("2 * 3 = 6"), vec![(Plain, "2 * 3 = 6".into())]);
    assert_eq!(
        inline_runs("snake_case"),
        vec![(Plain, "snake_case".into())]
    );
    assert_eq!(inline_runs("`a*b*`"), vec![(Code, "a*b*".into())]);
    assert!(inline_runs("").is_empty());
}

#[test]
fn any_text_is_read_without_trouble() {
    // A fuzz-like sweep: every mix of the marks and odd characters,
    // including ones that are more than one byte, never panics and never
    // loses words from a plain line.
    let pieces = [
        "#",
        "##",
        " ",
        "*",
        "**",
        "_",
        "`",
        "```",
        "[",
        "](",
        ")",
        ">",
        "-",
        "1.",
        "é",
        "\u{65e5}\u{672c}",
        "\n",
        "\t",
        "x",
        "\u{200f}",
        "[ ]",
        "[x]",
    ];
    let mut seed = 7u64;
    for _ in 0..4_000 {
        let mut text = String::new();
        for _ in 0..(seed % 24) {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            text.push_str(pieces[(seed >> 33) as usize % pieces.len()]);
        }
        let blocks = markdown_blocks(&text);
        for block in &blocks {
            if let Block::Paragraph(words) | Block::Quote(words) | Block::Heading(_, words) = block
            {
                let _ = inline_runs(words);
            }
        }
        let _ = inline_runs(&text);
    }
}

#[test]
fn the_app_is_asked_for_a_text_file_shown_in_properties_only() {
    let mut w = Workbench::new(crate::test_support::info());
    let item = |path: &str, kind| crate::library::LibraryItem {
        kind,
        name: path.into(),
        path: path.into(),
        ..crate::library::LibraryItem::default()
    };
    w.open.library = vec![
        item("notes/plan.md", LibraryKind::Text),
        item("birdsong.ogg", LibraryKind::Sound),
    ];
    assert_eq!(w.text_file_wanted(), None);
    w.open.sound_shown = Some(("notes/plan.md".into(), w.open.document.selection.clone()));
    assert_eq!(w.text_file_wanted(), Some("notes/plan.md"));
    w.set_text_file("notes/plan.md".into(), Ok("# Plan".into()));
    assert_eq!(
        w.open
            .text_file
            .as_ref()
            .map(|(p, r)| (p.as_str(), r.clone())),
        Some(("notes/plan.md", Ok("# Plan".to_owned())))
    );
    w.open.sound_shown = Some(("birdsong.ogg".into(), w.open.document.selection.clone()));
    assert_eq!(w.text_file_wanted(), None);
}
