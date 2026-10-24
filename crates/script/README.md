# script

**Layer 1 · Engine**

Game code ([decision 0028](../../docs/decisions/0028-lust-runs-as-luau-while-editing.md)): a project's Lust, checked and turned into Lua by the Lust compiler (used as a library), and run in Luau while the game plays.

## Purpose

- **The engine described to Lust** (`describe.rs`, [decision 0016](../../docs/decisions/0016-how-lust-joins-solis.md)): `NATIVES` lists every native method Solis gives game code. `descriptions` turns it into the Lust files a project gets in `assets/solis/` (`world.lst`, `entity.lst`), so they always match the engine.
- **A project's code** (`code.rs`): `gather` reads every `.lst` file in the assets within limits (1 MB a file, 16 MB and 10,000 files in all, hidden folders skipped). `check` runs Lust's checker from `main.lst` and gives back every mistake as a `CodeProblem` with file, line and column. Both are meant for a worker thread.
- **Running components** (`runtime.rs`): a `Runtime` loads the Lua in Luau with a 64 MB memory limit and a 100 ms limit per call. It reads each component's fields into a `reflect` schema (starting values from a fresh component), makes and syncs running components from the area's values (`sync` sends only fields that changed, so values the game changed are kept), and runs ticks: entities in ID order, components in name order. The native methods reach the game through a `Host`, lent once per tick in a `LUST_NATIVES` table that exists only while game code runs. A component that fails stops alone, and its failure points at the Lust (`Failure`). `runtime.rs` keeps the runtime itself; `host.rs` the `Host` and the natives; `convert.rs` values between `reflect` and Luau.
- **New code while running** (`Runtime::swap`): the new chunk is loaded into the same Luau and each running Lust component gets its code in place (Lust's `swap`), keeping field values by name; what changed and what started over come back as `Swapped`. If the new code's start fails, the old keeps running.
- **Events** (`events.rs`): Lust's `send` and Rust components queue an `Event`; after each tick's updates the queue is delivered in order to each component on the entity in name order, events sent meanwhile joining the end, cut off after `MAX_DELIVERIES` (10,000) in a tick. A failure handling an event is reported and leaves the component running.
- **Rust components** (`rust_component.rs`): a game's `RustComponent`, made from a `RustKind` (schema and constructor), runs in the same ticks and order and sends and handles the same events through a `Context`. A panic in one is caught and stops only it.

## Must never

- Crash on any code, however damaged: a panic inside the compiler is caught and reported as a mistake (fuzz-tested).
- Read files beyond its limits, or follow links to folders (they could loop).
- Let game code reach files, programs or the network (Luau has none of these).

## Depends on

`reflect` and glam; the Lust compiler (`lust`, without its default features, pinned to a commit) and `mlua` with Luau.

## Status

Phase 4, M1: the description files, gathering and checking. M2: running Lust components. M3: events, and Rust components. M4: swapping new code into a running game.
