# Phase 15 plan: Effects (Solstice basics)

**Draft for the project lead.** Since [decision 0046](../decisions/0046-light-before-netcode.md), effects are Phase 15 on their own, after materials and shaders in Phase 14 ([decision 0048](../decisions/0048-materials-and-shaders-before-effects.md)); game UI is Phase 12 ([its plan](game-ui.md)); the suite is called **Solstice** ([decision 0047](../decisions/0047-the-effects-suite-is-solstice.md)); and the post chain, bloom and grading (M1 and the grading below) move to light's Phase 13 ([its plan](light.md), L5), which also lifts the 16-lamp limit on flash lamps (L11). The rest of this plan is unchanged until Phase 14 starts. The working plan for the effects half of Phase 14 (ROADMAP.md, "Phase 14: Effects and game UI"): Solstice basics (particles through instancing, flipbooks, trails, stylized shading for effects) and post-processing (bloom and colour grading). Sungraph UI, the other half, has its own plan; the section "Shared with Sungraph UI" says what the two build once between them. Nothing here is decided until the project lead answers the questions below. Then this becomes `docs/plans/phase-9.md` (with the UI half), and the answers become decisions in `docs/decisions/` and lines in DESIGN.md §17. Update it as milestones are built.

**Gate (to be widened in the rewrite):** laser bolts, hits and explosions look good in the stylized look, in the benchmark battle, at the target level's 60 fps; a beginner makes a convincing campfire from presets alone, whose fire lights the scene and its own smoke.

## Answers (the project lead, 2026-10-25)

Recommended for every choice except three:

