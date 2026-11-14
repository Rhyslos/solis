//! The window as an exported game, a frame at a time with no screen
//! behind it: what every frame must do whichever kind of frame it is.

use super::*;
use crate::sound::{Opener, Sound};
use sound::SoundDevice;
use std::path::Path;
use std::time::{Duration, Instant};

fn scratch(name: &str) -> reflect::test_support::TempFolder {
    reflect::test_support::TempFolder::new(&format!("window-{name}")).unwrap()
}

fn works() -> Result<SoundDevice, String> {
    Ok(SoundDevice::for_tests())
}

fn fails() -> Result<SoundDevice, String> {
    Err("there's no sound card".to_owned())
}

/// An exported game's window: its game in `dir/game` (not there, so the
/// window only says it can't be read), the player's folder `dir/saved`,
/// and its sound card opened by `opener`.
fn game(dir: &Path, opener: Opener) -> (egui::Context, App) {
    let ctx = egui::Context::default();
    let cc = eframe::CreationContext::_new_kittest(ctx.clone());
    let found = Found {
        root: dir.join("game"),
        playtest: editor::export_dialog::Playtest::default(),
    };
    let mut app = App::new(
        &cc,
        crate::app_info(),
        crate::cli::Launch::default(),
        crate::updates::PendingSlot::default(),
        Some(found),
    );
    app.sound = Sound::with_opener(opener, crate::sound::RETRY);
    app.playtest.as_mut().unwrap().saved = Some(dir.join("saved"));
    (ctx, app)
}

/// One frame, as eframe asks for it.
fn frame(ctx: &egui::Context, app: &mut App) {
    let mut frame = eframe::Frame::_new_kittest();
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
        eframe::App::ui(app, ui, &mut frame);
    });
    // Nothing draws it: the pictures it would upload are let go.
    output.textures_delta.clear();
}

/// Frames until the sound card's worker has been heard from. Without
/// the frame asking, it never is: this gives up after ten seconds.
fn frames_until_the_card_answers(ctx: &egui::Context, app: &mut App) {
    let give_up = Instant::now() + Duration::from_secs(10);
    while app.sound.is_opening() && Instant::now() < give_up {
        frame(ctx, app);
        std::thread::yield_now();
    }
}

/// BUG-01: an exported game's frame returned before the only place the
/// sound card's worker was asked, so the game kept the silent device and
/// played no sound at all.
#[test]
fn an_exported_game_gets_the_sound_card_once_it_opens() {
    let dir = scratch("card");
    let (ctx, mut app) = game(&dir, works);
    assert!(app.sound.device().is_silent(), "silent until it opens");
    frames_until_the_card_answers(&ctx, &mut app);
    assert!(!app.sound.is_opening(), "the frame never asked the worker");
    assert!(
        !app.sound.device().is_silent(),
        "the game plays on the card"
    );
}

/// With no sound card the game says so where its maker can read it: its
/// log, in the player's folder.
#[test]
fn an_exported_game_with_no_sound_card_says_so_in_its_log() {
    let dir = scratch("no-card");
    let (ctx, mut app) = game(&dir, fails);
    frames_until_the_card_answers(&ctx, &mut app);
    assert!(app.sound.device().is_silent());
    let said = app
        .workbench
        .log_lines()
        .iter()
        .filter(|l| l.level == LogLevel::Warning && l.text.starts_with("No sound: "))
        .count();
    assert_eq!(said, 1, "said once");
    // One more frame takes the line into the log; closing writes it.
    frame(&ctx, &mut app);
    app.flush_log(true);
    let log = std::fs::read_to_string(dir.join("saved").join(LOG_FILE)).unwrap();
    assert!(log.contains("No sound: there's no sound card"), "{log}");
}

/// The card is asked for before the frame splits into its kinds, so the
/// editor's frames (the splash, then the launcher) get it as well.
#[test]
fn the_editor_s_frames_get_the_sound_card_too() {
    let ctx = egui::Context::default();
    // Nothing is asked of the network: checking for updates is off.
    let mut storage = super::super::tests::Memory::default();
    eframe::set_value(&mut storage, super::super::AUTO_UPDATE_KEY, &false);
    let mut cc = eframe::CreationContext::_new_kittest(ctx.clone());
    cc.storage = Some(&storage);
    let mut app = App::new(
        &cc,
        crate::app_info(),
        crate::cli::Launch::default(),
        crate::updates::PendingSlot::default(),
        None,
    );
    app.sound = Sound::with_opener(works, crate::sound::RETRY);
    frames_until_the_card_answers(&ctx, &mut app);
    assert!(!app.sound.device().is_silent());
}

