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
    let view = screens.view("Hud", None, &mut NoReader).unwrap();
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
    texts(
        &screens.view("Hud", None, &mut NoReader).unwrap(),
        &mut said,
    );
    assert_eq!(said, vec!["Hour 8".to_string()]);
    assert!(screens.view("Nope", None, &mut NoReader).is_err());
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
    let err = screens.view("Hud", None, &mut NoReader).unwrap_err();
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
    assert!(screens.view("Hud", None, &mut NoReader).is_err());
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

const HEALTH: &str = "component Health {\n    has {\n        float hp = 20;\n        float maxHp = 20;\n    }\n    method float fraction() {\n        return hp / maxHp;\n    }\n}\ncomponent Position {\n    has { float x = 0; }\n}\npass Health, Position;\nend\n";

const BAR: &str = "fetch \"solis/screen\":Ui;\nfetch \"main\":Health;\n\nassembly Bar {\n    view {\n        Health h = Ui:entity():health();\n        text h.hp.text() + \" of \" + h.maxHp.text();\n        progress h:fraction() [to 1.0];\n        h.hp = 0;\n    }\n}\n\npass Bar;\nend\n";

/// A game whose Health on `drone` has `hp`.
struct Drone(reflect::EntityId, f64);

impl ScreenReader for Drone {
    fn values(&mut self, entity: reflect::EntityId, name: &str) -> Option<reflect::ComponentData> {
        (entity == self.0 && name == "Health").then(|| {
            let mut data = reflect::ComponentData::default();
            data.set("hp", reflect::Value::Float(self.1));
            data.set("maxHp", reflect::Value::Float(20.0));
            data
        })
    }
}

#[test]
fn a_screen_reads_the_entity_it_s_shown_for_through_the_engine() {
    let compiled = compile(&code(&[("main.lst", HEALTH), ("ui/bar.lst", BAR)]))
        .unwrap()
        .unwrap();
    let screen_code = compiled.screens.as_ref().unwrap();
    assert!(
        screen_code
            .readers
            .contains(&("Health".into(), "health".into()))
    );
    // Position's reader would be entities' own position(): left out, said.
    assert!(!screen_code.readers.iter().any(|(n, _)| n == "Position"));
    assert!(
        screen_code.notes.iter().any(|n| n.contains("Position")),
        "{:?}",
        screen_code.notes
    );
    let screens = Screens::start(screen_code).unwrap();
    let drone = reflect::EntityId::random();
    let mut game = Drone(drone, 5.0);
    let view = screens.view("Bar", Some(drone), &mut game).unwrap();
    let mut said = Vec::new();
    texts(&view, &mut said);
    assert_eq!(said, vec!["5.0 of 20.0".to_string()]);
    assert_eq!(
        view.children[1].value,
        Some(0.25),
        "its method works on the copy"
    );
    // The screen changed its copy (hp = 0): the game's is untouched, and
    // the next view reads it again.
    game.1 = 15.0;
    let mut said = Vec::new();
    texts(
        &screens.view("Bar", Some(drone), &mut game).unwrap(),
        &mut said,
    );
    assert_eq!(said, vec!["15.0 of 20.0".to_string()]);
    // Shown for an entity with no Health, or for none: stopped, at its line.
    let err = screens
        .view("Bar", Some(reflect::EntityId::random()), &mut game)
        .unwrap_err();
    assert!(err.message.contains("no Health"), "{err:?}");
    assert_eq!((err.file.as_str(), err.line), ("ui/bar.lst", 6));
    assert!(screens.view("Bar", None, &mut game).is_err());
}

#[test]
fn only_screens_can_fetch_the_readers() {
    let main = HEALTH.replace("pass Health", "fetch \"solis/screen\":Ui;\npass Health");
    assert!(compile(&code(&[("main.lst", &main)])).is_err());
}

const EVENTS: &str =
    "event Resumed();\nevent Bought(string item, int count);\npass Resumed, Bought;\nend\n";

