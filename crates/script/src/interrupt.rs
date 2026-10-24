//! The time limits' watchman inside Luau ([decision 0034](../../../docs/decisions/0034-a-raw-interrupt-for-the-time-limits.md)):
//! Luau calls an "interrupt" at every function call and every turn of a
//! loop, and the time limits ([`crate::luau::Clock`]) need it to stop
//! code that runs too long. mlua's own way of setting one costs a fifth of
//! game code's time (it prepares for errors on every call), so Solis sets
//! Luau's directly: a plain function that does nothing unless a clock is
//! running on this thread, and raises the clock's error when it says so.
//!
//! This is the one place Solis's own code is `unsafe`. Every use is
//! explained where it is.

use crate::luau::Clock;
use mlua::ffi;
use std::cell::Cell;
use std::os::raw::c_int;

thread_local! {
    /// The clock watching the game code running on this thread now, if
    /// any.
    static RUNNING: Cell<*const Clock> = const { Cell::new(std::ptr::null()) };
}

/// While it lives, `clock` watches the game code run on this thread; then
/// the one before it does again (none, usually).
pub(crate) struct Watching(*const Clock);

impl Watching {
    /// `clock` watches from now until this is dropped. The clock must
    /// outlive it (it's borrowed for its length).
    pub(crate) fn start(clock: &Clock) -> Self {
        Self(RUNNING.with(|r| r.replace(std::ptr::from_ref(clock))))
    }
}

impl Drop for Watching {
    fn drop(&mut self) {
        RUNNING.with(|r| r.set(self.0));
    }
}

/// Luau's interrupt: asks the running clock, if any, and raises its error
/// as a Lua error when time is up.
unsafe extern "C-unwind" fn interrupt(state: *mut ffi::lua_State, gc: c_int) {
    // Garbage collection's interrupts can't take an error.
    if gc >= 0 {
        return;
    }
    let clock = RUNNING.with(Cell::get);
    if clock.is_null() {
        return;
    }
    // SAFETY: `RUNNING` points at a clock only while a `Watching` for it
    // lives, which borrows the clock for that long, on this thread.
    let Some(message) = (unsafe { &*clock }).verdict() else {
        return;
    };
    // SAFETY: `state` is the thread Luau is running. An interrupt comes at
    // the edge of a call's stack, so room is made for the message first;
    // the text is copied. Without room (the stack at Luau's own limit) the
    // code is still stopped, with whatever error is on top.
    unsafe {
        if ffi::lua_checkstack(state, 2) != 0 {
            ffi::lua_pushlstring(state, message.as_ptr().cast(), message.len());
        }
    }
    drop(message);
    // SAFETY: raising an error from an interrupt (not a garbage
    // collection's) is what Luau allows; nothing of ours needs dropping
    // past this point, so unwinding over this frame skips nothing.
    unsafe { ffi::lua_error(state) }
}

/// Makes `lua`'s interrupt the one above.
pub(crate) fn watch(lua: &mlua::Lua) -> mlua::Result<()> {
    // SAFETY: `exec_raw` hands over Luau's state for this call only; the
    // callbacks are shared by all of its threads, and setting the
    // interrupt is how Luau is told about it.
    unsafe {
        lua.exec_raw::<()>((), |state| {
            (*ffi::lua_callbacks(state)).interrupt = Some(interrupt);
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn running() -> *const Clock {
        RUNNING.with(Cell::get)
    }

    #[test]
    fn watching_hands_back_to_the_clock_before_even_after_a_panic() {
        let (outer, inner) = (Clock::default(), Clock::default());
        assert!(running().is_null());
        {
            let _a = Watching::start(&outer);
            assert_eq!(running(), std::ptr::from_ref(&outer));
            {
                let _b = Watching::start(&inner);
                assert_eq!(running(), std::ptr::from_ref(&inner));
            }
            assert_eq!(running(), std::ptr::from_ref(&outer));
            let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _c = Watching::start(&inner);
                panic!("game code's host broke");
            }));
            assert!(caught.is_err());
            assert_eq!(running(), std::ptr::from_ref(&outer));
        }
        assert!(running().is_null());
    }

    #[test]
    fn code_run_with_no_clock_watching_runs_untouched() {
        // A Luau with the interrupt set, its code run outside any limit:
        // the interrupt finds no clock and lets it finish.
        let lua = mlua::Lua::new();
        watch(&lua).unwrap();
        let sum: i64 = lua
            .load("local s = 0 for i = 1, 100000 do s = s + i end return s")
            .eval()
            .unwrap();
        assert_eq!(sum, 5_000_050_000);
    }
}