- **Particles: both CPU and GPU in Phase 15.** Particles are worked out on the graphics card (compute shaders) where it can, and on the CPU, in jobs, where it can't (older cards, the software driver tests use). One effect file drives both; tests check the two give closely matching pictures. The milestones need a GPU path added (likely its own milestone after M3).
- **Editing: a node graph now** (not a settings list first), saved as text (decision 0039's rule for graphs). M4 becomes the graph editor; it should share its graph widget with the animator's (Phase 7, A5) so both feel the same.
- **Light from explosions: one short flash lamp per effect**, nearest first, within the 16 lamps (decision 0014).

The rest as recommended: placed effects come back on load, bursts and particles don't; glowing and eroded (stylized) particles first, soft sorted smoke later; `.soleffect`; `playEffect`, `playEffectOn`, `stopEffect`; the ready-made kit; effects play in the viewport only while selected or with Preview effects on; Solis draws the basic shapes; bloom on and gentle everywhere; colour grading by sliders plus an optional LUT; no new keys.

## Direction (the project lead, 2026-10-27)

**Solstice should be good enough to be a professional tool in its own right**, while games stay the priority. The project lead "would be happy if Solis became popular for its effects suite alone". That changes this plan, which is to be **rewritten from an in-depth study before Phase 15 starts** ([decision 0051](../decisions/0051-understand-then-build.md)). What the rewrite must deliver:

- **Easy to learn, the same as the rest of Solis** ([decision 0050](../decisions/0050-one-way-to-work-everywhere.md)):
  - the material graph's boxes plus a few particle ones, in the same graph editor;
  - the same inputs, curves and gradients as materials;
  - the same keys;
  - no second graph language to learn.
- **Settings and presets for beginners:** an extensive library of ready-made effects and building blocks (fire, smoke, sparks, magic, weather, explosions, trails), each opened as an ordinary graph to learn from and change. Simple settings on top of each (size, colour, intensity, wind), so a beginner makes advanced effects without a degree in simulation.
- **Custom code in Lust, Rust or Lua,** like game code. The study decides where each runs fast enough:
  - per-particle work on the graphics card is graphs and shader code boxes;
  - per-particle work on the processor is Lust or Rust, compiled;
  - Lua suits per-effect and per-event logic (when to burst, what to spawn on a hit) rather than 100,000 particles a frame.
- **Light that is interactive, not just glow.** Emitting particles and glowing materials light their surroundings:
  - real lamps, with many particles gathered into a few lights so the cost stays fixed;
  - light into the bounced light and the volumetric fog, so fire lights its own smoke;
  - smoke that is lit and casts soft shadows.

  This builds on Phase 13 (many lamps, fog, bounced light) and Phase 14 (inputs).
- **Baking:** an effect or a simulation can be baked:
  - into flipbooks (with normal and glow maps, so baked smoke still takes light);
  - into textures that move a mesh's corners;
  - into cached playback.

  Heavy effects then cost little at run time, and baked results can be exported as image sequences for use in other tools.
- **Fast because it's understood:** the study goes deep on spawning, simulating, sorting, culling, drawing and budgeting particles on the reference card (decision 0049), and the design is measured before it's built.

## Pixel mode for pixel-art games (the project lead, 2026-10-27)

For 2D games in pixel art ([decision 0054](../decisions/0054-first-class-2d.md)). Their world is already drawn at the game's own small resolution, so effects become pixelated by themselves; the pixel mode makes them *look* right rather than just low-resolution:
- **Pixel-shaped particles:**
  - single pixels;
  - pixel lines for wind streaks and rain;
  - pixel circles for bubbles;
  - small pixel sprites.

  No soft blurry dots.
- **Movement snapped to the pixel grid,** so particles never shimmer.
- **Colours from the game's palette,** stepped, not blended.
- **Flipbooks at pixel-art frame rates.**
- **A ready-made set:** wind streaks, fireflies, bubbles, rain, snow, dust, falling leaves, sparkles, smoke puffs. Each is an ordinary effect to open and change.
- **Light:** fireflies and sparks can give off real light (the interactive light above), so a swarm lights a field at night.
- **Collisions:** particles collide with the 2D distance field from Phase 10 (rain splashing on roofs, leaves settling on the ground).

It's a mode turned on per effect or per project, using the same graph, presets and keys as the rest of the suite.

## Choices to make before any code

Each question has a recommended answer and the alternatives, in plain words.

1. **Where particles are worked out: on the processor (CPU) or on the graphics card (GPU)?**
   - **Recommended: on the CPU, in jobs, drawn by the GPU as instances.** This is how projectiles already work (Phase 5, M6: 100,000 handed to the GPU in 1.2 ms). Each frame the job system (`rayon`, decision 0031) moves every particle, then hands the GPU a list of small quads to draw in one go per kind of particle. It works on every graphics card Solis opens on, including old ones and the software driver the tests use (the OpenGL path cannot always run "compute shaders", the GPU programs other engines use for particles: wgpu says GLES 3.0 devices have none). It can be tested without a graphics card, and culling, levels of detail and trails are plain Rust. The target battle needs tens of thousands of particles, which the CPU handles in a millisecond or two.
   - Alternative: **on the GPU with compute shaders**, as Unreal's Niagara (GPU emitters), Unity's VFX Graph and Bevy's Hanabi do. Millions of particles, but harder to test, no fallback on older cards, and Solis would need two paths. Better as part of the full Solstice toolkit after 0.1, behind the same effect files.
   - Alternative: **both now.** Twice the work in a phase that is already large.
2. **What happens to effects in a save game?**
   - **Recommended: effects placed on entities come back; one-off bursts don't.** An effect placed on an entity (a ship's engine trail, a burning wreck's smoke, a campfire) is a component, so it is saved with the entity's other fields (decision 0038) and starts again after loading, already running (it "pre-warms", so smoke is already rising). A one-off burst fired from game code (a hit, an explosion) is over in a second or two and is not saved. Particles themselves are never saved: they are decoration, they would make saves much bigger, and the next frame makes new ones that look the same.
   - Alternative: nothing about effects is saved (a loaded ship has lost its engine trail until game code starts it again).
   - Alternative: every particle saved (big saves, nothing gained).
3. **How are effects edited in Phase 15?**
   - **Recommended: a list of settings per emitter, in a preview tab; a graph editor later.** Like Niagara's "stack" and Godot's particle settings: each emitter is a list of sections (where particles start, how many, how they move, how they look), each with values, ranges, curves and colour gradients. The effect file is written so a graph editor can come later without changing what's saved (see "The effect file").
   - Alternative: a node graph now (decision 0039's rule, "graphs saved as text"). Much more to build, and the full toolkit is "not in 0.1" (ROADMAP.md).
4. **The effect file's extension.**
   - **Recommended: `.soleffect`**, following decision 0015's rule (one extension per kind, starting `sol`, the kind in the name).
   - Alternatives: `.solfx`, `.solvfx` (shorter, less plain).
5. **What game code calls them.** Lust reserves some words (`save`, `load`, `stop`, `style`, `view`, `new`, `create` and others, LUST.md §22), so names should be distinctive.
   - **Recommended:** `playEffect(file, place, facing, size, tint)` for a one-off burst; `playEffectOn(entity, file, size, tint)`, which follows the entity and gives back an effect number; `stopEffect(number)`, which stops it making particles and lets the last ones fade. `tint` lets one hit effect take each fleet's laser colour.
   - Alternatives: `effectAt` / `effectOn` / `endEffect`; or one `effect(...)` with an entity or a place.
6. **See-through smoke.**
   - **Recommended: glowing (added) particles and stylized "eroded" particles first; soft see-through smoke later.** Added light (sparks, fire, flashes, lasers) can be drawn in any order. Stylized smoke and fire are drawn as solid shapes whose edge eats inwards over their life ("alpha erosion"), which fits the cel-shaded look, hides things behind it properly and needs no sorting. Soft, half-transparent smoke must be sorted back to front every frame, which costs time at the scale target.
   - Alternative: soft smoke in Phase 15 too, sorted per effect (not per particle) to keep it cheap.
7. **Where the pictures come from.**
   - **Recommended: Solis draws the basic particle shapes itself** (soft dot, circle, ring, star, spark, puff, noise), with the same shape functions Sungraph UI draws with, so effects need no outside images, look sharp at any size and carry no licence questions. Flipbooks (an animation painted as a grid of frames in one image) are added from the project's own PNG files.
   - Alternative: ship a set of particle images with Solis (each needs its licence in `licenses/`).
8. **Bloom's default.**
   - **Recommended: on, gentle, for the editor's viewport and the Game panel alike**, so what you build is what you'll see; strength in the area's Environment settings, and off with one tick.
   - Alternatives: off by default; or on in the Game panel only.
9. **Colour grading's controls.**
   - **Recommended: sliders (warmth, tint, contrast, saturation, shadows / midtones / highlights) plus an optional "look-up table" image** (a LUT: a small picture that maps every colour to a new one, made by grading a screenshot in Photoshop, Krita or GIMP and doing the same to a plain table, as Unity and Unreal do).
   - Alternatives: sliders only; LUT only.
10. **Ready-made effects.**
    - **Recommended: Solis ships a small kit** (laser hit, explosion, muzzle flash, engine trail, smoke, sparks) made by Solis like the benchmark ship, written into a project from the Help menu like the benchmark, for the gate and to copy from.
    - Alternative: none; the manual teaches making them.
11. **Effects in the editor's viewport.** Moving particles need frames back to back, and decision 0026 draws only when something changes.
    - **Recommended:** placed effects play in the viewport only while their entity is selected, or while **Preview effects** in the viewport's ≡ menu is on.
    - Alternative: always play (the editor never rests while an area has a campfire in it).
12. **Light from explosions** (a flash that lights up nearby ships).
    - **Recommended: later.** Solis lights at most 16 lamps at once (decision 0014); a battle's explosions would use them up.
    - Alternative: one short flash lamp per effect, nearest first, within the 16.
13. **Controls (CONTRIBUTING rule 17).** **Recommended: no new keys or mouse actions.** The preview tab has buttons (Play, Pause, Restart, Loop, speed). Alternative: Space restarts the preview while its tab is in focus.

## Where things stand

- **The picture is drawn in HDR** (decision 0017): the world goes into a 16-bit floating-point image with multisampling, and one last pass (`post_fs` in `crates/render/src/viewport.wgsl`) applies exposure and tone mapping (AgX, ACES or none) and turns it into sRGB. The decision says bloom and grading "will slot in there". The image's alpha is a mask (1 for the lit world, 0 for the editor's backdrop and lines), which every new pass must respect.
- **Projectiles** (`world::Projectiles`, `render::projectiles`): moved in jobs each tick, drawn as camera-facing glowing streaks at least 1.5 pixels wide, added to the picture without writing depth, in one instanced draw. They are game state: saved, in the checksum, deterministic. With no bloom they don't glow, they are only bright.
- **Instancing and culling** (Phase 5): `render::instances` gathers copies in jobs and checks each against the view (`Cull`). Particles can use the same culling.
- **Looks** (decision 0013): Standard, Stylized (cel bands, a crisp highlight, inverted-hull outlines) and Unlit, still for the whole view. Stylized is `banded()` in the shader, with a number of bands that effects can share.
- **Randomness** (Phase 6): game code's random numbers come from the game's seed and the tick's order. Effects must never use them.
- **Hits:** projectiles don't hit anything yet ("Hitting things waits for Phase 6's physics", Phase 5 plan). Game code can find hits with `castRay`. The gate's "hits" need projectile hits that game code hears about: game state, which is not part of this plan (see Risks).

## Determinism: effects are decoration, never game state

Projectiles change the game; effects only show it. So:

- **Game code can start and stop effects but never read them.** No native says where a particle is or whether an effect is still playing. An effect can't decide anything.
- **Effects' randomness is their own:** each effect gets a number made from a frame counter and its place in the frame's order, and its particles draw from that, never from the game's seed. Two PCs in a lockstep game may show slightly different sparks, which is fine; the game's checksum and replays are untouched (a test: the same game with effects on and off gives the same checksums).
- **The effect numbers game code is given** (from `playEffectOn`) are game state, so they are made from the tick's order like spawned IDs (decision 0035), the same on any number of cores.
- **Requests land like everything else:** `playEffect` is kept and landed at the end of the phase in order (decision 0033), then passed out of the world into a list for drawing. That list is not in the checksum and not saved.
- **Effects run on frame time, not ticks:** smooth on a 144 Hz screen, frozen while paused, one tick's worth on Step. A dedicated server (Phase 17) drops the requests and pays nothing.

## The effect file

An **effect** is a `.soleffect` text file in the project's assets (with a `.solmeta` sidecar for its ID, decision 0021), Lust-style blocks like areas (decision 0020) so it can be read, changed by hand and merged in Git. An effect is one or more **emitters**, each making one kind of particle. A sketch:

```
-- An effect. Safe to edit by hand.
effect version 1;

emitter sparks {
    start.shape = "sphere";        start.radius = 0.3;
    amount.burst = 40;             amount.rate = 0.0;
    life = [0.3, 0.6];             -- a range: each particle picks within it
    move.speed = [8.0, 16.0];      move.gravity = -9.8;   move.drag = 3.0;
    look.kind = "streak";          look.shape = "spark";  look.blend = "add";
    look.size = curve [0.0 0.15, 1.0 0.0];
    look.colour = gradient [0.0 [12.0, 6.0, 2.0], 0.6 [4.0, 0.8, 0.1], 1.0 [0.0, 0.0, 0.0]];
}

emitter puff {
    start.delay = 0.05;  amount.burst = 12;  life = [0.8, 1.4];
    look.kind = "billboard";  look.shape = "puff";  look.blend = "eroded";
    look.erode = curve [0.0 0.0, 1.0 1.0];  look.bands = "look";   -- banded in the stylized look
}
```

- **Sections in a fixed order**, like Niagara's emitter stack and Unity VFX Graph's contexts (spawn, initialize, update, output): `start` (shape, place, delay), `amount` (burst, rate, rate by distance moved for trails), `life`, `move` (speed, direction spread, gravity, drag, spin, follow the entity or stay in the world), `look`.
- **Every value is a number, a range, a curve or a gradient,** capped (curves 16 points, gradients 8 stops). The later graph editor adds one more kind, a reference to a node in the file's own graph section; old files stay valid, and the graph is saved as text as decision 0039 asks.
- **Looks (`look.kind`):** `billboard` (a quad facing the camera, optionally a flipbook), `streak` (stretched along its movement, as bolts are), `mesh` (a small model from the project, instanced like any model), `trail` (a ribbon behind the emitter or each particle).
- **Blends (`look.blend`):** `add` (glows, any order), `eroded` (solid, writes depth, edge eaten by `look.erode`), and later `soft`.
- **Limits:** at most 16 emitters per effect, 4,096 particles per emitter, a life of 30 s, a loop or a one-off; the file is read within size limits and fuzz tested.

## Particles at the scale target

The gate's battle: 2,000 ships, 20,000 lasers, hits and explosions. An explosion is about 150 particles for 1.5 s, a hit spark about 12 for 0.3 s.

| Case | What's alive | Must hold |
|---|---|---|
| Target battle | the target level plus 300 explosions alive and 3,000 hits a second (about 60,000 particles) | 60 fps; effects within 2 ms of CPU and 1.5 ms of GPU at 1080p |
| Stress battle | the stress level plus five times the effects (about 300,000) | 30 fps without stutter |
| Ceiling | effects added until 60 fps ends | reported |

- **One pool of particles**, capped at 1,000,000 alive (as projectiles), with a message past it, and a **drawn budget** (250,000 by default, a project setting). Over budget, far and small effects make fewer particles first; near ones never lose theirs.
- **Moved in jobs,** each job a run of effects, writing only its own slice (decision 0031). Each effect keeps a ball around its particles, grown as they move.
- **Culling:** an effect whose ball is out of view isn't drawn; one far out of view stops making new particles and only ages (it's decoration, so nobody can tell).
- **Levels of detail by size on screen,** as models (decision 0032): small on screen, an effect makes a share of its particles (each a little bigger, so it looks as full), drops emitters marked `detail = true`, skips trails and frame blending; a few pixels across, a single glow dot.
- **The GPU's cost is covering the screen** ("overdraw"): a big explosion in front of the camera draws many layers of large quads. Particles are capped in how much of the screen each may cover, and eroded particles write depth so they hide each other.
- **Small to send:** each particle sent to the GPU is about 32 bytes (place, size, rotation, colour, frame or erosion).

