# 00 · Overview: the rest of 2D, the shared phases, and the rest of 3D

Research for the development plan, read against `origin/main` at e763513 (R1 merged, 17 November 2026), assuming the editor rethink (R1 to R6) is built: "area" is "level", prefabs exist, projects are 2D, 3D or both. One file per phase in this folder:

| File | Phase | Size |
| --- | --- | --- |
| `01-usable-first.md` | Finishing "Usable first": one project file, the rest as Lust (L2–L6), the gate | medium |
| `02-shared-2.md` | Shared 2: tile map questions, tags and world questions, spawning with values, game tests, unfetched files | medium |
| `03-2d-1.md` | 2D 1: rooms on a world map, sort points, advanced tiling, pathfinding | large |
| `04-2d-2.md` | 2D 2: triggers, platformer and grid-step controllers, soft shadows, normal/glow layers, 2D bounced light, polish | large |
| `05-shared-3.md` | Shared 3: timers, tweens, steps, states with enter/leave, shake/flash/fade/rumble, noise and weighted random | medium–large |
| `06-shared-4.md` | Shared 4: field hints, more kinds of settings (lists), data tables, paths, editor drawing, save slots | large |
| `07-shared-5.md` | Shared 5: text layout, text effects, button pictures, tooltips and drag-drop, translations and captions, adaptive music | large |
| `08-shared-6.md` | Shared 6: rooms and transitions, pools, debug drawing and cheat box, bug reports | large |
| `09-shared-7.md` | Shared 7: materials and shaders | large |
| `10-shared-8.md` | Shared 8: effects (Solstice) | large |
| `11-shared-9.md` | Shared 9: bloom, white balance, grading, weather and moods | medium |
| `12-shared-10.md` | Shared 10: cutscene timeline, behaviour for characters | large |
| `13-tools-1b.md` | Tools 1b: the sculpted ground (G1–G5) | large |
| `14-open-worlds.md` | Open worlds W1–W5, revised from decision 0117 | large |
| `15-3d-1.md` | 3D 1: skeletal animation from glTF, navmeshes and avoidance | large |
| `16-3d-2-3.md` | 3D 2 and 3: the frame's shape, exposure, physical sky, sky light, night, clouds, AO, shadows | large each |
| `17-3d-4-5.md` | 3D 4 and 5: clustered lamps, light ramp look, volumetric fog, reflections, wetness, bounced light | large each |
| `18-shared-11-13.md` | Shared 11–13 (briefly): what scripting, export, Git, mods, netcode and 0.1 need from earlier phases | — |

Every file follows the brief's eight points per milestone (what and why; what exists; techniques compared with a recommendation; how in Solis; dependencies; tests and scale cases; questions; size) and ends with crates and sources. Product and game names appear only in source lists and in the names of cited talks; the plan itself must keep rule 20.

## Cross-cutting findings

