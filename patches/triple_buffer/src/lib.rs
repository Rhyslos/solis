//! Hands the newest value from one thread (the writer) to another (the
//! reader) without either one waiting: the part of the `triple_buffer`
//! crate's interface that Kira uses to pass commands to its audio thread.
//!
//! It stands in for `triple_buffer` (MPL-2.0), so that every dependency of
//! Solis stays permissive (decision 0056). It was written for Solis from
//! the interface alone.
//!
//! How it works: the value lives in a `crossbeam_utils::atomic::AtomicCell`
//! (copied whole, never torn), with a counter beside it that is odd while
//! a write is in progress (a "seqlock"). The reader takes a value only if
//! the counter was even and unchanged across the copy, so it never sees a
//! value twice or out of order. Neither side blocks: a reader that keeps
//! meeting writes in progress gives up for this call and picks the value
//! up next time.

use crossbeam_utils::atomic::AtomicCell;
use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// How many times a read retries while a write is in progress before
/// giving up for this call.
const READ_TRIES: usize = 8;

/// What the writer and reader share.
struct Shared<T> {
    value: AtomicCell<T>,
    /// Odd while a write is in progress; goes up by 2 with each write.
    sequence: AtomicU64,
}

/// Makes a writer and reader pair, starting with `initial`.
pub fn triple_buffer<T: Copy + Send>(initial: &T) -> (Input<T>, Output<T>) {
    let shared = Arc::new(Shared {
        value: AtomicCell::new(*initial),
        sequence: AtomicU64::new(0),
    });
    (
        Input {
            shared: Arc::clone(&shared),
        },
        Output {
            shared,
            seen: 0,
            local: *initial,
        },
    )
}

/// The writing end.
pub struct Input<T: Copy + Send> {
    shared: Arc<Shared<T>>,
}

impl<T: Copy + Send> Input<T> {
    /// Writes a new value, replacing any the reader hasn't taken yet.
    pub fn write(&mut self, value: T) {
        self.shared.sequence.fetch_add(1, Ordering::AcqRel);
        self.shared.value.store(value);
        self.shared.sequence.fetch_add(1, Ordering::Release);
    }
}

impl<T: Copy + Send> fmt::Debug for Input<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Input").finish_non_exhaustive()
    }
}

/// The reading end.
pub struct Output<T: Copy + Send> {
    shared: Arc<Shared<T>>,
    /// The write last taken.
    seen: u64,
    /// The value last taken.
    local: T,
}

impl<T: Copy + Send> Output<T> {
    /// Takes the newest value if there's one not yet taken. Returns
    /// whether it did.
    pub fn update(&mut self) -> bool {
        for _ in 0..READ_TRIES {
            let before = self.shared.sequence.load(Ordering::Acquire);
            if before % 2 == 1 {
                std::hint::spin_loop();
                continue;
            }
            if before == self.seen {
                return false;
            }
            let value = self.shared.value.load();
            if self.shared.sequence.load(Ordering::Acquire) == before {
                self.local = value;
                self.seen = before;
                return true;
            }
        }
        false
    }

    /// The value last taken.
    pub fn output_buffer(&self) -> &T {
        &self.local
    }

    /// The value last taken, to change in place.
    pub fn output_buffer_mut(&mut self) -> &mut T {
        &mut self.local
    }

    /// Takes the newest value, if any, and returns the value now held.
    pub fn read(&mut self) -> &T {
        self.update();
        &self.local
    }
}

impl<T: Copy + Send> fmt::Debug for Output<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Output").finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_new_until_something_is_written() {
        let (mut input, mut output) = triple_buffer(&Some(1));
        assert!(!output.update());
        assert_eq!(*output.output_buffer(), Some(1));
        input.write(Some(2));
        assert!(output.update());
        assert_eq!(*output.output_buffer_mut(), Some(2));
        assert!(!output.update(), "a value is taken once");
    }

    #[test]
    fn the_newest_write_wins() {
        let (mut input, mut output) = triple_buffer(&0_u32);
        for v in 1..=10 {
            input.write(v);
        }
        assert_eq!(*output.read(), 10);
        assert!(!output.update());
    }

    #[test]
    fn values_bigger_than_a_word_are_never_torn() {
        let (mut input, mut output) = triple_buffer(&[0_u64; 8]);
        let writer = std::thread::spawn(move || {
            for v in 1..=200_000_u64 {
                input.write([v; 8]);
            }
        });
        let mut last = 0;
        while last < 200_000 {
            if output.update() {
                let got = *output.output_buffer();
                assert!(got.iter().all(|&x| x == got[0]), "torn: {got:?}");
                assert!(got[0] > last, "went backwards or repeated");
                last = got[0];
            }
        }
        writer.join().unwrap();
    }

    #[test]
    fn both_ends_say_what_they_are() {
        let (input, output) = triple_buffer(&());
        assert!(format!("{input:?}").contains("Input"));
        assert!(format!("{output:?}").contains("Output"));
    }
}