/// An exported game of a project with no game code: an area with one
/// entity, and whatever `extra` makes in its assets folder first.
fn game_without_code(dir: &Path, extra: impl Fn(&Path)) -> (egui::Context, App) {
    let root = dir.join("game");
    asset_db::Project::create(&root, "Plain").unwrap();
    let assets = root.join(asset_db::ASSETS_FOLDER);
    extra(&assets);
    let mut area = area::Area::new();
    area.spawn(area::EntityData::new("Rock").with(&area::builtin::Transform::default()));
    let registry = area::builtin::registry().unwrap();
    std::fs::write(
        assets.join("plain.solarea"),
        area::text::write(&area, &registry, &|_| None),
    )
    .unwrap();
    let (ctx, mut app) = game(dir, fails);
    app.playtest.as_mut().unwrap().found.playtest.area = "plain.solarea".into();
    (ctx, app)
}

/// Frames until play starts, or the game says why it can't (its files,
/// or mistakes in its code once checked). Gives up after 30 seconds.
fn frames_until_it_plays(ctx: &egui::Context, app: &mut App) -> Result<(), String> {
    let give_up = Instant::now() + Duration::from_secs(30);
    while Instant::now() < give_up {
        frame(ctx, app);
        if app.workbench.play_state() != editor::PlayState::Editing {
            return Ok(());
        }
        if let Some(why) = app.playtest.as_ref().and_then(|r| r.failed.clone()) {
            return Err(why);
        }
        if !app.game_code.waiting()
            && let Some(why) = app.workbench.code_mistakes()
        {
            return Err(format!("its code has mistakes: {why}"));
        }
        std::thread::yield_now();
    }
    Err(format!(
        "never started: code ready {}, checking {}",
        app.workbench.code_ready(),
        app.game_code.waiting()
    ))
}

/// NEW-10: a game with no code of its own plays, as it does in the
/// editor. The engine's files describing it to game code (written into
/// every project) were taken for game code without a `main.lst`, and
/// with that put right the game still waited for a program there's none
/// of: it never started.
#[test]
fn an_exported_game_with_no_code_starts() {
    let dir = scratch("no-code");
    let (ctx, mut app) = game_without_code(&dir, |_| {});
    assert_eq!(frames_until_it_plays(&ctx, &mut app), Ok(()));
    assert_eq!(app.workbench.code_mistakes(), None);
}

/// N-01: a game written only in Lua starts too: the engine's Lust files
/// beside it don't make it a Lust game missing its `main.lst`.
#[test]
fn an_exported_game_only_in_lua_starts() {
    let dir = scratch("lua-only");
    let (ctx, mut app) = game_without_code(&dir, |assets| {
        std::fs::write(
            assets.join("spin.lua"),
            "local Spin = component(\"Spin\", {})\n",
        )
        .unwrap();
    });
    assert_eq!(frames_until_it_plays(&ctx, &mut app), Ok(()));
    assert_eq!(app.workbench.code_mistakes(), None);
}

/// Nor do folders with no code in them make any.
#[test]
fn an_exported_game_with_only_empty_folders_starts() {
    let dir = scratch("empty-folder");
    let (ctx, mut app) = game_without_code(&dir, |assets| {
        std::fs::create_dir_all(assets.join("code")).unwrap();
        std::fs::create_dir_all(assets.join("solis")).unwrap();
    });
    assert_eq!(frames_until_it_plays(&ctx, &mut app), Ok(()));
}

/// A game whose code has a mistake still says so, and doesn't start.
#[test]
fn an_exported_game_whose_code_has_a_mistake_says_so() {
    let dir = scratch("mistake");
    let (ctx, mut app) = game_without_code(&dir, |assets| {
        std::fs::write(assets.join("hero.lst"), "end\n").unwrap();
    });
    let said = frames_until_it_plays(&ctx, &mut app).unwrap_err();
    assert!(said.contains("main.lst"), "{said}");
}
