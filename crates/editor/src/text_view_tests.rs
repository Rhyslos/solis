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
    w.open.library = crate::library::LibraryFiles::from(vec![
        item("notes/plan.md", LibraryKind::Text),
        item("birdsong.ogg", LibraryKind::Sound),
    ]);
    assert_eq!(w.text_file_wanted(), None);
    w.open.sound_shown = Some(("notes/plan.md".into(), w.open.document.selection.clone()));
    assert_eq!(w.text_file_wanted(), Some("notes/plan.md"));
    w.set_text_file("notes/plan.md".into(), Ok("# Plan".into()));
    assert_eq!(
        w.open
            .text_file
            .as_ref()
            .map(|(p, r)| (p.as_str(), r.clone())),
        Some((
            "notes/plan.md",
            Ok(ShownText::of("notes/plan.md", "# Plan"))
        ))
    );
    w.open.sound_shown = Some(("birdsong.ogg".into(), w.open.document.selection.clone()));
    assert_eq!(w.text_file_wanted(), None);
}

/// A Markdown note of one long paragraph of emphasised words: the marks
/// it never uses (code, links, strong) were looked for from each word on
/// to the paragraph's end, for every word.
fn long_note(words: usize) -> String {
    format!("# Notes\n\n{}\n", "*a* ".repeat(words))
}

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
fn scale_check_a_long_markdown_note_in_properties() {
    // BUG-27: the note was read into blocks and runs again every frame,
    // and reading a paragraph's marks took the square of its length.
    const TARGET_MS: f64 = 4.0;
    let text = long_note(20_000);
    let read: TextShown = Ok(ShownText::of("notes/long.md", &text));
    let mut h = egui_kittest::Harness::builder()
        .with_size(egui::vec2(400.0, 800.0))
        .build_ui(move |ui| {
            let _ = text_properties(ui, "notes/long.md", Some(&read));
        });
    h.step();
    let frames = 3;
    let start = std::time::Instant::now();
    for _ in 0..frames {
        h.step();
    }
    let ms = start.elapsed().as_secs_f64() * 1000.0 / f64::from(frames);
    let verdict = if ms <= TARGET_MS { "ok" } else { "SLOW" };
    println!(
        "Scale check: a Markdown note of {} KB in Properties, a frame {ms:.2} ms (target \
         {TARGET_MS} ms) {verdict}",
        text.len() / 1024
    );
    assert!(ms <= TARGET_MS, "{ms:.2} ms");
}

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
fn scale_check_reading_a_long_paragraph_s_marks() {
    // BUG-27: each turn looked for each mark from where it was to the
    // paragraph's end, so a mark never used cost the square of its length.
    const TARGET_MS: f64 = 20.0;
    let paragraph = "*a* ".repeat(131_072);
    let start = std::time::Instant::now();
    let runs = inline_runs(&paragraph);
    let ms = start.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(runs.len(), 2 * 131_072);
    let verdict = if ms <= TARGET_MS { "ok" } else { "SLOW" };
    println!(
        "Scale check: reading the marks of a 512 KB paragraph {ms:.2} ms (target {TARGET_MS} \
         ms) {verdict}"
    );
    assert!(ms <= TARGET_MS, "{ms:.2} ms");
}

/// `inline_runs` as it was before it became one pass (the review,
/// BUG-27): what it gives must not change.
fn inline_runs_as_before(text: &str) -> Vec<(Style, String)> {
    let mut runs: Vec<(Style, String)> = Vec::new();
    let mut push = |style: Style, words: &str| {
        if words.is_empty() {
            return;
        }
        match runs.last_mut() {
            Some((last, so_far)) if *last == style => so_far.push_str(words),
            _ => runs.push((style, words.to_owned())),
        }
    };
    let mut rest = text;
    while !rest.is_empty() {
        let found = [
            ("`", "`", Style::Code),
            ("**", "**", Style::Strong),
            ("__", "__", Style::Strong),
            ("*", "*", Style::Emphasis),
            ("_", "_", Style::Emphasis),
            ("[", "](", Style::Link),
        ]
        .iter()
        .filter_map(|(open, close, style)| {
            let at = rest.find(open)?;
            let inside = &rest[at + open.len()..];
            let end = inside.find(close)?;
            (end > 0).then_some((at, *open, *close, *style, end))
        })
        .min_by_key(|(at, open, ..)| (*at, std::cmp::Reverse(open.len())));
        let Some((at, open, close, style, end)) = found else {
            push(Style::Plain, rest);
            break;
        };
        push(Style::Plain, &rest[..at]);
        let inside = &rest[at + open.len()..at + open.len() + end];
        let mut after = &rest[at + open.len() + end + close.len()..];
        if style == Style::Link {
            // The address, up to its ")", isn't shown.
            after = after.find(')').map_or(after, |i| &after[i + 1..]);
        }
        push(style, inside);
        rest = after;
    }
    runs
}

/// BUG-27: the one pass reads every mix of marks exactly as the reading
/// it replaced did.
#[test]
fn inline_marks_read_in_one_pass_as_before() {
    let pieces = [
        "`", "**", "__", "*", "_", "[", "](", ")", "a", " ", "é", "x*", "](b)", "`c`",
    ];
    let mut seed = 3u64;
    for _ in 0..20_000 {
        let mut text = String::new();
        for _ in 0..(seed % 30) {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            text.push_str(pieces[(seed >> 33) as usize % pieces.len()]);
        }
        assert_eq!(inline_runs(&text), inline_runs_as_before(&text), "{text:?}");
    }
}

#[test]
fn a_text_file_is_made_ready_to_show_once() {
    // Markdown: its blocks, each with its words' runs.
    let note = ShownText::of(
        "notes/Plan.MD",
        "# A *plan*\n\n- one `two`\n```\n*not*\n```",
    );
    assert_eq!((note.lines, note.empty, note.more), (6, false, false));
    let Body::Markdown(blocks) = &note.body else {
        panic!("{note:?}");
    };
    assert_eq!(
        blocks,
        &[
            (
                Block::Heading(1, "A *plan*".into()),
                inline_runs("A *plan*")
            ),
            (
                Block::Item(0, "•".into(), "one `two`".into()),
                inline_runs("one `two`")
            ),
            (Block::Code("*not*".into()), Vec::new()),
        ]
    );
    // Anything else as it is; the lines past MAX_LINES left out.
    let long = "x\n".repeat(MAX_LINES + 5);
    let data = ShownText::of("data/table.csv", &long);
    assert_eq!((data.lines, data.more), (MAX_LINES + 5, true));
    assert_eq!(data.body, Body::Plain(first_lines(&long).0));
    assert!(ShownText::of("a.txt", " \n\t").empty);
}