## Stylized shading for effects

What makes effects match the stylized look (decision 0013) rather than look borrowed from a realistic game:

- **Colour from a gradient over a grey shape** (a "gradient map"), banded into the look's number of bands when the view is Stylized, so fire has flat steps of yellow, orange and red like the models' light.
- **Eroded edges:** solid shapes that shrink from the edge inwards over their life, with a hard (Stylized) or soft (Standard) edge, and an optional dark rim just inside the edge: the effects' version of the models' outlines.
- **Lit puffs:** smoke shaded as if each puff were a ball, lit by the sun and sky in the same bands as models, so smoke has a lit side and a shadow side (the look of cel-shaded adventure games).
- **Mesh particles** use the models' own shading, outlines included.
- **Bolts** gain a white-hot core inside their colour, and bloom does the rest.

## Post-processing

The last pass grows into a short chain, all in plain drawing passes (no compute, so it runs on every card):

```
world (HDR) → bloom → exposure and tone mapping → colour grading → [pixelation] → UI layer → sRGB
```

- **Bloom: Jorge Jimenez's method** (SIGGRAPH 2014; the LearnOpenGL "physically based bloom" article). The HDR image is halved five or six times with a 13-tap filter, then built back up with a small tent filter, each level added to the next, and the result blended gently into the picture (about 4%) with no brightness threshold: bright things spread light, dark things barely change, as with a real lens. The first halving uses a "Karis average" so a single very bright pixel (a far laser 1.5 pixels wide) doesn't flicker. Only the lit world blooms (the alpha mask); the editor's backdrop and lines never do.
- **Stylized bloom:** an option that bands the glow into steps, for the toon look.
- **Colour grading:** the sliders are turned, on the CPU, into a small 32×32×32 colour table (a 3D texture) whenever they change; an optional LUT image (the 1024×32 strip Unity and Unreal use) is folded into the same table. The last pass looks each pixel up once, after tone mapping. A LUT changes each colour on its own, so blurs and glows can't be graded into one.
- **Settings** live on the area's Environment component (bloom on, strength, size, stylized; grading sliders and LUT), so each area has its own and they're saved and undone like any field.

