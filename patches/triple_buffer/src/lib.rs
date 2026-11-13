//! Hands the newest value from one thread (the writer) to another (the
//! reader) without either one waiting: the part of the `triple_buffer`
//! crate's interface that Kira uses to pass commands to its audio thread.
//!
//! It stands in for `triple_buffer` (MPL-2.0), so that every dependency of
//! Solis stays permissive (decision 0056). It was written for Solis from
//! the interface alone.
//!
//! How it works: three slots, each owned by one side at a time: the
//! writer's, the reader's and one in the middle. The writer fills its own
//! slot, then swaps it for the middle one in one atomic step, marking the
//! middle new. The reader, finding the middle marked new, swaps its own
//! slot for it the same way and copies the value out. Only a slot's owner
//! ever touches it, so its lock is always free and taking it is a single
//! step that never waits; the lock is there only because the workspace
//! has no `unsafe` code to share a slot without one. A value too wide for
//! one atomic operation therefore never goes through a lock another
//! thread could be holding, as it would in an atomic cell.

use std::fmt;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, TryLockError};

/// The bits of `Shared::middle` that say which slot is in the middle.
const SLOT: u8 = 0b011;
/// The bit of `Shared::middle` set while the middle slot holds a value
/// the reader hasn't taken.
const NEW: u8 = 0b100;

/// What the writer and reader share.
struct Shared<T> {
    slots: [Mutex<T>; 3],
    /// The middle slot's index, and `NEW`.
    middle: AtomicU8,
}

impl<T> Shared<T> {
    /// Slot `index`, taken by its owner. `None` only if the other side
    /// were using it, which ownership rules out; the caller then gives up
    /// for this call instead of waiting.
    fn own(&self, index: u8) -> Option<MutexGuard<'_, T>> {
        match self.slots.get(usize::from(index))?.try_lock() {
            Ok(slot) => Some(slot),
            // Nothing can panic while a slot is held (a copy in or out),
            // but if something did, the value is still whole.
            Err(TryLockError::Poisoned(held)) => Some(held.into_inner()),
            Err(TryLockError::WouldBlock) => None,
        }
    }
}

/// Makes a writer and reader pair, starting with `initial`.
pub fn triple_buffer<T: Copy + Send>(initial: &T) -> (Input<T>, Output<T>) {
    let shared = Arc::new(Shared {
        slots: [
            Mutex::new(*initial),
            Mutex::new(*initial),
            Mutex::new(*initial),
        ],
        middle: AtomicU8::new(1),
    });
    (
        Input {
            shared: Arc::clone(&shared),
            own: 0,
        },
        Output {
            shared,
            own: 2,
            local: *initial,
        },
    )
}

/// The writing end.
pub struct Input<T: Copy + Send> {
    shared: Arc<Shared<T>>,
    /// The slot the writer owns.
    own: u8,
}

impl<T: Copy + Send> Input<T> {
    /// Writes a new value, replacing any the reader hasn't taken yet.
    pub fn write(&mut self, value: T) {
        let Some(mut slot) = self.shared.own(self.own) else {
            return;
        };
        *slot = value;
        drop(slot);
        let was = self.shared.middle.swap(self.own | NEW, Ordering::AcqRel);
        self.own = was & SLOT;
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
    /// The slot the reader owns.
    own: u8,
    /// The value last taken.
    local: T,
}

impl<T: Copy + Send> Output<T> {
    /// Takes the newest value if there's one not yet taken. Returns
    /// whether it did.
    pub fn update(&mut self) -> bool {
        if self.shared.middle.load(Ordering::Acquire) & NEW == 0 {
            return false;
        }
        let was = self.shared.middle.swap(self.own, Ordering::AcqRel);
        self.own = was & SLOT;
        match self.shared.own(self.own) {
            Some(slot) => {
                self.local = *slot;
                true
            }
            None => false,
        }
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

    /// The writer stopped by the system halfway through a write: its slot
    /// is held. The reader must neither wait nor see the half-written
    /// value, and gets what was written before.
    #[test]
    fn a_reader_never_waits_on_a_writer_stopped_mid_write() {
        let (mut input, mut output) = triple_buffer(&[0_u64; 8]);
        input.write([1; 8]);
        let shared = Arc::clone(&input.shared);
        let _writing = shared.slots[usize::from(input.own)].lock().unwrap();
        assert!(output.update());
        assert_eq!(*output.output_buffer(), [1; 8]);
        assert!(!output.update());
        assert_eq!(*output.read(), [1; 8]);
    }

    /// The reader stopped halfway through taking a value: its slot is
    /// held. The writer must not wait, and the newest write still arrives.
    #[test]
    fn a_writer_never_waits_on_a_reader_stopped_mid_read() {
        let (mut input, mut output) = triple_buffer(&0_u32);
        let shared = Arc::clone(&output.shared);
        {
            let _reading = shared.slots[usize::from(output.own)].lock().unwrap();
            for v in 1..=5 {
                input.write(v);
            }
        }
        assert_eq!(*output.read(), 5);
    }

    /// Whatever the order of writes and reads, the three slots stay one
    /// each: the writer's, the reader's and the middle.
    #[test]
    fn each_slot_has_one_owner() {
        let (mut input, mut output) = triple_buffer(&0_u32);
        // A fixed pattern of writes and reads (a xorshift sequence from a
        // fixed start, so the test is the same every run).
        let mut state = 0x2545_f491_u32;
        let mut last_written = 0;
        for step in 1..=10_000_u32 {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            if state.is_multiple_of(3) {
                if output.update() {
                    assert_eq!(*output.output_buffer(), last_written);
                }
            } else {
                input.write(step);
                last_written = step;
            }
            let middle = input.shared.middle.load(Ordering::Acquire) & SLOT;
            let mut owners = [input.own, output.own, middle];
            owners.sort_unstable();
            assert_eq!(owners, [0, 1, 2], "step {step}");
        }
    }

    #[test]
    fn both_ends_say_what_they_are() {
        let (input, output) = triple_buffer(&());
        assert!(format!("{input:?}").contains("Input"));
        assert!(format!("{output:?}").contains("Output"));
    }
}
