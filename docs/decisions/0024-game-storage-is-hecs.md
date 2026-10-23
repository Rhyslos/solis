# 0024. The game's fast storage is `hecs`

- **Status:** Accepted
- **Date:** 2026-10-23

## Problem

[Decision 0019](0019-scene-document-now-game-storage-later.md) keeps the editor on an area document of plain values, and leaves the running game's fast storage to the start of Phase 3: built from the document when Play is pressed, thrown away on Stop, and chosen with a benchmark at the scale target (DESIGN.md §2) between `hecs` and a storage of Solis's own.

## Options

1. **`hecs`** (MIT or Apache-2.0): a small, stable entity-component library. It stores entities with the same set of components together, so going through them is quick.
2. **Solis's own storage** in safe Rust: one packed list per component type, with an index to find any entity's slot at once. About 200 lines to write and look after.

Both were built and timed (`tools/storage-bench`, outside the workspace so `hecs` stayed out of the build until chosen). Each unit has seven components, among them a transform, a velocity, health, a team, a parent, a world matrix, and a component made while the program runs (fields by name, as Lua's will be). Microseconds, median of 301 runs, on the development machine:

| What a tick does | 2,000 units: `hecs` | own | 20,000: `hecs` | own |
|---|---:|---:|---:|---:|
| Build the world (Play pressed) | 472 | 652 | 6,072 | 8,838 |
| Move every unit | 2.7 | 3.3 | 27 | 35 |
| Read three components of every unit | 1.3 | 2.4 | 13 | 24 |
| One unit asks another for its health, ×n | 42 | 29 | 441 | 306 |
| World matrices from parents | 27 | 25 | 400 | 386 |
| A run-time component's field, every unit | 31 | 30 | 404 | 514 |
| A tenth of the units die and are replaced | 38 | 47 | 785 | 873 |

A 60 fps frame is 16,700 µs. At the scale target every row is well under 1% of a frame with either storage; even at ten times the target nothing comes near.

## Decision

**`hecs`**, in a new crate `world` (layer 1), the only crate that names it.

- **Speed doesn't decide it:** both are far inside the budget. `hecs` is quicker at going through components (what most systems do) and building the world; Solis's own is quicker at looking up one entity.
- **Ground rule 5 does:** reuse before writing. `hecs` is small, has had a stable interface for years, and is tested by its many users; Solis's own would be 200 more lines to test and look after.
- **Components made at run time** (Lua, and Lust while its library is being swapped) cost the same either way: they're plain values by name inside one Rust component, `Values`. Neither storage can make a real column for a type that only exists at run time without `unsafe` code.
- **Determinism** (lockstep, Phase 8): `hecs` goes through entities in the same order every time for the same sequence of changes, which is what lockstep needs; Solis's own would too.
- **Projectiles** stay a packed pool of their own in Phase 5 (decision 0016), not entities.

The `world` crate keeps `hecs` behind Solis's own small interface (build from an area, step, read and write a component by entity ID), so swapping it later touches one crate.

## Consequences

- `hecs` uses `unsafe` code inside it, like most of Solis's libraries (wgpu, egui). Solis's own rule (no `unsafe` without a decision record) covers Solis's code, not its libraries.
- The benchmark stays in `tools/storage-bench` to run again if the scale target changes.
- Phase 3's play mode builds a `world::World` from the area when Play is pressed (M5 of the [Phase 3 plan](../plans/phase-3.md)).
