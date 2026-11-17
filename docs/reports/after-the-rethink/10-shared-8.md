# 10 · Shared 8: effects (Solstice)

ROADMAP.md (was Phase 15; plan `docs/plans/effects.md`): **Solstice**: the file, particles on the CPU and the GPU, game code (particles drawn with Shared 7's materials); **the effect preview tab and its graph**; **flipbooks, eroded and mesh particles** (the stylized look); **trails and ribbons**; **the effects kit and the battle**; **a pixel mode for pixel-art games** (pixel-shaped particles, movement on the pixel grid, the game's palette, a ready-made set; fireflies and sparks giving real light). **Gate:** laser bolts, hits and explosions look good in the stylized look in the benchmark battle at 60 fps; in a pixel-art game, fireflies light a field at night and wind streaks cross it, crisp on the grid.

`effects.md` is detailed (answers of 2026-10-25: **both CPU and GPU**, **a node graph now**, one flash lamp per effect within 16; `.soleffect`; `playEffect`/`playEffectOn`/`stopEffect`) but the project lead's 2026-10-27 direction asks for it to be **rewritten from an in-depth study before it starts** (decision 0051), aiming for "a professional tool in its own right". Its post chain, bloom and grading moved to light (now Shared 9). This report is input for that study.

## What exists

- **Projectiles** (`world::Projectiles`, `render::projectiles`): a million-capacity pool of non-entities moved in jobs each tick, drawn as camera-facing streaks in one instanced draw: the model for CPU particles. But projectiles are **game state**; particles are **decoration**.
- **Instancing and culling** (`render::instances`, `Cull`), LODs by size on screen, the frame profiler with GPU pass times.
- **The OpenGL path has no compute shaders** (WebGL2-like limits). Particles on the GPU need compute, so the CPU path is the fallback the tests draw.
- **Materials** (Shared 7) bring the graph, inputs, curves and gradients; **game UI's distance-field shapes** (circle, ring, star, rounded box) can draw built-in particle shapes, as the plan proposes.
- **2D:** the pixel stage, palette rows (2D 2), the 2D distance field (2D 2) for particle collisions; **lamps** (16, decision 0014; clustered later in 3D 4); 2D bounced light (2D 2) can take particle emission.

## Techniques compared: where particles are simulated

