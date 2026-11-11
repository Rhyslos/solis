//! Rapier's work, guarded: a panic in it (a state that doesn't hold
//! together, numbers out of the range it can work with) never crashes
//! Solis. 3D and 2D physics each run Rapier's changes through a
//! [`Guard`], which remembers that one failed until it's asked; the
//! physics that failed empties itself, and whoever asks decides what
//! happens next (3D: the game pauses and says why; 2D: the world starts
//! it again).

/// Runs Rapier's work for one physics world, and remembers whether any of
/// it failed.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct Guard {
    /// Some work failed since [`Self::take_broke`] last asked.
    broke: bool,
    /// The next work fails without being run ([`Self::fail_next`]).
    fail_next: bool,
}

impl Guard {
    /// What `work` gives, or `None` if it panicked (remembered as a
    /// break): the caller empties its physics then, since what the work
    /// was changing can't be trusted any more.
    pub(crate) fn run<T>(&mut self, work: impl FnOnce() -> T) -> Option<T> {
        let done = if std::mem::take(&mut self.fail_next) {
            None
        } else {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(work)).ok()
        };
        self.broke |= done.is_none();
        done
    }

    /// Remembers a break found some other way (work on other cores, which
    /// catches its own panics).
    pub(crate) fn note_break(&mut self) {
        self.broke = true;
    }

    /// Whether any work failed since last asked; asking clears it, so
    /// it's said once.
    pub(crate) fn take_broke(&mut self) -> bool {
        std::mem::take(&mut self.broke)
    }

    /// Makes the next work fail as a panic in Rapier would, without
    /// running it: the only way a test can see what follows a break,
    /// since nothing that reaches Rapier through this crate makes it
    /// panic on purpose.
    pub(crate) fn fail_next(&mut self) {
        self.fail_next = true;
    }
}
