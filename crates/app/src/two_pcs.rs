//! The same game on two PCs (stage 1; decisions 0120 and 0122): a game
//! recorded on Linux, kept in `tests/two-pcs/`, is played back here, and
//! every tick's checksums are compared with the list of states made when
//! it was recorded. CI plays it on Windows, so a sum worked out
//! differently by another processor, operating system or maths library
//! is caught before it reaches anyone's game; it's played on one core and
//! on four, which must come out the same too.
//!
//! On the first tick that differs, the two states are compared part by
//! part (`world::state_parts`, `Workbench::game_state`): the parts that
//! differ are named, and both states are written to a file under
//! `target/two-pcs/`, which CI keeps when the check fails.
//!
//! Test code only: the check is `two_pcs_tests.rs`.

use crate::check_project::{Read, Report, check_code, workbench_for};
use editor::{PartDifference, StateList, Workbench};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// The folder holding the game, its recording and its list of states.
pub(crate) fn folder() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/two-pcs")
}

/// The game's project, inside [`folder`].
pub(crate) const GAME: &str = "game";
/// The level it was recorded in.
pub(crate) const LEVEL: &str = "yard.sollevel";
/// The recording, inside [`folder`].
pub(crate) const RECORDING: &str = "game.solrec";
/// The list of states, inside [`folder`].
pub(crate) const STATES: &str = "game.states";
/// What the engine's description files say made them.
const MADE_BY: &str = "Two PCs 1.0";
/// How long the recorded game is: ten seconds.
pub(crate) const TICKS: usize = 600;
/// A frame shorter than a tick even at the game's fastest (1.5 times, in
/// the recorded game), so each tick is seen by itself.
pub(crate) const FRAME: f32 = (TICK * 0.6) as f32;
/// How long a tick is, in seconds (the world's `TICK`).
const TICK: f64 = 1.0 / 60.0;

/// The game, opened from a copy of its project (Solis writes its own
/// files into a project it opens), playing unhurried.
pub(crate) struct Game {
    pub(crate) w: Workbench,
    pub(crate) root: PathBuf,
    _temp: reflect::test_support::TempFolder,
}

/// Opens the game as `solis --check-project` would, from a copy.
pub(crate) fn open(name: &str) -> Game {
    let temp = reflect::test_support::TempFolder::new(&format!("two-pcs-{name}")).unwrap();
    let root = temp.join(GAME);
    copy_folder(&folder().join(GAME), &root);
    let project = asset_db::Project::open(&root).unwrap();
    let mut report = Report::default();
    let program = check_code(&project, MADE_BY, &mut report);
    assert!(program.is_some(), "the game's code: {:#?}", report.lines);
    let (db, _, _) = asset_db::AssetDb::open(project, assets::Limits::default());
    let mut w = workbench_for(&root, &db, program, MADE_BY, LEVEL, &mut Read::default()).unwrap();
    w.play_unhurried();
    Game {
        w,
        root,
        _temp: temp,
    }
}

/// Copies the folder `from` to `to`, files and folders inside.
pub(crate) fn copy_folder(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap().flatten() {
        let path = entry.path();
        let target = to.join(entry.file_name());
        if path.is_dir() {
            copy_folder(&path, &target);
        } else {
            std::fs::copy(&path, &target).unwrap();
        }
    }
}

/// The game's world clock: how many ticks have run since Play.
pub(crate) fn ticks_run(w: &Workbench) -> usize {
    w.world().map_or(0, |world| world.clock().ticks as usize)
}

