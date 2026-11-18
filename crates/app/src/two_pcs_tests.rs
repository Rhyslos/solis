//! The same game on two PCs (stage 1): the recorded game in
//! `tests/two-pcs/` comes out tick by tick as its list of states says, on
//! one core and on four, on whichever PC runs the tests (CI's Windows
//! machine among them, against a list made on Linux). A rule broken on
//! purpose is caught at the tick it was broken and named; the update
//! switch writes a list that then passes.
//!
//! - `SOLIS_UPDATE_TWO_PCS=1 cargo test -p app two_pcs` writes the list
//!   again from this PC's play (on Linux, after a deliberate change to the
//!   game's rules).
//! - `cargo test -p app record_the_game -- --ignored` records the game
//!   again (after the level or its code changes) and writes the list.

use super::*;
use std::collections::BTreeSet;

/// Whether this run writes the list instead of checking against it.
fn updating() -> bool {
    std::env::var_os("SOLIS_UPDATE_TWO_PCS").is_some_and(|v| v == "1")
}

/// The list kept in the repository.
fn kept_list() -> StateList {
    let text = std::fs::read_to_string(folder().join(STATES)).unwrap();
    StateList::read(&text).unwrap()
}

/// Checks the whole recorded game on `threads` cores against the kept
/// list, saying how long it took.
fn check_on(threads: usize) {
    let list = kept_list();
    assert_eq!(list.len(), TICKS, "the list covers the whole recording");
    match compare(&list, TICKS, threads, |_, _| {}) {
        Ok((matched, took)) => {
            assert_eq!(matched, TICKS);
            eprintln!(
                "The two-PC check: {TICKS} ticks matched on {threads} core(s) in {:.2} s.",
                took.as_secs_f64()
            );
        }
        Err(m) => panic!("{}", m.message),
    }
}

#[test]
fn two_pcs_the_recorded_game_matches_its_list_on_one_core() {
    if updating() {
        let list = make_list(TICKS, 1);
        std::fs::write(folder().join(STATES), list.write(LIST_COMMENT)).unwrap();
        eprintln!("Wrote tests/two-pcs/{STATES}: commit it.");
        return;
    }
    check_on(1);
}

#[test]
fn two_pcs_the_recorded_game_matches_its_list_on_four_cores() {
    if updating() {
        return;
    }
    check_on(4);
}

#[test]
fn two_pcs_a_rule_broken_at_a_tick_fails_there_naming_what_changed() {
    let list = kept_list();
    let hive: reflect::EntityId = "2PC00000000E7".parse().unwrap();
    let broken = 50;
    let m = compare(&list, broken + 10, 1, |w, tick| {
        if tick == broken {
            assert!(w.nudge(hive, glam::Vec3::new(0.0, 0.0, 0.001)));
        }
    })
    .unwrap_err();
    assert_eq!(m.tick, broken, "{}", m.message);
    let named: BTreeSet<&str> = m.parts.iter().map(|d| d.part.as_str()).collect();
    assert_eq!(
        named,
        BTreeSet::from([format!("{hive} \"Hive\" Transform (in the world)").as_str()]),
        "{}",
        m.message
    );
    assert!(m.message.contains("at tick 50"), "{}", m.message);
    assert!(m.message.contains("\"Hive\" Transform"), "{}", m.message);
    assert!(m.message.contains("What to do"), "{}", m.message);
    let dump = std::fs::read_to_string(&m.dump).unwrap();
    assert!(dump.contains("Parts that differ (1)"), "{dump}");
    assert!(dump.contains("Expected, whole:"), "{dump}");
    let _ = std::fs::remove_file(&m.dump);
}

#[test]
fn two_pcs_the_update_switch_writes_a_list_that_then_passes() {
    let ticks = 40;
    let list = make_list(ticks, 1);
    let text = list.write(LIST_COMMENT);
    let read = StateList::read(&text).unwrap();
    assert_eq!(read.len(), ticks);
    let (matched, _) =
        compare(&read, ticks, 1, |_, _| {}).unwrap_or_else(|m| panic!("{}", m.message));
    assert_eq!(matched, ticks);
    // The kept list is the same game: its first ticks are these.
    let kept = kept_list();
    for tick in 0..ticks {
        assert_eq!(read.checksums(tick), kept.checksums(tick), "tick {tick}");
    }
}

/// The stick and buttons at each tick of the recording: walking up
/// through the gate zone, across, jumping, back down a diagonal, and the
/// hive's button twice.
fn pad_at(tick: usize) -> input::Pad {
    let left = match tick {
        0..120 => glam::Vec2::new(0.0, 1.0),
        120..240 => glam::Vec2::new(1.0, 0.3),
        240..300 => glam::Vec2::ZERO,
        300..420 => glam::Vec2::new(-0.7, -0.7),
        420..520 => glam::Vec2::new(0.2, -1.0),
        _ => glam::Vec2::ZERO,
    };
    let mut buttons = BTreeSet::new();
    if matches!(tick, 100..104 | 350..354) {
        buttons.insert("A".to_string());
    }
    if matches!(tick, 200..203 | 400..403) {
        buttons.insert("X".to_string());
    }
    input::Pad {
        buttons,
        left,
        right: glam::Vec2::ZERO,
    }
}

/// Records the game again and writes its list: run by hand, on Linux,
/// when the level or its code changes (`cargo test -p app
/// record_the_game -- --ignored`). Also gives the project's sidecars their
/// files' hashes, and keeps the sidecars Solis made for files that had
/// none.
#[test]
#[ignore = "records the two-PC game again; run by hand"]
fn record_the_game() {
    let text = on_cores(1, || {
        let mut game = open("record");
        game.w.play_and_record();
        while ticks_run(&game.w) < TICKS {
            let tick = ticks_run(&game.w);
            game.w.set_gamepad(Some(pad_at(tick)));
            game.w.play_for((world::TICK * 0.6) as f32);
        }
        keep_sidecars(&game.root);
        game.w.recording_text().unwrap()
    });
    std::fs::write(folder().join(RECORDING), text).unwrap();
    let list = make_list(TICKS, 1);
    std::fs::write(folder().join(STATES), list.write(LIST_COMMENT)).unwrap();
}

/// Copies the sidecars of the game's files back to the kept project,
/// each with its file's hash.
fn keep_sidecars(root: &std::path::Path) {
    let kept = folder().join(GAME);
    for entry in std::fs::read_dir(root).unwrap().flatten() {
        let path = entry.path();
        let Some(file) = path
            .to_str()
            .and_then(|p| p.strip_suffix(".solmeta"))
            .map(std::path::PathBuf::from)
        else {
            continue;
        };
        let Ok(bytes) = std::fs::read(&file) else {
            continue;
        };
        let mut hash = asset_db::ContentHash::default();
        hash.update(&bytes);
        let mut sidecar = asset_db::Sidecar::read(&path).unwrap();
        sidecar.hash = format!("{:016x}", hash.finish());
        std::fs::write(kept.join(entry.file_name()), sidecar.to_text()).unwrap();
    }
}