| Way | What it is | Strengths | Weaknesses |
| --- | --- | --- | --- |
| **CPU in jobs, drawn instanced** | As projectiles: each frame `rayon` moves particles, writes a compact instance list (≈32 B each), one draw per kind | Every card and the test driver; easy to test; culling, LOD, trails in plain Rust; events back to the CPU easy | Upload bandwidth (300,000 × 32 B ≈ 10 MB a frame at the stress level); CPU time ≈ 1–2 ms at 60,000 |
| **GPU compute** (spawn with an append buffer, simulate, compact the live list, write an indirect draw's count; sort if needed) | The established GPU-particle design (Latta's GPU particles; AMD's compute-based particles with tiled culling) | Millions of particles; no upload; collisions with depth buffer or distance fields cheap | No GL path; harder to test and debug; reading results back to the CPU (spawn-on-death events) costs a frame of delay |
| **Both, one file** (the lead's answer) | The effect file drives either; a test checks they look alike | Scales up on good cards, works everywhere | Two implementations of every module; behaviour must match closely; the graph must compile to both Rust (or an interpreter) and WGSL |

**Recommendation for the study to confirm:** **CPU first for everything, GPU as an opt-in per emitter** ("GPU" tick box, only modules that can run on the GPU allowed), not two complete paths. Per-particle graph logic compiles to WGSL for GPU emitters (reusing Shared 7's code generator) and is **interpreted on the CPU** from the same graph (a small expression VM, plenty fast at the target's 60,000 particles), so one graph serves both with a test that the two agree within tolerance. Per-effect logic (when to burst, what to spawn on a hit) stays in game code (Lust, Lua, Rust), as the lead asked. This keeps the GPU path for what needs it (rain over a whole valley, sparks by the hundred thousand) without doubling every feature.

## Rendering particles cheaply

- **Overdraw** is the likely cost (the plan's risk 3): cap screen coverage per particle, LOD by size on screen, **eroded (alpha-tested) particles that write depth** for the stylized look (no sorting, hide each other), additive glows that need no sorting.
- **Soft see-through smoke** needs sorting (per effect, back to front; per particle within an effect only when needed) and soft edges against geometry (soft particles read scene depth: needs a depth texture, which 3D 3's depth prepass brings; until then, in the forward pass with the depth attachment copied).
- **Low-resolution particles** (draw smoke at half resolution and upsample) is an established saving; keep for High quality.
- **Lit particles:** puffs shaded as spheres in the look's bands (the plan's "lit puffs"); real lighting from Shared 7's template for mesh particles.

## Light that's interactive

The lead asked that emitting particles light their surroundings. Techniques:
1. **Gather particles into a few lights** per effect (a fire = one flickering lamp at its centroid, brightness from the live emitters' energy). Cheap and fits today's 16-lamp limit (the answered "one flash lamp per effect, nearest first").
2. **Many small lights** via clustered lighting (3D 4), each bright particle a short-range lamp: real but needs 3D 4.
3. **Into bounced light and fog:** 2D: particle emission drawn into 2D 2's radiance-cascade emitters (fireflies lighting a field: the gate's pixel half). 3D: into 3D 4's fog and 3D 5's bounce.

**Recommendation:** 1 now, 3 for 2D now (the 2D gate needs it and 2D 2's cascades take emitters), 2 and 3D's part when those phases land.

## The pixel mode

Particles drawn into the game's small picture already look pixelated; the mode makes them look **right**: shapes drawn as single pixels, pixel lines (rain, streaks) and pixel circles (bubbles) rasterised exactly at the game's resolution; positions snapped to whole pixels when drawn (sub-pixel kept in the simulation, as the pixel stage does for the camera); colours from the palette (2D 2's palette asset; stepped, not blended); flipbooks at pixel-art frame rates; collisions with the 2D distance field (rain splashing on roofs, leaves settling). A ready-made set (wind streaks, fireflies, bubbles, rain, snow, dust, falling leaves, sparkles, smoke puffs), each an ordinary effect to open and change.

## Determinism

Unchanged from the plan, and important: effects are **decoration**: game code can start and stop them, never read them; their randomness is their own (never the game's seed); requests land in order at the tick's end; they run on frame time; a test checks checksums are equal with effects on and off and on 1 and 4 cores. Effects placed on entities come back after loading (pre-warmed); bursts don't.

## Game code (checked against the guidelines)

The plan's `playEffect(file, place, facing, size, tint)` predates the guidelines (decision 0065). Under them: effects are project names (`state Effect`), so `Audio`-like calls on a new engine object:
- `Effects:play(Effect.Hits.Laser, place, facing)` and `Effects:playTinted(…, colour)` (variant at the end), `playEffect(Effect.Engines.Trail)` on an entity (follows it) returning a handle (`EffectHandle`, a small class with `stop()`), and `stopEffects()` on an entity.
- Effects that expose parameters (size, intensity, colour) reach code through a file the editor writes per effect (rule 4): `fire:setIntensity(0.5)`.

## Milestones (proposal for the study to refine)

1. **E1 The file and CPU particles:** `.effect.lst` (a Lust data file, decision 0081, instead of `.soleffect`: ask), emitters with spawn, initialise, update, render stages; the pool in jobs; culling, budget, LOD; billboards and streaks; game code; Effect component (pre-warm). Fuzz target. *Large.*
2. **E2 The preview tab and the graph:** the shared graph editor with particle boxes (Shared 7's boxes plus age, speed, random, spawn shapes, forces, curves over life), previews, presets as ordinary graphs, simple settings on top (size, colour, intensity, wind). *Large.*
3. **E3 The stylized look:** flipbooks (with normal and glow maps), eroded particles with hard or soft edges and a dark rim, gradient maps banded by the look, lit puffs, mesh particles. *Medium.*
4. **E4 Trails and ribbons.** *Medium.*
5. **E5 GPU emitters:** compute simulation, indirect draws, depth/distance-field collisions, the same graph compiled to WGSL; the GL path falls back to CPU with a note. *Large.*
6. **E6 The pixel mode** (2D set, palette, grid, 2D light from particles). *Medium.*
7. **E7 The kit and the battle,** baking to flipbooks (render an effect into a sheet of frames with normals), the gate. *Medium.*

## Dependencies and order

- Needs Shared 7 (materials, graph code generator), Shared 3 (noise), 2D 2 (palette, 2D distance field, 2D cascades' emitters).
- Shared 9's weather uses its rain and snow; Shared 10's cutscenes trigger effects.
- 3D 3's depth prepass helps soft particles; 3D 4's clustered lights lift the lamp limit. Design for them, don't wait.
- **Hits** (projectiles that game code hears about) were the plan's risk 1: still not built; the gate needs "hits spark". A small game-state milestone (projectile hits via physics rays, deterministic) before E7.

## Tests and the scale check

Effects never change checksums (on/off, 1/4 cores); CPU and GPU emitters agree statistically and visually within tolerance; caps hold; the file round-trips and is fuzzed; flipbook frames chosen right at life's ends. **Scale cases:** the plan's target battle (≈60,000 particles, 2 ms CPU and 1.5 ms GPU at 1080p), stress (≈300,000 at 30 fps), ceiling; 2,000 ships with engine trails; a full-screen explosion's overdraw at 1080p and 4K.

## Risks and questions for the project lead

1. **CPU and GPU:** the 2026-10-25 answer was both. **Recommended:** CPU for every emitter, GPU as a per-emitter option with one graph compiled to both (not two complete systems); alternative: two full paths as answered.
2. **File name:** **recommended** `<name>.effect.lst` (decision 0081); alternative: `.soleffect`.
3. **Calls under the guidelines:** **recommended** `Effects:play(Effect.Hits.Laser, place, facing)`, `playEffect(...)` on an entity, parameters through the generated file; alternative: the plan's original names.
4. **Hits as game state** before the gate: **recommended** a small milestone giving projectile hits to game code.
5. **The study first** (decision 0051 and the lead's direction): recommended a study report (`docs/reports/effects-study.md`) measuring CPU vs GPU particles at the scale target on the reference card before E1.

## Crates

None needed (the plan's conclusion holds: `image` for flipbooks, `rayon` for jobs). `bevy_hanabi` (MIT or Apache-2.0) is a design reference for GPU stages and expression graphs only.

## Size

Phase: large (seven milestones).

## Sources

- `docs/plans/effects.md` and its sources (Jimenez's bloom, alpha erosion, soft particles, Hanabi, wgpu downlevel flags).
- GPU particles: [Gareth Thomas (AMD), Holy smoke! Faster particle rendering using DirectCompute (slides)](https://www.slideshare.net/slideshow/holy-smoke-faster-particle-rendering-using-direct-compute-by-gareth-thomas/35656271); [Advanced visual effects with DirectX (GDC)](https://gdcvault.com/play/1020002/Advanced-Visual-Effects-with-DirectX); [Wicked Engine: GPU-based particle simulation](https://wickedengine.net/2017/11/gpu-based-particle-simulation); [tile-based particle rendering discussion](https://gamedev.net/forums/topic/692435-tile-based-particle-rendering/?page=1).
