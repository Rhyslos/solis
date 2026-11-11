//! Checks of the game's code take turns, and a check that never finishes
//! is the last one started until it ends.
//!
//! The Lust compiler can't be stopped from outside: a check that runs
//! past its time limit is given up on, and its thread keeps a processor
//! and a large stack for as long as it runs (for ever, if the compiler
//! never finishes). So only one is ever left behind. While it runs, the
//! next check isn't started: the author is told which check never
//! finished and what it was checking, and the code is checked again when
//! that one ends, or when the editor is restarted. One check runs at a
//! time, so two can't be given up on together.

use crate::code::{Code, CodeProblem, Compiled, ENTRY, compiler_stopped};
use crate::luau::Limits;
use std::hash::{Hash, Hasher};
use std::sync::mpsc::RecvTimeoutError;
use std::sync::{Condvar, Mutex, MutexGuard, PoisonError};
use std::thread::JoinHandle;
use std::time::Instant;

/// The stack a check (and the code's start, to read its components) runs
/// with. Lust keeps its own nesting limit and stack since 2.0.0-alpha.35;
/// this is room to spare for Luau compiling what Lust made.
const COMPILER_STACK: usize = 64 << 20;

/// What a check gives back: the code ready to run (`None`: there is no
/// code), or every mistake found.
pub(crate) type Checked = Result<Option<Compiled>, Vec<CodeProblem>>;

/// A check that wasn't started, because one that was given up on is
/// still running ([`crate::code::try_compile`]): what to tell the author.
/// Ask again once [`crate::code::check_unfinished`] says it has ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotStarted(pub CodeProblem);

/// Whose turn it is to check the game's code.
pub(crate) struct Turns {
    turn: Mutex<Turn>,
    /// Told whenever a turn ends.
    ended: Condvar,
}

enum Turn {
    /// Nothing is being checked.
    Free,
    /// A check is running, and is waited for.
    Taken,
    /// A check was given up on, and its thread may still be running.
    GivenUp(Unfinished),
}

/// A check that was given up on.
struct Unfinished {
    worker: JoinHandle<()>,
    began: Instant,
    /// Each file it was checking, with a hash of its text.
    files: Vec<(String, u64)>,
}

impl Turns {
    /// Turns with nothing being checked.
    pub(crate) const fn new() -> Self {
        Self {
            turn: Mutex::new(Turn::Free),
            ended: Condvar::new(),
        }
    }

    /// Whether a check that was given up on is still running.
    pub(crate) fn unfinished(&self) -> bool {
        matches!(&*self.turn(), Turn::GivenUp(unfinished) if !unfinished.worker.is_finished())
    }

    /// Checks `code` with `check` (the compiler; a test's stand-in for
    /// one) on a thread of its own with a large stack, when its turn
    /// comes, and waits for it at most `limits.compile`. Past that it's
    /// given up on, and no check starts until its thread ends.
    pub(crate) fn check(
        &self,
        code: &Code,
        limits: Limits,
        check: impl FnOnce(&Code, Limits) -> Checked + Send + 'static,
    ) -> Result<Checked, NotStarted> {
        let mut turn = self.turn();
        loop {
            match &*turn {
                Turn::Free => break,
                Turn::Taken => {
                    turn = self
                        .ended
                        .wait(turn)
                        .unwrap_or_else(PoisonError::into_inner);
                }
                Turn::GivenUp(unfinished) if unfinished.worker.is_finished() => break,
                Turn::GivenUp(unfinished) => return Err(NotStarted(unfinished.waiting(code))),
            }
        }
        // The one given up on has ended at last: nothing of it is left.
        if let Turn::GivenUp(ended) = std::mem::replace(&mut *turn, Turn::Taken) {
            let _ = ended.worker.join();
        }
        drop(turn);
        let (next, checked) = Self::run(code, limits, check);
        *self.turn() = next;
        self.ended.notify_all();
        Ok(checked)
    }