## Shared with Sungraph UI

So nothing is built twice, the two plans agree:

- **One post chain** in `render` (`post.rs`), with fixed places for each step, built by this half's M1. The UI layer is composited near the end, after grading, so UI isn't tone mapped, bloomed or graded unless a style asks.
- **One blur pyramid** (the halve-then-build-up chain above), written once for bloom and used by Sungraph's **glow** style on the UI layer. Bloom builds it first.
- **One pixelation step,** for both the world (3D pixel-art games: drawn at a lower size and enlarged with hard edges, colours snapped by the grading table) and the UI layer. Built once, by whichever half reaches it first; recommended: the UI half, since pixelation is a Sungraph style, with the world option added in this half's M2.
- **One set of shape functions** (circle, ring, star, rounded box, drawn exactly per pixel), used by Sungraph for UI shapes and by effects for built-in particle shapes. Recommended: the UI half writes them; this half's M3 uses them, or writes the first few if it comes first.
- **Outlines** don't share code (models use inverted hulls, effects an eroded rim, UI its shapes' edges), but share their settings' names and colours, so a game's "outline colour" means one thing.

## Safety (CONTRIBUTING rules 8 to 10)

- **Limits:** particles alive (1,000,000), the drawn budget, emitters per effect (16), particles per emitter (4,096), effects started per tick (as spawns are capped), curve and gradient sizes, flipbook images (4096×4096, 256 frames), trail points per trail (64).
- **Files read get fuzz tests:** `.soleffect` files, LUT images and flipbook images; damaged ones are refused with a message in Problems, never a crash; a missing effect file named by game code is a warning, once.
- **No panics reach the frame:** NaNs from game code are refused at the native (as `fire` does); curves and gradients with points out of order are sorted when read.
- **The frame never waits:** effect files and images load on workers through the asset database and reload when changed on disk; an effect not yet loaded plays nothing that frame.

