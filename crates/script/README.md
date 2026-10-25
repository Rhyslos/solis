# script

**Layer 1 · Engine**

Game code ([decision 0028](../../docs/decisions/0028-lust-runs-as-luau-while-editing.md)): a project's Lust, checked and turned into Lua by the Lust compiler (used as a library), and run in Luau while the game plays.

## Purpose

- **The engine described to Lust** (`describe.rs`, [decision 0016](../../docs/decisions/0016-how-lust-joins-solis.md)): `NATIVES` lists every native method Solis gives game code. `descriptions` turns it into the Lust files a project gets in `assets/solis/` (`world.lst`, `entity.lst`), so they always match the engine.
- **A project's code** (`code.rs`): `gather` reads every `.lst` file in the assets within limits (1 MB a file, 16 MB and 10,000 files in all, hidden folders skipped). `check` runs Lust's checker from `main.lst` and gives back every mistake as a `CodeProblem` with file, line and column. Both are meant for a worker thread.
- **Running components** in Luau (Phase 4, M2 onward; see `docs/plans/phase-4.md`).

## Must never

- Crash on any code, however damaged: a panic inside the compiler is caught and reported as a mistake (fuzz-tested).
- Read files beyond its limits, or follow links to folders (they could loop).
- Let game code reach files, programs or the network (Luau has none of these).

## Depends on

`reflect`; the Lust compiler (`lust`, without its default features, pinned to a commit) and `mlua` with Luau.

## Status

Phase 4, M1: the description files, gathering and checking.
