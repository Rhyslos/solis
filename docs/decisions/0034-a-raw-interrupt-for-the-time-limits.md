# 0034. A raw interrupt for the time limits (Solis's one `unsafe` module)

- **Status:** Accepted
- **Date:** 2026-10-25

## Problem

Game code runs under time limits, so a loop that never ends can't freeze Solis (decision 0028): Luau calls an "interrupt" at every function call and every turn of a loop, and the interrupt stops code whose time is up. Solis set it through mlua's `set_interrupt`, which wraps every interrupt in its general error handling: it clones a reference and prepares room for an error, every time. Profiling the scale levels' ships (Phase 5, M7) showed that this alone was **19%** of game code's time, and game code is most of the stress level's frame.

Luau lets a program set its interrupt directly: a plain function. Doing that takes `unsafe` code, which Solis's rules allow only with a decision record (CONTRIBUTING.md, rule 7).

## Options

1. **Keep mlua's interrupt.** Safe Rust, a fifth of game code's time.
2. **Set Luau's interrupt directly**, in one small module: a plain function that does nothing unless a clock is running on this thread, and raises the clock's error when it says so.
3. **No interrupt** (no time limits). A loop that never ends would freeze the editor: not an option.

## Decision

Option 2: `crates/script/src/interrupt.rs`, the only module where Solis's own code may be `unsafe` (`#[allow(unsafe_code)]` on that module alone; the workspace still denies it everywhere else).

- The running clock is found through a thread-local pointer, set only while a call into game code runs on that thread (`Watching`, which restores the one before when dropped, even if the call panics), so the interrupt never reads a clock that's gone.
- Garbage collection's interrupts are ignored (Luau doesn't allow errors there).
- The error is raised the way Luau expects from an interrupt: room is made on the stack, the message is copied onto it, and `lua_error` is called with nothing of Rust's left to drop.
- Every `unsafe` block says why it's sound, next to it.

## Consequences

- Game code's instructions in the scale levels' profile fell by 14% (2.48 to 2.13 billion), the interrupt from 19% to 5% of them.
- The time limits behave as before: the same messages, the same tests (a loop stopped by its call's limit or the tick's).
- A Luau or mlua update that changes how interrupts work must be checked against this module; its tests catch a crash.
- Any other `unsafe` code still needs its own decision.