## The shape of the code

| Crate | Layer | What it gains | Milestone |
|---|---|---|---|
| `vfx` (new) | 1 | Effect files read and written; the particle pool; emitters moved in jobs; culling, budgets and levels of detail; trails. No GPU | M3–M6 |
| `render` | 1 | `post.rs`: the post chain, the blur pyramid, bloom, grading; `particles.rs`: billboards, streaks, eroded and mesh particles, trails, in instanced draws | M1–M6 |
| `area` | 0 | Environment's bloom and grading fields; the built-in `Effect` component (effect file, playing, size, tint) | M1, M2, M3 |
| `world` | 1 | The effect requests landed each tick (outside the checksum and saves) | M3 |
| `script` | 1 | Natives `playEffect`, `playEffectOn`, `stopEffect`, for Lust, Lua and Rust | M3 |
| `asset_db` | 1 | `.soleffect` as an asset kind; LUT and flipbook images | M2, M3 |
| `editor` | 2 | The effect preview tab and its settings; Preview effects in the viewport; Environment's new fields | M1–M4 |
| `app` | 3 | Help > Make the effects kit…; effect files loaded on workers | M3, M7 |

## Milestones

### M1. The post chain and bloom

- The last pass becomes a chain (`render::post`), with the blur pyramid and bloom as above, Karis average on the first halving, the alpha mask respected.
- Environment gains bloom on, strength, size and stylized; Properties shows them.
- Bolts gain a white-hot core.
- Profiler: a "post" timing beside shadows, models and the last pass.
- Manual: a page on bloom (what it is, why a gentle amount looks real, why too much looks like a dream sequence).
- Tests: a bright dot spreads and a dark area doesn't change; bloom off gives today's picture pixel for pixel; the backdrop and lines are untouched; a 1-pixel moving laser doesn't flicker frame to frame (Karis); odd and tiny image sizes (1×1, 3×7) don't fail. Scale case: bloom at 1080p and 4K.

