//! The UI sample (game UI M7): made as a project, played with its HUD and
//! pause menu, and its recording checked.

use super::*;

const MADE_BY: &str = "Test 1.0";

/// A new UI sample in its own folder, open in a workbench on its area.
fn made(name: &str) -> (std::path::PathBuf, editor::Workbench) {
    let root = std::env::temp_dir()
        .join(format!("solis-ui-sample-{name}-{}", std::process::id()))
        .join("UI sample");
    let _ = std::fs::remove_dir_all(root.parent().unwrap());
    make(&root).unwrap();
    assert!(make(&root).is_err(), "not over an existing project");
    let project = Project::open(&root).unwrap();
    assert_eq!(project.name(), NAME);
    let (db, _, _) = asset_db::AssetDb::open(project, assets::Limits::default());
    crate::game_code::GameCode::describe(&root.join(asset_db::ASSETS_FOLDER), MADE_BY);
    let code = script::gather(&root.join(asset_db::ASSETS_FOLDER));
    let compiled = match script::compile(&code) {
        Ok(Some(compiled)) => compiled,
        Ok(None) => panic!("no code"),
        Err(problems) => panic!("{}", problems[0]),
    };
    let w = crate::check_project::workbench_for(
        &root,
        &db,
        Some(std::sync::Arc::new(compiled)),
        MADE_BY,
        AREA,
        &mut crate::check_project::Read::default(),
    )
    .unwrap();
    (root, w)
}

/// Every text and button's words on the screens shown.
fn words(w: &editor::Workbench) -> Vec<String> {
    fn texts(n: &ui::view::Node, out: &mut Vec<String>) {
        if n.kind == "text" || n.kind == "button" {
            out.push(n.text.clone());
        }
        n.children.iter().for_each(|c| texts(c, out));
    }
    let mut out = Vec::new();
    for s in w.game_screens() {
        texts(s.view.as_ref().unwrap(), &mut out);
    }
    out
}

/// A tap of a gamepad button, then a tick with it up.
fn tap(w: &mut editor::Workbench, button: &str) {
    let mut pad = input::Pad::default();
    pad.buttons.insert(button.to_string());
    w.set_gamepad(Some(pad));
    w.play_for(1.0 / 60.0);
    w.set_gamepad(Some(input::Pad::default()));
    w.play_for(1.0 / 60.0);
}

fn has(w: &editor::Workbench, start: &str) -> bool {
    words(w).iter().any(|t| t.starts_with(start))
}

/// Fires twice, then opens the menu and makes text bigger, goes
/// fullscreen and steps the resolution, then resumes.
fn play_through(w: &mut editor::Workbench) {
    w.play_for(1.0 / 60.0);
    tap(w, "A");
    for _ in 0..20 {
        w.play_for(1.0 / 60.0);
    }
    tap(w, "A");
    w.play_for(1.0 / 60.0);
    tap(w, "Start");
    for button in ["Down", "A", "Down", "A", "Down", "A", "Up", "Up", "Up", "A"] {
        tap(w, button);
    }
    w.play_for(1.0 / 60.0);
}

#[test]
fn the_ui_sample_s_screens_follow_the_game_and_its_menu_changes_the_settings() {
    let (root, mut w) = made("plays");
    w.play();
    w.play_for(1.0 / 60.0);
    assert!(has(&w, "Ammo 12 / 12"), "{:?}", words(&w));
    assert!(has(&w, "Drone 20.0 / 20.0"), "{:?}", words(&w));
    tap(&mut w, "A");
    w.play_for(1.0 / 60.0);
    assert!(has(&w, "Ammo 11 / 12"), "fired: {:?}", words(&w));
    assert!(has(&w, "Hits 1"), "{:?}", words(&w));
    assert!(has(&w, "Drone 17.5 / 20.0"), "{:?}", words(&w));
    // The pause menu: P (Start) stops the game and shows the settings.
    tap(&mut w, "Start");
    assert_eq!(w.world().unwrap().speed(), 0.0, "paused");
    assert!(has(&w, "Text size 100%"), "{:?}", words(&w));
    assert!(has(&w, "Fullscreen: off"), "{:?}", words(&w));
    assert!(has(&w, "Resolution 1280 × 720"), "{:?}", words(&w));
    // Down to Text size, and choose it: a quarter bigger.
    tap(&mut w, "Down");
    tap(&mut w, "A");
    w.play_for(1.0 / 60.0);
    assert_eq!(w.player_settings().text_size, 1.25);
    assert!(has(&w, "Text size 125%"), "{:?}", words(&w));
    assert_eq!(
        w.game_screens()[0].looks.text_percent,
        125,
        "every screen's text grew"
    );
    // Fullscreen, then the next resolution.
    tap(&mut w, "Down");
    tap(&mut w, "A");
    w.play_for(1.0 / 60.0);
    assert!(w.player_settings().fullscreen);
    assert!(has(&w, "Fullscreen: on"), "{:?}", words(&w));
    tap(&mut w, "Down");
    tap(&mut w, "A");
    w.play_for(1.0 / 60.0);
    assert_eq!(w.player_settings().resolution, [1600, 900]);
    assert!(has(&w, "Resolution 1600 × 900"), "{:?}", words(&w));
    // Back up to Resume.
    for _ in 0..3 {
        tap(&mut w, "Up");
    }
    tap(&mut w, "A");
    w.play_for(1.0 / 60.0);
    assert_eq!(w.world().unwrap().speed(), 1.0, "resumed");
    assert!(!has(&w, "Paused"));
    // The settings stay after Stop, for the next play.
    w.stop();
    assert_eq!(w.player_settings().text_size, 1.25);
    let _ = std::fs::remove_dir_all(root.parent().unwrap());
}

