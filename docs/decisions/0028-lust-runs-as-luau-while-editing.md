# 0028. Lust runs as Luau while editing, and as Rust in built games

- **Status:** Accepted (settles DESIGN.md §3.1, "Iteration speed"; builds on decision 0016)
- **Date:** 2026-10-24

## Problem

Phase 4's goal: a change to a Lust component is running in play mode within **2 seconds** (DESIGN.md §3.1). Lust can become Lua or Rust (decision 0016), and Solis has to choose which one the editor runs.

## Options

1. **Luau while editing, Rust in built games.**
   - Turning Lust into Lua takes a fraction of a second. The running game swaps the new code into its components and keeps their values (Lust's `swap`, LUST.md §21).
   - The Luau runtime is needed anyway, for Lua components (decision 0003).
   - Nobody needs Rust installed to make a game.
   - The risk: the shipped game runs differently from the editor. DESIGN.md §3.1 called this "probably ruled out" for that reason.
2. **Rust always.** A change builds the game's Lust to Rust, and the editor loads the new library.
   - It's exactly what ships.
   - But it takes seconds per change at best, needs Rust and Microsoft's build tools (several GB) on every game maker's computer, and swapping compiled libraries into a running program on Windows is fragile.
3. **Both,** with Luau by default and a setting for Rust. The most to build and keep working.

## Decision

Option 1, chosen by the project lead (2026-10-24).

Since §3.1 was written, Lust has been built to make the two behave the same:
- it keeps `vec3` and `rotation` numbers as f32 in both;
- `float` is f64 in both;
- a test runs 200 steps of mixed maths both ways and gets the same bits.

What's left is guarded like this:
- **Comparison tests:** Solis's tests run the same Lust both ways (Luau, and Rust built with `lust::to_rust_library`) and compare the results, for the samples and the ready-made components.
- **Maths functions** (sine, square root and the like) come from different libraries in Lua and Rust, and their last digit can differ. This only matters for lockstep multiplayer (Phase 8). If it shows up, Lust provides its own.
- **Playing the built game** from the editor, which needs Rust installed, comes with building games (Phase 10).

## Consequences

- Lust code runs a few times slower in the editor than in the shipped game. Timings of heavy game code are taken from the built version.
- Solis depends on `mlua` with Luau, and on the Lust compiler as a library (`lust`, without its default features), pinned to a commit or tag.
- The Lua runtime is sandboxed: Luau has no file or network access. Memory is limited. Code that runs too long is stopped with a message instead of freezing the editor.
- DESIGN.md §3.1 is settled.