### M2. Colour grading

- Sliders on Environment (warmth, tint, contrast, saturation, shadows, midtones, highlights) and an optional LUT image, folded into one 32³ table when they change; looked up after tone mapping. "Make a plain LUT…" writes the neutral strip into the project to grade in a paint program.
- The world pixelation option, if the UI half has built the pixelation step (see "Shared with Sungraph UI").
- Manual: grading, with a walk-through of making a LUT in Krita or GIMP.
- Tests: neutral settings change nothing (within a step of rounding); a LUT round trips; damaged and wrong-sized LUT images are refused (fuzz); the table is rebuilt only when a setting changes.

### M3. Effects: the file, the particles, and game code

- The `vfx` crate: `.soleffect` read and written (round trip, fuzz), emitters with `start`, `amount`, `life`, `move` and `look`; the pool moved in jobs on frame time; balls, culling, the budget and levels of detail.
- Drawing: billboards and streaks, added blend, with Solis's own shapes; particles leave the alpha mask lit where they cover the backdrop, so they're tone mapped.
- Game code: `playEffect`, `playEffectOn`, `stopEffect` (names per question 5), landed in order; the `Effect` component for effects placed in the area (saved with the entity, pre-warmed on load).
- Manual: a page on effects (what an emitter is, why effects are decoration and never change the game) and the natives in all three languages.
- Tests: an effect moves and expires on time; the same visual seed gives the same picture; the game's checksums are the same with effects on and off, and on 1 core and 4; caps and the budget hold; a despawned entity's effect stops making particles and fades. Scale cases: the target and stress battles' effects, and 300,000 particles handed to the GPU.

