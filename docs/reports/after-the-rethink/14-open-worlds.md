# 14 · Open worlds (W1 to W5), revised from decision 0117

ROADMAP.md lists W1 to W5 as the open-worlds study proposed them, and says "the phase's first builder revises them from decision 0117 (W1 streams from the recipe and sculpted regions; W2 draws voxels to 4 km and the horizon option; W4 makes the recipe's tools)". Decision 0117: **10 × 10 km by default** (20 km by setting, 41 km tested as stress); **voxels to 4 km by default, voxels to the horizon an option made as well as the default** (own scale cases, no seams, cracks or popping, streaming that keeps up, and an honest message when a PC can't); **the starting ground is a recipe**, only sculpted regions stored, the rest made on workers ahead of the camera, coarse levels first, a throw-away disk cache; **1 GB** for a big world's ground; **512 m high or more**. Gate (proposed): a 20 × 20 km ground from noise, sculpted and painted kilometres apart, flown across without holes or stalls in each look, walked on and crossed by 2,000 units, saved and reopened. (With 0117 the default is 10 km; the gate should say 10 km at full targets and 20 km working.)

## What the study measured that the revision must respect

- A chunk costs the same at every level (≈1.1–1.7 ms Smooth, 1.8–2.2 Sharp, 0.5–0.7 Blocky on one core, ≈2,200–2,500 triangles); rings of 8 fixed-size chunks beat an octree (half the triangles, predictable); the transition band closes seams in Smooth and Sharp with no extra triangles; walls between sizes close Blocky; the far field (a height map made from the voxels) draws like coarse voxels but is one small file.
- **Making a chunk from formulas costs ≈2.5 ms vs 0.06 ms reading it: forty times more.** Making the first view from formulas took 2 to 2.6 s (vs under 0.1 s reading) and kept one worker 92 % busy at 200 m/s with frames late. This is decision 0117's main cost.
- Streaming by priority with a ring read ahead never missed a chunk at 50 to 800 m/s; the budget must hold a view (48 MB) plus units' columns (≈226 MB for 2,000 units spread over 20 km).
- Colliders: 1.2 of 1.24 ms is shape building, movable to a worker; a character held at the edge of ground not yet solid never fell.
- Precision: 2 mm at 20 km, 8 mm at 82 km.

## The recipe changes the phase most: findings

1. **A recipe must be local.** To make any chunk on demand, the ground at a place must be a pure function of that place and the recipe's settings. That rules out **simulated erosion** (water carving valleys is a global process). Options: erosion-like looks from local noise (fractal noise using its own derivatives so slopes erode, ridged multifractals, domain warping: Quilez's and Musgrave's methods), or a **baked** layer (an imported or pre-eroded height map) that the recipe reads as one ingredient. **Recommended:** both: local noise by default, and "height map picture" as an ingredient (0117 already says a picture import can be "one more recipe").
2. **A recipe can never change behind a world's back.** The study warned: "the formulas could never change again without changing every world". Every noise function and recipe step gets a **version**, the world's settings record it, and Solis keeps old versions' code forever (as rule 23 keeps old file readers): a recipe upgrade is opt-in per world, with a backup. Tests keep golden chunks per recipe version.
3. **Bit-exact recipes for game state, fast approximations for drawing.** Units and physics read level-0 voxels: those must be identical on every PC (determinism, decisions 0033, 0036). Far levels and the far field are **drawing only**. So:
   - level-0 (and anything game code or physics reads) is made on the CPU with integer-hashed noise and careful float order (Windows only, decision 0012, but CPU models still differ in fused multiply-add use: compile the noise without FMA contraction or use fixed point);
   - far levels and the far field may be made on the **GPU** (compute: noise evaluated per sample is extremely fast there), where small differences only change pixels. The GL path has no compute: it falls back to the CPU.
4. **Make coarse first, fine later, and remember.** Order of work: the far field for the whole view (cheap: a height per 32 m), then the coarsest voxel ring, then finer rings toward the camera; each made chunk goes into a **disk cache** (in the project's cache folder, throw-away, never in Git) so flying back is a read (0.06 ms), and a **memory cache** under the 1 GB budget.
5. **Sculpted regions override the recipe.** Store sculpted chunks only (plus their levels 1 to 3 averaged), in region files as Tools 1b writes them; a chunk is "sculpted if present in a region file, else the recipe". Levels above a sculpt are re-averaged from below near it and made from the recipe elsewhere; the transition band's "one rule whichever chunk asks" keeps seams closed even where the two ways meet (it only adds a little popping, measured).
6. **Units don't need full chunks on unsculpted ground.** 2,000 units spread out need their columns; making them from the recipe costs ≈2.5 ms each (≈0.8 s of CPU a second at a walk for 307 new columns a second). Instead, give the recipe a **surface query** (the top surface height at a place, evaluated directly: microseconds) and use full voxels only where a column is sculpted or has caves near the surface. Units walking in caves are the exception (they need the voxels).
7. **512 m high:** chunks stack in columns (16 chunks of 32 m at level 0); only chunks crossing the surface or holding caves are kept; uniform chunks are one voxel. At level 4 a chunk (512 m) holds the whole height. Rings are of columns; within a column, chunks far above or below the camera's view can be dropped at fine levels.

## Revised milestones

### W1. Recipes, regions and streaming
- The **recipe** (a versioned list of steps: fractal noise for continents and mountains, valleys, ridges, warping, cave fields, a height-map picture as an ingredient, kinds by height and slope for painting), evaluated per chunk on workers, with the surface query.
- Region files from Tools 1b gain sculpted-only storage and levels 1 to 3 (if not already reserved in Tools 1b).
- **Streaming by priority** (the study's recommended way) over three sources: memory cache → disk cache → sculpted region file → recipe, with the ring read ahead, a memory budget (1 GB, a setting), sculpted chunks written back (whole region file written beside and swapped, rule 11) before release.
- Height 512 m with stacked chunks.
- **Gate of W1:** a 10 km world opens in under a second (first view from cache or coarse-first), flying at 200 m/s never shows a missing chunk on the reference PC after the first pass, a dig 5 km away survives save and reopen, memory stays under budget.
- *Size: large.*

### W2. Far ground coarser, voxels to 4 km, and the horizon option
- Rings of 8 per level; levels 0 to 4 (1 to 16 m voxels) to about 4 km (0117's default: one level more than the study's 2 km); the transition band (Smooth, Sharp) and walls between sizes (Blocky); the far field beyond (height map tiles with skirts, from the recipe directly where unsculpted).
- **Voxels to the horizon** (levels 5 to 8 or so for 10 km): made from the recipe at each level's spacing (not averaged from level 0, which would need the whole world), GPU where available; its own scale cases; when a PC can't meet the frame target, the setting says so in Project settings and the editor (0117: "plainly rather than quietly drawing less").
- Popping control: averaging keeps levels within half a voxel; caves' thin roofs pop (the study's 12 to 19 m jumps). A **cross-fade** (dithered blend over a few frames when a chunk changes level) hides it at small cost.
- *Size: large.*

### W3. Physics and units across the world
- Colliders only round physics characters, shapes built on workers (shared with Tools 1b's path), characters held at the edge of ground not yet solid; units' ground via the surface query or level-0 columns kept by the streamer whatever the camera sees.
- 2,000 units spread over 10 km and gathered in one km² at the target.
- *Size: medium.*

### W4. The world map and the recipe's tools
- **The world map:** from the far field (one small file), regions drawn, sculpted ones marked, click to go; markers for levels' entities.
- **Recipe tools:** the noise settings with a live preview (a 2D map preview made on workers, then 3D), presets (hills, mountains, islands, canyons, caves), seed, height-map import (16-bit PNG/greyscale; `image` reads 16-bit PNG), an optional **bake** of a region range into region files (for a world that must never change if the recipe code changes, or to sculpt from).
- *Size: medium to large.*

### W5. Precision, game code, the scale cases, the manual
- **Precision:** draw relative to the camera always (chunk origins in f64 on the CPU, positions sent to the GPU relative to the camera): cheap and removes jitter at any size. **Physics and the game world** at 10 km are fine in f32 (≈1 mm); the 20 km and 41 km settings shift the simulation origin (a floating origin: when the player is far from the origin, everything is moved back by a whole region at a tick boundary, deterministically, and saved as an offset). Recommend making the origin shift a world setting tested at 41 km.
- **Game code** (rule 25): the ground's size, `isReady(place)` / event `GroundReady(...)` (wait for ground under a place before teleporting), regions loaded; the ground's generated file gains these.
- Scale cases: the 10 km world at full targets in each look, 20 km working, 41 km stress; voxels-to-the-horizon cases.
- *Size: medium.*

## Dependencies and order

- Needs Tools 1b (and what `13-tools-1b.md` asks it to get right now: chunk table, absent chunks, recipe hook, height as a parameter, chunk-local positions).
- The **noise module** (shared with Shared 3, materials, effects): build it here if Open worlds stays first.
- Interacts with **3D 3's cascaded shadows** (shadows reaching 10 km, far cascades refreshed rarely) and **3D 2's haze** (hides the far field's coarseness): the far ground looks best only after them.
- Entities across a big world (towns, trees) need streaming too: Shared 6's rooms/zones mechanism is the entity side; foliage and scattering are DESIGN.md §8 "Terrain: … scattering of grass, trees and rocks", not in any milestone yet.
- **Order concern:** the whole phase is 3D-only and large, and ROADMAP puts it before finishing Usable first and all remaining 2D (see `00-overview.md`, Q1).

## Risks and questions for the project lead

1. **Recipe versions kept forever** (so a Solis update never reshapes a world): **recommended** yes, with opt-in upgrades and backups; alternative: bake every world fully (large files).
2. **Erosion:** **recommended** local erosion-like noise by default plus a height-map picture ingredient; alternative: a baked erosion pass over a chosen area (non-local, slow, then stored like sculpts).
3. **GPU-made far ground** (drawing only, not game state): **recommended** yes where compute exists, CPU otherwise; alternative: CPU only (simpler, slower horizon option).
4. **Floating origin:** **recommended** camera-relative drawing always; simulation origin shifts only for worlds over 20 km; alternative: always shift.
5. **Gate size:** **recommended** 10 km at full targets (0117's promise) and 20 km working; alternative: the study's 20 km gate as written.
6. **Foliage and scattering** (trees, grass, rocks on the big ground) aren't in any milestone; **recommended** add a milestone (here or after 3D 2) when the lead wants it, since big worlds look empty without them.

## Crates

None needed beyond Tools 1b's (`flate2`/`miniz_oxide`, already in the lock). Noise: Solis's own module (FastNoise Lite, MIT, as reference).

## Size

Phase: large (five milestones; W1 and W2 large).

## Sources

- `docs/reports/open-worlds-study.md`, `docs/reports/ground-study.md`, decisions 0116 and 0117.
- Transition cells: [Eric Lengyel, Voxel-based terrain for real-time virtual simulations (Transvoxel, dissertation)](https://transvoxel.org/Lengyel-VoxelTerrain.pdf).
- Height-map LOD (for the far field): [Losasso and Hoppe, Geometry clipmaps (2004)](https://hhoppe.com/geomclipmap.pdf); [Strugar, Continuous distance-dependent level of detail (CDLOD, 2009)](https://aggrobird.com/files/cdlod_latest.pdf).
- Local terrain noise: [Inigo Quilez, fBm](https://iquilezles.org/articles/fbm/), [domain warping](https://iquilezles.org/articles/warp/), [noise derivatives (erosion-like fBm)](https://iquilezles.org/articles/morenoise/); Musgrave's ridged multifractal (Texturing and Modeling: A Procedural Approach).
- Precision: [Unity-independent explanation of floating origin (Thorne)](https://www.researchgate.net/publication/331628217_Using_a_Floating_Origin_to_Improve_Fidelity_and_Performance_of_Large_Distributed_Virtual_Worlds).