/// Plays the recording back in `game` for `ticks` ticks (or all of it),
/// calling `each` after every tick with its number (from 0); `before`
/// is called before each tick runs. Stops early when `each` says so.
pub(crate) fn replay(
    game: &mut Game,
    ticks: usize,
    mut before: impl FnMut(&mut Workbench, usize),
    mut each: impl FnMut(&mut Workbench, usize) -> bool,
) {
    let text = std::fs::read_to_string(folder().join(RECORDING)).unwrap();
    let recording = input::Recording::read(&text).unwrap();
    let ticks = ticks.min(recording.ticks.len());
    game.w.replay(&text, RECORDING);
    let start = game.w.world().map(|world| world.checksum());
    assert_eq!(
        start,
        Some(recording.start),
        "the level isn't as it was recorded: record the game again (two_pcs_tests.rs, \
         record_the_game)"
    );
    let mut next = 0;
    let mut frames = 0;
    while next < ticks {
        before(&mut game.w, next);
        game.w.play_for(FRAME);
        frames += 1;
        assert!(frames < ticks * 4, "the game stopped moving at tick {next}");
        let run = ticks_run(&game.w);
        if run == next {
            continue;
        }
        assert_eq!(run, next + 1, "two ticks ran in one frame");
        if !each(&mut game.w, next) {
            return;
        }
        next = run;
    }
}

/// The list of states the game comes out as, on `threads` cores, for its
/// first `ticks` ticks.
pub(crate) fn make_list(ticks: usize, threads: usize) -> StateList {
    on_cores(threads, || {
        let mut game = open("list");
        let mut list = StateList::default();
        replay(
            &mut game,
            ticks,
            |_, _| {},
            |w, _| {
                let (world, code) = w.game_checksums().unwrap();
                list.push(world, code, w.game_state().unwrap());
                true
            },
        );
        list
    })
}

/// The comment at the top of the list's file.
pub(crate) const LIST_COMMENT: &str = "\
The state of tests/two-pcs/game.solrec at the end of every tick, made on Linux
(crates/app/src/two_pcs.rs). Don't edit it: a deliberate change to the game's
rules writes it again with SOLIS_UPDATE_TWO_PCS=1 cargo test -p app two_pcs.";

/// The first tick at which a played-back game differed from its list.
#[derive(Debug)]
pub(crate) struct Mismatch {
    /// The tick, from 0.
    pub(crate) tick: usize,
    /// The parts that differ there.
    pub(crate) parts: Vec<PartDifference>,
    /// The file both states were written to.
    pub(crate) dump: PathBuf,
    /// The whole message, for a failing test.
    pub(crate) message: String,
}

/// Plays the game on `threads` cores and holds every tick to `list` (its
/// first `ticks`), with `nudge` called before each tick (a test's broken
/// rule). How many ticks matched and how long it took, or the first that
/// didn't.
pub(crate) fn compare(
    list: &StateList,
    ticks: usize,
    threads: usize,
    mut nudge: impl FnMut(&mut Workbench, usize) + Send,
) -> Result<(usize, Duration), Mismatch> {
    on_cores(threads, || {
        let mut game = open(&format!("cores-{threads}"));
        let started = Instant::now();
        let mut matched = 0;
        let mut found = None;
        let shown_at = std::env::var("SOLIS_TWO_PCS_STATE_AT")
            .ok()
            .and_then(|t| t.parse::<usize>().ok());
        replay(&mut game, ticks.min(list.len()), &mut nudge, |w, tick| {
            if shown_at == Some(tick) {
                write_state(w, tick, threads);
            }
            let sums = w.game_checksums().unwrap();
            if list.checksums(tick) == Some(sums) {
                matched += 1;
                return true;
            }
            found = Some(mismatch(w, list, tick, threads, sums));
            false
        });
        match found {
            Some(m) => Err(m),
            None => Ok((matched, started.elapsed())),
        }
    })
}

/// Writes the game's state at `tick` in full to `target/two-pcs/` (with
/// `SOLIS_TWO_PCS_STATE_AT=<tick>`: what a part the list keeps as a
/// checksum holds on this PC, to compare with another's).
fn write_state(w: &mut Workbench, tick: usize, threads: usize) {
    let state = w.game_state().unwrap_or_default();
    let text: String = state
        .iter()
        .map(|(part, value)| format!("{part}\t{value}\n"))
        .collect();
    let path = dump_folder().join(format!("state-at-tick-{tick}-on-{threads}-cores.txt"));
    let _ = std::fs::create_dir_all(dump_folder());
    if std::fs::write(&path, text).is_ok() {
        eprintln!("Wrote the state at tick {tick} to {}.", path.display());
    }
}

