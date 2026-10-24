# Phase 5 plan: Scale

The working plan for Phase 5 (ROADMAP.md, "Phase 5: Scale"). It records the choices made before any code and a plan for every milestone, so a later session can carry on where this one stopped. Update it as milestones are built: tick them off here and in the README tracker, and change a milestone's plan if its design changes.

**Gate:** on the project lead's PC, the benchmark's **target** level (2,000 moving units, 20,000 projectiles) runs at **60 fps**; its **stress** level (10,000 units, 100,000 projectiles) holds **30 fps without stutter**; its **ceiling** run reports how far Solis goes; and the frame profiler shows where the time goes.

## Choices made before any code

- **Three levels, not one number** ([decision 0030](../decisions/0030-scale-is-tested-at-three-levels.md), chosen by the project lead 2026-10-24). Testing only at the target says nothing about what's just past it, and games will go past it (two fleets of fighter squads, a horde of 6,000 zombies):

  | Level | Units | Projectiles | Must hold |
  |---|---|---|---|
  | Target | 2,000 | 20,000 | 60 fps |
  | Stress | 10,000 | 100,000 | 30 fps, no stutter |
  | Ceiling | keeps adding | keeps adding | reported: where 60 fps ends |

  The scale check reports every level at the end of every phase from now on, so a drop shows before a game finds it. Later phases keep the levels (Phase 7 with animated units).
- **The job system is `rayon`** ([decision 0031](../decisions/0031-the-job-system-is-rayon.md)): the standard Rust library for spreading work over every core, MIT or Apache. Work is split into jobs that don't share what they change, so results are the same whatever the number of cores.
- **LODs: made automatically, or the model's own** ([decision 0032](../decisions/0032-lods-made-on-import-or-the-model-s-own.md), chosen by the project lead 2026-10-24). Solis makes simpler versions of each mesh when it imports a model (with `meshopt`, MIT or Apache); a model that brings its own (meshes named `…_LOD1`, `…_LOD2`, from Aventail or Blender) uses those instead.
- **The benchmark is made from the Help menu** (chosen by the project lead 2026-10-24): **Help > Make the benchmark…** makes it as a project, like the turret sample, so it can be looked around and changed.
- **What runs the units:** the benchmark's units are moved by a Lust component, so the levels time game code too, not only drawing. Drawing 10,000 units and running game code for 10,000 are different costs; the profiler shows them apart.
- **Phase 5 helps every kind of game,** not only battles: drawing many copies at once, skipping what can't be seen, and simpler models far away are what forests, grass, buildings and crowds in an open world need. Game-specific systems (squads, formations) stay in games (DESIGN.md §2).

## Where things stand

- **Instancing exists** (Phase 2): every mesh copy is an instance in one buffer per frame, and each mesh is drawn once for all its copies. But the buffer is gathered and sorted on one core every frame, from every placement.
- **Nothing is culled:** everything is drawn, whether the camera sees it or not, and drawn again into the sun's shadow map, which is fitted around the whole world (so shadows get blurrier as the world grows).
- **One core:** the world, game code and drawing all run on the frame's thread.
- **Game code** (Phase 4): a tick of 2,000 Lust components that move their entities takes 3.6 ms, so 10,000 would take about 18 ms on one core.

## Safety (CONTRIBUTING rules 8 to 10)