#[test]
fn the_ui_sample_plays_the_same_from_a_recording_menus_included() {
    let (root, mut w) = made("recorded");
    w.play_and_record();
    play_through(&mut w);
    let text = w.recording_text().expect("recording");
    w.stop();
    let checks = root
        .join(crate::play_files::RECORDINGS)
        .join(crate::check_project::CHECKS);
    std::fs::create_dir_all(&checks).unwrap();
    std::fs::write(
        checks.join(format!("menu.{}", crate::play_files::RECORDING_EXTENSION)),
        text,
    )
    .unwrap();
    let report = crate::check_project::check(&root, MADE_BY);
    let said = report.text();
    assert_eq!(report.failed, 0, "{said}");
    assert!(said.contains("plays back the same"), "{said}");
    let _ = std::fs::remove_dir_all(root.parent().unwrap());
}

#[test]
fn the_ui_sample_never_replaces_a_file_of_the_person_s() {
    let root = std::env::temp_dir()
        .join(format!("solis-ui-sample-mine-{}", std::process::id()))
        .join("My game");
    let _ = std::fs::remove_dir_all(&root);
    let assets = root.join(asset_db::ASSETS_FOLDER);
    std::fs::create_dir_all(assets.join("ui")).unwrap();
    std::fs::write(assets.join("ui/hud.lst"), "-- mine\nend\n").unwrap();
    let why = make(&root).unwrap_err().to_string();
    assert!(why.contains("hud.lst"), "{why}");
    assert_eq!(
        std::fs::read_to_string(assets.join("ui/hud.lst")).unwrap(),
        "-- mine\nend\n"
    );
    assert!(
        !root.join(asset_db::PROJECT_FILE).exists(),
        "nothing was made"
    );
}

#[test]
fn every_screen_of_the_ui_sample_opens_on_the_canvas() {
    for (path, text) in FILES.iter().filter(|(p, _)| p.starts_with("ui/")) {
        let screens = ui::source::parse(text).unwrap_or_else(|e| panic!("{path}: {e}"));
        assert_eq!(screens.len(), 1, "{path}");
        assert!(
            !screens[0].view.pieces.is_empty(),
            "{path}: its pieces can be picked"
        );
    }
}

#[test]
fn review_findings_stay_fixed() {
    let (root, mut w) = made("review");
    w.play();
    w.play_for(1.0 / 60.0);
    // Twelve shots empty it; it stays empty while reloading, then refills.
    for _ in 0..12 {
        tap(&mut w, "A");
        for _ in 0..16 {
            w.play_for(1.0 / 60.0);
        }
    }
    assert!(has(&w, "Ammo 0 / 12"), "{:?}", words(&w));
    for _ in 0..60 {
        w.play_for(1.0 / 60.0);
    }
    assert!(has(&w, "Ammo 0 / 12"), "a second in: still reloading");
    for _ in 0..70 {
        w.play_for(1.0 / 60.0);
    }
    assert!(has(&w, "Ammo 12 / 12"), "{:?}", words(&w));
    // Quit closes the game: in the editor, play stops (decision 0093).
    tap(&mut w, "Start");
    for _ in 0..4 {
        tap(&mut w, "Down");
    }
    tap(&mut w, "A");
    w.play_for(1.0 / 60.0);
    assert_eq!(w.play_state(), editor::PlayState::Editing, "play stopped");
    assert!(
        w.log_lines()
            .iter()
            .any(|l| l.text.contains("the exported game would close")),
        "the Console says so"
    );
    let _ = std::fs::remove_dir_all(root.parent().unwrap());
}
