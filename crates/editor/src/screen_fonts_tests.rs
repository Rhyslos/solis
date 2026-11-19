//! The project's own fonts on the game's screens (stage 1, text layout).

use crate::library::{LibraryItem, LibraryKind};
use crate::test_support::*;
use crate::workbench::Workbench;
use std::sync::Arc;

const INTER: &[u8] = include_bytes!("../assets/fonts/Inter-Regular.ttf");
const MONO: &[u8] = include_bytes!("../assets/fonts/JetBrainsMonoNL-Regular.ttf");

fn screen(font: &str) -> String {
    format!(
        "assembly Shop {{\n    view [font \"{font}\"] {{\n        column {{\n            text \"Welcome\";\n            text \"Price\" [font \"fonts/Mono.ttf\"];\n        }}\n    }}\n}}\npass Shop;\nend\n"
    )
}

fn program(screen: &str) -> Arc<script::Compiled> {
    let code = script::Code {
        files: vec![
            ("main.lst".to_string(), "end\n".to_string()),
            ("ui/shop.lst".to_string(), screen.to_string()),
        ],
        lua: Vec::new(),
        notes: Vec::new(),
    };
    match script::compile(&code) {
        Ok(Some(c)) => Arc::new(c),
        Ok(None) => panic!("no code"),
        Err(problems) => panic!("{}", problems[0]),
    }
}

fn font(path: &str) -> LibraryItem {
    LibraryItem {
        id: None,
        kind: LibraryKind::Font,
        name: path.rsplit('/').next().unwrap().into(),
        path: path.into(),
        thumbnail: None,
    }
}

#[test]
fn fonts_named_by_screens_are_read_and_put_on_them_with_their_fallbacks() {
    let mut w = Workbench::new(info());
    w.set_program(Some(program(&screen("fonts/Lora.ttf, fonts/Inter.ttf"))));
    w.set_library(vec![
        font("fonts/Lora.ttf"),
        font("fonts/Inter.ttf"),
        font("fonts/Mono.ttf"),
    ]);
    w.refresh_screen_fonts();
    assert!(w.font_problems().is_empty(), "{:?}", w.font_problems());
    let wanted: Vec<String> = w.take_wanted_fonts().unwrap().into_iter().collect();
    assert_eq!(
        wanted,
        ["fonts/Inter.ttf", "fonts/Lora.ttf", "fonts/Mono.ttf"]
    );
    assert_eq!(w.take_wanted_fonts(), None, "taken once");
    // Nothing read yet: no fonts, and nothing said.
    assert!(w.open.screen_fonts.files.is_none());
    w.set_font_file("fonts/Inter.ttf".into(), Ok(INTER.to_vec()));
    w.set_font_file("fonts/Mono.ttf".into(), Ok(MONO.to_vec()));
    w.set_font_file("fonts/Lora.ttf".into(), Err("it was taken away".into()));
    w.refresh_screen_fonts();
    let files = w.open.screen_fonts.files.clone().unwrap();
    let names: Vec<&str> = files.names().collect();
    assert_eq!(names, ["fonts/Inter.ttf", "fonts/Mono.ttf"]);
    let problems = w.font_problems();
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert_eq!(
        (problems[0].file.as_str(), problems[0].line, problems[0].column),
        ("ui/shop.lst", 2, 17),
        "at the name, inside the list"
    );
    assert!(problems[0].message.contains("couldn't be read"), "{}", problems[0]);
    assert_eq!(w.all_code_problems().len(), 1, "listed in Problems");
}

#[test]
fn a_font_that_isn_t_one_or_isn_t_there_is_a_problem_at_its_line() {
    let mut w = Workbench::new(info());
    w.set_program(Some(program(&screen("fonts/Lora.png"))));
    w.set_library(vec![font("fonts/Bad.ttf")]);
    w.refresh_screen_fonts();
    let messages: Vec<String> = w
        .font_problems()
        .iter()
        .map(|p| p.message.clone())
        .collect();
    assert_eq!(messages.len(), 2, "{messages:?}");
    assert!(messages[0].contains("isn't a font"), "{messages:?}");
    assert!(
        messages[1].contains("no font fonts/Mono.ttf in the project"),
        "{messages:?}"
    );
    assert_eq!(w.status().problems, 2);
    // A file that isn't a font Solis can read says why.
    w.set_program(Some(program(&screen("fonts/Bad.ttf"))));
    w.refresh_screen_fonts();
    w.set_font_file("fonts/Bad.ttf".into(), Ok(b"not a font at all".to_vec()));
    w.refresh_screen_fonts();
    assert!(
        w.font_problems()[0].message.contains("can't be used"),
        "{:?}",
        w.font_problems()
    );
}

#[test]
fn screens_without_fonts_of_their_own_ask_for_none() {
    let mut w = Workbench::new(info());
    w.set_program(Some(program(
        "assembly Shop {\n    view {\n        text \"Hi\";\n    }\n}\npass Shop;\nend\n",
    )));
    w.refresh_screen_fonts();
    assert!(w.font_problems().is_empty());
    assert_eq!(w.take_wanted_fonts(), None);
    assert!(w.open.screen_fonts.files.is_none());
}

#[test]
fn font_names_seen_as_screens_run_are_found_in_their_views() {
    use ui::view::{Node, Style};
    let view = Node {
        kind: "view".into(),
        styles: vec![("font".into(), Style::Text("a.ttf, b.otf".into()))],
        children: vec![Node {
            kind: "text".into(),
            styles: vec![("font".into(), Style::Text("c.ttf".into()))],
            ..Node::default()
        }],
        ..Node::default()
    };
    let mut seen = std::collections::BTreeSet::new();
    crate::screen_fonts::names_in(&view, &mut seen);
    assert_eq!(seen.into_iter().collect::<Vec<_>>(), ["a.ttf", "b.otf", "c.ttf"]);
}
