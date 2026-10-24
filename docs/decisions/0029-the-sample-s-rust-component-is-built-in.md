# 0029. The turret sample's Rust component is built into Solis until Phase 10

- **Status:** Accepted (an exception to "game-specific systems stay out of the engine", until Phase 10)
- **Date:** 2026-10-24

## Problem

Phase 4's gate asks for one entity carrying a Rust, a Lust and a Lua component that talk through events, tried by hand. Solis runs Lust and Lua while editing (decision 0028), but can't build a game's own Rust until Phase 10. So no game's Rust component can reach the project lead's copy of Solis.

## Options

1. **Build the sample's Rust component into Solis,** offered only in the sample project.
2. **Test it automatically only,** and try that line of the gate by hand in Phase 10.
3. **Build a game's Rust now,** bringing part of Phase 10 forward. It needs Rust installed, and loading compiled code into a running editor is fragile on Windows.

## Decision

Option 1, chosen by the project lead (2026-10-24).

- The turret sample's Rust component, Armour, lives in the sample's own `samples/turret/assets/armour.rs`. Solis compiles that very file in (`crates/app/src/sample_game.rs`), so what the project shows is what runs.
- It's offered only in a project whose assets have `armour.rs`. Changing that file does nothing yet, and the Console says so.
- **Help > Make the sample game…** writes the sample as a new project.

## Consequences

- One small piece of a game is inside the engine until Phase 10. It's removed when Solis builds a game's own Rust, and the sample then builds `armour.rs` like any game would.
- A comparison test that runs the sample's Lust both in Luau and built as Rust (decision 0028) also waits for Phase 10: the Rust that Lust generates uses `unsafe` for the local time, which Solis's rules don't allow without review. Lust's own checks already compare 2,000 components over 600 ticks both ways, bit for bit.
