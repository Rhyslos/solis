# 0016. How Lust joins Solis, and joining earlier

- **Status:** Accepted (moves the Lust phase from Phase 7 to Phase 4; builds on decisions 0002 and 0003)
- **Date:** 2026-10-22

## Problem

Decision 0002 says games are written in Lust and the engine in Rust, with Lust compiling to the same kind of Rust component as the engine's own. Two things were open: **how** a game's Lust code and the engine actually connect, and **when**. The plan had Lust joining in Phase 7, after scale, simulation, and sound and animation, partly because Lust wasn't ready.

Since then, Lust has built the connection (Lust pull request #51, LUST.md §21 "Lust inside other programs"):

- **`native` methods:** a Lust file can describe something whose body lives in the program Lust is part of, such as `method moveBy(float dx) native;`. Lust checks every call against the description, so a typo or wrong type is caught before anything runs.
- **`lust build game.lst --library`** makes a Rust library from a game's Lust. Each class it passes becomes a Rust type with getters, setters and methods. The engine provides the native bodies by implementing one generated Rust interface (`game::Host`) and handing it over before the game starts.
- **Lust's mistakes come back as text,** with file and line, never a crash in the engine.
- A working example (`tests/as-library/` in the Lust repo) runs weapon logic in Lust, driven every frame by a small Rust program standing in for Solis, on Windows and Linux.

## Options

**How:**
1. **A special `fetch solis;` built into the language,** as first planned. Lust would need to know about Solis.
2. **Description files** (the project lead's idea): Solis puts plain Lust files describing the engine into each game project, with `native` methods. No special rules in the language, and the same trick serves Garry's Mod, Aventail or anything else.

**When:**
1. **Phase 7,** as planned.
2. **Right after Phase 2,** as soon as components exist.
3. **Right after Phase 3 (the editor).** Lust components need the inspector to show their fields, and play mode to change them while the game runs; both arrive in Phase 3.

## Decision

Description files, and the Lust phase moves to **Phase 4**, right after the editor.

- **The engine described to Lust:** Solis writes `solis/entity.lst`, `solis/world.lst`, `solis/input.lst` and so on into each game project, generated from the engine's own Rust code so they always match the engine version. They describe the engine, not the game, so they stay the same size however big the game gets. A game uses them with Lust's ordinary `fetch "solis/entity":Entity;`.
- **A game's Lust becomes a library** (`lust build --library`) the engine includes, and Solis implements the generated `Host` interface to answer the native calls.
- **Lust's mistakes go to the Problems panel,** with file and line.
- **The new order:** 4 Lust and Lua join, 5 Scale, 6 Simulation and play, 7 Sound and animation; Phases 0–3 and 8–11 keep their numbers.
- **Ready-made gameplay components** (character and camera controllers, in Phase 6) are written in Lust from the start, rather than in Rust and rewritten later. The Lust phase's last milestone becomes a first small game in Lust, which later phases keep growing.

**Still to build in Lust, during Phase 4** (LUST.md §23): vectors and rotations as real value types, typed events, leaner Rust for code that runs every frame, building a game's Lust separately and loading it without restarting (for the 2-second goal, DESIGN.md §3.1), and, with Phase 6, checker rules for deterministic netcode. The 20,000 projectiles of DESIGN.md §2 stay an engine system in Rust; Lust decides what happens when they hit.

## Consequences

- Game logic can be written in Lust about three phases sooner, and Phases 5 to 7 are tested with real Lust code rather than Rust stand-ins.
- If Lust's remaining pieces are late, Phase 5 (Scale) can go first; it doesn't need Lust.
- "`fetch solis;`" in older documents now means the description files.
