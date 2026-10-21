# Solis roadmap

The plan from an empty repo to **Solis 0.1**, in phases. The engine is Rust; games are written in Lust, Rust or Lua ([decisions 0002](docs/decisions/0002-engine-in-rust-games-in-lust.md) and [0003](docs/decisions/0003-games-in-rust-lust-or-lua.md)). DESIGN.md says what Solis is; this file says in what order it gets built.

**About a year,** if Solis gets steady attention alongside Lust. Months are a guide, not a promise: the pace is set by decisions, testing on Windows and reviews.

Each phase ends with a **gate**: something the project lead tries by hand before the next phase starts. Each milestone inside a phase is one pull request.

## At a glance

| Phase | Months | What you get |
| --- | --- | --- |
| 0. Groundwork | 0–½ | The editor window opens, in Solis's theme |
| 1. Seeing things | ½–2 | Models, materials, lights and shadows in the viewport |
| 2. The world | 2–3 | Entities, components, scenes and prefabs, saved as text |
| 3. The editor | 3–4 | Building a scene by hand, with play mode |
| 4. Scale | 4–5 | 2,000 units and 20,000 projectiles at 60 fps |
| 5. Simulation and play | 5–6 | Physics, input, a deterministic simulation |
| 6. Sound and animation | 6–7 | Sound events and buses; animated characters |
| 7. Lust and Lua join | 7–8 | Components written in Lust or Lua, in play mode within 2 seconds |
| 8. Netcode | 8–10 | Two PCs playing together over the internet |
| 9. Effects and game UI | 9–10 | Solis Eclipse basics; HUDs and menus |
| 10. Scripting, export and Git | 10–11 | The scripting window; games packaged for Windows and Linux; working in teams through Git |
| 11. Solis 0.1 | 11–12 | The flagship space battle, docs, release |

Phases 0–6 don't need Lust. If Lust isn't ready by Phase 7, Phases 8 and 9 go first.

---

## Phase 0: Groundwork

- A Cargo workspace with layered crates and a test that enforces the layers (as in Aventail).
- CI following [decision 0001](docs/decisions/0001-builds-on-demand.md): ready pull requests and manual runs only.
- A window (winit) with wgpu drawing into it.
- The egui editor shell, as in [decision 0004](docs/decisions/0004-editor-layout.md): the one-row top bar with document tabs, and dockable panels (`egui_dock`) for Scene, Properties, the viewport, the library and the console, empty for now. Layouts saved per computer, with the three presets.
- **The theme:** dark greys and one accent colour, kept in one file so it's easy to adjust.

**Decide before starting:** the licence (MIT proposed). Whether to copy Aventail's foundation code or share it as a common crate: copy now, share later is recommended, so neither project waits for the other.

**Gate:** the editor opens on your PC, panels move and remember their places, and the theme feels right.

## Phase 1: Seeing things

- Importing glTF models and textures.
- The viewport camera and navigation, from Aventail.
- Materials: a standard lit material and a stylized/toon option.
- Directional, point and spot lights; shadows from the sun.
- Tonemapping, sky and fog.

**Decide before starting:** the lighting approach (baked lightmaps, light probes or real-time GI, DESIGN.md §7).

**Gate:** a model from Aventail or Blender looks good in the viewport, in both the standard and the stylized look.

## Phase 2: The world

- Entity and component storage (ECS style).
- Transforms and parenting.
- Components that describe themselves, so the inspector can show any component, built-in or custom.
- Scenes as text files that merge cleanly in Git: one entity per block, stable IDs, fields in a fixed order ([decision 0006](docs/decisions/0006-git-built-in.md)).
- New projects come with a `.gitignore` and `.gitattributes` (Git LFS for models, textures and sound); generated files stay out of the repository.
- Prefabs: nested, with per-instance overrides.
- The asset database: stable IDs kept in a small text file beside each asset, so renaming a file doesn't break anything; assets reload when changed on disk.
- Undo and redo for every edit.

