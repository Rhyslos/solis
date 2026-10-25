# Solis

A free, open-source 3D game engine for stylized games, strategy games, online co-op and PvP, and big open worlds. Lightweight and beginner-friendly. The engine is written in Rust, and games in [Lust](https://github.com/Rhyslos/Lust), Rust or Lua.

**Status:** Phase 1 (seeing things): the editor opens with its dockable panels and theme, you can fly around the viewport, and glTF models from Blender or Aventail can be imported and looked at. Models can be seen in a standard, stylized (cel-shaded) or unlit look, lit by a sun with shadows and by point lights and spotlights, under a daylight sky with fog, and tone mapped as in Blender. Phase 2 (the world) adds entities and components, undo, parenting, projects with a Library and hot reload, areas saved as text Git can merge, and assemblies; Phases 1 and 2 are built and wait for their gates. Solis is made for Windows ([decision 0012](docs/decisions/0012-windows-only.md)).

## Running it

**On Windows, without building it yourself:** open the repo's **Actions** tab, click the latest **Windows EXE** run, and download **solis-windows**. Unzip it and run `solis.exe`. A new one is built whenever code is merged; to build one now, choose **Windows EXE** and **Run workflow**.

**From the source code:**

You need [Rust](https://rustup.rs). Then, in this folder:

```
cargo run --release -p app
```

The first build takes a few minutes. `cargo run -p app -- --help` lists the options.

**The manual** ([docs/manual](docs/manual/SUMMARY.md)) explains everything Solis can do so far, starting with [getting started](docs/manual/getting-started.md).

**Moving around the viewport** ([decision 0009](docs/decisions/0009-fly-camera-controls.md)): hold the right mouse button and move the mouse to look around; while it's held, W A S D move, Q and E go down and up, the wheel changes speed, and Shift goes faster. The wheel on its own moves forward and back, and dragging with the middle button slides the view.

## Progress tracker

Solis is built in phases, one milestone per pull request. Each phase ends with a **gate** the project lead tries by hand before the next starts. This tracker is updated by the pull request that finishes each milestone; the full plan is in [ROADMAP.md](ROADMAP.md).

**Overall: 25 of 60 milestones** `█████░░░░░░░░` 42%

**Now:** Phases 1, 2 and 3 are built and wait for their gates to be tried by hand ([Phase 3 plan](docs/plans/phase-3.md)). Phase 4, game code, is built and reviewed, and waits for its gate too ([Phase 4 plan](docs/plans/phase-4.md)). Phase 5, scale, is built and reviewed, and waits for its gate ([Phase 5 plan](docs/plans/phase-5.md)): try **Help > Make the benchmark…**. Phase 6, simulation and play, is built and reviewed three times, and waits for its gate ([Phase 6 plan](docs/plans/phase-6.md)): try **Help > Make the playground…**. Next is a hardening milestone before Phase 7: stronger tests, built before the open pull requests are tested, then merging them ([plan](docs/plans/phase-6-hardening.md)).

### Phase 0 · Groundwork — 7 of 7

`█████████████` 100%

- [x] The workspace: crates in layers, with a check that keeps them apart
- [x] Automatic checks on ready pull requests (decision 0001)
- [x] A window with the GPU drawing a 3D viewport
- [x] The editor shell: top bar, dockable panels, three layouts, remembered between launches
- [x] The theme, matching the final mockup
- [x] A ready-made `solis.exe` built on GitHub (decision 0010)
- [x] The ground rules, changelog check, fuzz tests, scale check and user manual

**Gate:** the editor opens, panels move and remember their places, and the theme feels right. *Passed 2026-10-21.*

### Phase 1 · Seeing things — 5 of 5

`█████████████` 100%

- [x] Fly camera: moving around the viewport, with the speed shown and remembered
- [x] Models: importing glTF models and textures, drawn with a simple material
- [x] Materials: a standard lit material and a stylized/toon option
- [x] Lights and shadows: sun, point and spot lights; shadows from the sun; ambient light from the sky (decision 0014)
- [x] The finished picture: HDR, tone mapping, sky and fog (decision 0017)

**Gate:** a model from Aventail or Blender looks good in the viewport, in both the standard and the stylized look.

### Phase 2 · The world — 7 of 7

`█████████████` 100%

<details>
<summary>Milestones</summary>

- [x] Entities and components, in an area document (decision 0019)
- [x] Undo and redo for every edit (decision 0022)
- [x] Transforms and parenting
- [x] Components that describe themselves, so the inspector can show any of them
- [x] The asset database: renaming a file breaks nothing; assets reload when changed (decision 0021)
- [x] Areas saved as text that Git merges cleanly; new projects set up for Git (decision 0020)
- [x] Assemblies: nested, with per-copy changes

</details>

**Gate:** save an area, edit it in a text editor and see it reload; rename a texture and nothing breaks; Git merges two people's changes to one area.

### Phase 3 · The editor — 6 of 6

`█████████████` 100%

<details>
<summary>Milestones</summary>

- [x] Picking things in the viewport, and the move, rotate and scale gizmos (decision 0025)
- [x] The entity list and the inspector
- [x] The library: assets and code
- [x] The console: commands as well as messages
- [x] Play mode, which puts the area back when you stop
- [x] Several viewports

</details>

**Gate:** build a small area from scratch, press Play, stop, and everything is back as it was.

### Phase 4 · Lust and Lua join — 6 of 6

`█████████████` 100%

<details>
<summary>Milestones</summary>

- [x] Lust builds into Solis: the engine described to Lust (`solis/*.lst`), a game's Lust checked with mistakes in Problems ([plan](docs/plans/phase-4.md), decisions 0016 and 0028)
- [x] Lust components, with their fields in the inspector and running in play mode
- [x] Vectors and rotations in Lust, and typed events between components (Lust and Rust)
- [x] Fast changes: a Lust edit in play mode within 2 seconds (about 0.33 s)
- [x] Lua components
- [x] A first game in Lust: the turret sample (Help > Make the sample game), in Lust, Lua and Rust

</details>

**Gate:** change a Lust component while the game runs and see it within 2 seconds; Rust, Lust and Lua components talk through events.

### Phase 5 · Scale — 7 of 7

`█████████████` 100%

<details>
<summary>Milestones</summary>

- [x] Measure first: the frame profiler, and scale cases for the three levels ([plan](docs/plans/phase-5.md), decision 0030)
- [x] A job system that uses every CPU core (decision 0031)
- [x] Game code on every core: it sees the world as it was at the start of the tick (decision 0033)
- [x] Drawing only what's seen: culling, and shadows fitted to the view
- [x] LODs, made on import or the model's own (decision 0032)
- [x] Projectiles: hundreds of thousands, drawn in one go
- [x] The benchmark (Help > Make the benchmark…): target, stress and ceiling

</details>

**Gate:** on your PC, the target runs at 60 fps and stress holds 30 fps without stutter; the ceiling is reported; the profiler shows where the time goes.

### Phase 6 · Simulation and play — 5 of 5

`█████████████` 100%

<details>
<summary>Milestones</summary>

- [x] A fixed-timestep simulation ([plan](docs/plans/phase-6.md))
- [x] Determinism: seeded randomness, state checksums, deterministic physics
- [x] Input as named actions, with gamepads and rebinding
- [x] Save games
- [x] Ready-made components in Lust: character and camera controllers

</details>

**Gate:** walk a character around a level, save, quit and load; the same inputs always give the same result.

### Phase 6 hardening · Stronger tests — 0 of 5

`░░░░░░░░░░░░░` 0%

<details>
<summary>Milestones</summary>

- [ ] Merge the open pull requests ([plan](docs/plans/phase-6-hardening.md), [decision 0042](docs/decisions/0042-a-hardening-milestone-before-phase-7.md))
- [x] Whole-game invariant tests
- [ ] Fuzzing every file reader
- [ ] Measuring the tests: coverage and mutation testing
- [ ] Checks that stay

</details>

**Gate:** read the hardening report (coverage, mutations caught, hours fuzzed and what was found), and try the playground once more.

### Phase 7 · Sound and animation — 0 of 4

`░░░░░░░░░░░░░` 0%

<details>
<summary>Milestones</summary>

- [ ] Sound events, buses and ducking
- [ ] 3D sound and reverb zones
- [ ] Skeletal animation from glTF
- [ ] Animation state machines, blending and events

</details>

**Gate:** a character walks and runs with smooth blending, and its footsteps sound right.

### Phase 8 · Netcode — 0 of 5

`░░░░░░░░░░░░░` 0%

<details>
<summary>Milestones</summary>

- [ ] Connections and sessions: lobbies, relays, encryption
- [ ] Network tools: lag simulator, stats, desync detection, replays
- [ ] Lockstep, for strategy games
- [ ] Other netcode models plugging in, with examples
- [ ] Dedicated servers

</details>

**Gate:** two PCs on different networks play the benchmark together without desyncing.

### Phase 9 · Effects and game UI — 0 of 3

`░░░░░░░░░░░░░` 0%

<details>
<summary>Milestones</summary>

- [ ] Solis Eclipse basics: particles, trails, stylized effects
- [ ] Post-processing: bloom and colour grading
- [ ] Game UI basics: HUDs and menus

</details>

**Gate:** lasers, hits and explosions look good, with a health bar and pause menu on screen.

### Phase 10 · Scripting, export and Git — 0 of 4

`░░░░░░░░░░░░░` 0%

<details>
<summary>Milestones</summary>

- [ ] The scripting window: editor tools in LustScript or Lua
- [ ] Exporting games for Windows
- [ ] The Git panel: commit, pull, push, branches, locking
- [ ] The area merge tool

</details>

**Gate:** write a small editor tool; export a game that runs without Solis; two people work on one project through GitHub.

### Phase 11 · Solis 0.1 — 0 of 4

`░░░░░░░░░░░░░` 0%

<details>
<summary>Milestones</summary>

- [ ] The flagship sample: a space battle, written in Lust
- [ ] Docs: getting started, a first-game tutorial, the API reference
- [ ] New project templates
- [ ] The 0.1 release

</details>

**Gate:** someone new to Solis follows the tutorial and makes something that runs.

## More

- [DESIGN.md](DESIGN.md): what Solis is and how it's planned
- [ROADMAP.md](ROADMAP.md): the phases to Solis 0.1
- [docs/decisions](docs/decisions): decisions and their reasons
- [docs/manual](docs/manual/SUMMARY.md): the user manual
- [CONTRIBUTING.md](CONTRIBUTING.md): how Solis is developed, and its ground rules

MIT licensed: see [LICENSE](LICENSE).
