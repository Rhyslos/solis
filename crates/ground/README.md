# ground

**Layer 1 · Engine**

The ground (Tools 1b; decisions [0111](../../docs/decisions/0111-tools-1-brushes-and-ground.md), [0116](../../docs/decisions/0116-the-ground-voxels-regions-and-what-games-may-change.md) and [0117](../../docs/decisions/0117-open-worlds-ten-km-a-recipe-and-far-caves.md)): one per level, with caves and overhangs, kept as voxels in chunks. Its own crate by [decision 0119](../../docs/decisions/0119-the-ground-s-own-crate.md): it fits nothing else (`tiles` is 2D grids, `mesh` is brushes' polygons). Turned from the study's prototypes (`tools/ground-study`) into Solis's own code.

## Purpose

- **Voxels and chunks** (`voxel.rs`, `store.rs`): a voxel is two bytes, its distance from the surface in 1/64ths of a voxel (held to ±2 voxels) and its ground kind. Chunks are 32 × 32 × 32 one-metre voxels behind the small `Store` interface (fixed chunks now; sparse chunks can replace them, decision 0116). A chunk the same all through is kept as one voxel; any other keeps its voxels behind an `Arc`, so the editor's ground, its undo history and the running game's copy share every unchanged chunk. A store can say a chunk **isn't loaded** (decision 0117, for streaming), and every question about it then answers "unknown", never "air".
- **The ground** (`ground.rs`): its chunks, the voxel at any place (past the edges the edge voxels carry on, so the surface just stops), whether a chunk can make any triangles at all, and the samples a chunk is meshed from: its voxels and one more all round, copied, so a worker can mesh them alone. The height is a setting (128 m now), never a number baked into indexing.
- **The Smooth look** (`smooth.rs`): surface nets. Each crossed edge is worked out once; each point faces the way the voxels' distance grows (so seams shade smoothly); places are chunk-local beside a chunk origin (decision 0117: a far origin later) and held to 1/1024 m, so a point two chunks share lands on exactly the same place. The same samples give the same bytes every time.
- **Questions** (`query.rs`): the height at a place, the kind at a place, and a ray against the ground (stepped, then closed in on).
- **A starting shape** (`start.rs`): flat at a height, or gently rolling (the study's noise, ±10 m), made chunk by chunk with sky and deep rock as one voxel each.
- **Noise** (`noise.rs`): value noise from integer hashing and plain `f32` steps, the same on every PC. It lives here until a second crate needs it on the processor (materials' and lighting's noise runs in shaders); it needs only `glam`, so moving it is a file move.
- **The settings file** (`settings.rs`): `farm.ground.lst`, a Lust data file (version 1): the look (Smooth), the size (whole 256 m regions, up to 2 × 2 km), the height (whole 32 m chunks, up to 512 m), the palette of ground kinds (names and colours, up to 256; painting is G4) and the recipe slot, empty until Open worlds. Read as a hostile file (1 MB); fuzzed (`fuzz/fuzz_targets/ground_settings.rs`).
- **Region files** (`region.rs`): the voxels, one binary file per 256 × 256 m region (`0_1.region`), in the folder beside the settings file (`farm.ground/`): a table of chunks first (so one chunk reads alone), room for coarser levels 1 to 3, the height written in, then each chunk's runs of voxels deflated (`flate2`). Read as a hostile file (160 MB); fuzzed (`fuzz/fuzz_targets/ground_region.rs`).

## Must never

- Know about the graphics card, the editor, the window or the physics library (rule 2; `tools/layer-check`).
- Crash, hang or read past a limit on a damaged settings or region file.
- Answer "air" for a chunk that isn't loaded.
- Give different voxels or meshes for the same input on two PCs.

## Depends on

`glam` (places), `flate2` (deflating region files), `level` (the Lust data-file reader), `reflect` (the format's version).

## Status

Tools 1b, G1: the Smooth look, a flat or gently rolling start, its files and questions. Sculpting (G2), Blocky and Sharp (G3), painting (G4) and game code's calls (G5) come next.