- **Limits stay:** the most instances in one frame stays bounded by what the GPU allows, with a message past it; the most projectiles alive is capped (1,000,000), with a message; LOD making has a time and size limit per mesh, and falls back to the full mesh.
- **Jobs never panic the frame:** a panic in a job is caught where the job is joined and reported, like a Rust component's.
- **Same result on every PC:** parallel work only ever writes to its own slice; anything that adds up results does it in a fixed order (Phase 6's determinism builds on this).

## The shape of the code

| Crate | Layer | What it gains | Milestone |
|---|---|---|---|
| `editor` | 2 | The Profiler panel; the game's view in jobs; the benchmark in play mode | M1, M2, M7 |
| `render` | 1 | GPU timings; gathering in jobs; culling; LOD choice; projectiles drawn in one go; shadows fitted to what's seen | M1, M2, M4, M5, M6 |
| `world` | 1 | Queries by component; the start-of-tick copy; the projectile pool; spawning from game code | M2, M3, M6, M7 |
| `assets` | 1 | Making LODs on import, and reading a model's own | M5 |
| `script` | 1 | Game code on every core (shards); new native methods: `fire`, `spawn`, `frameTime` | M3, M6, M7 |
| `app` | 3 | The job system's pool; Help > Make the benchmark…; GPU timestamps for the profiler | M1, M2, M7 |

## Milestones

### M1. Measure first: the frame profiler and the levels' scale cases ☑

- **The Profiler panel** (in the bottom dock, beside the Console and Problems; no new keys): this frame and the last few seconds as a graph, split into what takes the time: game code, the world, preparing the drawing, and the GPU's own passes (shadows, models, the last pass), using GPU timestamps where the card has them ("not measured on this card" where it hasn't). It names the counts too: units drawn and culled, triangles, projectiles.
- **Scale cases for the levels,** headless: the CPU's share of a frame (game code, the world, preparing the drawing) for target and stress, and a ceiling search that doubles, then narrows, until a frame takes over 16 ms. Synthetic for now (entities made in Rust); M6 swaps in the benchmark itself.
- Tests: the profiler's numbers add up to the frame; a missing GPU timer is said, not hidden.
- **Built.** The first measurement (CPU share, release build, a 2-core cloud machine): the target level takes 4.9 ms a frame (within its 8 ms); stress takes 30.5 ms against 16.7, of which **game code is 22.4 ms** and preparing to draw 6.2; the CPU alone keeps 60 fps up to about **6,000 units**. So game code is the stress level's biggest cost. Luau's native code generation (`luau-jit`) was tried and gave only 8%: a profile showed the Lust code itself is about 30% of game code's time, the safety timer about 20%, and the rest the crossings between Lust and the engine. So game code goes on every core (M3, decision 0033).

### M2. The job system ☑

