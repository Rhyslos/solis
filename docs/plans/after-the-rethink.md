# The plan after the rethink: the rest of 2D, the shared tools, then 3D

**The order is settled** ([decision 0120](../decisions/0120-the-order-after-the-rethink.md), the project lead, 2026-11-18): every recommended order below was accepted, and for the recipe, baking a whole world into files is a setting beside the recommended rules. ROADMAP.md follows it. Each phase's own questions are still asked when it starts.

The project lead asked on 17 November 2026 for extensive research on everything left after the editor rethink (Shared 1f): what has to be built, the best way to build each part, and how to build it in Solis's own code, followed by a detailed plan. A research agent read the repository at `origin/main` (e763513) and published papers and talks, and wrote one report per phase. They're in [`docs/reports/after-the-rethink/`](../reports/after-the-rethink/README.md), with every source. This plan is the main session's reading of that research: checked against the code, corrected where it was wrong, and turned into milestones.

Every milestone is still one pull request. Every phase still ends with its review, scale check, guide pictures and gate. UI is still mocked up first, and every new key or mouse action is still asked about first (rule 17). Nothing here changes a rule; it orders the work and says how to do it.

## Contents

1. [How this plan was checked](#how-this-plan-was-checked)
2. [The order](#the-order)
3. [Groundwork that runs through everything](#groundwork-that-runs-through-everything)
4. [Phase by phase](#phase-by-phase)
5. [Questions, and when each is asked](#questions-and-when-each-is-asked)
6. [Risks](#risks)

## How this plan was checked

The research was read in full: the overview, and the phases that come soonest (Tools 1b, "Usable first", Open worlds and 2D 2) line by line. The others were read for their recommendations, and the claims the plan leans on were checked in the code.

**Found true:**
- Two determinism gaps exist today, as decision 0106 records:
  - tweens' done blocks can run in another order with another number of cores;
  - Lust's own `Random` isn't kept to the game's seed.
- `crates/render/src/viewport.wgsl` is one 956-line file, and the renderer is forward with MSAA.
- The prototypes in `tools/ground-study` cover every part of the ground that Tools 1b needs.

**Corrected or sharpened:**
1. **"The OpenGL path has no compute."** That path is the test driver (Mesa's software OpenGL) and the fallback. It has no compute because egui-wgpu asks for WebGL2-like limits there (`docs/plans/phase-7-animation.md`), not because the driver lacks it: Mesa's software OpenGL offers OpenGL 4.5, which has compute shaders and storage buffers. Real players are on Windows with Direct3D 12 or Vulkan (decision 0012), where compute is always there.
   - **So:** a small check comes first in the renderer's restructure (Shared 7, M0): ask for higher limits on the GL path and see whether wgpu's GL backend gives compute under the tests. If it does, compute features are tested like everything else. If not, the research's rule stands: every compute feature has a fragment-pass or CPU version for the GL path, or is Default and High quality only.
2. **File names need no question.** Decision 0081 already says everything Solis makes, except `project.solis`, is `<name>.<kind>.lst`. So materials, effects, weather, music, cutscenes, behaviours, tables and the ground's settings are `.material.lst`, `.effect.lst` and so on. The research's question 5 and question 11 fall away:
   - **the ground's settings file is born as `.ground.lst` in Tools 1b,** whenever L2 and L3 happen;
   - **R5's prefab file is born as `.prefab.lst`,** so it never needs moving.

   The reserved `.solmat` extension (decision 0015) is noted as unused in RENAMING.md when Shared 7 starts.
3. **Levels move twice.** R2 has already made `.sollevel` files (format 4), so "Usable first" L4 moves them once more, to `.level.lst`. That costs one more upgrader and fixture, and nothing else.
4. **81 questions are too many to ask at once.** Most belong to the start of their own phase, as before. Eight are asked now, because they change the order or the shape of the work ([below](#questions-and-when-each-is-asked)). The research's recommendations that follow a rule or decision already made (a `Dash` action not added to the kit; tags saved in a Lust file) are taken as written and not asked.
5. **The research's order puts Open worlds after the shared phases.** The project lead asked for big worlds early (decisions 0116 and 0117). Moving them is the lead's choice, so it's asked first, with both orders laid out.

## The order

### Today's order (ROADMAP.md)

R1 to R6 → Tools 1b → **Open worlds** → finish "Usable first" → Shared 2 → 2D 1 → 2D 2 → Shared 3 to 10 → the rethink → Shared 11 → 3D 1 → 3D 2 to 5 → Shared 12 → Shared 13.

### What the research found in it

1. **Open worlds sits ahead of all the remaining 2D.** Decision 0097 put 2D first. Decision 0108 brought Tools 1 forward, and Open worlds was then attached behind it (0116, 0117). That puts five large, 3D-only milestones in front of the farming game's gate.
   - Open worlds also looks right only once 3D 2 and 3D 3 bring haze over distance and far shadows. Before that, its far ground shows plainly as coarse.
2. **2D 2 needs pieces from later phases:**
   - hit-stop, squash and shake need Shared 3's timers and tweens;
   - "the sky over 2D" needs 3D 2's physical sky;
   - "256 lamps" needs lamp culling, which 3D 4 brings.
3. **2D 1's "entities with typed fields"** is mostly Shared 4's field hints and new kinds of settings.
4. **The effects gate needs projectile hits as game state,** and they aren't built.
5. **The light plan puts the depth prepass in 3D 3,** but 3D 2 already needs it.
6. **Nothing covers foliage and scattering on the ground, or feet on slopes:**
   - grass, trees and rocks scattered on the ground (DESIGN.md §8) have no milestone;
   - feet resting on sculpted slopes need foot IK, which DESIGN.md leaves for later.

### The recommended order

★ marks a change from ROADMAP.md as it was. The lead accepted each one (questions O1 to O8, decision 0120).

| # | Phase | Milestones | Size | Why here |
| --- | --- | --- | --- | --- |
| 1 | **Shared 1f, the editor rethink** (R3 to R6) | 4 | large | In progress. |
| 2 | **Tools 1b, world sculpting** (G1 to G5): ★ moved into the 3D track before Open worlds by decision 0121 | 5 | large | As planned. Its files are shaped for Open worlds now, so they never need a second format. |
| 3 | ★ **Finish "Usable first"** (L2 to L6) and its gate | 5 | medium | Small file moves, so everything Solis makes is Lust. L2 and L3 are short and can go between any two milestones. |
| 4 | **Shared 2, game code: finding and making things** | 5, plus ★ a determinism check | medium | Any 2D game wants "the nearest of a kind" and "the cells that hold a value". The determinism check comes before Shared 3 puts more maths into game state. |
| 5 | **2D 1, the world and tiles** | 3 | large | Typed fields move to Shared 4 (★). |
| 6 | ★ **Shared 3, time and feel** | 5 | medium to large | Moved before 2D 2, so moving platforms, hit-stop, squash and shake are built once, properly. |
| 7 | **2D 2, movement, light and polish** | 5 | large | 2D complete. The sky over 2D uses day-and-night colours now, and the physical sky later. |
| 8 | **Shared 4, Shared 5, Shared 6** | 6, 6, 4 | large each | The ideas list's tools. |
| 9 | **Shared 7, materials and shaders** (with ★ the renderer's restructure first) | 7 | large | Everything visual after it builds on the restructure. |
| 10 | **Shared 8, 9, 10** | 7, 3, 2 | large, medium, large | Effects (★ projectile hits first), weather and the finished picture, cutscenes and behaviour. |
| 11 | **The rethink**, the big testing pass; **Shared 11** | | | As planned. |
| 12 | **3D 1, animation and navigation** (★ plus foot IK) | 6 | large | |
| 13 | ★ **Open worlds** (W1 to W5), then ★ **foliage and scattering** | 5 + 1 | large | Lands where haze and far shadows arrive (3D 2 and 3D 3), alongside or just before them. |
| 14 | **3D 2 to 3D 5**, light, sky and atmosphere | 4 + 4 + 5 + 4 | large each | |
| 15 | **Shared 12 netcode, Shared 13 Solis 0.1** | | | As planned. |

The lead chose Open worlds in the 3D track (question O1). The research's question 73 (a 10 km gate at full targets, with 20 km working) applies in both orders.

## Groundwork that runs through everything

### 1. Determinism

Game state must be the same on 1 core or 16 and on any PC: saved, checksummed and replayed (decisions 0033, 0036, 0106). Output never has to be: effects, light, sky, UI, music, shake and debug drawing.

- **The two gaps today:**
  - **Tweens' done blocks** need to know whose tween each is. This is asked of Lust (below). Until then, Shared 3's M2 orders them by entity and component wherever Solis can tell.
  - **Lust's `Random`** should take the game's seed. This is asked of Lust too. Until then, Problems warns when game code uses it.
- ★ **A cross-PC check** (a new last milestone of Shared 2): a recorded game is replayed by CI's Windows machine and by the local Linux run, and their checksums are compared tick by tick. The first tick that differs is reported with the components that changed. It's cheap to build (recordings, replays and checksums exist) and catches maths that rounds differently on another processor.
- **Rules for new game-state systems:**
  - brushes, ground edits and anything else that changes voxels or cells use whole numbers (fixed point), never floats that round differently;
  - noise for game state is integer-hashed and computed in Rust;
  - pathfinding answers are worked out from the world as it stood at the start of the tick;
  - anything made on the graphics card is for drawing only.

### 2. What to ask of Lust, early

Lust is its own project, so these go to the Lust side as early as possible, one request each. Every phase file names a fallback that works with today's Lust.

| Change | Needed by | Fallback today |
| --- | --- | --- |
| Attributes on component fields (`float speed = 4 [range 0 10, tip "…"]`) | Shared 4 M1 | Hints in a side file the editor writes |
| Lists of plain kinds in component fields, with a length limit | Shared 4 M2, 2D 1 | Lists only on built-in components |
| Blocks run later that may use only the component's own fields, with stable IDs and owners | Shared 3 M1 and M2 | Timers named in fields, with `TimerFinished` events |
| `on enter` and `on leave` for a component's own state | Shared 3 M3 | A `StateChanged` event |
| Weighted picks (`Random[Common 70, Rare 25, Legendary 5]`) using the game's seed, and `Random` seeded by the host | Shared 3 M5, determinism | `World:weighted(…)` |
| Test blocks (`test "…" { }`) and `expect` | Shared 2 M4 | Tests as components run by a test level |
| Drag-and-drop words in screens' `view` | Shared 5 M3 | Drag handled in game code from pointer events |

### 3. Pieces built once and shared (rule 21)

| Piece | First built in | Used again by |
| --- | --- | --- |
| **Noise** (one bit-exact module, with a twin in WGSL for drawing) | Tools 1b (from the study's `noise.rs`, placed in `reflect` or a small `noise` crate, not inside the ground) | 2D 1's decoration rules, Shared 3's smooth randomness, materials, effects, Open worlds' recipe |
| **The 2D distance field** (jump flood on the game's small picture; a brushfire pass on the map's grid) | 2D 1 M3 (clearance for pathfinding) | 2D 2's soft shadows and bounced light, Shared 8's pixel particles |
| **The neighbour grid** (a uniform grid built once a tick) | Shared 2 M2 (tags and "within", "nearest") | 2D 2's triggers, Shared 6's cheat box, 3D 1's avoidance, Shared 10's behaviour |
| **The `Draw` calls** (line, circle, box, cone, arrow, text, path) | Shared 4 M5 (drawing in the editor) | Shared 6's debug drawing while playing |
| **The timeline** (the Frames tab's) | built (decision 0064) | Shared 5's music sections, Shared 10's cutscenes, 3D 1's cues |
| **The graph editor with typed pins** | built for the Animator (decision 0050); typed pins in Shared 7 M3 | Shared 8's effects, Shared 10's behaviour |
| **The Lust file the editor writes per asset** (decisions 0078, 0079) | built | maps' new questions, per-prefab spawn builders, tables, materials' values, effects' parameters, music, cutscenes, behaviours and the ground: how game code reaches anything made in an editor (rule 25) |

### 4. The renderer's shape

Every feature from materials on wants **depth and normals before shading** (ambient occlusion, soft particles, reflections on the screen, fog, light round a surface) and **many lights**. So the renderer is restructured once, at the start of Shared 7 (★ question O4), before materials and light pile on:

1. **The GL check** (above): can the test driver get compute?
2. **Shader modules:** `viewport.wgsl` split into named modules, joined by Solis's own small composer, with naga checking the result.
3. **A small frame graph:** named passes that declare what they read and write, so features plug in without editing one long function.
4. **A depth-and-normals prepass with clustered lights ("forward+"):** this keeps MSAA, which the stylized outlines need, and keeps transparency and every look on one shading path. Clusters are built on the CPU and uploaded as textures, so the GL path lights the same way. Compute can build them later on cards that have it, if the CPU cost shows. This also starts 3D 4's many lamps early, and gives 2D 2's lamp culling a home.

Deferred shading was considered and set aside, because it loses MSAA.

### 5. Files

Every new format is `<name>.<kind>.lst` (decision 0081), starts at version 1 with an example in `tests/fixtures/` (rule 23), has a fuzz target (rule 9), and is written through `app::upgrades::save` with a backup. Big binary data (the ground's voxels) is the one exception decision 0116 allowed: region files with their own version and fuzz target.

## Phase by phase

Each phase lists its milestones with the way chosen to build them. The full comparisons, the code each touches, tests and sources are in the phase's report.

### Shared 1f, the editor rethink: R3 to R6 (in progress)

As [its plan](editor-rethink.md) says. One change from this research: **R5's prefab file is born as `<name>.prefab.lst`**, with the Lust shape and classes in `solis/data/` (decision 0081), so L4 has only levels to move. R5 starts by asking its two open questions (which settings a copy may change, and what happens when the prefab changes the same one).

### Tools 1b, world sculpting ([report 13](../reports/after-the-rethink/13-tools-1b.md))

**Moved** into the 3D track, after 3D 1 and before Open worlds ([decision 0121](../decisions/0121-tools-1b-joins-the-3d-track.md)). G1 waits, mostly built, on the branch `tools-1b/g1-ground`.

The study (G0) built and measured every approach, and decision 0116 settled the storage, the looks, the files, the size and what games may change. The work is turning the prototypes in `tools/ground-study` into Solis's crates.

**Where the code goes:**
- **a new `ground` crate** in layer 1, beside `tiles`: storage behind the `Store` interface, brushes, the three meshers, region files, the settings file, and queries. It knows nothing of the graphics card, the editor or the physics library. Rule 1 asks for a new crate to be proposed first; this plan proposes it.
- **physics:** colliders built on a worker;
- **render:** a pool for chunk buffers;
- **world:** the running game's copy-on-write ground;
- **script:** a generated file per ground;
- **editor:** HOME TOOLS · Ground, and the Ground shelf tab (mock-up 5, approved).

| Milestone | Gives | How | Size |
| --- | --- | --- | --- |
| **G1 The ground, Smooth** | One ground per level: solid, saved, drawn | Chunks of 32³ shared through `Arc` by the document, undo and the running game. Surface nets on workers, each crossed edge worked out once, normals from the voxels so seams shade smoothly, and a counter that throws away out-of-date meshes. Region files **with a chunk table and room for coarser levels now**. `<name>.ground.lst` settings with an empty "recipe" slot. Height a setting, not a constant. Chunk-local positions plus a chunk origin. A character held at the edge of a chunk whose collider is still being built. | large |
| **G2 Sculpting** | Add, dig, smooth, flatten with the approved keys | **Whole-number arithmetic on the voxel distances**, so the same strokes give the same ground on every PC. Dabs spaced along the drag, so speed doesn't matter. A ray marched through the voxels for the brush circle, so it follows caves. Undo keeps the touched chunks' "before", packed, under a memory cap. | medium |
| **G3 Blocky and Sharp** | The look as a setting per ground | Sharp: points pulled toward the planes, with hard edges past 40°. Blocky: cube faces merged into rectangles, with the grid snapped in the shader against hairlines and a fill underneath as a backup. A step height for characters on Blocky grounds. Switching looks keeps the old chunk drawn until its new one arrives, nearest first. | medium |
| **G4 Painting** | Ground kinds painted with the same brush | One picture per kind in a texture array, laid on from three sides. Smooth and Sharp blend three corners' kinds, sharpened by height, so grass gives way along rock's cracks. Blocky has one kind per face. Painting never moves the surface. | medium |
| **G5 Speed, game code, the manual** | Game code changes the ground; 2,000 units walk on it | The ground's file gives `height`, `kind`, `castRay` (one `GroundHit` class), `dig`, `add`, `smooth`, `paint`, and the slow `setLook` and `setSize`. Reads see the start of the tick; edits land at its end in order, with a limit per tick. Changed chunks go in saves (a version raise) and the checksum. A batch `heights(…)` for many units. Scale cases at 1 km² (the target) and 4 km² (stress). | medium |

**Asked when it starts:** the research's questions 64 to 68:
- Blocky step height, 1.05 m;
- the undo cap, 256 MB;
- region files shaped for Open worlds;
- game code's limit, 64 dabs a tick;
- how painted kinds blend.

**Mock-ups:** 5 and 6 are approved. The step-height and palette settings are drawn on them before G3 and G4.

### Finishing "Usable first": L2 to L6 ([report 01](../reports/after-the-rethink/01-usable-first.md))

Every file the editor writes becomes Lust data (decision 0081, `lust-data.md`).

| Milestone | Files | How | Size |
| --- | --- | --- | --- |
| **L2** | `.animator.lst`, `.clips.lst` | Lust's own reader. The block IDs are already stable. | small |
| **L3** | `.sound.lst`, `.mixer.lst`, `.pixels.lst`, `.input.lst` | The same way. Input keeps its key names. | small |
| **L4** | `.level.lst` (was `.sollevel`) | Lust's reader, with the scale check proving that 10,000 entities open within budget. A fast reader for Lust's shape is held in reserve, tested against Lust's own. | medium |
| **L5** | `<file>.meta.lst` sidecars | Read on the asset database's worker, never the frame. "A project of 5,000 assets opens without a late frame" as a scale case. | small to medium |
| **L6** | `.save.lst`, `.recording.lst` | A Lust header with packed sections for the bulk (projectiles, input frames), since saves and recordings are never merged in Git and must be fast. Needs the lead's yes (question U1). | medium |

**The gate:** a small 2D game of the lead's choosing, made in Solis in Lust while learning (the farming game is shelved; the project lead, 2026-11-18).

**Asked when it starts:** U1 (packed sections in saves and recordings).

### Shared 2, game code: finding and making things ([report 02](../reports/after-the-rethink/02-shared-2.md))

| Milestone | Gives | How | Size |
| --- | --- | --- | --- |
| **M1 Asking a tile map questions** | "Which cells are tilled?", "how many are watered?" | Generated methods per value (`cellsWhereTilled(…)`), sorted by row and column. Counts are already kept as cells change, so counting is free. Results are capped, and the cap is said aloud. | small |
| **M2 Tags and questions about the world** | `Tag.Crop`, "all crops within 3 m", "the nearest enemy" | Tags are a project state, set in Project settings and saved as `tags.lst`. A uniform grid built once a tick answers "within" and "nearest", in entity order, the same on every core. | medium |
| **M3 Spawning with values** | A bullet already moving on its first tick | A generated builder per prefab (`BulletSpawn:new():setSpeed(12):spawn(p)`), the run-time twin of a placed copy's overrides | medium |
| **M4 Game tests** | "A crop grows in three days", run from the editor or the command line | `test` blocks in `*.test.lst` with `expect`, able to press actions and step ticks, run without a window | medium |
| **M5 A note for code nothing fetches** | Problems says when a code file isn't reached from `main.lst` | `script::reach` already knows what is reached, so this lists what isn't, with a quick fix | small |
| ★ **M6 The cross-PC check** | Replays proven the same on two machines | The CI machine and the local run compare checksums tick by tick | small |

**Asked when it starts:** questions 12 to 17. They're about naming, and each has a recommended answer: per-value methods, tags in Project settings, 3D distance, generated builders, `test` blocks, and the query chain left to Lust.

### 2D 1, the world and tiles ([report 03](../reports/after-the-rethink/03-2d-1.md))

| Milestone | Gives | How | Size |
| --- | --- | --- | --- |
| **M1 Levels and the world** | Rooms laid out on a world map; tall things sorted properly | Level files plus a world file and a World view (moving between rooms at run time comes in Shared 6). Sort points, **sort lines** (one depth per column, nearly free since sprites already write depth), and a "Stands up" tile layer. Typed fields go to Shared 4. | large |
| **M2 Advanced tiling** | Maps that look hand-made: decoration rules with chance, noise, flips and offsets; scatter brushes; stamps | Rule groups over the terrain layer, written to a shareable `.decor.lst`, run on workers per changed chunk, with the shared noise | large |
| **M3 Pathfinding** | Characters walk round walls; many share one goal cheaply; mud costs more | **Hierarchical A\*** over the map's existing 32 × 32 chunks, refined one chunk at a time as a character walks, with corners cut by line of sight. **Flow fields** for goals many share. A cost per terrain (1 to 255). Clearance from the distance field for bigger characters. Only changed chunks rebuild. The budget: 500 characters re-pathing within 1 ms. | large |

Plain A* misses the budget, and jump-point search can't handle a cost per cell. The `pathfinding` crate is used only to check answers in tests.

**Asked when it starts:** questions 18 to 22, plus keys. Recommended: Scatter on `C`; saving a stamp with a button and `Ctrl + Shift + C`; no key for the World view. Mock-ups for the World view, the rule editor and scatter.

### Shared 3, time and feel ([report 05](../reports/after-the-rethink/05-shared-3.md)), moved before 2D 2

| Milestone | Gives | How | Size |
| --- | --- | --- | --- |
| **M1 Timers and steps in order** | "Do this in 2 seconds", "every day at 6", "walk there, wait, say this" | Blocks run later that use only the component's fields, so they can be saved (Lust change), plus named timers with `TimerFinished` | medium |
| **M2 Tweens made game-grade** | Tweens saved, restored and ordered on any number of cores | Closes decision 0106's gap | small to medium |
| **M3 States that know when they start and stop** | Anything started while "stunned" stops by itself when the stun ends | `on enter` and `on leave` (Lust change), extending "made in a state, ends with it" to timers, tweens, sounds and spawns | medium |
| **M4 Shake, flash, fade, rumble, hit-stop, squash** | Feel | Trauma-based shake from smooth noise. Flash and fade as overlays. Rumble to the pad last used. Hit-stop is game state; squash is drawing only. All of it moves the drawn camera or picture, never the game. | medium |
| **M5 Smooth randomness and weighted random** | Swaying, flickering, "usually common, sometimes rare", shuffled decks | The shared noise as a native call; weighted picks over a state's options with the game's seed | small |

**Asked when it starts:** questions 29 to 34.

### 2D 2, movement, light and polish: 2D complete ([report 04](../reports/after-the-rethink/04-2d-2.md))

| Milestone | Gives | How | Size |
| --- | --- | --- | --- |
| **M1 Triggers** | Doors, shop counters, pits | Solis's own overlap pass after physics, on the neighbour grid: `Trigger2D` and its 3D twin, built together. `Entered` and `Exited` sorted by trigger and entity. The "inside" sets are saved, so loading doesn't fire them again. | small to medium |
| **M2 Controllers and platforms** | A platformer that feels precise; a grid-step mover; one-way platforms, slopes, moving platforms | Solis's mover extended with sub-pixel remainders and the **carry-then-push** rule (riders carried, others pushed or squished). One-way edges merged along rows, so they never fail where tiles meet. Slope shapes per terrain. The kit's `PlatformerController2D`: jump set by height and time, coyote time, buffering, corner correction, wall jump. `GridStepController2D`. | large |
| **M3 Soft shadows, normal and glow layers** | Lamps cast soft shadows from walls and sprites; sprites can have normal and glow layers | A distance field of the casters by jump flood, at the game's resolution, then a soft trace toward each lamp, all in fragment passes. Each caster has a height. Normal and glow layers come from named Aseprite layers. A bevel is optional; flat stays the default. | medium to large |
| **M4 2D bounced light, the sky over 2D** | Light round corners and off coloured walls | **Radiance cascades** marched through M3's field, within 1.5 ms at 480 × 270. The sky over 2D comes from today's day-and-night colours and a sun direction, with the physical sky's colour later (3D 2). | large |
| **M5 Polish, the 2D sample, the gate** | Palette swaps and cycling, clean pixel-art rotation, a small sample (top-down and platformer rooms), made with Solis's tools, the manual's 2D section | A palette row per copy; an edge-aware rotation shader, with baked rotations as an import option | medium |

**Asked when it starts:** questions 23 to 27. Recommended: the mover extended; trigger event names; a teleport fires nothing; the sky from colour stops; 2D lamp culling built here.

### Shared 4, data and the editor ([report 06](../reports/after-the-rethink/06-shared-4.md))

- **M1 Field hints:** sliders, units and help from attributes on fields.
- **M2 More kinds of settings:** asset pickers with previews and drag from the Library, and lists (Lust change). This includes 2D 1's typed fields.
- **M3 Data tables:** `crops.table.lst` with typed columns, a key column that becomes a state, and a generated lookup.
- **M4 Paths drawn in the editor:** centripetal Catmull-Rom by default, curves with handles as an option.
- **M5 Components that draw in the editor:** the shared `Draw` calls, from game code run in its own sandbox with a time limit.
- **M6 Save slots:** a header, a picture read back without waiting, and autosave.

**Asked when it starts:** questions 35 to 41, including the path editing keys (Tab, click, `Ctrl` + click, Delete, `Alt` + drag).

### Shared 5, screens, words and sound ([report 07](../reports/after-the-rethink/07-shared-5.md))

- **M0 Text layout first:** wrapping, shaping and fallback fonts. `parley` if it passes the dependency checks, else Solis's own with `harfrust`, which is already in the build.
- **M1 Text effects:** markup in the text, each letter its own quad.
- **M2 Button pictures:** the last device used, with shapes drawn by Solis (no outside images).
- **M3 Tooltips, and drag and drop:** drops reach game code as events.
- **M4 Translations and captions:** Fluent files under a side-by-side table.
- **M5 Music that follows the game:** layers and sections, changing on the beat with kira's clocks.

**Asked when it starts:** questions 42 to 47.

### Shared 6, worlds and testing ([report 08](../reports/after-the-rethink/08-shared-6.md))

- **M1 Rooms and moving between them:** the run-time half of 2D 1's world file. Game time pauses during a change, so loading speed never changes results.
- **M2 Pools:** measure spawning first, and speed up plain spawning before adding engine pools.
- **M3 Debug drawing and the cheat box:** the shared `Draw` calls while playing; your own commands typed by type.
- **M4 Bug reports:** a recording from a known start, the build, a note, a screenshot and logs, in one file.

**Asked when it starts:** questions 48 to 53, including keys (`F8`, and the key left of `1`).

### Shared 7, materials and shaders ([report 09](../reports/after-the-rethink/09-shared-7.md))

- **M0 The renderer's restructure** (above), ★ new.
- **M1 Materials as assets:** `.material.lst` files, a preview tab, a material per brush face and per ground kind.
- **M2 Inputs and per-copy values:** a second, optional per-copy stream, so materials without per-copy values cost nothing; values set through the file the editor writes.
- **M3 The graph:** typed pins, WGSL made and checked by naga, live within a second, broken materials drawn pink.
- **M4 Code boxes:** loops only with a fixed limit, checked.
- **M5 Stylized blocks and motion.**
- **M6 At scale and the showcase.**

**Asked when it starts:** question 54 and `materials.md`'s nine.

### Shared 8, effects ([report 10](../reports/after-the-rethink/10-shared-8.md))

★ **First, projectile hits as game state** (a small milestone): the effects gate needs them. Then decision 0051's **effects study** (rule 22).

Then E1 to E7:
- **E1** the file and CPU particles;
- **E2** the graph and preview;
- **E3** the stylized look: flipbooks, eroded edges, banded colour;
- **E4** trails;
- **E5** GPU particles as a per-emitter option from the same graph;
- **E6** the pixel mode;
- **E7** the kit, baking to flipbooks, and the battle.

**Asked when it starts:** questions 55 to 58.

### Shared 9, weather and the finished picture ([report 11](../reports/after-the-rethink/11-shared-9.md))

- **M1 Bloom and white balance:** the post chain.
- **M2 Colour grading:** this takes over 2D 2's palette limiting.
- **M3 Weather and moods:** the chosen weather and wetness are game state, so rain can water crops (question 59).

**Asked when it starts:** questions 59 and 60.

### Shared 10, cutscenes and behaviour ([report 12](../reports/after-the-rethink/12-shared-10.md))

- **M1 A cutscene timeline:** on the shared timeline. Cutscenes are game state, so they're saved and can be skipped to the same end.
- **M2 Behaviour for characters:** state machines on the Animator's model, with a day's schedule as a top layer and utility scores as a kind of transition. One graph editor, one runtime, and 2,000 villagers within budget.
  - A mock-up shows it beside a behaviour tree first, so the lead can say which reads more easily (question 61).

**Asked when it starts:** questions 61 to 63, including the timeline's keys.

### 3D 1, 3D animation and navigation ([report 15](../reports/after-the-rethink/15-3d-1.md))

- **A1 Skins and clips from glTF:** rotations packed small; decoding exact.
- **A2 Skinned drawing:** bones from a texture, so the GL path works. Every material gets a skinned variant.
- **A3 Crowds:** clips baked for far characters, and animation detail by distance.
- **A4 Blending and blend spaces.**
- **A6 Cues on 3D clips:** on the shared timeline.
- ★ **A7 Foot IK:** two bones, feet on sculpted slopes (question O7).
- **N1 Navmeshes and avoidance:** built in tiles from brushes, models and the ground's colliders, with ORCA avoidance on the neighbour grid.
  - The navmesh builder is `rerecast` if it's checked and gains tiles, behind Solis's own interface; otherwise Solis's own.

**Asked when it starts:** questions 74 to 76.

### Open worlds ([report 14](../reports/after-the-rethink/14-open-worlds.md)), revised from decision 0117

The research revised the study's W1 to W5 as decision 0117 asked:

| Milestone | Gives | How | Size |
| --- | --- | --- | --- |
| **W1 Recipes, regions and streaming** | A 10 × 10 km world that opens in under a second | A versioned recipe (fractal noise, valleys, ridges, warping, caves, a height-map picture, kinds by height and slope), made per chunk on workers, with a quick "surface height here" query. Only sculpted chunks are stored. Chunks stream by priority from the memory cache, then the disk cache, then region files, then the recipe, under the 1 GB budget. 512 m high in stacked chunks. | large |
| **W2 Far ground coarser, to 4 km, and the horizon option** | No holes, cracks or popping in any look | Rings of fixed-size chunks with transition bands (Smooth, Sharp) or walls (Blocky). A height-map far field beyond. **Voxels to the horizon** as an option made as well as the default, on the graphics card where it can, with its own scale cases and an honest message when a PC can't keep up. A short cross-fade when a chunk changes detail. | large |
| **W3 Physics and units across the world** | 2,000 units anywhere | Colliders only near physics characters; units read the surface query, or voxels where caves are | medium |
| **W4 The world map and the recipe's tools** | Go anywhere, shape the start | A map from the far field, and the recipe with a live preview, presets, a seed, height-map import and an optional bake | medium to large |
| **W5 Precision, game code, scale cases, the manual** | Steady at any distance | Always drawn relative to the camera; the game's origin moved only for worlds past 20 km; `isReady(place)` and `GroundReady` | medium |

**Three rules the recipe needs** (question O8):
- **It must be local**, so no simulated erosion. Erosion-like noise is used instead, plus a height-map picture as an ingredient.
- **Recipe versions are kept forever**, so an update never reshapes a world.
- **Ground that game code or physics reads is made exactly on the CPU.** Far ground may come from the graphics card, because it's drawing only.

★ **Foliage and scattering** after it (question O6): grass, trees and rocks scattered by rules on the ground, drawn in batches, with detail by distance.

**Asked when it starts:** questions 69 to 73.

### 3D 2 to 3D 5, light, sky and atmosphere ([reports 16](../reports/after-the-rethink/16-3d-2-3.md) and [17](../reports/after-the-rethink/17-3d-4-5.md))

As `light.md` plans, with these choices:
- **3D 2:**
  - the showcase and quality levels;
  - real light units with automatic exposure;
  - **Hillaire's sky** (four small tables rebuilt every frame, built by fragment passes on GL);
  - light from the sky into a small cube map and nine numbers.
- **3D 3:**
  - sun, moon and stars from real formulas and a real catalogue;
  - clouds: a flat layer by default, volumetric at High;
  - **visibility-bitmask ambient occlusion**;
  - **cached cascaded shadows**, with virtual shadow maps not now.
- **3D 4:**
  - clustered lamps, which the restructure began, with 4 shadowed lamps at Default and 8 at High;
  - the light-ramp look for blocky games;
  - froxel fog;
  - reflection probes;
  - reflections on the screen, and wet streets.
- **3D 5:**
  - baked probes;
  - real-time bounced light through sparse distance fields, porting the open-source MIT code where it fits. The Blocky ground's voxels are a distance field already, worth measuring;
  - screen bounce;
  - a time-boxed radiance-cascades experiment.

**Asked when they start:** questions 77 to 81 and `light.md`'s own.

### Shared 11 to 13 ([report 18](../reports/after-the-rethink/18-shared-11-13.md))

Not planned in detail until the rethink (decision 0059). What earlier phases must keep, so these aren't blocked:
- every tool reached through commands, never an editor-only path (for the scripting window);
- binary files lockable in Git (for teams);
- determinism everywhere (for netcode).

## Questions, and when each is asked

### Asked first (the order), answered 2026-11-18

| # | Question | Recommended | Alternatives | Answer |
| --- | --- | --- | --- | --- |
| O1 | Where do Open worlds go? | After 3D 1, alongside 3D 2 and 3D 3, where haze and far shadows make big worlds look right | Straight after Tools 1b, as today | Later, in the 3D track |
| O2 | Shared 3 (time and feel) before 2D 2? | Yes | Small versions of hit-stop, squash and shake in 2D 2 | Yes |
| O3 | Typed fields from 2D 1 into Shared 4? | Yes | Pull field hints into 2D 1 | Yes |
| O4 | Restructure the renderer at the start of Shared 7? | Yes | At the start of 3D 2; or deferred shading (loses MSAA) | At the start of Shared 7 |
| O5 | A cross-PC determinism check at the end of Shared 2? | Yes | Later, before netcode | Yes |
| O6 | A milestone for foliage and scattering? | Yes, after Open worlds | When asked for | Yes, after Open worlds |
| O7 | A small foot-IK milestone in 3D 1? | Yes | DESIGN.md's "later" | Yes |
| O8 | The recipe's rules: local, versions kept forever, exact where the game reads it? | Yes | Bake every world fully into files | Yes by default, and baking a whole world as a setting |

### Asked when each phase starts

As before, each phase's builder brief begins by asking its own questions (the research's Q9 to Q81), one per choice, with the recommended answer first. The ones the plan already settles (file names, by decision 0081; the ground's settings born as Lust) aren't asked.

## Risks

1. **Lust's changes come from another project.** Each has a fallback, but the fallbacks are less pleasant to use. Ask early.
2. **The GL path** may not give compute. Then compute features need a second version or stay at Default and High, and the tests see less of them.
3. **Open worlds' recipe** is a promise for good: its old versions must keep working forever. Golden chunks per version, in tests, hold it.
4. **Radiance cascades in 2D** have known artefacts (rings round tiny bright lights, leaks). The fixes are known, and the budget is measured at 480 × 270 and 960 × 540.
5. **`rerecast`** has no tiles yet, and its licence wasn't shown on its documentation page. It's checked before 3D 1, and Solis's own builder is the plan B.
6. **The order itself.** Waiting longer for big worlds is a real cost to the lead's wishes, which is why O1 is theirs to answer.
