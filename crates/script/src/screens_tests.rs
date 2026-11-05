//! Game screens (decision 0087): compiled into their own program, run in
//! their own Luau, reading the game's objects.

use super::*;
use crate::code::{Code, compile};

const MAIN: &str = "object Clock {\n    has { int hour = 6; }\n    method update(float dt) {\n        hour = hour + 1;\n    }\n}\npass Clock;\nend\n";

const HUD: &str = "fetch \"main\":Clock;\n\nassembly Hud {\n    has {\n        int coins = 3;\n    }\n\n    view [width 1920, height 1080] {\n        column [pad 48, gap 8] {\n            text \"Hour \" + Clock.hour.text() [size 32, bold, white];\n            progress coins [to 10, green];\n        }\n    }\n}\n\npass Hud;\nend\n";

fn code(files: &[(&str, &str)]) -> Code {
    Code {
        files: files
            .iter()
            .map(|(p, t)| (p.to_string(), t.to_string()))
            .collect(),
        ..Code::default()
    }
}

fn texts(node: &Node, out: &mut Vec<String>) {
    if node.kind == "text" {
        out.push(node.text.clone());
    }
    for c in &node.children {
        texts(c, out);
    }
}

#[test]
fn screen_names_come_from_their_files() {
    assert_eq!(screen_name("ui/hud.lst").as_deref(), Some("Hud"));
    assert_eq!(
        screen_name("ui/menus/pauseMenu.lst").as_deref(),
        Some("PauseMenu")
    );
    for bad in ["ui/2d.lst", "ui/pause-menu.lst", "ui/.lst", "ui/hud.txt"] {
        assert_eq!(screen_name(bad), None, "{bad}");
    }
    let (found, problems) = find(&[
        ("main.lst".into(), String::new()),
        ("ui/hud.lst".into(), String::new()),
        ("ui/more/hud.lst".into(), String::new()),
        ("ui/bad-name.lst".into(), String::new()),
    ]);
    assert_eq!(found, vec![("Hud".to_string(), "ui/hud.lst".to_string())]);
    assert_eq!(problems.len(), 2);
    assert!(entry(&found).contains("fetch \"ui/hud\":Hud;\nHud:open();"));
}

#[test]
fn a_screen_is_compiled_on_its_own_and_reads_the_game_s_objects() {
    let compiled = compile(&code(&[("main.lst", MAIN), ("ui/hud.lst", HUD)]))
        .unwrap()
        .unwrap();
    let screen_code = compiled.screens.clone().expect("a screen");
    assert_eq!(screen_code.screens[0].0, "Hud");
    let screens = Screens::start(&screen_code).unwrap();
    assert_eq!(screens.names(), vec!["Hud".to_string()]);
    let view = screens.view("Hud").unwrap();
    assert_eq!(view.kind, "view");
    let mut said = Vec::new();
    texts(&view, &mut said);
    assert_eq!(said, vec!["Hour 6".to_string()]);
    assert_eq!(ui::view::design_of(&view), [1920.0, 1080.0]);
    let column = &view.children[0];
    assert_eq!(column.kind, "column");
    assert!(column.styles.contains(&("pad".into(), Style::Number(48.0))));
    assert_eq!(column.children[1].value, Some(3.0), "the bar's value");
    // The game ticks: its clock moves on, and the screen shows it once the
    // objects are copied in.
    let mut game = crate::Runtime::new().unwrap();
    game.load(&compiled, &mut crate::NoHost).unwrap();
    game.tick(0.1, &mut crate::NoHost);
    game.tick(0.1, &mut crate::NoHost);
    screens.set_objects(&game.object_values());
    let mut said = Vec::new();
    texts(&screens.view("Hud").unwrap(), &mut said);
    assert_eq!(said, vec!["Hour 8".to_string()]);
    assert!(screens.view("Nope").is_err());
}

#[test]
fn a_screen_can_t_change_the_game_s_objects() {
    let hud = HUD.replace(
        "text \"Hour \"",
        "is true? then { Clock.hour = 99; }\n            text \"Hour \"",
    );
    let compiled = compile(&code(&[("main.lst", MAIN), ("ui/hud.lst", &hud)]))
        .unwrap()
        .unwrap();
    let screens = Screens::start(compiled.screens.as_ref().unwrap()).unwrap();
    let err = screens.view("Hud").unwrap_err();
    assert!(err.message.contains("can't be changed here"), "{err:?}");
    assert_eq!(err.file, "ui/hud.lst");
    assert!(err.line > 0);
}

#[test]
fn a_screen_that_runs_forever_is_stopped() {
    let hud = HUD.replace(
        "text \"Hour \"",
        "int n = 0;\n            loop { n = n + 1; }\n            text \"Hour \"",
    );
    let compiled = compile(&code(&[("main.lst", MAIN), ("ui/hud.lst", &hud)]))
        .unwrap()
        .unwrap();
    let screens = Screens::start(compiled.screens.as_ref().unwrap()).unwrap();
    let started = std::time::Instant::now();
    assert!(screens.view("Hud").is_err());
    assert!(started.elapsed() < std::time::Duration::from_secs(2));
}

#[test]
fn mistakes_in_screens_name_their_file() {
    // Not passing the assembly named after the file.
    let wrong = HUD
        .replace("pass Hud;", "pass;")
        .replace("assembly Hud", "assembly Other");
    let problems = compile(&code(&[("main.lst", MAIN), ("ui/hud.lst", &wrong)])).unwrap_err();
    assert!(
        problems
            .iter()
            .any(|p| p.message.contains("passes the assembly named after it")),
        "{problems:?}"
    );
    // A mistake inside the screen points at it.
    let typo = HUD.replace("Clock.hour", "Clock.hours");
    let problems = compile(&code(&[("main.lst", MAIN), ("ui/hud.lst", &typo)])).unwrap_err();
    assert!(
        problems
            .iter()
            .any(|p| p.file == "ui/hud.lst" && p.line > 0),
        "{problems:?}"
    );
    // No screens: nothing to run.
    let compiled = compile(&code(&[("main.lst", MAIN)])).unwrap().unwrap();
    assert!(compiled.screens.is_none());
}
