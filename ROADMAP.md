# Solis roadmap

The plan from an empty repo to **Solis 0.1**, in phases. The engine is Rust; games are written in Lust, Rust or Lua ([decisions 0002](docs/decisions/0002-engine-in-rust-games-in-lust.md) and [0003](docs/decisions/0003-games-in-rust-lust-or-lua.md)). DESIGN.md says what Solis is; this file says in what order it gets built.

**No timeframe:** phases are done when they're done (the project lead, 2026-10-27). A phase is finished when the project lead has tried its gate by hand. Light, sky and atmosphere got a phase of their own ([decision 0046](docs/decisions/0046-light-before-netcode.md)), and so did materials and shaders ([decision 0048](docs/decisions/0048-materials-and-shaders-before-effects.md)). Compatibility, first-class 2D, a playtest export and game UI then moved to the front, so the project lead's own 2D game can move to Solis ([decisions 0053](docs/decisions/0053-compatibility-before-2d.md) and [0054](docs/decisions/0054-first-class-2d.md)). Plans for phases not started are named by subject (`docs/plans/light.md`), since the order can still change.

Each phase ends with a **gate**: something the project lead tries by hand before the next phase starts. Each milestone inside a phase is one pull request.

## At a glance

| Phase | What you get |
| --- | --- |
| 0. Groundwork | The editor window opens, in Solis's theme |
| 1. Seeing things | Models, materials, lights and shadows in the viewport |
| 2. The world | Entities, components, areas and assemblies, saved as text |
| 3. The editor | Building an area by hand, with play mode |
| 4. Lust and Lua join | Components written in Lust or Lua, in play mode within 2 seconds |
| 5. Scale | 2,000 units and 20,000 projectiles at 60 fps; 10,000 and 100,000 still playable; the ceiling measured |
| 6. Simulation and play | Physics, input, a deterministic simulation |
| 7. Sound and animation | Sound events and buses; animated characters |
| 8. Compatibility | Updates never lose work: backups before upgrades, old files keep opening, saves survive updates, the game-code interface guarded, real projects tested before an update ships |
| 9. 2D, part 1 | Pixel-perfect 2D, sprites from Aseprite, frame animation, tile maps that change as you play, autotiling, the tile painter, top-down movement, the 2D camera, lamps on sprites |
| 10. 2D, part 2 | Levels and the world map, advanced tiling, pathfinding, soft 2D shadows, the platformer and grid-step controllers, parallax and palette tools, the 2D sample |
| 11. Playtest export | Games packaged for playtesting, without the editor |
| 12. Game UI | Sungraph UI: HUDs and menus drawn in Solis |
| 13. Light, sky and atmosphere | A physical sky, sunsets and night; shadows near and far; many lamps; fog and clouds; bounced light, baked and real-time; 2D bounced light |
| 14. Materials and shaders | Materials as assets, values driven by the world, a node graph for shaders |
| 15. Effects | Solstice, the effects suite: particles, trails, flipbooks; a pixel mode for pixel art |
| 16. Netcode | Two PCs playing together over the internet |
| 17. Scripting, full export and Git | The scripting window; finished games built for Windows, with their own Rust; working in teams through Git |
| 18. Solis 0.1 | The flagship space battle, docs, release |

Lust joins in Phase 4, right after the editor, so game logic can be written in Lust from then on ([decision 0016](docs/decisions/0016-how-lust-joins-solis.md)). Phases 0–3 don't need Lust; if it isn't ready by Phase 4, Phase 5 (Scale) goes first.

---

## Phase 0: Groundwork

- A Cargo workspace with layered crates and a test that enforces the layers (as in Aventail).
- CI following [decision 0001](docs/decisions/0001-builds-on-demand.md): ready pull requests and manual runs only.
- A window (winit) with wgpu drawing into it.
- The egui editor shell, as in [decision 0004](docs/decisions/0004-editor-layout.md): the one-row top bar with document tabs, and dockable panels (`egui_dock`) for Area, Properties, the viewport, the library and the console, empty for now. Layouts saved per computer, with the three presets.
- **The theme:** dark greys and one accent colour, kept in one file so it's easy to adjust.

