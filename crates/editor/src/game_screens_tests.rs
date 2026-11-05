//! The game's screens while it plays (decision 0087).

use crate::test_support::*;
use crate::workbench::Workbench;
use area::EntityData;
use area::builtin::Screen;
use reflect::EntityId;
use std::sync::Arc;

const MAIN: &str = "object Clock {\n    has { int hour = 6; }\n    method update(float dt) {\n        hour = hour + 1;\n    }\n}\npass Clock;\nend\n";

fn hud(says: &str) -> String {
    format!(
        "fetch \"main\":Clock;\nassembly Hud {{\n    view {{\n        column [pad 8] {{\n            {says}\n        }}\n    }}\n}}\npass Hud;\nend\n"
    )
}

/// The program for `main.lst` and `ui/hud.lst`, and `ui/menu.lst` when given.
fn program(hud_text: &str, menu: Option<&str>) -> Arc<script::Compiled> {
    let mut files = vec![
        ("main.lst".to_string(), MAIN.to_string()),
        ("ui/hud.lst".to_string(), hud_text.to_string()),
    ];
    if let Some(menu) = menu {
        files.push(("ui/menu.lst".to_string(), menu.to_string()));
    }
    let code = script::Code {
        files,
        lua: Vec::new(),
        notes: Vec::new(),
    };
    match script::compile(&code) {
        Ok(Some(c)) => Arc::new(c),
        Ok(None) => panic!("no code"),
        Err(problems) => panic!("{}", problems[0]),
    }
}

/// A workbench with the program and an entity showing each `(screen,
/// shown, order)`.
fn showing(program: Arc<script::Compiled>, screens: &[(&str, bool, i64)]) -> Workbench {
    let mut w = Workbench::new(info(), None);
    w.set_program(Some(program));
    for (i, (name, shown, order)) in screens.iter().enumerate() {
        let entity = EntityData::new(&format!("Shower {i}")).with(&Screen {
            screen: (*name).into(),
            shown: *shown,
            order: *order,
        });
        w.document
            .edit(
                "Add",
                vec![area::Edit::Spawn {
                    entities: vec![(EntityId::random(), entity)],
                }],
                None,
                None,
            )
            .unwrap();
    }
    w
}

/// The words each screen drawn shows, in drawing order.
fn drawn(w: &Workbench) -> Vec<String> {
    fn texts(node: &ui::view::Node, out: &mut Vec<String>) {
        if node.kind == "text" {
            out.push(node.text.clone());
        }
        for c in &node.children {
            texts(c, out);
        }
    }
    let mut out = Vec::new();
    for picture in &w.game_screens.pictures {
        texts(picture.view.as_ref().unwrap(), &mut out);
    }
    out
}

#[test]
fn a_shown_screen_is_drawn_after_each_tick_with_the_game_s_objects() {
    let mut w = showing(
        program(&hud("text \"Hour \" + Clock.hour.text();"), None),
        &[("Hud", true, 0)],
    );
    w.play();
    w.run_game(1.0 / 60.0);
    let first = drawn(&w);
    assert_eq!(first.len(), 1);
    assert!(first[0].starts_with("Hour "), "{first:?}");
    let hour: i64 = first[0].trim_start_matches("Hour ").parse().unwrap();
    w.run_game(1.0 / 60.0);
    assert_eq!(drawn(&w), vec![format!("Hour {}", hour + 1)]);
    assert_eq!(w.game_screens.pictures[0].design, [1920.0, 1080.0]);
    w.stop();
    assert!(w.game_screens.pictures.is_empty(), "Stop takes it away");
}

#[test]
fn screens_are_drawn_in_their_order_and_hidden_ones_aren_t() {
    let menu = "assembly Menu {\n    view {\n        text \"Menu\";\n    }\n}\npass Menu;\nend\n";
    let mut w = showing(
        program(&hud("text \"Hud\";"), Some(menu)),
        &[
            ("Menu", true, 5),
            ("Hud", true, 1),
            ("Hud", false, 0),
            ("", true, 0),
        ],
    );
    w.play();
    w.run_game(1.0 / 60.0);
    assert_eq!(drawn(&w), vec!["Hud".to_string(), "Menu".to_string()]);
}

#[test]
fn a_screen_that_fails_keeps_its_last_picture_and_is_said_once() {
    // Fails from the third hour on.
    let mut w = showing(
        program(
            &hud("is Clock.hour > 7? then { Clock.hour = 0; }\n            text \"Fine\";"),
            None,
        ),
        &[("Hud", true, 0)],
    );
    w.play();
    for _ in 0..6 {
        w.run_game(1.0 / 60.0);
    }
    assert_eq!(drawn(&w), vec!["Fine".to_string()], "the last good picture");
    let said: Vec<String> = w
        .console
        .lines()
        .iter()
        .map(|l| l.text.clone())
        .filter(|l| l.contains("The screen Hud stopped"))
        .collect();
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(said[0].contains("ui/hud.lst"), "{}", said[0]);
}

#[test]
fn the_screens_names_are_offered_for_a_screen_component() {
    let w = showing(program(&hud("text \"x\";"), None), &[]);
    assert_eq!(w.screen_names(), vec!["Hud".to_string()]);
    assert!(Workbench::new(info(), None).screen_names().is_empty());
}
