# Solis roadmap

The plan from an empty repo to **Solis 0.1**, in phases. The engine is Rust; games are written in Lust, Rust or Lua ([decisions 0002](docs/decisions/0002-engine-in-rust-games-in-lust.md) and [0003](docs/decisions/0003-games-in-rust-lust-or-lua.md)). DESIGN.md says what Solis is; this file says in what order it gets built.

**About a year,** if Solis gets steady attention alongside Lust. Months are a guide, not a promise: the pace is set by decisions, testing on Windows and reviews.

Each phase ends with a **gate**: something the project lead tries by hand before the next phase starts. Each milestone inside a phase is one pull request.

## At a glance

| Phase | Months | What you get |
| --- | --- | --- |
| 0. Groundwork | 0–½ | The editor window opens, in Solis's theme |
| 1. Seeing things | ½–2 | Models, materials, lights and shadows in the viewport |
| 2. The world | 2–3 | Entities, components, areas and assemblies, saved as text |
| 3. The editor | 3–4 | Building an area by hand, with play mode |
| 4. Lust and Lua join | 4–5 | Components written in Lust or Lua, in play mode within 2 seconds |
| 5. Scale | 5–6 | 2,000 units and 20,000 projectiles at 60 fps; 10,000 and 100,000 still playable; the ceiling measured |
| 6. Simulation and play | 6–7 | Physics, input, a deterministic simulation |
| 7. Sound and animation | 7–8 | Sound events and buses; animated characters |
| 8. Netcode | 8–10 | Two PCs playing together over the internet |
| 9. Effects and game UI | 9–10 | Solis Eclipse basics; Sungraph UI, game UI drawn in Solis |
| 10. Scripting, export and Git | 10–11 | The scripting window; games packaged for Windows; working in teams through Git |
| 11. Solis 0.1 | 11–12 | The flagship space battle, docs, release |

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

## Phase 7: Sound and animation

- **Sound** (Kira): sound events with random clips, pitch and volume; buses and ducking; 3D falloff; reverb zones.
- **Animation:** skeletal animation from glTF, state machines, blending, and animation events.

**Gate:** a character walks and runs with smooth blending, footsteps play on the right frames, and a reverb zone changes how they sound.

## Phase 8: Netcode

Netcode in layers ([decision 0005](docs/decisions/0005-netcode-in-layers.md)):

- **Transport and sessions:** connections, reliable and unreliable messages, encryption, lobbies, and relays or Steam's networking for players behind home routers.
- **Tools:** a lag and packet-loss simulator, network stats, desync detection with the checksums from Phase 6, and replays.
- **Lockstep,** as an optional module, for strategy games.
- **Other models plug in:** check that `lightyear` (server-authoritative) and `ggrs` (rollback) work with Solis, with a small example each.
- Dedicated server builds.

**Decide before starting:** relays of our own or Steam's networking.

**Gate:** two PCs on different networks play the benchmark area together without desyncing, with simulated lag.

## Phase 9: Effects and game UI

- **Solis Eclipse basics:** particles through instancing, flipbooks, trails, and stylized shading for effects.
- **Post-processing:** bloom and colour grading.
- **Sungraph UI** ([decision 0039](docs/decisions/0039-game-ui-is-drawn-in-solis.md)), whole: a visual UI editor where game UI is drawn canvas style (lines, shapes, curves, text) instead of built from images; values tied live to game code; every element reachable from code, because a UI file is Lust (`view { }`); styles such as glow, pixelation and stylized outlines over the whole UI. HUDs and menus are made with it.

**Gate:** laser bolts, hits and explosions look good in the stylized look; a health bar and a pause menu, drawn in Sungraph UI with their values tied to game code, are on screen.

## Phase 10: Scripting, export and Git

- **The scripting window:** LustScript and Lua through Luau, calling the same commands as the built-in tools, with undo, a sandbox and time limits (the design Aventail plans for its Phase 9).
- **Build and export:** packaged games for Windows ([decision 0012](docs/decisions/0012-windows-only.md)), and Build-and-Play from the top bar.
- **The Git panel** ([decision 0006](docs/decisions/0006-git-built-in.md)): changes with plain-language area summaries, commit, pull and push, branches, history, Git LFS locking, markers in the library and area tree, and new projects from or published to a repository.
- **The area merge tool:** merges areas and assemblies entity by entity; when two people changed the same field, asks which to keep.

**Gate:** write a small editor tool that places objects in a ring; export a game and run it on a PC without Solis installed. Two people on two PCs work on the same project through GitHub for an evening: commit, pull, lock a model, and resolve one real area conflict in the editor.

## Phase 11: Solis 0.1

- **The flagship sample, written in Lust:** a small space battle. Two capital ships, a carrier launching fighter squadrons, a few hundred lasers in the air, over the network.
- **Docs:** getting started, a first-game tutorial, and a reference for the engine as Lust sees it (the `solis/` files).
- Templates for a new project.
- The 0.1 release.

**Gate:** someone new to Solis follows the tutorial and makes something that runs.

---

## Not in 0.1

Open-world streaming and terrain, navigation and AI, the full Solis Eclipse toolkit, the shader editor, rollback netcode, advanced animation (IK, root motion), Steam features, Linux and macOS (decision 0012).

## Who does what

- **Claude:** the engine in Rust, and the Lust side of Phase 4 in the Lust repo.
- **The project lead:** the decisions before each phase, every gate, the theme, and choosing controls. From Phase 4: the gameplay components in Lust, the flagship sample's game logic, and LustScript editor tools. Before then, small Rust tasks tagged `good first issue`, if you want to try some.
