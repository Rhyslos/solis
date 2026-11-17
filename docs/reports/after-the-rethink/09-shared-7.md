# 09 · Shared 7: materials and shaders

ROADMAP.md (was Phase 14; plan `docs/plans/materials.md`, decision 0048): **materials as assets** (`.solmat`, each with its own look: standard, stylized, light ramp, unlit; edited in Properties with a live preview); **a material per brush face** (moved here from Tools 1a B5); **inputs that drive any value** (the light around the surface, time, distance, height, wetness, noise, values from game code); **a node graph for materials and shaders**, saved as text, sharing its graph editor with the animator and effects; **code for experts** (a node holding a few lines of shader code, checked before it runs); **stylized building blocks** (light ramps, rim light, hatching, dithering, outlines, wind sway, triplanar; for sprites dissolve, flash, outline, glow); **per-copy values at scale**. **Gate:** without code, a sprite that flashes white when hit and dissolves when it dies, a plant that glows as night falls, a toon character with its own outline, and a tree that sways; the benchmark's target still at 60 fps.

`materials.md` already has M1 to M6 and nine questions with recommendations; its answers aren't recorded as a decision yet. This report checks that plan against today's code and the wider roadmap, and adds the technique comparisons.

## What exists (and what the plan assumed)

- **The renderer is a forward renderer with one large shader file** (`crates/render/src/viewport.wgsl`, 956 lines: sky, grid, lines, models in each look, lighting and shadows, sprites, the last pass), pipelines per look and pass in `gpu_model.rs` (`MaterialUniform`, `GpuMaterial`: glTF's metal-roughness material from the model) and `pipelines.rs`. The look (`render::Look`: Standard, Stylized, Unlit) is chosen **for the whole view** (decision 0013).
- **Instances:** 52 bytes (three rows of the placement and a pick number), batched per model, mesh, LOD and mirroring; gathered in jobs.
- **The OpenGL path** (Mesa, used by tests and the CI fallback): egui-wgpu asks for WebGL2-like limits there: **no storage buffers in shaders and no compute** (`phase-7-animation.md`, "Where things stand"). Per-instance values must travel as vertex attributes, not a storage buffer, or the GL path loses them.
- **The shared graph editor** exists with one user: the Animator tab (states as boxes, transitions as arrows). Materials need **typed pins and wires**, which it doesn't have.
- **Brushes save a material slot per face** (`Brush.materials`), drawn with one plain material now. **Sprites** have flip, tint, flash, outline, see-through per instance (88 bytes). **Ground kinds** (Tools 1b) are a palette of up to 256 (name, colour, picture) waiting for materials (decision 0116).
- **Order relative to light:** decision 0097 put materials **before** 3D light: "materials take the light around them as an input, so they work with today's lights and gain the new ones as they arrive". So the "light around the surface" input must be defined now in terms of today's sun, 16 lamps and flat sky ambient, and grow later.

## Techniques compared: how a graph becomes a shader

| Way | What it is | Strengths | Weaknesses |
| --- | --- | --- | --- |
| **Surface function inserted into Solis's lit shader** (the plan's recommendation) | The graph compiles to a WGSL function `surface(inputs) -> SurfaceOut { base_colour, normal, roughness, metal, emission, opacity, vertex_offset }`; Solis's template does the lighting, shadows, fog, look | Every material gets all light, every quality level, every look; a light feature is written once | One pipeline per material × pass × look (permutations); compile time on first use |
| **Fully custom shaders** | The graph or code writes the whole fragment shader | Total freedom | Misses every later light feature; breaks quality levels; the plan rejects it (an "unlit, I do it myself" output later) |
| **An interpreted graph** (a small bytecode run by one uber shader) | One pipeline for all materials | No compile stalls | Slow per pixel, branches, register pressure; poor on the reference cards |
| **Parameters only** (one uber material with many switches) | What exists now, grown | No compiling | Not a graph; doesn't meet the gate |

