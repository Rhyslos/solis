# 13 · Tools 1b: world sculpting (G1 to G5)

ROADMAP.md: **G0 the study** (done, `docs/reports/ground-study.md`); **G1 the ground, Smooth** (one per level, chunks made on workers, solid, saved); **G2 sculpting** (left-drag adds, Ctrl digs, Shift smooths, [ ] size, Shift + [ ] strength; Flatten); **G3 Blocky and Sharp** (the look a setting per ground); **G4 painting** (materials painted with the same brush); **G5 speed, game code, the manual**. **Gate:** a level of brushes on a sculpted, painted ground with a cave, in each look, played, saved and reopened, at the scale check's targets. Decisions 0111 (the tools and keys), 0116 (storage, looks, files, size 1 × 1 km, what games may change, a palette of 256 kinds). The plan is `docs/plans/tools-1.md`; mock-ups 5 and 6 are approved (`docs/design/tools-1/`).

This phase is unusually well prepared: the study measured every competing approach in working prototypes (`tools/ground-study`: `store.rs`, `mesh/{marching,dual,blocky,cases,normals,qef}.rs`, `brush.rs`, `collision.rs`, `file.rs`, `noise.rs`, plus the open-worlds study's `lod/`, `stream/`, `far.rs`) and the lead answered its questions. So this report concentrates on **how to turn the prototypes into Solis's crates**, the open technical choices the study left (blending, Blocky's hairlines and step height, undo memory, determinism of edits), and what the Open worlds phase needs Tools 1b to get right now.

## Settled (decision 0116), in short

32³ chunks of 1 m voxels, 2 bytes each (distance in 1/64 voxel clamped to ±2 voxels; ground kind), 128 m high, uniform chunks kept as one voxel, storage behind one small interface; Smooth = surface nets, Sharp = sharpened surface nets (hard edges over 40°), Blocky = merged cube faces; chunks meshed on workers; colliders built when a stroke ends, off the frame; a text settings file plus binary 256 × 256 m region files (runs, deflated), versioned, fuzzed; edges: the ground just stops; game code may change everything the editor can, with a limit per tick, changes kept in saves and put back on Stop; a palette of up to 256 kinds (name, colour, or a picture laid on from three sides). Units at scale follow the ground by reading voxels (0.25 ms a tick for 2,000), not as physics characters (43 ms).

## Where the code goes

| Crate | What it gains | Notes |
| --- | --- | --- |
| **`ground` (new, layer 1)** | storage (`Store` trait; fixed chunks), brushes (add, dig, smooth, flatten, paint), the three meshers, region file reading/writing, the settings file, queries (height at, ray, kind at), the dirty set | Promoted from `tools/ground-study` (rule 1: a new crate is proposed first; it fits nothing existing: `tiles` is 2D, `mesh` is brushes' n-gons). Knows nothing of the GPU, the editor or Rapier. Layer 1 like `tiles`. |
| `area` (level) | a built-in `Ground` component (settings file's asset ID, look, solid) | one per level (0111) |
| `asset_db` | ground settings (`<name>.ground.lst`, decision 0081) and its region folder as one asset | region files are binary: the asset database must treat the folder as the asset's parts, not separate assets |
| `physics` | **build a triangle collider's shape on a worker** and swap it in on the game's thread (the study: 1.2 of 1.24 ms is shape building) | already planned by both studies; `Shape::Mesh` exists from B1 |
| `render` | ground chunk meshes (a vertex with position, normal and kind data; the study suggests a smaller vertex later), drawn with culling per chunk, triplanar pictures per kind | `upload_model` was used in the study; a dedicated chunk buffer pool avoids per-chunk allocations |
| `world` | the running game's copy of the ground (changed chunks only, copy-on-write over the editor's), game code's edits landed at the tick's end, the checksum of changed chunks, saves | as tile maps do (`MapState`) |
| `script` | natives on a `Ground` object (or a generated per-ground file, rule 4) | see G5 |
| `editor` | HOME TOOLS · Ground tab, the brush circle, the Ground shelf tab with the palette, strokes as undo steps | mock-up 5 |
| `app` | jobs for meshing and colliders on the existing `rayon` pool | one worker per core, one core left for the window |

## G1. The ground, Smooth

- **Storage:** fixed chunks as the study's `store.rs`. Keep chunk data in `Arc` so the editor's document, the undo history and the running game share unchanged chunks (as tile maps share layers).
- **Meshing on workers:** the frame copies the touched chunks' samples (plus a one-voxel border from neighbours: the mesher needs it) and hands them to a job; meshes come back by message and are uploaded then (rule 10). A **generation counter** per chunk discards stale results (a newer edit landed while the old mesh was cooking).
- **Surface nets detail (study's note):** compute each crossed edge's intersection once, not once per cube (the busiest chunk was 4.5 ms).
- **Normals:** from the voxel gradient (smooth) rather than from triangles, so chunk seams shade continuously.
- **The file:** region files exactly as the study's `file.rs` but **already shaped for Open worlds** (decision 0117 says Tools 1b's format must allow taller grounds and recipes later by a version raise): put a **chunk table** at the start of each region file now (the open-worlds study added it later; adding it now saves a format version), reserve space for levels 1 to 3 per chunk, and make the settings file able to say "made from a recipe" (empty for now).
- **Solid:** colliders per surface chunk via the physics worker path; a character entering a chunk whose collider is still cooking is held at the edge (the open-worlds study's rule) rather than falling.
- **Tests:** a chunk meshed twice is byte-identical; neighbours' seams share vertices exactly (no cracks: count open edges, as the study's crack tests do); fuzz the region file and settings reader (rule 9); fixtures for format v1 (rule 23). **Scale case:** open the 1 km² target ground; frame times while chunks stream in.

**Size:** large.

## G2. Sculpting

- **The brush:** a sphere (or capsule along the drag) with radius, strength and falloff; Add raises the field (min of distances), Dig lowers it, Smooth averages a 3³ neighbourhood, Flatten pulls toward a plane picked at the stroke's start. **Use integer (fixed-point) arithmetic on the 1/64 distances** for every brush, so the same strokes give the same voxels on every machine: game code's edits (G5) are game state and must be deterministic, and replays of edits must match.
- **Strokes:** dabs spaced along the mouse path (a fraction of the radius) so speed doesn't change the result; only touched chunks re-mesh (on workers); colliders rebuilt when the stroke ends (one undo step, 0111).
- **Undo memory:** a stroke's undo stores the **before** state of touched chunks. A 12 m dig touches 8 to 27 chunks × 64 KB = up to 1.7 MB raw. Keep them run-length encoded (the study's runs shrink sky and rock to almost nothing) and cap the history by bytes (e.g. 256 MB), dropping the oldest steps with a note. Shared `Arc` chunks make redo free.
- **Keys** (answered, 0111): left-drag adds, Ctrl digs, Shift smooths, [ ] size, Shift + [ ] strength, Flatten a tool. Ctrl conflicts with "Ctrl flips snapping" (decision 0025) only for gizmos, not while sculpting; check the keys' owner (`editor::key_owner`).
- **Picking under the pointer:** a ray against the voxels (march the distance field, then refine), not against the GPU's pick image, so the brush circle follows caves and overhangs exactly.
- **Tests:** a stroke is one undo step; undo restores voxels bit for bit; the same stroke at two mouse speeds gives the same voxels. **Scale case:** sculpting at the 1 km² target within the editor's frame target (the study: 2.4 to 10 ms of frame work per dab before moving meshing to workers; under 1.5 ms after).

**Size:** medium.

## G3. Blocky and Sharp

- **Sharp:** sharpened surface nets (points pulled four times toward the planes; no equations), hard edges where faces meet over 40° by splitting vertices along creases. Switching Smooth ↔ Sharp only moves points and normals.
- **Blocky:** merged cube faces (greedy meshing) with the cube lines drawn by the material (from world position), four times fewer triangles. Two problems the study left:
  - **Hairlines** where a merged rectangle's side meets another's middle (a T-junction): rasterisation can leave a pixel gap. Fixes: (a) split rectangles where neighbours' edges are (loses some merging); (b) draw a cheap back-face or "skirt" fill under the surface so a gap shows ground colour instead of sky; (c) snap vertices in the shader to exact grid values (removes the arithmetic drift that causes most of it). **Recommended:** (c) first, with (b) as backup, measured with the study's grazing-ray test (2 in 448 slipped through).
  - **Step height:** a character that climbs 35 cm can't climb one-metre blocks (411 of 2,000 stopped). **Recommended:** a per-ground "step height" setting defaulting to 1.05 m on Blocky grounds (characters auto-step blocks, the convention in block games); alternative: smooth colliders under a blocky look (the eye and the feet disagree).
- **Switching looks** re-meshes every chunk (≈1.2 s of one worker at 1 km²): keep the old look drawn until the new chunk arrives, nearest to the camera first.

**Size:** medium.

## G4. Painting

**Ways to blend kinds where they meet** (one kind per voxel, decision 0116):
1. **Corner kinds with barycentric blending:** each triangle carries its three corners' kinds (from the nearest solid voxels); the shader blends the three kinds' pictures by barycentric weights, sharpened by a height map per picture so grass gives way to rock along the rock's cracks ("height blending"). Soft as a voxel is wide; standard in voxel terrain.
2. **Splat weights per vertex** (four kinds per vertex): softer, but limited to four per chunk or a lookup.
3. **Hard per-triangle kinds:** crisp, faceted: right for **Blocky** (each cube face one kind).

**Recommendation:** 1 for Smooth and Sharp, 3 for Blocky; pictures in a **texture array** (one layer per kind, up to 256), sampled **triplanar** so steep and overhanging ground isn't stretched (0116 asks for it), with a colour-only kind needing no picture. When Shared 7 arrives, a kind names a material whose surface function receives the same triplanar inputs.

- **The palette:** the Ground shelf tab (mock-up 5): kinds as swatches, add, rename, recolour, pick a picture; painting with the same brush (paint mode replaces the kind of solid voxels near the surface within the radius without changing distances).
- **Tests:** painting never changes the surface (distances unchanged); a kind's picture swap redraws without re-meshing (kinds index the array). **Scale case:** painting at the target.

**Size:** medium.

## G5. Speed, game code, the manual

- **Game code** (decision 0116: everything the editor can, a limit per tick; rule 25 checklist). A ground is a component, so by rule 4 a generated file per ground gives its kinds as a state (`FarmGroundKind.Rock`) and a class with: `height(place)`, `kind(place)`, `castRay(from, dir, reach)` → `GroundHit` (one class, rule 6), `dig(place, radius)`, `add(place, radius)`, `smooth(place, radius)`, `paint(place, radius, kind)`, and the slow ones `setLook(GroundLook.Blocky)` and `setSize(…)` (documented as slow). Reads see the ground as the tick started; edits land at the tick's end in order (entity ID, call order), clamped to N dabs per tick (e.g. 64) with a loud note past it. Changed chunks go in save games (a save format version raise, rule 23) and the checksum; Stop puts the editor's ground back (`world` holds a copy-on-write ground).
- **Units following the ground:** the study's 0.25 ms "read the voxels under each unit" is a native per unit per tick; expose `height(place)` cheaply and let the kit's movers use it (no physics). A batch call for many units (`heights(vec3 array)`) avoids 2,000 crossings into Luau.
- **Scale cases** (0116): the 1 km² target and 4 km² stress grounds; sculpting them; 2,000 units walking over them; brushes on top (the gate).
- **The manual:** ground, looks, sculpting, painting, game code in three languages.

**Size:** medium.

## What Open worlds needs Tools 1b to get right now

1. **Region files with a chunk table** and room for levels 1 to 3 (saves a format version and an upgrader later).
2. **The `Store` interface** that can say "this chunk isn't loaded" (streaming will make chunks absent), and queries that return "unknown" rather than "air" for absent chunks.
3. **A recipe hook** in the settings file (empty now): decision 0117 makes the starting ground a recipe.
4. **Height as a parameter** (128 m now; 512 m later by stacking chunks), not a constant baked into indexing.
5. **The noise module** (the study's `noise.rs`) placed where Shared 3 and materials can reuse it (see `05-shared-3.md`).
6. **Positions near the camera** in render kept relative-friendly (Open worlds' floating origin; at 1 km it doesn't matter, but avoid baking world positions into vertex data as large floats: store chunk-local positions and a chunk origin).

## Risks and questions for the project lead

1. **Blocky step height:** **recommended** 1.05 m auto-step on Blocky grounds by default; alternative: smooth colliders under the blocky look.
2. **Undo memory cap:** **recommended** 256 MB of ground history, oldest dropped with a note; alternative: unlimited (risky on 8 GB machines).
3. **Region file format now shaped for Open worlds** (chunk table, levels reserved): **recommended** yes.
4. **Game code edit limit per tick:** **recommended** 64 dabs per ground per tick, more refused loudly.
5. **Blending painted kinds:** **recommended** soft height-blended corners on Smooth and Sharp, hard per-face on Blocky.

## Crates

- `flate2` / `miniz_oxide` (MIT or Apache-2.0; `miniz_oxide` also Zlib) are **already in Cargo.lock**: use for region deflate (register in `docs/dependencies.md` if used directly, rule 5).
- `fast-surface-nets` and `block-mesh` (MIT or Apache-2.0, same author) exist but pin an old `glam` (0.20) and are small: **write our own** (the study already did, decision 0051).

## Size

Phase: large (G1 large, G2 to G5 medium).

## Sources

- `docs/reports/ground-study.md`, `docs/reports/open-worlds-study.md`, decisions 0111, 0116, 0117, `docs/plans/tools-1.md`, `tools/ground-study/`.
- Surface nets: [Mikola Lysenko, Smooth voxel terrain (part 2)](https://0fps.net/2012/07/12/smooth-voxel-terrain-part-2/); greedy meshing: [Lysenko, Meshing in a Minecraft game](https://0fps.net/2012/06/30/meshing-in-a-minecraft-game/).
- Triplanar and blending: [Ben Golus, Normal mapping for a triplanar shader](https://bgolus.medium.com/normal-mapping-for-a-triplanar-shader-10bf39dca05a); height-based texture blending: [Andrey Mishkinis, Advanced terrain texture splatting](https://www.gamedeveloper.com/programming/advanced-terrain-texture-splatting).
- Existing crates: [fast-surface-nets Cargo.toml](https://docs.rs/crate/fast-surface-nets/0.2.0/source/Cargo.toml), [block-mesh](https://docs.rs/crate/block-mesh/latest/source/README.md).
