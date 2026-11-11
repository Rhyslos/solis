//! A check that never finishes (BUG-50): tried with a stand-in for the
//! compiler that waits until the test lets it go, so nothing here really
//! hangs, takes long, or leaves a thread behind.

use super::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

/// A check is given up on after a moment: only the stand-in, which never
/// answers by itself, runs into it.
fn limits() -> Limits {
    Limits {
        compile: Duration::from_millis(20),
        ..Limits::unhurried()
    }
}

fn code(files: &[(&str, &str)]) -> Code {
    Code {
        files: files
            .iter()
            .map(|(path, text)| ((*path).to_string(), (*text).to_string()))
            .collect(),
        lua: Vec::new(),
        notes: Vec::new(),
    }
}

/// A stand-in for code the compiler never finishes: it counts itself
/// started, then waits to be let go (by a message, or by `Sender` being
/// dropped).
fn never_finishes(
    started: &Arc<AtomicUsize>,
) -> (
    Sender<()>,
    impl FnOnce(&Code, Limits) -> Checked + Send + 'static,
) {
    let (let_go, held): (Sender<()>, Receiver<()>) = channel();
    let started = Arc::clone(started);
    (let_go, move |_: &Code, _: Limits| {
        started.fetch_add(1, Ordering::SeqCst);
        let _ = held.recv();
        Ok(None)
    })
}

/// A stand-in for code that checks at once.
fn finishes(started: &Arc<AtomicUsize>) -> impl FnOnce(&Code, Limits) -> Checked + Send + 'static {
    let started = Arc::clone(started);
    move |_: &Code, _: Limits| {
        started.fetch_add(1, Ordering::SeqCst);
        Ok(None)
    }
}