- `rayon` with one pool for the whole program, sized to the cores, leaving one for the window.
- First jobs: working out what the game's view draws, and gathering and sorting the GPU's copies.
- Less work to spread: the world lists the entities with a given component by asking for those only (a query), not by walking all of them; the editor's own view of the area is worked out again only when the area, the models or what's hidden changed (while playing, it usually didn't).
- Jobs run only Solis's own code; `rayon` hands a panic back to the frame, as if it happened there.
- Tests: the same frame on 1 core and on 4 gives the same view and the same picture, pixel for pixel. Scale case: gathering 100,000 copies.
- **Built.** Preparing to draw at the stress level went from 6.2 ms to 0.7 ms a frame. Gathering 100,000 copies (200,000 mesh copies) went from about 45 ms to about 32 ms on a 2-core machine; most of what's left is making and sending 20 MB a frame, which M4 cuts (only what moved, in a smaller form).

### M3. Game code on every core ☑

[Decision 0033](../decisions/0033-game-code-sees-the-start-of-the-tick.md), chosen by the project lead 2026-10-24: during a tick, game code sees the world as it was when the tick started.

- **Shards:** as many workers as the job system has, each with its own Luau, the same code, and its share of the entities (chosen by ID, so it doesn't depend on how many cores there are for the result).
- **Reading** goes to a copy of the world made at the start of the tick (places, rotations, names): `positionOf`, `rotationOf`, `nameOf`, `find`; a component's own changes this tick are seen by it at once.
- **Writing** (`moveTo`, `turnTo`, `say`, `send`) is kept and lands when the updates are done, sorted by entity ID, component name, then order made; events are delivered in rounds, each reading the world the round before left.
- Rust components follow the same rules (they run on the frame's thread, reading the same copy).
- Swapping new code, Properties' changes, failures, the time limits and the tick budget work as before, per shard; failures and what's said come back in the same fixed order.
- Also, cheaper crossings between Lust and the engine (M1's profile): entities kept as numbers where they cross, and the safety timer's cost.
- Tests: the same game on 1 shard and on 4 gives the same world, the same Console and the same events, bit for bit; a component sees its own move at once but another's only next tick; two components moving one entity: the last in the fixed order wins. The turret sample plays the same.
- Scale: the stress level's game code, from about 22 ms.
- **Built.** `script::Game` runs one `Runtime` per core; reads go to a view made at the start of each phase, writes are kept with their place in the order and landed after. On a 2-core machine the stress level's CPU frame went from 29.5 ms to 16.7 ms (game code from about 26 ms to 13.3 ms) and the ceiling from about 7,000 to about 9,500 units; more cores give more. Along the way: the tick limit is per core (the frame waits for the slowest), and stopping components on many entities is one trip per core (a drag of 512 playing entities had become 16 ms; it's 2.9 ms). The cheaper crossings into Lust wait for M7's profile with real ships.

### M4. Drawing only what's seen ☑

- **Culling:** each mesh copy is checked against the camera's view (a sphere around it, then its box), in jobs, and only what's in view is drawn. The sun's shadow map draws only what can cast a shadow into view.
- **Shadows fitted to the view:** the shadow map covers what the camera sees (near things sharper), not the whole world, so a big world doesn't blur them.
- **Less to send each frame:** instances that didn't move keep their place in the GPU's buffer; only changes are sent, and each copy is smaller (its place as 3 × 4 numbers; the shader works out the rest), so the gathering scale case reaches its target.
- Tests: off-screen copies aren't drawn (pixel and count tests); shadows from just outside the view still fall into it; nothing pops at the screen's edges. Scale case: 10,000 units with the camera seeing a tenth of them.
- **Built.** Copies are culled against the view and the shadows' box, in the gathering jobs; the shadow map covers what's in view up to 200 m (`SHADOW_REACH`) and steps a texel at a time. Each copy went from 104 to 52 bytes (the shader works out the normals' matrix), and only runs of copies that changed are sent. Gathering 100,000 copies went from about 34 ms to about 9 ms on the 2-core machine (Vulkan); a still scene sends nothing after its first frame, but when every copy moves, all of them are still sent (about 10 ms). The rest is memory-bound on this machine; M7's benchmark measures it on real ones before the levels are judged. 10,000 moving units with a tenth in view and shadows on: 1.2 ms. Keeping each copy's place in the GPU's buffer across frames (so a copy entering the view doesn't shift the rest) waited: it needs the world to say what changed, which the review can weigh.

### M5. LODs ☐

- **Made on import** (decision 0032): for each mesh, two or three simpler versions (about half, a quarter and an eighth of the triangles), kept in the asset cache so it happens once. Within limits; a mesh that can't be simplified well keeps its full version.
- **A model's own:** meshes named `Name_LOD1`, `Name_LOD2` … in the file are used instead, and not drawn as parts of their own.
- **Choosing:** by how big the copy is on screen, with a little overlap so a unit at the boundary doesn't flicker between two. Properties shows a model's LODs, and the Profiler how many copies are drawn at each.
- Manual: a page on LODs (what they are, why, how to make your own in Aventail or Blender).
- Tests: a far copy draws the simpler mesh; a model's own LODs win; LODs are made once and kept; damaged LOD names are ignored, not a crash. Scale case: 10,000 units, most far away.

### M6. Projectiles ☐

- **A projectile pool** in the world: thousands of small, fast things that aren't entities (lasers, bullets, tracers), each with a place, direction, speed, time left and look. Moved in jobs every tick; they expire when their time is up. Hitting things waits for Phase 6's physics.
- **Drawn in one go:** stretched, glowing quads (lasers) or small meshes, instanced, one draw call per look.
- **From game code:** a native method `fire(from, direction, speed, life, look)` in `solis/world.lst`, for Lust, Lua (`solis.fire`) and Rust.
- Capped at 1,000,000 alive, with a message.
- Tests: they move and expire on time; same result on 1 core and many; the cap holds. Scale case: 100,000 projectiles.

### M7. The benchmark ☐

- **Help > Make the benchmark…** makes a project: two fleets of simple ships (made by Solis, nothing proprietary) whose Lust component flies them around and fires lasers, in three areas: `target.solarea`, `stress.solarea` and `ceiling.solarea`.
- **Spawning from game code** (brought forward from Phase 6, since the ceiling run needs it): `spawn(assembly, place)` and `despawn(entity)` native methods, entities made and removed while the game runs.
- **The ceiling run:** its Lust adds ships in steps while it plays, watching `frameTime()`, and says in the Console where 60 fps ends ("60 fps up to 14,000 ships and 140,000 lasers on this PC").
- The scale check runs the benchmark's own areas headless from now on.
- Manual: a page on the benchmark and the Profiler; the README's Phase 5 section.
- Tests: the benchmark's code checks cleanly and plays; spawning and despawning in a tick are safe (events to a removed entity go nowhere).

### End of the phase

- The review (decision 0027): three reviewers, the fixes in pull requests by kind, the scale check with all three levels.
- The gate, tried by the project lead on their PC.