**Recommendation:** the plan's choice: the **surface function in a template**. Mitigate its weaknesses:
- **Permutations:** a material's pipelines are made for the passes it's used in (main, depth/shadow, pick, outline; later skinned and sprite variants), keyed by (material graph hash, vertex kind, pass, look, quality). Depth-only passes use a **shared** pipeline unless the material moves its corners or cuts out (alpha test), so most materials add one or two pipelines, not five.
- **Stalls:** build pipelines on a worker thread (wgpu's device is thread-safe for creation), draw with the parent or a fallback material until ready, never wait on the frame (rule 10). wgpu's `PipelineCache` (reuses the driver's compiled code between runs) is **Vulkan-only**; Solis's Windows backend choice (Direct3D 12 or Vulkan, decision 0049 "chosen by measuring") affects whether it helps. Keep the cache of generated WGSL by hash on disk in the project's cache either way.
- **Shader modules:** split `viewport.wgsl` into modules (lighting, shadows, fog, looks, sprites) with a small composer, so the template and graph code are assembled from parts. `naga_oil` (MIT or Apache-2.0, `#import`/`#define` over naga) or Solis's own few-hundred-line include-and-define step. **Recommended:** Solis's own (narrow, decision 0051), naga validates every result anyway.

## Per-copy values at the scale target

The gate wants 2,000 ships each tinted differently, drawn together.

| Way | Notes |
| --- | --- |
| **Extra per-instance vertex attributes** (up to 8 floats = 32 bytes; instances 52 → 84 bytes) | Works on every backend, including GL; adds bandwidth for every copy even when unused (unless a separate stream only for materials that use them) |
| **A storage buffer indexed by instance** | Flexible, no vertex layout change; **unavailable on the GL path** |
| **A separate draw per changed copy** | Simplest; fails at scale |

**Recommendation:** a **second, optional instance stream** for material values (bound only for batches whose material declares settable values), 16 or 32 bytes per copy. GL keeps working; copies of materials without settable values pay nothing.

## Inputs that drive any value

The plan's inputs, with what each needs today:
- **The light around the surface:** today the sun (with its shadow), the sky's flat ambient and the 16 lamps; computed in the template before the surface function, passed as `light_here` (luminance and colour). Grows with 3D 4's clustered lamps and 3D 5's bounced light without material changes.
- **Time** (game or real), **distance to camera**, **height**, **time of day** (decision 0092's clock exists), **noise** (Shared 3's shared noise, with a WGSL twin), **settable per-copy values**.
- **Wetness** comes from weather (Shared 9) and 3D 4's wet surfaces; define the input now (a value on the Environment, 0 until weather drives it).
- **Smoothing** ("fade in over 10 s"): per-copy state changing over time can't live in a stateless shader; it must be integrated on the CPU per copy (an eased value stored in the instance stream) or approximated by driving from a smoothed global (time of day already changes slowly). **Recommendation:** smoothing applies to the input on the CPU side (per material for global inputs; per copy only for the light-around input, which needs a GPU read-back otherwise). Flag: per-copy "light around me, smoothed" for 10,000 plants means reading back light per copy; instead approximate with the time of day and the lamps near each copy on the CPU. Ask the lead whether "glow as night falls" (global) is enough for the gate.

## Sprites and 2D

Sprites draw through their own pipeline (cut-out with `textureLoad`, depth from the foot, exact colours). Sprite materials are a **second template** (sprite surface: frame colour, palette row, light around, flash, outline, dissolve, glow) using the same graph boxes, so "dissolve when it dies" and "flash white" are graphs, and **pixel-art materials keep to the pixel grid** (UVs snapped to texels, a light-steps box). Ground kinds (Tools 1b) and brush faces use the 3D template with **triplanar mapping** (world-space, no unwrap), which the plan's M5 lists: bring it forward to M1 for brushes and ground.

## Milestones (the plan's, with notes)

- **M1 Materials as assets:** `.solmat` as a **Lust data file** (`<name>.material.lst`, decision 0081, not the reserved `.solmat` text format; decision 0015 reserved the extension before 0081 changed the rule: ask), each with its look; the simple view in Properties; a preview tab; **a material per brush face** (Material ▾, drag from Assets onto a face, pictures from the world so stretching never stretches them) and **per ground kind** (decision 0116 promised "a kind can name a material"); Make editable for imported materials. Fuzz the reader. *Medium.*
- **M2 Inputs and per-copy values:** the inputs above; a `MaterialValues` component; game code `setMaterialValue` / `setMaterialColour` under the guidelines (rule 4 says: through a file the editor writes per material, so `shipPaint:setTint(colour)` with settable names checked, not strings). *Medium.*
- **M3 The graph:** typed pins and wires added to the shared graph editor (the animator gains nothing it doesn't want; decision 0050's one editor); box search, groups, comments, previews per box (each box's preview is a tiny pipeline: expensive; render previews at 64 px on a worker, only for visible boxes); WGSL generation, naga validation, cache, live within about a second; mistakes on the box in Problems; a broken material draws pink. *Large.*
- **M4 Code boxes:** a WGSL function with typed pins, checked by naga; **loops need a fixed limit** (WGSL has no such check: the plan's rule must be enforced by Solis parsing the function's loops, or by forbidding loops except `for (var i = 0; i < CONST; i++)`, checked on naga's IR); errors mapped to box and line. *Medium.*
- **M5 Stylized blocks and motion:** ramps and bands, rim, hatching, dithering, halftone, pixel snapping, outlines per material (inverted hull exists for the whole view; per material needs the outline pass to read each material's width and colour), triplanar (move to M1), wind sway (area wind on the Environment) and ripples inside the corner-offset limit (culling grows bounds by it, as the plan says). *Medium.*
- **M6 At scale and the showcase:** the benchmark with per-ship tints, 10,000 swaying plants, Help › Make the material showcase. *Small to medium.*

## Dependencies and order

- **Before:** Shared 3's noise (for the noise box's twin), Tools 1b (brush faces and ground kinds use M1), 2D 2's palette rows (sprite template reads them).
- **After:** Shared 8 (particles drawn with these materials; particle inputs age, speed, random), Shared 9 (wetness from weather; bloom for glow), 3D 1 (skinned variants of every material pipeline), 3D 2 to 5 (more light in `light_here`).
- **A render refactor is due somewhere:** 3D 2 to 5 need a depth prepass, clustered lights, cascades, a post chain, and passes with dependencies. Materials will multiply pipelines across those passes. **Recommendation:** at the start of Shared 7, split `viewport.wgsl` into modules and introduce a small **frame graph** (named passes, their targets and order, written by Solis; no general render-graph library), so materials and every later light milestone plug into it rather than into one growing file.

## Tests and the scale check

GPU pixel tests per look and per box family (Mesa and D3D12); a material changed on disk reloads in a running game within about a second; a broken graph draws pink and lists the box; per-copy values keep one draw per batch (draw counts); the gate's four materials as fixtures. **Scale cases:** 2,000 ships each tinted differently; 10,000 swaying plants; pipeline count and creation time for the showcase; first-use stall measured (must not exceed the frame target; fallback drawn instead).

## Risks and questions for the project lead

The nine questions in `materials.md` stand (two views of one file; graph fills Solis's lit shader; WGSL code boxes; per-copy values; up to eight values per copy; Make editable; `.solmat`; moving corners with a promised limit; graph controls as the animator's). New ones:
1. **The material file's name:** **recommended** a Lust data file `<name>.material.lst` (decision 0081's rule); alternative: the reserved `.solmat`.
2. **Smoothing per copy** ("glow fades in over 10 s as it gets dark around *this* plant"): **recommended** global inputs smoothed on the CPU, and per-copy light approximated from the clock and nearby lamps; alternative: real per-copy light read back from the GPU a frame late (more cost).
3. **A render refactor at the start of this phase** (shader modules and a small frame graph): **recommended** yes; alternative: grow the single file and refactor at 3D 2.

## Crates

| Crate | Licence | Use |
| --- | --- | --- |
| naga (with wgpu, present) | MIT or Apache-2.0 | validating generated WGSL |
| `naga_oil` | MIT or Apache-2.0 | candidate for shader composition; recommended to write our own instead |
| `egui-snarl` | MIT or Apache-2.0 | node graph widget; the animation plan noted it; the shared graph editor is Solis's own, so a reference only |

## Size

Phase: large.

## Sources

- `docs/plans/materials.md`, `docs/plans/phase-7-animation.md` ("Where things stand": GL limits), decisions 0013, 0015, 0048, 0081, 0097, 0116.
- [wgpu PipelineCache (Vulkan only)](https://wgpu.rs/doc/wgpu/struct.PipelineCache.html); [Vulkan pipeline cache guide](https://docs.vulkan.org/guide/latest/pipeline_cache.html); [building a robust pipeline cache](https://about.roblox.com/fr/newsroom/2021/02/creating-a-robust-pipeline-cache-with-vulkan).
- [naga_oil](https://github.com/Wumpf/naga_oil) (shader composition).
- Triplanar mapping: [Ben Golus, Normal mapping for a triplanar shader](https://bgolus.medium.com/normal-mapping-for-a-triplanar-shader-10bf39dca05a).
- Stylized techniques: alpha erosion and gradient maps (see `docs/plans/effects.md` sources).