/// Lets the check that was given up on go, and waits until its thread
/// has ended: no test leaves one behind.
fn let_go(turns: &Turns, stuck: Sender<()>) {
    drop(stuck);
    while turns.unfinished() {
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn message(attempt: Result<Checked, NotStarted>) -> String {
    match attempt {
        Ok(Err(problems)) => {
            assert_eq!(problems.len(), 1, "{problems:?}");
            assert_eq!((problems[0].file.as_str(), problems[0].line), (ENTRY, 0));
            format!("checked: {}", problems[0].message)
        }
        Err(NotStarted(waiting)) => {
            assert_eq!((waiting.file.as_str(), waiting.line), (ENTRY, 0));
            format!("not started: {}", waiting.message)
        }
        Ok(Ok(_)) => "checked: sound".to_string(),
    }
}

#[test]
fn a_check_that_never_finishes_is_the_only_one_left_running_however_often_the_code_is_saved() {
    let turns = Turns::new();
    let started = Arc::new(AtomicUsize::new(0));
    let saved = code(&[("gun.lst", "one"), ("main.lst", "end\n")]);
    assert!(!turns.unfinished());
    let (stuck, stand_in) = never_finishes(&started);
    let said = message(turns.check(&saved, limits(), stand_in));
    assert!(
        said.starts_with("checked: checking the code took more than"),
        "{said}"
    );
    assert!(
        said.contains("still running") && said.contains("restarted"),
        "{said}"
    );
    assert!(turns.unfinished());
    // Every save after it: no check is started, and the author is told
    // which one never finished and what it was checking.
    for save in 0..20 {
        let text = format!("change {save}");
        let later = code(&[("gun.lst", &text), ("main.lst", "end\n")]);
        let said = message(turns.check(&later, limits(), finishes(&started)));
        assert_eq!(
            said,
            "not started: the code wasn't checked: the check that started less than a minute \
             ago never finished, and is still running. It was checking the code as it was \
             before gun.lst changed. The code is checked again by itself when that check ends, \
             or when the editor is restarted"
        );
        assert!(turns.unfinished());
    }
    assert_eq!(started.load(Ordering::SeqCst), 1, "only the first");
    // It ends: the next check runs as usual.
    let_go(&turns, stuck);
    let said = message(turns.check(&saved, limits(), finishes(&started)));
    assert_eq!(said, "checked: sound");
    assert_eq!(started.load(Ordering::SeqCst), 2);
    assert!(!turns.unfinished());
}

#[test]
fn the_author_is_told_which_files_changed_since_the_check_that_never_finished() {
    let turns = Turns::new();
    let started = Arc::new(AtomicUsize::new(0));
    let saved = Code {
        lua: vec![("gun.lua".into(), "local a = 1".into())],
        ..code(&[
            ("main.lst", "end\n"),
            ("parts/gun.lst", "one"),
            ("ship.lst", "one"),
        ])
    };
    let (stuck, stand_in) = never_finishes(&started);
    let _ = turns.check(&saved, limits(), stand_in);
    let told = |now: &Code| {
        let said = message(turns.check(now, limits(), finishes(&started)));
        let what = said.split("It was checking ").nth(1).unwrap().to_string();
        what.split(". The code is checked again")
            .next()
            .unwrap()
            .to_string()
    };
    assert_eq!(told(&saved), "the same code as now");
    // One file changed, one taken away and one new, Lust and Lua alike.
    let mut now = saved.clone();
    now.files[1].1 = "two".into();
    assert_eq!(
        told(&now),
        "the code as it was before parts/gun.lst changed"
    );
    now.lua.clear();
    assert_eq!(
        told(&now),
        "the code as it was before gun.lua and parts/gun.lst changed"
    );
    now.files.push(("zeppelin.lst".into(), "new".into()));
    now.files[2].1 = "two".into();
    assert_eq!(
        told(&now),
        "the code as it was before gun.lua, parts/gun.lst and 2 more changed"
    );
    assert_eq!(started.load(Ordering::SeqCst), 1);
    let_go(&turns, stuck);
}

#[test]
fn checks_take_turns_so_two_that_never_finish_are_never_running_together() {
    // Two checks asked for at once, of code that never finishes: whichever
    // goes first is given up on, and the other isn't started.
    let turns = Turns::new();
    let started = Arc::new(AtomicUsize::new(0));
    let saved = code(&[("main.lst", "end\n")]);
    let (first, one) = never_finishes(&started);
    let (second, two) = never_finishes(&started);
    let mut said = std::thread::scope(|scope| {
        let a = scope.spawn(|| message(turns.check(&saved, limits(), one)));
        let b = scope.spawn(|| message(turns.check(&saved, limits(), two)));
        vec![a.join().unwrap(), b.join().unwrap()]
    });
    said.sort();
    assert!(
        said[0].starts_with("checked: checking the code took more than"),
        "{said:?}"
    );
    assert!(
        said[1].starts_with("not started: the code wasn't checked"),
        "{said:?}"
    );
    assert!(said[1].contains("the same code as now"), "{said:?}");
    assert_eq!(started.load(Ordering::SeqCst), 1);
    drop(second);
    let_go(&turns, first);
    // (Whichever of the two went first, letting both go ends it.)
    assert!(!turns.unfinished());
}

#[test]
fn checks_that_finish_take_turns_and_leave_nothing_behind() {
    let turns = Turns::new();
    let started = Arc::new(AtomicUsize::new(0));
    let saved = code(&[("main.lst", "end\n")]);
    std::thread::scope(|scope| {
        for _ in 0..8 {
            scope.spawn(|| {
                let said = message(turns.check(&saved, Limits::unhurried(), finishes(&started)));
                assert_eq!(said, "checked: sound");
            });
        }
    });
    assert_eq!(started.load(Ordering::SeqCst), 8);
    assert!(!turns.unfinished());
    // A compiler that crashes isn't one that never finishes: the next
    // check runs.
    let crashes = |_: &Code, _: Limits| -> Checked { panic!("the compiler crashed") };
    let said = message(turns.check(&saved, Limits::unhurried(), crashes));
    assert!(said.contains("stopped unexpectedly"), "{said}");
    assert!(!turns.unfinished());
    let said = message(turns.check(&saved, Limits::unhurried(), finishes(&started)));
    assert_eq!(said, "checked: sound");
}

#[test]
fn the_program_s_own_checks_take_the_same_turns() {
    // `compile`, `check` and `try_compile` (so the editor, an exported
    // game and --check-project) all go through the program's one set of
    // turns, with the real compiler.
    let sound = code(&[("main.lst", "end\n")]);
    assert!(matches!(crate::code::try_compile(&sound), Ok(Ok(Some(_)))));
    assert!(crate::code::compile(&sound).unwrap().is_some());
    assert!(crate::code::check(&sound).is_empty());
    assert!(matches!(
        crate::code::try_compile(&Code::default()),
        Ok(Ok(None))
    ));
}