const MENU: &str = "fetch \"solis/screen\":Ui;\nfetch \"events\":[Resumed, Bought];\n\nassembly Menu {\n    has {\n        int clicks = 0;\n    }\n\n    view {\n        column [gap 8] {\n            text \"Clicked \" + clicks.text();\n            button \"Resume\" [blue, hover:[background red5]] {\n                clicks = clicks + 1;\n                send Resumed() to Ui:entity();\n            }\n            button \"Buy\" {\n                send Bought(\"Seeds\", 3) to Ui:entity();\n            }\n        }\n    }\n}\n\npass Menu;\nend\n";

#[test]
fn a_screen_s_buttons_send_events_and_keep_its_own_memory() {
    let compiled = compile(&code(&[
        ("main.lst", MAIN),
        ("events.lst", EVENTS),
        ("ui/menu.lst", MENU),
    ]))
    .unwrap()
    .unwrap();
    let screens = Screens::start(compiled.screens.as_ref().unwrap()).unwrap();
    let id = reflect::EntityId::random();
    let me = Some(id);
    let view = screens.view("Menu", me, &mut NoReader).unwrap();
    let column = &view.children[0];
    assert_eq!(column.children[1].action, Some(0));
    assert_eq!(column.children[2].action, Some(1));
    assert!(matches!(
        column.children[1].styles.iter().find(|(k, _)| k == "hover"),
        Some((_, Style::Nested(inner))) if inner[0].0 == "background"
    ));
    let sent = screens.press("Menu", me, 0, &mut NoReader).unwrap();
    assert_eq!(sent.len(), 1);
    assert_eq!((sent[0].to, sent[0].name.as_str()), (id, "Resumed"));
    let sent = screens.press("Menu", me, 1, &mut NoReader).unwrap();
    assert_eq!(
        sent[0].values,
        vec![reflect::Value::Text("Seeds".into()), reflect::Value::Int(3)]
    );
    // The click changed the screen's own field: the next view shows it.
    let mut said = Vec::new();
    texts(&screens.view("Menu", me, &mut NoReader).unwrap(), &mut said);
    assert_eq!(said, vec!["Clicked 1".to_string()]);
    // A button that isn't there (any more) does nothing.
    assert!(
        screens
            .press("Menu", me, 9, &mut NoReader)
            .unwrap()
            .is_empty()
    );
    assert!(
        screens
            .press("Menu", None, 0, &mut NoReader)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn components_not_in_lust_are_described_to_screens_as_lust_ones() {
    use reflect::{ComponentSchema, FieldInfo, FieldKind, Value};
    let schema = |name: &str, fields: Vec<FieldInfo>| ComponentSchema {
        name: name.into(),
        help: String::new(),
        fields,
    };
    let sparks = schema(
        "Sparks",
        vec![
            FieldInfo::new("shown", "Shown", FieldKind::Int, Value::Int(0)),
            FieldInfo::new("loop", "Loop", FieldKind::Bool, Value::Bool(false)),
            FieldInfo::new("look", "Look", FieldKind::Asset, Value::Asset(None)),
            FieldInfo::new(
                "turn",
                "Turn",
                FieldKind::Rotation,
                Value::floats(&[0.0, 0.0, 0.0, 1.0]),
            ),
        ],
    );
    let (text, readers, notes) = readers_file(
        &[("Health".into(), "main.lst".into())],
        &[
            sparks,
            schema("Health", Vec::new()),
            schema("Screen", Vec::new()),
            schema("Position", Vec::new()),
        ],
    );
    assert!(text.contains("fetch \"main\":Health;"));
    assert!(text.contains("component Sparks {\n    has {\n        int shown = 0;\n        rotation turn = [0, 0, 0];\n    }\n}"), "{text}");
    assert!(!text.contains("loop"), "a Lust word isn't a field name");
    assert!(text.contains("method Sparks entity:sparks() native;"));
    assert!(text.ends_with("pass Ui, Sparks;\nend\n"), "{text}");
    assert_eq!(
        readers,
        vec![
            ("Health".to_string(), "health".to_string()),
            ("Sparks".to_string(), "sparks".to_string())
        ],
        "a Lust component isn't described again"
    );
    assert_eq!(
        notes.len(),
        2,
        "Screen is a project state, position() is taken: {notes:?}"
    );
}