**Decided:** MIT, and Aventail's foundation code is copied in and adapted rather than shared ([decision 0007](docs/decisions/0007-mit-licence-and-separate-from-aventail.md)).

**Gate:** the editor opens on your PC, panels move and remember their places, and the theme feels right.

## Phase 1: Seeing things

**Decided:** lighting is a choice per project, with real-time lights and sky light first ([decision 0008](docs/decisions/0008-lighting-is-a-project-choice.md)); the camera flies with Unreal's controls ([decision 0009](docs/decisions/0009-fly-camera-controls.md)).

Milestones, one pull request each:

1. **Fly camera:** moving around the viewport, with the speed shown and remembered.
2. **Models:** importing glTF models and textures, drawn with a simple material.
3. **Materials:** a standard lit material and a stylized/toon option.
4. **Lights and shadows:** directional, point and spot lights; shadows from the sun; ambient light from the sky.
5. **The finished picture:** HDR, tonemapping, sky and fog.

**Gate:** a model from Aventail or Blender looks good in the viewport, in both the standard and the stylized look.

## Phase 2: The world

The working plan, with the order milestones are built in, is [docs/plans/phase-2.md](docs/plans/phase-2.md).

- Entities and components, in an area document ([decision 0019](docs/decisions/0019-scene-document-now-game-storage-later.md)).
- Transforms and parenting.
- Components that describe themselves, so the inspector can show any component, built-in or custom.
- Areas as text files that merge cleanly in Git: one entity per block, stable IDs, fields in a fixed order ([decision 0006](docs/decisions/0006-git-built-in.md)).
- New projects come with a `.gitignore` and `.gitattributes` (Git LFS for models, textures and sound); generated files stay out of the repository.
- Assemblies: nested, with per-instance overrides.
- The asset database: stable IDs kept in a small text file beside each asset, so renaming a file doesn't break anything; assets reload when changed on disk.
- Undo and redo for every edit.

**Decided:** an area document now and the game's fast storage in Phase 3 (0019); Lust-style area files (0020); the asset database and models as node trees (0021); undo as reversible edits (0022).

**Gate:** save an area, change it in a text editor, and see it reload. Rename a texture and nothing breaks. Two people change different entities in the same area on two branches, and Git merges them without a conflict.

## Phase 3: The editor

- **First:** the game's fast storage, chosen with a benchmark at the scale target (`hecs` or Solis's own), built from the area document when Play is pressed ([decision 0019](docs/decisions/0019-scene-document-now-game-storage-later.md)). Chosen: `hecs` ([decision 0024](docs/decisions/0024-game-storage-is-hecs.md)). The working plan is [docs/plans/phase-3.md](docs/plans/phase-3.md).
- Picking and selection in the viewport, and move, rotate and scale gizmos.
- The entity list and the inspector.
- The library: assets and code, with a collapsible panel.
- The console: logs, errors and commands.
- Play mode, which resets the area when you stop. Several viewports.

**Gate:** build a small area from scratch in the editor, press Play, stop, and everything is back as it was.

## Phase 4: Lust and Lua join

Components in Rust work from Phase 2, since they're the engine's own kind. This phase adds the other two. It comes straight after the editor, because Lust components need the inspector and play mode, and nothing in Phases 5 to 7 has to wait for it ([decision 0016](docs/decisions/0016-how-lust-joins-solis.md)).

**Ready in Lust already:** `native` methods, `lust build --library`, vectors and rotations, components, events, and the compiler as a library with swaps (LUST.md §21, Lust 2.0.0-alpha.32), tested in Luau by Lust's `engine-test`.