### M4. The effect preview tab

- Opening a `.soleffect` from the Library opens it in a document tab: a small viewport with the effect playing over the grid (Play, Pause, Restart, Loop, speed, Standard or Stylized), and its emitters as lists of settings with curve and gradient editors; changes show at once and are undoable; saved changes reload in a running game.
- Particle count and cost shown in the tab.
- Preview effects in the viewport's ≡ menu (question 11).
- Manual: making an effect step by step.
- Tests: edits round trip through the file; undo; a damaged file opens as a message, not a crash.

### M5. Flipbooks, eroded and mesh particles: the stylized look

- Flipbooks (rows × columns of a PNG, by frame rate or over life, blended between frames), eroded blend with hard or soft edges and the dark rim, gradient maps banded by the look, lit puffs, mesh particles with the models' shading.
- Manual: the stylized effects page (why erosion and bands, with pictures).
- Tests: pixel tests for each look in Standard and Stylized; frames chosen correctly at the ends of life; eroded particles hide what's behind them.

### M6. Trails and ribbons

- `trail` look: a ribbon of up to 64 points behind an emitter (engine trails) or each particle (sparks with tails), facing the camera, its width and colour along its length and over its age; points dropped as they age, added by distance moved so slow and fast look alike.
- Tests: a trail follows its emitter and shrinks when it stops; a teleported entity doesn't leave a streak across the world; the point cap holds. Scale case: 2,000 ships with engine trails.

### M7. The effects kit and the battle

- Help > Make the effects kit… (question 10): laser hit, explosion, muzzle flash, engine trail, smoke and sparks, all stylized, made by Solis.
- The benchmark uses them: hits spark, destroyed ships explode, ships trail their engines; the three levels run with effects, and the scale check reports their cost.
- Manual: the kit, and how to change its effects.

