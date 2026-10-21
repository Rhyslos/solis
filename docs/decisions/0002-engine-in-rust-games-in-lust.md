# 0002. The engine is Rust; games are Lust

- **Status:** Accepted (replaces "the engine in Lust" from the first design)
- **Date:** 2026-10-21

## Problem

The first design had the engine written in Lust on a thin Rust foundation. Then Lust's plan changed to compile to Rust. If Lust turns into Rust anyway, is there a point writing the engine in Lust, or should the engine be Rust?

## Options

1. **The engine in Lust,** on a thin Rust foundation (the first design).
2. **The engine in Rust, games in Lust.** The split every successful engine uses: Godot is C++ with GDScript for games, Unity is C++ with C# for games.

## Decision

Option 2.

- **An engine needs the control Lust deliberately hides:** memory layouts, GPU buffer formats, threads and fine-grained optimisation. Lust-written Rust would at best match hand-written Rust.
- **The libraries are Rust-shaped** (wgpu, Rapier, ECS crates). As the engine's language, Lust would have to wrap them all; as the game language, it only sees a small, friendly surface.
- **Two young projects don't block each other.** The engine can start now, and Lust only has to be ready when games are written (ROADMAP.md, Phase 7).
- **It helps iteration speed** (DESIGN.md §3.1): the engine comes prebuilt, and only a game's own code is compiled.
- **Rust developers can contribute.**

Lust stays what every Solis game is made of, and where Solis is different: components whose `has` fields appear in the inspector, typed events instead of signals, mistakes caught before Play, and the checker enforcing the rules deterministic netcode needs.

**How the two meet:** a component is a plain Rust struct that the engine can describe to the inspector. Built-in components are written that way in Rust. A Lust `object … like Component` compiles to exactly the same kind of struct, so the engine can't tell them apart. Until Lust is ready, the engine's own gameplay components are written in Rust against that same surface; afterwards the ready-made gameplay components (Health, character and camera controllers) move to Lust, as examples for game makers.

## Consequences

- Solis can start before Lust is finished.
- The game API (`fetch solis;`) is designed from the start for Lust to read well, and tested from Rust until Lust arrives.
- Games could also be written in Rust, for people who prefer it. Lust is the one the docs teach.
- The engine is no longer a test of Lust as a language for big systems. Games are its test instead.