- **Lust builds into Solis:** Solis writes the files that describe the engine to Lust (`solis/entity.lst`, `solis/world.lst` and so on) from its own Rust code, and checks a game's Lust with the compiler as a library; while editing it runs as Luau, and in built games as Rust, its `native` methods calling the engine.
- `object … like Component` becomes the same kind of component the engine's own are, with `has` fields in the inspector. Lust's mistakes show in the Problems panel with file and line, never a Rust error.
- **Engine types and events:** vectors and rotations as real value types in Lust, and typed events between components (`onHit(Entity by, int damage)`).
- **Fast changes** (DESIGN.md §3.1): the engine prebuilt, only the game's Lust built, and loaded into play mode without restarting.
- **Lua components** through Luau: the same inspector fields, events and entities, with the game API's Lua face.
- **A first game in Lust:** a small sample (a turret with ammo, cooldown and damage) that the later phases keep growing, and a first page of the Lust guide for Solis.

**Decided before starting:** while editing, Lust runs as Luau and is swapped into the running game, keeping values; built games run it as Rust ([decision 0028](docs/decisions/0028-lust-runs-as-luau-while-editing.md)). The working plan is [docs/plans/phase-4.md](docs/plans/phase-4.md).

**Gate:** change a Lust component while the game is running and see the change within **2 seconds**; a Lua component changes instantly. A mistake in Lust shows a friendly message, never a Rust error. One entity can carry a Rust, a Lust and a Lua component that talk to each other through events.

## Phase 5: Scale

The phase that proves DESIGN.md §2 early, while changing course is still cheap.

- GPU instancing, frustum culling and LODs.
- A job system, so the engine uses every CPU core.
- Drawing large numbers of small, fast things (projectiles) cheaply.
- A benchmark (Help > Make the benchmark…) at three levels ([decision 0030](docs/decisions/0030-scale-is-tested-at-three-levels.md)): **target**, 2,000 moving units and 20,000 projectiles; **stress**, 10,000 and 100,000; and a **ceiling** run that keeps adding until 60 fps ends.
- A frame profiler.

**Decided:** the job system is `rayon` ([decision 0031](docs/decisions/0031-the-job-system-is-rayon.md)); LODs are made on import or come from the model ([decision 0032](docs/decisions/0032-lods-made-on-import-or-the-model-s-own.md)). The working plan: [docs/plans/phase-5.md](docs/plans/phase-5.md).

**Gate:** on your PC, the target level runs at 60 fps and the stress level holds 30 fps without stutter; the ceiling run reports how far Solis goes; and the frame profiler shows where the time goes.

## Phase 6: Simulation and play

- A fixed-timestep simulation, separate from the frame rate.
- Determinism rules: seeded randomness, checksums of the game state, and Rapier physics in its determinism mode; Lust's checker and Lua warnings flag code that could break them.
- Input as named actions, with gamepads and rebinding.
- Save games.
- The first ready-made gameplay components, written in Lust as examples for game makers: a character controller and a camera controller.

**Gate:** walk a character around a level with physics, save, quit and load. The same recorded inputs always give exactly the same result.

## Phase 6 hardening

Stronger tests before building on the game loop ([decision 0042](docs/decisions/0042-a-hardening-milestone-before-phase-7.md)). Phase 6's reviews kept finding problems, some caused by earlier fixes, so this milestone makes the tests catch them first. The working plan: [docs/plans/phase-6-hardening.md](docs/plans/phase-6-hardening.md).

- Merge the open pull requests once tested (last: the project lead asked for the tests first).
- Whole-game invariant tests: random games with saving, loading, replays and edits, checked against rules that must always hold.
- Fuzzing every reader of a file a project can ship.
- Measuring the tests: coverage, and mutation testing on the core crates.
- Checks that stay: in CI, weekly, and in each phase's review.

**Gate:** the project lead reads the hardening report (coverage, mutations caught, hours fuzzed and what was found) and tries the playground once more.

## Phase 7: Sound and animation

The working plans are [docs/plans/phase-7-sound.md](docs/plans/phase-7-sound.md) and [docs/plans/phase-7-animation.md](docs/plans/phase-7-animation.md) ([decisions 0056](docs/decisions/0056-sound-with-kira.md) and [0057](docs/decisions/0057-animation-near-live-far-baked.md)): thirteen milestones, sound first.