/// What's said, and written to a file, when tick `tick` differs.
fn mismatch(
    w: &mut Workbench,
    list: &StateList,
    tick: usize,
    threads: usize,
    (world, code): (u64, u64),
) -> Mismatch {
    let actual = w.game_state().unwrap_or_default();
    let expected = list.state_at(tick).unwrap_or_default();
    let parts = list.differences(tick, &actual).unwrap_or_default();
    let (want_world, want_code) = list.checksums(tick).unwrap_or_default();
    let dump = dump_folder().join(format!("tick-{tick}-on-{threads}-cores.txt"));
    let mut text = format!(
        "The two-PC check: tick {tick}, on {threads} core(s).\n\
         World checksum: expected {want_world:016x}, got {world:016x}.\n\
         Game code's checksum: expected {want_code:016x}, got {code:016x}.\n\n\
         Parts that differ ({}):\n",
        parts.len()
    );
    for d in &parts {
        text.push_str(&format!(
            "\n{}\n  expected: {}\n  here:     {}\n",
            d.part,
            d.expected.as_deref().unwrap_or("(not there)"),
            d.actual.as_deref().unwrap_or("(not there)")
        ));
    }
    text.push_str(
        "\n(A value the list keeps as # and a checksum is longer than it keeps in full: run \
         the check on the PC that made the list with SOLIS_TWO_PCS_STATE_AT=<tick> to see \
         it there.)\n",
    );
    for (title, state) in [("Here", &actual), ("Expected", &expected)] {
        text.push_str(&format!("\n\n{title}, whole:\n"));
        for (part, value) in state {
            text.push_str(&format!("{part}\t{value}\n"));
        }
    }
    let _ = std::fs::create_dir_all(dump_folder());
    let written = std::fs::write(&dump, &text).is_ok();
    let named: Vec<&str> = parts.iter().take(12).map(|d| d.part.as_str()).collect();
    let message = format!(
        "The recorded game (tests/two-pcs/{RECORDING}) came out differently at tick {tick} \
         ({:.2} s in) on {threads} core(s) from the list made on Linux ({STATES}).\n\
         {}\n\
         Both states are in {}{}.\n\
         What to do: if this pull request changes the game's rules on purpose (physics, game \
         code's calls, what the checksum covers), run `SOLIS_UPDATE_TWO_PCS=1 cargo test -p app \
         two_pcs` on Linux and commit the new list. If not, something works out differently on \
         this PC or with this many cores: find the maths behind the first part named and make it \
         exact (see ARCHITECTURE.md, Checks).",
        tick as f64 * TICK,
        if parts.is_empty() {
            "The checksums differ, but every part the dump shows is the same: something the \
             checksum covers isn't in the dump yet (world::state_parts)."
                .to_string()
        } else {
            format!("Parts that differ: {}.", named.join("; "))
        },
        dump.display(),
        if written {
            ""
        } else {
            " (which couldn't be written)"
        },
    );
    Mismatch {
        tick,
        parts,
        dump,
        message,
    }
}

/// Where a failure's states are written: `target/two-pcs/` in the
/// workspace, which CI uploads when the check fails.
pub(crate) fn dump_folder() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/two-pcs")
}

/// Runs `run` with the job system (and so game code) on `threads` cores,
/// as the editor runs on a PC with that many.
pub(crate) fn on_cores<T: Send>(threads: usize, run: impl FnOnce() -> T + Send) -> T {
    rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .unwrap()
        .install(run)
}

#[cfg(test)]
#[path = "two_pcs_tests.rs"]
mod tests;
