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
        Vec::new(),
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
        Vec::new(),
        crate::updates::PendingSlot::default(),
        None,
    );
    app.sound = Sound::with_opener(works, crate::sound::RETRY);
    frames_until_the_card_answers(&ctx, &mut app);
    assert!(!app.sound.device().is_silent());
}