- **Sound** (Kira): sound events with random clips, pitch and volume; buses and ducking; 3D falloff; reverb zones.
- **Animation:** skeletal animation from glTF, state machines, blending, and animation events. The state machine is built to drive 2D sprite frames too (Phase 9), so 2D and 3D share one animation system.

**Gate:** a character walks and runs with smooth blending, footsteps play on the right frames, and a reverb zone changes how they sound.

## Phase 8: Compatibility

The project lead's game moves to Solis next, so before it does, updates must never lose work or break a game ([decision 0053](docs/decisions/0053-compatibility-before-2d.md)). The working plan is [docs/plans/compatibility.md](docs/plans/compatibility.md). Safety here is automatic: nothing to remember to use.

- **A backup before every upgrade:** before Solis changes any file in a project, it saves a dated copy and checks the upgraded file reads back correctly; if any step fails, nothing changes.
- **Old files keep opening, forever:** every format (areas, assemblies, the project, settings, input, recordings, and later tile maps) has an upgrader from every older version, and an example file of each old version is kept as a test.
- **Save games survive updates,** so a game's update never wipes its players' progress.
- **The game-code interface is guarded:** a test fails if what game code can call changes by accident; a deliberate rename keeps the old name working, warns in Problems and offers to fix the code.
- **Real projects are tested:** a project (the samples, and the project lead's game if wanted, from a private repository or on their own PC) is opened and played from a recording before an update ships; if it breaks, the update is stopped.
- **Breaking changes are labelled** in the changelog, and must bring their upgrader.

**Gate:** open a project saved by an older build, see the backup and the upgrade; a save game from before an update loads after it.

## Phase 9: 2D, part 1: a 2D game can move in

First-class 2D ([decision 0054](docs/decisions/0054-first-class-2d.md)), built as flat things in the same world, so lamps, effects, materials and the rest work on 2D too, with real 2D tools on top. The working plan, with the research behind it and the bar every milestone must pass, is [docs/plans/2d.md](docs/plans/2d.md).

- **The pixel-perfect view and the 2D editor:** drawn at the game's own resolution and scaled up crisply, smooth scrolling without shimmer, sharp text and menus.
- **Sprites from Aseprite:** tags, slices, events, layers and palettes, reloaded on save; atlases packed automatically; 100,000 sprites drawn cheaply.
- **Frame animation** on the same state machine as 3D.
- **Tile maps that change as you play:** cells store meaning and their own data (a crop's growth), changes batched per chunk, saved as text that merges in Git.
- **Autotiling:** each tileset chooses its kind (16-tile, 47-tile, edge sets or custom rules), set up with a wizard.
- **The tile painter.**
- **2D physics** (Rapier2D) with tile collision merged into smooth shapes, triggers, and a ready-made top-down controller in Lust.
- **The 2D camera and feel tools.**
- **Lamps and day and night on sprites,** crisp on the pixel grid.

**Gate:** the project lead rebuilds a slice of their own game in Lust: walk the farm, till, plant and water, crops grow over days, sleep, lamps at night; smooth, crisp, no jitter.

## Phase 10: 2D, part 2: 2D complete

- **Levels and the world:** entities with typed fields, rooms on a world map, sort points for tall things.
- **Advanced tiling:** rules for decoration and variety, scatter brushes, patterns and stamps.
- **Pathfinding** on tile maps: A* and flow fields, updated as the map changes.
- **Soft shadows** from walls and objects, and optional normal and glow layers.
- **More movement:** grid-step and platformer controllers, one-way platforms, slopes and moving platforms.
- **Polish:** parallax, palette swaps and cycling, clean pixel-art rotation, hit-stop and squash, the 2D sample and the manual's 2D section.

**Gate:** the 2D sample and the project lead's extended game slice; every speed budget met; the old tools' known problems, each a passing test.

## Phase 11: Playtest export

- **File > Export for playtesting:** a folder to zip and send, with a game program without the editor and the project's files packed beside it.
- Lust and Lua run as in Play, so there's no long compile; windowed, fullscreen and resolution settings; saves in the player's saved-games folder; a crash log playtesters can send back.
- What waits for the full export (Phase 17): building a game's own Rust code, an installer, and Steam.

**Gate:** a friend plays an exported build on their PC without Solis installed.

## Phase 12: Game UI

- **Sungraph UI** ([decision 0039](docs/decisions/0039-game-ui-is-drawn-in-solis.md)), whole: a visual UI editor where game UI is drawn canvas style (lines, shapes, curves, text) instead of built from images; values tied live to game code; every element reachable from code, because a UI file is Lust (`view { }`); styles such as glow, pixelation and stylized outlines over the whole UI. HUDs and menus are made with it. Plan: [docs/plans/game-ui.md](docs/plans/game-ui.md).

**Gate:** a health bar and a pause menu on screen, drawn and tied to the game in the editor.

## Phase 13: Light, sky and atmosphere

Solis's selling point: skies, sunsets and light that feel right, never flat. The working plan, with the research and the choices to make, is [docs/plans/light.md](docs/plans/light.md) ([decision 0046](docs/decisions/0046-light-before-netcode.md)).

- **Measure first:** a light showcase project and quality levels (Low, Default, High).
- **Real light and automatic exposure,** as eyes adjust.
- **A physical sky** (Hillaire's method): blue by day, orange at sunset, blue at twilight, haze that tints distance; the painted sky kept as a style.
- **Light from the sky:** ambient light and reflections from the real sky, instead of one flat colour.
- **The finished picture, part two:** bloom, white balance, colour grading (moved from effects).
- **Time of day and night:** sun, moons and stars where they really are, the aurora; colour curves and a tint over the sky for stylized games.
- **Weather and moods:** weather files keyed by time of day, blended, with one overcast control (after the project lead's reference looks).
- **Clouds:** a flat layer, and volumetric clouds at High.
- **Ambient occlusion,** **cascaded and contact shadows** reaching far, and **hundreds of lamps** with a few shadowed, glowing surfaces that light the world (and glow that wakes in the dark, for plants and creatures), and area lights.
- **The light ramp look,** for blocky games: simple, readable light where dark means dark.
- **Volumetric fog and light shafts,** with moving, coloured fog volumes and caustics.
- **Reflection probes,** and **reflections on the screen with wet surfaces** (for neon streets in the rain).
- **Bounced light, every way that can be made fast:** baked light probes (decision 0008's second source of indirect light), bounce from the screen, real-time probes traced against a simplified world (no ray-tracing hardware needed), and a time-boxed experiment for a better method, kept only if it wins.

**Decide before starting:** the plan's open questions (the effects suite's name, light units, time of day as game state, quality levels, the night sky, the sky's styles). The order, clouds and bounced light are answered.

**Gate:** in the light showcase, the day runs from noon to night with no jumps; the golden hour feels like the project lead's favourite game sunsets; indoors never looks like the Backrooms; the stylized look is as good; the benchmark's target still runs at 60 fps.

## Phase 14: Materials and shaders

Light and effects meet in materials: how a surface takes light, glows, gets wet or sways in the wind. This phase sits between them ([decision 0048](docs/decisions/0048-materials-and-shaders-before-effects.md)). The working plan, with its choices, is [docs/plans/materials.md](docs/plans/materials.md).

- **Materials as assets** (`.solmat`), each with its own look (standard, stylized, light ramp, unlit), edited in Properties with a live preview.
- **Inputs that drive any material value:** the light around the surface, time, distance, height, the weather's wetness, noise, and values set by game code. General tools, so one glows as night falls, another fades with distance, another pulses to a game's beat.
- **A node graph for materials and shaders,** saved as readable text (decision 0039's rule), sharing its graph editor with the animator and the effects.
- **Code for experts:** a node holding a few lines of shader code, checked before it runs.
- **Building blocks for stylized looks:** light ramps, rim light, hatching, dithering, outlines per material, wind sway, triplanar mapping.
- **Per-copy values at scale:** thousands of copies of one material, each with its own colour or glow, still drawn together.

**Gate:** without writing code, make a plant that glows as night falls, stone that darkens and shines when it rains, a toon character with its own outline, and a tree that sways; the benchmark's target still runs at 60 fps.

## Phase 15: Effects

- **Solstice, the effects suite:** particles through instancing (on the CPU and the GPU), flipbooks, trails, stylized shading for effects, and a node graph editor. Plan: [docs/plans/effects.md](docs/plans/effects.md). Its bloom and grading moved to Phase 13; particles are drawn with Phase 14's materials.
- **A pixel mode for pixel-art games:** pixel-shaped particles (single pixels, pixel lines for wind streaks and rain, pixel circles for bubbles), movement snapped to the pixel grid, colours from the game's palette, flipbooks at pixel-art frame rates, and a ready-made set (wind streaks, fireflies, bubbles, rain, snow, dust, falling leaves, sparkles, smoke puffs); fireflies and sparks can give off real light.

**Gate:** laser bolts, hits and explosions look good in the stylized look, in the benchmark battle, at 60 fps; in a pixel-art game, fireflies light a field at night and wind streaks cross it, all crisp on the pixel grid.

## Phase 16: Netcode

Moved here from its first place, Phase 8 ([decisions 0046](docs/decisions/0046-light-before-netcode.md), [0048](docs/decisions/0048-materials-and-shaders-before-effects.md) and [0054](docs/decisions/0054-first-class-2d.md)): what it needs (fixed ticks, determinism, checksums, save games) is built, and a CI check keeps two PCs playing the same game until then. Netcode in layers ([decision 0005](docs/decisions/0005-netcode-in-layers.md)):

- **Transport and sessions:** connections, reliable and unreliable messages, encryption, lobbies, and relays or Steam's networking for players behind home routers.
- **Tools:** a lag and packet-loss simulator, network stats, desync detection with the checksums from Phase 6, and replays.
- **Lockstep,** as an optional module, for strategy games.
- **Other models plug in:** check that `lightyear` (server-authoritative) and `ggrs` (rollback) work with Solis, with a small example each.
- Dedicated server builds.

**Decide before starting:** relays of our own or Steam's networking.

**Gate:** two PCs on different networks play the benchmark area together without desyncing, with simulated lag.

## Phase 17: Scripting, full export and Git

- **The scripting window:** LustScript and Lua through Luau, calling the same commands as the built-in tools, with undo, a sandbox and time limits (the design Aventail plans for its Phase 9).
- **The full export:** finished games for Windows ([decision 0012](docs/decisions/0012-windows-only.md)), with the game's own Rust built in and Lust compiled for speed, an installer, and Build-and-Play from the top bar. It grows from the playtest export (Phase 11).
- **The Git panel** ([decision 0006](docs/decisions/0006-git-built-in.md)): changes with plain-language area summaries, commit, pull and push, branches, history, Git LFS locking, markers in the library and area tree, and new projects from or published to a repository.
- **The area merge tool:** merges areas and assemblies entity by entity; when two people changed the same field, asks which to keep.

**Gate:** write a small editor tool that places objects in a ring; export a game and run it on a PC without Solis installed. Two people on two PCs work on the same project through GitHub for an evening: commit, pull, lock a model, and resolve one real area conflict in the editor.

## Phase 18: Solis 0.1

- **The flagship sample, written in Lust:** a small space battle. Two capital ships, a carrier launching fighter squadrons, a few hundred lasers in the air, over the network.
- **Docs:** getting started, a first-game tutorial, and a reference for the engine as Lust sees it (the `solis/` files).
- Templates for a new project.
- The 0.1 release.

**Gate:** someone new to Solis follows the tutorial and makes something that runs.

---

## Not in 0.1

Open-world streaming and terrain, navigation and AI, the full Solstice toolkit, rollback netcode, advanced animation (IK, root motion), Steam features, Linux and macOS (decision 0012).

## Who does what

- **Claude:** the engine in Rust, and the Lust side of Phase 4 in the Lust repo.
- **The project lead:** the decisions before each phase, every gate, the theme, and choosing controls. From Phase 4: the gameplay components in Lust, the flagship sample's game logic, and LustScript editor tools. Before then, small Rust tasks tagged `good first issue`, if you want to try some.
