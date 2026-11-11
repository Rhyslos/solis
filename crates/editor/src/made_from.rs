//! "Made from this very value": how something kept from frame to frame (a
//! picture, a texture, a list worked out) knows whether what it was made
//! from is still what's in front of it (the review, BUG-10).
//!
//! The project's sheets, pictures and tilesets and the game's program are
//! each shared as an `Arc`, and replaced by a new one when they change.
//! Where a value sits in memory can't tell one from the next: the moment
//! a value is let go, its place is free for the next one made, so the new
//! one can turn up exactly where the old one was. A cache that remembered
//! only that number then took the new value for the old, and kept showing
//! what it had made from the old one. [`MadeFrom`] remembers the value
//! itself instead, weakly: it doesn't keep the value (or its megabytes of
//! pixels) alive, and nothing else is ever taken for it. [`Numbered`]
//! turns that into a number for each value in use, for keys made of
//! several things.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Weak};

/// The value something was made from. Asked whether another is that very
/// one, it says yes only for the same value, never for one just like it,
/// nor for one made where it used to be.
pub(crate) struct MadeFrom<T>(Weak<T>);

impl<T> MadeFrom<T> {
    /// Made from `from`.
    pub(crate) fn new(from: &Arc<T>) -> Self {
        Self(Arc::downgrade(from))
    }

    /// Whether `now` is the very value this was made from.
    pub(crate) fn is(&self, now: &Arc<T>) -> bool {
        // Held here while they're compared: two values both alive are
        // never in one place. If it's gone, `now` can't be it.
        self.0.upgrade().is_some_and(|from| Arc::ptr_eq(&from, now))
    }

    /// Whether the value it was made from is gone (let go by everyone).
    pub(crate) fn gone(&self) -> bool {
        self.0.strong_count() == 0
    }

    /// The same for something made from a value or from none: whether
    /// `was` and `now` are both none, or the very same value.
    pub(crate) fn same(was: Option<&Self>, now: Option<&Arc<T>>) -> bool {
        match (was, now) {
            (None, None) => true,
            (Some(was), Some(now)) => was.is(now),
            _ => false,
        }
    }
}

impl<T> Clone for MadeFrom<T> {
    fn clone(&self) -> Self {
        Self(Weak::clone(&self.0))
    }
}

impl<T> std::fmt::Debug for MadeFrom<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MadeFrom(..)")
    }
}

/// A number for each value in use (never 0): the same for a value for as
/// long as the value lasts, and never given to another, here or in any
/// other `Numbered` (so a key made with one can't be met again by chance
/// after another project opens). For a key that takes in several values
/// with other numbers, where each value's own number stands in for it.
#[derive(Debug)]
pub(crate) struct Numbered<T> {
    /// Each value known, and its number.
    known: Vec<(MadeFrom<T>, u64)>,
}

impl<T> Default for Numbered<T> {
    fn default() -> Self {
        Self { known: Vec::new() }
    }
}

/// A number no value has had.
fn next_number() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

impl<T> Numbered<T> {
    /// `value`'s number: the one it has, or a new one no value has had.
    pub(crate) fn number(&mut self, value: &Arc<T>) -> u64 {
        if let Some((_, number)) = self.known.iter().find(|(from, _)| from.is(value)) {
            return *number;
        }
        let number = next_number();
        self.known.push((MadeFrom::new(value), number));
        number
    }

    /// Forgets the values that are gone, so the list never outgrows what
    /// there is.
    pub(crate) fn forget_gone(&mut self) {
        self.known.retain(|(from, _)| !from.gone());
    }
}

#[cfg(test)]
#[path = "made_from_tests.rs"]
mod tests;
