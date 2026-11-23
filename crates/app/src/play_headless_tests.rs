//! `solis --play` (Fixes 4, T1): the UI sample played without a window,
//! with and without a script of input, printing the Console and the
//! components asked for; mistakes in what's asked are said plainly.

use super::*;
use std::path::PathBuf;

const MADE_BY: &str = "Test 1.0";

fn ui_sample(name: &str) -> reflect::test_support::TempFolder {
    let temp = reflect::test_support::TempFolder::new(&format!("play-{name}")).unwrap();
    crate::ui_sample::make(&temp.join("UI")).unwrap();
    temp
}

fn asked(folder: PathBuf) -> Play {
    Play {
        folder,
        level: None,
        ticks: None,
        inputs: None,
        print: Vec::new(),
    }
}

#[test]
fn a_script_presses_jump_and_the_turret_fires() {
    let temp = ui_sample("script");
    let script = temp.join("fire.txt");
    // Three shots, a quarter second apart (the turret's cooldown).
    std::fs::write(
        &script,
        "-- fire three times\nat 5 press Jump\nat 25 press Jump\nat 45 press Jump\n",
    )
    .unwrap();
    let mut play = asked(temp.join("UI"));
    play.inputs = Some(script);
    play.print = vec!["Turret".into(), "Health".into()];
    let played = super::play(&play, MADE_BY);
    assert!(played.ok, "{}", played.text);
    // A script's play lasts to its last line and a tick more.
    assert!(
        played
            .text
            .starts_with("Playing ui.sollevel for 47 ticks, with "),
        "{}",
        played.text
    );
    let turret = played
        .text
        .lines()
        .find(|l| l.contains("Turret on"))
        .unwrap_or_else(|| panic!("{}", played.text));
    assert!(
        turret.contains("ammo = 9") && turret.contains("hits = 3"),
        "{turret}"
    );
    assert!(played.text.contains("Health on"), "{}", played.text);
}

#[test]
fn without_a_script_nothing_is_pressed() {
    let temp = ui_sample("idle");
    let mut play = asked(temp.join("UI"));
    play.ticks = Some(20);
    play.print = vec!["Turret".into(), "Nobody".into()];
    let played = super::play(&play, MADE_BY);
    assert!(played.ok, "{}", played.text);
    assert!(played.text.contains("for 20 ticks"), "{}", played.text);
    assert!(played.text.contains("hits = 0"), "{}", played.text);
    assert!(
        played.text.contains("no entity has a Nobody"),
        "{}",
        played.text
    );
}

#[test]
fn what_can_t_be_played_is_said() {
    let temp = ui_sample("wrong");
    let mut play = asked(temp.join("UI"));
    play.level = Some("levels/none.sollevel".into());
    let played = super::play(&play, MADE_BY);
    assert!(
        !played.ok && played.text.contains("can't be played"),
        "{}",
        played.text
    );

    let mut play = asked(temp.join("UI"));
    let script = temp.join("bad.txt");
    std::fs::write(&script, "at ten press Jump\n").unwrap();
    play.inputs = Some(script);
    let played = super::play(&play, MADE_BY);
    assert!(
        !played.ok && played.text.contains("line 1"),
        "{}",
        played.text
    );

    let played = super::play(&asked(temp.join("nowhere")), MADE_BY);
    assert!(
        !played.ok && played.text.contains("can't be opened"),
        "{}",
        played.text
    );

    // An action the project hasn't is said, and the play goes on.
    let mut play = asked(temp.join("UI"));
    let script = temp.join("unknown.txt");
    std::fs::write(&script, "at 1 press Teleport\n").unwrap();
    play.inputs = Some(script);
    let played = super::play(&play, MADE_BY);
    assert!(played.ok, "{}", played.text);
    assert!(
        played
            .text
            .contains("Teleport, which isn't one of the project's actions"),
        "{}",
        played.text
    );
}
