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
        if node.kind == "text" || node.kind == "button" {
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

/// Game code that hides the menu when its button says so and shows the
/// HUD instead, with the project's names.
const PAUSER: &str = "fetch \"solis/project\":Screen;\n\
    event Resumed();\n\
    component Pauser {\n\
        has {\n\
            int ticks = 0;\n\
            int resumed = 0;\n\
            bool sawMenu = false;\n\
        }\n\
        method update(float dt) {\n\
            ticks = ticks + 1;\n\
            sawMenu = screenShown(Screen.Menu);\n\
        }\n\
        on Resumed() {\n\
            resumed = resumed + 1;\n\
            hideScreen(Screen.Menu);\n\
            showScreen(Screen.Hud);\n\
        }\n\
    }\n\
    pass Pauser, Resumed;\nend\n";

/// A menu that's one big button.
const RESUME: &str = "fetch \"solis/screen\":Ui;\nfetch \"main\":Resumed;\n\
    assembly Menu {\n    view {\n        stack [width fill, height fill] {\n            \
    button \"Resume\" [width fill, height fill] {\n                send Resumed() to Ui:entity();\n            }\n        }\n    }\n}\npass Menu;\nend\n";

fn pauser_program() -> Arc<script::Compiled> {
    let (mut files, _) = script::describe::descriptions_for(
        "Test Engine 9.9.9",
        "solis",
        &script::project_names::Names::standard().and_screens(&["Hud", "Menu"]),
    );
    files.push(("main.lst".into(), PAUSER.into()));
    files.push(("ui/menu.lst".into(), RESUME.into()));
    files.push((
        "ui/hud.lst".into(),
        "assembly Hud {\n    view {\n        text \"Hud\";\n    }\n}\npass Hud;\nend\n".into(),
    ));
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

/// A workbench showing the menu on an entity with a Pauser.
fn with_a_menu() -> (Workbench, EntityId) {
    let mut w = showing(pauser_program(), &[("Menu", true, 0)]);
    let id = w
        .document
        .area()
        .iter()
        .find(|(_, data)| data.get::<Screen>().is_some())
        .unwrap()
        .0;
    w.apply_property(
        crate::properties::PropertyRequest::AddComponent {
            entity: id,
            component: "Pauser".into(),
        },
        1,
    );
    (w, id)
}

fn pauser(w: &mut Workbench, id: EntityId, field: &str) -> reflect::Value {
    let game = w.playing.as_mut().unwrap().game.as_mut().unwrap();
    game.values(id, "Pauser")
        .unwrap()
        .get(field)
        .unwrap()
        .clone()
}

#[test]
fn a_button_clicked_sends_its_event_in_the_next_tick_and_game_code_hides_the_screen() {
    let (mut w, id) = with_a_menu();
    w.play();
    w.run_game(1.0 / 60.0);
    assert_eq!(drawn(&w), vec!["Resume".to_string()]);
    assert!(
        w.game_screens.wants_pointer(),
        "a screen with a button frees the pointer"
    );
    assert_eq!(pauser(&mut w, id, "sawMenu"), reflect::Value::Bool(true));
    let area = crate::game_screens::ScreensArea {
        origin: [0, 0],
        size: [1280, 720],
        step: None,
    };
    w.game_screens.point_at(Some([640.0, 360.0]), area);
    assert_eq!(w.game_screens.hover, Some((0, 0)));
    assert_eq!(
        w.game_screens.pictures[0].looks.hover,
        Some(0),
        "drawn hovered"
    );
    w.press_hovered();
    assert_eq!(w.game_screens.sent.len(), 1, "waiting for the next tick");
    assert_eq!(pauser(&mut w, id, "resumed"), reflect::Value::Int(0));
    w.run_game(1.0 / 60.0);
    assert!(w.game_screens.sent.is_empty());
    assert_eq!(pauser(&mut w, id, "resumed"), reflect::Value::Int(1));
    // Hidden at the end of that tick, and the HUD shown instead.
    assert_eq!(drawn(&w), vec!["Hud".to_string()]);
    assert!(!w.game_screens.wants_pointer());
    assert_eq!(
        w.game_screens.hover, None,
        "the button went with its screen"
    );
    w.run_game(1.0 / 60.0);
    assert_eq!(pauser(&mut w, id, "sawMenu"), reflect::Value::Bool(false));
}

#[test]
fn a_pointer_off_the_buttons_or_off_the_panel_hovers_nothing() {
    let (mut w, _) = with_a_menu();
    w.play();
    w.run_game(1.0 / 60.0);
    let area = crate::game_screens::ScreensArea {
        origin: [100, 50],
        size: [400, 300],
        step: Some(2),
    };
    w.game_screens.point_at(Some([50.0, 20.0]), area);
    assert_eq!(w.game_screens.hover, None, "in the bars");
    w.game_screens.point_at(Some([300.0, 200.0]), area);
    assert_eq!(w.game_screens.hover, Some((0, 0)));
    w.game_screens.point_at(None, area);
    assert_eq!(w.game_screens.hover, None);
    assert_eq!(w.game_screens.pictures[0].looks.hover, None);
    w.game_screens.point_at(Some([f32::NAN, 1.0]), area);
    assert_eq!(w.game_screens.hover, None);
    // Nothing hovered: a press does nothing.
    w.press_hovered();
    assert!(w.game_screens.sent.is_empty());
}

fn run_to(w: &mut Workbench, ticks: u64) {
    while w.world().unwrap().clock().ticks < ticks {
        let left = ticks - w.world().unwrap().clock().ticks;
        w.run_game((world::TICK * (left.min(3) as f64) + 1e-9) as f32);
    }
}

/// Clicks the menu's one button, as the Game panel does.
fn click(w: &mut Workbench) {
    let area = crate::game_screens::ScreensArea {
        origin: [0, 0],
        size: [1280, 720],
        step: None,
    };
    w.game_screens.point_at(Some([640.0, 360.0]), area);
    w.press_hovered();
}

#[test]
fn a_recording_keeps_what_buttons_sent_and_its_replay_sends_it_again() {
    let (mut w, id) = with_a_menu();
    w.project = Some(crate::library::ProjectInfo {
        name: "Menus".into(),
        root: std::env::temp_dir(),
        assets: std::env::temp_dir(),
    });
    w.play_and_record();
    run_to(&mut w, 5);
    click(&mut w);
    run_to(&mut w, 20);
    assert_eq!(pauser(&mut w, id, "resumed"), reflect::Value::Int(1));
    w.stop();
    let text = w
        .outbox
        .iter()
        .find_map(|a| match a {
            crate::workbench::WorkbenchAction::SaveRecording(text) => Some(text.clone()),
            _ => None,
        })
        .unwrap();
    let recording = input::Recording::read(&text).unwrap();
    let sent: Vec<&input::Sent> = recording.ticks.iter().flat_map(|t| &t.frame.sent).collect();
    assert_eq!(sent.len(), 1, "kept on its tick");
    assert_eq!((sent[0].to, sent[0].name.as_str()), (id, "Resumed"));
    // Played back, the recording's click arrives; a click meanwhile doesn't.
    w.replay(&text, "menu.solrec");
    run_to(&mut w, 3);
    click(&mut w);
    for _ in 0..100 {
        if w.play_state() == crate::play::PlayState::Paused {
            break;
        }
        w.run_game((world::TICK * 2.0) as f32);
    }
    assert!(
        w.log_lines()
            .iter()
            .any(|l| l.text.contains("matched the recording")),
        "{:?}",
        w.log_lines().iter().map(|l| &l.text).collect::<Vec<_>>()
    );
    assert_eq!(pauser(&mut w, id, "resumed"), reflect::Value::Int(1));
}

#[test]
fn a_screen_game_code_hid_stays_hidden_after_a_load() {
    let (mut w, id) = with_a_menu();
    w.project = Some(crate::library::ProjectInfo {
        name: "Menus".into(),
        root: std::env::temp_dir(),
        assets: std::env::temp_dir(),
    });
    w.play();
    run_to(&mut w, 2);
    click(&mut w);
    run_to(&mut w, 4);
    let world = &w.playing.as_ref().unwrap().world;
    assert_eq!(world.screen_shown(id).as_deref(), Some("Hud"));
    let text = w.save_text().unwrap();
    w.stop();
    w.play();
    run_to(&mut w, 1);
    assert_eq!(
        drawn(&w),
        vec!["Resume".to_string()],
        "a new play starts over"
    );
    w.load_game_from_code(&text, "quick");
    w.run_game((world::TICK + 1e-9) as f32);
    assert_eq!(drawn(&w), vec!["Hud".to_string()], "as saved");
}

/// Game code counting what the menu sends, and whether it saw Jump.
const COUNTER: &str = "fetch \"solis/project\":[Screen, Action];\n\
    fetch \"solis/input\":Input;\n\
    event Resumed();\n\
    event Quit();\n\
    component Pauser {\n\
        has {\n\
            int resumed = 0;\n\
            int quit = 0;\n\
            int jumps = 0;\n\
            int choices = 0;\n\
        }\n\
        method update(float dt) {\n\
            is Input:pressed(Action.Jump)? then { jumps = jumps + 1; }\n\
            is Input:pressed(Action.MenuChoose)? then { choices = choices + 1; }\n\
        }\n\
        on Resumed() { resumed = resumed + 1; }\n\
        on Quit() { quit = quit + 1; }\n\
    }\n\
    pass Pauser, Resumed, Quit;\nend\n";

/// A menu of two buttons, one above the other.
const TWO: &str = "fetch \"solis/screen\":Ui;\nfetch \"main\":[Resumed, Quit];\n\
    assembly Menu {\n    view {\n        column [gap 20, pad 40] {\n            \
    button \"Resume\" [width 300] { send Resumed() to Ui:entity(); }\n            \
    button \"Quit\" [width 300] { send Quit() to Ui:entity(); }\n        }\n    }\n}\npass Menu;\nend\n";

fn with_two_buttons() -> (Workbench, EntityId) {
    let (mut files, _) = script::describe::descriptions_for(
        "Test Engine 9.9.9",
        "solis",
        &script::project_names::Names::standard().and_screens(&["Menu"]),
    );
    files.push(("main.lst".into(), COUNTER.into()));
    files.push(("ui/menu.lst".into(), TWO.into()));
    let code = script::Code {
        files,
        lua: Vec::new(),
        notes: Vec::new(),
    };
    let program = match script::compile(&code) {
        Ok(Some(c)) => Arc::new(c),
        Ok(None) => panic!("no code"),
        Err(problems) => panic!("{}", problems[0]),
    };
    let mut w = showing(program, &[("Menu", true, 0)]);
    let id = w
        .document
        .area()
        .iter()
        .find(|(_, data)| data.get::<Screen>().is_some())
        .unwrap()
        .0;
    w.apply_property(
        crate::properties::PropertyRequest::AddComponent {
            entity: id,
            component: "Pauser".into(),
        },
        1,
    );
    (w, id)
}

/// Holds `key` for a tick, then lets it go for one.
fn tap(w: &mut Workbench, key: &str) {
    let now = w.world().unwrap().clock().ticks;
    w.devices.set(key, true);
    run_to(w, now + 1);
    w.devices.set(key, false);
    run_to(w, now + 2);
}

#[test]
fn the_menu_keys_move_the_focus_and_choose_and_the_game_s_own_actions_wait() {
    let (mut w, id) = with_two_buttons();
    w.play();
    w.give_game_input(&egui::Context::default());
    run_to(&mut w, 1);
    assert!(w.game_screens.wants_pointer());
    assert_eq!(w.game_screens.focus, None);
    // The first press finds the first button and shows the ring.
    tap(&mut w, "Down");
    assert_eq!(w.game_screens.focus, Some((0, 0)));
    assert_eq!(w.game_screens.pictures[0].looks.focus, Some(0));
    tap(&mut w, "Down");
    assert_eq!(w.game_screens.focus, Some((0, 1)));
    tap(&mut w, "Down");
    assert_eq!(w.game_screens.focus, Some((0, 1)), "nothing further down");
    // Space chooses (and isn't Jump while the menu is shown).
    tap(&mut w, "Space");
    let now = w.world().unwrap().clock().ticks;
    run_to(&mut w, now + 1);
    assert_eq!(pauser(&mut w, id, "quit"), reflect::Value::Int(1));
    assert_eq!(pauser(&mut w, id, "resumed"), reflect::Value::Int(0));
    assert_eq!(
        pauser(&mut w, id, "jumps"),
        reflect::Value::Int(0),
        "the game didn't jump"
    );
    assert_eq!(
        pauser(&mut w, id, "choices"),
        reflect::Value::Int(1),
        "game code sees the menu's"
    );
    tap(&mut w, "Up");
    assert_eq!(w.game_screens.focus, Some((0, 0)));
    // The mouse over a button takes the focus there, and hides the ring.
    let area = crate::game_screens::ScreensArea {
        origin: [0, 0],
        size: [1920, 1080],
        step: None,
    };
    let quit = (40..1000)
        .step_by(4)
        .find(|&y| {
            w.game_screens.point_at(Some([100.0, y as f32]), area);
            w.game_screens.hover == Some((0, 1))
        })
        .expect("Quit is somewhere down the left");
    assert!(quit > 40);
    assert_eq!(w.game_screens.focus, Some((0, 1)));
    assert_eq!(w.game_screens.pictures[0].looks.focus, None);
}

#[test]
fn with_no_screen_with_buttons_the_game_gets_every_action() {
    let (mut w, id) = with_two_buttons();
    w.document
        .edit(
            "Hide",
            vec![area::Edit::SetField {
                entity: id,
                component: "Screen".into(),
                field: "shown".into(),
                value: Some(reflect::Value::Bool(false)),
            }],
            None,
            None,
        )
        .unwrap();
    w.play();
    w.give_game_input(&egui::Context::default());
    run_to(&mut w, 1);
    tap(&mut w, "Space");
    assert_eq!(pauser(&mut w, id, "jumps"), reflect::Value::Int(1));
    assert_eq!(pauser(&mut w, id, "choices"), reflect::Value::Int(1));
    assert_eq!(w.game_screens.focus, None);
}