1. **Lust changes gate a lot of this.** The phases need, from the Lust side (ask early, one PR each, with today's-Lust fallbacks noted in each file):
   - attributes on component fields (`float speed = 4 [range 0 10, tip "…"]`) — Shared 4;
   - **arrays (lists) of plain kinds in component fields** and kept fields — Shared 4 (also wanted by 2D 1's typed fields);
   - **blocks run later that capture only fields and `this`, with stable IDs and owners**, so timers, tweens and steps can be saved and ordered across cores — Shared 3 (this also closes decision 0106's tween-ordering gap);
   - `on enter` / `on leave` for a component's own state options, extending "things made in a state end with it" to engine things — Shared 3;
   - weighted picks over a state's options using the game's seed (and Lust's own `Random` seeded by the host) — Shared 3;
   - test blocks (`test "…" { }`) and `expect` — Shared 2;
   - drag-and-drop words in `view` — Shared 5.
2. **One pattern for everything made in an editor:** a generated Lust file per asset (decisions 0078, 0079) is the right shape for maps' new questions, per-prefab spawn builders, tables, materials' settable values, effects' parameters, music, cutscenes, behaviours and the ground. It keeps rule 25 and makes renames safe.
3. **Determinism is the backbone.** Game state (tile and ground edits, paths, triggers, timers, tweens, behaviour, cutscenes, weather choice, rooms, spawns, pools) must be identical on 1 core or 16 and on any PC, saved and checksummed; output (effects, light, sky, UI, music, shake, debug drawing) never is. Two holes exist **today**: tween done blocks ordered differently across core counts, and Lust's unseeded `Random` (decision 0106). A **cross-PC checksum check** (a recorded game replayed on two machines) should exist before Shared 3 brings more transcendental maths into game state. Ground brushes should use fixed-point; pathfinding answers are pure functions of the start-of-tick world (or "answered next tick, always"); GPU-made data is drawing only.
4. **The GL path shapes the renderer.** On Mesa's OpenGL (the tests' driver) wgpu gives no compute and no storage buffers. So: per-copy material values and skinning data as vertex attributes or textures; clustered light lists built on the CPU and uploaded as textures; 2D distance fields, jump flood and radiance cascades as fragment passes; GPU particles, volumetric fog, volumetric clouds and real-time GI are Default/High only, with Low on GL.
5. **A render restructure is due before materials and light pile in.** `viewport.wgsl` is one 956-line file in a forward renderer with MSAA. Recommended at the start of Shared 7 (or at the latest 3D 2's L1): split shaders into modules (Solis's own small composer, naga validating), a small frame graph of named passes, and a **depth-and-normals prepass with clustered (forward+) lighting**, keeping MSAA for the stylized outlines. light.md puts the prepass in 3D 3 (L9), after features that need it.
6. **Shared pieces worth building once:**
   - **Noise** (one bit-exact module, a WGSL twin for drawing): first needed by Open worlds' recipe, then 2D 1's decoration rules, Shared 3, materials and effects.
   - **The 2D distance field**: pathfinding clearance (2D 1), soft shadows and bounced light (2D 2), particle collisions (Shared 8).
   - **The `Draw` API**: editor drawing (Shared 4) and debug drawing in play (Shared 6).
   - **The timeline widget**: the Frames tab (exists), music sections (Shared 5), cutscenes (Shared 10), 3D clip cues (3D 1).
   - **The graph editor with typed pins**: materials (Shared 7) and effects (Shared 8) on the Animator's editor (decision 0050).
   - **The neighbour grid** (tags' world questions, Shared 2): reused by triggers, ORCA avoidance and the cheat box.
7. **File names:** decision 0081 says everything Solis makes is `<name>.<kind>.lst`. Older plans name `.solmat`, `.soleffect`, `.solweather` (decision 0015 reserved `.solmat`). Recommend the `.lst` forms throughout (a question for the lead).
8. **Several roadmap items are partly built already:** tile map "cell at a place" (decision 0082), state fields as asset pickers (project names are states), tweens (Lust, decision 0106), the player's settings kept apart from saves (0091), parallax (0094), time of day as game state (0092; light.md's question 4 is answered).

## Order: what looks wrong, and a recommended order

**Problems found in today's order:**
- **Q1. Open worlds sits ahead of all remaining 2D.** ROADMAP's step 1 runs Tools 1 then Open worlds (five large, purely 3D milestones) before finishing Usable first, the farming-game gate and 2D 1/2D 2, which decision 0097 put first. Open worlds also looks its best only with 3D 2/3's haze and far shadows.
- 2D 1's "entities with typed fields" is mostly Shared 4's field hints and new kinds of settings.
- 2D 2's "sky over 2D" depends on 3D 2's physical sky; its hit-stop and squash depend on Shared 3's tweens and timers; its "256 lamps" depends on clustered lighting (3D 4).
- 2D 1's "rooms on a world map" and Shared 6's "rooms and moving between them" are one system in two halves (fine if designed together: files and editor in 2D 1, run time in Shared 6).
- Effects' gate needs projectile **hits** as game state, still not built.
- Light's depth prepass (L9, 3D 3) is needed from 3D 2 on.
- Tools 1b's ground arrives long before 3D 1, so 3D characters on sculpted slopes lack navmeshes and foot IK until then; foliage and scattering (DESIGN.md §8) have no milestone.

**Recommended order** (each arrow a phase; ★ = differs from ROADMAP):
1. R1–R6 (in progress).
2. **Tools 1b** (as ROADMAP), shaping its region format, store interface and settings file for Open worlds now (`13-tools-1b.md`, "What Open worlds needs").
3. ★ **Finish Usable first** (L2–L6) and its gate (L2/L3 can even slot in before Tools 1b).
4. **Shared 2.**
5. **2D 1** (typed fields moved to Shared 4).
6. ★ **Shared 3** (time and feel) moved before 2D 2, so moving platforms, hit-stop, squash and shake use it. Alternative: keep the order and build small versions in 2D 2.
7. **2D 2** (sky over 2D from day-and-night colours; physical sky's colour later).
8. **Shared 4 → Shared 5 → Shared 6.**
9. **Shared 7** (with the render restructure) **→ Shared 8 → Shared 9 → Shared 10.**
10. The rethink; **Shared 11.**
11. **3D 1** (plus a small foot-IK milestone).
12. ★ **Open worlds** here, just before or alongside 3D 2/3 (it needs their haze and far shadows to look right), unless the lead wants big worlds sooner.
13. **3D 2 → 3D 3 → 3D 4 → 3D 5; Shared 12; Shared 13.**

**Dependency graph (main edges):**
```
Tools 1b ──► Open worlds ──► (looks right with) 3D 2/3 haze & far shadows
Tools 1b ──► Shared 7 (ground kinds and brush faces get materials) ──► 3D 1 navmesh (ground colliders)
Usable first L4 ◄── R2/R5 (level and prefab formats)
Shared 2 tags/grid ──► 2D 2 triggers, Shared 6 cheat box, 3D 1 avoidance, Shared 10 behaviour
Shared 2 spawn-with-values ──► Shared 6 pools
2D 1 distance field ──► 2D 2 shadows & cascades ──► Shared 8 pixel particles
2D 1 world file ──► Shared 6 rooms runtime
Shared 3 timers/tweens/states ──► 2D 2 polish, Shared 10 behaviour & cutscenes
Shared 3 noise ◄── first needed by Open worlds recipe / 2D 1 decoration
Shared 4 Draw API ──► Shared 6 debug drawing;  Shared 4 paths ──► 2D 2 platforms, Shared 10 cameras
Shared 4 table widget ──► Shared 5 translations view
Shared 5 text layout ──► text effects, translations
Shared 7 materials & graph ──► Shared 8 particles, Shared 9 wetness, 3D 1 skinned variants, 3D 2–5 inputs
Shared 8 effects ──► Shared 9 weather (rain, snow) ──► 3D 2 sky tint, 3D 3 clouds, 3D 4 wetness
render restructure (prepass, clusters) ──► 3D 2 … 3D 5
Lust changes ──► Shared 2 (tests), Shared 3, Shared 4, Shared 5
every new tool's commands ──► Shared 11 scripting window
determinism everywhere ──► Shared 12 netcode
```

## Every question for the project lead

Each has a recommended answer (first) and alternatives; details in the named file.

**Order and scope**
- **Q1** Open worlds before 2D? *Recommended:* move it to the 3D track (before 3D 2/3); keep Tools 1b where it is. *Alt:* keep ROADMAP's order. (`14`, `00`)
- **Q2** Move Shared 3 before 2D 2? *Recommended:* yes. *Alt:* small versions of hit-stop, squash and shake in 2D 2. (`04`, `05`)
- **Q3** Move 2D 1's typed fields into Shared 4? *Recommended:* yes. *Alt:* pull field hints and new setting kinds into 2D 1. (`03`)
- **Q4** Restructure the renderer (shader modules, frame graph, depth/normal prepass, forward+) at Shared 7's start? *Recommended:* yes. *Alt:* at 3D 2's L1; or deferred shading (loses MSAA). (`09`, `16`)
- **Q5** File names `<name>.<kind>.lst` for materials, effects, weather, music, cutscenes, behaviours, tables? *Recommended:* yes. *Alt:* the reserved `.sol*` extensions. (`09`, `10`, `11`)
- **Q6** Add a milestone for foliage and scattering on the ground? *Recommended:* yes, after Open worlds or 3D 2. (`14`)
- **Q7** A small foot-IK milestone after 3D 1? *Recommended:* yes. (`15`)
- **Q8** A cross-PC determinism check before Shared 3? *Recommended:* yes. (`18`)

**Usable first (`01`)**
- **Q9** Saves and recordings as Lust: a Lust header with packed bulk sections? *Alt:* all plain Lust; keep today's format.
- **Q10** Write the level and prefab formats as Lust directly in R2/R5 if not merged yet? *Alt:* a separate L4.
- **Q11** Do L2/L3 before Tools 1b so the ground's settings file is born as Lust? *Alt:* keep the order.

**Shared 2 (`02`)**
- **Q12** Map questions' shape: `cellsWhere<Value>(…)` per value. *Alt:* one generic `cellsWhere(FarmValue.X, …)`.
- **Q13** Where tags live: Project settings, saved as `tags.lst`. *Alt:* in `project.solis`.
- **Q14** World questions use 3D distance with a ground-plane grid. *Alt:* a flat-distance setting for 2D.
- **Q15** Spawning with values: a generated per-prefab builder (`BulletSpawn:new():setSpeed(12):spawn(p)`). *Alt:* component literals (Lust change); spawn plus an event.
- **Q16** Game tests: `test "…" { }` blocks in `*.test.lst` with `expect`, able to press actions. *Alt:* golden output; recordings only.
- **Q17** The ideas list's query chain (`:where(...)`) waits for Lust's query design; plain calls now.

**2D 1 (`03`)**
- **Q18** Rooms: level files plus a world file and World view now, run time in Shared 6. *Alt:* one big level; all of it now.
- **Q19** Sort lines (per-column depth) and a "Stands up" tile layer, besides sort points. *Alt:* sort points only.
- **Q20** Decoration rules in a shareable `.decor.lst`. *Alt:* inside the tileset.
- **Q21** Pathfinding: HPA* over chunks plus flow fields, cost per terrain. *Alt:* plain A*.
- **Q22** Keys: Scatter `C`; save a stamp with a button and Ctrl + Shift + C; no key for the World view.

**2D 2 (`04`)**
- **Q23** Platformer feel: extend Solis's mover with sub-pixel remainders and carry-then-push. *Alt:* a separate pixel-exact box mover.
- **Q24** Trigger events: `Entered(entity who)`/`Exited(entity who)` on the zone; `EnteredZone`/`LeftZone` on the walker. *Alt:* zone events only.
- **Q25** A teleport through a trigger fires nothing. *Alt:* both events.
- **Q26** Sky over 2D from day-and-night colours now, physical sky later. *Alt:* bring 3D 2's sky forward.
- **Q27** Build 2D lamp culling now (for 256 lamps) and share it with 3D 4. *Alt:* 16 lamps until 3D 4.
- **Q28** A `Dash` action in the kit? *Recommended:* no.

**Shared 3 (`05`)**
- **Q29** Blocks run later may use only the component's fields (so they can be saved). *Alt:* allow locals and don't save pending blocks.
- **Q30** Timers both as blocks (`after`, `every`) and as named timers with `TimerFinished`.
- **Q31** A clock time jumped over fires once. *Alt:* skipped.
- **Q32** Shake and fade snap to whole pixels in pixel games by default.
- **Q33** Rumble goes to the pad that last gave input.
- **Q34** Weighted random syntax `Random[Common 70, Rare 25, Legendary 5]` with the game's seed.

**Shared 4 (`06`)**
- **Q35** Lists in components (a Lust change; plain kinds, a length limit). *Alt:* lists only on built-in components.
- **Q36** Hint words `range`, `step`, `tip`, `unit`, `slider`, `group`.
- **Q37** Data tables are read-only constants, with a replay warning if changed. *Alt:* tables game code changes (game state).
- **Q38** Path kinds Straight, Smooth, Curves with handles. *Alt:* Smooth only.
- **Q39** Path keys: Tab to edit points, click adds, Ctrl + click inserts, Delete removes, Alt + drag breaks a handle.
- **Q40** Editor drawing runs game code sandboxed, for selected entities by default.
- **Q41** Save pictures on by default at 320 × 180.

**Shared 5 (`07`)**
- **Q42** Text layout with `parley` (if it passes dependency checks). *Alt:* grow Solis's own with `harfrust`.
- **Q43** Writing systems: Latin, Greek, Cyrillic now; CJK via fallback fonts; right-to-left later. *Alt:* all now; Latin only.
- **Q44** Translations in Fluent files under a side-by-side table view. *Alt:* Solis tables only; `.po`.
- **Q45** Gamepad pictures as neutral shapes by position, maker-style symbols drawn by Solis optional.
- **Q46** Text markup words `[wave] [shake] [colour=…] [b] [i] [icon=…] [button=…] [pause n] [speed n]`.
- **Q47** Music: layers and sections with beat-quantised changes. *Alt:* layers only.

**Shared 6 (`08`)**
- **Q48** What a room keeps: a setting per room (reset, remember, keep running), remember by default.
- **Q49** Game time pauses during transitions so loading speed never changes results. *Alt:* a fixed tick delay.
- **Q50** Pools: measure and speed up spawning first, then engine-managed pools as a prefab setting.
- **Q51** Cheat box in exported games: off by default, allowed by a project setting, the key left of 1.
- **Q52** Bug-report key F8 in playtest builds plus a pause-menu item.
- **Q53** What a report holds (recording, build identity, note, screenshot, logs, settings, Windows and GPU; nothing personal).

**Shared 7 (`09`)** (plus `materials.md`'s nine open questions, recommendations unchanged)
- **Q54** Per-copy smoothing of the light-around input approximated on the CPU. *Alt:* GPU read-back a frame late.

**Shared 8 (`10`)**
- **Q55** CPU for every emitter, GPU as a per-emitter option from one graph. *Alt:* two full paths (the 2026-10-25 answer).
- **Q56** Calls under the guidelines: `Effects:play(Effect.Hits.Laser, place, facing)`, `playEffect(...)` on entities, parameters through a generated file.
- **Q57** A small milestone for projectile hits as game state before the effects gate.
- **Q58** An effects study report before E1 (decision 0051).

**Shared 9 (`11`)**
- **Q59** The chosen weather and wetness are game state (rain can water crops). *Alt:* decoration only.
- **Q60** Which lens extras exist at all (flare, grain, fringes, heat haze; all off by default).

**Shared 10 (`12`)**
- **Q61** Behaviour as state machines with schedules and utility scores (mock-up beside a behaviour tree first). *Alt:* behaviour trees.
- **Q62** Cutscenes as game state (deterministic, saved, skippable to the same end state). *Alt:* output that pauses the game.
- **Q63** Timeline keys: Space (only while focused), J/K/L, I/O, S, arrows. *Alt:* buttons only.

**Tools 1b (`13`)**
- **Q64** Blocky step height 1.05 m by default. *Alt:* smooth colliders under a blocky look.
- **Q65** Ground undo capped at 256 MB. *Alt:* unlimited.
- **Q66** Region files shaped for Open worlds now (chunk table, levels reserved).
- **Q67** Game code's ground edits limited to 64 dabs per tick.
- **Q68** Painted kinds: soft height-blended corners on Smooth/Sharp, hard per face on Blocky.

**Open worlds (`14`)**
- **Q69** Recipe versions kept forever, upgrades opt-in with backups. *Alt:* bake worlds fully.
- **Q70** Erosion as local noise plus a height-map ingredient. *Alt:* a baked erosion pass over chosen areas.
- **Q71** Far ground made on the GPU where possible (drawing only). *Alt:* CPU only.
- **Q72** Camera-relative drawing always; simulation origin shifts only past 20 km.
- **Q73** Gate at 10 km full targets and 20 km working. *Alt:* the study's 20 km gate.

**3D 1 (`15`)**
- **Q74** Navmesh builder: evaluate `rerecast` behind a wall (no tiles yet), own tiled builder if needed. *Alt:* own from the start.
- **Q75** Navmeshes rebuilt on load, cached for big levels (never in Git).
- **Q76** Avoidance by ORCA. *Alt:* separation steering.

**3D 2 to 5 (`16`, `17`)** (plus `light.md`'s open questions, recommendations unchanged; its question 4 is answered by decision 0092)
- **Q77** Clusters built on the CPU so the GL path lights the same way.
- **Q78** Virtual shadow maps not now (cached cascades instead).
- **Q79** Port AMD's distance-field GI (MIT) to WGSL where it fits, own code where not.
- **Q80** Wet streets at Default: probes and wetness; SSR at High.
- **Q81** Lamp shadow budget 4 at Default, 8 at High.

## Crates mentioned (all to be checked against `deny.toml` and registered, rule 5)

| Crate | Licence (from its page) | Recommendation |
| --- | --- | --- |
| `pathfinding` 4.x | MIT or Apache-2.0 | test oracle only (2D 1) |
| `kiddo`, `rstar` | permissive (confirm) | exit plan for world questions (Shared 2) |
| `gilrs` 0.11 (present) | MIT or Apache-2.0 | rumble (Shared 3) |
| `fastnoise-lite` | MIT | reference for Solis's own noise |
| `parley`, `unicode-linebreak` | Apache-2.0 or MIT | text layout (Shared 5) |
| `fluent`, `fluent-bundle` | Apache-2.0 or MIT | translations (Shared 5) |
| `kira` 0.12 (present) | MIT or Apache-2.0 | clocks for music (Shared 5) |
| `naga_oil` | MIT or Apache-2.0 | reference; write own shader composer (Shared 7) |
| `flate2`, `miniz_oxide` (in the lock) | MIT/Apache(/Zlib) | ground region files (Tools 1b) |
| `fast-surface-nets`, `block-mesh` | MIT or Apache-2.0 | no (old glam, small); own code from the study |
| `rerecast` 1.0 | to check (Recast is Zlib) | candidate navmesh builder (3D 1) |
| `dodgy_2d`, `landmass` | MIT or Apache-2.0 | references for ORCA and agents |
| `rotsprite` | unconfirmed | only if baking rotations (2D 2) |
| `ozz-animation-rs` | MPL-2.0 | not allowed; reference only |

## Sources (all files)

Pathfinding: [HPA*](https://webdocs.cs.ualberta.ca/%7emmueller/ps/hpastar.pdf), [DHPA*/SHPA*](https://ojs.aaai.org/index.php/AIIDE/article/view/12397), [Amit's A* pages](https://theory.stanford.edu/~amitp/GameProgramming/index.html), [JPS and goal bounding (JAIR)](https://jair.org/index.php/jair/article/view/12255), [JPS+ (GDC)](https://gdcvault.com/play/1022094/contactUs), [flow field tiles (Game AI Pro)](https://www.taylorfrancis.com/books/9780429100277/chapters/10.1201/b16725-29), [bevy_flowfield_tiles_plugin](https://docs.rs/crate/bevy_flowfield_tiles_plugin/latest/source/README.md), [pathfinding crate](https://docs.rs/pathfinding/4.9.0), [Red Blob flow fields](https://www.redblobgames.com/pathfinding/tower-defense/index.html), [Polyanya](https://www.ijcai.org/proceedings/2017/0070.pdf).
Spatial queries: [fixed-radius neighbours](https://gamedev.net/forums/topic/649674-fixed-radius-near-neighbor-search/), [spatial hashing](https://saeed1262.github.io/blog/2025/spatial-hashing-collision), [kiddo](https://docs.rs/crate/kiddo/5.2.3/source/README.md), [rstar](https://docs.rs/crate/rstar/latest).
2D worlds and tiling: [LDtk worlds](https://ldtk.io/docs/general/world/), [LDtk auto-layer rules](https://ldtk.io/docs/general/auto-layers/auto-layer-rules/), [2D sorting](https://adventurecreator.org/tutorials/2d-sorting).
2D movement: [Celeste & TowerFall physics](https://maddymakesgames.com/articles/celeste_and_towerfall_physics), [Building a better jump](https://gdcvault.com/play/1023148/Math-for-Game-Programmers-Building), [ghost collisions](https://box2d.org/posts/2020/06/ghost-collisions/), [Rapier one-way platforms](https://git.nea.moe/github/rapier.git/tree/examples2d/one_way_platforms2.rs?h=v0.18.0).
2D light: [jump flooding](https://www.comp.nus.edu.sg/~tants/softShadow/p173-rong.pdf), [RTSDF](https://diglib.eg.org/handle/10.2312/pg20201232), [radiance cascades explained](https://jason.today/rc), [holographic radiance cascades](https://arxiv.org/pdf/2505.02041) and [summary](https://www.themoonlight.io/review/holographic-radiance-cascades-for-2d-global-illumination), [Chalmers RC thesis](https://odr.chalmers.se/items/3ee9fb4e-1880-46c2-802a-a660e38dc9ee), [pixel-art normal maps study](https://arxiv.org/pdf/2212.09692), [pixel rotation](https://godotshaders.com/shader/rotsprite-like-algorithm-for-cleaner-pixel-art-rotation/), [cleanEdge](https://community.aseprite.org/t/cleanedge-rotation-algorithm-implementation/20371).
Time and feel: [Penner easing](http://robertpenner.com/easing/), [Juicing your cameras](https://gdcvault.com/play/1023146/Math-for-Game-Programmers-Juicing), [gilrs](https://docs.rs/crate/gilrs/0.11.2/source/README.md), [FastNoise Lite](https://lib.rs/crates/fastnoise-lite), [Statecharts](https://www.inf.ed.ac.uk/teaching/courses/seoc/2005_2006/resources/statecharts.pdf).
Data and editor: [centripetal Catmull-Rom](https://www.cemyuksel.com/research/catmullrom_param/catmullrom_cad.pdf).
Words and sound: [kira clocks](https://docs.rs/crate/kira/latest/source/src/clock.rs), [fluent-rs](https://github.com/projectfluent/fluent-rs/), [fluent licences](https://licenses.dev/cargo/fluent/0.16.1).
Testing: [deterministic lockstep](https://gafferongames.com/post/deterministic_lockstep/).
Materials: [wgpu PipelineCache](https://wgpu.rs/doc/wgpu/struct.PipelineCache.html), [Vulkan pipeline cache](https://docs.vulkan.org/guide/latest/pipeline_cache.html), [naga_oil](https://github.com/Wumpf/naga_oil), [triplanar normals](https://bgolus.medium.com/normal-mapping-for-a-triplanar-shader-10bf39dca05a).
Effects: [AMD compute particles](https://www.slideshare.net/slideshow/holy-smoke-faster-particle-rendering-using-direct-compute-by-gareth-thomas/35656271), [GDC advanced effects](https://gdcvault.com/play/1020002/Advanced-Visual-Effects-with-DirectX), [GPU particle simulation](https://wickedengine.net/2017/11/gpu-based-particle-simulation).
Finished picture: [Jimenez bloom](https://www.iryoku.com/next-generation-post-processing-in-call-of-duty-advanced-warfare/), [physically based bloom](https://learnopengl.com/Guest-Articles/2022/Phys.-Based-Bloom), [chromatic adaptation](http://www.brucelindbloom.com/index.html?Eqn_ChromAdapt.html), [Tony McMapface](https://github.com/h3r2tic/tony-mc-mapface).
Behaviour: [Halo 2 AI](https://www.gamedeveloper.com/programming/gdc-2005-proceeding-handling-complexity-in-the-i-halo-2-i-ai), [behaviour trees (arXiv)](https://arxiv.org/abs/1709.00084), [utility AI (GDC)](https://www.gdcvault.com/play/1021848/Building-a-Better-Centaur-AI), [GOAP](https://www.gamedevs.org/uploads/three-states-plan-ai-of-fear.pdf).
Ground and worlds: [surface nets](https://0fps.net/2012/07/12/smooth-voxel-terrain-part-2/), [greedy meshing](https://0fps.net/2012/06/30/meshing-in-a-minecraft-game/), [voxel AO](https://0fps.net/2013/07/03/ambient-occlusion-for-minecraft-like-worlds/), [Transvoxel](https://transvoxel.org/Lengyel-VoxelTerrain.pdf), [geometry clipmaps](https://hhoppe.com/geomclipmap.pdf), [CDLOD](https://aggrobird.com/files/cdlod_latest.pdf), [fBm](https://iquilezles.org/articles/fbm/), [domain warping](https://iquilezles.org/articles/warp/), [noise derivatives](https://iquilezles.org/articles/morenoise/), [terrain splatting](https://www.gamedeveloper.com/programming/advanced-terrain-texture-splatting), [fast-surface-nets](https://docs.rs/crate/fast-surface-nets/0.2.0/source/Cargo.toml).
3D animation and navigation: [skinned instancing](https://developer.download.nvidia.com/SDK/10/direct3d/Source/SkinnedInstancing/doc/SkinnedInstancingWhitePaper.pdf), [Recast](https://github.com/recastnavigation/recastnavigation), [rerecast](https://docs.rs/crate/rerecast/0.3.2), [ORCA](https://gamma.cs.unc.edu/ORCA/publications/ORCA.pdf), [dodgy_2d](https://docs.rs/crate/dodgy_2d/latest/source/Cargo.toml), [semi-procedural locomotion](https://runevision.com/thesis/).
3D light: [Hillaire sky (2020)](https://sebh.github.io/publications/egsr2020.pdf), [Bruneton scattering](https://ebruneton.github.io/precomputed_atmospheric_scattering/), [volumetric clouds (2015)](https://advances.realtimerendering.com/s2015/The%20Real-time%20Volumetric%20Cloudscapes%20of%20Horizon%20-%20Zero%20Dawn%20-%20ARTR.pdf), [physically based sky and clouds (2016)](https://www.ea.com/frostbite/news/physically-based-sky-atmosphere-and-cloud-rendering), [GTAO](https://www.activision.com/cdn/research/Practical_Real_Time_Strategies_for_Accurate_Indirect_Occlusion_NEW%20VERSION_COLOR.pdf), [visibility bitmask](https://arxiv.org/abs/2301.11376), [clustered shading](https://www.cse.chalmers.se/~uffe/clustered_shading_preprint.pdf), [practical clustered shading](https://www.humus.name/Articles/PracticalClusteredShading.pdf), [shadow map techniques](https://learn.microsoft.com/en-us/windows/win32/dxtecharts/common-techniques-to-improve-shadow-depth-maps), [LTC area lights](https://eheitzresearch.wordpress.com/415-2/), [volumetric fog](https://bartwronski.com/publications/), [unified volumetrics](https://www.ea.com/frostbite/news/physically-based-unified-volumetric-rendering-in-frostbite), [parallax-corrected cubemaps](https://seblagarde.wordpress.com/2012/09/29/image-based-lighting-approaches-and-parallax-corrected-cubemap/), [stochastic SSR](https://www.ea.com/frostbite/news/stochastic-screen-space-reflections), [DDGI](https://jcgt.org/published/0008/02/01/), [Brixelizer GI (MIT)](https://gpuopen.com/fidelityfx-brixelizer/), [FidelityFX SDK](https://gpuopen.com/manuals/fidelityfx_sdk/), [wgpu ray tracing (experimental)](https://wgpu.rs/doc/src/wgpu/documentation/extensions/ray_tracing.rs.html), [Yale Bright Star Catalogue](https://cdsarc.cds.unistra.fr/viz-bin/cat/V/50).

**Blocked during research:** `https://arxiv.org/abs/2505.02041` (a permission request timed out; the PDF and a summary were used instead). The AIIDE DHPA* page returned a bot-check page, so its abstract wasn't read; it is cited by title.
