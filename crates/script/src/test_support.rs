//! What other crates' tests need of game code (the `test-support`
//! feature, only ever turned on by a dev-dependency; the review's
//! TEST-02).
//!
//! Game code's time limits (100 ms a call, 250 ms a tick, 1 s for a
//! load's starts) are kept on the real clock, so a test thread the machine
//! keeps off the processor uses them up while running nothing: a tick is
//! cut short, or a slow call stopped, though nothing was wrong. A test
//! that compares one run of a game with another gives its game and its
//! code limits no busy machine reaches, here, game by game. Never all
//! games at once: other tests rely on the real limits to stop code that
//! never ends.

use crate::code::{Code, CodeProblem, Compiled, compile_within};
use crate::game::Game;
use crate::luau::Limits;

/// Gives `game` limits of an hour each: only code that never ends is
/// stopped (and the test with it).
pub fn unhurried(game: &mut Game) {
    game.set_limits(Limits::unhurried());
}

/// Gives `game` a game's limits, but an hour for each call: for tests of
/// the tick's own limit, whose slow calls a busy machine would otherwise
/// stop one by one.
pub fn calls_unhurried(game: &mut Game) {
    game.set_limits(Limits::calls_unhurried());
}

/// [`crate::compile`], with the code's own start and the checking given an
/// hour.
pub fn compile_unhurried(code: &Code) -> Result<Option<Compiled>, Vec<CodeProblem>> {
    compile_within(code, Limits::unhurried())
}
