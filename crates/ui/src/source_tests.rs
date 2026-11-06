use super::*;

const MENU: &str = r#"fetch "solis/screen":Ui;
fetch "events":[Resumed];

-- The pause menu.
assembly PauseMenu {
    has {
        int clicks = 0;
    }

    method helper() { }

    view {
        stack [width fill, height fill, center, middle] {
            column [gap 14, background gray9, pad 24, round 8] {
                text "Paused" [size 28, bold]; -- the title
                button "Resume" [blue, hover:[background red5]] {
                    clicks = clicks + 1;
                    send Resumed() to Ui:entity();
                }
                button "Quit";
                text "Clicked " + clicks.text();
                loop (int i = 0; i < 3; i += 1) {
                    text "row";
                }
                int x = 4;
            }
        }
    }
}

pass PauseMenu;
end
"#;

fn view(text: &str) -> Block {
    parse(text).unwrap().remove(0).view
}

#[test]
fn a_screen_s_pieces_are_read_with_where_they_are() {
    let screens = parse(MENU).unwrap();
    assert_eq!(screens.len(), 1);
    assert_eq!(screens[0].name, "PauseMenu");
    let v = &screens[0].view;
    let stack = v.at(&[0]).unwrap();
    assert_eq!(stack.kind, "stack");
    assert_eq!(stack.style_text(MENU, "width"), Some("fill"));
    assert!(stack.has("center"));
    let column = v.at(&[0, 0]).unwrap();
    let kinds: Vec<String> = column
        .children
        .as_ref()
        .unwrap()
        .pieces
        .iter()
        .map(|p| match p {
            Piece::Element(e) => e.kind.clone(),
            Piece::Code(c) => format!("code{}", if c.makes_pieces { "+" } else { "" }),
        })
        .collect();
    assert_eq!(
        kinds,
        ["text", "button", "button", "text", "code+", "code"],
        "a loop makes pieces; a declaration doesn't"
    );
    let title = v.at(&[0, 0, 0]).unwrap();
    assert_eq!(title.line, 15);
    assert_eq!(title.arg.as_ref().unwrap().words.as_deref(), Some("Paused"));
    assert_eq!(
        &MENU[title.span.clone()],
        "text \"Paused\" [size 28, bold];"
    );
    let resume = v.at(&[0, 0, 1]).unwrap();
    assert!(resume.action.is_some(), "a button's {{ }} is what it does");
    assert_eq!(resume.style_text(MENU, "hover"), Some("[background red5]"));
    assert!(resume.has("blue"));
    let counted = v.at(&[0, 0, 3]).unwrap();
    assert_eq!(counted.arg.as_ref().unwrap().words, None, "code makes it");
}

#[test]
fn a_style_changes_only_its_own_letters() {
    let v = view(MENU);
    let column = v.at(&[0, 0]).unwrap();
    let changed = set_style(MENU, column, "gap", Some("20"));
    assert_eq!(
        changed.replace("gap 20", "gap 14"),
        MENU,
        "nothing else moved"
    );
    // A new one goes at the end of its [ ].
    let title = view(&changed).at(&[0, 0, 0]).unwrap().clone();
    let added = set_style(&changed, &title, "color", Some("yellow"));
    assert!(added.contains("text \"Paused\" [size 28, bold, color yellow]; -- the title"));
    // No [ ] yet: one is made after its value.
    let quit = view(&added).at(&[0, 0, 2]).unwrap().clone();
    let made = set_style(&added, &quit, "red", None);
    assert!(made.contains("button \"Quit\" [red];"), "{made}");
    // Taken away, and the [ ] with the last.
    let quit = view(&made).at(&[0, 0, 2]).unwrap().clone();
    let gone = remove_styles(&made, &quit, &["red"]);
    assert!(gone.contains("button \"Quit\";"), "{gone}");
    let title = view(&gone).at(&[0, 0, 0]).unwrap().clone();
    let less = remove_styles(&gone, &title, &["bold"]);
    assert!(less.contains("[size 28, color yellow]"), "{less}");
}

