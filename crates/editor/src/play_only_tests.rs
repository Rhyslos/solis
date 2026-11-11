//! An exported game's workbench (decision 0093).

use super::*;
use crate::test_support::{Picker, info};
use crate::workbench::Workbench;
use egui_kittest::kittest::Queryable;

/// A workbench playing only the game, whose Quitter says the tick and
/// calls `Window:quit()` on its third update.
fn quitter(play_only: bool) -> Workbench {
    quitting_at(play_only, 3)
}

fn quitting_at(play_only: bool, tick: u32) -> Workbench {
    let main = "fetch \"solis/world\":World;\nfetch \"solis/window\":Window;\ncomponent Quitter {\n    has { int ticks = 0; }\n    method update(float dt) {\n        ticks = ticks + 1;\n        World:say(\"tick \" + ticks.text());\n        is ticks == TICK? then {\n            Window:quit();\n        }\n    }\n}\npass Quitter;\nend\n"
        .replace("TICK", &tick.to_string());
    let (mut files, _) = script::describe::descriptions_for(
        "Test Engine 9.9.9",
        "solis",
        &script::project_names::Names::standard(),
    );
    files.push(("main.lst".into(), main));
    let code = script::Code {
        files,
        lua: Vec::new(),
        notes: Vec::new(),
    };
    let program = match script::compile(&code) {
        Ok(Some(c)) => std::sync::Arc::new(c),
        Ok(None) => panic!("no code"),
        Err(problems) => panic!("{}", problems[0]),
    };
    let mut w = Workbench::new(info(), None);
    if play_only {
        w.set_play_only();
    }
    w.set_project(Some(crate::library::ProjectInfo {
        name: "Farm".into(),
        root: "/projects/farm".into(),
        assets: "/projects/farm/assets".into(),
    }));
    w.set_program(Some(program));
    let mut entity = area::EntityData::new("Quitter");
    entity
        .components
        .insert("Quitter".into(), reflect::ComponentData::default());
    let mut area = area::Area::new();
    area.spawn(entity);
    w.open_area(area, std::path::PathBuf::from("farm.solarea"));
    w
}

type State = (Workbench, Picker, NotPlaying);

fn harness(w: Workbench) -> egui_kittest::Harness<'static, State> {
    egui_kittest::Harness::builder()
        .with_size(egui::vec2(640.0, 360.0))
        .build_ui_state(
            |ui, (w, picker, waiting): &mut State| {
                let actions = w.show_play_only(ui, picker, waiting);
                picker.actions.extend(actions);
            },
            (w, Picker::default(), NotPlaying::Starting),
        )
}

#[test]
fn an_exported_game_shows_only_the_game_and_closes_when_its_code_quits() {
    let w = quitter(true);
    assert!(w.play_only() && !w.launcher.open, "no launcher");
    assert!(w.code_ready());
    let mut h = harness(w);
    h.run();
    h.get_by_label("Starting…");
    let ctx = h.ctx.clone();
    h.state_mut().0.start_play_only(&ctx);
    assert!(h.state().0.open.playing.is_some());
    assert!(h.state().0.game_input, "the game has the mouse and keys");
    h.step();
    assert!(
        h.query_by_label_contains("Playing ·").is_none()
            && h.query_by_label_contains("Esc gives").is_none(),
        "no editor's notes over the game"
    );
    for _ in 0..30 {
        h.step();
        h.state_mut().0.run_game(1.0 / 60.0);
        if h.state().0.quit_asked() {
            break;
        }
    }
    h.step();
    let (w, picker, _) = h.state();
    assert!(w.quit_asked());
    assert!(w.open.playing.is_none(), "play stopped");
    assert!(
        picker.actions.contains(&WorkbenchAction::Quit),
        "{:?}",
        picker.actions
    );
    // Nothing of the editor's is said: only the game.
    assert!(
        !w.log_lines()
            .iter()
            .any(|l| l.text.contains("exported game would") || l.text.contains("press Esc")),
        "{:?}",
        w.log_lines().iter().map(|l| &l.text).collect::<Vec<_>>()
    );
    let ticks = w
        .log_lines()
        .iter()
        .filter(|l| l.text.starts_with("tick "))
        .count();
    assert_eq!(ticks, 3, "no tick after quitting");
}

#[test]
fn in_the_editor_quitting_stops_play_and_says_so() {
    let mut w = quitter(false);
    w.play();
    for _ in 0..10 {
        w.run_game(1.0 / 60.0);
    }
    assert!(w.open.playing.is_none());
    assert!(w.log_lines().iter().any(|l| {
        l.text
            .contains("Window:quit(): the exported game would close")
    }));
    // The editor itself stays open.
    let mut actions = Vec::new();
    w.hold_while_playing(&mut actions);
    assert!(actions.is_empty());
}

#[test]
fn esc_is_the_game_s_in_an_exported_game() {
    let w = quitting_at(true, 100_000);
    let mut h = harness(w);
    h.run();
    let ctx = h.ctx.clone();
    h.state_mut().0.start_play_only(&ctx);
    h.key_press(egui::Key::Escape);
    h.step();
    assert!(h.state().0.game_input, "Esc doesn't give the keys away");
}

#[test]
fn code_with_mistakes_says_why_the_game_can_t_start() {
    let mut w = Workbench::new(info(), None);
    w.set_play_only();
    w.set_code_problems(
        vec![script::CodeProblem {
            file: "main.lst".into(),
            line: 4,
            column: 2,
            message: "an unknown name".into(),
        }],
        &[],
    );
    let why = w.code_mistakes().unwrap();
    assert!(why.contains("main.lst, line 4:2: an unknown name"), "{why}");
    let mut h = harness(w);
    h.state_mut().2 = NotPlaying::Cannot(format!("The game can't start.\n\n{why}"));
    h.run();
    assert!(
        h.query_by_label_contains("an unknown name").is_some(),
        "the reason shows"
    );
}

#[test]
fn an_exported_game_s_log_keeps_up_while_the_console_lets_old_lines_go() {
    let mut w = Workbench::new(info(), None);
    let (_, mut seen) = w.log_since(0);
    let mut written = Vec::new();
    for i in 0..5_000 {
        w.log(crate::LogLevel::Warning, format!("line {i}"));
        if i % 7 == 0 {
            let (new, now) = w.log_since(seen);
            written.extend(new.iter().map(|l| l.text.clone()));
            seen = now;
        }
    }
    let (new, _) = w.log_since(seen);
    written.extend(new.iter().map(|l| l.text.clone()));
    let expected: Vec<String> = (0..5_000).map(|i| format!("line {i}")).collect();
    assert_eq!(written, expected, "every line once, in order");
}