    /// One check, from start to its answer or to giving up on it, and
    /// what the turn becomes.
    fn run(
        code: &Code,
        limits: Limits,
        check: impl FnOnce(&Code, Limits) -> Checked + Send + 'static,
    ) -> (Turn, Checked) {
        let began = Instant::now();
        let (sender, answer) = std::sync::mpsc::channel();
        let theirs = code.clone();
        let worker = std::thread::Builder::new()
            .name("lust compiler".into())
            .stack_size(COMPILER_STACK)
            .spawn(move || {
                let _ = sender.send(check(&theirs, limits));
            });
        let worker = match worker {
            Ok(worker) => worker,
            Err(why) => {
                let message =
                    format!("the code couldn't be checked: there was no room to start ({why})");
                return (Turn::Free, Err(vec![at_entry(message)]));
            }
        };
        match answer.recv_timeout(limits.compile) {
            // It answered: its thread is ending, and is waited for, so
            // nothing of it is left behind (found by fuzzing).
            Ok(checked) => {
                let _ = worker.join();
                (Turn::Free, checked)
            }
            // A compiler that never finishes (a case no guard knows yet)
            // is given up on, not waited for forever, so the editor isn't
            // left waiting. Its thread can't be stopped: it's kept, so
            // that no other check starts while it runs.
            Err(RecvTimeoutError::Timeout) => {
                let message = format!(
                    "checking the code took more than {} seconds, so it was given up on. \
                     Something in it is more than the checker can handle: undo the last change, \
                     or look for something unusual (please report it). That check is still \
                     running: until it ends, or the editor is restarted, the code can't be \
                     checked again",
                    limits.compile.as_secs()
                );
                let unfinished = Unfinished {
                    worker,
                    began,
                    files: marks(code),
                };
                (Turn::GivenUp(unfinished), Err(vec![at_entry(message)]))
            }
            Err(RecvTimeoutError::Disconnected) => {
                let _ = worker.join();
                (Turn::Free, Err(vec![compiler_stopped()]))
            }
        }
    }

    fn turn(&self) -> MutexGuard<'_, Turn> {
        self.turn.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Unfinished {
    /// What's said to the author whose code (`now`) isn't checked because
    /// this check still runs: when it started, and what it was checking,
    /// as the files that have changed since.
    fn waiting(&self, now: &Code) -> CodeProblem {
        let when = match self.began.elapsed().as_secs() / 60 {
            0 => "less than a minute ago".to_string(),
            1 => "a minute ago".to_string(),
            minutes => format!("{minutes} minutes ago"),
        };
        let now = marks(now);
        let mut changed: Vec<&str> = self
            .files
            .iter()
            .filter(|file| !now.contains(file))
            .chain(now.iter().filter(|file| !self.files.contains(file)))
            .map(|(path, _)| path.as_str())
            .collect();
        changed.sort_unstable();
        changed.dedup();
        let what = match changed.as_slice() {
            [] => "the same code as now".to_string(),
            [one] => format!("the code as it was before {one} changed"),
            [one, two] => format!("the code as it was before {one} and {two} changed"),
            [one, two, rest @ ..] => format!(
                "the code as it was before {one}, {two} and {} more changed",
                rest.len()
            ),
        };
        at_entry(format!(
            "the code wasn't checked: the check that started {when} never finished, and is \
             still running. It was checking {what}. The code is checked again by itself when \
             that check ends, or when the editor is restarted"
        ))
    }
}

/// Each of the code's files with a hash of its text, to tell later which
/// have changed.
fn marks(code: &Code) -> Vec<(String, u64)> {
    code.files
        .iter()
        .chain(&code.lua)
        .map(|(path, text)| {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            text.hash(&mut hasher);
            (path.clone(), hasher.finish())
        })
        .collect()
}

/// A mistake that isn't at a place in a file: said of the code as a whole.
fn at_entry(message: String) -> CodeProblem {
    CodeProblem {
        file: ENTRY.to_string(),
        line: 0,
        column: 0,
        message,
    }
}

#[cfg(test)]
#[path = "check_turns_tests.rs"]
mod tests;