#[test]
fn words_change_in_quotes_and_code_is_left_alone() {
    let v = view(MENU);
    let title = v.at(&[0, 0, 0]).unwrap();
    let changed = set_words(MENU, title, "Paused \"for now\" {really}").unwrap();
    assert!(changed.contains(r#"text "Paused \"for now\" \{really\}" [size 28, bold];"#));
    let back = view(&changed);
    assert_eq!(
        back.at(&[0, 0, 0])
            .unwrap()
            .arg
            .as_ref()
            .unwrap()
            .words
            .as_deref(),
        Some("Paused \"for now\" {really}"),
        "reads back as typed"
    );
    assert_eq!(set_words(MENU, v.at(&[0, 0, 3]).unwrap(), "x"), None);
    assert_eq!(
        set_words(MENU, v.at(&[0, 0]).unwrap(), "x"),
        None,
        "a column has none"
    );
}

#[test]
fn pieces_are_taken_out_put_in_and_moved_on_lines_of_their_own() {
    let v = view(MENU);
    let quit = v.at(&[0, 0, 2]).unwrap();
    let without = remove_piece(MENU, quit);
    assert!(!without.contains("Quit"));
    assert!(without.contains("                }\n                text \"Clicked"));
    let column = view(&without)
        .at(&[0, 0])
        .unwrap()
        .children
        .clone()
        .unwrap();
    let back = insert_piece(&without, &column, 2, "button \"Quit\";");
    assert_eq!(back, MENU, "put back where it was");
    // Moved to the front of its column, then into the stack.
    let moved = move_piece(MENU, "PauseMenu", &[0, 0, 2], &[0, 0], 0).unwrap();
    let column = view(&moved).at(&[0, 0]).unwrap().clone();
    assert_eq!(
        column
            .children
            .as_ref()
            .unwrap()
            .at(&[0])
            .unwrap()
            .arg
            .as_ref()
            .unwrap()
            .words
            .as_deref(),
        Some("Quit")
    );
    let out = move_piece(MENU, "PauseMenu", &[0, 0, 1], &[0], 1).unwrap();
    let resume = view(&out).at(&[0, 1]).unwrap().clone();
    assert_eq!(resume.kind, "button");
    assert!(
        out.contains("            button \"Resume\" [blue, hover:[background red5]] {\n                clicks = clicks + 1;"),
        "its lines indented for where it is now:\n{out}"
    );
    assert_eq!(
        move_piece(MENU, "PauseMenu", &[0, 0], &[0, 0], 0),
        None,
        "not into itself"
    );
}

#[test]
fn a_one_line_container_gets_its_new_piece_on_a_line_of_its_own() {
    let text = "assembly A {\n    view {\n        row { text \"a\"; }\n    }\n}\n";
    let row = view(text).at(&[0]).unwrap().children.clone().unwrap();
    let out = insert_piece(text, &row, 1, "text \"b\";");
    let again = view(&out);
    assert_eq!(
        again
            .at(&[0])
            .unwrap()
            .children
            .as_ref()
            .unwrap()
            .pieces
            .len(),
        2
    );
    let empty = "assembly A {\n    view {\n        column {\n        }\n    }\n}\n";
    let column = view(empty).at(&[0]).unwrap().children.clone().unwrap();
    let out = insert_piece(empty, &column, 0, "text \"hi\";");
    assert!(
        out.contains("        column {\n            text \"hi\";\n        }"),
        "{out}"
    );
}

#[test]
fn files_it_can_t_follow_say_why_and_never_panic() {
    assert!(parse("assembly A { view { column [gap 8 { } } }").is_err());
    assert!(parse("assembly A { view { text \"a\" } }").is_err());
    assert!(parse(&"x".repeat(MAX_BYTES + 1)).is_err());
    let deep = format!(
        "assembly A {{ view {{ {}{} }} }}",
        "column { ".repeat(MAX_DEPTH + 2),
        "} ".repeat(MAX_DEPTH + 2)
    );
    assert!(parse(&deep).unwrap_err().contains("deep"));
    assert_eq!(
        parse("assembly Data { has { int a = 1; } }").unwrap(),
        vec![]
    );
}

proptest::proptest! {
    #[test]
    fn reading_and_changing_anything_never_panics(
        cut in 0usize..MENU.len(),
        junk in "[a-z \\[\\]{};\"\\\\,:#-]{0,12}",
    ) {
        let mut text = MENU.to_string();
        let at = (0..=cut).rev().find(|&i| text.is_char_boundary(i)).unwrap_or(0);
        text.insert_str(at, &junk);
        if let Ok(screens) = parse(&text) {
            for s in &screens {
                if let Some(e) = s.view.at(&[0]) {
                    let _ = set_style(&text, e, "gap", Some("3"));
                    let _ = remove_styles(&text, e, &["gap"]);
                    let _ = set_words(&text, e, "x");
                    let _ = remove_piece(&text, e);
                }
                let _ = move_piece(&text, &s.name, &[0], &[], 0);
            }
        }
    }
}

#[test]
fn review_findings_stay_fixed() {
    // A backslash before a letter of more than one byte: no panic.
    assert!(parse("assembly A { view { text \"caf\\é\"; } }").is_ok());
    // A dropdown's new [ ] goes after its choices, so Lust still reads it.
    let text = "assembly A {\n    view {\n        dropdown size in [\"S\", \"M\"];\n    }\n}\n";
    let d = view(text).at(&[0]).unwrap().clone();
    let out = set_style(text, &d, "bold", None);
    assert!(
        out.contains("dropdown size in [\"S\", \"M\"] [bold];"),
        "{out}"
    );
    assert!(parse(&out).is_ok());
    // An input with enter { } and no ; reads.
    let text = "assembly A {\n    view {\n        input name [width 100] enter {\n            go();\n        }\n        text \"after\";\n    }\n}\n";
    assert_eq!(view(text).pieces.len(), 2);
    // A game's own piece is one piece.
    let text = "assembly A {\n    view {\n        TodoRow [width 200] { label = \"x\"; }\n        text \"after\";\n    }\n}\n";
    let v = view(text);
    assert_eq!(v.at(&[0]).unwrap().kind, "TodoRow");
    assert_eq!(v.at(&[1]).unwrap().kind, "text");
    // Windows line ends stay Windows line ends.
    let crlf = MENU.replace('\n', "\r\n");
    let column = view(&crlf).at(&[0, 0]).unwrap().children.clone().unwrap();
    let out = insert_piece(&crlf, &column, 0, "text \"new\";");
    assert!(!out.replace("\r\n", "").contains('\n'), "no bare \\n");
}

#[test]
fn a_value_s_code_is_set_or_added() {
    let text = "assembly A {\n    view {\n        text \"Hi\" [size 9];\n        progress 3;\n        column {}\n        text;\n    }\n}\n";
    let view = parse(text).unwrap().remove(0).view;
    let hi = view.at(&[0]).unwrap();
    assert_eq!(
        set_arg(text, hi, "h.hp.text()").unwrap(),
        text.replace("text \"Hi\" [size 9]", "text h.hp.text() [size 9]")
    );
    let bar = view.at(&[1]).unwrap();
    assert_eq!(
        set_arg(text, bar, "h.hp").unwrap(),
        text.replace("progress 3;", "progress h.hp;")
    );
    assert_eq!(set_arg(text, view.at(&[2]).unwrap(), "x"), None, "a column takes none");
    let bare = view.at(&[3]).unwrap();
    assert_eq!(
        set_arg(text, bare, "x.text()").unwrap(),
        text.replace("text;", "text x.text();")
    );
}