**Decide before starting:** storage (`bevy_ecs` on its own, or Solis's own) and the scene file format.

**Gate:** save a scene, change it in a text editor, and see it reload. Rename a texture and nothing breaks. Two people change different entities in the same scene on two branches, and Git merges them without a conflict.

## Phase 3: The editor

- Picking and selection in the viewport, and move, rotate and scale gizmos.
- The entity list and the inspector.
- The library: assets and code, with a collapsible panel.
- The terminal: logs, errors and commands.
- Play mode, which resets the scene when you stop. Several viewports.

**Gate:** build a small scene from scratch in the editor, press Play, stop, and everything is back as it was.

## Phase 4: Scale

The phase that proves DESIGN.md §2 early, while changing course is still cheap.

- GPU instancing, frustum culling and LODs.
- A job system, so the engine uses every CPU core.
- Drawing large numbers of small, fast things (projectiles) cheaply.
- A benchmark scene in the repo: **2,000 moving units and 20,000 projectiles.**

**Gate:** the benchmark runs at 60 fps on your PC, and the frame profiler shows where the time goes.

## Phase 5: Simulation and play

- A fixed-timestep simulation, separate from the frame rate.
- Determinism rules: seeded randomness, checksums of the game state, and Rapier physics in its determinism mode.
- Input as named actions, with gamepads and rebinding.
- Save games.
- The first ready-made gameplay components, in Rust for now: a character controller and a camera controller.

**Gate:** walk a character around a level with physics, save, quit and load. The same recorded inputs always give exactly the same result.

## Phase 6: Sound and animation

- **Sound** (Kira): sound events with random clips, pitch and volume; buses and ducking; 3D falloff; reverb zones.
- **Animation:** skeletal animation from glTF, state machines, blending, and animation events.

**Gate:** a character walks and runs with smooth blending, footsteps play on the right frames, and a reverb zone changes how they sound.

## Phase 7: Lust and Lua join

Components in Rust work from Phase 2, since they're the engine's own kind. This phase adds the other two.

**Needs from Lust:** its Rust output, and using Rust crates from Lust.

- `fetch solis;`: the game API as Lust sees it.
- `object … like Component` compiles to the same kind of component the engine's own are, with `has` fields in the inspector.
- Typed events between components.
- Fast changes (DESIGN.md §3.1): the engine prebuilt, only the game's code compiled, a faster compiler for development, and hot reload.
- **Lua components** through Luau: the same inspector fields, events and entities, with the game API's Lua face, and warnings where Lua code could break determinism.
- The ready-made gameplay components move from Rust to Lust, as examples for game makers.

**Decide before starting:** how fast changes reach play mode (DESIGN.md §3.1).

**Gate:** change a Lust component while the game is running and see the change within **2 seconds**; a Lua component changes instantly. A mistake in Lust shows a friendly message, never a Rust error. One entity can carry a Rust, a Lust and a Lua component that talk to each other through events.

## Phase 8: Netcode

Netcode in layers ([decision 0005](docs/decisions/0005-netcode-in-layers.md)):

- **Transport and sessions:** connections, reliable and unreliable messages, encryption, lobbies, and relays or Steam's networking for players behind home routers.
- **Tools:** a lag and packet-loss simulator, network stats, desync detection with the checksums from Phase 5, and replays.
- **Lockstep,** as an optional module, for strategy games.
- **Other models plug in:** check that `lightyear` (server-authoritative) and `ggrs` (rollback) work with Solis, with a small example each.
- Dedicated server builds.

**Decide before starting:** relays of our own or Steam's networking.

**Gate:** two PCs on different networks play the benchmark scene together without desyncing, with simulated lag.

## Phase 9: Effects and game UI

- **Solis Eclipse basics:** particles through instancing, flipbooks, trails, and stylized shading for effects.
- **Post-processing:** bloom and colour grading.
- **Game UI basics:** text, images, buttons and layout for HUDs and menus. The visual UI editor and Lust's `view { }` come later.

**Gate:** laser bolts, hits and explosions look good in the stylized look, with a health bar and a pause menu on screen.

## Phase 10: Scripting, export and Git

- **The scripting window:** LustScript and Lua through Luau, calling the same commands as the built-in tools, with undo, a sandbox and time limits (the design Aventail plans for its Phase 9).
- **Build and export:** packaged games for Windows and Linux, and Build-and-Play from the top bar.
- **The Git panel** ([decision 0006](docs/decisions/0006-git-built-in.md)): changes with plain-language scene summaries, commit, pull and push, branches, history, Git LFS locking, markers in the library and scene tree, and new projects from or published to a repository.
- **The scene merge tool:** merges scenes and prefabs entity by entity; when two people changed the same field, asks which to keep.

**Gate:** write a small editor tool that places objects in a ring; export a game and run it on a PC without Solis installed. Two people on two PCs work on the same project through GitHub for an evening: commit, pull, lock a model, and resolve one real scene conflict in the editor.

## Phase 11: Solis 0.1

- **The flagship sample, written in Lust:** a small space battle. Two capital ships, a carrier launching fighter squadrons, a few hundred lasers in the air, over the network.
- **Docs:** getting started, a first-game tutorial, and a reference for `fetch solis;`.
- Templates for a new project.
- The 0.1 release.

**Gate:** someone new to Solis follows the tutorial and makes something that runs.

---

## Not in 0.1

Open-world streaming and terrain, navigation and AI, the full Solis Eclipse toolkit, the visual UI editor, the shader editor, rollback netcode, advanced animation (IK, root motion), Steam features, macOS.

## Who does what

- **Claude:** the engine in Rust, and the Lust side of Phase 7 in the Lust repo.
- **The project lead:** the decisions before each phase, every gate, the theme, and choosing controls. From Phase 7: the gameplay components in Lust, the flagship sample's game logic, and LustScript editor tools. Before then, small Rust tasks tagged `good first issue`, if you want to try some.
