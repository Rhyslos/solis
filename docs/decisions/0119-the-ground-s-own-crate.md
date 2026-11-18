# 0119: The ground's own crate

- **Status:** Accepted (proposed by the plan for Tools 1b, `docs/reports/after-the-rethink/13-tools-1b.md`, under rule 1; built in G1)
- **Date:** 2026-11-18

## Problem

The ground (decisions 0111, 0116 and 0117) needs a home for its voxel chunks, its meshers, its settings and region files and the questions games ask of it. Rule 1 says new code goes in the crate whose job it is, and a new crate is proposed first.

## Options

- **`tiles`:** also chunks and files, but its job is 2D grids of terrain codes; the ground would double what it holds and drag 3D into a 2D crate.
- **`mesh`** (layer 0): its job is brushes' n-sided polygons edited point by point, not voxel fields.
- **A new `ground` crate in layer 1**, beside `tiles` (recommended by the plan).

## Decision

A new crate, **`ground`**, in layer 1 (the engine). It holds the storage behind its small `Store` interface, the looks' meshers, the settings file (Lust) and the binary region files, the starting shapes and the noise they use, and the questions (height, kind, a ray). It knows nothing of the graphics card, the editor or the physics library: meshes leave it as plain lists, files as bytes and text, and the app, `render`, `physics` and the editor do the rest.

## Consequences

- `tools/layer-check` and ARCHITECTURE.md's layers table list it; `flate2` is allowed in it (`docs/dependencies.md`).
- The noise lives in `ground::noise` until a second crate needs it on the processor (materials' and lighting's noise runs in shaders); it needs only `glam`, so moving it to a layer-0 crate is a file move.
- `level` holds the `Ground` component (the settings file's asset ID); `level` stays below `ground` and never depends on it.
