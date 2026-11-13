//! Starting up behind the splash screen ([decision
//! 0045](../../../docs/decisions/0045-a-splash-screen-that-asks-before-updating.md)),
//! as [decision 0090](../../../docs/decisions/0090-always-through-the-launcher.md)
//! left it: the splash shows "Starting…" for at least [`SPLASH_AT_LEAST`],
//! then the editor opens. It no longer waits for GitHub or asks about a
//! newer build: the check starts as Solis does and finishes in
//! `updates.rs`, and a newer build shows on File › Updates.
//!
//! This is the plain logic, without a window, so tests can drive it; the
//! window (`window.rs`) draws the splash and opens the editor when
//! [`done`] says so.

use std::time::Duration;

/// The shortest time the splash shows, so a quick start doesn't just
/// flash it (as Aventail's).
pub const SPLASH_AT_LEAST: Duration = Duration::from_millis(600);

/// What the splash says while it shows.
pub const DOING: &str = "Starting…";

/// Whether the splash, `elapsed` since it appeared, has shown long
/// enough for the editor to open.
pub fn done(elapsed: Duration) -> bool {
    elapsed >= SPLASH_AT_LEAST
}

#[cfg(test)]
#[path = "startup_tests.rs"]
mod tests;