### End of the phase

- The review (decision 0027): three reviewers, the fixes in pull requests by kind, the scale check with all three levels, effects included.
- The gate, tried by the project lead on their PC.

## Risks

1. **The gate needs hits.** Projectiles don't hit anything yet. Hits that game code hears about are game state (deterministic, saved) and belong to the simulation, not to effects. If Phase 6 or 8 hasn't added them, a small milestone before M7 should.
2. **"Looks good" is taste.** The gate depends on the kit's effects as much as on the code. The preview tab (M4) comes early so the project lead can tune them while M5 to M7 are built.
3. **Overdraw** on the project lead's card, not the CPU, is the likeliest cost: big explosions close to the camera. The screen-cover cap and levels of detail are there for it; the scale cases measure it at 1080p and 4K.
4. **Phase 15 is large:** seven milestones. If time is short, soft smoke, stylized bloom and frame blending can wait for the full toolkit.
5. **Two halves, one chain:** if the UI half and this half both reach the post chain, the blur pyramid or the shape functions at once, they'll be written twice. The "Shared" section's order should be agreed before either starts.

## Sources

- Bloom: Jorge Jimenez, [Next Generation Post Processing](https://www.iryoku.com/next-generation-post-processing-in-call-of-duty-advanced-warfare/) (SIGGRAPH 2014); LearnOpenGL, [Physically Based Bloom](https://learnopengl.com/Guest-Articles/2022/Phys.-Based-Bloom).
- Compute shaders and older cards: wgpu-types 30.0.1, [`DownlevelFlags::COMPUTE_SHADERS`](https://docs.rs/wgpu-types/latest/wgpu_types/struct.DownlevelFlags.html) ("WebGL2, and GLES3.0 devices do not support compute"); the wgpu [boids example](https://wgpu.rs/doc/src/wgpu_examples/boids/mod.rs.html) for the GPU way.
- How others shape effects: Epic, [Overview of Niagara effects](https://dev.epicgames.com/documentation/en-us/unreal-engine/overview-of-niagara-effects-for-unreal-engine) (systems, emitters, spawn and update stacks, renderers); Unity, [VFX Graph contexts](https://docs.unity3d.com/Packages/com.unity.visualeffectgraph@7.6/manual/Contexts.html) and the [Flipbook Player block](https://docs.unity3d.com/Packages/com.unity.visualeffectgraph@17.5/manual/Block-FlipbookPlayer.html); Godot, [GPUParticles3D](https://docs.godotengine.org/en/4.2/classes/class_gpuparticles3d.html); Niagara's [ribbon renderer](https://dev.epicgames.com/documentation/en-us/unreal-engine/python-api/class/NiagaraRibbonRendererProperties).
- Bevy Hanabi 0.19.0 ([docs.rs](https://docs.rs/bevy_hanabi), MIT or Apache-2.0): GPU particles with init, update and render stages and an expression graph. Not suggested as a dependency (it needs Bevy and compute shaders); useful as a reference for a later GPU path.
- Colour grading LUTs: Unity, [User LUT](https://docs.unity3d.com/2018.3/Documentation/Manual/PostProcessing-UserLut.html) (strip format, grading a screenshot and the neutral LUT together).
- Stylized effects: [alpha erosion](https://vfxdoc.readthedocs.io/en/latest/shaders/alpha-erosion/); [Creating stylised explosions](https://quirkbit.itch.io/adrift-arena/devlog/122433/creating-stylised-explosions); soft particles for later: NVIDIA, [Soft Particles](https://developer.download.nvidia.com/whitepapers/2007/SDK10/SoftParticles_hi.pdf).
- New dependencies: **none needed.** `image` 0.25 (MIT or Apache-2.0) already reads PNGs for flipbooks and LUTs; `rayon` 1.12 runs the jobs; effects' randomness is a few lines of hashing, so no random-number crate.
